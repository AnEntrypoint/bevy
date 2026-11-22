use super::buffers::{EncodedRecords, GrassRecord, RockRecord, TreeRecord, VegGpuConfig, VegKind, tint_rgb8};
use super::cull::{CullView, TreeCullSettings};
use super::twin::{cull_trees, keep_probability, sphere_in_frustum, TwinTree};
use super::store::{CHUNK_TABLE_BYTES, ChunkTable, RangeAllocator, UploadError, UploadQueue, UploadWrite, VegStore};
use crate::planet_lod::origin::FloatingOrigin;
use bevy::camera::CameraProjection;
use bevy::math::{DVec3, Mat4, Quat, Vec3};
use bevy::prelude::{PerspectiveProjection, Transform};

fn tree(seed: u32) -> TreeRecord {
    TreeRecord { local_position: [seed as f32, 2.0, 3.0], scale: 1.5, tilt: [0.0, 0.0, 0.0, 1.0], yaw: 0.25, wind_phase: 0.5, tint_rgb8: 7, trunk_id: seed, species: (seed % 15) as u8, shape: (seed % 3) as u8 }
}

fn trees(count: u32, first: u32) -> EncodedRecords {
    EncodedRecords::new(&(first..first + count).map(tree).collect::<Vec<_>>())
}

fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

fn apply(buffer: &mut [u8], writes: &[UploadWrite]) {
    writes.iter().for_each(|write| buffer[write.byte_offset as usize..write.byte_offset as usize + write.bytes.len()].copy_from_slice(&write.bytes));
}

#[test]
fn veg_record_sizes_and_offsets_match_the_wgsl_structs() {
    let encoded = EncodedRecords::new(&[tree(9)]);
    assert_eq!(encoded.bytes.len(), 64);
    assert_eq!(f32::from_le_bytes(encoded.bytes[12..16].try_into().unwrap()), 1.5);
    assert_eq!(word(&encoded.bytes, 40), 7);
    assert_eq!(word(&encoded.bytes, 44), 9);
    assert_eq!(word(&encoded.bytes, 48), 9 % 15 | (9 % 3) << 8);
    assert_eq!(EncodedRecords::new(&[RockRecord { kind: 4, ..Default::default() }]).bytes.len(), 48);
    assert_eq!(word(&EncodedRecords::new(&[RockRecord { kind: 4, ..Default::default() }]).bytes, 40), 4);
    let grass = EncodedRecords::new(&[GrassRecord { ground_rgb: [0.1, 0.2, 0.3], tint: 0.5, ..Default::default() }]);
    assert_eq!(grass.bytes.len(), 64);
    assert_eq!(f32::from_le_bytes(grass.bytes[32..36].try_into().unwrap()), 0.1);
    assert_eq!(f32::from_le_bytes(grass.bytes[44..48].try_into().unwrap()), 0.5);
    for kind in VegKind::ALL {
        assert_eq!(kind.chunk_word_offset() + 4 + if kind == VegKind::Tree { 8 } else { 0 }, kind.record_bytes());
    }
    assert_eq!(tint_rgb8([2.0, 1.0, 0.0]), 255 | 128 << 8);
}

#[test]
fn veg_default_slot_layout_is_the_design_table() {
    let config = VegGpuConfig::default();
    assert_eq!(config.tree_geo_count(), 45);
    assert_eq!(config.slot_count(VegKind::Tree), 181);
    assert_eq!(config.visible_len(VegKind::Tree), 205_568);
    assert_eq!(config.slot_count(VegKind::Rock), 6);
    assert_eq!(config.slot_count(VegKind::Grass), 2);
    assert_eq!([config.slot_capacity(VegKind::Tree, 0), config.slot_capacity(VegKind::Tree, 1), config.slot_capacity(VegKind::Tree, 2), config.slot_capacity(VegKind::Tree, 3)], [256, 1024, 2048, 512]);
    assert_eq!(config.slot_capacity(VegKind::Tree, config.impostor_slot()), 32768);
    let layout = config.tree_slot_layout();
    let mut end = 0;
    for [offset, capacity] in layout {
        assert_eq!(offset, end);
        end = offset + capacity;
    }
    assert_eq!(end, config.visible_len(VegKind::Tree));
}

#[test]
fn veg_chunk_table_is_fixed_size_rebased_and_generation_checked() {
    let mut table = ChunkTable::default();
    let origin = FloatingOrigin { position: DVec3::new(6_360_000.0, 120.0, -40.0) };
    let anchor = origin.position + DVec3::new(123.456, 7.0, -80.25);
    let chunk = table.load(anchor, Quat::from_rotation_y(0.5)).unwrap();
    assert!(table.is_live(chunk));
    let record = table.record(usize::from(chunk.slot), &origin);
    assert!((record.origin.as_dvec3() - DVec3::new(123.456, 7.0, -80.25)).length() < 1e-4);
    assert_eq!(record.generation, u32::from(chunk.generation));
    assert_eq!(table.encode(&origin).len(), CHUNK_TABLE_BYTES);
    assert_eq!(CHUNK_TABLE_BYTES, 64 * 1024);
    let moved = FloatingOrigin { position: origin.position + DVec3::new(1000.0, 0.0, 0.0) };
    assert!((table.record(usize::from(chunk.slot), &moved).origin.x + 876.544).abs() < 1e-3);
    assert!(table.unload(chunk));
    assert!(!table.is_live(chunk));
    assert!(!table.unload(chunk));
    assert_ne!(table.generation(chunk.slot), chunk.generation);
    assert_eq!(table.record(usize::from(chunk.slot), &origin).generation, u32::MAX);
}

#[test]
fn veg_chunk_generation_never_repeats_zero_after_many_reloads() {
    let mut table = ChunkTable::default();
    for _ in 0..70_000 {
        let chunk = table.load(DVec3::ZERO, Quat::IDENTITY).unwrap();
        assert_ne!(chunk.generation, 0);
        table.unload(chunk);
    }
}

#[test]
fn veg_range_allocator_coalesces_and_never_overlaps() {
    let mut allocator = RangeAllocator::new(1000);
    let mut live: Vec<(u32, u32)> = Vec::new();
    let mut state = 12345u32;
    for _ in 0..400 {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        if state % 3 != 0 || live.is_empty() {
            let len = 1 + (state >> 8) % 60;
            if let Some(start) = allocator.alloc(len) {
                assert!(live.iter().all(|(s, l)| start + len <= *s || *s + *l <= start));
                live.push((start, len));
            }
        } else {
            let (start, len) = live.swap_remove((state >> 8) as usize % live.len());
            allocator.release(start, len);
        }
        assert_eq!(allocator.free_total() + live.iter().map(|(_, l)| l).sum::<u32>(), 1000);
    }
    live.drain(..).for_each(|(start, len)| allocator.release(start, len));
    assert_eq!(allocator.alloc(1000), Some(0));
}

#[test]
fn veg_upload_budget_is_never_exceeded_and_nothing_is_lost() {
    let mut store = VegStore::new(&VegGpuConfig::default());
    let origin = FloatingOrigin::default();
    let mut expected = vec![0u8; 32768 * 64];
    let mut chunks = Vec::new();
    for index in 0..60u32 {
        let chunk = store.add_chunk(DVec3::new(f64::from(index), 0.0, 0.0), Quat::IDENTITY).unwrap();
        let encoded = trees(1 + index * 7 % 300, index * 1000);
        let (first, _) = {
            store.upload(chunk, (index * 37 % 91) as f32, encoded.clone()).unwrap();
            store.chunk_range(chunk, VegKind::Tree).unwrap()
        };
        let mut bytes = encoded.bytes.clone();
        bytes.chunks_exact_mut(64).for_each(|record| record[52..56].copy_from_slice(&chunk.packed().to_le_bytes()));
        expected[first as usize * 64..first as usize * 64 + bytes.len()].copy_from_slice(&bytes);
        chunks.push(chunk);
    }
    let mut gpu = vec![0u8; 32768 * 64];
    let mut frames = 0;
    while store.pending_bytes(VegKind::Tree) > 0 {
        let frame = store.frame(&origin);
        let written: usize = frame.writes[0].iter().map(|write| write.bytes.len()).sum();
        assert!(written <= 64 * 1024);
        assert_eq!(frame.chunk_table.len(), CHUNK_TABLE_BYTES);
        apply(&mut gpu, &frame.writes[0]);
        frames += 1;
    }
    assert!(frames > 1);
    assert_eq!(gpu, expected);
    assert_eq!(store.frame(&origin).writes[0], Vec::new());
}

#[test]
fn veg_uploads_drain_nearest_chunk_first() {
    let mut store = VegStore::new(&VegGpuConfig::default());
    let origin = FloatingOrigin::default();
    let distances = [500.0f32, 20.0, 300.0, 5.0, 650.0, 90.0];
    let chunks: Vec<_> = distances
        .iter()
        .enumerate()
        .map(|(index, distance)| {
            let chunk = store.add_chunk(DVec3::ZERO, Quat::IDENTITY).unwrap();
            store.upload(chunk, *distance, trees(800, index as u32 * 1000)).unwrap();
            chunk
        })
        .collect();
    let mut order = Vec::new();
    while store.pending_bytes(VegKind::Tree) > 0 {
        for write in &store.frame(&origin).writes[0] {
            for record in write.bytes.chunks_exact(64) {
                let packed = word(record, 52);
                let chunk = chunks.iter().position(|chunk| chunk.packed() == packed).unwrap();
                if order.last() != Some(&chunk) {
                    order.push(chunk);
                }
            }
        }
    }
    let mut by_distance: Vec<usize> = (0..distances.len()).collect();
    by_distance.sort_by(|a, b| distances[*a].total_cmp(&distances[*b]));
    assert_eq!(order, by_distance);
}

#[test]
fn veg_large_grass_chunk_is_split_across_frames_by_sub_ranges() {
    let mut queue = UploadQueue::new(VegKind::Grass, VegKind::Grass.upload_budget_bytes());
    let chunk = super::store::ChunkRef { slot: 3, generation: 1 };
    let records = vec![GrassRecord { scale: 1.0, ..Default::default() }; 20224];
    let encoded = EncodedRecords::new(&records);
    queue.push(chunk, 10.0, 100, encoded.bytes.clone());
    let mut frames = 0;
    let mut gpu = vec![0u8; (100 + 20224) * 64];
    loop {
        let writes = queue.drain();
        if writes.is_empty() {
            break;
        }
        assert!(writes.iter().map(|write| write.bytes.len()).sum::<usize>() <= 512 * 1024);
        assert!(writes.iter().all(|write| write.bytes.len() % 64 == 0 && write.byte_offset % 64 == 0));
        apply(&mut gpu, &writes);
        frames += 1;
    }
    assert_eq!(frames, (20224usize * 64).div_ceil(512 * 1024));
    assert!(gpu[100 * 64..].chunks_exact(64).all(|record| word(record, 60) == chunk.packed() && f32::from_le_bytes(record[12..16].try_into().unwrap()) == 1.0));
}

#[test]
fn veg_unload_bumps_the_generation_drops_pending_uploads_and_frees_the_range() {
    let mut store = VegStore::new(&VegGpuConfig::default());
    let origin = FloatingOrigin::default();
    let chunk = store.add_chunk(DVec3::ZERO, Quat::IDENTITY).unwrap();
    store.upload(chunk, 1.0, trees(3000, 0)).unwrap();
    let first_frame = store.frame(&origin);
    assert!(!first_frame.writes[0].is_empty());
    assert!(store.pending_bytes(VegKind::Tree) > 0);
    assert_eq!(store.resident_records(VegKind::Tree), 3000);
    assert!(store.remove_chunk(chunk));
    assert!(!store.is_live(chunk));
    assert!(!store.remove_chunk(chunk));
    assert_eq!(store.pending_bytes(VegKind::Tree), 0);
    assert_eq!(store.resident_records(VegKind::Tree), 0);
    assert_eq!(store.frame(&origin).writes[0], Vec::new());
    assert_eq!(store.upload(chunk, 1.0, trees(1, 0)), Err(UploadError::StaleChunk));
    let next = store.add_chunk(DVec3::ZERO, Quat::IDENTITY).unwrap();
    assert_ne!(next.packed(), chunk.packed());
    assert_eq!(store.frame(&origin).instance_counts[0], 0);
}

#[test]
fn veg_store_refuses_double_upload_and_overflow() {
    let config = VegGpuConfig { instance_capacity: [100, 100, 100], ..Default::default() };
    let mut store = VegStore::new(&config);
    let chunk = store.add_chunk(DVec3::ZERO, Quat::IDENTITY).unwrap();
    assert_eq!(store.upload(chunk, 1.0, trees(60, 0)), Ok(()));
    assert_eq!(store.upload(chunk, 1.0, trees(1, 0)), Err(UploadError::AlreadyUploaded));
    let second = store.add_chunk(DVec3::ZERO, Quat::IDENTITY).unwrap();
    assert_eq!(store.upload(second, 1.0, trees(60, 0)), Err(UploadError::NoRoom));
    assert_eq!(store.upload(second, 1.0, trees(40, 0)), Ok(()));
}

fn near(value: f32, want: f32) {
    assert!((value - want).abs() < 1e-3, "{value} != {want}");
}

fn wide_projection() -> Mat4 {
    PerspectiveProjection { fov: std::f32::consts::FRAC_PI_2, aspect_ratio: 1.0, near: 0.1, far: 1000.0, ..Default::default() }.get_clip_from_view()
}

#[test]
fn veg_tree_tier_distances_match_the_reference_table() {
    let at_640 = TreeCullSettings::reference(640.0);
    near(at_640.d1_m, 14.0);
    near(at_640.d2_m, 35.0);
    near(at_640.impostor_start_m, 73.544);
    near(at_640.mesh_far_m, 79.544);
    near(at_640.mesh_reentry_m, 77.044);
    near(at_640.hysteresis, 0.12);
    near(at_640.ring_enter_m, 680.0);
    near(at_640.ring_drop_m, 744.0);
    near(at_640.falloff_near_m, 96.0);
    near(at_640.falloff_floor, 0.15);
    near(at_640.shadow_distance_m, 35.0);
    let at_100 = TreeCullSettings::reference(100.0);
    near(at_100.d1_m, 4.5);
    near(at_100.d2_m, 11.0);
    near(at_100.impostor_start_m, 18.16);
    near(at_100.mesh_far_m, 24.16);
}

#[test]
fn veg_thinning_keeps_everything_near_and_fifteen_percent_at_the_render_distance() {
    let settings = TreeCullSettings::reference(640.0);
    near(keep_probability(&settings, 50.0), 1.0);
    near(keep_probability(&settings, 96.0), 1.0);
    near(keep_probability(&settings, 368.0), 0.575);
    near(keep_probability(&settings, 640.0), 0.15);
    near(keep_probability(&settings, 900.0), 0.15);
}

#[test]
fn veg_frustum_planes_accept_inside_and_reject_each_side() {
    let view = CullView::new(Vec3::ZERO, wide_projection());

    let inside = |point: Vec3| sphere_in_frustum(&view.planes, point, 0.0);
    assert!(inside(Vec3::new(0.0, 0.0, -10.0)));
    assert!(inside(Vec3::new(9.0, -9.0, -10.0)));
    assert!(!inside(Vec3::new(11.0, 0.0, -10.0)));
    assert!(!inside(Vec3::new(-11.0, 0.0, -10.0)));
    assert!(!inside(Vec3::new(0.0, 11.0, -10.0)));
    assert!(!inside(Vec3::new(0.0, -11.0, -10.0)));
    assert!(!inside(Vec3::new(0.0, 0.0, 5.0)));
    assert!(sphere_in_frustum(&view.planes, Vec3::new(12.0, 0.0, -10.0), 3.0));
}

#[test]
fn veg_frustum_planes_follow_the_eye_away_from_the_render_origin() {
    for eye in [Vec3::new(0.0, 0.0, -180.0), Vec3::new(6.0e4, -3.0e3, 2.5e4)] {
        let view = CullView::new(eye, wide_projection() * Mat4::from_translation(-eye));
        let inside = |offset: Vec3| sphere_in_frustum(&view.planes, offset, 0.0);
        assert!(inside(Vec3::new(0.0, 0.0, -20.0)), "20 m ahead of {eye:?} is culled");
        assert!(inside(Vec3::new(9.0, -9.0, -10.0)), "inside corner of {eye:?} is culled");
        assert!(!inside(Vec3::new(11.0, 0.0, -10.0)), "right of {eye:?} is kept");
        assert!(!inside(Vec3::new(0.0, 0.0, 5.0)), "behind {eye:?} is kept");
    }
}

#[test]
fn veg_upload_budgets_are_the_design_numbers() {
    assert_eq!(VegKind::Tree.upload_budget_bytes(), 64 * 1024);
    assert_eq!(VegKind::Rock.upload_budget_bytes(), 64 * 1024);
    assert_eq!(VegKind::Grass.upload_budget_bytes(), 512 * 1024);
}

struct Probe {
    config: VegGpuConfig,
    settings: TreeCullSettings,
    chunks: Vec<super::buffers::ChunkRecord>,
    packed: u32,
    view: CullView,
}

fn probe() -> Probe {
    let mut table = ChunkTable::default();
    let chunk = table.load(DVec3::ZERO, Quat::IDENTITY).unwrap();
    let origin = FloatingOrigin::default();
    let view_from_world = Transform::IDENTITY.looking_to(Vec3::X, Vec3::Y).to_matrix().inverse();
    Probe {
        config: VegGpuConfig::default(),
        settings: TreeCullSettings::reference(640.0),
        chunks: (0..super::store::CHUNK_SLOTS).map(|slot| table.record(slot, &origin)).collect(),
        packed: chunk.packed(),
        view: CullView::new(Vec3::ZERO, wide_projection() * view_from_world),
    }
}

impl Probe {
    fn walk(&self, distances: &[f32]) -> Vec<(Option<u32>, bool)> {
        let mut states = vec![0u32];
        let mut seen = Vec::new();
        for distance in distances {
            let tree = TwinTree { record: TreeRecord { local_position: [*distance, 0.0, 0.0], scale: 1.0, tilt: [0.0, 0.0, 0.0, 1.0], trunk_id: 0, ..Default::default() }, chunk: self.packed };
            let outcome = cull_trees(&[tree], &self.chunks, &[[0.0, 0.0, 0.0, 1.0]; 45], &self.config, &self.settings, &self.view, &states);
            let tier = (0..3).find(|tier| !outcome.lists[self.config.tree_slot(0, *tier) as usize].is_empty());
            seen.push((tier, !outcome.lists[self.config.impostor_slot() as usize].is_empty()));
            states = outcome.states;
        }
        seen
    }
}

#[test]
fn veg_twin_applies_tier_hysteresis_in_both_directions() {
    let tiers: Vec<_> = probe().walk(&[10.0, 15.0, 16.0, 13.0, 12.0, 40.0, 36.0, 30.0]).into_iter().map(|(tier, _)| tier).collect();
    assert_eq!(tiers, [Some(0), Some(0), Some(1), Some(1), Some(0), Some(2), Some(2), Some(1)]);
}

#[test]
fn veg_twin_swaps_mesh_for_impostor_with_its_own_overlap_and_reentry() {
    let walked = probe().walk(&[75.0, 80.0, 78.0, 77.0, 60.0]);
    assert_eq!(walked, [(Some(2), true), (None, true), (None, true), (Some(2), true), (Some(2), false)]);
}
