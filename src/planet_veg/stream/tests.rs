use super::*;
use crate::globe::{GlobeWorld, equip_camera};
use crate::planet_scene::add_vegetation;
use crate::height::PlanetConfig;
use crate::offscreen::{Frame, SKY_COLOUR, add_target, headless_app, pump, render_lock, spawn_view};
use crate::planet_lod::guard;
use crate::planet_sky::light::Quality;
use crate::planet_sky::sky::SkySun;
use crate::planet_veg::context::fixture;
use crate::planet_veg::grass;
use crate::planet_veg::grass::place_grass_chunk;
use crate::planet_veg::rock::place_rock_chunk;
use crate::planet_veg::tree::place_tree_chunk;
use crate::planet_veg::gpu::buffers::{GRASS_SLOTS, VegBounds, VegGpuConfig};
use crate::planet_veg::gpu::cull::{CullStats, CullView, GrassCullSettings, VegCullSettings, VegWitness, frustum_planes};
use crate::planet_veg::gpu::draw::VegDrawSet;
use crate::planet_veg::gpu::store::CHUNK_SLOTS;
use crate::planet_veg::gpu::twin::{TwinBlade, TwinRock, TwinTree, cull_grass, cull_rocks, cull_trees};
use crate::planet_veg::trees_render::TreesWitness;
use crate::testkit::{cases, parse};
use bevy::render::render_resource::TextureFormat;
use serde_json::Value;
use bevy::tasks::TaskPool;
use std::collections::BTreeSet;
use std::sync::OnceLock;
use std::sync::atomic::Ordering;

const PUMP_LIMIT: usize = 4000;
const STABLE_FRAMES: usize = 4;
const TREE_RING_CHUNKS: usize = 1863;
const TREE_RING_PLACED: u32 = 9888;
const TREE_RING_RESIDENT: u32 = 4982;
const ROCK_RING_CHUNKS: usize = 416;
const ROCK_RING_PLACED: u32 = 103;
const GRASS_RING_CHUNKS: usize = 32;
const GRASS_RING_PLACED: u32 = 182_552;
const MESH_TIERS: u32 = 3;
const SECOND_POSE_M: f64 = 100.0;
const REFERENCE_TREE_RING_M: f64 = 680.0;
const REFERENCE_TREE_DROP_M: f64 = 744.0;
const PITCH_DOWN_DEG: f64 = 6.0;
const PLANE_NORMAL_TOLERANCE: f32 = 1e-4;
const PLANE_OFFSET_TOLERANCE_M: f32 = 0.05;
const GRASS_ROUNDING_M: f32 = 1e-3;

fn fixture_json(text: &'static str, cell: &'static OnceLock<Value>) -> &'static Value {
    cell.get_or_init(|| parse(text))
}

fn trees_fixture() -> &'static Value {
    static CELL: OnceLock<Value> = OnceLock::new();
    fixture_json(include_str!("../../../testdata/parity/spoint-planet-veg-placement.json"), &CELL)
}

fn rocks_fixture() -> &'static Value {
    static CELL: OnceLock<Value> = OnceLock::new();
    fixture_json(include_str!("../../../testdata/parity/spoint-planet-rock-placement.json"), &CELL)
}

fn grass_fixture() -> &'static Value {
    static CELL: OnceLock<Value> = OnceLock::new();
    fixture_json(include_str!("../../../testdata/parity/spoint-planet-grass-placement.json"), &CELL)
}

fn ring_keys(fixture: &Value) -> BTreeSet<u64> {
    cases(&fixture["ring"], "chunks").iter().map(|row| row[0].as_u64().unwrap()).collect()
}

fn v3(value: &Value) -> DVec3 {
    DVec3::from_slice(&value.as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect::<Vec<_>>())
}

fn player_world() -> DVec3 {
    v3(&trees_fixture()["thinning"]["playerWorld"])
}

fn planet() -> Arc<Planet> {
    static PLANET: OnceLock<Arc<Planet>> = OnceLock::new();
    PLANET.get_or_init(|| Arc::new(Planet::new(&fixture::planet_config()))).clone()
}

fn source() -> VegSource {
    VegSource { planet: planet(), seed: fixture::SEED, radius: fixture::RADIUS, hash: HashVersion::Integer }
}

fn preset_globe() -> GlobeWorld {
    let config: PlanetConfig = fixture::planet_config();
    GlobeWorld { planet: planet(), seed: u64::from(fixture::SEED), radius: config.radius as f32, relief_scale: config.relief_scale, hash_version: config.hash, ..GlobeWorld::default() }
}

fn bare_stream() -> VegStream {
    VegStream::new(&VegStreamPlugin { source: source(), centre: DVec3::ZERO, anchor_dir: [0.0, 1.0, 0.0] })
}

fn along_north(position: DVec3, metres: f64) -> DVec3 {
    let up = position.normalize();
    let east = DVec3::Y.cross(up).normalize();
    let north = up.cross(east);
    let angle = metres / position.length();
    (up * angle.cos() + north * angle.sin()) * position.length()
}

fn ring_around(position: DVec3, radius_m: f64) -> BTreeSet<u64> {
    bare_stream().lattice.ring_around_dir(position.normalize().to_array(), radius_m).into_iter().map(|key| key as u64).collect()
}

fn within(position: DVec3, radius_m: f64, keys: &BTreeSet<u64>) -> BTreeSet<u64> {
    let stream = bare_stream();
    let (dir, limit) = (position.normalize(), chord_sq_within(radius_m, fixture::RADIUS));
    keys.iter().copied().filter(|key| (DVec3::from(stream.lattice.chunk_centre_dir(*key as f64)) - dir).length_squared() <= limit).collect()
}

fn refocus_now(stream: &mut VegStream, position: DVec3, store: &mut VegStore) {
    let plans = plan_rings(&stream.lattice, position.normalize().to_array(), stream.kinds.each_ref().map(KindStream::streamed_keys));
    stream.kinds.iter_mut().zip(plans).for_each(|(kind, plan)| kind.replan(plan));
    while stream.kinds.iter().any(|kind| !kind.dropping.is_empty()) {
        let mut work = 0;
        stream.drop_due(&mut work, store);
        assert!(work <= RING_WORK_PER_FRAME);
    }
}

fn loaded_keys(stream: &VegStream, kind: VegKind) -> BTreeSet<u64> {
    stream.kinds[kind.index()].loaded.keys().copied().collect()
}

#[test]
fn the_rings_are_the_reference_load_and_drop_radii() {
    assert_eq!(RINGS, [Ring { load_m: 680.0, drop_m: 744.0 }, Ring { load_m: 360.0, drop_m: 424.0 }, Ring { load_m: 88.0, drop_m: 104.0 }]);
    assert_eq!(RINGS[VegKind::Tree.index()].load_m, trees_fixture()["ring"]["radiusM"].as_f64().unwrap());
    assert_eq!(RINGS[VegKind::Grass.index()].load_m, grass_fixture()["ring"]["radiusM"].as_f64().unwrap());
    assert_eq!(RINGS[VegKind::Rock.index()].load_m - RING_MARGIN_M, rocks_fixture()["ring"]["radiusM"].as_f64().unwrap());
}

#[test]
fn chunks_stay_until_they_pass_the_drop_radius() {
    let mut stream = bare_stream();
    let mut store = VegStore::new(&VegGpuConfig::default());
    let first = player_world();
    refocus_now(&mut stream, first, &mut store);
    let trees = &mut stream.kinds[VegKind::Tree.index()];
    let first_ring: BTreeSet<u64> = trees.ring.iter().copied().collect();
    first_ring.iter().for_each(|key| {
        trees.loaded.insert(*key, None);
    });
    let near = along_north(first, 2.0 * CHUNK_M);
    refocus_now(&mut stream, near, &mut store);
    let kept = loaded_keys(&stream, VegKind::Tree);
    let beyond_ring: BTreeSet<u64> = first_ring.difference(&ring_around(near, RINGS[0].load_m)).copied().collect();
    assert!(!beyond_ring.is_empty());
    assert_eq!(kept, within(near, RINGS[0].drop_m, &first_ring));
    assert!(beyond_ring.is_subset(&kept), "chunks past the ring but inside the drop radius stay loaded");
    let far = along_north(first, 4.0 * DROP_MARGIN_M);
    refocus_now(&mut stream, far, &mut store);
    assert_eq!(loaded_keys(&stream, VegKind::Tree), within(far, RINGS[0].drop_m, &first_ring));
    assert!(loaded_keys(&stream, VegKind::Tree).len() < kept.len());
}

#[test]
fn a_refocus_spreads_its_ring_work_over_frames_under_the_cap() {
    AsyncComputeTaskPool::get_or_init(TaskPool::new);
    let mut stream = bare_stream();
    let mut store = VegStore::new(&VegGpuConfig::default());
    let (first, near) = (player_world(), along_north(player_world(), 2.0 * CHUNK_M));
    let streamed = RINGS.map(|ring| ring_around(first, ring.drop_m + DROP_MARGIN_M).union(&ring_around(near, ring.load_m)).copied().collect::<BTreeSet<u64>>());
    for (kind, keys) in stream.kinds.iter_mut().zip(&streamed) {
        kind.loaded.extend(keys.iter().map(|key| (*key, None)));
    }
    let kept: usize = RINGS.iter().zip(&streamed).map(|(ring, keys)| within(near, ring.drop_m, keys).len()).sum();
    let ring_slots: usize = RINGS.iter().map(|ring| ring_around(near, ring.load_m).len()).sum();
    let total = streamed.iter().map(BTreeSet::len).sum::<usize>() - kept + ring_slots;
    let settled = |stream: &VegStream| stream.focus.is_some() && stream.stats(0).tasks_in_flight == 0 && stream.stats(0).queued_chunks == [0; 3];
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let mut busy = Vec::new();
    while !settled(&stream) {
        assert!(std::time::Instant::now() < deadline, "the refocus did not finish");
        busy.extend(Some(stream.advance(near, &mut store)).filter(|work| *work > 0));
    }
    assert!(total > RING_WORK_PER_FRAME && busy.iter().all(|work| *work <= RING_WORK_PER_FRAME), "{busy:?}");
    assert_eq!((busy.iter().sum::<usize>(), busy.len()), (total, total.div_ceil(RING_WORK_PER_FRAME)));
    assert_eq!(stream.kinds.iter().map(|kind| kind.loaded.len()).sum::<usize>(), kept);
}

#[test]
fn the_wind_chart_is_fixed_to_the_planet() {
    let stream = bare_stream();
    let blade = DVec3::new(120.0, fixture::RADIUS + 40.0, -75.0);
    let wind_at = |eye: DVec3| {
        let look = stream.wind.look(eye, GrassLook::default());
        let to_blade = (blade - eye).as_vec3();
        look.wind_origin + Vec2::new(to_blade.dot(look.wind_axis_x), to_blade.dot(look.wind_axis_z))
    };
    let expected = stream.wind.look(blade, GrassLook::default()).wind_origin;
    assert!(expected.length() > 100.0);
    for eye in [DVec3::new(100.0, fixture::RADIUS + 2.0, -60.0), DVec3::new(-300.0, fixture::RADIUS + 1.0, 250.0)] {
        let got = wind_at(eye);
        assert!((got - expected).length() < 1e-2, "wind position {got} for eye {eye}, expected {expected}");
    }
}

#[test]
fn records_keep_the_reference_rank_and_rotation_order() {
    for q in [0.05_f32, 0.3, 1.0] {
        let record = grass_record(&GrassBlade { q, pos: [0.0; 3], scale: 1.0, yaw: 0.0, normal: [0.0, 1.0, 0.0], tint: 0.0, wind_phase: 0.0, shadow: 1.0, ground: [0.0; 3] });
        assert!((grass::FULL_R_M as f32 / record.rank + grass::FADE_M as f32 - grass::grass_blade_edge_m(q)).abs() < 1e-4);
    }
    let tilt = Quat::from_rotation_x(0.3);
    let rock = RockInstance { pos: [0.0; 3], rock_type: 2, scale: 1.0, yaw: 1.1, squash: 1.0, variant: 0.0, normal: [0.0, 1.0, 0.0], tilt_quat: tilt.to_array(), rock_id: 0 };
    let rotation = Quat::from_array(rock_record(&rock).rotation);
    assert!(rotation.mul_vec3(Vec3::X).distance(tilt.mul_vec3(Quat::from_rotation_y(1.1).mul_vec3(Vec3::X))) < 1e-6);
}

struct Census {
    app: App,
    camera: Entity,
    peak_ring_work: usize,
}

impl Census {
    fn new() -> Self {
        guard::mark_main_thread();
        let mut app = headless_app();
        add_vegetation(&mut app, &preset_globe());
        app.insert_resource(Quality::default()).init_resource::<SkySun>().add_systems(Update, equip_camera);
        app.finish();
        app.cleanup();
        let target = add_target(&mut app, TextureFormat::Rgba8UnormSrgb);
        let player = player_world();
        let frame = Frame::above(player.normalize(), player.length(), 0.0, PITCH_DOWN_DEG);
        spawn_view(&mut app, &target, SKY_COLOUR, &frame, 1.0);
        let camera = app.world_mut().query_filtered::<Entity, With<Camera3d>>().single(app.world()).unwrap();
        app.world().resource::<VegWitness>().readback.store(true, Ordering::SeqCst);
        let mut census = Self { app, camera, peak_ring_work: 0 };
        census.place(player);
        census
    }

    fn place(&mut self, position: DVec3) {
        let mut transform = self.app.world_mut().get_mut::<Transform>(self.camera).unwrap();
        transform.translation = position.as_vec3();
    }

    fn entities(&mut self) -> usize {
        self.app.world_mut().query::<Entity>().iter(self.app.world()).count()
    }

    fn assets_ready(&mut self) -> bool {
        let witness = self.app.world().resource::<TreesWitness>().clone();
        assert!(witness.refusal.lock().unwrap().is_none(), "{:?}", witness.refusal.lock().unwrap());
        let draw_sets = self.app.world_mut().query::<&VegDrawSet>().iter(self.app.world()).count();
        witness.loaded.load(Ordering::SeqCst) && witness.baked.load(Ordering::SeqCst) && draw_sets == expected_draw_sets(self.app.world().resource::<VegGpuConfig>())
    }

    fn streamed(&self) -> bool {
        let stats = self.app.world().resource::<VegStreamStats>();
        let store = self.app.world().resource::<VegStore>();
        self.app.world().resource::<VegStream>().focus.is_some() && stats.tasks_in_flight == 0 && stats.queued_chunks == [0; 3] && VegKind::ALL.iter().all(|kind| store.pending_bytes(*kind) == 0)
    }

    fn pump(&mut self) {
        pump(&mut self.app);
        let ring_work = self.app.world().resource::<VegStreamStats>().ring_work;
        assert!(ring_work <= RING_WORK_PER_FRAME, "{ring_work} ring work in one frame");
        self.peak_ring_work = self.peak_ring_work.max(ring_work);
    }

    fn settle(&mut self, label: &str) -> CullStats {
        let entities = (0..PUMP_LIMIT).find_map(|_| {
            self.pump();
            self.assets_ready().then(|| self.entities())
        });
        let entities = entities.unwrap_or_else(|| panic!("{label}: the vegetation assets did not load in {PUMP_LIMIT} frames"));
        let mut seen = Vec::new();
        for _ in 0..PUMP_LIMIT {
            self.pump();
            assert_eq!(self.entities(), entities, "{label}: streaming changed the entity count");
            let stats = self.app.world().resource::<VegWitness>().latest_stats();
            seen.push(stats.filter(|_| self.streamed()));
            if seen.len() >= STABLE_FRAMES && seen[seen.len() - STABLE_FRAMES..].windows(2).all(|pair| pair[0].is_some() && pair[0] == pair[1]) {
                return seen.last().unwrap().unwrap();
            }
        }
        panic!("{label}: streaming did not settle in {PUMP_LIMIT} frames: {:?}", self.app.world().resource::<VegStreamStats>());
    }

    fn stream(&self) -> &VegStream {
        self.app.world().resource::<VegStream>()
    }

    fn twin(&self) -> Twin {
        let world = self.app.world();
        let (store, config, settings, bounds) = (world.resource::<VegStore>(), world.resource::<VegGpuConfig>(), world.resource::<VegCullSettings>(), world.resource::<VegBounds>());
        let view = world.resource::<VegWitness>().latest_view().expect("the cull saw a view");
        let origin = world.resource::<FloatingOrigin>();
        let chunks: Vec<_> = (0..CHUNK_SLOTS).map(|slot| store.table().record(slot, origin)).collect();
        let planet = planet();
        let ctx = PlacementContext::new(&planet, fixture::SEED, fixture::RADIUS, HashVersion::Integer);
        let resident = |kind: VegKind| self.stream().kinds[kind.index()].loaded.iter().filter_map(|(key, chunk)| chunk.map(|chunk| (*key, chunk.packed()))).collect::<Vec<_>>();
        let trees: Vec<_> = resident(VegKind::Tree).into_iter().flat_map(|(key, chunk)| place_tree_chunk(&ctx, key).iter().map(move |tree| TwinTree { record: tree_record(tree), chunk }).collect::<Vec<_>>()).collect();
        let rocks: Vec<_> = resident(VegKind::Rock).into_iter().flat_map(|(key, chunk)| place_rock_chunk(&ctx, key).iter().map(move |rock| TwinRock { record: rock_record(rock), chunk }).collect::<Vec<_>>()).collect();
        let blades: Vec<_> = resident(VegKind::Grass).into_iter().flat_map(|(key, chunk)| place_grass_chunk(&ctx, key).iter().map(move |blade| TwinBlade { record: grass_record(blade), chunk }).collect::<Vec<_>>()).collect();
        let first = cull_trees(&trees, &chunks, &bounds.tree_geometry, config, &settings.tree, &view, &vec![0; trees.len()]);
        let trees = cull_trees(&trees, &chunks, &bounds.tree_geometry, config, &settings.tree, &view, &first.states);
        let first_rocks = cull_rocks(&rocks, &chunks, &bounds.rock_kind, config, &settings.rock, &view, &vec![0; rocks.len()]);
        let rocks = cull_rocks(&rocks, &chunks, &bounds.rock_kind, config, &settings.rock, &view, &first_rocks.states);
        let grass_with = |nudge_m: f32| {
            let grass = GrassCullSettings { near_m: settings.grass.near_m + nudge_m, fade_m: settings.grass.fade_m + nudge_m, edge_cap_m: settings.grass.edge_cap_m + nudge_m, ..settings.grass };
            let view = CullView { planes: view.planes.map(|plane| plane + Vec4::W * nudge_m), ..view };
            cull_grass(&blades, &chunks, &grass, &view)
        };
        let grass = grass_with(0.0);
        let overflow = trees.overflow_for(config, VegKind::Tree) + rocks.overflow_for(config, VegKind::Rock) + grass.overflow_for(config, VegKind::Grass);
        let exact = CullStats { overflow, rock_visible: rocks.stats.rock_visible, grass_near: grass.stats.grass_near, grass_mid: grass.stats.grass_mid, ..trees.stats };
        Twin { exact, grass_low: grass_with(-GRASS_ROUNDING_M).stats, grass_high: grass_with(GRASS_ROUNDING_M).stats }
    }

    fn assert_planes_follow_the_camera(&mut self) {
        let view = self.app.world().resource::<VegWitness>().latest_view().unwrap();
        let (transform, camera) = self.app.world_mut().query::<(&GlobalTransform, &Camera)>().get(self.app.world(), self.camera).unwrap();
        let eye_relative = frustum_planes(camera.clip_from_view() * Mat4::from_quat(transform.rotation().inverse()));
        assert_eq!(view.eye, transform.translation());
        for (got, want) in view.planes.iter().zip(eye_relative) {
            assert!((got.truncate() - want.truncate()).length() < PLANE_NORMAL_TOLERANCE && (got.w - want.w).abs() < PLANE_OFFSET_TOLERANCE_M, "cull plane {got} against the eye-relative plane {want}");
        }
    }
}

struct Twin {
    exact: CullStats,
    grass_low: CullStats,
    grass_high: CullStats,
}

impl Twin {
    fn assert_matches(&self, gpu: &CullStats) {
        let without_grass = |stats: &CullStats| CullStats { grass_near: 0, grass_mid: 0, ..*stats };
        assert_eq!(without_grass(gpu), without_grass(&self.exact), "trees and rocks against the twin");
        let total = |stats: &CullStats| stats.grass_near + stats.grass_mid;
        assert!((self.grass_low.grass_near..=self.grass_high.grass_near).contains(&gpu.grass_near), "grass near {gpu:?} outside {:?} to {:?}", self.grass_low, self.grass_high);
        assert!((total(&self.grass_low)..=total(&self.grass_high)).contains(&total(gpu)), "grass drawn {gpu:?} outside {:?} to {:?}", self.grass_low, self.grass_high);
        eprintln!("grass twin exact {:?}, within {GRASS_ROUNDING_M} m of a threshold {:?} to {:?}", self.exact, (self.grass_low.grass_near, self.grass_low.grass_mid), (self.grass_high.grass_near, self.grass_high.grass_mid));
    }
}

fn expected_draw_sets(config: &VegGpuConfig) -> usize {
    (config.tree_geo_count() + 1 + config.rock_kinds + 1) as usize
}

fn draw_slots(config: &VegGpuConfig) -> u32 {
    config.tree_geo_count() * MESH_TIERS + 1 + config.rock_kinds + GRASS_SLOTS
}

#[test]
fn the_forest_census_streams_the_reference_rings_onto_the_gpu() {
    let _serial = render_lock();
    let mut census = Census::new();
    let stats = census.settle("the anchor pose");
    assert_eq!(guard::builds_on_main_thread(), 0);
    let entities = census.entities();
    let player = player_world();
    let stream = census.stream();
    assert_eq!(loaded_keys(stream, VegKind::Tree), ring_keys(trees_fixture()));
    assert_eq!(loaded_keys(stream, VegKind::Tree).len(), TREE_RING_CHUNKS);
    assert_eq!(loaded_keys(stream, VegKind::Rock), ring_around(player, RINGS[1].load_m));
    assert!(ring_keys(rocks_fixture()).is_subset(&loaded_keys(stream, VegKind::Rock)) && ring_keys(rocks_fixture()).len() == ROCK_RING_CHUNKS);
    assert_eq!(loaded_keys(stream, VegKind::Grass), ring_keys(grass_fixture()));
    assert_eq!(loaded_keys(stream, VegKind::Grass).len(), GRASS_RING_CHUNKS);
    let store = census.app.world().resource::<VegStore>();
    let rock_kind = &stream.kinds[VegKind::Rock.index()];
    eprintln!("census: rock records in the 360 m ring {}", store.resident_records(VegKind::Rock));
    let rocks_in_fixture_ring: u32 = ring_keys(rocks_fixture()).iter().filter_map(|key| rock_kind.loaded[key]).filter_map(|chunk| store.chunk_range(chunk, VegKind::Rock)).map(|(_, len)| len).sum();
    assert_eq!((store.resident_records(VegKind::Tree), rocks_in_fixture_ring, store.resident_records(VegKind::Grass)), (TREE_RING_PLACED, ROCK_RING_PLACED, GRASS_RING_PLACED));
    assert_eq!(census.app.world().resource::<VegStreamStats>().refused_chunks, 0);
    assert_eq!(stats.tree_resident, TREE_RING_RESIDENT);
    assert_eq!(stats.overflow, 0);
    census.assert_planes_follow_the_camera();
    census.twin().assert_matches(&stats);
    assert!(stats.tree_mesh + stats.tree_impostor > 0 && stats.rock_visible > 0 && stats.grass_near + stats.grass_mid > 0, "{stats:?}");
    let draws = census.app.world().resource::<VegWitness>().draws();
    assert!(draws > 0 && draws <= draw_slots(census.app.world().resource::<VegGpuConfig>()), "{draws} draws");
    let first_trees = loaded_keys(census.stream(), VegKind::Tree);
    let second = along_north(player, SECOND_POSE_M);
    census.place(second);
    let moved = census.settle("the second pose");
    assert_eq!(census.entities(), entities);
    assert_eq!(moved.overflow, 0);
    let ring = ring_around(second, REFERENCE_TREE_RING_M);
    assert_eq!(loaded_keys(census.stream(), VegKind::Tree), ring.union(&within(second, REFERENCE_TREE_DROP_M, &first_trees)).copied().collect());
    assert_ne!(moved, stats);
    assert_eq!(census.peak_ring_work, RING_WORK_PER_FRAME);
    let store = census.app.world().resource::<VegStore>();
    let resident = VegKind::ALL.map(|kind| store.resident_records(kind));
    eprintln!("census: anchor {stats:?}, draws {draws}, entities {entities}, second pose {moved:?}, records {resident:?}, stream {:?}", census.app.world().resource::<VegStreamStats>());
}
