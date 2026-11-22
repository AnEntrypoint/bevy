use super::VegGpuPlugin;
use super::buffers::{GrassRecord, RockRecord, TreeRecord, VegBounds, VegGpu, VegGpuConfig, VegKind, EncodedRecords};
use super::cull::{CullStats, TreeCullSettings, VegCullSettings, VegWitness};
use super::draw::{VegCamera, VegDrawSet, VegMaterial};
use super::store::{ChunkRef, VegStore};
use super::twin::{TwinBlade, TwinOutcome, TwinRock, TwinTree, cull_grass, cull_rocks, cull_trees};
use crate::offscreen::{Frame, SKY_COLOUR, add_target, capture, headless_app, image_stats, mean_change, pump, render_lock, spawn_view};
use crate::planet_lod::origin::FloatingOrigin;
use bevy::asset::uuid_handle;
use bevy::math::DVec3;
use bevy::prelude::*;
use bevy::render::RenderApp;
use bevy::render::render_resource::{Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, MapMode, PollType, TextureFormat};
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::shader::Shader;
use std::collections::{HashMap, HashSet};

const TEST_SHADER: Handle<Shader> = uuid_handle!("0a8e2c17-5d43-4b96-8f1e-7c3b9d2a6e40");
const TEST_SOURCE: &str = include_str!("test_material.wgsl");
const CHUNK_M: f32 = 32.0;
const PUMP_LIMIT: usize = 600;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }

    fn unit(&mut self) -> f32 {
        self.next() as f32 / 2147483648.0
    }

    fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
    }

    fn quat(&mut self, spread: f32) -> [f32; 4] {
        Quat::from_euler(EulerRot::YXZ, self.range(-3.1, 3.1), self.range(-spread, spread), self.range(-spread, spread)).to_array()
    }
}

struct Rig {
    app: App,
    camera: Entity,
    target: Handle<Image>,
    witness: VegWitness,
    config: VegGpuConfig,
    origin: FloatingOrigin,
}

fn rig(config: VegGpuConfig) -> Rig {
    let mut app = headless_app();
    let origin = FloatingOrigin { position: DVec3::new(6_360_000.0, 345.0, -2_100.0) };
    app.add_plugins(VegGpuPlugin { config: config.clone() });
    app.insert_resource(origin);
    app.finish();
    app.cleanup();
    let target = add_target(&mut app, TextureFormat::Rgba8UnormSrgb);
    let frame = Frame { position: DVec3::ZERO, forward: DVec3::NEG_Z, up: DVec3::Y, sun_to: DVec3::new(0.3, 0.9, 0.2).normalize() };
    spawn_view(&mut app, &target, SKY_COLOUR, &frame, 1.0);
    let camera = app.world_mut().query_filtered::<Entity, With<Camera3d>>().single(app.world()).unwrap();
    let witness = app.world().resource::<VegWitness>().clone();
    Rig { app, camera, target, witness, config, origin }
}

impl Rig {
    fn drain_uploads(&mut self) {
        for _ in 0..PUMP_LIMIT {
            let store = self.app.world().resource::<VegStore>();
            if VegKind::ALL.iter().all(|kind| store.pending_bytes(*kind) == 0) {
                break;
            }
            pump(&mut self.app);
        }
        pump(&mut self.app);
        pump(&mut self.app);
        let store = self.app.world().resource::<VegStore>();
        assert!(VegKind::ALL.iter().all(|kind| store.pending_bytes(*kind) == 0), "uploads did not drain");
    }

    fn enable_view(&mut self) {
        self.app.world_mut().entity_mut(self.camera).insert(VegCamera);
    }

    fn place_camera(&mut self, eye: Vec3, direction: Vec3) {
        *self.app.world_mut().get_mut::<Transform>(self.camera).unwrap() = Transform::from_translation(eye).looking_to(direction, Vec3::Y);
    }

    fn dispatched_step(&mut self, eye: Vec3, direction: Vec3) {
        self.place_camera(eye, direction);
        let before = self.witness.dispatches();
        for _ in 0..8 {
            pump(&mut self.app);
            if self.witness.dispatches() != before {
                break;
            }
        }
        assert_eq!(self.witness.dispatches(), before + 1, "exactly one cull dispatch per stepped frame");
    }

    fn render_world(&self) -> &World {
        self.app.sub_app(RenderApp).world()
    }

    fn gpu_buffer(&self, pick: impl Fn(&VegGpu) -> &Buffer) -> Buffer {
        pick(self.render_world().resource::<VegGpu>()).clone()
    }

    fn read(&self, buffer: &Buffer, bytes: u64) -> Vec<u8> {
        let device = self.render_world().resource::<RenderDevice>();
        let queue = self.render_world().resource::<RenderQueue>();
        let staging = device.create_buffer(&BufferDescriptor { label: None, size: bytes, usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST, mapped_at_creation: false });
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());
        encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, bytes);
        queue.submit(std::iter::once(encoder.finish()));
        staging.slice(..).map_async(MapMode::Read, |result| result.unwrap());
        device.wgpu_device().poll(PollType::Wait { submission_index: None, timeout: None }).unwrap();
        let data = staging.slice(..).get_mapped_range().unwrap().to_vec();
        staging.unmap();
        data
    }

    fn read_words(&self, pick: impl Fn(&VegGpu) -> &Buffer, words: usize) -> Vec<u32> {
        let buffer = self.gpu_buffer(pick);
        self.read(&buffer, words as u64 * 4).chunks_exact(4).map(|word| u32::from_le_bytes(word.try_into().unwrap())).collect()
    }
}

struct Scene {
    chunks: Vec<ChunkRef>,
    trees: Vec<TwinTree>,
    rocks: Vec<TwinRock>,
    blades: Vec<TwinBlade>,
    bounds: VegBounds,
}

struct ChunkBucket {
    chunk: ChunkRef,
    trees: Vec<TreeRecord>,
    rocks: Vec<RockRecord>,
    blades: Vec<GrassRecord>,
}

fn synthetic_bounds(config: &VegGpuConfig) -> VegBounds {
    VegBounds {
        tree_geometry: (0..config.tree_geo_count()).map(|geo| [0.3, 1.2, -0.4, 1.5 + 0.1 * geo as f32]).collect(),
        rock_kind: (0..config.rock_kinds).map(|kind| [0.1, 0.2, 0.0, 1.0 + 0.3 * kind as f32]).collect(),
        impostor_dims: vec![[0.0, 1.0, 0.0, 2.0]; config.tree_species as usize],
    }
}

fn bucket<'a>(rig: &mut Rig, buckets: &'a mut HashMap<(i32, i32), ChunkBucket>, rng: &mut Rng, x: f32, z: f32) -> (&'a mut ChunkBucket, Vec3) {
    let cell = ((x / CHUNK_M).floor() as i32, (z / CHUNK_M).floor() as i32);
    let centre = Vec3::new((cell.0 as f32 + 0.5) * CHUNK_M, 3.0, (cell.1 as f32 + 0.5) * CHUNK_M);
    let origin = rig.origin;
    let entry = buckets.entry(cell).or_insert_with(|| {
        let rotation = Quat::from_array(rng.quat(0.2));
        let chunk = rig.app.world_mut().resource_mut::<VegStore>().add_chunk(origin.position + centre.as_dvec3(), rotation).unwrap();
        ChunkBucket { chunk, trees: Vec::new(), rocks: Vec::new(), blades: Vec::new() }
    });
    (entry, Vec3::new(x - centre.x, 0.0, z - centre.z))
}

fn synthesize(rig: &mut Rig, trees: u32, rocks: u32, blades: u32) -> Scene {
    let mut rng = Rng(0x5eed);
    let mut buckets: HashMap<(i32, i32), ChunkBucket> = HashMap::new();
    let config = rig.config.clone();
    for index in 0..trees {
        let near = index % 5 == 0;
        let spread = if near { 60.0 } else { 640.0 };
        let (x, z) = (rng.range(-spread, spread), rng.range(-spread, spread));
        let (entry, local) = bucket(rig, &mut buckets, &mut rng, x, z);
        let id = (rng.next() << 1) ^ rng.next();
        entry.trees.push(TreeRecord {
            local_position: [local.x, rng.range(0.0, 6.0), local.z],
            scale: rng.range(0.7, 1.6),
            tilt: rng.quat(0.15),
            yaw: rng.range(0.0, 6.0),
            wind_phase: rng.unit(),
            tint_rgb8: id & 0xffffff,
            trunk_id: id,
            species: (rng.next() % config.tree_species) as u8,
            shape: (rng.next() % config.tree_shapes) as u8,
        });
    }
    for _ in 0..rocks {
        let (x, z) = (rng.range(-450.0, 450.0), rng.range(-450.0, 450.0));
        let (entry, local) = bucket(rig, &mut buckets, &mut rng, x, z);
        entry.rocks.push(RockRecord {
            local_position: [local.x, rng.range(0.0, 3.0), local.z],
            scale: rng.range(0.5, 4.0),
            rotation: rng.quat(0.4),
            squash: rng.range(0.55, 1.1),
            variant: rng.unit(),
            kind: rng.next() % config.rock_kinds,
        });
    }
    for _ in 0..blades {
        let (x, z) = (rng.range(-64.0, 64.0), rng.range(-64.0, 64.0));
        let (entry, local) = bucket(rig, &mut buckets, &mut rng, x, z);
        entry.blades.push(GrassRecord {
            local_position: [local.x, rng.range(0.0, 1.0), local.z],
            scale: rng.range(0.5, 1.5),
            rotation: rng.quat(0.2),
            ground_rgb: [0.2, 0.4, 0.1],
            tint: rng.unit(),
            rank: rng.range(0.05, 1.0),
            wind_phase: rng.unit(),
            shadow: 0.0,
        });
    }
    let mut scene = Scene { chunks: Vec::new(), trees: Vec::new(), rocks: Vec::new(), blades: Vec::new(), bounds: synthetic_bounds(&config) };
    let mut ordered: Vec<_> = buckets.into_values().collect();
    ordered.sort_by_key(|bucket| bucket.chunk.slot);
    scene.chunks = ordered.iter().map(|bucket| bucket.chunk).collect();
    let placeholder_tree = TwinTree { record: TreeRecord::default(), chunk: u32::MAX };
    scene.trees.resize(trees as usize, placeholder_tree);
    scene.rocks.resize(rocks as usize, TwinRock { record: RockRecord::default(), chunk: u32::MAX });
    scene.blades.resize(blades as usize, TwinBlade { record: GrassRecord::default(), chunk: u32::MAX });
    let mut store = rig.app.world_mut().resource_mut::<VegStore>();
    for bucket in &ordered {
        let packed = bucket.chunk.packed();
        if !bucket.trees.is_empty() {
            store.upload(bucket.chunk, 10.0, EncodedRecords::new(&bucket.trees)).unwrap();
            let first = store.chunk_range(bucket.chunk, VegKind::Tree).unwrap().0 as usize;
            bucket.trees.iter().enumerate().for_each(|(at, record)| scene.trees[first + at] = TwinTree { record: *record, chunk: packed });
        }
        if !bucket.rocks.is_empty() {
            store.upload(bucket.chunk, 10.0, EncodedRecords::new(&bucket.rocks)).unwrap();
            let first = store.chunk_range(bucket.chunk, VegKind::Rock).unwrap().0 as usize;
            bucket.rocks.iter().enumerate().for_each(|(at, record)| scene.rocks[first + at] = TwinRock { record: *record, chunk: packed });
        }
        if !bucket.blades.is_empty() {
            store.upload(bucket.chunk, 10.0, EncodedRecords::new(&bucket.blades)).unwrap();
            let first = store.chunk_range(bucket.chunk, VegKind::Grass).unwrap().0 as usize;
            bucket.blades.iter().enumerate().for_each(|(at, record)| scene.blades[first + at] = TwinBlade { record: *record, chunk: packed });
        }
    }
    *rig.app.world_mut().resource_mut::<VegBounds>() = scene.bounds.clone();
    scene
}

struct Twin {
    trees: TwinOutcome,
    rocks: TwinOutcome,
    grass: TwinOutcome,
    stats: CullStats,
}

fn admitted(outcome: &TwinOutcome, config: &VegGpuConfig, kind: VegKind, slots: std::ops::Range<u32>) -> u32 {
    slots.map(|slot| (outcome.lists[slot as usize].len() as u32).min(config.slot_capacity(kind, slot))).sum()
}

fn twin_step(rig: &Rig, scene: &Scene, previous: Option<&Twin>) -> Twin {
    let settings = *rig.app.world().resource::<VegCullSettings>();
    let view = rig.witness.latest_view().expect("the cull saw a view");
    let store = rig.app.world().resource::<VegStore>();
    let chunks: Vec<_> = (0..super::store::CHUNK_SLOTS).map(|slot| store.table().record(slot, &rig.origin)).collect();
    let tree_states = previous.map_or(vec![0; scene.trees.len()], |twin| twin.trees.states.clone());
    let rock_states = previous.map_or(vec![0; scene.rocks.len()], |twin| twin.rocks.states.clone());
    let trees = cull_trees(&scene.trees, &chunks, &scene.bounds.tree_geometry, &rig.config, &settings.tree, &view, &tree_states);
    let rocks = cull_rocks(&scene.rocks, &chunks, &scene.bounds.rock_kind, &rig.config, &settings.rock, &view, &rock_states);
    let grass = cull_grass(&scene.blades, &chunks, &settings.grass, &view);
    let stats = CullStats {
        overflow: trees.overflow_for(&rig.config, VegKind::Tree) + rocks.overflow_for(&rig.config, VegKind::Rock) + grass.overflow_for(&rig.config, VegKind::Grass),
        rock_visible: admitted(&rocks, &rig.config, VegKind::Rock, 0..rig.config.rock_kinds),
        grass_near: admitted(&grass, &rig.config, VegKind::Grass, 0..1),
        grass_mid: admitted(&grass, &rig.config, VegKind::Grass, 1..2),
        ..trees.stats
    };
    Twin { trees, rocks, grass, stats }
}

fn assert_lists(label: &str, config: &VegGpuConfig, kind: VegKind, gpu_args: &[u32], gpu_visible: &[u32], twin: &TwinOutcome) {
    for slot in 0..config.slot_count(kind) {
        let count = gpu_args[slot as usize * 5 + 1];
        let capacity = config.slot_capacity(kind, slot);
        let offset = config.list_offset(kind, slot) as usize;
        let expected = &twin.lists[slot as usize];
        assert!(count <= capacity, "{label} {kind:?} slot {slot}: count {count} above capacity {capacity}");
        assert_eq!(gpu_args[slot as usize * 5 + 4], 0, "{label} {kind:?} slot {slot}: first_instance");
        let listed = &gpu_visible[offset..offset + count as usize];
        if expected.len() as u32 <= capacity {
            assert_eq!(count as usize, expected.len(), "{label} {kind:?} slot {slot}: count");
            let mut got = listed.to_vec();
            got.sort_unstable();
            let mut want = expected.clone();
            want.sort_unstable();
            assert_eq!(got, want, "{label} {kind:?} slot {slot}: index set");
        } else {
            assert_eq!(count, capacity, "{label} {kind:?} slot {slot}: clamped count");
            let allowed: HashSet<u32> = expected.iter().copied().collect();
            assert!(listed.iter().all(|index| allowed.contains(index)), "{label} {kind:?} slot {slot}: clamped list holds foreign indices");
        }
    }
}

fn assert_gpu_matches(rig: &Rig, scene: &Scene, twin: &Twin, label: &str) {
    let config = &rig.config;
    for kind in VegKind::ALL {
        let args = rig.read_words(|gpu| &gpu.draws[kind.index()], config.slot_count(kind) as usize * 5);
        let visible = rig.read_words(|gpu| &gpu.visible[kind.index()], config.visible_len(kind) as usize);
        let outcome = match kind {
            VegKind::Tree => &twin.trees,
            VegKind::Rock => &twin.rocks,
            VegKind::Grass => &twin.grass,
        };
        assert_lists(label, config, kind, &args, &visible, outcome);
    }
    let stats = CullStats::from_bytes(&rig.read(&rig.gpu_buffer(|gpu| &gpu.stats), 48));
    assert_eq!(stats, twin.stats, "{label}: stats");
    assert_eq!(rig.read_words(|gpu| &gpu.tree_state, scene.trees.len()), twin.trees.states, "{label}: tree hysteresis states");
    assert_eq!(rig.read_words(|gpu| &gpu.rock_state, scene.rocks.len()), twin.rocks.states, "{label}: rock states");
}

fn camera_path() -> Vec<(Vec3, Vec3)> {
    let mut path = Vec::new();
    for step in 0..44 {
        let yaw = step as f32 * 0.17;
        path.push((Vec3::new(-20.0 + step as f32 * 0.9, 2.0, 5.0), Vec3::new(yaw.sin(), -0.05, -yaw.cos())));
    }
    for step in 0..44 {
        let yaw = (44 + step) as f32 * 0.17;
        path.push((Vec3::new(19.6 - step as f32 * 0.9, 2.0, 5.0), Vec3::new(yaw.sin(), 0.1, -yaw.cos())));
    }
    path
}

#[test]
fn veg_cull_parity() {
    let _serial = render_lock();
    let mut rig = rig(VegGpuConfig::default());
    let scene = synthesize(&mut rig, 30_000, 3000, 60_000);
    rig.drain_uploads();
    rig.enable_view();
    let mut previous: Option<Twin> = None;
    let mut seen = CullStats::default();
    let mut tiers_seen = [false; 3];
    for (index, (eye, direction)) in camera_path().into_iter().enumerate() {
        if index == 10 {
            let mut store = rig.app.world_mut().resource_mut::<VegStore>();
            scene.chunks.iter().step_by(13).for_each(|chunk| assert!(store.remove_chunk(*chunk)));
        }
        rig.dispatched_step(eye, direction);
        let twin = twin_step(&rig, &scene, previous.as_ref());
        assert_gpu_matches(&rig, &scene, &twin, &format!("frame {index}"));
        assert_eq!(twin.stats.overflow, 0);
        seen.tree_mesh += twin.stats.tree_mesh;
        seen.tree_impostor += twin.stats.tree_impostor;
        seen.tree_shadow += twin.stats.tree_shadow;
        seen.tree_thinned += twin.stats.tree_thinned;
        seen.tree_frustum_culled += twin.stats.tree_frustum_culled;
        seen.rock_visible += twin.stats.rock_visible;
        seen.grass_near += twin.stats.grass_near;
        seen.grass_mid += twin.stats.grass_mid;
        for tier in 0..3 {
            tiers_seen[tier] |= (0..rig.config.tree_geo_count()).any(|geo| !twin.trees.lists[rig.config.tree_slot(geo, tier as u32) as usize].is_empty());
        }
        previous = Some(twin);
    }
    assert_eq!(tiers_seen, [true; 3]);
    for (name, value) in [("mesh", seen.tree_mesh), ("impostor", seen.tree_impostor), ("shadow", seen.tree_shadow), ("thinned", seen.tree_thinned), ("frustum", seen.tree_frustum_culled), ("rock", seen.rock_visible), ("grass near", seen.grass_near), ("grass mid", seen.grass_mid)] {
        assert!(value > 0, "the path never exercised {name}");
    }
}

#[test]
fn veg_cull_clamps_to_capacity_and_counts_the_overflow() {
    let _serial = render_lock();
    let config = VegGpuConfig { tier_capacity: [8, 8, 8, 8], impostor_capacity: 32, rock_slot_capacity: 20, grass_slot_capacity: 200, ..VegGpuConfig::default() };
    let mut rig = rig(config);
    let scene = synthesize(&mut rig, 12_000, 1500, 20_000);
    rig.drain_uploads();
    rig.enable_view();
    let mut previous: Option<Twin> = None;
    for (index, (eye, direction)) in camera_path().into_iter().take(6).enumerate() {
        rig.dispatched_step(eye, direction);
        let twin = twin_step(&rig, &scene, previous.as_ref());
        assert!(twin.stats.overflow > 0, "the synthetic set must exceed a capacity");
        assert_gpu_matches(&rig, &scene, &twin, &format!("clamped frame {index}"));
        previous = Some(twin);
    }
}

#[test]
fn veg_stats_are_read_back_asynchronously() {
    let _serial = render_lock();
    let mut rig = rig(VegGpuConfig::default());
    let scene = synthesize(&mut rig, 4000, 300, 5000);
    rig.drain_uploads();
    rig.enable_view();
    rig.witness.readback.store(true, std::sync::atomic::Ordering::SeqCst);
    rig.dispatched_step(Vec3::new(0.0, 2.0, 0.0), Vec3::NEG_Z);
    for _ in 0..6 {
        rig.dispatched_step(Vec3::new(0.0, 2.0, 0.0), Vec3::NEG_Z);
    }
    let direct = CullStats::from_bytes(&rig.read(&rig.gpu_buffer(|gpu| &gpu.stats), 48));
    assert_eq!(rig.witness.latest_stats(), Some(direct));
    assert!(direct.tree_resident > 0 && scene.trees.len() == 4000);
}

fn material() -> VegMaterial {
    VegMaterial { shader: TEST_SHADER, vertex_entry: "vs_test", fragment_entry: "fs_test", group2: None, cull_back: true }
}

#[derive(Clone, Copy, PartialEq)]
enum Variation {
    Baseline,
    CullOff,
    BrokenArgs,
}

struct Rendered {
    image: Image,
    entities: usize,
    draw_sets: usize,
    draws: u32,
    index_counts: Vec<u32>,
    first_indices: Vec<u32>,
    base_vertices: Vec<u32>,
    instance_counts: Vec<u32>,
}

fn render_scene(instances: u32, variation: Variation) -> Rendered {
    let _serial = render_lock();
    let config = VegGpuConfig { tree_species: 4, instance_capacity: [20_000, 100, 100], ..VegGpuConfig::default() };
    let mut rig = rig(config.clone());
    rig.app.world_mut().resource_mut::<Assets<Shader>>().insert(TEST_SHADER.id(), Shader::from_wgsl(TEST_SOURCE, "test_material.wgsl")).unwrap();
    let tiers: [Handle<Mesh>; 3] = [1.0f32, 1.1, 1.2].map(|size| rig.app.world_mut().resource_mut::<Assets<Mesh>>().add(Mesh::from(Cuboid::new(size, size, size))));
    for geometry in 0..config.tree_geo_count() {
        rig.app.world_mut().spawn(VegDrawSet::tree(&config, geometry, tiers.clone(), material()));
    }
    let mut rng = Rng(77);
    let chunk = rig.app.world_mut().resource_mut::<VegStore>().add_chunk(rig.origin.position + DVec3::new(0.0, 0.0, -40.0), Quat::IDENTITY).unwrap();
    let trees: Vec<_> = (0..instances)
        .map(|_| TreeRecord {
            local_position: [rng.range(-30.0, 30.0), rng.range(-4.0, 4.0), rng.range(-30.0, 30.0)],
            scale: rng.range(1.5, 3.0),
            tilt: [0.0, 0.0, 0.0, 1.0],
            trunk_id: rng.next(),
            species: (rng.next() % 4) as u8,
            shape: (rng.next() % 3) as u8,
            ..TreeRecord::default()
        })
        .collect();
    rig.app.world_mut().resource_mut::<VegStore>().upload(chunk, 10.0, EncodedRecords::new(&trees)).unwrap();
    *rig.app.world_mut().resource_mut::<VegBounds>() = VegBounds { tree_geometry: vec![[0.0, 0.0, 0.0, 1.0]; config.tree_geo_count() as usize], ..VegBounds::default() };
    if variation == Variation::CullOff {
        let mut settings = VegCullSettings::default();
        settings.tree = TreeCullSettings { ring_enter_m: 0.0, ring_drop_m: 0.0, ..settings.tree };
        rig.app.insert_resource(settings);
    }
    rig.drain_uploads();
    rig.enable_view();
    rig.place_camera(Vec3::ZERO, Vec3::NEG_Z);
    for _ in 0..4 {
        pump(&mut rig.app);
    }
    if variation == Variation::BrokenArgs {
        let draws = rig.gpu_buffer(|gpu| &gpu.draws[0]);
        let queue = rig.render_world().resource::<RenderQueue>();
        queue.write_buffer(&draws, 0, &vec![0u8; config.slot_count(VegKind::Tree) as usize * 20]);
    }
    let entities = rig.app.world_mut().query::<Entity>().iter(rig.app.world()).count();
    let draw_sets = rig.app.world_mut().query::<&VegDrawSet>().iter(rig.app.world()).count();
    let image = capture(&mut rig.app, rig.target.clone(), "veg draw");
    let args = rig.read_words(|gpu| &gpu.draws[0], config.slot_count(VegKind::Tree) as usize * 5);
    let slots = |field: usize| (0..config.tree_geo_count()).flat_map(|geo| (0..3).map(move |tier| (geo, tier))).map(|(geo, tier)| args[config.tree_slot(geo, tier) as usize * 5 + field]).collect();
    Rendered { image, entities, draw_sets, draws: rig.witness.draws(), index_counts: slots(0), first_indices: slots(2), base_vertices: slots(3), instance_counts: slots(1) }
}

#[test]
fn veg_draws_instances_through_the_indirect_path_and_the_cull_and_args_matter() {
    let baseline = render_scene(1000, Variation::Baseline);
    let stats = image_stats(&baseline.image);
    assert!(stats.covered > 0.01, "the instances cover {}", stats.covered);
    assert!(stats.distinct_colours > 8, "{} colours", stats.distinct_colours);
    assert!(baseline.index_counts.iter().all(|count| *count == 36), "cube index counts {:?}", baseline.index_counts);
    let distinct = |values: &[u32]| values.iter().collect::<HashSet<_>>().len();
    assert_eq!(distinct(&baseline.first_indices[..3]), 3, "tier meshes sit at distinct index offsets {:?}", baseline.first_indices);
    assert_eq!(distinct(&baseline.base_vertices[..3]), 3, "tier meshes sit at distinct vertex offsets {:?}", baseline.base_vertices);
    assert!(baseline.instance_counts.iter().sum::<u32>() > 100);
    let culled = render_scene(1000, Variation::CullOff);
    assert!(image_stats(&culled.image).covered < 1e-4, "cull off still draws {}", image_stats(&culled.image).covered);
    assert!(culled.instance_counts.iter().all(|count| *count == 0));
    assert!(mean_change(&baseline.image, &culled.image) > 0.005);
    let broken = render_scene(1000, Variation::BrokenArgs);
    assert!(image_stats(&broken.image).covered < 1e-4, "broken args still draw {}", image_stats(&broken.image).covered);
    assert!(mean_change(&baseline.image, &broken.image) > 0.005);
}

#[test]
fn veg_draw_calls_and_entities_do_not_grow_with_instances() {
    let small = render_scene(1000, Variation::Baseline);
    let large = render_scene(10_000, Variation::Baseline);
    assert_eq!(small.entities, large.entities);
    assert_eq!(small.draw_sets, 12);
    assert_eq!(large.draw_sets, 12);
    assert!(small.draw_sets <= 54);
    assert_eq!(small.draws, large.draws);
    assert_eq!(small.draws, 36);
    assert!(large.draws <= VegGpuConfig { tree_species: 4, ..VegGpuConfig::default() }.slot_count(VegKind::Tree));
    assert!(image_stats(&large.image).covered > image_stats(&small.image).covered * 0.9);
}
