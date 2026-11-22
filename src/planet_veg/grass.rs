use std::f64::consts::PI;

use super::context::{
    hash3, is_seabed, painted_slope_of, rand, rendered_soil_weight, smoothstep, tangent_frame, tangent_to_local, ChunkFrame, ChunkLayout, PlacementCell, PlacementContext,
    SectorClimate, TangentFrame,
};
use crate::height::ops_jsgen::V3;
use crate::planet_lod::guard;
use crate::planet_material::splat::SplatWeights;
use crate::vec3::{dot, length, normalise};

pub const CHUNK_M: f64 = 32.0;
pub const CELL_M: f64 = 2.0;
pub const GRID: f64 = 16.0;
pub const JITTER_M: f64 = 0.9;
pub const SLOPE_D: f64 = 1.0;
pub const SLOPE_MAX: f64 = 0.9;
pub const SCALE_MIN: f64 = 0.462;
pub const SCALE_SPAN: f64 = 3.213;
pub const BLADES_PER_CELL: i32 = 79;
pub const CLUMP_R: f64 = 1.15;
pub const FULL_R_M: f64 = 16.0;
pub const FALLOFF_EXPONENT: f64 = 1.25;
pub const FADE_M: f64 = 5.0;
pub const MIN_Q: f64 = 1e-4;

const K_JIT_X: i32 = 20;
const K_JIT_Z: i32 = 21;
const K_COIN: i32 = 22;
const K_SCALE: i32 = 23;
const K_YAW: i32 = 24;
const K_TINT: i32 = 25;
const K_WIND: i32 = 26;

const SEED_SALT: u32 = 0x6a55;
const CELL_HASH_SEED: u32 = 0x6a55;
const SUN_DIR_RAW: V3 = [0.4, 0.8, 0.3];
const SHADOW_FLOOR: f64 = 0.55;
const SHADOW_SPAN: f64 = 0.45;

const DENSITY_BASE: f64 = 0.5;
const DENSITY_WET_GAIN: f64 = 0.5;
const DENSITY_WARM_FLOOR: f64 = 0.45;
const DENSITY_WARM_SPAN: f64 = 0.55;
const DENSITY_FLAT_FLOOR: f64 = 0.35;
const DENSITY_FLAT_SPAN: f64 = 0.65;
const DENSITY_DRY_EDGE: f64 = 0.05;
const DENSITY_DRY_SPAN: f64 = 0.15;
const DENSITY_CONTRAST: f64 = 4.0;

const BC_SHORE: V3 = [0.82, 0.76, 0.52];
const BC_LOWLAND: V3 = [0.18, 0.38, 0.10];
const BC_GRASS: V3 = [0.30, 0.46, 0.12];
const BC_ROCK: V3 = [0.48, 0.40, 0.30];
const BC_SNOW: V3 = [0.94, 0.96, 1.00];
const BAND_EDGES_LO: [f64; 2] = [8.0, 20.0];
const BAND_EDGES_HI: [f64; 2] = [60.0, 100.0];
const TEX_BRIGHT: f64 = 1.12;
const VARIATION_AMT: f64 = 0.05;
const BIOME_TINT: f64 = 0.62;
const SAND_TINT_RELIEF: f64 = 0.85;
const BEACH_FADE_START: f64 = 0.3;
const WARP_FREQ: f64 = 130.0;
const WARP_AMP: f64 = 0.004;
const BAND_WARP_FREQS: [f64; 3] = [210.0, 560.0, 1450.0];
const BAND_WARP_GAINS: [f64; 3] = [1.0, 0.5, 0.25];
const MOTTLE_FREQ: f64 = 120.0;
const RIDGE_OCTAVES: [[f64; 3]; 2] = [[75.0, 1.0, 0.0], [375.0, 0.6, 7.3]];
const RIDGE_AMP_SUM: f64 = 1.6;
const RIDGE_GAIN: f64 = 0.02;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrassBlade {
    pub q: f32,
    pub pos: [f32; 3],
    pub scale: f32,
    pub yaw: f32,
    pub normal: [f32; 3],
    pub tint: f32,
    pub wind_phase: f32,
    pub shadow: f32,
    pub ground: [f32; 3],
}

pub fn grass_blade_edge_m(q: f32) -> f32 {
    (FULL_R_M / f64::from(q).max(MIN_Q).powf(1.0 / FALLOFF_EXPONENT) + FADE_M) as f32
}

pub fn grass_density(temp: f64, humidity: f64, slope_ratio: f64) -> f64 {
    let wet = humidity.clamp(0.0, 1.0);
    let warm = temp.clamp(0.0, 1.0);
    let flat = 1.0 - slope_ratio.clamp(0.0, 1.0);
    let base = (DENSITY_BASE + DENSITY_WET_GAIN * wet * (DENSITY_WARM_FLOOR + DENSITY_WARM_SPAN * warm)) * (DENSITY_FLAT_FLOOR + DENSITY_FLAT_SPAN * flat);
    let dry = ((wet - DENSITY_DRY_EDGE) / DENSITY_DRY_SPAN).clamp(0.0, 1.0);
    let biome = dry * dry * (3.0 - 2.0 * dry);
    (DENSITY_BASE + DENSITY_CONTRAST * (base * biome - DENSITY_BASE)).clamp(0.0, 1.0)
}

fn mix3(a: V3, b: V3, t: f64) -> V3 {
    [0, 1, 2].map(|c| a[c] + (b[c] - a[c]) * t)
}

fn land_biome_rgb(ctx: &PlacementContext, dir: V3, h: f64, rock_slope: f64) -> V3 {
    let params = ctx.splat_params();
    let at = |p: V3, freq: f64| ctx.snoise3(p.map(|c| c * freq));
    let rock_t = smoothstep(params.slope_rock[0], params.slope_rock[1], rock_slope);
    let lowland = mix3(mix3(BC_SHORE, BC_LOWLAND, smoothstep(0.0, BAND_EDGES_LO[0], h)), BC_GRASS, smoothstep(BAND_EDGES_LO[0], BAND_EDGES_LO[1], h));
    let warp = at(dir, WARP_FREQ) * WARP_AMP;
    let warped = dir.map(|c| c + warp);
    let band_warp = (0..3).map(|k| BAND_WARP_GAINS[k] * at(warped, BAND_WARP_FREQS[k])).sum::<f64>() * params.band_warp;
    let banded = mix3(lowland, BC_ROCK, smoothstep(band_warp + BAND_EDGES_HI[0], band_warp + BAND_EDGES_HI[1], h));
    let snowy = mix3(banded, BC_SNOW, smoothstep(band_warp + params.snow_edges[0], band_warp + params.snow_edges[1], h));
    let steep = mix3(snowy, BC_ROCK, rock_t);
    let mottle = at(dir, MOTTLE_FREQ) * VARIATION_AMT + 1.0;
    let beach = (1.0 - smoothstep(params.beach_top * BEACH_FADE_START, params.beach_top, h)) * (1.0 - rock_t);
    let beached = mix3(steep.map(|c| c * mottle), BC_SHORE, beach);
    let ridge: f64 = RIDGE_OCTAVES.iter().map(|[freq, amp, shift]| amp * ctx.snoise3(dir.map(|c| c * freq + shift))).sum();
    let ridge_gain = ridge / RIDGE_AMP_SUM * RIDGE_GAIN + 1.0;
    beached.map(|c| c * ridge_gain)
}

fn ground_rgb(ctx: &PlacementContext, dir: V3, h: f64, rock_slope: f64, splat: &SplatWeights) -> V3 {
    let biome = land_biome_rgb(ctx, dir, h, rock_slope);
    let surface = [0, 1, 2].map(|c| splat.grass * BC_GRASS[c] + splat.rock * BC_ROCK[c] + splat.sand * BC_SHORE[c] + splat.snow * BC_SNOW[c]);
    let tinted = mix3(surface, biome, BIOME_TINT * (1.0 - SAND_TINT_RELIEF * splat.sand.clamp(0.0, 1.0)));
    tinted.map(|c| (c * TEX_BRIGHT).clamp(0.0, 1.0))
}

struct Site {
    at: V3,
    cell_hash: u32,
    density: f64,
    ground: [f32; 3],
    shadow: f32,
    normal: [f32; 3],
    tf: TangentFrame,
    heading: f64,
    surface_rho: f64,
}

struct Classifier<'a> {
    ctx: &'a PlacementContext<'a>,
    frame: &'a ChunkFrame,
    climate: SectorClimate,
}

impl Classifier<'_> {
    fn classify(&mut self, cell: &PlacementCell) -> Option<Site> {
        let cell_hash = hash3(CELL_HASH_SEED, cell.row, cell.j);
        let surface = self.ctx.surface_along_dir(self.frame, cell.dir)?;
        let climate = self.climate.at_dir(cell.dir);
        if is_seabed(climate.sea_bias) {
            return None;
        }
        let elev = surface.rho - self.ctx.radius;
        let soil = rendered_soil_weight(elev);
        if soil <= 0.0 {
            return None;
        }
        let tf = tangent_frame(self.frame, cell.dir);
        let rho = surface.rho;
        let slope = self.ctx.radial_slope(self.frame, &tf, rho, SLOPE_D)?;
        let grad = slope[0].hypot(slope[1]);
        if grad > SLOPE_MAX {
            return None;
        }
        let slope_ratio = grad / (grad + 1.0);
        let splat = self.ctx.painted_weights(cell.dir, elev, slope, climate.temp, climate.humidity);
        let density = (grass_density(climate.temp, climate.humidity, slope_ratio) * soil * splat.grass * cell.area).clamp(0.0, 1.0);
        if density <= 0.0 {
            return None;
        }
        let ground = ground_rgb(self.ctx, cell.dir, elev, painted_slope_of(slope), &splat).map(|c| c as f32);
        let slope_normal = [-slope[0], 1.0, -slope[1]];
        let ndl = dot(slope_normal, normalise(SUN_DIR_RAW)) / length(slope_normal);
        let shadow = (SHADOW_FLOOR + SHADOW_SPAN * ndl).clamp(0.0, 1.0) as f32;
        let surf_n = [0, 1, 2].map(|i| tf.up_l[i] - slope[0] * tf.east_l[i] - slope[1] * tf.north_l[i]);
        let surf_inv = 1.0 / length(surf_n);
        Some(Site {
            at: surface.at,
            cell_hash,
            density,
            ground,
            shadow,
            normal: surf_n.map(|c| (c * surf_inv) as f32),
            heading: (-tf.east_l[2]).atan2(tf.east_l[0]),
            tf,
            surface_rho: rho,
        })
    }

    fn rise_at(&self, site: &Site, off_east: f64, off_north: f64) -> f64 {
        self.ctx.surface_rho_at(self.frame, &site.tf, site.surface_rho, off_east, off_north).map_or(0.0, |rho| rho - site.surface_rho)
    }

    fn blade(&self, site: &Site, index: i32) -> Option<GrassBlade> {
        let h = hash3(site.cell_hash, index + 1, 0);
        let coin = rand(h, K_COIN);
        if coin >= site.density {
            return None;
        }
        let ang = rand(h, K_YAW) * PI * 2.0;
        let rad = rand(h, K_JIT_X).sqrt() * CLUMP_R;
        let (off_east, off_north) = (ang.cos() * rad, ang.sin() * rad);
        let off = tangent_to_local(&site.tf, off_east, self.rise_at(site, off_east, off_north), off_north);
        Some(GrassBlade {
            q: (coin / site.density) as f32,
            pos: [0, 1, 2].map(|i| (site.at[i] + off[i]) as f32),
            scale: (SCALE_MIN + rand(h, K_SCALE) * SCALE_SPAN) as f32,
            yaw: (ang + site.heading) as f32,
            normal: site.normal,
            tint: rand(h, K_TINT) as f32,
            wind_phase: (rand(h, K_WIND) * PI * 2.0) as f32,
            shadow: site.shadow,
            ground: site.ground,
        })
    }
}

const LAYOUT: ChunkLayout = ChunkLayout { chunk_m: CHUNK_M, cell_m: CELL_M, grid: GRID, jitter_m: JITTER_M, seed_salt: SEED_SALT, k_x: K_JIT_X, k_z: K_JIT_Z };

pub fn place_grass_chunk(ctx: &PlacementContext, key: u64) -> Vec<GrassBlade> {
    guard::note_build();
    let (frame, cells) = ctx.chunk_cells(&LAYOUT, key);
    let mut classifier = Classifier { ctx, frame: &frame, climate: ctx.sector_climate() };
    let mut blades = Vec::new();
    for cell in &cells {
        if let Some(site) = classifier.classify(cell) {
            blades.extend((0..BLADES_PER_CELL).filter_map(|b| classifier.blade(&site, b)));
        }
    }
    blades
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
    const MEAN_EDGE_TOLERANCE_M: f64 = 1e-6;
    const EXACT: f64 = 1e-12;
    const FIXTURE_CHUNKS: usize = 34;
    const RING_CHUNKS: usize = 32;
    const RING_PLACED: u64 = 182552;
    const ANCHOR_BLADES: usize = 6492;
    const UPLAND_BLADES: usize = 2532;

    fn fixture() -> &'static Value {
        static FIXTURE: OnceLock<Value> = OnceLock::new();
        FIXTURE.get_or_init(|| parse(include_str!("../../testdata/parity/spoint-planet-grass-placement.json")))
    }

    fn row(blade: &GrassBlade) -> [f32; 15] {
        let [x, y, z] = blade.pos;
        let [nx, ny, nz] = blade.normal;
        let [r, g, b] = blade.ground;
        [blade.q, x, y, z, blade.scale, blade.yaw, nx, ny, nz, blade.tint, blade.wind_phase, blade.shadow, r, g, b]
    }

    fn tolerance_of(field: usize) -> f64 {
        match field {
            1..=3 => POSITION_TOLERANCE_M,
            _ => F32_FIELD_TOLERANCE,
        }
    }

    fn compare_blades(got: &[GrassBlade], want: &Value, key: u64, label: &str) {
        let ctx = context();
        let frame = ctx.chunk_frame(&ctx.lattice(CHUNK_M, GRID), key);
        let want = want.as_array().unwrap();
        assert_eq!(got.len(), want.len(), "{label} blade count");
        for (i, (blade, w)) in got.iter().zip(want).enumerate() {
            let w = w.as_array().unwrap();
            assert_eq!(w.len(), 18, "{label} fields");
            for (field, (g, v)) in row(blade).iter().zip(w).enumerate() {
                close(f64::from(*g), v.as_f64().unwrap(), tolerance_of(field), &format!("{label} blade {i} field {field}"));
            }
            let world = frame.local_to_world(f64::from(blade.pos[0]), f64::from(blade.pos[1]), f64::from(blade.pos[2]));
            for (axis, g) in world.iter().enumerate() {
                close(*g, w[15 + axis].as_f64().unwrap(), WORLD_TOLERANCE_M, &format!("{label} blade {i} world[{axis}]"));
            }
        }
    }

    #[test]
    fn constants_match_the_fixture() {
        let f = fixture();
        let grass = &f["grass"];
        let got = [CHUNK_M, CELL_M, GRID, JITTER_M, SLOPE_D, SLOPE_MAX, SEA_REJECT, SCALE_MIN, SCALE_SPAN, f64::from(BLADES_PER_CELL), CLUMP_R];
        let want = ["CHUNK", "CELL", "GRID", "JITTER", "SLOPE_D", "SLOPE_MAX", "SEA_REJECT", "SCALE_MIN", "SCALE_SPAN", "BLADES_PER_CELL", "CLUMP_R"];
        assert_eq!(grass.as_object().unwrap().len(), want.len());
        for (g, name) in got.iter().zip(want) {
            close(*g, grass[name].as_f64().unwrap(), EXACT, name);
        }
        let density = &f["density"];
        close(FULL_R_M, density["FULL_R"].as_f64().unwrap(), EXACT, "full radius");
        close(FALLOFF_EXPONENT, density["P"].as_f64().unwrap(), EXACT, "falloff exponent");
        close(FADE_M, density["FADE_M"].as_f64().unwrap(), EXACT, "fade");
        let keys = [("K_JITX", K_JIT_X), ("K_JITZ", K_JIT_Z), ("K_COIN", K_COIN), ("K_SCALE", K_SCALE), ("K_YAW", K_YAW), ("K_TINT", K_TINT), ("K_WIND", K_WIND)];
        let table = f["keyTable"].as_object().unwrap();
        assert_eq!(table.len(), keys.len());
        for (name, k) in keys {
            assert_eq!(table[name].as_i64().unwrap(), i64::from(k), "{name}");
        }
        close(f["grassConfig"]["grassRenderDistance"].as_f64().unwrap(), 88.0, EXACT, "render distance");
        close(f["planet"]["radius"].as_f64().unwrap(), RADIUS, EXACT, "radius");
        close(f["planet"]["reliefScale"].as_f64().unwrap(), RELIEF_SCALE, EXACT, "relief scale");
        assert_eq!(f["planet"]["seed"].as_u64().unwrap(), u64::from(SEED));
    }

    #[test]
    fn blade_edges_follow_the_full_radius_and_falloff_exponent() {
        for q in [0.0_f32, 0.05, 0.2, 0.5, 0.9, 1.0] {
            let want = 16.0 / f64::from(q).max(1e-4).powf(0.8) + 5.0;
            close(f64::from(grass_blade_edge_m(q)), want, want * 1e-7, &format!("edge of q {q}"));
        }
    }

    #[test]
    fn placed_counts_and_mean_edges_match_the_spoint_chunks() {
        let f = fixture();
        let ctx = context();
        let chunks = cases(f, "chunks");
        assert_eq!(chunks.len(), FIXTURE_CHUNKS);
        assert_eq!(f["chunkCount"].as_u64().unwrap() as usize, FIXTURE_CHUNKS);
        let mut with_blades = 0;
        for chunk in chunks {
            let key = chunk["key"].as_u64().unwrap();
            let label = format!("{} chunk {key}", chunk["role"].as_str().unwrap());
            let got = place_grass_chunk(&ctx, key);
            assert_eq!(got.len(), chunk["placedCount"].as_u64().unwrap() as usize, "{label} placed count");
            let mean = if got.is_empty() { 0.0 } else { got.iter().map(|b| f64::from(grass_blade_edge_m(b.q))).sum::<f64>() / got.len() as f64 };
            close(mean, chunk["meanEdgeM"].as_f64().unwrap(), MEAN_EDGE_TOLERANCE_M, &format!("{label} mean edge"));
            with_blades += usize::from(!got.is_empty());
        }
        assert_eq!(with_blades, 24);
    }

    #[test]
    fn the_anchor_and_upland_chunks_match_blade_for_blade() {
        let f = fixture();
        let ctx = context();
        let mut compared = 0;
        for chunk in cases(f, "chunks").iter().filter(|c| !c["blades"].is_null()) {
            let key = chunk["key"].as_u64().unwrap();
            let label = format!("{} chunk {key}", chunk["role"].as_str().unwrap());
            let got = place_grass_chunk(&ctx, key);
            compare_blades(&got, &chunk["blades"], key, &label);
            compared += got.len();
        }
        assert_eq!(compared, ANCHOR_BLADES + UPLAND_BLADES);
    }

    #[test]
    fn the_88_m_ring_places_the_reference_count_in_every_chunk() {
        let ring = &fixture()["ring"];
        let ctx = context();
        let chunks = cases(ring, "chunks");
        assert_eq!(chunks.len(), RING_CHUNKS);
        let mut placed = 0;
        for entry in chunks {
            let key = entry[0].as_u64().unwrap();
            let got = place_grass_chunk(&ctx, key).len() as u64;
            assert_eq!(got, entry[1].as_u64().unwrap(), "ring chunk {key}");
            placed += got;
        }
        assert_eq!(placed, ring["placed"].as_u64().unwrap());
        assert_eq!(placed, RING_PLACED);
    }

    const REFERENCE_GRASS_DENSITY: [[f64; 4]; 45] = [
        [0.0, 0.02, 0.0, 0.0],
        [0.0, 0.02, 0.3, 0.0],
        [0.0, 0.02, 0.47, 0.0],
        [0.0, 0.1, 0.0, 0.0],
        [0.0, 0.1, 0.3, 0.0],
        [0.0, 0.1, 0.47, 0.0],
        [0.0, 0.18, 0.0, 0.556942814814815],
        [0.0, 0.18, 0.3, 0.15583896592592583],
        [0.0, 0.18, 0.47, 0.0],
        [0.0, 0.3, 0.0, 0.77],
        [0.0, 0.3, 0.3, 0.3273499999999998],
        [0.0, 0.3, 0.47, 0.07651500000000011],
        [0.0, 0.8, 0.0, 1.0],
        [0.0, 0.8, 0.3, 0.6896],
        [0.0, 0.8, 0.47, 0.38904000000000005],
        [0.5, 0.02, 0.0, 0.0],
        [0.5, 0.02, 0.3, 0.0],
        [0.5, 0.02, 0.47, 0.0],
        [0.5, 0.1, 0.0, 0.0],
        [0.5, 0.1, 0.3, 0.0],
        [0.5, 0.1, 0.47, 0.0],
        [0.5, 0.18, 0.0, 0.6511321481481485],
        [0.5, 0.18, 0.3, 0.23166137925925923],
        [0.5, 0.18, 0.47, 0.0],
        [0.5, 0.3, 0.0, 0.935],
        [0.5, 0.3, 0.3, 0.460175],
        [0.5, 0.3, 0.47, 0.19110749999999999],
        [0.5, 0.8, 0.0, 1.0],
        [0.5, 0.8, 0.3, 1.0],
        [0.5, 0.8, 0.47, 0.69462],
        [1.0, 0.02, 0.0, 0.0],
        [1.0, 0.02, 0.3, 0.0],
        [1.0, 0.02, 0.47, 0.0],
        [1.0, 0.1, 0.0, 0.0],
        [1.0, 0.1, 0.3, 0.0],
        [1.0, 0.1, 0.47, 0.0],
        [1.0, 0.18, 0.0, 0.7453214814814815],
        [1.0, 0.18, 0.3, 0.3074837925925924],
        [1.0, 0.18, 0.47, 0.05937576888888896],
        [1.0, 0.3, 0.0, 1.0],
        [1.0, 0.3, 0.3, 0.593],
        [1.0, 0.3, 0.47, 0.3057000000000001],
        [1.0, 0.8, 0.0, 1.0],
        [1.0, 0.8, 0.3, 1.0],
        [1.0, 0.8, 0.47, 1.0],
    ];

    #[test]
    fn grass_density_matches_the_values_the_reference_function_returns() {
        for [temp, humidity, slope_ratio, want] in REFERENCE_GRASS_DENSITY {
            close(grass_density(temp, humidity, slope_ratio), want, EXACT, &format!("temp {temp} humidity {humidity} slope {slope_ratio}"));
        }
    }
}
