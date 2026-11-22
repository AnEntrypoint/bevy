use std::collections::HashMap;

use crate::height::climate::{dir_to_face_uv, sample_uv, Climate};
use crate::height::height_spec::{HashVersion, HeightSpec, Params};
use crate::height::ops_jsgen::{Precision, V3};
use crate::height::placement::PlacementLattice;
use crate::height::planet_frame::{anchor_basis, Basis};
use crate::height::Planet;
use crate::planet_material::splat::{band_warp, splat_weights, SplatParams, SplatWeights};
use crate::vec3::{dot, normalise};

pub const SEA_REJECT: f64 = -2.0;
pub const ARIDITY_LINE: f64 = 0.28;
const SECTOR_M: f64 = 8.0;
const SECTOR_CELLS: f64 = 1.0;
const POLE_REF_SWITCH: f64 = 0.99;
const RELIEF_CALIBRATION_BASELINE: f64 = 0.01;
const RENDERED_BEACH_TOP_M: f64 = 15.0;
const RENDERED_SAND_ONLY_BELOW_M: f64 = RENDERED_BEACH_TOP_M * 0.3;
const NO_PARAMS: Params = Params { hpf_res: 8.0, land_bias: 0.0, beach_shelf_m: 0.0, relief_scale: 0.0 };

pub struct PlacementContext<'a> {
    pub planet: &'a Planet,
    pub seed: u32,
    pub radius: f64,
    pub relief_scale: f64,
    noise: HeightSpec<'static>,
    splat: SplatParams,
}

impl<'a> PlacementContext<'a> {
    pub fn new(planet: &'a Planet, seed: u32, radius: f64, hash: HashVersion) -> Self {
        let precision = match hash {
            HashVersion::Float => Precision::F32,
            HashVersion::Integer => Precision::F64,
        };
        let noise = HeightSpec::new(hash, precision, NO_PARAMS, Vec::new(), &[]).expect("a spec without carves always builds");
        Self { planet, seed, radius, relief_scale: planet.relief(radius), noise, splat: SplatParams::reference() }
    }

    pub fn lattice(&self, chunk_m: f64, cells_per_chunk: f64) -> PlacementLattice {
        PlacementLattice::new(self.radius, chunk_m, cells_per_chunk).expect("a placement lattice over a finite planet radius always builds")
    }

    pub fn chunk_frame(&self, lattice: &PlacementLattice, key: u64) -> ChunkFrame {
        ChunkFrame::at(self, lattice.chunk_centre_dir(key as f64))
    }

    pub fn surface_along_dir(&self, frame: &ChunkFrame, dir: V3) -> Option<Surface> {
        let [de, du, dn] = frame.chart_of(dir);
        let height = self.planet.height(normalise(dir), self.radius);
        let rho = self.radius + height;
        height.is_finite().then(|| Surface { rho, at: [rho * de, rho * du - frame.surface_base(), rho * dn] })
    }

    pub fn surface_rho_at(&self, frame: &ChunkFrame, tf: &TangentFrame, rho: f64, off_east: f64, off_north: f64) -> Option<f64> {
        let p = [0, 1, 2].map(|i| tf.up[i] * rho + tf.east[i] * off_east + tf.north[i] * off_north);
        self.surface_along_dir(frame, normalise(p)).map(|s| s.rho)
    }

    pub fn radial_slope(&self, frame: &ChunkFrame, tf: &TangentFrame, rho: f64, step_m: f64) -> Option<[f64; 2]> {
        let probe = |off_east: f64, off_north: f64| self.surface_rho_at(frame, tf, rho, off_east, off_north);
        let (east_hi, east_lo) = (probe(step_m, 0.0)?, probe(-step_m, 0.0)?);
        let (north_hi, north_lo) = (probe(0.0, step_m)?, probe(0.0, -step_m)?);
        Some([(east_hi - east_lo) / (2.0 * step_m), (north_hi - north_lo) / (2.0 * step_m)])
    }

    pub fn snoise3(&self, p: V3) -> f64 {
        self.noise.snoise3(p)
    }

    pub fn splat_params(&self) -> &SplatParams {
        &self.splat
    }

    pub fn chunk_cells(&self, layout: &ChunkLayout, key: u64) -> (ChunkFrame, Vec<PlacementCell>) {
        let lattice = self.lattice(layout.chunk_m, layout.grid);
        let dec = lattice.decode_chunk(key as f64);
        let jitter = CellJitter { prejitter_seed: self.seed ^ layout.seed_salt, over_cell: layout.jitter_m / layout.cell_m, k_x: layout.k_x, k_z: layout.k_z };
        let cells = (0..layout.grid as u32).flat_map(|gz| (0..layout.grid as u32).map(move |gx| (f64::from(gx), f64::from(gz))));
        let cells = cells.map(|(gx, gz)| placement_cell_at(&lattice, dec, gx, gz, &jitter)).collect();
        (self.chunk_frame(&lattice, key), cells)
    }

    pub fn painted_weights(&self, dir: V3, elev: f64, slope: [f64; 2], temp: f64, humidity: f64) -> SplatWeights {
        let warp = band_warp(|x, y, z| self.snoise3([x, y, z]), dir, self.splat.band_warp);
        splat_weights(&self.splat, warp, elev, painted_slope_of(slope), temp, humidity)
    }

    pub fn sector_climate(&self) -> SectorClimate {
        SectorClimate { lattice: self.lattice(SECTOR_M, SECTOR_CELLS), seed: i64::from(self.seed as i32), cache: HashMap::new() }
    }
}

pub struct ChunkFrame {
    pub basis: Basis,
    pub radius: f64,
    pub anchor_height: f64,
}

impl ChunkFrame {
    pub fn at(ctx: &PlacementContext, anchor_dir: V3) -> Self {
        let basis = anchor_basis(anchor_dir);
        let anchor_height = ctx.planet.height(basis.up, ctx.radius);
        Self { basis, radius: ctx.radius, anchor_height }
    }

    pub fn surface_base(&self) -> f64 {
        self.radius + self.anchor_height
    }

    pub fn chart_of(&self, v: V3) -> V3 {
        [dot(v, self.basis.east), dot(v, self.basis.up), dot(v, self.basis.north)]
    }

    pub fn local_to_world(&self, x: f64, y: f64, z: f64) -> V3 {
        let s = self.surface_base() + y;
        [0, 1, 2].map(|i| self.basis.up[i] * s + (self.basis.east[i] * x + self.basis.north[i] * z))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Surface {
    pub rho: f64,
    pub at: V3,
}

pub struct TangentFrame {
    pub east: V3,
    pub north: V3,
    pub up: V3,
    pub east_l: V3,
    pub up_l: V3,
    pub north_l: V3,
}

pub fn tangent_frame(frame: &ChunkFrame, d: V3) -> TangentFrame {
    let ref_is_y = d[1].abs() < POLE_REF_SWITCH;
    let raw = if ref_is_y { [d[2], 0.0, -d[0]] } else { [0.0, -d[2], d[1]] };
    let east = normalise(raw);
    let north = [east[1] * d[2] - east[2] * d[1], east[2] * d[0] - east[0] * d[2], east[0] * d[1] - east[1] * d[0]];
    TangentFrame { east, north, up: d, east_l: frame.chart_of(east), up_l: frame.chart_of(d), north_l: frame.chart_of(north) }
}

pub fn tangent_frame_quat(tf: &TangentFrame) -> [f64; 4] {
    let (m00, m10, m20) = (tf.east_l[0], tf.east_l[1], tf.east_l[2]);
    let (m01, m11, m21) = (tf.up_l[0], tf.up_l[1], tf.up_l[2]);
    let (m02, m12, m22) = (tf.north_l[0], tf.north_l[1], tf.north_l[2]);
    let trace = m00 + m11 + m22;
    if trace > 0.0 {
        let s = 0.5 / (trace + 1.0).sqrt();
        [(m21 - m12) * s, (m02 - m20) * s, (m10 - m01) * s, 0.25 / s]
    } else if m00 > m11 && m00 > m22 {
        let s = 2.0 * (1.0 + m00 - m11 - m22).sqrt();
        [0.25 * s, (m01 + m10) / s, (m02 + m20) / s, (m21 - m12) / s]
    } else if m11 > m22 {
        let s = 2.0 * (1.0 + m11 - m00 - m22).sqrt();
        [(m01 + m10) / s, 0.25 * s, (m12 + m21) / s, (m02 - m20) / s]
    } else {
        let s = 2.0 * (1.0 + m22 - m00 - m11).sqrt();
        [(m02 + m20) / s, (m12 + m21) / s, 0.25 * s, (m10 - m01) / s]
    }
}

pub fn tangent_to_local(tf: &TangentFrame, a: f64, b: f64, c: f64) -> V3 {
    [0, 1, 2].map(|i| tf.east_l[i] * a + tf.up_l[i] * b + tf.north_l[i] * c)
}

pub fn round_f32(v: f64) -> f64 {
    f64::from(v as f32)
}

pub fn quat_mul_f32(a: [f64; 4], b: [f64; 4]) -> [f32; 4] {
    [
        a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
        a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
        a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
        a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
    ]
    .map(|c| c as f32)
}

pub fn lean_quat(magnitude: f64, dir_angle: f64) -> [f64; 4] {
    let (s, c) = ((magnitude * 0.5).sin(), (magnitude * 0.5).cos());
    [round_f32(dir_angle.cos() * s), 0.0, round_f32(dir_angle.sin() * s), round_f32(c)]
}

pub fn painted_slope_of(slope: [f64; 2]) -> f64 {
    1.0 - 1.0 / (1.0 + slope[0] * slope[0] + slope[1] * slope[1]).sqrt()
}

pub fn hash3(seed: u32, ix: i32, iz: i32) -> u32 {
    let mut h = (seed ^ ix as u32).wrapping_mul(0x27d4eb2d);
    h = (h ^ iz as u32).wrapping_mul(0x165667b1);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a2d39);
    h ^ (h >> 15)
}

pub fn rand(h: u32, k: i32) -> f64 {
    let mut x = h ^ (k.wrapping_add(1) as u32).wrapping_mul(0x9e3779b1);
    x = (x ^ (x >> 16)).wrapping_mul(0x85ebca6b);
    x ^= x >> 13;
    f64::from(x) / 4_294_967_296.0
}

pub fn smoothstep(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn value_noise3(seed: u32, p: V3, cell_m: f64) -> f64 {
    let f = p.map(|c| c / cell_m);
    let i = f.map(|c| c.floor());
    let [tx, ty, tz] = [0, 1, 2].map(|k| {
        let t = f[k] - i[k];
        t * t * (3.0 - 2.0 * t)
    });
    let [ix, iy, iz] = i.map(|c| c as i64 as i32);
    let corner = |dx: i32, dy: i32, dz: i32| f64::from(hash3(hash3(seed, ix.wrapping_add(dx), iy.wrapping_add(dy)), iz.wrapping_add(dz), 0)) / 4_294_967_296.0;
    let lerp = |a: f64, b: f64, t: f64| a + (b - a) * t;
    let x00 = lerp(corner(0, 0, 0), corner(1, 0, 0), tx);
    let x10 = lerp(corner(0, 1, 0), corner(1, 1, 0), tx);
    let x01 = lerp(corner(0, 0, 1), corner(1, 0, 1), tx);
    let x11 = lerp(corner(0, 1, 1), corner(1, 1, 1), tx);
    let y0 = lerp(x00, x10, ty);
    let y1 = lerp(x01, x11, ty);
    lerp(y0, y1, tz)
}

pub fn is_seabed(sea_bias: f64) -> bool {
    sea_bias < SEA_REJECT
}

pub fn relief_margin_scale(relief_scale: f64) -> f64 {
    relief_scale / RELIEF_CALIBRATION_BASELINE
}

pub fn rendered_soil_weight(elev_above_sea: f64) -> f64 {
    let t = (elev_above_sea - RENDERED_SAND_ONLY_BELOW_M) / (RENDERED_BEACH_TOP_M - RENDERED_SAND_ONLY_BELOW_M);
    if t.is_nan() || t <= 0.0 {
        0.0
    } else if t >= 1.0 {
        1.0
    } else {
        t * t * (3.0 - 2.0 * t)
    }
}

pub struct SectorClimate {
    lattice: PlacementLattice,
    seed: i64,
    cache: HashMap<u64, Climate>,
}

impl SectorClimate {
    pub fn at_dir(&mut self, dir: V3) -> Climate {
        let key = self.lattice.chunk_key_of_dir(dir);
        let (lattice, seed) = (&self.lattice, self.seed);
        *self.cache.entry(key as u64).or_insert_with(|| {
            let (face, fu, fv) = dir_to_face_uv(lattice.chunk_centre_dir(key));
            sample_uv(seed, face, fu, fv, None)
        })
    }
}

pub struct PlacementCell {
    pub dir: V3,
    pub sea: V3,
    pub row: i32,
    pub j: i32,
    pub id: f64,
    pub area: f64,
}

pub struct ChunkLayout {
    pub chunk_m: f64,
    pub cell_m: f64,
    pub grid: f64,
    pub jitter_m: f64,
    pub seed_salt: u32,
    pub k_x: i32,
    pub k_z: i32,
}

pub struct CellJitter {
    pub prejitter_seed: u32,
    pub over_cell: f64,
    pub k_x: i32,
    pub k_z: i32,
}

pub fn placement_cell_at(lattice: &PlacementLattice, dec: [f64; 3], gx: f64, gz: f64, jitter: &CellJitter) -> PlacementCell {
    let i = dec[1] * lattice.cells_per_chunk + gx;
    let j = dec[2] * lattice.cells_per_chunk + gz;
    let row = lattice.hash_row(dec[0], i);
    let (row_bits, j_bits) = (row as i64 as i32, j as i64 as i32);
    let h = hash3(jitter.prejitter_seed, row_bits, j_bits);
    let offset = |k: i32| (rand(h, k) * 2.0 - 1.0) * jitter.over_cell;
    let dir = lattice.cell_dir(dec[0], i + 0.5 + offset(jitter.k_x), j + 0.5 + offset(jitter.k_z));
    PlacementCell {
        dir,
        sea: dir.map(|c| c * lattice.radius),
        row: row_bits,
        j: j_bits,
        id: lattice.cell_id(dec[0], i, j),
        area: lattice.cell_area_over_target(i + 0.5, j + 0.5),
    }
}

#[cfg(test)]
pub mod fixture {
    use super::PlacementContext;
    use crate::height::{HashVersion, Planet, PlanetConfig};
    use std::sync::OnceLock;

    pub const SEED: u32 = 1337;
    pub const RADIUS: f64 = 63_600.0;
    pub const RELIEF_SCALE: f64 = 0.001;

    pub fn planet() -> &'static Planet {
        static PLANET: OnceLock<Planet> = OnceLock::new();
        PLANET.get_or_init(|| {
            Planet::new(&PlanetConfig { seed: SEED as i32, hpf_res: 128, hash: HashVersion::Integer, relief_scale: Some(RELIEF_SCALE), anchor_dir: [0.0, 1.0, 0.0], radius: RADIUS, carves: Vec::new() })
        })
    }

    pub fn context() -> PlacementContext<'static> {
        PlacementContext::new(planet(), SEED, RADIUS, HashVersion::Integer)
    }
}
