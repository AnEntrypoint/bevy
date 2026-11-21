use super::collider::ColliderStreamPlugin;
use super::guard::mark_main_thread;
use super::origin::{FloatingOrigin, OriginRelative, rebase_system};
use super::patch::{PatchMesh, build_patch};
use super::quadtree::{NodeKey, QuadtreeConfig, select_leaves};
use crate::height::Planet;
use bevy::asset::RenderAssetUsages;
use bevy::math::DVec3;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures::check_ready};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub const PATCH_SPAWNS_PER_FRAME: usize = 8;
pub const MAX_TASKS_IN_FLIGHT: usize = 64;
const LAND_GREEN: Color = Color::srgb(0.25, 0.55, 0.22);

#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PatchKey(pub NodeKey);

#[derive(Resource, Default, Clone, Debug, PartialEq)]
pub struct PlanetLodStats {
    pub live_patches: usize,
    pub vertices: usize,
    pub triangles: usize,
    pub tasks_in_flight: usize,
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
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, patch.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, patch.normals)
        .with_inserted_indices(Indices::U32(patch.indices))
}

fn insert_material(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    let material = StandardMaterial { base_color: LAND_GREEN, perceptual_roughness: 0.95, ..default() };
    commands.insert_resource(PatchMaterial(materials.add(material)));
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
    let altitude = position.length() - source.config.radius;
    let streamer = &mut *streamer;
    streamer.camera = Some(position);
    streamer.wanted = select_leaves(&source.config, position, altitude).into_iter().collect();
    streamer.pending.retain(|key, _| streamer.wanted.contains(key));
}

fn start_tasks(source: Res<Source>, mut streamer: ResMut<Streamer>) {
    let room = MAX_TASKS_IN_FLIGHT.saturating_sub(streamer.pending.len());
    let camera = streamer.camera.unwrap_or_default();
    let mut missing: Vec<(f64, NodeKey)> = streamer
        .wanted
        .iter()
        .filter(|key| !streamer.live.contains_key(key) && !streamer.pending.contains_key(key))
        .map(|key| ((key.centre_dir() * source.config.radius - camera).length_squared(), *key))
        .collect();
    missing.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (_, key) in missing.into_iter().take(room) {
        let (planet, config) = (source.planet.clone(), source.config.clone());
        let task = AsyncComputeTaskPool::get().spawn(async move { build_patch(&planet, &config, key) });
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
}

impl Plugin for PlanetLodPlugin {
    fn build(&self, app: &mut App) {
        mark_main_thread();
        app.insert_resource(Source { planet: self.planet.clone(), config: self.config.clone(), centre: self.centre })
            .init_resource::<FloatingOrigin>()
            .init_resource::<Streamer>()
            .init_resource::<PlanetLodStats>()
            .add_plugins(ColliderStreamPlugin)
            .add_systems(Startup, insert_material)
            .add_systems(Update, (rebase_system, plan_leaves, start_tasks, finish_tasks, retire_patches, publish_stats).chain());
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

    const PATCH_VERTICES: usize = 144;
    const PATCH_TRIANGLES: usize = 242;
    const FRAME_PAUSE: Duration = Duration::from_millis(5);
    const SETTLE_FRAME_LIMIT: usize = 4000;
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
        let planet = Planet::new(&PlanetConfig { seed: 7, hpf_res: 128, hash: HashVersion::Integer, relief_scale: None, anchor_dir: [0.0, 1.0, 0.0], radius, carves: Vec::new() });
        let config = QuadtreeConfig { max_level, ..QuadtreeConfig::reference(radius, VIEWPORT_HEIGHT, FOV_Y_RAD) };
        let mut app = App::new();
        app.edit_schedule(Update, |schedule| {
            schedule.set_executor(SingleThreadedExecutor::new());
        });
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .insert_resource(FloatingOrigin { position: camera })
            .add_plugins(PlanetLodPlugin { planet: Arc::new(planet), config, centre: DVec3::ZERO });
        app.world_mut().spawn((Camera3d::default(), Transform::IDENTITY, OriginRelative));
        app
    }

    fn camera_local(app: &mut App) -> Vec3 {
        app.world_mut().query_filtered::<&Transform, With<Camera3d>>().single(app.world()).unwrap().translation
    }

    fn expected_leaves(app: &mut App) -> HashSet<NodeKey> {
        let source = app.world().resource::<Source>();
        let origin = *app.world().resource::<FloatingOrigin>();
        let (config, centre) = (source.config.clone(), source.centre);
        let position = origin.to_world(camera_local(app)) - centre;
        select_leaves(&config, position, position.length() - config.radius).into_iter().collect()
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

    fn settle(app: &mut App, old: &HashSet<NodeKey>) {
        for _ in 0..SETTLE_FRAME_LIMIT {
            std::thread::sleep(FRAME_PAUSE);
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
