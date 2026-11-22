use super::assets::TreeAssets;
use super::{CAPTURE_SHADER, IMPOSTOR_SHADER, TREE_SHADER, TreeRenderData, TreesWitness};
use crate::planet_veg::gpu::cull::VegCullSettings;
use crate::planet_veg::gpu::draw::VegMaterialGroups;
use bevy::mesh::{VertexBufferLayout, VertexFormat as MeshVertexFormat};
use bevy::prelude::*;
use bevy::render::render_resource::{
    BindGroup, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBinding, BufferBindingType, BufferInitDescriptor,
    BufferUsages, CachedRenderPipelineId, ColorTargetState, ColorWrites, CompareFunction, DepthBiasState, DepthStencilState, Extent3d, FilterMode, FragmentState, IndexFormat, LoadOp, MipmapFilterMode,
    MultisampleState, Operations, PipelineCache, PrimitiveState, RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipelineDescriptor, SamplerBindingType,
    Sampler, SamplerDescriptor, ShaderStages, StencilState, StoreOp, Texture, TextureDataOrder, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureViewDescriptor,
    TextureViewDimension, VertexAttribute, VertexState, VertexStepMode, TextureView,
};
use bevy::render::renderer::{RenderContext, RenderDevice, RenderQueue};
use std::num::NonZeroU64;
use std::sync::atomic::Ordering;

pub const SPRITES_PER_SIDE: u32 = 8;
pub const SPRITE_PX: u32 = 64;
pub const ATLAS_GRID_SIDE: u32 = 4;
pub const ATLAS_PX: u32 = ATLAS_GRID_SIDE * SPRITES_PER_SIDE * SPRITE_PX;
pub const ATLAS_MIPS: u32 = 12;
pub const ALPHA_CLAMP: f32 = 0.4;
pub const IMPOSTOR_FADE_BAND_M: f32 = 3.0;
pub const IMPOSTOR_ANISOTROPY: u16 = 4;
const CELL_STRIDE: u64 = 256;
const CELL_BYTES: u64 = 32;
const VERTEX_STRIDE: u64 = 32;
const GRAPHICS: ShaderStages = ShaderStages::VERTEX_FRAGMENT;

#[derive(Clone)]
pub struct AtlasTextures {
    pub albedo: Texture,
    pub normal_depth: Texture,
}

struct BakeGpu {
    capture: CachedRenderPipelineId,
    mip: CachedRenderPipelineId,
    vertices: Buffer,
    indices: Buffer,
    index_counts: Vec<u32>,
    cell_group: BindGroup,
    empty_group: BindGroup,
    material_group: BindGroup,
    depth: Texture,
}

#[derive(Resource)]
pub struct TreeGpu {
    pub atlas: AtlasTextures,
    impostor_params: Buffer,
    bake: Option<BakeGpu>,
}

fn entry(binding: u32, ty: BindingType) -> BindGroupLayoutEntry {
    BindGroupLayoutEntry { binding, visibility: GRAPHICS, ty, count: None }
}

fn texture_entry(binding: u32, dimension: TextureViewDimension) -> BindGroupLayoutEntry {
    entry(binding, BindingType::Texture { sample_type: TextureSampleType::Float { filterable: true }, view_dimension: dimension, multisampled: false })
}

fn sampler_entry(binding: u32) -> BindGroupLayoutEntry {
    entry(binding, BindingType::Sampler(SamplerBindingType::Filtering))
}

fn uniform_entry(binding: u32, dynamic: bool) -> BindGroupLayoutEntry {
    entry(binding, BindingType::Buffer { ty: BufferBindingType::Uniform, has_dynamic_offset: dynamic, min_binding_size: None })
}

pub fn material_layout() -> BindGroupLayoutDescriptor {
    BindGroupLayoutDescriptor::new(
        "veg_tree_material",
        &[texture_entry(0, TextureViewDimension::D2Array), texture_entry(1, TextureViewDimension::D2Array), sampler_entry(2), sampler_entry(3), uniform_entry(4, false)],
    )
}

pub fn impostor_layout() -> BindGroupLayoutDescriptor {
    BindGroupLayoutDescriptor::new(
        "veg_impostor_material",
        &[texture_entry(0, TextureViewDimension::D2), texture_entry(1, TextureViewDimension::D2), sampler_entry(2), uniform_entry(3, false), sampler_entry(4)],
    )
}

fn cell_layout() -> BindGroupLayoutDescriptor {
    BindGroupLayoutDescriptor::new("veg_capture_cell", &[uniform_entry(0, true)])
}

fn empty_layout() -> BindGroupLayoutDescriptor {
    BindGroupLayoutDescriptor::new("veg_capture_empty", &[])
}

fn mip_layout() -> BindGroupLayoutDescriptor {
    BindGroupLayoutDescriptor::new("veg_capture_mip", &[texture_entry(1, TextureViewDimension::D2), texture_entry(2, TextureViewDimension::D2)])
}

fn uniform_buffer(device: &RenderDevice, label: &str, contents: &[u8]) -> Buffer {
    device.create_buffer_with_data(&BufferInitDescriptor { label: Some(label), contents, usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST })
}

fn array_view(device: &RenderDevice, queue: &RenderQueue, label: &str, size_px: u32, layers: u32, mips: u32, data: &[u8]) -> TextureView {
    let texture = device.create_texture_with_data(
        queue,
        &TextureDescriptor {
            label: Some(label),
            size: Extent3d { width: size_px, height: size_px, depth_or_array_layers: layers },
            mip_level_count: mips,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        TextureDataOrder::LayerMajor,
        data,
    );
    texture.create_view(&TextureViewDescriptor { dimension: Some(TextureViewDimension::D2Array), ..default() })
}

fn atlas_texture(device: &RenderDevice, label: &str) -> Texture {
    device.create_texture(&TextureDescriptor {
        label: Some(label),
        size: Extent3d { width: ATLAS_PX, height: ATLAS_PX, depth_or_array_layers: 1 },
        mip_level_count: ATLAS_MIPS,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8Unorm,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

fn sampler(device: &RenderDevice, address: bevy::render::render_resource::AddressMode, filter: FilterMode, mipmap: MipmapFilterMode, anisotropy: u16) -> Sampler {
    device.create_sampler(&SamplerDescriptor {
        address_mode_u: address,
        address_mode_v: address,
        address_mode_w: address,
        mag_filter: filter,
        min_filter: filter,
        mipmap_filter: mipmap,
        anisotropy_clamp: anisotropy,
        ..default()
    })
}

pub fn impostor_params_bytes(near_cutoff_m: f32) -> Vec<u8> {
    [SPRITES_PER_SIDE as f32, ALPHA_CLAMP, ATLAS_GRID_SIDE as f32, near_cutoff_m, IMPOSTOR_FADE_BAND_M, 0.0, 0.0, 0.0].iter().flat_map(|value| value.to_le_bytes()).collect()
}

fn capture_geometry(assets: &TreeAssets) -> (Vec<u8>, Vec<u8>, Vec<u32>) {
    let (mut vertices, mut indices, mut index_counts) = (Vec::new(), Vec::new(), Vec::new());
    let mut base_vertex = 0u32;
    for tiers in &assets.tiers {
        let geometry = &tiers[0];
        for ((position, normal), uv) in geometry.positions.iter().zip(&geometry.normals).zip(&geometry.uvs) {
            [position[0], position[1], position[2], normal[0], normal[1], normal[2], uv[0], uv[1]].iter().for_each(|value| vertices.extend_from_slice(&value.to_le_bytes()));
        }
        index_counts.push(geometry.indices.len() as u32);
        geometry.indices.iter().for_each(|index| indices.extend_from_slice(&(index + base_vertex).to_le_bytes()));
        base_vertex += geometry.positions.len() as u32;
    }
    (vertices, indices, index_counts)
}

fn cell_bytes(assets: &TreeAssets) -> Vec<u8> {
    let sides = SPRITES_PER_SIDE as usize;
    let mut bytes = vec![0u8; assets.species_count() * sides * sides * CELL_STRIDE as usize];
    for (species, sphere) in assets.spheres.iter().enumerate() {
        for row in 0..sides {
            for column in 0..sides {
                let at = ((species * sides + row) * sides + column) * CELL_STRIDE as usize;
                let words = [sphere[0].to_bits(), sphere[1].to_bits(), sphere[2].to_bits(), sphere[3].to_bits(), (column as f32).to_bits(), (row as f32).to_bits(), species as u32, ((SPRITES_PER_SIDE - 1) as f32).to_bits()];
                words.iter().enumerate().for_each(|(word, value)| bytes[at + word * 4..at + word * 4 + 4].copy_from_slice(&value.to_le_bytes()));
            }
        }
    }
    bytes
}

fn capture_pipeline(cache: &PipelineCache) -> CachedRenderPipelineId {
    let attributes = vec![
        VertexAttribute { format: MeshVertexFormat::Float32x3.into(), offset: 0, shader_location: 0 },
        VertexAttribute { format: MeshVertexFormat::Float32x3.into(), offset: 12, shader_location: 1 },
        VertexAttribute { format: MeshVertexFormat::Float32x2.into(), offset: 24, shader_location: 2 },
    ];
    let target = Some(ColorTargetState { format: TextureFormat::Rgba8Unorm, blend: None, write_mask: ColorWrites::ALL });
    cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some("veg_capture".into()),
        layout: vec![cell_layout(), empty_layout(), material_layout()],
        vertex: VertexState {
            shader: CAPTURE_SHADER,
            entry_point: Some("vs_capture".into()),
            buffers: vec![VertexBufferLayout { array_stride: VERTEX_STRIDE, step_mode: VertexStepMode::Vertex, attributes }],
            ..default()
        },
        primitive: PrimitiveState::default(),
        depth_stencil: Some(DepthStencilState { format: TextureFormat::Depth32Float, depth_write_enabled: Some(true), depth_compare: Some(CompareFunction::Less), stencil: StencilState::default(), bias: DepthBiasState::default() }),
        multisample: MultisampleState::default(),
        fragment: Some(FragmentState { shader: CAPTURE_SHADER, entry_point: Some("fs_capture".into()), targets: vec![target.clone(), target], ..default() }),
        ..default()
    })
}

fn mip_pipeline(cache: &PipelineCache) -> CachedRenderPipelineId {
    let target = Some(ColorTargetState { format: TextureFormat::Rgba8Unorm, blend: None, write_mask: ColorWrites::ALL });
    cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some("veg_capture_mip".into()),
        layout: vec![mip_layout()],
        vertex: VertexState { shader: CAPTURE_SHADER, entry_point: Some("vs_mip".into()), ..default() },
        fragment: Some(FragmentState { shader: CAPTURE_SHADER, entry_point: Some("fs_mip".into()), targets: vec![target.clone(), target], ..default() }),
        ..default()
    })
}

pub fn create_tree_gpu(
    mut commands: Commands,
    data: Option<Res<TreeRenderData>>,
    existing: Option<Res<TreeGpu>>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    groups: Option<ResMut<VegMaterialGroups>>,
) {
    let (Some(data), None, Some(mut groups)) = (data, existing, groups) else { return };
    let assets = &data.assets;
    let bark = array_view(&device, &queue, "veg_bark_textures", assets.bark.size_px, assets.bark.layers, assets.bark.mips, &assets.bark.data);
    let leaf = array_view(&device, &queue, "veg_leaf_textures", assets.leaf.size_px, assets.leaf.layers, assets.leaf.mips, &assets.leaf.data);
    let repeat = sampler(&device, bevy::render::render_resource::AddressMode::Repeat, FilterMode::Linear, MipmapFilterMode::Linear, 1);
    let clamp = sampler(&device, bevy::render::render_resource::AddressMode::ClampToEdge, FilterMode::Linear, MipmapFilterMode::Linear, 1);
    let table = uniform_buffer(&device, "veg_tree_material_table", &assets.table_bytes());
    let material_group = device.create_bind_group(
        "veg_tree_material_group",
        &cache.get_bind_group_layout(&material_layout()),
        &[
            BindGroupEntry { binding: 0, resource: BindingResource::TextureView(&bark) },
            BindGroupEntry { binding: 1, resource: BindingResource::TextureView(&leaf) },
            BindGroupEntry { binding: 2, resource: BindingResource::Sampler(&repeat) },
            BindGroupEntry { binding: 3, resource: BindingResource::Sampler(&clamp) },
            BindGroupEntry { binding: 4, resource: table.as_entire_binding() },
        ],
    );
    groups.0.insert(TREE_SHADER.id(), material_group.clone());

    let atlas = AtlasTextures { albedo: atlas_texture(&device, "veg_atlas_albedo"), normal_depth: atlas_texture(&device, "veg_atlas_normal_depth") };
    let albedo_view = atlas.albedo.create_view(&TextureViewDescriptor::default());
    let normal_view = atlas.normal_depth.create_view(&TextureViewDescriptor::default());
    let linear = sampler(&device, bevy::render::render_resource::AddressMode::ClampToEdge, FilterMode::Linear, MipmapFilterMode::Linear, IMPOSTOR_ANISOTROPY);
    let nearest = sampler(&device, bevy::render::render_resource::AddressMode::ClampToEdge, FilterMode::Nearest, MipmapFilterMode::Nearest, 1);
    let impostor_params = uniform_buffer(&device, "veg_impostor_params", &impostor_params_bytes(0.0));
    let impostor_group = device.create_bind_group(
        "veg_impostor_group",
        &cache.get_bind_group_layout(&impostor_layout()),
        &[
            BindGroupEntry { binding: 0, resource: BindingResource::TextureView(&albedo_view) },
            BindGroupEntry { binding: 1, resource: BindingResource::TextureView(&normal_view) },
            BindGroupEntry { binding: 2, resource: BindingResource::Sampler(&linear) },
            BindGroupEntry { binding: 3, resource: impostor_params.as_entire_binding() },
            BindGroupEntry { binding: 4, resource: BindingResource::Sampler(&nearest) },
        ],
    );
    groups.0.insert(IMPOSTOR_SHADER.id(), impostor_group);

    let bake = data.bake_atlas.then(|| {
        let (vertices, indices, index_counts) = capture_geometry(assets);
        let cells = device.create_buffer_with_data(&BufferInitDescriptor { label: Some("veg_capture_cells"), contents: &cell_bytes(assets), usage: BufferUsages::UNIFORM });
        let cell_group = device.create_bind_group(
            "veg_capture_cell_group",
            &cache.get_bind_group_layout(&cell_layout()),
            &[BindGroupEntry { binding: 0, resource: BindingResource::Buffer(BufferBinding { buffer: &cells, offset: 0, size: NonZeroU64::new(CELL_BYTES) }) }],
        );
        let empty_group = device.create_bind_group("veg_capture_empty_group", &cache.get_bind_group_layout(&empty_layout()), &[]);
        BakeGpu {
            capture: capture_pipeline(&cache),
            mip: mip_pipeline(&cache),
            vertices: device.create_buffer_with_data(&BufferInitDescriptor { label: Some("veg_capture_vertices"), contents: &vertices, usage: BufferUsages::VERTEX }),
            indices: device.create_buffer_with_data(&BufferInitDescriptor { label: Some("veg_capture_indices"), contents: &indices, usage: BufferUsages::INDEX }),
            index_counts,
            cell_group,
            empty_group,
            material_group,
            depth: device.create_texture(&TextureDescriptor {
                label: Some("veg_capture_depth"),
                size: Extent3d { width: ATLAS_PX, height: ATLAS_PX, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Depth32Float,
                usage: TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            }),
        }
    });
    commands.insert_resource(TreeGpu { atlas, impostor_params, bake });
}

pub fn write_impostor_params(gpu: Option<Res<TreeGpu>>, settings: Option<Res<VegCullSettings>>, queue: Res<RenderQueue>) {
    let (Some(gpu), Some(settings)) = (gpu, settings) else { return };
    queue.write_buffer(&gpu.impostor_params, 0, &impostor_params_bytes(settings.tree.impostor_start_m + IMPOSTOR_FADE_BAND_M));
}

fn mip_view(texture: &Texture, level: u32) -> TextureView {
    texture.create_view(&TextureViewDescriptor { base_mip_level: level, mip_level_count: Some(1), ..default() })
}

fn clear_attachment(view: &TextureView) -> Option<RenderPassColorAttachment<'_>> {
    Some(RenderPassColorAttachment { view, depth_slice: None, resolve_target: None, ops: Operations { load: LoadOp::Clear(default()), store: StoreOp::Store } })
}

fn sprite_rect(species: u32, row: u32, column: u32) -> (f32, f32) {
    let (tile_column, tile_row) = (species % ATLAS_GRID_SIDE, species / ATLAS_GRID_SIDE);
    let x = (tile_column * SPRITES_PER_SIDE + column) * SPRITE_PX;
    let top = ATLAS_PX - (tile_row * SPRITES_PER_SIDE + row + 1) * SPRITE_PX;
    (x as f32, top as f32)
}

pub fn run_atlas_bake(mut ctx: RenderContext, gpu: Option<ResMut<TreeGpu>>, cache: Res<PipelineCache>, witness: Res<TreesWitness>) {
    let Some(mut gpu) = gpu else { return };
    let ready = gpu.bake.as_ref().is_some_and(|bake| cache.get_render_pipeline(bake.capture).is_some() && cache.get_render_pipeline(bake.mip).is_some());
    if !ready {
        return;
    }
    let Some(bake) = gpu.bake.take() else { return };
    let (Some(capture), Some(mip)) = (cache.get_render_pipeline(bake.capture), cache.get_render_pipeline(bake.mip)) else { return };
    let albedo = mip_view(&gpu.atlas.albedo, 0);
    let normal_depth = mip_view(&gpu.atlas.normal_depth, 0);
    let depth = bake.depth.create_view(&TextureViewDescriptor::default());
    {
        let attachments = [clear_attachment(&albedo), clear_attachment(&normal_depth)];
        let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("veg_atlas_capture"),
            color_attachments: &attachments,
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment { view: &depth, depth_ops: Some(Operations { load: LoadOp::Clear(1.0), store: StoreOp::Discard }), stencil_ops: None }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_render_pipeline(capture);
        pass.set_bind_group(1, &bake.empty_group, &[]);
        pass.set_bind_group(2, &bake.material_group, &[]);
        pass.set_vertex_buffer(0, bake.vertices.slice(..));
        pass.set_index_buffer(bake.indices.slice(..), IndexFormat::Uint32);
        let mut first_index = 0;
        for (species, index_count) in bake.index_counts.iter().enumerate() {
            for row in 0..SPRITES_PER_SIDE {
                for column in 0..SPRITES_PER_SIDE {
                    let (x, y) = sprite_rect(species as u32, row, column);
                    pass.set_viewport(x, y, SPRITE_PX as f32, SPRITE_PX as f32, 0.0, 1.0);
                    pass.set_scissor_rect(x as u32, y as u32, SPRITE_PX, SPRITE_PX);
                    let cell = (species as u32 * SPRITES_PER_SIDE + row) * SPRITES_PER_SIDE + column;
                    pass.set_bind_group(0, &bake.cell_group, &[cell * CELL_STRIDE as u32]);
                    pass.draw_indexed(first_index..first_index + index_count, 0, 0..1);
                }
            }
            first_index += index_count;
        }
    }
    let mip_layout = cache.get_bind_group_layout(&mip_layout());
    for level in 1..ATLAS_MIPS {
        let sources = [mip_view(&gpu.atlas.albedo, level - 1), mip_view(&gpu.atlas.normal_depth, level - 1)];
        let group = ctx.render_device().create_bind_group(
            "veg_capture_mip_group",
            &mip_layout,
            &[BindGroupEntry { binding: 1, resource: BindingResource::TextureView(&sources[0]) }, BindGroupEntry { binding: 2, resource: BindingResource::TextureView(&sources[1]) }],
        );
        let targets = [mip_view(&gpu.atlas.albedo, level), mip_view(&gpu.atlas.normal_depth, level)];
        let attachments = [clear_attachment(&targets[0]), clear_attachment(&targets[1])];
        let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("veg_atlas_mip"),
            color_attachments: &attachments,
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_render_pipeline(mip);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
    }
    witness.baked.store(true, Ordering::SeqCst);
}
