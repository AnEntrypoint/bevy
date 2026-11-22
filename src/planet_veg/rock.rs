use std::f64::consts::PI;

use super::context::{
    hash3, is_seabed, quat_mul_f32, rand, relief_margin_scale, tangent_frame, tangent_frame_quat, tangent_to_local, value_noise3, ChunkFrame, ChunkLayout, PlacementCell,
    PlacementContext, SectorClimate, ARIDITY_LINE,
};
use super::tree::TREELINE_M;
use crate::height::ops_jsgen::V3;
use crate::js_math::round_half_up;
use crate::vec3::normalise;
use crate::planet_lod::guard;

pub const CHUNK_M: f64 = 32.0;
pub const CELL_M: f64 = 8.0;
pub const GRID: f64 = 4.0;
pub const JITTER_M: f64 = 3.2;
pub const SLOPE_D: f64 = 1.5;
pub const WATER_MARGIN_M: f64 = 0.3;
pub const TYPES: u32 = 6;
pub const FLOOR: f64 = 0.025;
pub const SCALE_MIN: f64 = 2.0;
pub const SCALE_SPAN: f64 = 8.0;
pub const SQUASH_MIN: f64 = 0.55;
pub const SQUASH_SPAN: f64 = 0.55;
pub const SCALE_LEVELS: usize = 10;
pub const SQUASH_LEVELS: usize = 3;
#[cfg(test)]
pub const MAX_INSTANCES: usize = 12000;

const K_JIT_X: i32 = 0;
const K_JIT_Z: i32 = 1;
const K_COIN: i32 = 10;
const K_TYPE: i32 = 11;
const K_SCALE: i32 = 12;
const K_YAW: i32 = 13;
const K_TILT_X: i32 = 14;
const K_TILT_Z: i32 = 15;
const K_SQUASH: i32 = 16;
const K_VARIANT: i32 = 17;

const SEED_SALT: u32 = 0x70c5;
const CELL_HASH_SEED: u32 = 0x70c;
const PATCH_SEED: u32 = 0x70c1;
const CLUSTER_SEED: u32 = 0x70c2;
const PATCH_CELL_M: f64 = 112.0;
const CLUSTER_CELL_M: f64 = 224.0;
const PATCH_FLOOR: f64 = 0.25;
const PATCH_SPAN: f64 = 1.5;
const MAX_CLUSTER_STRENGTH: f64 = 0.6;
const CLUSTER_MID: f64 = 0.5;
const CLUSTER_SCALE_BIAS: f64 = 0.6;
const TILT_WOBBLE: f64 = 0.12;
const BAND_FLOOR: f64 = 0.6;
const BAND_SPAN: f64 = 0.4;
const EROSION_FLOOR: f64 = 0.15;
const EROSION_SPAN: f64 = 0.85;
const SLOPE_FLOOR: f64 = 0.4;
const SLOPE_SPAN: f64 = 1.6;
const NORMAL_QUAT_EPS: f64 = 1e-6;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RockInstance {
    pub pos: [f32; 3],
    pub rock_type: u8,
    pub scale: f32,
    pub yaw: f32,
    pub squash: f32,
    pub variant: f32,
    pub normal: [f32; 3],
    pub tilt_quat: [f32; 4],
    pub rock_id: u32,
}

fn log_step(lo: f64, span: f64, levels: usize) -> f64 {
    ((lo + span) / lo).ln() / levels as f64
}

fn log_entry(lo: f64, span: f64, levels: usize, level: usize) -> f32 {
    (lo * ((level as f64 + 0.5) * log_step(lo, span, levels)).exp()) as f32
}

fn log_level(value: f64, lo: f64, span: f64, levels: usize) -> usize {
    let raw = round_half_up((value.max(lo) / lo).ln() / log_step(lo, span, levels) - 0.5);
    raw.clamp(0.0, (levels - 1) as f64) as usize
}

#[cfg(test)]
pub fn scale_table() -> [f32; SCALE_LEVELS] {
    std::array::from_fn(|level| log_entry(SCALE_MIN, SCALE_SPAN, SCALE_LEVELS, level))
}

#[cfg(test)]
pub fn squash_table() -> [f32; SQUASH_LEVELS] {
    std::array::from_fn(|level| log_entry(SQUASH_MIN, SQUASH_SPAN, SQUASH_LEVELS, level))
}

pub fn scale_level(scale: f64) -> usize {
    log_level(scale, SCALE_MIN, SCALE_SPAN, SCALE_LEVELS)
}

pub fn squash_level(squash: f64) -> usize {
    log_level(squash, SQUASH_MIN, SQUASH_SPAN, SQUASH_LEVELS)
}

pub fn rock_density(erosion: f64, slope_ratio: f64, humidity: f64, elev_norm: f64) -> f64 {
    let ero = erosion.clamp(0.0, 1.0);
    let slope = slope_ratio.clamp(0.0, 1.0);
    let hum = humidity.clamp(0.0, 1.0);
    let arid = if hum < ARIDITY_LINE { 1.0 + (1.0 - hum / ARIDITY_LINE) } else { 1.0 };
    let band = BAND_FLOOR + BAND_SPAN * elev_norm.clamp(0.0, 1.0);
    let d = (EROSION_FLOOR + EROSION_SPAN * ero) * (SLOPE_FLOOR + SLOPE_SPAN * slope * slope) * arid * band;
    d.clamp(FLOOR, 1.0)
}

#[cfg(test)]
pub fn room_under_cap(resident: usize, placed: usize) -> usize {
    placed.min(MAX_INSTANCES.saturating_sub(resident))
}

fn normal_quat(n: V3) -> [f64; 4] {
    let ang = n[1].clamp(-1.0, 1.0).acos();
    let (ax, az) = (n[2], -n[0]);
    let al = ax.hypot(az);
    if al < NORMAL_QUAT_EPS {
        return [0.0, 0.0, 0.0, 1.0];
    }
    let s = (ang / 2.0).sin();
    [ax / al * s, 0.0, az / al * s, (ang / 2.0).cos()].map(|c| f64::from(c as f32))
}

struct Classifier<'a> {
    ctx: &'a PlacementContext<'a>,
    frame: &'a ChunkFrame,
    climate: SectorClimate,
}

impl Classifier<'_> {
    fn classify(&mut self, cell: &PlacementCell) -> Option<RockInstance> {
        let climate = self.climate.at_dir(cell.dir);
        if is_seabed(climate.sea_bias) {
            return None;
        }
        let (erosion, humidity) = (climate.erosion, climate.humidity);
        let cell_hash = hash3(CELL_HASH_SEED, cell.row, cell.j);
        let patch = PATCH_FLOOR + PATCH_SPAN * value_noise3(PATCH_SEED, cell.sea, PATCH_CELL_M);
        let cluster = value_noise3(CLUSTER_SEED, cell.sea, CLUSTER_CELL_M);
        let coin = rand(cell_hash, K_COIN);
        let cluster_boost_bound = 1.0 + MAX_CLUSTER_STRENGTH * (cluster - CLUSTER_MID).max(0.0);
        let ceiling_over_all_elevations = (rock_density(erosion, 1.0, humidity, 1.0) * patch * cluster_boost_bound).clamp(FLOOR, 1.0) * cell.area;
        if coin >= ceiling_over_all_elevations {
            return None;
        }

        let surface = self.ctx.surface_along_dir(self.frame, cell.dir)?;
        let elev = surface.rho - self.ctx.radius;
        if !(elev > WATER_MARGIN_M * relief_margin_scale(self.ctx.relief_scale)) {
            return None;
        }
        let elev_norm = (elev / TREELINE_M).clamp(0.0, 1.0);
        let ceiling = (rock_density(erosion, 1.0, humidity, elev_norm) * patch * cluster_boost_bound).clamp(FLOOR, 1.0) * cell.area;
        if coin >= ceiling {
            return None;
        }

        let tf = tangent_frame(self.frame, cell.dir);
        let slope = self.ctx.radial_slope(self.frame, &tf, surface.rho, SLOPE_D)?;
        let grad = slope[0].hypot(slope[1]);
        let slope_ratio = grad / (grad + 1.0);
        let painted_rock = self.ctx.painted_weights(cell.dir, elev, slope, climate.temp, humidity).rock;
        let normal = normalise([-slope[0], 1.0, -slope[1]]);

        let cluster_strength = MAX_CLUSTER_STRENGTH * (1.0 - slope_ratio);
        let accept = (rock_density(erosion, slope_ratio, humidity, elev_norm) * patch * (1.0 + cluster_strength * (cluster - CLUSTER_MID))).clamp(FLOOR, 1.0) * cell.area * painted_rock;
        if coin >= accept {
            return None;
        }

        let rock_type = (rand(cell_hash, K_TYPE) * f64::from(TYPES)).floor() as u32 % TYPES;
        let scale_bias = (rand(cell_hash, K_SCALE) + (cluster - CLUSTER_MID) * CLUSTER_SCALE_BIAS).clamp(0.0, 1.0);
        let scale = log_entry(SCALE_MIN, SCALE_SPAN, SCALE_LEVELS, scale_level(SCALE_MIN + scale_bias * scale_bias * scale_bias * SCALE_SPAN));
        let yaw = (rand(cell_hash, K_YAW) * PI * 2.0) as f32;
        let wobble_x = (rand(cell_hash, K_TILT_X) - 0.5) * TILT_WOBBLE;
        let wobble_z = (rand(cell_hash, K_TILT_Z) - 0.5) * TILT_WOBBLE;
        let squash = log_entry(SQUASH_MIN, SQUASH_SPAN, SQUASH_LEVELS, squash_level(SQUASH_MIN + rand(cell_hash, K_SQUASH) * SQUASH_SPAN));
        let variant = rand(cell_hash, K_VARIANT) as f32;
        let tilted = normalise([normal[0] + wobble_x, normal[1], normal[2] + wobble_z]);
        let tilt_quat = quat_mul_f32(tangent_frame_quat(&tf), normal_quat(tilted));
        Some(RockInstance {
            pos: surface.at.map(|c| c as f32),
            rock_type: rock_type as u8,
            scale,
            yaw,
            squash,
            variant,
            normal: tangent_to_local(&tf, normal[0], normal[1], normal[2]).map(|c| c as f32),
            tilt_quat,
            rock_id: cell.id as u64 as u32,
        })
    }
}

const LAYOUT: ChunkLayout = ChunkLayout { chunk_m: CHUNK_M, cell_m: CELL_M, grid: GRID, jitter_m: JITTER_M, seed_salt: SEED_SALT, k_x: K_JIT_X, k_z: K_JIT_Z };

pub fn place_rock_chunk(ctx: &PlacementContext, key: u64) -> Vec<RockInstance> {
    guard::note_build();
    let (frame, cells) = ctx.chunk_cells(&LAYOUT, key);
    let mut classifier = Classifier { ctx, frame: &frame, climate: ctx.sector_climate() };
    cells.iter().filter_map(|cell| classifier.classify(cell)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet_veg::context::fixture::{context, RADIUS, RELIEF_SCALE, SEED};
    use crate::planet_veg::context::SEA_REJECT;
    use crate::testkit::{cases, close, parse};
    use serde_json::Value;
    use std::sync::OnceLock;

    const F32_FIELD_TOLERANCE: f64 = 1e-7;
    const POSITION_TOLERANCE_M: f64 = 1e-6;
    const WORLD_TOLERANCE_M: f64 = 1e-5;
    const EXACT: f64 = 1e-12;
    const FIXTURE_CHUNKS: usize = 46;
    const RING_CHUNKS: usize = 416;
    const RING_PLACED: u64 = 103;
    const PLACED_IN_FIXTURE_CHUNKS: usize = 60;

    fn fixture() -> &'static Value {
        static FIXTURE: OnceLock<Value> = OnceLock::new();
        FIXTURE.get_or_init(|| parse(include_str!("../../testdata/parity/spoint-planet-rock-placement.json")))
    }

    fn floats(value: &Value) -> Vec<f64> {
        value.as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect()
    }

    fn close_all(got: &[f32], want: &Value, tolerance: f64, context: &str) {
        let want = floats(want);
        assert_eq!(got.len(), want.len(), "{context} length");
        for (i, (g, w)) in got.iter().zip(&want).enumerate() {
            close(f64::from(*g), *w, tolerance, &format!("{context}[{i}]"));
        }
    }

    #[test]
    fn constants_match_the_fixture() {
        let f = fixture();
        let rock = &f["rock"];
        let got = [CHUNK_M, CELL_M, GRID, JITTER_M, SLOPE_D, WATER_MARGIN_M, SEA_REJECT, f64::from(TYPES), FLOOR, SCALE_MIN, SCALE_SPAN, SQUASH_MIN, SQUASH_SPAN];
        let want = ["CHUNK", "CELL", "GRID", "JITTER", "SLOPE_D", "WATER_MARGIN", "SEA_REJECT", "TYPES", "FLOOR", "SCALE_MIN", "SCALE_SPAN", "SQUASH_MIN", "SQUASH_SPAN"];
        assert_eq!(rock.as_object().unwrap().len(), want.len());
        for (g, name) in got.iter().zip(want) {
            close(*g, rock[name].as_f64().unwrap(), EXACT, name);
        }
        assert_eq!(SCALE_LEVELS as u64, f["scaleLevels"].as_u64().unwrap());
        assert_eq!(SQUASH_LEVELS as u64, f["squashLevels"].as_u64().unwrap());
        close_all(&scale_table(), &f["scaleTable"], EXACT, "scale table");
        close_all(&squash_table(), &f["squashTable"], EXACT, "squash table");
        let keys = [("K_COIN", K_COIN), ("K_TYPE", K_TYPE), ("K_SCALE", K_SCALE), ("K_YAW", K_YAW), ("K_TILTX", K_TILT_X), ("K_TILTZ", K_TILT_Z), ("K_SQUASH", K_SQUASH), ("K_VAR", K_VARIANT)];
        let table = f["keyTable"].as_object().unwrap();
        assert_eq!(table.len(), keys.len());
        for (name, k) in keys {
            assert_eq!(table[name].as_i64().unwrap(), i64::from(k), "{name}");
        }
        assert_eq!(f["rockConfig"]["rockMaxInstances"].as_u64().unwrap() as usize, MAX_INSTANCES);
        close(f["planet"]["radius"].as_f64().unwrap(), RADIUS, EXACT, "radius");
        close(f["planet"]["reliefScale"].as_f64().unwrap(), RELIEF_SCALE, EXACT, "relief scale");
        assert_eq!(f["planet"]["seed"].as_u64().unwrap(), u64::from(SEED));
    }

    #[test]
    fn levels_round_to_the_nearest_table_entry_in_log_space() {
        let scales = scale_table();
        for (level, entry) in scales.iter().enumerate() {
            assert_eq!(scale_level(f64::from(*entry)), level);
        }
        let squashes = squash_table();
        for (level, entry) in squashes.iter().enumerate() {
            assert_eq!(squash_level(f64::from(*entry)), level);
        }
        assert_eq!(scale_level(0.0), 0);
        assert_eq!(scale_level(1e9), SCALE_LEVELS - 1);
        assert_eq!(squash_level(0.0), 0);
        assert_eq!(squash_level(1e9), SQUASH_LEVELS - 1);
    }

    #[test]
    fn placements_match_the_spoint_chunks() {
        let f = fixture();
        let ctx = context();
        let lattice = ctx.lattice(CHUNK_M, GRID);
        let chunks = cases(f, "chunks");
        assert_eq!(chunks.len(), FIXTURE_CHUNKS);
        assert_eq!(f["chunkCount"].as_u64().unwrap() as usize, FIXTURE_CHUNKS);
        let mut compared = 0;
        for chunk in chunks {
            let key = chunk["key"].as_u64().unwrap();
            let label = format!("{} chunk {key}", chunk["role"].as_str().unwrap());
            let got = place_rock_chunk(&ctx, key);
            let want = chunk["instances"].as_array().unwrap();
            assert_eq!(got.len(), chunk["placedCount"].as_u64().unwrap() as usize, "{label} placed count");
            assert_eq!(got.len(), want.len(), "{label} instance count");
            let frame = ctx.chunk_frame(&lattice, key);
            for (i, (rock, w)) in got.iter().zip(want).enumerate() {
                let at = format!("{label} rock {i}");
                close_all(&rock.pos, &serde_json::json!([w["x"], w["y"], w["z"]]), POSITION_TOLERANCE_M, &format!("{at} pos"));
                assert_eq!(u64::from(rock.rock_type), w["type"].as_u64().unwrap(), "{at} type");
                assert_eq!(u64::from(rock.rock_id), w["rockId"].as_u64().unwrap(), "{at} id");
                close(f64::from(rock.scale), w["scale"].as_f64().unwrap(), EXACT, &format!("{at} scale"));
                close(f64::from(rock.squash), w["squash"].as_f64().unwrap(), EXACT, &format!("{at} squash"));
                close(f64::from(rock.yaw), w["yaw"].as_f64().unwrap(), F32_FIELD_TOLERANCE, &format!("{at} yaw"));
                close(f64::from(rock.variant), w["variant"].as_f64().unwrap(), F32_FIELD_TOLERANCE, &format!("{at} variant"));
                close_all(&rock.normal, &w["normal"], F32_FIELD_TOLERANCE, &format!("{at} normal"));
                close_all(&rock.tilt_quat, &w["tiltQuat"], F32_FIELD_TOLERANCE, &format!("{at} tilt"));
                let world = frame.local_to_world(f64::from(rock.pos[0]), f64::from(rock.pos[1]), f64::from(rock.pos[2]));
                for (axis, (g, v)) in world.iter().zip(floats(&w["world"])).enumerate() {
                    close(*g, v, WORLD_TOLERANCE_M, &format!("{at} world[{axis}]"));
                }
                compared += 1;
            }
        }
        assert_eq!(compared, PLACED_IN_FIXTURE_CHUNKS);
    }

    #[test]
    fn the_320_m_ring_places_the_reference_count_in_every_chunk() {
        let ring = &fixture()["ring"];
        let ctx = context();
        let chunks = cases(ring, "chunks");
        assert_eq!(chunks.len(), RING_CHUNKS);
        let mut placed = 0;
        for entry in chunks {
            let key = entry[0].as_u64().unwrap();
            let got = place_rock_chunk(&ctx, key).len() as u64;
            assert_eq!(got, entry[1].as_u64().unwrap(), "ring chunk {key}");
            placed += got;
        }
        assert_eq!(placed, ring["placed"].as_u64().unwrap());
        assert_eq!(placed, RING_PLACED);
    }

    #[test]
    fn residency_follows_placement_up_to_the_instance_cap() {
        assert_eq!(MAX_INSTANCES, 12000);
        assert_eq!(room_under_cap(0, RING_PLACED as usize), RING_PLACED as usize);
        assert_eq!(room_under_cap(MAX_INSTANCES - 3, 10), 3);
        assert_eq!(room_under_cap(MAX_INSTANCES - 1, 1), 1);
        assert_eq!(room_under_cap(MAX_INSTANCES, 5), 0);
        assert_eq!(room_under_cap(MAX_INSTANCES + 7, 5), 0);
    }

    const REFERENCE_ROCK_DENSITY: [[f64; 5]; 81] = [
        [0.0, 0.0, 0.05, 0.0, 0.06557142857142856],
        [0.0, 0.0, 0.05, 0.5, 0.08742857142857142],
        [0.0, 0.0, 0.05, 1.0, 0.10928571428571428],
        [0.0, 0.0, 0.28, 0.0, 0.036],
        [0.0, 0.0, 0.28, 0.5, 0.048],
        [0.0, 0.0, 0.28, 1.0, 0.06],
        [0.0, 0.0, 0.6, 0.0, 0.036],
        [0.0, 0.0, 0.6, 0.5, 0.048],
        [0.0, 0.0, 0.6, 1.0, 0.06],
        [0.0, 0.4, 0.05, 0.0, 0.10753714285714287],
        [0.0, 0.4, 0.05, 0.5, 0.14338285714285717],
        [0.0, 0.4, 0.05, 1.0, 0.17922857142857146],
        [0.0, 0.4, 0.28, 0.0, 0.05904000000000001],
        [0.0, 0.4, 0.28, 0.5, 0.07872000000000001],
        [0.0, 0.4, 0.28, 1.0, 0.09840000000000002],
        [0.0, 0.4, 0.6, 0.0, 0.05904000000000001],
        [0.0, 0.4, 0.6, 0.5, 0.07872000000000001],
        [0.0, 0.4, 0.6, 1.0, 0.09840000000000002],
        [0.0, 1.0, 0.05, 0.0, 0.3278571428571428],
        [0.0, 1.0, 0.05, 0.5, 0.4371428571428571],
        [0.0, 1.0, 0.05, 1.0, 0.5464285714285714],
        [0.0, 1.0, 0.28, 0.0, 0.18],
        [0.0, 1.0, 0.28, 0.5, 0.24],
        [0.0, 1.0, 0.28, 1.0, 0.3],
        [0.0, 1.0, 0.6, 0.0, 0.18],
        [0.0, 1.0, 0.6, 0.5, 0.24],
        [0.0, 1.0, 0.6, 1.0, 0.3],
        [0.3, 0.0, 0.05, 0.0, 0.1770428571428572],
        [0.3, 0.0, 0.05, 0.5, 0.23605714285714294],
        [0.3, 0.0, 0.05, 1.0, 0.29507142857142865],
        [0.3, 0.0, 0.28, 0.0, 0.09720000000000002],
        [0.3, 0.0, 0.28, 0.5, 0.12960000000000002],
        [0.3, 0.0, 0.28, 1.0, 0.16200000000000003],
        [0.3, 0.0, 0.6, 0.0, 0.09720000000000002],
        [0.3, 0.0, 0.6, 0.5, 0.12960000000000002],
        [0.3, 0.0, 0.6, 1.0, 0.16200000000000003],
        [0.3, 0.4, 0.05, 0.0, 0.2903502857142858],
        [0.3, 0.4, 0.05, 0.5, 0.3871337142857144],
        [0.3, 0.4, 0.05, 1.0, 0.483917142857143],
        [0.3, 0.4, 0.28, 0.0, 0.15940800000000005],
        [0.3, 0.4, 0.28, 0.5, 0.21254400000000007],
        [0.3, 0.4, 0.28, 1.0, 0.2656800000000001],
        [0.3, 0.4, 0.6, 0.0, 0.15940800000000005],
        [0.3, 0.4, 0.6, 0.5, 0.21254400000000007],
        [0.3, 0.4, 0.6, 1.0, 0.2656800000000001],
        [0.3, 1.0, 0.05, 0.0, 0.8852142857142858],
        [0.3, 1.0, 0.05, 0.5, 1.0],
        [0.3, 1.0, 0.05, 1.0, 1.0],
        [0.3, 1.0, 0.28, 0.0, 0.486],
        [0.3, 1.0, 0.28, 0.5, 0.6480000000000001],
        [0.3, 1.0, 0.28, 1.0, 0.81],
        [0.3, 1.0, 0.6, 0.0, 0.486],
        [0.3, 1.0, 0.6, 0.5, 0.6480000000000001],
        [0.3, 1.0, 0.6, 1.0, 0.81],
        [1.0, 0.0, 0.05, 0.0, 0.43714285714285717],
        [1.0, 0.0, 0.05, 0.5, 0.582857142857143],
        [1.0, 0.0, 0.05, 1.0, 0.7285714285714286],
        [1.0, 0.0, 0.28, 0.0, 0.24],
        [1.0, 0.0, 0.28, 0.5, 0.32000000000000006],
        [1.0, 0.0, 0.28, 1.0, 0.4],
        [1.0, 0.0, 0.6, 0.0, 0.24],
        [1.0, 0.0, 0.6, 0.5, 0.32000000000000006],
        [1.0, 0.0, 0.6, 1.0, 0.4],
        [1.0, 0.4, 0.05, 0.0, 0.7169142857142858],
        [1.0, 0.4, 0.05, 0.5, 0.9558857142857144],
        [1.0, 0.4, 0.05, 1.0, 1.0],
        [1.0, 0.4, 0.28, 0.0, 0.39360000000000006],
        [1.0, 0.4, 0.28, 0.5, 0.5248000000000002],
        [1.0, 0.4, 0.28, 1.0, 0.6560000000000001],
        [1.0, 0.4, 0.6, 0.0, 0.39360000000000006],
        [1.0, 0.4, 0.6, 0.5, 0.5248000000000002],
        [1.0, 0.4, 0.6, 1.0, 0.6560000000000001],
        [1.0, 1.0, 0.05, 0.0, 1.0],
        [1.0, 1.0, 0.05, 0.5, 1.0],
        [1.0, 1.0, 0.05, 1.0, 1.0],
        [1.0, 1.0, 0.28, 0.0, 1.0],
        [1.0, 1.0, 0.28, 0.5, 1.0],
        [1.0, 1.0, 0.28, 1.0, 1.0],
        [1.0, 1.0, 0.6, 0.0, 1.0],
        [1.0, 1.0, 0.6, 0.5, 1.0],
        [1.0, 1.0, 0.6, 1.0, 1.0],
    ];

    #[test]
    fn rock_density_matches_the_values_the_reference_function_returns() {
        for [erosion, slope, humidity, elev_norm, want] in REFERENCE_ROCK_DENSITY {
            close(rock_density(erosion, slope, humidity, elev_norm), want, EXACT, &format!("erosion {erosion} slope {slope} humidity {humidity} elevation {elev_norm}"));
        }
    }
}
