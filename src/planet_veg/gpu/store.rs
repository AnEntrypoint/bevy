use super::buffers::{ChunkRecord, EncodedRecords, VegGpuConfig, VegKind};
use crate::planet_lod::origin::FloatingOrigin;
use bevy::math::{DVec3, Quat};
use bevy::prelude::*;
use bevy::render::RenderApp;
use bevy::render::extract_resource::ExtractResource;
use std::collections::{BTreeMap, HashMap};

pub const CHUNK_SLOTS: usize = 2048;
pub const CHUNK_RECORD_BYTES: usize = 32;
pub const CHUNK_TABLE_BYTES: usize = CHUNK_SLOTS * CHUNK_RECORD_BYTES;
const FREE_SLOT_GENERATION: u32 = u32::MAX;
const GENERATION_MASK: u32 = 0xFFFF;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ChunkRef {
    pub slot: u16,
    pub generation: u16,
}

impl ChunkRef {
    pub fn packed(self) -> u32 {
        u32::from(self.slot) | u32::from(self.generation) << 16
    }
}

#[derive(Clone, Copy)]
struct ChunkEntry {
    anchor: DVec3,
    rotation: Quat,
}

pub struct ChunkTable {
    entries: Vec<Option<ChunkEntry>>,
    generations: Vec<u16>,
    free: Vec<u16>,
}

fn next_generation(generation: u16) -> u16 {
    match (u32::from(generation) + 1) & GENERATION_MASK {
        0 => 1,
        next => next as u16,
    }
}

impl Default for ChunkTable {
    fn default() -> Self {
        Self { entries: vec![None; CHUNK_SLOTS], generations: vec![1; CHUNK_SLOTS], free: (0..CHUNK_SLOTS as u16).rev().collect() }
    }
}

impl ChunkTable {
    pub fn load(&mut self, anchor: DVec3, rotation: Quat) -> Option<ChunkRef> {
        let slot = self.free.pop()?;
        self.entries[usize::from(slot)] = Some(ChunkEntry { anchor, rotation });
        Some(ChunkRef { slot, generation: self.generations[usize::from(slot)] })
    }

    pub fn is_live(&self, chunk: ChunkRef) -> bool {
        self.entries[usize::from(chunk.slot)].is_some() && self.generations[usize::from(chunk.slot)] == chunk.generation
    }

    pub fn unload(&mut self, chunk: ChunkRef) -> bool {
        if !self.is_live(chunk) {
            return false;
        }
        let slot = usize::from(chunk.slot);
        self.entries[slot] = None;
        self.generations[slot] = next_generation(self.generations[slot]);
        self.free.insert(0, chunk.slot);
        true
    }

    pub fn generation(&self, slot: u16) -> u16 {
        self.generations[usize::from(slot)]
    }

    pub fn record(&self, slot: usize, origin: &FloatingOrigin) -> ChunkRecord {
        match self.entries[slot] {
            Some(entry) => ChunkRecord { origin: origin.to_local(entry.anchor), generation: u32::from(self.generations[slot]), rotation: entry.rotation },
            None => ChunkRecord { origin: Vec3::ZERO, generation: FREE_SLOT_GENERATION, rotation: Quat::IDENTITY },
        }
    }

    pub fn encode(&self, origin: &FloatingOrigin) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(CHUNK_TABLE_BYTES);
        (0..CHUNK_SLOTS).for_each(|slot| self.record(slot, origin).write(&mut bytes));
        bytes
    }
}

#[derive(Debug)]
pub struct RangeAllocator {
    capacity: u32,
    free: BTreeMap<u32, u32>,
}

impl RangeAllocator {
    pub fn new(capacity: u32) -> Self {
        Self { capacity, free: BTreeMap::from([(0, capacity)]) }
    }

    pub fn alloc(&mut self, len: u32) -> Option<u32> {
        let (&start, &available) = self.free.iter().find(|(_, available)| **available >= len)?;
        self.free.remove(&start);
        if available > len {
            self.free.insert(start + len, available - len);
        }
        Some(start)
    }

    pub fn release(&mut self, start: u32, len: u32) {
        let mut start = start;
        let mut len = len;
        if let Some((&before, &before_len)) = self.free.range(..start).next_back()
            && before + before_len == start
        {
            self.free.remove(&before);
            start = before;
            len += before_len;
        }
        if let Some(after_len) = self.free.remove(&(start + len)) {
            len += after_len;
        }
        self.free.insert(start, len);
    }

    pub fn used_end(&self) -> u32 {
        match self.free.iter().next_back() {
            Some((&start, &len)) if start + len == self.capacity => start,
            _ => self.capacity,
        }
    }

    pub fn free_total(&self) -> u32 {
        self.free.values().sum()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UploadWrite {
    pub byte_offset: u64,
    pub bytes: Vec<u8>,
}

struct PendingUpload {
    chunk: ChunkRef,
    first_record: u32,
    bytes: Vec<u8>,
    sent: usize,
}

pub struct UploadQueue {
    kind: VegKind,
    budget_bytes: usize,
    pending: BTreeMap<(u32, u64), PendingUpload>,
    next_sequence: u64,
}

impl UploadQueue {
    pub fn new(kind: VegKind, budget_bytes: usize) -> Self {
        Self { kind, budget_bytes, pending: BTreeMap::new(), next_sequence: 0 }
    }

    pub fn push(&mut self, chunk: ChunkRef, distance_m: f32, first_record: u32, bytes: Vec<u8>) {
        let key = (distance_m.max(0.0).to_bits(), self.next_sequence);
        self.next_sequence += 1;
        self.pending.insert(key, PendingUpload { chunk, first_record, bytes, sent: 0 });
    }

    pub fn drop_chunk(&mut self, chunk: ChunkRef) {
        self.pending.retain(|_, upload| upload.chunk != chunk);
    }

    pub fn pending_bytes(&self) -> usize {
        self.pending.values().map(|upload| upload.bytes.len() - upload.sent).sum()
    }

    pub fn drain(&mut self) -> Vec<UploadWrite> {
        let record_bytes = self.kind.record_bytes();
        let chunk_word = self.kind.chunk_word_offset();
        let mut remaining = self.budget_bytes / record_bytes * record_bytes;
        let mut writes = Vec::new();
        while remaining > 0 {
            let Some(mut entry) = self.pending.first_entry() else { break };
            let upload = entry.get_mut();
            let take = remaining.min(upload.bytes.len() - upload.sent);
            let mut bytes = upload.bytes[upload.sent..upload.sent + take].to_vec();
            let packed = upload.chunk.packed().to_le_bytes();
            bytes.chunks_exact_mut(record_bytes).for_each(|record| record[chunk_word..chunk_word + 4].copy_from_slice(&packed));
            writes.push(UploadWrite { byte_offset: (upload.first_record as usize * record_bytes + upload.sent) as u64, bytes });
            upload.sent += take;
            remaining -= take;
            if upload.sent == upload.bytes.len() {
                entry.remove();
            }
        }
        writes
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum UploadError {
    StaleChunk,
    AlreadyUploaded,
    NoRoom,
}

struct KindStore {
    allocator: RangeAllocator,
    queue: UploadQueue,
    ranges: HashMap<u16, (u32, u32)>,
}

#[derive(Resource)]
pub struct VegStore {
    table: ChunkTable,
    kinds: [KindStore; 3],
}

#[derive(Resource, ExtractResource, Clone, Default, Debug)]
#[extract_app(RenderApp)]
pub struct VegFrame {
    pub chunk_table: Vec<u8>,
    pub writes: [Vec<UploadWrite>; 3],
    pub instance_counts: [u32; 3],
}

impl VegStore {
    pub fn new(config: &VegGpuConfig) -> Self {
        let kind_store = |kind: VegKind| KindStore {
            allocator: RangeAllocator::new(config.instance_capacity[kind.index()]),
            queue: UploadQueue::new(kind, kind.upload_budget_bytes()),
            ranges: HashMap::new(),
        };
        Self { table: ChunkTable::default(), kinds: VegKind::ALL.map(kind_store) }
    }

    pub fn add_chunk(&mut self, anchor: DVec3, rotation: Quat) -> Option<ChunkRef> {
        self.table.load(anchor, rotation)
    }

    pub fn is_live(&self, chunk: ChunkRef) -> bool {
        self.table.is_live(chunk)
    }

    pub fn upload(&mut self, chunk: ChunkRef, distance_m: f32, records: EncodedRecords) -> Result<(), UploadError> {
        if !self.table.is_live(chunk) {
            return Err(UploadError::StaleChunk);
        }
        let store = &mut self.kinds[records.kind.index()];
        if store.ranges.contains_key(&chunk.slot) {
            return Err(UploadError::AlreadyUploaded);
        }
        let count = records.len();
        let first = store.allocator.alloc(count).ok_or(UploadError::NoRoom)?;
        store.ranges.insert(chunk.slot, (first, count));
        store.queue.push(chunk, distance_m, first, records.bytes);
        Ok(())
    }

    pub fn remove_chunk(&mut self, chunk: ChunkRef) -> bool {
        if !self.table.unload(chunk) {
            return false;
        }
        for store in &mut self.kinds {
            if let Some((start, len)) = store.ranges.remove(&chunk.slot) {
                store.allocator.release(start, len);
            }
            store.queue.drop_chunk(chunk);
        }
        true
    }

    pub fn resident_records(&self, kind: VegKind) -> u32 {
        self.kinds[kind.index()].ranges.values().map(|(_, len)| len).sum()
    }

    pub fn pending_bytes(&self, kind: VegKind) -> usize {
        self.kinds[kind.index()].queue.pending_bytes()
    }

    pub fn chunk_range(&self, chunk: ChunkRef, kind: VegKind) -> Option<(u32, u32)> {
        self.kinds[kind.index()].ranges.get(&chunk.slot).copied()
    }

    pub fn table(&self) -> &ChunkTable {
        &self.table
    }

    pub fn frame(&mut self, origin: &FloatingOrigin) -> VegFrame {
        VegFrame {
            chunk_table: self.table.encode(origin),
            writes: VegKind::ALL.map(|kind| self.kinds[kind.index()].queue.drain()),
            instance_counts: VegKind::ALL.map(|kind| self.kinds[kind.index()].allocator.used_end()),
        }
    }
}

pub fn publish_frame(mut store: ResMut<VegStore>, origin: Res<FloatingOrigin>, mut frame: ResMut<VegFrame>) {
    *frame = store.frame(&origin);
}
