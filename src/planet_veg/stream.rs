use super::context::PlacementContext;
use super::gpu::buffers::{EncodedRecords, GrassRecord, RockRecord, TreeRecord, VegKind, tint_rgb8};
use super::gpu::draw::VegCamera;
use super::gpu::store::{ChunkRef, VegStore};
use super::grass::{GrassBlade, grass_rank, place_grass_chunk};
use super::grass_render::GrassLook;
use super::rock::{RockInstance, place_rock_chunk};
use super::tree::{self, TreeInstance, place_tree_chunk};
use crate::height::placement::PlacementLattice;
use crate::height::planet_frame::anchor_basis;
use crate::height::{HashVersion, Planet};
use crate::planet_lod::origin::FloatingOrigin;
use bevy::math::{DMat3, DVec2, DVec3};
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures::check_ready};
use std::collections::HashMap;
use std::sync::Arc;

const CHUNK_M: f64 = 32.0;
const RING_MARGIN_M: f64 = 40.0;
const DROP_MARGIN_M: f64 = 64.0;
const ROCK_RENDER_DISTANCE_M: f64 = 320.0;
const GRASS_RENDER_DISTANCE_M: f64 = 88.0;
const GRASS_DROP_MARGIN_M: f64 = 16.0;
const TASKS_PER_THREAD: usize = 2;
const RING_WORK_PER_FRAME: usize = 64;
const NEAREST_KIND_FIRST: [VegKind; 3] = [VegKind::Grass, VegKind::Rock, VegKind::Tree];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ring {
    pub load_m: f64,
    pub drop_m: f64,
}

const fn reference_ring(render_distance_m: f64) -> Ring {
    let load_m = render_distance_m + RING_MARGIN_M;
    Ring { load_m, drop_m: load_m + DROP_MARGIN_M }
}

pub const RINGS: [Ring; 3] = [
    reference_ring(tree::RENDER_DISTANCE_M),
    reference_ring(ROCK_RENDER_DISTANCE_M),
    Ring { load_m: GRASS_RENDER_DISTANCE_M, drop_m: GRASS_RENDER_DISTANCE_M + GRASS_DROP_MARGIN_M },
];

#[derive(Clone)]
pub struct VegSource {
    pub planet: Arc<Planet>,
    pub seed: u32,
    pub radius: f64,
    pub hash: HashVersion,
}

pub struct PlacedChunk {
    anchor: DVec3,
    rotation: Quat,
    records: EncodedRecords,
}

fn tree_record(tree: &TreeInstance) -> TreeRecord {
    TreeRecord {
        local_position: tree.pos,
        scale: tree.scale,
        tilt: tree.tilt_quat,
        yaw: tree.yaw,
        wind_phase: tree.wind_phase,
        tint_rgb8: tint_rgb8(tree.tint),
        trunk_id: tree.trunk_id,
        species: tree.species,
        shape: tree.shape,
    }
}

fn rock_record(rock: &RockInstance) -> RockRecord {
    let rotation = Quat::from_array(rock.tilt_quat) * Quat::from_rotation_y(rock.yaw);
    RockRecord { local_position: rock.pos, scale: rock.scale, rotation: rotation.to_array(), squash: rock.squash, variant: rock.variant, kind: u32::from(rock.rock_type) }
}

fn grass_record(blade: &GrassBlade) -> GrassRecord {
    let rotation = Quat::from_rotation_arc(Vec3::Y, Vec3::from(blade.normal)) * Quat::from_rotation_y(blade.yaw);
    GrassRecord {
        local_position: blade.pos,
        scale: blade.scale,
        rotation: rotation.to_array(),
        ground_rgb: blade.ground,
        tint: blade.tint,
        rank: grass_rank(blade.q) as f32,
        wind_phase: blade.wind_phase,
        shadow: blade.shadow,
    }
}

fn encode<I, R: super::gpu::buffers::GpuRecord>(instances: Vec<I>, record: fn(&I) -> R) -> EncodedRecords {
    EncodedRecords::new(&instances.iter().map(record).collect::<Vec<_>>())
}

pub fn place_chunk(source: &VegSource, kind: VegKind, key: u64) -> PlacedChunk {
    let ctx = PlacementContext::new(&source.planet, source.seed, source.radius, source.hash);
    let records = match kind {
        VegKind::Tree => encode(place_tree_chunk(&ctx, key), tree_record),
        VegKind::Rock => encode(place_rock_chunk(&ctx, key), rock_record),
        VegKind::Grass => encode(place_grass_chunk(&ctx, key), grass_record),
    };
    let frame = ctx.chunk_frame(&ctx.lattice(CHUNK_M, tree::GRID), key);
    let axes = [frame.basis.east, frame.basis.up, frame.basis.north].map(DVec3::from);
    let rotation = Quat::from_mat3(&DMat3::from_cols(axes[0], axes[1], axes[2]).as_mat3());
    PlacedChunk { anchor: axes[1] * frame.surface_base(), rotation, records }
}

fn chord_sq_within(radius_m: f64, planet_radius: f64) -> f64 {
    4.0 * (radius_m / planet_radius / 2.0).sin().powi(2)
}

fn has_room(work: usize) -> bool {
    work < RING_WORK_PER_FRAME
}

struct RingPlan {
    ring: Vec<u64>,
    dropped: Vec<u64>,
}

fn plan_rings(lattice: &PlacementLattice, dir: [f64; 3], streamed: [Vec<u64>; 3]) -> [RingPlan; 3] {
    let mut streamed = streamed.into_iter();
    RINGS.map(|ring| {
        let drop_chord_sq = chord_sq_within(ring.drop_m, lattice.radius);
        let beyond = |key: &u64| {
            let centre = lattice.chunk_centre_dir(*key as f64);
            (0..3).map(|axis| (centre[axis] - dir[axis]).powi(2)).sum::<f64>() > drop_chord_sq
        };
        let dropped = streamed.next().into_iter().flatten().filter(beyond).collect();
        RingPlan { ring: lattice.ring_around_dir(dir, ring.load_m).into_iter().map(|key| key as u64).collect(), dropped }
    })
}

#[derive(Default)]
struct KindStream {
    ring: Vec<u64>,
    cursor: usize,
    loaded: HashMap<u64, Option<ChunkRef>>,
    building: HashMap<u64, Task<PlacedChunk>>,
    dropping: Vec<u64>,
    refused: usize,
}

impl KindStream {
    fn is_streamed(&self, key: u64) -> bool {
        self.loaded.contains_key(&key) || self.building.contains_key(&key)
    }

    fn streamed_keys(&self) -> Vec<u64> {
        self.loaded.keys().chain(self.building.keys()).copied().collect()
    }

    fn replan(&mut self, plan: RingPlan) {
        self.ring = plan.ring;
        self.cursor = 0;
        self.dropping = plan.dropped;
    }

    fn drop_some(&mut self, work: &mut usize, store: &mut VegStore) {
        while has_room(*work) {
            let Some(key) = self.dropping.pop() else { return };
            *work += 1;
            self.building.remove(&key);
            if let Some(Some(chunk)) = self.loaded.remove(&key) {
                store.remove_chunk(chunk);
            }
        }
    }

    fn finish(&mut self, store: &mut VegStore, centre: DVec3, eye: DVec3) {
        let (loaded, refused) = (&mut self.loaded, &mut self.refused);
        self.building.retain(|key, task| {
            let Some(placed) = check_ready(task) else { return true };
            let chunk = (placed.records.len() > 0).then(|| {
                let anchor = centre + placed.anchor;
                let chunk = store.add_chunk(anchor, placed.rotation)?;
                match store.upload(chunk, (anchor - centre - eye).length() as f32, placed.records) {
                    Ok(()) => Some(chunk),
                    Err(_) => {
                        store.remove_chunk(chunk);
                        None
                    }
                }
            });
            *refused += usize::from(matches!(chunk, Some(None)));
            loaded.insert(*key, chunk.flatten());
            false
        });
    }
}

#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct VegStreamStats {
    pub resident_chunks: [usize; 3],
    pub queued_chunks: [usize; 3],
    pub refused_chunks: usize,
    pub tasks_in_flight: usize,
    pub ring_work: usize,
}

struct WindChart {
    point: DVec3,
    east: DVec3,
    north: DVec3,
}

impl WindChart {
    fn look(&self, eye_world: DVec3, look: GrassLook) -> GrassLook {
        let offset = eye_world - self.point;
        GrassLook { wind_origin: DVec2::new(offset.dot(self.east), offset.dot(self.north)).as_vec2(), wind_axis_x: self.east.as_vec3(), wind_axis_z: self.north.as_vec3(), ..look }
    }
}

#[derive(Resource)]
pub struct VegStream {
    source: VegSource,
    centre: DVec3,
    lattice: PlacementLattice,
    wind: WindChart,
    focus: Option<f64>,
    planning: Option<Task<[RingPlan; 3]>>,
    kinds: [KindStream; 3],
}

impl VegStream {
    fn new(plugin: &VegStreamPlugin) -> Self {
        let source = plugin.source.clone();
        let basis = anchor_basis(plugin.anchor_dir);
        let wind = WindChart { point: plugin.centre + DVec3::from(basis.up) * source.radius, east: DVec3::from(basis.east), north: DVec3::from(basis.north) };
        let lattice = PlacementLattice::new(source.radius, CHUNK_M, tree::GRID).expect("a placement lattice over a finite planet radius always builds");
        Self { source, centre: plugin.centre, lattice, wind, focus: None, planning: None, kinds: Default::default() }
    }

    fn advance(&mut self, eye: DVec3, store: &mut VegStore) -> usize {
        let dir = eye.normalize().to_array();
        let focus = self.lattice.chunk_key_of_dir(dir);
        if self.planning.is_none() && self.focus != Some(focus) {
            self.focus = Some(focus);
            let (lattice, streamed) = (self.lattice.clone(), self.kinds.each_ref().map(KindStream::streamed_keys));
            self.planning = Some(AsyncComputeTaskPool::get().spawn(async move { plan_rings(&lattice, dir, streamed) }));
        }
        if let Some(plans) = self.planning.as_mut().and_then(check_ready) {
            self.planning = None;
            self.kinds.iter_mut().zip(plans).for_each(|(kind, plan)| kind.replan(plan));
        }
        let centre = self.centre;
        for kind in &mut self.kinds {
            kind.finish(store, centre, eye);
        }
        let mut work = 0;
        self.drop_due(&mut work, store);
        if self.planning.is_none() {
            self.spawn(&mut work);
        }
        work
    }

    fn drop_due(&mut self, work: &mut usize, store: &mut VegStore) {
        for kind in &mut self.kinds {
            kind.drop_some(work, store);
        }
    }

    fn spawn(&mut self, work: &mut usize) {
        let pool = AsyncComputeTaskPool::get();
        let mut room = (pool.thread_num() * TASKS_PER_THREAD).saturating_sub(self.kinds.iter().map(|kind| kind.building.len()).sum());
        for kind in NEAREST_KIND_FIRST {
            let stream = &mut self.kinds[kind.index()];
            while room > 0 && has_room(*work) && stream.cursor < stream.ring.len() {
                let key = stream.ring[stream.cursor];
                stream.cursor += 1;
                *work += 1;
                if stream.is_streamed(key) {
                    continue;
                }
                let source = self.source.clone();
                stream.building.insert(key, pool.spawn(async move { place_chunk(&source, kind, key) }));
                room -= 1;
            }
        }
    }

    fn stats(&self, ring_work: usize) -> VegStreamStats {
        VegStreamStats {
            resident_chunks: self.kinds.each_ref().map(|kind| kind.loaded.values().filter(|chunk| chunk.is_some()).count()),
            queued_chunks: self.kinds.each_ref().map(|kind| kind.ring.len() - kind.cursor),
            refused_chunks: self.kinds.iter().map(|kind| kind.refused).sum(),
            tasks_in_flight: self.kinds.iter().map(|kind| kind.building.len()).sum::<usize>() + usize::from(self.planning.is_some()),
            ring_work,
        }
    }
}

fn stream_vegetation(
    mut stream: ResMut<VegStream>,
    mut store: ResMut<VegStore>,
    mut stats: ResMut<VegStreamStats>,
    mut look: Option<ResMut<GrassLook>>,
    origin: Res<FloatingOrigin>,
    camera: Query<&Transform, With<VegCamera>>,
) {
    crate::frame_clock::timed(8, || {
    let Ok(camera) = camera.single() else { return };
    let eye_world = origin.to_world(camera.translation);
    if let Some(look) = look.as_mut() {
        let anchored = stream.wind.look(eye_world, **look);
        look.set_if_neq(anchored);
    }
    let eye = eye_world - stream.centre;
    let ring_work = stream.advance(eye, &mut store);
    stats.set_if_neq(stream.stats(ring_work));
});
}

pub struct VegStreamPlugin {
    pub source: VegSource,
    pub centre: DVec3,
    pub anchor_dir: [f64; 3],
}

impl Plugin for VegStreamPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(VegStream::new(self)).init_resource::<FloatingOrigin>().init_resource::<VegStreamStats>().add_systems(Update, stream_vegetation);
    }
}

#[cfg(test)]
mod tests;
