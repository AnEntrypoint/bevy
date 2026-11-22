use std::f64::consts::PI;

use super::context::{
    hash3, is_seabed, lean_quat, quat_mul_f32, rand, relief_margin_scale, rendered_soil_weight, smoothstep, tangent_frame, tangent_frame_quat, value_noise3,
    ChunkFrame, ChunkLayout, PlacementCell, PlacementContext, SectorClimate, ARIDITY_LINE,
};
use crate::height::ops_jsgen::V3;
use crate::js_math::round_half_up;
use crate::planet_lod::guard;
use crate::planet_material::splat::SplatWeights;

pub const CHUNK_M: f64 = 32.0;
pub const CELL_M: f64 = 4.0;
pub const GRID: f64 = 8.0;
pub const JITTER_M: f64 = 1.8;
pub const SLOPE_D: f64 = 1.5;
pub const SLOPE_MAX: f64 = 0.6;
pub const TREELINE_M: f64 = 4000.0;
pub const TREELINE_FADE_M: f64 = 120.0;
pub const SHAPE_VARIANTS: u8 = 3;

pub const SPECIES: [&str; 15] = [
    "Oak Large", "Pine Medium", "Aspen Medium", "Ash Medium", "Bush", "Ash Small", "Ash Large", "Aspen Small", "Aspen Large", "Bush 2", "Bush 3", "Oak Small", "Oak Medium", "Pine Small",
    "Pine Large",
];

const OAK: u8 = 0;
const PINE: u8 = 1;
const ASPEN: u8 = 2;
const ASH: u8 = 3;
const BUSH: u8 = 4;

const K_JIT_X: i32 = 0;
const K_JIT_Z: i32 = 1;
const K_COIN: i32 = 2;
const K_VARIANT: i32 = 3;
const K_YAW: i32 = 4;
const K_SCALE: i32 = 5;
const K_WIND: i32 = 6;
const K_SPECIES_H: i32 = 7;
const K_SIZE: i32 = 8;
const K_SHAPE: i32 = 9;
const K_TINT_HUE: i32 = 10;
const K_TINT_SAT: i32 = 11;
const K_TINT_VAL: i32 = 12;
const K_LEAN: i32 = 13;
const K_LEAN_DIR: i32 = 14;
const K_UNDERSTORY: i32 = 15;
const K_COMPANION: i32 = 16;

const SEED_SALT: u32 = 0x7eed;
const CELL_HASH_SEED: u32 = 0x5eed;
const GROVE_SEED: u32 = 0x67a0e5;
const GROVE_DETAIL_SALT: u32 = 0x9e37;
const PATCH_T_SEED: u32 = 0x51a7c1;
const PATCH_H_SEED: u32 = 0x2b3c4d;
const SEASON_SEED: u32 = 0x5ea50a;
const GROVE_CELL_M: f64 = 96.0;
const GROVE_DETAIL_CELL_M: f64 = 37.0;
const PATCH_CELL_M: f64 = 64.0;
const SEASON_CELL_M: f64 = 420.0;
const GROVE_MUL_FLOOR: f64 = 0.15;
const GROVE_MUL_SPAN: f64 = 1.45;
const GROVE_EDGES: [f64; 2] = [0.30, 0.70];
const GROVE_COARSE_WEIGHT: f64 = 0.7;
const GROVE_DETAIL_WEIGHT: f64 = 0.3;
const UNDERSTORY_BASE: f64 = 0.08;
const UNDERSTORY_CLEARING_GAIN: f64 = 0.35;
const SCALE_LOW: f64 = 0.62;
const SCALE_SPAN_AGE: f64 = 0.72;
const SCALE_AGE_SKEW: f64 = 1.4;
const SIZE_CLASS_SCALE: [f64; 3] = [0.90, 1.0, 1.10];
const SIZE_CLASS_CUTS: [f64; 2] = [0.40, 0.75];
const SIZE_RANDOM_WEIGHT: f64 = 0.7;
const SIZE_LOWLAND_WEIGHT: f64 = 0.3;
const LEAN_MAX_TREE: f64 = 0.09;
const LEAN_MAX_BUSH: f64 = 0.16;
const TINT_BRIGHT_LOW: f64 = 0.80;
const TINT_BRIGHT_SPAN: f64 = 0.36;
const TINT_AGE_DARKEN: f64 = 0.06;
const TINT_SAT_LOW: f64 = 0.35;
const TINT_SAT_SPAN: f64 = 0.65;
const TINT_HUE_RANDOM_WEIGHT: f64 = 0.55;
const TINT_HUE_SEASON_WEIGHT: f64 = 0.45;
const TINT_WARM: [f64; 3] = [1.22, 0.76, 0.52];
const TINT_COOL: [f64; 3] = [0.64, 1.07, 0.58];
const PINE_TINT_STRENGTH: f64 = 0.4;
const BUSH_TINT_STRENGTH: f64 = 0.8;
const COMPANION_CHANCE: f64 = 0.22;
const PINE_ELEV_NORM: f64 = 0.62;
const PINE_TEMP: f64 = 0.30;
const OAK_TEMP: f64 = 0.62;
const OAK_HUMIDITY: f64 = 0.55;
const ASH_HUMIDITY: f64 = 0.50;
const SPECIES_ROUND: f64 = 10.0;
const PATCH_SPREAD: f64 = 0.60;
const VARIANT_SPREAD: f64 = 0.30;

pub const FALLOFF_NEAR_M: f64 = 96.0;
pub const FALLOFF_FLOOR: f64 = 0.15;
pub const RENDER_DISTANCE_M: f64 = 640.0;
pub const RING_M: f64 = RENDER_DISTANCE_M + 40.0;

const SIZE_SIBLINGS: [[u8; 3]; 5] = [[11, 12, OAK], [13, PINE, 14], [7, ASPEN, 8], [5, ASH, 6], [BUSH, 9, 10]];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TreeInstance {
    pub pos: [f32; 3],
    pub species: u8,
    pub shape: u8,
    pub scale: f32,
    pub yaw: f32,
    pub wind_phase: f32,
    pub tint: [f32; 3],
    pub tilt_quat: [f32; 4],
    pub normal: [f32; 3],
    pub trunk_id: u32,
}

fn size_class_of(size_r: f64, elev_norm: f64) -> usize {
    let p = size_r * SIZE_RANDOM_WEIGHT + (1.0 - elev_norm.clamp(0.0, 1.0)) * SIZE_LOWLAND_WEIGHT;
    if p < SIZE_CLASS_CUTS[0] {
        0
    } else if p < SIZE_CLASS_CUTS[1] {
        1
    } else {
        2
    }
}

fn size_sibling(genus: u8, size_class: usize) -> u8 {
    SIZE_SIBLINGS[usize::from(genus)][size_class]
}

fn species_for(temp: f64, humidity: f64, elev_norm: f64, v_t: f64, v_h: f64) -> u8 {
    let t = round_half_up((temp + v_t) * SPECIES_ROUND) / SPECIES_ROUND;
    let h = round_half_up((humidity + v_h) * SPECIES_ROUND) / SPECIES_ROUND;
    if round_half_up(humidity * SPECIES_ROUND) / SPECIES_ROUND < ARIDITY_LINE {
        BUSH
    } else if elev_norm > PINE_ELEV_NORM || t < PINE_TEMP {
        PINE
    } else if t > OAK_TEMP && h > OAK_HUMIDITY {
        OAK
    } else if h > ASH_HUMIDITY {
        ASH
    } else {
        ASPEN
    }
}

fn companion_of(genus: u8) -> Option<u8> {
    match genus {
        OAK => Some(ASH),
        ASH => Some(ASPEN),
        ASPEN => Some(OAK),
        PINE => Some(ASPEN),
        _ => None,
    }
}

fn snow_dominates(w: &SplatWeights) -> bool {
    w.snow > w.grass && w.snow > w.rock && w.snow > w.sand
}

fn base_density(temp: f64, humidity: f64, erosion: f64) -> f64 {
    let wet = humidity.clamp(0.0, 1.0);
    let warm = temp.clamp(0.0, 1.0);
    let ero = erosion.clamp(0.0, 1.0);
    (0.12 + 0.73 * wet * (0.4 + 0.6 * warm)) * (1.0 - 0.45 * ero)
}

fn grove_at(sea: V3) -> f64 {
    value_noise3(GROVE_SEED, sea, GROVE_CELL_M) * GROVE_COARSE_WEIGHT + value_noise3(GROVE_SEED ^ GROVE_DETAIL_SALT, sea, GROVE_DETAIL_CELL_M) * GROVE_DETAIL_WEIGHT
}

fn tint_multiplier(hue: f64, sat: f64, value: f64) -> [f32; 3] {
    let target = if hue >= 0.0 { TINT_WARM } else { TINT_COOL };
    let w = hue.abs() * sat;
    target.map(|t| (value * (1.0 + w * (t - 1.0))) as f32)
}

fn treeline_multiplier(elev: f64, relief_margin: f64) -> f64 {
    let treeline = TREELINE_M * relief_margin;
    if elev > treeline {
        1.0 - (elev - treeline) / (TREELINE_FADE_M * relief_margin)
    } else {
        1.0
    }
}

pub fn soil_multiplier(elev: f64, relief_scale: f64) -> f64 {
    rendered_soil_weight(elev) * treeline_multiplier(elev, relief_margin_scale(relief_scale))
}

pub fn tree_coin(trunk_id: u32) -> f64 {
    rand(trunk_id, 0)
}

pub fn veg_keep_prob(distance_m: f64) -> f64 {
    if distance_m <= FALLOFF_NEAR_M {
        1.0
    } else if distance_m >= RENDER_DISTANCE_M {
        FALLOFF_FLOOR
    } else {
        let t = (distance_m - FALLOFF_NEAR_M) / (RENDER_DISTANCE_M - FALLOFF_NEAR_M);
        1.0 - (1.0 - FALLOFF_FLOOR) * (t * t * (3.0 - 2.0 * t))
    }
}

pub fn thin_trees(instances: &[TreeInstance], player_distances: &[f64]) -> Vec<usize> {
    (0..instances.len())
        .filter(|&i| {
            let d = player_distances[i];
            d <= RING_M && (d <= FALLOFF_NEAR_M || tree_coin(instances[i].trunk_id) < veg_keep_prob(d))
        })
        .collect()
}

struct Classifier<'a, 'b> {
    ctx: &'a PlacementContext<'b>,
    frame: &'a ChunkFrame,
    climate: SectorClimate,
    density_ceiling: f64,
}

impl Classifier<'_, '_> {
    fn classify(&mut self, cell: &PlacementCell) -> Option<TreeInstance> {
        let grove_weight = smoothstep(GROVE_EDGES[0], GROVE_EDGES[1], grove_at(cell.sea));
        let grove_mul = (GROVE_MUL_FLOOR + GROVE_MUL_SPAN * grove_weight) * cell.area;
        let cell_hash = hash3(CELL_HASH_SEED, cell.row, cell.j);
        let coin = rand(cell_hash, K_COIN);
        if coin >= self.density_ceiling * grove_mul {
            return None;
        }
        let climate = self.climate.at_dir(cell.dir);
        if is_seabed(climate.sea_bias) {
            return None;
        }
        let base = base_density(climate.temp, climate.humidity, climate.erosion) * grove_mul;
        if coin >= base {
            return None;
        }
        let surface = self.ctx.surface_along_dir(self.frame, cell.dir)?;
        let elev = surface.rho - self.ctx.radius;
        let soil_mul = soil_multiplier(elev, self.ctx.relief_scale);
        if soil_mul <= 0.0 {
            return None;
        }
        let tf = tangent_frame(self.frame, cell.dir);
        let slope = self.ctx.radial_slope(self.frame, &tf, surface.rho, SLOPE_D)?;
        if slope[0].hypot(slope[1]) > SLOPE_MAX {
            return None;
        }
        let painted = self.ctx.painted_weights(cell.dir, elev, slope, climate.temp, climate.humidity);
        if snow_dominates(&painted) {
            return None;
        }
        let density_mul = soil_mul * painted.grass;
        if density_mul <= 0.0 || coin >= base * density_mul {
            return None;
        }
        let elev_norm = (elev / TREELINE_M).clamp(0.0, 1.0);
        let v_t = (rand(cell_hash, K_VARIANT) - 0.5) * VARIANT_SPREAD + (value_noise3(PATCH_T_SEED, cell.sea, PATCH_CELL_M) - 0.5) * PATCH_SPREAD;
        let v_h = (rand(cell_hash, K_SPECIES_H) - 0.5) * VARIANT_SPREAD + (value_noise3(PATCH_H_SEED, cell.sea, PATCH_CELL_M) - 0.5) * PATCH_SPREAD;
        let climate_genus = species_for(climate.temp, climate.humidity, elev_norm, v_t, v_h);
        let understory_chance = UNDERSTORY_BASE + UNDERSTORY_CLEARING_GAIN * (1.0 - grove_weight);
        let canopy_genus = match companion_of(climate_genus) {
            Some(companion) if rand(cell_hash, K_COMPANION) < COMPANION_CHANCE => companion,
            _ => climate_genus,
        };
        let genus = if rand(cell_hash, K_UNDERSTORY) < understory_chance { BUSH } else { canopy_genus };
        let size_class = size_class_of(rand(cell_hash, K_SIZE), elev_norm);
        let age = rand(cell_hash, K_SCALE).powf(SCALE_AGE_SKEW);
        let scale = ((SCALE_LOW + SCALE_SPAN_AGE * age) * SIZE_CLASS_SCALE[size_class]) as f32;
        let yaw = (rand(cell_hash, K_YAW) * PI * 2.0) as f32;
        let wind_phase = (rand(cell_hash, K_WIND) * PI * 2.0) as f32;
        let shape = ((rand(cell_hash, K_SHAPE) * f64::from(SHAPE_VARIANTS)).floor() as u8).min(SHAPE_VARIANTS - 1);
        let lean_r = rand(cell_hash, K_LEAN);
        let lean_max = if genus == BUSH { LEAN_MAX_BUSH } else { LEAN_MAX_TREE };
        let lean = lean_quat(lean_r * lean_r * lean_max, rand(cell_hash, K_LEAN_DIR) * PI * 2.0);
        let tilt_quat = quat_mul_f32(tangent_frame_quat(&tf), lean);
        let season = (value_noise3(SEASON_SEED, cell.sea, SEASON_CELL_M) - 0.5) * 2.0;
        let hue = ((rand(cell_hash, K_TINT_HUE) * 2.0 - 1.0) * TINT_HUE_RANDOM_WEIGHT + season * TINT_HUE_SEASON_WEIGHT).clamp(-1.0, 1.0);
        let genus_tint = match genus {
            PINE => PINE_TINT_STRENGTH,
            BUSH => BUSH_TINT_STRENGTH,
            _ => 1.0,
        };
        let sat = (TINT_SAT_LOW + TINT_SAT_SPAN * rand(cell_hash, K_TINT_SAT)) * genus_tint;
        let value = (TINT_BRIGHT_LOW + TINT_BRIGHT_SPAN * rand(cell_hash, K_TINT_VAL)) * (1.0 - TINT_AGE_DARKEN * (f64::from(scale) - 1.0));
        Some(TreeInstance {
            pos: surface.at.map(|c| c as f32),
            species: size_sibling(genus, size_class),
            shape,
            scale,
            yaw,
            wind_phase,
            tint: tint_multiplier(hue, sat, value),
            tilt_quat,
            normal: tf.up_l.map(|c| c as f32),
            trunk_id: cell.id as u64 as u32,
        })
    }
}

const LAYOUT: ChunkLayout = ChunkLayout { chunk_m: CHUNK_M, cell_m: CELL_M, grid: GRID, jitter_m: JITTER_M, seed_salt: SEED_SALT, k_x: K_JIT_X, k_z: K_JIT_Z };

pub fn place_tree_chunk(ctx: &PlacementContext, key: u64) -> Vec<TreeInstance> {
    guard::note_build();
    let (frame, cells) = ctx.chunk_cells(&LAYOUT, key);
    let mut classifier = Classifier { ctx, frame: &frame, climate: ctx.sector_climate(), density_ceiling: base_density(1.0, 1.0, 0.0) };
    cells.iter().filter_map(|cell| classifier.classify(cell)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::height::placement::PlacementLattice;
    use crate::height::{HashVersion, Planet, PlanetConfig};
    use crate::planet_veg::context::fixture::{context, RADIUS, RELIEF_SCALE, SEED};
    use crate::planet_veg::context::{painted_slope_of, placement_cell_at, CellJitter, SEA_REJECT};
    use crate::testkit::{cases, close, parse};
    use serde_json::Value;
    use std::sync::OnceLock;
    const F32_FIELD_TOLERANCE: f64 = 1e-9;
    const WORLD_TOLERANCE_M: f64 = 1e-9;
    const EXACT: f64 = 1e-12;
    const FIXTURE_CHUNKS: usize = 34;
    const RING_CHUNKS: usize = 1863;
    const RING_PLACED: u64 = 9888;
    const RING_RESIDENT: u64 = 4982;

    fn fixture() -> &'static Value {
        static FIXTURE: OnceLock<Value> = OnceLock::new();
        FIXTURE.get_or_init(|| parse(include_str!("../../testdata/parity/spoint-planet-veg-placement.json")))
    }

    fn v3(value: &Value) -> V3 {
        let a = value.as_array().unwrap();
        [a[0].as_f64().unwrap(), a[1].as_f64().unwrap(), a[2].as_f64().unwrap()]
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

    fn player_world() -> V3 {
        v3(&fixture()["thinning"]["playerWorld"])
    }

    fn distance(a: V3, b: V3) -> f64 {
        (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt()
    }

    fn key_of(chunk: &Value) -> u64 {
        chunk["key"].as_u64().unwrap()
    }

    #[test]
    fn constants_match_the_fixture() {
        let f = fixture();
        let veg = &f["veg"];
        let got = [CHUNK_M, CELL_M, GRID, JITTER_M, SLOPE_D, SLOPE_MAX, SEA_REJECT, TREELINE_M, TREELINE_FADE_M];
        let want = ["CHUNK", "CELL", "GRID", "JITTER", "SLOPE_D", "SLOPE_MAX", "SEA_REJECT", "TREELINE", "TREELINE_FADE"];
        for (g, name) in got.iter().zip(want) {
            close(*g, veg[name].as_f64().unwrap(), 0.0, name);
        }
        let species: Vec<&str> = f["species"].as_array().unwrap().iter().map(|s| s.as_str().unwrap()).collect();
        assert_eq!(species, SPECIES);
        let keys = [
            ("K_JITX", K_JIT_X), ("K_JITZ", K_JIT_Z), ("K_COIN", K_COIN), ("K_VARIANT", K_VARIANT), ("K_YAW", K_YAW), ("K_SCALE", K_SCALE), ("K_WIND", K_WIND), ("K_SPECIESH", K_SPECIES_H),
            ("K_SIZE", K_SIZE), ("K_SHAPE", K_SHAPE), ("K_TINT_HUE", K_TINT_HUE), ("K_TINT_SAT", K_TINT_SAT), ("K_TINT_VAL", K_TINT_VAL), ("K_LEAN", K_LEAN), ("K_LEAN_DIR", K_LEAN_DIR),
            ("K_UNDERSTORY", K_UNDERSTORY), ("K_COMPANION", K_COMPANION),
        ];
        let table = f["keyTable"]["VegPlacement.js"].as_object().unwrap();
        assert_eq!(table.len(), keys.len());
        for (name, k) in keys {
            assert_eq!(table[name].as_i64().unwrap(), i64::from(k), "{name}");
        }
        let thinning = &f["thinning"];
        close(FALLOFF_NEAR_M, thinning["nearM"].as_f64().unwrap(), 0.0, "near");
        close(FALLOFF_FLOOR, thinning["floor"].as_f64().unwrap(), 0.0, "floor");
        close(RENDER_DISTANCE_M, thinning["renderDistanceM"].as_f64().unwrap(), 0.0, "render distance");
        close(RING_M, thinning["residentRadiusM"].as_f64().unwrap(), 0.0, "ring");
    }

    #[test]
    fn treeline_follows_the_relief_scale_of_the_planet() {
        let f = fixture();
        let preset = relief_margin_scale(context().relief_scale);
        close(preset, 0.1, EXACT, "relief margin scale at the preset");
        close(TREELINE_M * preset, f["treelineM"].as_f64().unwrap(), EXACT, "treeline");
        close(TREELINE_FADE_M * preset, f["treelineFadeM"].as_f64().unwrap(), EXACT, "treeline fade");
        let relief = context().relief_scale;
        close(soil_multiplier(400.0, relief), 1.0, EXACT, "at the treeline");
        close(soil_multiplier(406.0, relief), 0.5, EXACT, "halfway through the fade");
        close(soil_multiplier(412.0, relief), 0.0, EXACT, "end of the fade");
        close(soil_multiplier(100.0, relief), 1.0, EXACT, "below the treeline");
        close(soil_multiplier(406.0, 0.01), 1.0, EXACT, "the treeline scales with the relief scale");
        close(soil_multiplier(15.0, relief), 1.0, EXACT, "soil weight is full at the beach top");
        close(soil_multiplier(4.5, relief), 0.0, EXACT, "soil weight is zero where only sand lies");
        close(soil_multiplier(9.75, relief), 0.5, EXACT, "soil weight is half way between");
        close(soil_multiplier(5.0, relief), 0.006586761688802504, EXACT, "soil weight just above the sand-only line");
        let defaults = |radius: f64| PlanetConfig { seed: SEED as i32, hpf_res: 128, hash: HashVersion::Integer, relief_scale: None, anchor_dir: [0.0, 1.0, 0.0], radius, carves: Vec::new() };
        let derived = Planet::new(&defaults(RADIUS));
        close(PlacementContext::new(&derived, SEED, RADIUS, HashVersion::Integer).relief_scale, RELIEF_SCALE, EXACT, "relief scale derived at radius 63600");
        let earth = Planet::new(&defaults(6_360_000.0));
        let earth_scale = PlacementContext::new(&earth, SEED, 6_360_000.0, HashVersion::Integer).relief_scale;
        close(relief_margin_scale(earth_scale), 10.0, EXACT, "relief margin scale at radius 6360000");
    }

    #[test]
    fn trunk_ids_keep_the_low_32_bits_of_cell_ids_beyond_2_pow_32() {
        let radius = 6_360_000.0;
        let lattice = PlacementLattice::new(radius, CHUNK_M, GRID).unwrap();
        let last = lattice.chunks_per_face - 1.0;
        let key = lattice.chunk_key(5.0, last, last);
        assert!(key > f64::from(u32::MAX));
        let dec = lattice.decode_chunk(key);
        let jitter = CellJitter { prejitter_seed: SEED ^ LAYOUT.seed_salt, over_cell: LAYOUT.jitter_m / LAYOUT.cell_m, k_x: LAYOUT.k_x, k_z: LAYOUT.k_z };
        let cell = placement_cell_at(&lattice, dec, GRID - 1.0, GRID - 1.0, &jitter);
        let cells_per_face = lattice.cells_per_face as u128;
        let exact = (5 * cells_per_face + cells_per_face - 1) * cells_per_face + cells_per_face - 1;
        assert!(exact > u128::from(u32::MAX));
        assert_eq!(cell.id as u128, exact);
        assert_eq!(cell.id as u64 as u32, exact as u32);
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
            let key = key_of(chunk);
            let label = format!("{} chunk {key}", chunk["role"].as_str().unwrap());
            let got = place_tree_chunk(&ctx, key);
            let want = chunk["instances"].as_array().unwrap();
            assert_eq!(got.len(), want.len(), "{label} count");
            assert_eq!(got.len() as u64, chunk["placedCount"].as_u64().unwrap(), "{label} placedCount");
            let frame = ctx.chunk_frame(&lattice, key);
            close(frame.anchor_height, chunk["frame"]["anchorHeight"].as_f64().unwrap(), WORLD_TOLERANCE_M, &format!("{label} anchor height"));
            for (i, (g, w)) in got.iter().zip(want).enumerate() {
                let at = format!("{label} instance {i}");
                assert_eq!(g.species as u64, w["species"].as_u64().unwrap(), "{at} species");
                assert_eq!(g.shape as u64, w["shape"].as_u64().unwrap(), "{at} shape");
                assert_eq!(u64::from(g.trunk_id), w["trunkId"].as_u64().unwrap() & u64::from(u32::MAX), "{at} trunk id");
                close_all(&g.pos, &serde_json::json!([w["x"], w["y"], w["z"]]), F32_FIELD_TOLERANCE, &format!("{at} position"));
                close(f64::from(g.scale), w["scale"].as_f64().unwrap(), F32_FIELD_TOLERANCE, &format!("{at} scale"));
                close(f64::from(g.yaw), w["yaw"].as_f64().unwrap(), F32_FIELD_TOLERANCE, &format!("{at} yaw"));
                close(f64::from(g.wind_phase), w["windPhase"].as_f64().unwrap(), F32_FIELD_TOLERANCE, &format!("{at} wind phase"));
                close_all(&g.tint, &w["tint"], F32_FIELD_TOLERANCE, &format!("{at} tint"));
                close_all(&g.tilt_quat, &w["tiltQuat"], F32_FIELD_TOLERANCE, &format!("{at} tilt"));
                close_all(&g.normal, &w["normal"], F32_FIELD_TOLERANCE, &format!("{at} normal"));
                let world = frame.local_to_world(f64::from(g.pos[0]), f64::from(g.pos[1]), f64::from(g.pos[2]));
                let want_world = v3(&w["world"]);
                for axis in 0..3 {
                    close(world[axis], want_world[axis], WORLD_TOLERANCE_M, &format!("{at} world[{axis}]"));
                }
                compared += 1;
            }
        }
        assert_eq!(compared, 653);
    }

    #[test]
    fn thinning_matches_the_recorded_coins_and_keep_probabilities() {
        let f = fixture();
        let mut checked = 0;
        for chunk in cases(f, "chunks") {
            let want = chunk["instances"].as_array().unwrap();
            let instances: Vec<TreeInstance> = want.iter().map(|w| TreeInstance { trunk_id: w["trunkId"].as_u64().unwrap() as u32, ..blank() }).collect();
            let distances: Vec<f64> = want.iter().map(|w| w["playerDistanceM"].as_f64().unwrap()).collect();
            for (w, (instance, d)) in want.iter().zip(instances.iter().zip(&distances)) {
                close(tree_coin(instance.trunk_id), w["thinCoin"].as_f64().unwrap(), EXACT, "thin coin");
                close(veg_keep_prob(*d), w["keepProb"].as_f64().unwrap(), EXACT, "keep probability");
                checked += 1;
            }
            let resident: Vec<usize> = (0..want.len()).filter(|&i| want[i]["resident"].as_bool().unwrap()).collect();
            assert_eq!(thin_trees(&instances, &distances), resident, "chunk {}", key_of(chunk));
        }
        assert_eq!(checked, 653);
        close(veg_keep_prob(96.0), 1.0, EXACT, "at the near distance");
        close(veg_keep_prob(640.0), 0.15, EXACT, "at the render distance");
        close(veg_keep_prob(368.0), 0.575, EXACT, "halfway through the falloff");
        close(veg_keep_prob(1000.0), 0.15, EXACT, "beyond the render distance");
    }

    #[test]
    fn species_follow_the_climate_cuts() {
        assert_eq!(species_for(0.5, 0.2, 0.0, 0.0, 0.0), BUSH);
        assert_ne!(species_for(0.5, 0.31, 0.0, 0.0, 0.0), BUSH);
        assert_eq!(species_for(0.7, 0.7, 0.63, 0.0, 0.0), PINE);
        assert_eq!(species_for(0.7, 0.7, 0.61, 0.0, 0.0), OAK);
        assert_eq!(species_for(0.2, 0.7, 0.0, 0.0, 0.0), PINE);
        assert_eq!(species_for(0.3, 0.7, 0.0, 0.0, 0.0), ASH);
        assert_eq!(species_for(0.7, 0.6, 0.0, 0.0, 0.0), OAK);
        assert_eq!(species_for(0.6, 0.6, 0.0, 0.0, 0.0), ASH);
        assert_eq!(species_for(0.7, 0.5, 0.0, 0.0, 0.0), ASPEN);
        assert_eq!(species_for(0.5, 0.6, 0.0, 0.0, 0.0), ASH);
        assert_eq!(species_for(0.5, 0.5, 0.0, 0.0, 0.0), ASPEN);
        assert_eq!(species_for(0.5, 0.5, 0.0, 0.0, 0.1), ASH);
        assert_eq!(species_for(0.5, 0.5, 0.0, -0.3, 0.0), PINE);
    }

    #[test]
    fn sea_and_snow_rules_reject_by_the_reference_comparisons() {
        assert!(is_seabed(-2.5));
        assert!(!is_seabed(-2.0));
        assert!(!is_seabed(5.0));
        let weights = |grass, rock, sand, snow| SplatWeights { grass, rock, sand, snow };
        assert!(snow_dominates(&weights(0.1, 0.1, 0.1, 0.7)));
        assert!(!snow_dominates(&weights(0.5, 0.1, 0.1, 0.3)));
        assert!(!snow_dominates(&weights(0.1, 0.5, 0.1, 0.3)));
        assert!(!snow_dominates(&weights(0.1, 0.1, 0.5, 0.3)));
        assert!(!snow_dominates(&weights(0.3, 0.1, 0.1, 0.3)));
    }

    #[test]
    fn painted_slope_uses_both_gradient_axes() {
        close(painted_slope_of([0.3, 0.4]), 1.0 - 1.0 / 1.25_f64.sqrt(), EXACT, "painted slope");
        close(painted_slope_of([0.0, 0.5]), 1.0 - 1.0 / 1.25_f64.sqrt(), EXACT, "painted slope north axis");
        close(painted_slope_of([0.0, 0.0]), 0.0, EXACT, "flat ground");
    }

    fn blank() -> TreeInstance {
        TreeInstance { pos: [0.0; 3], species: 0, shape: 0, scale: 0.0, yaw: 0.0, wind_phase: 0.0, tint: [0.0; 3], tilt_quat: [0.0; 4], normal: [0.0; 3], trunk_id: 0 }
    }

    #[test]
    fn the_player_ring_matches_the_spoint_counts() {
        let f = fixture();
        let ctx = context();
        let lattice = ctx.lattice(CHUNK_M, GRID);
        let player = player_world();
        let player_dir = v3(&f["thinning"]["playerDirection"]);
        let ring = &f["ring"];
        let rows = cases(ring, "chunks");
        let keys = lattice.ring_around_dir(player_dir, ring["radiusM"].as_f64().unwrap());
        assert_eq!(keys.len(), RING_CHUNKS);
        assert_eq!(rows.len(), RING_CHUNKS);
        let (mut placed, mut resident) = (0_u64, 0_u64);
        for (key, row) in keys.iter().zip(rows) {
            assert_eq!(*key as u64, row[0].as_u64().unwrap());
            let frame = ctx.chunk_frame(&lattice, *key as u64);
            let instances = place_tree_chunk(&ctx, *key as u64);
            let distances: Vec<f64> = instances.iter().map(|t| distance(frame.local_to_world(f64::from(t.pos[0]), f64::from(t.pos[1]), f64::from(t.pos[2])), player)).collect();
            let kept = thin_trees(&instances, &distances).len() as u64;
            assert_eq!((instances.len() as u64, kept), (row[1].as_u64().unwrap(), row[2].as_u64().unwrap()), "ring chunk {key}");
            placed += instances.len() as u64;
            resident += kept;
        }
        assert_eq!((placed, resident), (RING_PLACED, RING_RESIDENT));
        assert_eq!((ring["placed"].as_u64().unwrap(), ring["resident"].as_u64().unwrap()), (RING_PLACED, RING_RESIDENT));
    }
}
