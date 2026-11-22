use super::buffers::{GRASS_PARAMS_BYTES, Packer, ROCK_PARAMS_BYTES, STATS_BYTES, TREE_PARAMS_BYTES, VegGpu, VegGpuConfig, VegKind};
use super::draw::VegCamera;
use bevy::asset::uuid_handle;
use bevy::prelude::*;
use bevy::render::RenderApp;
use bevy::render::extract_resource::ExtractResource;
use bevy::render::render_resource::{
    BindGroup, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferDescriptor, BufferUsages, CachedComputePipelineId,
    ComputePassDescriptor, ComputePipelineDescriptor, MapMode, PipelineCache, ShaderStages,
};
use bevy::render::renderer::{RenderContext, RenderDevice, RenderQueue};
use bevy::render::view::ExtractedView;
use bevy::shader::Shader;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

pub const CULL_SHADER: Handle<Shader> = uuid_handle!("6d1c4a52-0b7e-4f3a-9a58-2e8f61c7d3b4");
pub const CULL_SHADER_PATH: &str = "shaders/planet/cull_vegetation.wgsl";
pub const CULL_SOURCE: &str = include_str!("../../../assets/shaders/planet/cull_vegetation.wgsl");
pub const WORKGROUP_SIZE: u32 = 64;

const STAGING_IDLE: u8 = 0;
const STAGING_COPIED: u8 = 1;
const STAGING_MAPPING: u8 = 2;

const CULL_ENTRIES: [[&str; 3]; 3] = [
    ["reset_tree_draws", "cull_trees", "finish_tree_draws"],
    ["reset_rock_draws", "cull_rocks", "finish_rock_draws"],
    ["reset_grass_draws", "cull_grass", "finish_grass_draws"],
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TreeCullSettings {
    pub ring_enter_m: f32,
    pub ring_drop_m: f32,
    pub render_distance_m: f32,
    pub falloff_near_m: f32,
    pub falloff_floor: f32,
    pub hysteresis: f32,
    pub d1_m: f32,
    pub d2_m: f32,
    pub mesh_far_m: f32,
    pub mesh_reentry_m: f32,
    pub impostor_start_m: f32,
    pub shadow_distance_m: f32,
}

impl TreeCullSettings {
    pub fn reference(render_distance_m: f32) -> Self {
        let hysteresis = 0.12;
        let d1_m = (0.045 * render_distance_m).min(14.0);
        let d2_m = (0.11 * render_distance_m).min(35.0);
        let d3_m = render_distance_m.min((d2_m + 12.0).max(0.13 * render_distance_m));
        let cutoff_m = 0.92 * d3_m;
        Self {
            ring_enter_m: render_distance_m + 40.0,
            ring_drop_m: render_distance_m + 104.0,
            render_distance_m,
            falloff_near_m: 96.0,
            falloff_floor: 0.15,
            hysteresis,
            d1_m,
            d2_m,
            mesh_far_m: (d2_m * (1.0 + hysteresis)).max(cutoff_m + 3.0),
            mesh_reentry_m: cutoff_m + 0.5,
            impostor_start_m: cutoff_m - 3.0,
            shadow_distance_m: 35.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RockCullSettings {
    pub ring_enter_m: f32,
    pub ring_drop_m: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrassCullSettings {
    pub full_radius_m: f32,
    pub fade_m: f32,
    pub edge_cap_m: f32,
    pub near_m: f32,
    pub width_min: f32,
    pub width_span: f32,
    pub height_scale: f32,
}

#[derive(Resource, ExtractResource, Clone, Copy, Debug, PartialEq)]
#[extract_app(RenderApp)]
pub struct VegCullSettings {
    pub tree: TreeCullSettings,
    pub rock: RockCullSettings,
    pub grass: GrassCullSettings,
}

impl Default for VegCullSettings {
    fn default() -> Self {
        Self {
            tree: TreeCullSettings::reference(640.0),
            rock: RockCullSettings { ring_enter_m: 360.0, ring_drop_m: 424.0 },
            grass: GrassCullSettings { full_radius_m: 16.0, fade_m: 5.0, edge_cap_m: 80.0, near_m: 8.0, width_min: 2.268, width_span: 1.176, height_scale: 0.75 },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CullView {
    pub planes: [Vec4; 6],
    pub eye: Vec3,
}

pub fn frustum_planes(clip_from_world: Mat4) -> [Vec4; 6] {
    let (r0, r1, r2, r3) = (clip_from_world.row(0), clip_from_world.row(1), clip_from_world.row(2), clip_from_world.row(3));
    [r3 + r0, r3 - r0, r3 + r1, r3 - r1, r2, r3 - r2].map(|plane| {
        let length = plane.truncate().length();
        if length > 0.0 { plane / length } else { plane }
    })
}

impl CullView {
    pub fn new(eye: Vec3, clip_from_world: Mat4) -> Self {
        Self { planes: frustum_planes(clip_from_world).map(|plane| plane + Vec4::W * plane.truncate().dot(eye)), eye }
    }

    pub fn from_extracted(view: &ExtractedView) -> Self {
        let clip_from_world = view.clip_from_world.unwrap_or_else(|| view.clip_from_view * view.world_from_view.to_matrix().inverse());
        Self::new(view.world_from_view.translation(), clip_from_world)
    }

    fn write(&self, out: &mut Vec<u8>, instance_count: u32) {
        let mut packer = Packer(out);
        self.planes.iter().for_each(|plane| {
            packer.f32s(&plane.to_array());
        });
        packer.f32s(&self.eye.to_array()).u32s(&[instance_count]);
    }
}

pub fn tree_params_bytes(view: &CullView, settings: &TreeCullSettings, config: &VegGpuConfig, instance_count: u32) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(TREE_PARAMS_BYTES as usize);
    view.write(&mut bytes, instance_count);
    Packer(&mut bytes).u32s(&[config.tree_species, config.tree_geo_count(), config.slot_count(VegKind::Tree), config.impostor_slot()]).f32s(&[
        settings.ring_enter_m,
        settings.ring_drop_m,
        settings.render_distance_m,
        settings.falloff_near_m,
        settings.falloff_floor,
        settings.hysteresis,
        settings.d1_m,
        settings.d2_m,
        settings.mesh_far_m,
        settings.mesh_reentry_m,
        settings.impostor_start_m,
        settings.shadow_distance_m,
    ]);
    bytes
}

pub fn rock_params_bytes(view: &CullView, settings: &RockCullSettings, config: &VegGpuConfig, instance_count: u32) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(ROCK_PARAMS_BYTES as usize);
    view.write(&mut bytes, instance_count);
    Packer(&mut bytes).u32s(&[config.rock_kinds, config.rock_slot_capacity]).f32s(&[settings.ring_enter_m, settings.ring_drop_m]);
    bytes
}

pub fn grass_params_bytes(view: &CullView, settings: &GrassCullSettings, config: &VegGpuConfig, instance_count: u32) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(GRASS_PARAMS_BYTES as usize);
    view.write(&mut bytes, instance_count);
    Packer(&mut bytes).f32s(&[settings.full_radius_m, settings.fade_m, settings.edge_cap_m, settings.near_m]).u32s(&[config.grass_slot_capacity]).f32s(&[
        settings.width_min,
        settings.width_span,
        settings.height_scale,
    ]);
    bytes
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CullStats {
    pub overflow: u32,
    pub tree_mesh: u32,
    pub tree_impostor: u32,
    pub tree_shadow: u32,
    pub tree_resident: u32,
    pub tree_thinned: u32,
    pub tree_frustum_culled: u32,
    pub rock_visible: u32,
    pub grass_near: u32,
    pub grass_mid: u32,
}

impl CullStats {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let word = |index: usize| u32::from_le_bytes(bytes[index * 4..index * 4 + 4].try_into().unwrap());
        Self {
            overflow: word(0),
            tree_mesh: word(1),
            tree_impostor: word(2),
            tree_shadow: word(3),
            tree_resident: word(4),
            tree_thinned: word(5),
            tree_frustum_culled: word(6),
            rock_visible: word(7),
            grass_near: word(8),
            grass_mid: word(9),
        }
    }
}

#[derive(Resource, Clone, Default)]
pub struct VegWitness {
    pub dispatches: Arc<AtomicU32>,
    pub draws: Arc<AtomicU32>,
    pub stats: Arc<Mutex<Option<CullStats>>>,
    pub view: Arc<Mutex<Option<CullView>>>,
    pub readback: Arc<AtomicBool>,
}

#[cfg(test)]
impl VegWitness {
    pub fn dispatches(&self) -> u32 {
        self.dispatches.load(Ordering::SeqCst)
    }

    pub fn draws(&self) -> u32 {
        self.draws.load(Ordering::SeqCst)
    }

    pub fn latest_stats(&self) -> Option<CullStats> {
        *self.stats.lock().unwrap()
    }

    pub fn latest_view(&self) -> Option<CullView> {
        *self.view.lock().unwrap()
    }
}

#[derive(Resource)]
pub struct StatsStaging {
    buffer: Buffer,
    phase: Arc<AtomicU8>,
}

#[derive(Resource)]
pub struct VegCullPipelines {
    pub ids: [[CachedComputePipelineId; 3]; 3],
    pub groups: [BindGroup; 3],
}

#[derive(Clone, Copy)]
enum Binding {
    Uniform,
    ReadOnly,
    ReadWrite,
}

fn layout_entry(binding: u32, kind: Binding) -> BindGroupLayoutEntry {
    let ty = match kind {
        Binding::Uniform => BufferBindingType::Uniform,
        Binding::ReadOnly => BufferBindingType::Storage { read_only: true },
        Binding::ReadWrite => BufferBindingType::Storage { read_only: false },
    };
    BindGroupLayoutEntry { binding, visibility: ShaderStages::COMPUTE, ty: BindingType::Buffer { ty, has_dynamic_offset: false, min_binding_size: None }, count: None }
}

pub fn init_cull(mut commands: Commands, device: Res<RenderDevice>, cache: Res<PipelineCache>, config: Res<VegGpuConfig>) {
    let gpu = VegGpu::new(&device, &config);
    let tree: [(u32, Binding, &Buffer); 9] = [
        (0, Binding::Uniform, &gpu.cull_params[0]),
        (1, Binding::ReadOnly, &gpu.instances[0]),
        (2, Binding::ReadOnly, &gpu.chunks),
        (3, Binding::ReadOnly, &gpu.tree_bounds),
        (4, Binding::ReadWrite, &gpu.tree_state),
        (5, Binding::ReadOnly, &gpu.tree_slot_layout),
        (6, Binding::ReadWrite, &gpu.draws[0]),
        (7, Binding::ReadWrite, &gpu.visible[0]),
        (8, Binding::ReadWrite, &gpu.stats),
    ];
    let rock: [(u32, Binding, &Buffer); 8] = [
        (2, Binding::ReadOnly, &gpu.chunks),
        (8, Binding::ReadWrite, &gpu.stats),
        (10, Binding::Uniform, &gpu.cull_params[1]),
        (11, Binding::ReadOnly, &gpu.instances[1]),
        (12, Binding::ReadWrite, &gpu.rock_state),
        (13, Binding::ReadOnly, &gpu.rock_bounds),
        (14, Binding::ReadWrite, &gpu.draws[1]),
        (15, Binding::ReadWrite, &gpu.visible[1]),
    ];
    let grass: [(u32, Binding, &Buffer); 6] = [
        (2, Binding::ReadOnly, &gpu.chunks),
        (8, Binding::ReadWrite, &gpu.stats),
        (20, Binding::Uniform, &gpu.cull_params[2]),
        (21, Binding::ReadOnly, &gpu.instances[2]),
        (22, Binding::ReadWrite, &gpu.draws[2]),
        (23, Binding::ReadWrite, &gpu.visible[2]),
    ];
    let specs: [&[(u32, Binding, &Buffer)]; 3] = [&tree, &rock, &grass];
    let layouts = specs.map(|spec| {
        let entries: Vec<_> = spec.iter().map(|(binding, kind, _)| layout_entry(*binding, *kind)).collect();
        BindGroupLayoutDescriptor::new("veg_cull_layout", &entries)
    });
    let groups = std::array::from_fn(|index| {
        let entries: Vec<_> = specs[index].iter().map(|(binding, _, buffer)| BindGroupEntry { binding: *binding, resource: buffer.as_entire_binding() }).collect();
        device.create_bind_group("veg_cull_group", &cache.get_bind_group_layout(&layouts[index]), &entries)
    });
    let ids = std::array::from_fn(|kind| {
        std::array::from_fn(|step| {
            cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some(CULL_ENTRIES[kind][step].into()),
                layout: vec![layouts[kind].clone()],
                shader: CULL_SHADER,
                entry_point: Some(CULL_ENTRIES[kind][step].into()),
                ..default()
            })
        })
    });
    let staging = device.create_buffer(&BufferDescriptor { label: Some("veg_stats_staging"), size: STATS_BYTES, usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST, mapped_at_creation: false });
    commands.insert_resource(StatsStaging { buffer: staging, phase: Arc::new(AtomicU8::new(STAGING_IDLE)) });
    commands.insert_resource(VegCullPipelines { ids, groups });
    commands.insert_resource(gpu);
}

pub fn prepare_cull(
    views: Query<&ExtractedView, With<VegCamera>>,
    settings: Option<Res<VegCullSettings>>,
    gpu: Option<ResMut<VegGpu>>,
    config: Res<VegGpuConfig>,
    witness: Res<VegWitness>,
    queue: Res<RenderQueue>,
) {
    let (Some(mut gpu), Some(view)) = (gpu, views.iter().next()) else { return };
    let settings = settings.map(|settings| *settings).unwrap_or_default();
    let view = CullView::from_extracted(view);
    let counts = gpu.instance_counts;
    queue.write_buffer(&gpu.cull_params[0], 0, &tree_params_bytes(&view, &settings.tree, &config, counts[0]));
    queue.write_buffer(&gpu.cull_params[1], 0, &rock_params_bytes(&view, &settings.rock, &config, counts[1]));
    queue.write_buffer(&gpu.cull_params[2], 0, &grass_params_bytes(&view, &settings.grass, &config, counts[2]));
    gpu.view_ready = true;
    *witness.view.lock().unwrap() = Some(view);
}

pub fn poll_stats(staging: Option<Res<StatsStaging>>, witness: Res<VegWitness>) {
    let Some(staging) = staging else { return };
    if staging.phase.compare_exchange(STAGING_COPIED, STAGING_MAPPING, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        return;
    }
    let (buffer, phase, latest) = (staging.buffer.clone(), staging.phase.clone(), witness.stats.clone());
    staging.buffer.slice(..).map_async(MapMode::Read, move |result| {
        if result.is_ok() {
            let snapshot = CullStats::from_bytes(&buffer.slice(..).get_mapped_range().unwrap());
            *latest.lock().unwrap() = Some(snapshot);
            buffer.unmap();
        }
        phase.store(STAGING_IDLE, Ordering::SeqCst);
    });
}

fn groups_for(count: u32) -> u32 {
    count.div_ceil(WORKGROUP_SIZE)
}

pub fn run_cull(mut ctx: RenderContext, gpu: Option<Res<VegGpu>>, pipelines: Option<Res<VegCullPipelines>>, config: Res<VegGpuConfig>, cache: Res<PipelineCache>, witness: Res<VegWitness>, staging: Option<Res<StatsStaging>>) {
    let (Some(gpu), Some(pipelines)) = (gpu, pipelines) else { return };
    if !gpu.view_ready {
        return;
    }
    let compute: Vec<_> = pipelines.ids.iter().flatten().filter_map(|id| cache.get_compute_pipeline(*id)).collect();
    if compute.len() != 9 {
        return;
    }
    let encoder = ctx.command_encoder();
    encoder.clear_buffer(&gpu.stats, 0, None);
    {
        let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor { label: Some("veg_cull"), timestamp_writes: None });
        for kind in VegKind::ALL {
            let slots = groups_for(config.slot_count(kind));
            let instances = groups_for(gpu.instance_counts[kind.index()]);
            pass.set_bind_group(0, &pipelines.groups[kind.index()], &[]);
            pass.set_pipeline(compute[kind.index() * 3]);
            pass.dispatch_workgroups(slots, 1, 1);
            if instances > 0 {
                pass.set_pipeline(compute[kind.index() * 3 + 1]);
                pass.dispatch_workgroups(instances, 1, 1);
            }
            pass.set_pipeline(compute[kind.index() * 3 + 2]);
            pass.dispatch_workgroups(slots, 1, 1);
        }
    }
    witness.dispatches.fetch_add(1, Ordering::SeqCst);
    if let Some(staging) = staging
        && witness.readback.load(Ordering::SeqCst)
        && staging.phase.compare_exchange(STAGING_IDLE, STAGING_COPIED, Ordering::SeqCst, Ordering::SeqCst).is_ok()
    {
        encoder.copy_buffer_to_buffer(&gpu.stats, 0, &staging.buffer, 0, STATS_BYTES);
    }
}
