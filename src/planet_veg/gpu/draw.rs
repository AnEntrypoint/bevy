use super::buffers::{DRAW_ARGS_BYTES, LIGHT_UNIFORM_BYTES, Packer, SLOT_UNIFORM_BYTES, VIEW_UNIFORM_BYTES, VegGpu, VegGpuConfig, VegKind};
use super::cull::VegWitness;
use bevy::camera::{MainPassResolutionOverride, Viewport};
use bevy::core_pipeline::core_3d::CORE_3D_DEPTH_FORMAT;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::RenderApp;
use bevy::render::camera::ExtractedCamera;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::extract_resource::ExtractResource;
use bevy::render::mesh::RenderMesh;
use bevy::render::mesh::RenderMeshBufferInfo;
use bevy::render::mesh::allocator::MeshAllocator;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{
    BindGroup, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBinding, BufferBindingType, CachedRenderPipelineId, ColorTargetState,
    ColorWrites, CompareFunction, DepthBiasState, DepthStencilState, Face, FragmentState, IndexFormat, MultisampleState, PipelineCache, PrimitiveState, RenderPassDescriptor,
    RenderPipelineDescriptor, ShaderStages, SpecializedMeshPipeline, SpecializedMeshPipelineError, SpecializedMeshPipelines, StencilState, StoreOp, TextureFormat, VertexState,
};
use bevy::render::renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery};
use bevy::render::view::{ExtractedView, Msaa, ViewDepthStencilTexture, ViewTarget};
use bevy::shader::Shader;
use std::collections::HashMap;
use std::num::NonZeroU64;
use std::sync::atomic::Ordering;

pub fn insert_veg_shader(app: &mut App, shader: &Handle<Shader>, source: &'static str, path: &'static str) {
    if let Some(mut shaders) = app.world_mut().get_resource_mut::<Assets<Shader>>() {
        let _ = shaders.insert(shader.id(), Shader::from_wgsl(source, path));
    }
}

pub fn veg_mesh(positions: Vec<[f32; 3]>, normals: Vec<[f32; 3]>, uvs: Vec<[f32; 2]>, indices: Vec<u32>) -> Mesh {
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}

#[derive(Component, ExtractComponent, Clone, Copy, Debug, Default)]
#[extract_app(RenderApp)]
pub struct VegCamera;

#[derive(Resource, ExtractResource, Clone, Copy, Debug, PartialEq)]
#[extract_app(RenderApp)]
pub struct VegEnvironment {
    pub sun_direction: Vec3,
    pub day_shade: f32,
    pub sun_irradiance: Vec3,
    pub exposure: f32,
    pub ambient_mul: Vec3,
    pub look_saturation: f32,
    pub ambient_add: Vec3,
    pub look_contrast: f32,
    pub night_fill: Vec3,
    pub grade_mode: u32,
    pub time_s: f32,
    pub wind_strength: f32,
}

impl Default for VegEnvironment {
    fn default() -> Self {
        Self {
            sun_direction: Vec3::Y,
            day_shade: 1.0,
            sun_irradiance: Vec3::ONE,
            exposure: 1.0,
            ambient_mul: Vec3::splat(0.3),
            look_saturation: 1.0,
            ambient_add: Vec3::ZERO,
            look_contrast: 1.0,
            night_fill: Vec3::ZERO,
            grade_mode: 0,
            time_s: 0.0,
            wind_strength: 1.0,
        }
    }
}

impl VegEnvironment {
    pub fn view_bytes(&self, view: &ExtractedView) -> Vec<u8> {
        let clip_from_world = view.clip_from_world.unwrap_or_else(|| view.clip_from_view * view.world_from_view.to_matrix().inverse());
        let mut bytes = Vec::with_capacity(VIEW_UNIFORM_BYTES as usize);
        Packer(&mut bytes)
            .f32s(&clip_from_world.to_cols_array())
            .f32s(&view.world_from_view.to_matrix().to_cols_array())
            .f32s(&view.world_from_view.translation().to_array())
            .f32s(&[self.time_s, self.wind_strength, 0.0, 0.0, 0.0]);
        bytes
    }

    pub fn light_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(LIGHT_UNIFORM_BYTES as usize);
        Packer(&mut bytes)
            .f32s(&self.sun_direction.to_array())
            .f32s(&[self.day_shade])
            .f32s(&self.sun_irradiance.to_array())
            .f32s(&[self.exposure])
            .f32s(&self.ambient_mul.to_array())
            .f32s(&[self.look_saturation])
            .f32s(&self.ambient_add.to_array())
            .f32s(&[self.look_contrast])
            .f32s(&self.night_fill.to_array())
            .u32s(&[self.grade_mode]);
        bytes
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VegMaterial {
    pub shader: Handle<Shader>,
    pub vertex_entry: &'static str,
    pub fragment_entry: &'static str,
    pub group2: Option<BindGroupLayoutDescriptor>,
    pub cull_back: bool,
}

#[derive(Clone, Debug)]
pub struct VegTier {
    pub mesh: Handle<Mesh>,
    pub slot: u32,
}

#[derive(Component, ExtractComponent, Clone, Debug)]
#[extract_app(RenderApp)]
pub struct VegDrawSet {
    pub kind: VegKind,
    pub tiers: Vec<VegTier>,
    pub material: VegMaterial,
}

impl VegDrawSet {
    pub fn tree(config: &VegGpuConfig, geometry: u32, meshes: [Handle<Mesh>; 3], material: VegMaterial) -> Self {
        let tiers = meshes.into_iter().enumerate().map(|(tier, mesh)| VegTier { mesh, slot: config.tree_slot(geometry, tier as u32) }).collect();
        Self { kind: VegKind::Tree, tiers, material }
    }

    pub fn impostor(config: &VegGpuConfig, mesh: Handle<Mesh>, material: VegMaterial) -> Self {
        Self { kind: VegKind::Tree, tiers: vec![VegTier { mesh, slot: config.impostor_slot() }], material }
    }

    pub fn rock(kind: u32, mesh: Handle<Mesh>, material: VegMaterial) -> Self {
        Self { kind: VegKind::Rock, tiers: vec![VegTier { mesh, slot: kind }], material }
    }

    pub fn grass(near: Handle<Mesh>, mid: Handle<Mesh>, material: VegMaterial) -> Self {
        Self { kind: VegKind::Grass, tiers: vec![VegTier { mesh: near, slot: 0 }, VegTier { mesh: mid, slot: 1 }], material }
    }
}

#[derive(Resource, Default)]
pub struct VegMaterialGroups(pub HashMap<bevy::asset::AssetId<Shader>, BindGroup>);

#[derive(Resource)]
pub struct VegDrawPipeline {
    view_layout: BindGroupLayoutDescriptor,
    kind_layouts: [BindGroupLayoutDescriptor; 3],
    empty_layout: BindGroupLayoutDescriptor,
    slot_layout: BindGroupLayoutDescriptor,
}

#[derive(Resource)]
pub struct VegDrawBindings {
    view: BindGroup,
    kinds: [BindGroup; 3],
    empty: BindGroup,
    slot: BindGroup,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct VegPipelineKey {
    target_format: TextureFormat,
    samples: u32,
    kind: VegKind,
    material: VegMaterial,
}

fn whole(buffer: &Buffer) -> BindingResource<'_> {
    buffer.as_entire_binding()
}

const GRAPHICS: ShaderStages = ShaderStages::VERTEX_FRAGMENT;

fn buffer_entry(binding: u32, stages: ShaderStages, ty: BufferBindingType, dynamic: bool) -> BindGroupLayoutEntry {
    BindGroupLayoutEntry { binding, visibility: stages, ty: BindingType::Buffer { ty, has_dynamic_offset: dynamic, min_binding_size: None }, count: None }
}

pub fn init_draw(mut commands: Commands, device: Res<RenderDevice>, cache: Res<PipelineCache>, gpu: Res<VegGpu>) {
    let storage = |binding| buffer_entry(binding, ShaderStages::VERTEX, BufferBindingType::Storage { read_only: true }, false);
    let view_layout = BindGroupLayoutDescriptor::new(
        "veg_view_layout",
        &[buffer_entry(0, GRAPHICS, BufferBindingType::Uniform, false), buffer_entry(1, GRAPHICS, BufferBindingType::Uniform, false)],
    );
    let kind_layouts = [
        BindGroupLayoutDescriptor::new("veg_tree_instances", &[storage(0), storage(1), storage(2), storage(3)]),
        BindGroupLayoutDescriptor::new("veg_rock_instances", &[storage(0), storage(1), storage(2)]),
        BindGroupLayoutDescriptor::new("veg_grass_instances", &[storage(0), storage(1), storage(2)]),
    ];
    let empty_layout = BindGroupLayoutDescriptor::new("veg_empty", &[]);
    let slot_layout = BindGroupLayoutDescriptor::new("veg_slot", &[buffer_entry(0, GRAPHICS, BufferBindingType::Uniform, true)]);
    let view = device.create_bind_group(
        "veg_view_group",
        &cache.get_bind_group_layout(&view_layout),
        &[BindGroupEntry { binding: 0, resource: whole(&gpu.view) }, BindGroupEntry { binding: 1, resource: whole(&gpu.light) }],
    );
    let kind_group = |kind: VegKind| {
        let mut entries = vec![
            BindGroupEntry { binding: 0, resource: whole(&gpu.instances[kind.index()]) },
            BindGroupEntry { binding: 1, resource: whole(&gpu.chunks) },
            BindGroupEntry { binding: 2, resource: whole(&gpu.visible[kind.index()]) },
        ];
        if kind == VegKind::Tree {
            entries.push(BindGroupEntry { binding: 3, resource: whole(&gpu.impostor_dims) });
        }
        device.create_bind_group("veg_instance_group", &cache.get_bind_group_layout(&kind_layouts[kind.index()]), &entries)
    };
    let kinds = VegKind::ALL.map(kind_group);
    let empty = device.create_bind_group("veg_empty_group", &cache.get_bind_group_layout(&empty_layout), &[]);
    let slot = device.create_bind_group(
        "veg_slot_group",
        &cache.get_bind_group_layout(&slot_layout),
        &[BindGroupEntry { binding: 0, resource: BindingResource::Buffer(BufferBinding { buffer: &gpu.slot_uniforms, offset: 0, size: NonZeroU64::new(SLOT_UNIFORM_BYTES) }) }],
    );
    commands.insert_resource(VegDrawBindings { view, kinds, empty, slot });
    commands.insert_resource(VegDrawPipeline { view_layout, kind_layouts, empty_layout, slot_layout });
}

impl SpecializedMeshPipeline for VegDrawPipeline {
    type Key = VegPipelineKey;

    fn specialize(&self, key: Self::Key, layout: &MeshVertexBufferLayoutRef) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let attributes = [Mesh::ATTRIBUTE_POSITION.at_shader_location(0), Mesh::ATTRIBUTE_NORMAL.at_shader_location(1), Mesh::ATTRIBUTE_UV_0.at_shader_location(2)];
        let vertex_layout = layout.0.get_layout(&attributes)?;
        let material = key.material;
        Ok(RenderPipelineDescriptor {
            label: Some("veg_draw_pipeline".into()),
            layout: vec![
                self.view_layout.clone(),
                self.kind_layouts[key.kind.index()].clone(),
                material.group2.clone().unwrap_or_else(|| self.empty_layout.clone()),
                self.slot_layout.clone(),
            ],
            vertex: VertexState { shader: material.shader.clone(), entry_point: Some(material.vertex_entry.into()), buffers: vec![vertex_layout], ..default() },
            primitive: PrimitiveState { cull_mode: material.cull_back.then_some(Face::Back), ..default() },
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(CompareFunction::GreaterEqual),
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState { count: key.samples, mask: !0, alpha_to_coverage_enabled: false },
            fragment: Some(FragmentState {
                shader: material.shader,
                entry_point: Some(material.fragment_entry.into()),
                targets: vec![Some(ColorTargetState { format: key.target_format, blend: None, write_mask: ColorWrites::ALL })],
                ..default()
            }),
            ..default()
        })
    }
}

#[derive(Clone)]
struct QueuedDraw {
    pipeline: CachedRenderPipelineId,
    mesh: bevy::asset::AssetId<Mesh>,
    kind: VegKind,
    slot: u32,
    index_format: IndexFormat,
    group2: Option<BindGroup>,
}

#[derive(Component)]
pub struct VegViewDraws(Vec<QueuedDraw>);

pub fn prepare_draws(
    mut commands: Commands,
    views: Query<(Entity, &ExtractedView, &Msaa), With<VegCamera>>,
    sets: Query<&VegDrawSet>,
    meshes: Res<RenderAssets<RenderMesh>>,
    allocator: Res<MeshAllocator>,
    cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedMeshPipelines<VegDrawPipeline>>,
    pipeline: Option<Res<VegDrawPipeline>>,
    gpu: Option<Res<VegGpu>>,
    environment: Option<Res<VegEnvironment>>,
    groups: Option<Res<VegMaterialGroups>>,
    queue: Res<RenderQueue>,
    mut written_args: Local<HashMap<(VegKind, u32), [u32; 3]>>,
) {
    let (Some(pipeline), Some(gpu)) = (pipeline, gpu) else { return };
    let environment = environment.map(|environment| *environment).unwrap_or_default();
    for (entity, view, msaa) in &views {
        queue.write_buffer(&gpu.view, 0, &environment.view_bytes(view));
        queue.write_buffer(&gpu.light, 0, &environment.light_bytes());
        let mut draws = Vec::new();
        for set in &sets {
            for tier in &set.tiers {
                let id = tier.mesh.id();
                let Some(mesh) = meshes.get(id) else { continue };
                let RenderMeshBufferInfo::Indexed { index_format, count } = mesh.buffer_info else { continue };
                let (Some(vertices), Some(indices)) = (allocator.mesh_vertex_slice(&id), allocator.mesh_index_slice(&id)) else { continue };
                let args = [count, indices.range.start, vertices.range.start];
                if written_args.insert((set.kind, tier.slot), args) != Some(args) {
                    let at = u64::from(tier.slot) * DRAW_ARGS_BYTES;
                    let buffer = &gpu.draws[set.kind.index()];
                    queue.write_buffer(buffer, at, &args[0].to_le_bytes());
                    queue.write_buffer(buffer, at + 8, &[args[1].to_le_bytes(), args[2].to_le_bytes(), 0u32.to_le_bytes()].concat());
                }
                let key = VegPipelineKey { target_format: view.target_format, samples: msaa.samples(), kind: set.kind, material: set.material.clone() };
                let Ok(pipeline_id) = pipelines.specialize(&cache, &pipeline, key, &mesh.layout) else { continue };
                let group2 = groups.as_ref().and_then(|groups| groups.0.get(&set.material.shader.id()).cloned());
                draws.push(QueuedDraw { pipeline: pipeline_id, mesh: id, kind: set.kind, slot: tier.slot, index_format, group2 });
            }
        }
        commands.entity(entity).insert(VegViewDraws(draws));
    }
}

pub fn render_vegetation(
    view: ViewQuery<(&ExtractedCamera, &ViewTarget, &ViewDepthStencilTexture, &VegViewDraws, Option<&MainPassResolutionOverride>)>,
    cache: Res<PipelineCache>,
    allocator: Res<MeshAllocator>,
    gpu: Option<Res<VegGpu>>,
    bindings: Option<Res<VegDrawBindings>>,
    config: Res<VegGpuConfig>,
    witness: Res<VegWitness>,
    mut ctx: RenderContext,
) {
    let (camera, target, depth, draws, resolution_override) = view.into_inner();
    let (Some(gpu), Some(bindings)) = (gpu, bindings) else { return };
    if draws.0.is_empty() {
        return;
    }
    let color_attachments = [Some(target.get_color_attachment())];
    let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("veg_draw"),
        color_attachments: &color_attachments,
        depth_stencil_attachment: Some(depth.get_attachment(StoreOp::Store)),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    if let Some(viewport) = Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override) {
        pass.set_camera_viewport(&viewport);
    }
    let mut issued = 0;
    for draw in &draws.0 {
        let Some(pipeline) = cache.get_render_pipeline(draw.pipeline) else { continue };
        let (Some(vertices), Some(indices)) = (allocator.mesh_vertex_slice(&draw.mesh), allocator.mesh_index_slice(&draw.mesh)) else { continue };
        pass.set_render_pipeline(pipeline);
        pass.set_bind_group(0, &bindings.view, &[]);
        pass.set_bind_group(1, &bindings.kinds[draw.kind.index()], &[]);
        pass.set_bind_group(2, draw.group2.as_ref().unwrap_or(&bindings.empty), &[]);
        pass.set_bind_group(3, &bindings.slot, &[config.slot_uniform_offset(draw.kind, draw.slot)]);
        pass.set_vertex_buffer(0, vertices.buffer.slice(..));
        pass.set_index_buffer(indices.buffer.slice(..), draw.index_format);
        pass.draw_indexed_indirect(&gpu.draws[draw.kind.index()], u64::from(draw.slot) * DRAW_ARGS_BYTES);
        issued += 1;
    }
    witness.draws.store(issued, Ordering::SeqCst);
}
