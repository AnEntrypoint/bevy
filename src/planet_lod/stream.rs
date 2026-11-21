use super::collider::ColliderStreamPlugin;
use super::guard::{mark_main_thread, note_build};
use super::origin::{FloatingOrigin, OriginRelative, rebase_system};
use super::patch::{PatchMesh, build_patch};
use super::quadtree::{NodeKey, QuadtreeConfig, select_leaves};
use crate::height::Planet;
use crate::height::climate::HpfFaces;
use bevy::asset::RenderAssetUsages;
use bevy::math::DVec3;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures::check_ready};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub const PATCH_SPAWNS_PER_FRAME: usize = 8;
pub const MAX_TASKS_IN_FLIGHT: usize = 64;
const CLIMATE_BAKE_RES: usize = 128;
const CLIMATE_BAKE_MAX_LEVEL: u32 = 7;
const LAND_GREEN: Color = Color::srgb(0.25, 0.55, 0.22);

#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PatchKey(pub NodeKey);

#[derive(Resource, Default, Clone, Debug, PartialEq)]
pub struct PlanetLodStats {
    pub live_patches: usize,
    pub vertices: usize,
    pub triangles: usize,
    pub tasks_in_flight: usize,
    pub waiting_on_climate: usize,
    pub spawned_this_frame: usize,
    pub max_spawned_in_frame: usize,
    pub highest_level: u8,
    pub min_elevation: f32,
    pub max_elevation: f32,
}

#[derive(Resource)]
struct Source {
    planet: Arc<Planet>,
    config: QuadtreeConfig,
    centre: DVec3,
}

#[derive(Resource, Default)]
pub struct ClimateBake {
    baking: Option<Task<Arc<HpfFaces>>>,
    ready: Option<Arc<HpfFaces>>,
}

impl ClimateBake {
    pub fn ready(&self) -> Option<&Arc<HpfFaces>> {
        self.ready.as_ref()
    }
}

fn bake_climate(seed: i32) -> Arc<HpfFaces> {
    note_build();
    Arc::new(HpfFaces::bake(i64::from(seed), CLIMATE_BAKE_RES, CLIMATE_BAKE_MAX_LEVEL))
}

#[derive(Resource)]
struct PatchMaterial(Handle<StandardMaterial>);

struct LivePatch {
    entity: Entity,
    vertices: usize,
    triangles: usize,
    min_elevation: f32,
    max_elevation: f32,
}

#[derive(Resource, Default)]
struct Streamer {
    wanted: HashSet<NodeKey>,
    live: HashMap<NodeKey, LivePatch>,
    pending: HashMap<NodeKey, Task<PatchMesh>>,
    camera: Option<DVec3>,
}

fn overlaps(a: NodeKey, b: NodeKey) -> bool {
    let (coarse, fine) = if a.level <= b.level { (a, b) } else { (b, a) };
    let shift = fine.level - coarse.level;
    a.face == b.face && fine.x >> shift == coarse.x && fine.y >> shift == coarse.y
}

fn patch_mesh(patch: PatchMesh) -> Mesh {
    let heights: Vec<[f32; 2]> = patch.height.iter().map(|&height| [height, 0.0]).collect();
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, patch.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, patch.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, patch.climate)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, heights)
        .with_inserted_indices(Indices::U32(patch.indices))
}

fn insert_material(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    let material = StandardMaterial { base_color: LAND_GREEN, perceptual_roughness: 0.95, ..default() };
    commands.insert_resource(PatchMaterial(materials.add(material)));
}

fn leaves_at(source: &Source, position: DVec3) -> Vec<NodeKey> {
    let radius = source.config.radius;
    let ground = source.planet.height(position.normalize().to_array(), radius);
    select_leaves(&source.config.at(position, ground), position, position.length() - radius)
}

fn plan_leaves(
    source: Res<Source>,
    origin: Res<FloatingOrigin>,
    camera: Query<&Transform, With<Camera3d>>,
    mut streamer: ResMut<Streamer>,
) {
    let Ok(transform) = camera.single() else { return };
    let position = origin.to_world(transform.translation) - source.centre;
    if streamer.camera == Some(position) {
        return;
    }
    let streamer = &mut *streamer;
    streamer.camera = Some(position);
    streamer.wanted = leaves_at(&source, position).into_iter().collect();
    streamer.pending.retain(|key, _| streamer.wanted.contains(key));
}

fn poll_climate(mut bake: ResMut<ClimateBake>) {
    let Some(task) = bake.baking.as_mut() else { return };
    if let Some(baked) = check_ready(task) {
        bake.baking = None;
        bake.ready = Some(baked);
    }
}

fn start_tasks(source: Res<Source>, climate: Res<ClimateBake>, mut streamer: ResMut<Streamer>, mut stats: ResMut<PlanetLodStats>) {
    let room = MAX_TASKS_IN_FLIGHT.saturating_sub(streamer.pending.len());
    let camera = streamer.camera.unwrap_or_default();
    let mut missing: Vec<(f64, NodeKey)> = streamer
        .wanted
        .iter()
        .filter(|key| !streamer.live.contains_key(key) && !streamer.pending.contains_key(key))
        .map(|key| ((key.centre_dir() * source.config.radius - camera).length_squared(), *key))
        .collect();
    missing.sort_by(|a, b| a.0.total_cmp(&b.0));
    let Some(baked) = climate.ready().cloned() else {
        stats.waiting_on_climate = missing.len();
        return;
    };
    stats.waiting_on_climate = 0;
    for (_, key) in missing.into_iter().take(room) {
        let (planet, config, baked) = (source.planet.clone(), source.config.clone(), baked.clone());
        let task = AsyncComputeTaskPool::get().spawn(async move { build_patch(&planet, &baked, &config, key) });
        streamer.pending.insert(key, task);
    }
}

fn finish_tasks(
    mut commands: Commands,
    source: Res<Source>,
    origin: Res<FloatingOrigin>,
    material: Res<PatchMaterial>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut streamer: ResMut<Streamer>,
    mut stats: ResMut<PlanetLodStats>,
) {
    let mut finished = Vec::new();
    for (key, task) in streamer.pending.iter_mut() {
        if finished.len() == PATCH_SPAWNS_PER_FRAME {
            break;
        }
        if let Some(patch) = check_ready(task) {
            finished.push((*key, patch));
        }
    }
    stats.spawned_this_frame = finished.len();
    stats.max_spawned_in_frame = stats.max_spawned_in_frame.max(finished.len());
    for (key, patch) in finished {
        streamer.pending.remove(&key);
        let local = origin.to_local(patch.origin + source.centre);
        let (vertices, triangles, min_elevation, max_elevation) = (patch.positions.len(), patch.indices.len() / 3, patch.min_elevation, patch.max_elevation);
        let entity = commands
            .spawn((Mesh3d(meshes.add(patch_mesh(patch))), MeshMaterial3d(material.0.clone()), Transform::from_translation(local), OriginRelative, PatchKey(key)))
            .id();
        streamer.live.insert(key, LivePatch { entity, vertices, triangles, min_elevation, max_elevation });
    }
}

fn retire_patches(mut commands: Commands, mut streamer: ResMut<Streamer>) {
    let streamer = &mut *streamer;
    let replaced: Vec<NodeKey> = streamer
        .live
        .keys()
        .filter(|key| !streamer.wanted.contains(key))
        .filter(|key| streamer.wanted.iter().filter(|leaf| overlaps(**leaf, **key)).all(|leaf| streamer.live.contains_key(leaf)))
        .copied()
        .collect();
    for key in replaced {
        if let Some(patch) = streamer.live.remove(&key) {
            commands.entity(patch.entity).despawn();
        }
    }
}

fn publish_stats(streamer: Res<Streamer>, mut stats: ResMut<PlanetLodStats>) {
    stats.live_patches = streamer.live.len();
    stats.vertices = streamer.live.values().map(|patch| patch.vertices).sum();
    stats.triangles = streamer.live.values().map(|patch| patch.triangles).sum();
    stats.tasks_in_flight = streamer.pending.len();
    stats.highest_level = streamer.live.keys().map(|key| key.level).max().unwrap_or(0);
    stats.min_elevation = streamer.live.values().map(|patch| patch.min_elevation).reduce(f32::min).unwrap_or(0.0);
    stats.max_elevation = streamer.live.values().map(|patch| patch.max_elevation).fold(0.0, f32::max);
}

pub struct PlanetLodPlugin {
    pub planet: Arc<Planet>,
    pub config: QuadtreeConfig,
    pub centre: DVec3,
    pub seed: i32,
}

impl Plugin for PlanetLodPlugin {
    fn build(&self, app: &mut App) {
        mark_main_thread();
        let seed = self.seed;
        let baking = AsyncComputeTaskPool::get().spawn(async move { bake_climate(seed) });
        app.insert_resource(Source { planet: self.planet.clone(), config: self.config.clone(), centre: self.centre })
            .init_resource::<FloatingOrigin>()
            .insert_resource(ClimateBake { baking: Some(baking), ready: None })
            .init_resource::<Streamer>()
            .init_resource::<PlanetLodStats>()
            .add_plugins(ColliderStreamPlugin)
            .add_systems(Startup, insert_material)
            .add_systems(Update, (rebase_system, plan_leaves, poll_climate, start_tasks, finish_tasks, retire_patches, publish_stats).chain());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::height::{HashVersion, PlanetConfig};
    use crate::planet_lod::guard::builds_on_main_thread;
    use crate::planet_lod::origin::REBASE_DISTANCE_M;
    use bevy::asset::AssetPlugin;
    use bevy::ecs::schedule::SingleThreadedExecutor;
    use std::time::Duration;

    const TEST_SEED: i32 = 7;
    const PATCH_VERTICES: usize = 144;
    const PATCH_TRIANGLES: usize = 242;
    const FRAME_PAUSE: Duration = Duration::from_millis(5);
    const SETTLE_FRAME_LIMIT: usize = 4000;
    const FRAMES_BEFORE_READY: usize = 30;
    const MOVE_M: f64 = 5000.0;
    const VIEWPORT_HEIGHT: f64 = 1080.0;
    const FOV_Y_RAD: f64 = 1.0;
    const SHORELINE_ALTITUDE_M: f64 = 0.5;
    const GROUND_ALTITUDE_M: f64 = 2.0;

    fn altitudes(radius: f64) -> [(&'static str, f64); 5] {
        [("orbit", radius), ("high", radius * 0.1), ("low", radius * 0.005), ("ground", GROUND_ALTITUDE_M), ("shoreline", SHORELINE_ALTITUDE_M)]
    }

    fn surface_direction() -> DVec3 {
        DVec3::new(0.3, 0.8, 0.5).normalize()
    }

    fn app_at(radius: f64, max_level: u8, camera: DVec3) -> App {
        let planet = Planet::new(&PlanetConfig { seed: TEST_SEED, hpf_res: 128, hash: HashVersion::Integer, relief_scale: None, anchor_dir: [0.0, 1.0, 0.0], radius, carves: Vec::new() });
        let config = QuadtreeConfig { max_level, ..QuadtreeConfig::reference(radius, VIEWPORT_HEIGHT, FOV_Y_RAD) };
        app_with(Arc::new(planet), config, camera)
    }

    fn app_with(planet: Arc<Planet>, config: QuadtreeConfig, camera: DVec3) -> App {
        let mut app = App::new();
        app.edit_schedule(Update, |schedule| {
            schedule.set_executor(SingleThreadedExecutor::new());
        });
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .insert_resource(FloatingOrigin { position: camera })
            .add_plugins(PlanetLodPlugin { planet, config, centre: DVec3::ZERO, seed: TEST_SEED });
        app.world_mut().spawn((Camera3d::default(), Transform::IDENTITY, OriginRelative));
        app
    }

    fn camera_local(app: &mut App) -> Vec3 {
        app.world_mut().query_filtered::<&Transform, With<Camera3d>>().single(app.world()).unwrap().translation
    }

    fn expected_leaves(app: &mut App) -> HashSet<NodeKey> {
        let origin = *app.world().resource::<FloatingOrigin>();
        let position = origin.to_world(camera_local(app)) - app.world().resource::<Source>().centre;
        leaves_at(app.world().resource::<Source>(), position).into_iter().collect()
    }

    fn live_keys(app: &mut App) -> Vec<NodeKey> {
        app.world_mut().query::<&PatchKey>().iter(app.world()).map(|key| key.0).collect()
    }

    fn covers_every(app: &mut App, leaves: &HashSet<NodeKey>) -> bool {
        let live = live_keys(app);
        leaves.iter().all(|leaf| live.iter().any(|key| overlaps(*key, *leaf)))
    }

    fn settled(app: &mut App) -> bool {
        let expected = expected_leaves(app);
        let live: HashSet<NodeKey> = live_keys(app).into_iter().collect();
        live == expected && app.world().resource::<Streamer>().pending.is_empty()
    }

    fn wait_for_started_patches(app: &App) {
        while app.world().resource::<Streamer>().pending.values().any(|task| !task.is_finished()) {
            std::thread::sleep(FRAME_PAUSE);
        }
    }

    fn settle(app: &mut App, old: &HashSet<NodeKey>) {
        for _ in 0..SETTLE_FRAME_LIMIT {
            std::thread::sleep(FRAME_PAUSE);
            wait_for_started_patches(app);
            app.update();
            assert!(covers_every(app, old), "a leaf of the previous set lost its cover while streaming");
            if settled(app) {
                return;
            }
        }
        panic!("the patch set never settled");
    }

    fn move_camera_on_surface(app: &mut App, radius: f64, altitude: f64) {
        let origin = *app.world().resource::<FloatingOrigin>();
        let here = origin.to_world(camera_local(app));
        let tangent = here.cross(DVec3::Y).normalize();
        let moved = (here + tangent * MOVE_M).normalize() * (radius + altitude);
        let mut camera = app.world_mut().query_filtered::<&mut Transform, With<Camera3d>>();
        camera.single_mut(app.world_mut()).unwrap().translation = origin.to_local(moved);
    }

    fn assert_patch_shapes(app: &mut App) {
        let handles: Vec<Handle<Mesh>> = app.world_mut().query::<&Mesh3d>().iter(app.world()).map(|mesh| mesh.0.clone()).collect();
        let meshes = app.world().resource::<Assets<Mesh>>();
        for handle in handles {
            let mesh = meshes.get(&handle).unwrap();
            assert_eq!(mesh.count_vertices(), PATCH_VERTICES);
            for attribute in [Mesh::ATTRIBUTE_UV_0, Mesh::ATTRIBUTE_UV_1] {
                assert_eq!(mesh.attribute(attribute).unwrap().len(), PATCH_VERTICES);
            }
            assert_eq!(mesh.indices().unwrap().len() / 3, PATCH_TRIANGLES);
        }
    }

    fn witness(radius: f64, max_level: u8) {
        for (name, altitude) in altitudes(radius) {
            let start = surface_direction() * (radius + altitude);
            let mut app = app_at(radius, max_level, start);
            settle(&mut app, &HashSet::new());
            let first = expected_leaves(&mut app);
            let stats = app.world().resource::<PlanetLodStats>().clone();
            assert_eq!(live_keys(&mut app).len(), first.len(), "{name}: one entity per patch");
            assert_eq!(stats.live_patches, first.len(), "{name}");
            assert_eq!(stats.vertices, first.len() * PATCH_VERTICES, "{name}");
            assert_eq!(stats.triangles, first.len() * PATCH_TRIANGLES, "{name}");
            assert_eq!(stats.tasks_in_flight, 0, "{name}");
            assert_eq!(stats.highest_level, first.iter().map(|key| key.level).max().unwrap(), "{name}");
            assert!(stats.max_spawned_in_frame <= PATCH_SPAWNS_PER_FRAME, "{name}: spawned {} in one frame", stats.max_spawned_in_frame);
            if first.len() > PATCH_SPAWNS_PER_FRAME {
                assert_eq!(stats.max_spawned_in_frame, PATCH_SPAWNS_PER_FRAME, "{name}: the budget was never exercised");
            }
            if altitude <= GROUND_ALTITUDE_M {
                assert_eq!(stats.highest_level, max_level, "{name}: the near camera reaches the deepest level");
            }
            assert_patch_shapes(&mut app);
            assert_eq!(builds_on_main_thread(), 0, "{name}");

            let origin_before = app.world().resource::<FloatingOrigin>().position;
            move_camera_on_surface(&mut app, radius, altitude);
            settle(&mut app, &first);
            let second = expected_leaves(&mut app);
            if altitude <= GROUND_ALTITUDE_M {
                assert_ne!(first, second, "{name}: moving 5 km changes the leaves");
            }
            let live: HashSet<NodeKey> = live_keys(&mut app).into_iter().collect();
            assert_eq!(live, second, "{name}: the live set follows the camera");
            assert!(first.difference(&second).all(|old| !live.contains(old)), "{name}: old patches are dropped");
            assert_eq!(app.world().resource::<PlanetLodStats>().live_patches, second.len(), "{name}");
            assert_ne!(app.world().resource::<FloatingOrigin>().position, origin_before, "{name}: the origin was rebased");
            assert!(f64::from(camera_local(&mut app).length()) < REBASE_DISTANCE_M, "{name}");
            assert_patch_shapes(&mut app);
            assert_eq!(builds_on_main_thread(), 0, "{name}");
        }
    }

    #[test]
    fn the_game_near_floor_streams_leaves_finer_than_the_plain_max_level_with_the_camera_on_the_ground() {
        let plain_max_level = 11;
        let globe = crate::globe::GlobeWorld { max_level: plain_max_level, ..Default::default() };
        let config = globe.quadtree_config();
        assert!(config.near_floor.is_some() && config.near_field.is_some());
        let camera = globe.surface_position(0.0, 0.0, GROUND_ALTITUDE_M as f32).as_dvec3();
        let altitude = camera.length() - config.radius;
        let plain = QuadtreeConfig { near_floor: None, near_field: None, ..config.clone() };
        let plain_finest = select_leaves(&plain, camera, altitude).iter().map(|key| key.level).max().unwrap();
        assert_eq!(plain_finest, plain_max_level);
        let per_frame_only = Source { planet: globe.planet.clone(), config: QuadtreeConfig { near_floor: None, ..config.clone() }, centre: DVec3::ZERO };
        let per_frame_finest = leaves_at(&per_frame_only, camera).iter().map(|key| key.level).max().unwrap();
        assert!(per_frame_finest > plain_max_level, "the floor refreshed at the camera streams level {per_frame_finest}");
        let mut app = app_with(globe.planet.clone(), config, camera);
        settle(&mut app, &HashSet::new());
        let finest = live_keys(&mut app).iter().map(|key| key.level).max().unwrap();
        assert!(finest > plain_max_level, "the floor streams level {finest} where the plain tree stops at {plain_max_level}");
        assert_eq!(app.world().resource::<PlanetLodStats>().highest_level, finest);
    }

    #[test]
    fn no_patch_starts_before_the_climate_bake_is_ready() {
        let radius = 63_600.0;
        let mut app = app_at(radius, 13, surface_direction() * radius * 2.0);
        let never_ready = AsyncComputeTaskPool::get().spawn(std::future::pending());
        app.insert_resource(ClimateBake { baking: Some(never_ready), ready: None });
        for _ in 0..FRAMES_BEFORE_READY {
            app.update();
        }
        let wanted = expected_leaves(&mut app).len();
        assert!(wanted > 0);
        assert_eq!(app.world().resource::<PlanetLodStats>().waiting_on_climate, wanted);
        assert_eq!(app.world().resource::<PlanetLodStats>().tasks_in_flight, 0);
        assert!(live_keys(&mut app).is_empty());
        let baked = bake_climate(TEST_SEED);
        app.insert_resource(ClimateBake { baking: None, ready: Some(baked) });
        settle(&mut app, &HashSet::new());
        assert_eq!(app.world().resource::<PlanetLodStats>().waiting_on_climate, 0);
        assert_eq!(live_keys(&mut app).len(), wanted);
    }

    #[test]
    fn the_climate_bake_finishes_off_the_main_thread_and_is_shared() {
        let radius = 63_600.0;
        let mut app = app_at(radius, 13, surface_direction() * radius * 2.0);
        settle(&mut app, &HashSet::new());
        assert_eq!(builds_on_main_thread(), 0);
        let shared = app.world().resource::<ClimateBake>().ready().unwrap().clone();
        assert!(Arc::strong_count(&shared) >= 2);
        let reference = HpfFaces::bake(i64::from(TEST_SEED), 128, 7);
        for dir in [surface_direction(), -surface_direction(), DVec3::new(-0.7, 0.1, 0.2).normalize()] {
            assert_eq!(shared.at_dir(dir.to_array()), reference.at_dir(dir.to_array()));
        }
    }

    #[test]
    fn small_planet_streams_exactly_the_selected_leaves_at_every_altitude() {
        witness(63_600.0, 13);
    }

    #[test]
    fn every_patch_shares_one_land_green_material() {
        let radius = 63_600.0;
        let mut app = app_at(radius, 13, surface_direction() * radius * 2.0);
        settle(&mut app, &HashSet::new());
        let handles: HashSet<AssetId<StandardMaterial>> = app.world_mut().query::<&MeshMaterial3d<StandardMaterial>>().iter(app.world()).map(|material| material.0.id()).collect();
        assert_eq!(handles.len(), 1);
        let material = app.world().resource::<Assets<StandardMaterial>>().get(*handles.iter().next().unwrap()).unwrap();
        assert_eq!(material.base_color, LAND_GREEN);
    }

    #[test]
    fn reference_planet_streams_exactly_the_selected_leaves_at_every_altitude() {
        witness(6_360_000.0, 11);
    }
}
