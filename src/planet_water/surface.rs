use super::leaves::water_instances;
use super::mesh::{WaterMesh, build_water_mesh};
use super::waves::{DEFAULT_AMPLITUDE, DEFAULT_CHOPPINESS, DEFAULT_FOAM, sea_height_mean, wave_tile_offset};
use crate::height::Planet;
use crate::planet_lod::guard::note_build;
use crate::planet_lod::origin::{FloatingOrigin, rebase_system};
use crate::planet_lod::quadtree::{NodeKey, QuadtreeConfig};
use crate::planet_lod::stream::SelectedLeaves;
use bevy::asset::{RenderAssetUsages, uuid_handle};
use bevy::camera::visibility::NoFrustumCulling;
use bevy::math::DVec3;
use bevy::mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey, ScreenSpaceTransmission};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError};
use bevy::shader::{Shader, ShaderRef};
use bevy::tasks::{AsyncComputeTaskPool, Task, futures::check_ready};
use std::sync::Arc;

pub const UNDERWATER_BELOW_M: f64 = 2.0;
const HAZE_MUL: f32 = 0.4;
const EXPOSURE: f32 = 1.0;
const WATER_SHADER: Handle<Shader> = uuid_handle!("3b8f6c1e-94a2-4d0b-8c35-6a1d2e7f4b90");
const WATER_SHADER_PATH: &str = "shaders/planet/water.wgsl";
const VERTEX_ENTRY: &str = "water_vertex";
const FRAGMENT_ENTRY: &str = "water_fragment";
const CENSUS_ENTRY: &str = "water_census_fragment";

#[derive(Clone, Copy, Debug, PartialEq, ShaderType)]
pub struct WaterParams {
    pub origin_in_render: Vec3,
    pub time: f32,
    pub camera_in_render: Vec3,
    pub sea_mean: f32,
    pub sun_dir: Vec3,
    pub planet_radius: f32,
    pub cam_mod: Vec3,
    pub amp: f32,
    pub cam_dir: Vec3,
    pub choppy: f32,
    pub cam_alt: f32,
    pub foam: f32,
    pub haze_mul: f32,
    pub underwater: f32,
    pub viewport: Vec2,
    pub exposure: f32,
    pub aces: f32,
    pub census_mode: f32,
}

impl Default for WaterParams {
    fn default() -> Self {
        Self {
            origin_in_render: Vec3::ZERO,
            time: 0.0,
            camera_in_render: Vec3::ZERO,
            sea_mean: 0.0,
            sun_dir: Vec3::Y,
            planet_radius: 6_360_000.0,
            cam_mod: Vec3::ZERO,
            amp: DEFAULT_AMPLITUDE as f32,
            cam_dir: Vec3::Y,
            choppy: DEFAULT_CHOPPINESS as f32,
            cam_alt: 0.0,
            foam: DEFAULT_FOAM as f32,
            haze_mul: HAZE_MUL,
            underwater: 0.0,
            viewport: Vec2::ONE,
            exposure: EXPOSURE,
            aces: 1.0,
            census_mode: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct WaterKey {
    pub census: bool,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
#[bind_group_data(WaterKey)]
pub struct WaterMaterial {
    #[uniform(0)]
    pub params: WaterParams,
    pub key: WaterKey,
}

impl From<&WaterMaterial> for WaterKey {
    fn from(material: &WaterMaterial) -> Self {
        material.key
    }
}

impl Material for WaterMaterial {
    fn vertex_shader() -> ShaderRef {
        ShaderRef::Handle(WATER_SHADER)
    }

    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(WATER_SHADER)
    }

    fn reads_view_transmission_texture(&self) -> bool {
        true
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        descriptor.vertex.entry_point = Some(VERTEX_ENTRY.into());
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.entry_point = Some(if key.bind_group_data.census { CENSUS_ENTRY } else { FRAGMENT_ENTRY }.into());
        }
        Ok(())
    }
}

#[derive(Resource)]
pub struct WaterMaterialHandle(pub Handle<WaterMaterial>);

#[derive(Resource, Clone, Copy, Debug)]
pub struct WaterFrame {
    pub centre: DVec3,
    pub radius: f64,
}

#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct WaterClock(pub f32);

#[derive(Resource, Clone, Copy, Debug)]
pub struct WaterOcean {
    pub amplitude: f64,
    pub choppiness: f64,
    pub foam: f64,
}

impl Default for WaterOcean {
    fn default() -> Self {
        Self { amplitude: DEFAULT_AMPLITUDE, choppiness: DEFAULT_CHOPPINESS, foam: DEFAULT_FOAM }
    }
}

#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct WaterSurface {
    pub origin: DVec3,
}

pub fn water_shader() -> Shader {
    let display = include_str!("../../assets/shaders/planet/atmosphere_display.wgsl");
    let water = include_str!("../../assets/shaders/planet/water.wgsl");
    Shader::from_wgsl(format!("{display}\n{water}"), WATER_SHADER_PATH)
}

pub fn framed(
    mut params: WaterParams,
    frame: &WaterFrame,
    origin: &FloatingOrigin,
    camera_local: Vec3,
    sun_forward: Vec3,
    surface_origin: DVec3,
    viewport: Vec2,
) -> WaterParams {
    let camera = origin.to_world(camera_local) - frame.centre;
    params.origin_in_render = origin.to_local(surface_origin + frame.centre);
    params.camera_in_render = camera_local;
    params.sun_dir = -sun_forward.normalize_or_zero();
    params.planet_radius = frame.radius as f32;
    params.cam_mod = DVec3::new(wave_tile_offset(camera.x), wave_tile_offset(camera.y), wave_tile_offset(camera.z)).as_vec3();
    params.cam_dir = camera.normalize_or_zero().as_vec3();
    params.cam_alt = (camera.length() - frame.radius) as f32;
    params.underwater = f32::from(camera.length() < frame.radius - UNDERWATER_BELOW_M);
    params.viewport = viewport;
    params
}

pub fn water_mesh_asset(mesh: WaterMesh) -> Mesh {
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, mesh.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, mesh.dirs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, mesh.land)
        .with_inserted_indices(Indices::U32(mesh.indices))
}

pub fn build_water<'a>(planet: &Planet, radius: f64, leaves: impl IntoIterator<Item = &'a NodeKey>, origin: DVec3) -> WaterMesh {
    note_build();
    let instances = water_instances(leaves, |dir| planet.height(dir.to_array(), radius));
    build_water_mesh(&instances, radius, origin)
}

#[derive(Resource)]
struct Source {
    planet: Arc<Planet>,
    radius: f64,
}

#[derive(Resource, Default)]
struct WaterStreamer {
    generation: u64,
    live: Option<Entity>,
    pending: Option<Task<(DVec3, Mesh)>>,
}

#[derive(Resource)]
struct SeaMean {
    baking: Option<Task<f64>>,
    value: Option<f64>,
}

fn tick_clock(time: Res<Time>, mut clock: ResMut<WaterClock>) {
    clock.0 += time.delta_secs();
}

fn poll_sea_mean(mut mean: ResMut<SeaMean>) {
    let Some(task) = mean.baking.as_mut() else { return };
    if let Some(value) = check_ready(task) {
        mean.baking = None;
        mean.value = Some(value);
    }
}

fn start_water(source: Res<Source>, mean: Res<SeaMean>, selection: Res<SelectedLeaves>, mut streamer: ResMut<WaterStreamer>) {
    crate::frame_clock::timed(5, || {
    if mean.value.is_none() || streamer.pending.is_some() || streamer.generation == selection.generation {
        return;
    }
    let (planet, radius, keys, position) = (source.planet.clone(), source.radius, selection.keys.clone(), selection.camera);
    streamer.generation = selection.generation;
    streamer.pending = Some(AsyncComputeTaskPool::get().spawn(async move { (position, water_mesh_asset(build_water(&planet, radius, &keys, position))) }));
});
}

fn finish_water(
    mut commands: Commands,
    handle: Res<WaterMaterialHandle>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut surface: ResMut<WaterSurface>,
    mut streamer: ResMut<WaterStreamer>,
) {
    crate::frame_clock::timed(6, || {
    let Some(task) = streamer.pending.as_mut() else { return };
    let Some((origin, mesh)) = check_ready(task) else { return };
    streamer.pending = None;
    surface.origin = origin;
    let entity = commands.spawn((Mesh3d(meshes.add(mesh)), MeshMaterial3d(handle.0.clone()), Transform::IDENTITY, NoFrustumCulling)).id();
    if let Some(old) = streamer.live.replace(entity) {
        commands.entity(old).despawn();
    }
});
}

fn update_params(
    frame: Res<WaterFrame>,
    origin: Res<FloatingOrigin>,
    surface: Res<WaterSurface>,
    clock: Res<WaterClock>,
    ocean: Res<WaterOcean>,
    mean: Res<SeaMean>,
    camera: Query<(&Transform, &Camera), With<Camera3d>>,
    sun: Query<&GlobalTransform, With<DirectionalLight>>,
    handle: Res<WaterMaterialHandle>,
    mut materials: ResMut<Assets<WaterMaterial>>,
) {
    crate::frame_clock::timed(7, || {
    let (Ok((transform, camera)), Some(material)) = (camera.single(), materials.get(&handle.0)) else { return };
    let current = material.params;
    let sun_forward = sun.single().map_or(-current.sun_dir, |sun| sun.forward().as_vec3());
    let viewport = camera.physical_viewport_size().map_or(Vec2::ONE, |size| size.as_vec2());
    let mut params = framed(current, &frame, &origin, transform.translation, sun_forward, surface.origin, viewport);
    params.time = clock.0;
    params.amp = ocean.amplitude as f32;
    params.choppy = ocean.choppiness as f32;
    params.foam = ocean.foam as f32;
    params.sea_mean = mean.value.unwrap_or_default() as f32;
    if params != current {
        if let Some(mut material) = materials.get_mut(&handle.0) {
            material.params = params;
        }
    }
});
}

fn add_screen_space_transmission(mut commands: Commands, cameras: Query<Entity, (With<Camera3d>, Without<ScreenSpaceTransmission>)>) {
    for camera in &cameras {
        commands.entity(camera).insert(ScreenSpaceTransmission::default());
    }
}

fn insert_material(mut commands: Commands, mut materials: ResMut<Assets<WaterMaterial>>) {
    commands.insert_resource(WaterMaterialHandle(materials.add(WaterMaterial { params: WaterParams::default(), key: WaterKey::default() })));
}

pub struct WaterPipelinePlugin;

impl Plugin for WaterPipelinePlugin {
    fn build(&self, app: &mut App) {
        if let Some(mut shaders) = app.world_mut().get_resource_mut::<Assets<Shader>>() {
            let _ = shaders.insert(WATER_SHADER.id(), water_shader());
        }
        app.add_plugins(MaterialPlugin::<WaterMaterial>::default()).add_systems(Startup, insert_material)
            .add_systems(Update, add_screen_space_transmission);
    }
}

pub struct WaterPlugin {
    pub planet: Arc<Planet>,
    pub config: QuadtreeConfig,
    pub centre: DVec3,
}

impl Plugin for WaterPlugin {
    fn build(&self, app: &mut App) {
        let ocean = WaterOcean::default();
        let baking = AsyncComputeTaskPool::get().spawn(async move {
            note_build();
            sea_height_mean(ocean.choppiness, ocean.amplitude)
        });
        app.add_plugins(WaterPipelinePlugin)
            .insert_resource(WaterFrame { centre: self.centre, radius: self.config.radius })
            .insert_resource(Source { planet: self.planet.clone(), radius: self.config.radius })
            .insert_resource(SeaMean { baking: Some(baking), value: None })
            .insert_resource(ocean)
            .init_resource::<FloatingOrigin>()
            .init_resource::<WaterClock>()
            .init_resource::<WaterSurface>()
            .init_resource::<WaterStreamer>()
            .add_systems(Update, (tick_clock, poll_sea_mean, start_water, finish_water, update_params).chain().after(rebase_system));
    }
}
