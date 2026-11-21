use std::collections::HashMap;
const TILE_SIZE: i64 = 8;
const TILE_CELLS: i64 = 64;
const TEXELS_PER_TILE_ROW: usize = 8;
const BYTES_PER_TEXEL: usize = 4;
const TILE_BYTES: usize = TILE_CELLS as usize * BYTES_PER_TEXEL;

pub struct LatticeCell {
    pub face: i64,
    pub i: i64,
    pub j: i64,
}

pub trait FireLattice {
    fn cells_per_face(&self) -> i64;
    fn cell_of_placement_id(&self, id: i64) -> LatticeCell;
}

pub trait FireKernel {
    fn tile_generation(&self) -> i64;
    fn tile_index_of(&self, face: i64, ti: i64, tj: i64) -> i64;
    fn tile_serial_of(&self, t: i64) -> i64;
    fn read_tile_stage(&self, t: i64, out: &mut [u8]);
    fn state_code_at(&self, face: i64, i: i64, j: i64) -> u8;
}

#[derive(Default)]
pub struct StageStats {
    pub syncs: i64,
    pub tiles_written: i64,
    pub bytes_written: i64,
    pub resolves: i64,
}

pub struct FireStageMap<'a, K: FireKernel, L: FireLattice> {
    kernel: &'a K,
    lattice: &'a L,
    pub data: Vec<u8>,
    pub width: usize,
    slots_across: i64,
    slot_capacity: usize,
    tiles_per_axis: i64,
    slot_of_key: HashMap<i64, usize>,
    key_of_slot: Vec<i64>,
    ref_count: Vec<i64>,
    seen_serial: Vec<i64>,
    tile_of_slot: Vec<i64>,
    free_slots: Vec<usize>,
    slot_count: usize,
    generation: i64,
    scratch: [u8; TILE_BYTES],
    dirty: Vec<usize>,
    pub stats: StageStats,
}

impl<'a, K: FireKernel, L: FireLattice> FireStageMap<'a, K, L> {
    pub fn new(kernel: &'a K, lattice: &'a L, slots_across: i64, slots_down: i64) -> Result<Self, String> {
        if lattice.cells_per_face() < TILE_SIZE {
            return Err("[fireStageMap] lattice too small".to_string());
        }
        let slot_capacity = (slots_across * slots_down) as usize;
        let width = (slots_across * TILE_SIZE) as usize;
        let height = (slots_down * TILE_SIZE) as usize;
        Ok(Self {
            kernel,
            lattice,
            data: vec![0; width * height * BYTES_PER_TEXEL],
            width,
            slots_across,
            slot_capacity,
            tiles_per_axis: (lattice.cells_per_face() + TILE_SIZE - 1) / TILE_SIZE,
            slot_of_key: HashMap::new(),
            key_of_slot: vec![-1; slot_capacity],
            ref_count: vec![0; slot_capacity],
            seen_serial: vec![-1; slot_capacity],
            tile_of_slot: vec![-1; slot_capacity],
            free_slots: Vec::new(),
            slot_count: 0,
            generation: -1,
            scratch: [0; TILE_BYTES],
            dirty: Vec::new(),
            stats: StageStats::default(),
        })
    }

    fn tile_key(&self, face: i64, ti: i64, tj: i64) -> i64 {
        (face * self.tiles_per_axis + ti) * self.tiles_per_axis + tj
    }

    fn decode_key(&self, key: i64) -> (i64, i64, i64) {
        let tj = key % self.tiles_per_axis;
        let rest = (key - tj) / self.tiles_per_axis;
        let ti = rest % self.tiles_per_axis;
        ((rest - ti) / self.tiles_per_axis, ti, tj)
    }

    pub fn block_origin(&self, slot: usize) -> (i64, i64) {
        let s = slot as i64;
        ((s % self.slots_across) * TILE_SIZE, (s / self.slots_across) * TILE_SIZE)
    }

    fn write_tile(&mut self, slot: usize) {
        let (bx, by) = self.block_origin(slot);
        let row_bytes = TEXELS_PER_TILE_ROW * BYTES_PER_TEXEL;
        for row in 0..TILE_SIZE as usize {
            let dst = ((by as usize + row) * self.width + bx as usize) * BYTES_PER_TEXEL;
            self.data[dst..dst + row_bytes].copy_from_slice(&self.scratch[row * row_bytes..(row + 1) * row_bytes]);
        }
    }

    fn clear_tile(&mut self, slot: usize) {
        self.scratch = [0; TILE_BYTES];
        self.write_tile(slot);
    }

    pub fn acquire(&mut self, face: i64, ti: i64, tj: i64) -> Result<usize, String> {
        let key = self.tile_key(face, ti, tj);
        if let Some(&slot) = self.slot_of_key.get(&key) {
            self.ref_count[slot] += 1;
            return Ok(slot);
        }
        let slot = if let Some(slot) = self.free_slots.pop() {
            slot
        } else if self.slot_count < self.slot_capacity {
            self.slot_count += 1;
            self.slot_count - 1
        } else {
            return Err(format!("[fireStageMap] all {} tile slots are in use", self.slot_capacity));
        };
        self.slot_of_key.insert(key, slot);
        self.key_of_slot[slot] = key;
        self.ref_count[slot] = 0;
        self.seen_serial[slot] = -1;
        self.tile_of_slot[slot] = -1;
        self.clear_tile(slot);
        self.ref_count[slot] += 1;
        Ok(slot)
    }

    pub fn release(&mut self, face: i64, ti: i64, tj: i64) -> bool {
        let key = self.tile_key(face, ti, tj);
        let Some(&slot) = self.slot_of_key.get(&key) else {
            return false;
        };
        self.ref_count[slot] -= 1;
        if self.ref_count[slot] > 0 {
            return true;
        }
        self.slot_of_key.remove(&key);
        self.key_of_slot[slot] = -1;
        self.tile_of_slot[slot] = -1;
        self.clear_tile(slot);
        self.free_slots.push(slot);
        true
    }

    pub fn slot_value_of_cell(&self, face: i64, i: i64, j: i64) -> i64 {
        match self.slot_of_key.get(&self.tile_key(face, i >> 3, j >> 3)) {
            Some(&slot) => slot as i64 * TILE_CELLS + ((j & 7) << 3) + (i & 7),
            None => -1,
        }
    }

    pub fn slot_value_of_trunk(&self, id: i64) -> i64 {
        let cell = self.lattice.cell_of_placement_id(id);
        self.slot_value_of_cell(cell.face, cell.i, cell.j)
    }

    pub fn texel_of(&self, value: i64) -> (i64, i64) {
        let slot = value / TILE_CELLS;
        let local = value % TILE_CELLS;
        ((slot % self.slots_across) * TILE_SIZE + (local & 7), (slot / self.slots_across) * TILE_SIZE + (local >> 3))
    }

    pub fn sync(&mut self) -> usize {
        self.stats.syncs += 1;
        let generation = self.kernel.tile_generation();
        let regenerate = generation != self.generation;
        if regenerate {
            self.generation = generation;
            self.stats.resolves += 1;
        }
        self.dirty.clear();
        for slot in 0..self.slot_count {
            let key = self.key_of_slot[slot];
            if key < 0 {
                continue;
            }
            let mut t = self.tile_of_slot[slot];
            if regenerate || t < 0 {
                let (face, ti, tj) = self.decode_key(key);
                t = self.kernel.tile_index_of(face, ti, tj);
                self.tile_of_slot[slot] = t;
                if regenerate {
                    self.seen_serial[slot] = -1;
                }
            }
            if t < 0 {
                if self.seen_serial[slot] != 0 {
                    self.clear_tile(slot);
                    self.seen_serial[slot] = 0;
                    self.dirty.push(slot);
                    self.stats.bytes_written += TILE_BYTES as i64;
                }
                continue;
            }
            let serial = self.kernel.tile_serial_of(t);
            if serial == self.seen_serial[slot] {
                continue;
            }
            self.kernel.read_tile_stage(t, &mut self.scratch);
            self.write_tile(slot);
            self.seen_serial[slot] = serial;
            self.dirty.push(slot);
            self.stats.bytes_written += TILE_BYTES as i64;
        }
        self.stats.tiles_written += self.dirty.len() as i64;
        self.dirty.len()
    }

    pub fn dirty_slots(&self) -> &[usize] {
        &self.dirty
    }

    pub fn watched_tiles(&self) -> usize {
        self.slot_of_key.len()
    }
}

pub fn burn_mask_window(kernel: &impl FireKernel, face: i64, i0: i64, j0: i64, width: i64, height: i64, out: &mut [u8]) -> usize {
    let mut lit = 0;
    for j in 0..height {
        for i in 0..width {
            let state = kernel.state_code_at(face, i0 + i, j0 + j);
            out[(j * width + i) as usize] = match state {
                2 => 255,
                1 => 128,
                _ => 0,
            };
            if state != 0 {
                lit += 1;
            }
        }
    }
    lit
}

mod tests {
    use super::*;
    use crate::fnv1a::fnv1a_bytes;
    use serde_json::{json, Value};
    use std::cell::{Cell, RefCell};

    struct StubLattice {
        cells_per_face: i64,
    }

    impl FireLattice for StubLattice {
        fn cells_per_face(&self) -> i64 {
            self.cells_per_face
        }

        fn cell_of_placement_id(&self, id: i64) -> LatticeCell {
            LatticeCell { face: id % 3, i: (id * 5) % 24, j: (id * 7) % 24 }
        }
    }

    struct StubKernel {
        generation: Cell<i64>,
        serials: RefCell<HashMap<i64, i64>>,
    }

    impl FireKernel for StubKernel {
        fn tile_generation(&self) -> i64 {
            self.generation.get()
        }

        fn tile_index_of(&self, face: i64, ti: i64, tj: i64) -> i64 {
            if (face + ti + tj).rem_euclid(4) == 3 {
                -1
            } else {
                (face * 8 + ti) * 8 + tj
            }
        }

        fn tile_serial_of(&self, t: i64) -> i64 {
            self.serials.borrow().get(&t).copied().unwrap_or(1)
        }

        fn read_tile_stage(&self, t: i64, out: &mut [u8]) {
            let s = self.tile_serial_of(t);
            for (k, byte) in out.iter_mut().enumerate() {
                *byte = ((t * 37 + s * 11 + k as i64 * 5) & 255) as u8;
            }
        }

        fn state_code_at(&self, face: i64, i: i64, j: i64) -> u8 {
            ((face * 3 + i * 5 + j * 7) % 3) as u8
        }
    }

    fn int(step: &Value, key: &str) -> i64 {
        step[key].as_i64().unwrap()
    }

    fn apply(map: &mut FireStageMap<StubKernel, StubLattice>, kernel: &StubKernel, step: &Value) -> Value {
        match step["op"].as_str().unwrap() {
            "acquire" => match map.acquire(int(step, "face"), int(step, "ti"), int(step, "tj")) {
                Ok(slot) => json!(slot),
                Err(message) => json!({ "error": message }),
            },
            "release" => json!(map.release(int(step, "face"), int(step, "ti"), int(step, "tj"))),
            "slotOfCell" => json!(map.slot_value_of_cell(int(step, "face"), int(step, "I"), int(step, "J"))),
            "slotOfTrunk" => json!(map.slot_value_of_trunk(int(step, "id"))),
            "texel" => {
                let (x, y) = map.texel_of(int(step, "value"));
                json!({ "x": x, "y": y })
            }
            "blockOrigin" => {
                let (x, y) = map.block_origin(int(step, "slot") as usize);
                json!({ "x": x, "y": y })
            }
            "sync" => {
                let changed = map.sync();
                let mut dirty = map.dirty_slots().to_vec();
                dirty.sort_unstable();
                json!({
                    "changed": changed,
                    "hash": fnv1a_bytes(&map.data),
                    "watched": map.watched_tiles(),
                    "stats": {
                        "syncs": map.stats.syncs,
                        "tilesWritten": map.stats.tiles_written,
                        "bytesWritten": map.stats.bytes_written,
                        "resolves": map.stats.resolves
                    },
                    "dirty": dirty
                })
            }
            "generation" => {
                kernel.generation.set(int(step, "value"));
                Value::Null
            }
            "serial" => {
                kernel.serials.borrow_mut().insert(int(step, "t"), int(step, "value"));
                Value::Null
            }
            "burnMask" => {
                let (w, h) = (int(step, "w"), int(step, "h"));
                let mut out = vec![0u8; (w * h) as usize];
                let lit = burn_mask_window(kernel, int(step, "face"), int(step, "I0"), int(step, "J0"), w, h, &mut out);
                json!({ "lit": lit, "out": out })
            }
            other => panic!("unknown fire stage map op {other}"),
        }
    }

    #[test]
    fn create_fire_stage_map_matches_spoint() {
        let fixture: Value = crate::testkit::parse(include_str!("../testdata/parity/spoint-fire-stage-map.json"));
        let slots_across = int(&fixture, "slotsAcross");
        let slots_down = int(&fixture, "slotsDown");
        let kernel = StubKernel { generation: Cell::new(0), serials: RefCell::new(HashMap::new()) };
        let refused = FireStageMap::new(&kernel, &StubLattice { cells_per_face: 4 }, slots_across, slots_down).err().map(|message| json!({ "error": message }));
        assert_eq!(refused, Some(fixture["refusedConstruct"].clone()));
        let lattice = StubLattice { cells_per_face: int(&fixture["lattice"], "cellsPerFace") };
        let mut map = FireStageMap::new(&kernel, &lattice, slots_across, slots_down).unwrap();
        for (n, step) in fixture["steps"].as_array().unwrap().iter().enumerate() {
            assert_eq!(apply(&mut map, &kernel, step), step["expect"], "step {n}: {}", step["op"]);
        }
    }
}
