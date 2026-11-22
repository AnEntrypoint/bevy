use bevy::prelude::*;
use bevy::render::RenderApp;
use bevy::render::extract_resource::ExtractResource;
use bevy::render::render_resource::{Buffer, BufferDescriptor, BufferInitDescriptor, BufferUsages};
use bevy::render::renderer::{RenderDevice, RenderQueue};

use super::store::VegFrame;

pub const TREE_SLOTS_PER_GEO: u32 = 4;
#[cfg(test)]
pub const SHADOW_TIER: u32 = 3;
pub const GRASS_SLOTS: u32 = 2;
pub const DRAW_ARGS_BYTES: u64 = 20;
pub const SLOT_UNIFORM_STRIDE: u64 = 256;
pub const SLOT_UNIFORM_BYTES: u64 = 16;
const MIN_BUFFER_BYTES: u64 = 16;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum VegKind {
    Tree,
    Rock,
    Grass,
}

impl VegKind {
    pub const ALL: [VegKind; 3] = [VegKind::Tree, VegKind::Rock, VegKind::Grass];

    pub const fn index(self) -> usize {
        self as usize
    }

    pub const fn record_bytes(self) -> usize {
        match self {
            VegKind::Tree | VegKind::Grass => 64,
            VegKind::Rock => 48,
        }
    }

    pub const fn chunk_word_offset(self) -> usize {
        self.record_bytes() - 4 - if matches!(self, VegKind::Tree) { 8 } else { 0 }
    }

    pub const fn upload_budget_bytes(self) -> usize {
        match self {
            VegKind::Tree | VegKind::Rock => 64 * 1024,
            VegKind::Grass => 512 * 1024,
        }
    }
}

#[derive(Resource, ExtractResource, Clone, Debug, PartialEq)]
#[extract_app(RenderApp)]
pub struct VegGpuConfig {
    pub tree_species: u32,
    pub tree_shapes: u32,
    pub tier_capacity: [u32; 4],
    pub impostor_capacity: u32,
    pub rock_kinds: u32,
    pub rock_slot_capacity: u32,
    pub grass_slot_capacity: u32,
    pub instance_capacity: [u32; 3],
}

impl Default for VegGpuConfig {
    fn default() -> Self {
        Self {
            tree_species: 15,
            tree_shapes: 3,
            tier_capacity: [256, 1024, 2048, 512],
            impostor_capacity: 32768,
            rock_kinds: 6,
            rock_slot_capacity: 12288,
            grass_slot_capacity: 540_000,
            instance_capacity: [32768, 12288, 540_000],
        }
    }
}

impl VegGpuConfig {
    pub fn tree_geo_count(&self) -> u32 {
        self.tree_species * self.tree_shapes
    }

    pub fn tree_slot(&self, geo: u32, tier: u32) -> u32 {
        geo * TREE_SLOTS_PER_GEO + tier
    }

    pub fn impostor_slot(&self) -> u32 {
        self.tree_geo_count() * TREE_SLOTS_PER_GEO
    }

    pub fn slot_count(&self, kind: VegKind) -> u32 {
        match kind {
            VegKind::Tree => self.impostor_slot() + 1,
            VegKind::Rock => self.rock_kinds,
            VegKind::Grass => GRASS_SLOTS,
        }
    }

    pub fn slot_capacity(&self, kind: VegKind, slot: u32) -> u32 {
        match kind {
            VegKind::Tree if slot == self.impostor_slot() => self.impostor_capacity,
            VegKind::Tree => self.tier_capacity[(slot % TREE_SLOTS_PER_GEO) as usize],
            VegKind::Rock => self.rock_slot_capacity,
            VegKind::Grass => self.grass_slot_capacity,
        }
    }

    pub fn list_offset(&self, kind: VegKind, slot: u32) -> u32 {
        match kind {
            VegKind::Tree => {
                let per_geo: u32 = self.tier_capacity.iter().sum();
                let tier = (slot % TREE_SLOTS_PER_GEO) as usize;
                if slot == self.impostor_slot() {
                    self.tree_geo_count() * per_geo
                } else {
                    (slot / TREE_SLOTS_PER_GEO) * per_geo + self.tier_capacity[..tier].iter().sum::<u32>()
                }
            }
            VegKind::Rock => slot * self.rock_slot_capacity,
            VegKind::Grass => slot * self.grass_slot_capacity,
        }
    }

    pub fn visible_len(&self, kind: VegKind) -> u32 {
        match kind {
            VegKind::Tree => self.list_offset(kind, self.impostor_slot()) + self.impostor_capacity,
            VegKind::Rock => self.rock_kinds * self.rock_slot_capacity,
            VegKind::Grass => GRASS_SLOTS * self.grass_slot_capacity,
        }
    }

    pub fn tree_slot_layout(&self) -> Vec<[u32; 2]> {
        (0..self.slot_count(VegKind::Tree)).map(|slot| [self.list_offset(VegKind::Tree, slot), self.slot_capacity(VegKind::Tree, slot)]).collect()
    }

    pub fn slot_uniform_index(&self, kind: VegKind, slot: u32) -> u32 {
        let before = match kind {
            VegKind::Tree => 0,
            VegKind::Rock => self.slot_count(VegKind::Tree),
            VegKind::Grass => self.slot_count(VegKind::Tree) + self.slot_count(VegKind::Rock),
        };
        before + slot
    }

    pub fn slot_uniform_offset(&self, kind: VegKind, slot: u32) -> u32 {
        self.slot_uniform_index(kind, slot) * SLOT_UNIFORM_STRIDE as u32
    }

    fn total_slots(&self) -> u32 {
        VegKind::ALL.iter().map(|kind| self.slot_count(*kind)).sum()
    }
}

#[derive(Resource, ExtractResource, Clone, Debug, Default, PartialEq)]
#[extract_app(RenderApp)]
pub struct VegBounds {
    pub tree_geometry: Vec<[f32; 4]>,
    pub rock_kind: Vec<[f32; 4]>,
    pub impostor_dims: Vec<[f32; 4]>,
}

pub struct Packer<'a>(pub &'a mut Vec<u8>);

impl Packer<'_> {
    pub fn f32s(&mut self, values: &[f32]) -> &mut Self {
        values.iter().for_each(|value| self.0.extend_from_slice(&value.to_le_bytes()));
        self
    }

    pub fn u32s(&mut self, values: &[u32]) -> &mut Self {
        values.iter().for_each(|value| self.0.extend_from_slice(&value.to_le_bytes()));
        self
    }
}

pub trait GpuRecord {
    const KIND: VegKind;
    fn write(&self, out: &mut Vec<u8>);
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TreeRecord {
    pub local_position: [f32; 3],
    pub scale: f32,
    pub tilt: [f32; 4],
    pub yaw: f32,
    pub wind_phase: f32,
    pub tint_rgb8: u32,
    pub trunk_id: u32,
    pub species: u8,
    pub shape: u8,
}

pub fn tint_rgb8(tint: [f32; 3]) -> u32 {
    let unorm = |value: f32| ((value * 0.5).clamp(0.0, 1.0) * 255.0).round() as u32;
    unorm(tint[0]) | unorm(tint[1]) << 8 | unorm(tint[2]) << 16
}

impl GpuRecord for TreeRecord {
    const KIND: VegKind = VegKind::Tree;

    fn write(&self, out: &mut Vec<u8>) {
        Packer(out)
            .f32s(&self.local_position)
            .f32s(&[self.scale])
            .f32s(&self.tilt)
            .f32s(&[self.yaw, self.wind_phase])
            .u32s(&[self.tint_rgb8, self.trunk_id, u32::from(self.species) | u32::from(self.shape) << 8, 0, 0, 0]);
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RockRecord {
    pub local_position: [f32; 3],
    pub scale: f32,
    pub rotation: [f32; 4],
    pub squash: f32,
    pub variant: f32,
    pub kind: u32,
}

impl GpuRecord for RockRecord {
    const KIND: VegKind = VegKind::Rock;

    fn write(&self, out: &mut Vec<u8>) {
        Packer(out).f32s(&self.local_position).f32s(&[self.scale]).f32s(&self.rotation).f32s(&[self.squash, self.variant]).u32s(&[self.kind, 0]);
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GrassRecord {
    pub local_position: [f32; 3],
    pub scale: f32,
    pub rotation: [f32; 4],
    pub ground_rgb: [f32; 3],
    pub tint: f32,
    pub rank: f32,
    pub wind_phase: f32,
    pub shadow: f32,
}

impl GpuRecord for GrassRecord {
    const KIND: VegKind = VegKind::Grass;

    fn write(&self, out: &mut Vec<u8>) {
        Packer(out)
            .f32s(&self.local_position)
            .f32s(&[self.scale])
            .f32s(&self.rotation)
            .f32s(&self.ground_rgb)
            .f32s(&[self.tint, self.rank, self.wind_phase, self.shadow])
            .u32s(&[0]);
    }
}

#[derive(Clone, Debug)]
pub struct EncodedRecords {
    pub kind: VegKind,
    pub bytes: Vec<u8>,
}

impl EncodedRecords {
    pub fn new<R: GpuRecord>(records: &[R]) -> Self {
        let mut bytes = Vec::with_capacity(records.len() * R::KIND.record_bytes());
        records.iter().for_each(|record| record.write(&mut bytes));
        Self { kind: R::KIND, bytes }
    }

    pub fn len(&self) -> u32 {
        (self.bytes.len() / self.kind.record_bytes()) as u32
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChunkRecord {
    pub origin: Vec3,
    pub generation: u32,
    pub rotation: Quat,
}

impl ChunkRecord {
    pub fn write(&self, out: &mut Vec<u8>) {
        Packer(out).f32s(&self.origin.to_array()).u32s(&[self.generation]).f32s(&self.rotation.to_array());
    }
}

#[derive(Resource)]
pub struct VegGpu {
    pub instances: [Buffer; 3],
    pub chunks: Buffer,
    pub tree_state: Buffer,
    pub rock_state: Buffer,
    pub tree_bounds: Buffer,
    pub rock_bounds: Buffer,
    pub impostor_dims: Buffer,
    pub tree_slot_layout: Buffer,
    pub draws: [Buffer; 3],
    pub visible: [Buffer; 3],
    pub stats: Buffer,
    pub cull_params: [Buffer; 3],
    pub view: Buffer,
    pub light: Buffer,
    pub slot_uniforms: Buffer,
    pub instance_counts: [u32; 3],
    pub view_ready: bool,
}

fn storage(device: &RenderDevice, label: &str, bytes: u64, extra: BufferUsages) -> Buffer {
    device.create_buffer(&BufferDescriptor {
        label: Some(label),
        size: bytes.max(MIN_BUFFER_BYTES),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC | extra,
        mapped_at_creation: false,
    })
}

fn uniform(device: &RenderDevice, label: &str, bytes: u64) -> Buffer {
    device.create_buffer(&BufferDescriptor {
        label: Some(label),
        size: bytes,
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    })
}

pub const VIEW_UNIFORM_BYTES: u64 = 160;
pub const LIGHT_UNIFORM_BYTES: u64 = 80;
pub const TREE_PARAMS_BYTES: u64 = 176;
pub const ROCK_PARAMS_BYTES: u64 = 128;
pub const GRASS_PARAMS_BYTES: u64 = 144;
pub const STATS_BYTES: u64 = 48;

impl VegGpu {
    pub fn new(device: &RenderDevice, config: &VegGpuConfig) -> Self {
        let instance = |kind: VegKind| storage(device, "veg_instances", u64::from(config.instance_capacity[kind.index()]) * kind.record_bytes() as u64, BufferUsages::empty());
        let draws = |kind: VegKind| storage(device, "veg_draw_args", u64::from(config.slot_count(kind)) * DRAW_ARGS_BYTES, BufferUsages::INDIRECT);
        let visible = |kind: VegKind| storage(device, "veg_visible", u64::from(config.visible_len(kind)) * 4, BufferUsages::empty());
        let slot_uniforms = {
            let mut bytes = vec![0u8; (u64::from(config.total_slots()) * SLOT_UNIFORM_STRIDE) as usize];
            for kind in VegKind::ALL {
                for slot in 0..config.slot_count(kind) {
                    let at = config.slot_uniform_offset(kind, slot) as usize;
                    bytes[at..at + 4].copy_from_slice(&config.list_offset(kind, slot).to_le_bytes());
                }
            }
            device.create_buffer_with_data(&BufferInitDescriptor { label: Some("veg_slot_uniforms"), contents: &bytes, usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST })
        };
        let tree_slot_layout = {
            let mut bytes = Vec::new();
            config.tree_slot_layout().iter().for_each(|entry| {
                Packer(&mut bytes).u32s(entry);
            });
            device.create_buffer_with_data(&BufferInitDescriptor { label: Some("veg_tree_slot_layout"), contents: &bytes, usage: BufferUsages::STORAGE | BufferUsages::COPY_DST })
        };
        Self {
            instances: VegKind::ALL.map(instance),
            chunks: storage(device, "veg_chunks", super::store::CHUNK_TABLE_BYTES as u64, BufferUsages::empty()),
            tree_state: storage(device, "veg_tree_state", u64::from(config.instance_capacity[0]) * 4, BufferUsages::empty()),
            rock_state: storage(device, "veg_rock_state", u64::from(config.instance_capacity[1]) * 4, BufferUsages::empty()),
            tree_bounds: storage(device, "veg_tree_bounds", u64::from(config.tree_geo_count()) * 16, BufferUsages::empty()),
            rock_bounds: storage(device, "veg_rock_bounds", u64::from(config.rock_kinds) * 16, BufferUsages::empty()),
            impostor_dims: storage(device, "veg_impostor_dims", u64::from(config.tree_species) * 16, BufferUsages::empty()),
            tree_slot_layout,
            draws: VegKind::ALL.map(draws),
            visible: VegKind::ALL.map(visible),
            stats: storage(device, "veg_cull_stats", STATS_BYTES, BufferUsages::empty()),
            cull_params: [uniform(device, "veg_tree_cull_params", TREE_PARAMS_BYTES), uniform(device, "veg_rock_cull_params", ROCK_PARAMS_BYTES), uniform(device, "veg_grass_cull_params", GRASS_PARAMS_BYTES)],
            view: uniform(device, "veg_view", VIEW_UNIFORM_BYTES),
            light: uniform(device, "veg_light", LIGHT_UNIFORM_BYTES),
            slot_uniforms,
            instance_counts: [0; 3],
            view_ready: false,
        }
    }
}

pub fn apply_frame(frame: Option<Res<VegFrame>>, gpu: Option<ResMut<VegGpu>>, queue: Res<RenderQueue>) {
    let (Some(frame), Some(mut gpu)) = (frame, gpu) else { return };
    if !frame.is_changed() {
        return;
    }
    if !frame.chunk_table.is_empty() {
        queue.write_buffer(&gpu.chunks, 0, &frame.chunk_table);
    }
    for kind in VegKind::ALL {
        for write in &frame.writes[kind.index()] {
            queue.write_buffer(&gpu.instances[kind.index()], write.byte_offset, &write.bytes);
        }
    }
    gpu.instance_counts = frame.instance_counts;
}

pub fn apply_bounds(bounds: Option<Res<VegBounds>>, gpu: Option<Res<VegGpu>>, queue: Res<RenderQueue>) {
    let (Some(bounds), Some(gpu)) = (bounds, gpu) else { return };
    if !bounds.is_changed() {
        return;
    }
    let pack = |values: &[[f32; 4]]| {
        let mut bytes = Vec::new();
        values.iter().for_each(|value| {
            Packer(&mut bytes).f32s(value);
        });
        bytes
    };
    for (buffer, values) in [(&gpu.tree_bounds, &bounds.tree_geometry), (&gpu.rock_bounds, &bounds.rock_kind), (&gpu.impostor_dims, &bounds.impostor_dims)] {
        let bytes = pack(values);
        queue.write_buffer(buffer, 0, &bytes[..bytes.len().min(buffer.size() as usize)]);
    }
}
