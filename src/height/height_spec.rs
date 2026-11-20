use super::ops_jsgen::{Ops, Precision, V3, u32_of};

pub const MAX_TERRAIN_CARVES: usize = 4;
const LATTICE_MUL: [u32; 3] = [0x8da6_b343, 0xd816_3841, 0xcb1a_b31f];
const AVALANCHE_MUL: [u32; 2] = [0x7feb_352d, 0x846c_a68b];
const UNIT_24BIT: f64 = 1.0 / 16_777_216.0;
const OCTAVE_ROTATION_STEP: f64 = 0.5236;
const OCTAVE_ROTATION_TABLE_SIZE: usize = 32;
const FRACTAL_HEIGHT_M: f64 = 750_000.0;
const CONTINENTAL_BIAS_AMP: f64 = 50.0;
const UNDERWATER_GAIN: f64 = 1.25;
const UNDERWATER_FLOOR_M: f64 = -350_000.0;
const DEFAULT_BEACH_SHELF_M: f64 = 150.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HashVersion {
    Float,
    Integer,
}

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub hpf_res: f64,
    pub land_bias: f64,
    pub beach_shelf_m: f64,
    pub relief_scale: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Carve {
    pub dir: V3,
    pub inner_chord2: f64,
    pub outer_chord2: f64,
    pub target_h: f64,
}

pub struct HeightSpec<'a> {
    ops: Ops,
    hash: HashVersion,
    params: Params,
    carves: Vec<Carve>,
    hpf: &'a [Vec<f32>],
    rotation_cos: [f64; OCTAVE_ROTATION_TABLE_SIZE],
    rotation_sin: [f64; OCTAVE_ROTATION_TABLE_SIZE],
}

impl<'a> HeightSpec<'a> {
    pub fn new(hash: HashVersion, precision: Precision, params: Params, carves: Vec<Carve>, hpf: &'a [Vec<f32>]) -> Result<Self, String> {
        if carves.len() > MAX_TERRAIN_CARVES {
            return Err(format!("terrain carveCount must be an integer 0..{MAX_TERRAIN_CARVES}, got {}", carves.len()));
        }
        if !carves.is_empty() && hash == HashVersion::Float {
            return Err("terrain carves need hashVersion 2".into());
        }
        let rotation_cos = std::array::from_fn(|i| ((i as f64) * OCTAVE_ROTATION_STEP).cos() as f32 as f64);
        let rotation_sin = std::array::from_fn(|i| ((i as f64) * OCTAVE_ROTATION_STEP).sin() as f32 as f64);
        Ok(Self { ops: Ops::new(precision), hash, params, carves, hpf, rotation_cos, rotation_sin })
    }

    pub fn compose(&self, dir: V3) -> f64 {
        self.apply_relief_scale(self.carve_terrain(dir, self.natural_height(dir)))
    }

    pub fn snoise3(&self, p: V3) -> f64 {
        let o = self.ops;
        let fl = p.map(|c| o.floor(c));
        let f = [0, 1, 2].map(|i| o.sub(p[i], fl[i]));
        let u = f.map(|c| self.quintic(c));
        let i1 = fl.map(|c| o.add(c, o.lit(1.0)));
        let n000 = self.h3(fl);
        let n100 = self.h3([i1[0], fl[1], fl[2]]);
        let n010 = self.h3([fl[0], i1[1], fl[2]]);
        let n110 = self.h3([i1[0], i1[1], fl[2]]);
        let n001 = self.h3([fl[0], fl[1], i1[2]]);
        let n101 = self.h3([i1[0], fl[1], i1[2]]);
        let n011 = self.h3([fl[0], i1[1], i1[2]]);
        let n111 = self.h3(i1);
        let x00 = o.mix(n000, n100, u[0]);
        let x10 = o.mix(n010, n110, u[0]);
        let x01 = o.mix(n001, n101, u[0]);
        let x11 = o.mix(n011, n111, u[0]);
        o.mix(o.mix(x00, x10, u[1]), o.mix(x01, x11, u[1]), u[2])
    }

    fn h3(&self, p: V3) -> f64 {
        match self.hash {
            HashVersion::Float => self.h3_float(p),
            HashVersion::Integer => self.h3_integer(p),
        }
    }

    fn h3_float(&self, p: V3) -> f64 {
        let o = self.ops;
        let scale = [o.lit(0.1031), o.lit(0.1030), o.lit(0.0973)];
        let q0 = [0, 1, 2].map(|i| o.fract(o.mul(p[i], scale[i])));
        let shifted = [q0[1], q0[0], q0[2]].map(|c| o.add(c, o.lit(33.33)));
        let d = o.dot(q0, shifted);
        let q = q0.map(|c| o.add(c, d));
        let f = o.fract(o.mul(o.add(q[0], q[1]), q[2]));
        o.sub(o.mul(f, o.lit(2.0)), o.lit(1.0))
    }

    fn h3_integer(&self, p: V3) -> f64 {
        let o = self.ops;
        let (x, y, z) = (u32_of(p[0]), u32_of(p[1]), u32_of(p[2]));
        let mixed = x.wrapping_mul(LATTICE_MUL[0]) ^ y.wrapping_mul(LATTICE_MUL[1]) ^ z.wrapping_mul(LATTICE_MUL[2]);
        let a = (mixed ^ (mixed >> 16)).wrapping_mul(AVALANCHE_MUL[0]);
        let b = (a ^ (a >> 15)).wrapping_mul(AVALANCHE_MUL[1]);
        let h = b ^ (b >> 16);
        o.sub(o.mul(o.mul(o.ufloat(f64::from(h >> 8)), o.lit(UNIT_24BIT)), o.lit(2.0)), o.lit(1.0))
    }

    fn quintic(&self, t: f64) -> f64 {
        let o = self.ops;
        o.mul(o.mul(o.mul(t, t), t), o.add(o.mul(t, o.sub(o.mul(t, o.lit(6.0)), o.lit(15.0))), o.lit(10.0)))
    }

    fn octave_rotation(&self, i: usize) -> (f64, f64) {
        match self.hash {
            HashVersion::Integer => (self.rotation_cos[i], self.rotation_sin[i]),
            HashVersion::Float => {
                let angle = self.ops.mul(i as f64, self.ops.lit(OCTAVE_ROTATION_STEP));
                (self.ops.cos(angle), self.ops.sin(angle))
            }
        }
    }

    fn rotate_domain(&self, p: V3, i: usize) -> V3 {
        let o = self.ops;
        let (c, s) = self.octave_rotation(i);
        [o.sub(o.mul(c, p[0]), o.mul(s, p[2])), p[1], o.add(o.mul(s, p[0]), o.mul(c, p[2]))]
    }

    fn value_fbm(&self, x: V3, gain: f64, octaves: usize) -> f64 {
        let o = self.ops;
        let gain = o.lit(gain);
        let (mut v, mut a, mut n, mut p) = (0.0, 1.0, 0.0, x);
        for _ in 0..octaves {
            let (next_v, next_a, next_n, next_p) = (
                o.add(v, o.mul(a, self.snoise3(p))),
                o.mul(a, gain),
                o.add(n, a),
                p.map(|c| o.mul(c, o.lit(2.0))),
            );
            (v, a, n, p) = (next_v, next_a, next_n, next_p);
        }
        o.div(v, n)
    }

    fn value_ridged(&self, x: V3, gain: f64, octaves: usize, offset: f64, exponent: f64) -> f64 {
        let o = self.ops;
        let (gain, offset, exponent) = (o.lit(gain), o.lit(offset), o.lit(exponent));
        let (mut v, mut w, mut n, mut a, mut p) = (0.0, 1.0, 0.0, 1.0, x);
        for i in 0..octaves {
            let signal = o.pow(o.max(o.sub(offset, o.abs(self.snoise3(p))), o.lit(0.0)), exponent);
            let (next_v, next_w, next_n, next_a, next_p) = (
                o.add(v, o.mul(o.mul(signal, w), a)),
                o.clamp(signal, o.lit(0.0), o.lit(1.0)),
                o.add(n, a),
                o.mul(a, gain),
                self.rotate_domain(p.map(|c| o.mul(c, o.lit(2.0))), i),
            );
            (v, w, n, a, p) = (next_v, next_w, next_n, next_a, next_p);
        }
        o.div(v, o.max(n, o.lit(1e-5)))
    }

    fn layer_fbm(&self, p: V3) -> f64 {
        let o = self.ops;
        o.add(o.lit(-2.0), o.mul(o.lit(4.0), o.add(o.mul(self.value_fbm(p, 0.5, 18), o.lit(0.5)), o.lit(0.5))))
    }

    fn layer_ridged(&self, p: V3) -> f64 {
        let o = self.ops;
        o.add(o.lit(-2.0), o.mul(o.lit(4.0), self.value_ridged(p, 0.5, 18, 1.064, 1.1)))
    }

    fn fractal_sample(&self, p: V3) -> f64 {
        let o = self.ops;
        let h0 = self.value_ridged(p, 0.5, 10, 1.064, 1.005);
        let warped = p.map(|c| o.add(c, o.mul(o.mul(c, o.lit(1.6)), h0)));
        o.div(o.add(o.add(h0, self.layer_fbm(warped)), self.layer_ridged(warped)), o.lit(3.0))
    }

    fn fractal_terrain_h(&self, dir: V3) -> f64 {
        let o = self.ops;
        let dir_n = o.normalize(dir);
        let p = dir_n.map(|c| o.mul(c, o.lit(3.0)));
        let h = o.mul(o.sub(self.fractal_sample(p), o.lit(0.17)), o.lit(0.6));
        let mix_arg = [0, 1, 2].map(|i| o.add(o.mul(p[i], o.lit(0.53)), o.lit([123.0, 456.0, 789.0][i])));
        let pmix = o.add(o.mul(self.snoise3(mix_arg), o.lit(0.5)), o.lit(0.5));
        let e = o.mul(o.lit(0.8), o.mix(o.lit(0.95), o.lit(1.3), pmix));
        let sign = if h > 0.0 { o.lit(1.0) } else { o.lit(-1.0) };
        let shaped = o.mul(sign, o.pow(o.abs(h), e));
        let ratio_noise = self.snoise3(dir_n.map(|c| o.mul(c, o.lit(4.0))));
        let c_ratio = o.clamp(o.add(o.mul(ratio_noise, o.lit(0.5)), o.lit(0.7)), o.lit(0.3), o.lit(1.0));
        o.mul(shaped, c_ratio)
    }

    fn cube_face_uv(&self, dir: V3) -> (f64, f64, f64) {
        let o = self.ops;
        let [dx, dy, dz] = o.normalize(dir);
        let (ax, ay, az) = (o.abs(dx), o.abs(dy), o.abs(dz));
        let on_x = ax >= ay && ax >= az;
        let on_y = !on_x && ay >= az;
        let (sx, sy, sz) = (o.div(1.0, ax), o.div(1.0, ay), o.div(1.0, az));
        let face = if on_x {
            if dx > 0.0 { 0.0 } else { 1.0 }
        } else if on_y {
            if dy > 0.0 { 2.0 } else { 3.0 }
        } else if dz > 0.0 {
            4.0
        } else {
            5.0
        };
        let u = if on_x {
            o.mul(if dx > 0.0 { o.neg(dz) } else { dz }, sx)
        } else if on_y {
            o.mul(dx, sy)
        } else {
            o.mul(if dz > 0.0 { dx } else { o.neg(dx) }, sz)
        };
        let v = if on_x {
            o.mul(dy, sx)
        } else if on_y {
            o.mul(if dy > 0.0 { o.neg(dz) } else { dz }, sy)
        } else {
            o.mul(dy, sz)
        };
        (face, o.add(o.mul(u, o.lit(0.5)), o.lit(0.5)), o.add(o.mul(v, o.lit(0.5)), o.lit(0.5)))
    }

    fn texel(&self, face: f64, x: f64, y: f64) -> f64 {
        let width = self.params.hpf_res as usize;
        let index = ((y as usize) * width + x as usize) * 4;
        f64::from(self.hpf[face as usize][index])
    }

    fn hpf_x(&self, dir: V3) -> f64 {
        let o = self.ops;
        let (face, fu, fv) = self.cube_face_uv(dir);
        let denom = o.sub(o.lit(self.params.hpf_res), o.lit(1.0));
        let tx = o.mul(fu, denom);
        let ty = o.mul(fv, denom);
        let fx0 = o.floor(tx);
        let fy0 = o.floor(ty);
        let wx = self.quintic(o.sub(tx, fx0));
        let wy = self.quintic(o.sub(ty, fy0));
        let x0 = o.clamp(fx0, o.lit(0.0), denom);
        let y0 = o.clamp(fy0, o.lit(0.0), denom);
        let x1 = o.min(o.add(x0, o.lit(1.0)), denom);
        let y1 = o.min(o.add(y0, o.lit(1.0)), denom);
        let s00 = self.texel(face, x0, y0);
        let s10 = self.texel(face, x1, y0);
        let s01 = self.texel(face, x0, y1);
        let s11 = self.texel(face, x1, y1);
        let ea = o.add(s00, o.mul(o.sub(s10, s00), wx));
        let eb = o.add(s01, o.mul(o.sub(s11, s01), wx));
        o.add(ea, o.mul(o.sub(eb, ea), wy))
    }

    fn shape_height(&self, fraction: f64, continental: f64) -> f64 {
        let o = self.ops;
        let scaled = o.add(o.mul(fraction, o.lit(FRACTAL_HEIGHT_M)), o.mul(continental, o.lit(CONTINENTAL_BIAS_AMP)));
        let h = o.add(scaled, o.lit(self.params.land_bias));
        let beach = o.lit(self.params.beach_shelf_m);
        let shelf = if beach > o.lit(1.0) { beach } else { o.lit(DEFAULT_BEACH_SHELF_M) };
        let shelved = if h < shelf {
            o.mul(o.div(o.mul(h, h), shelf), o.sub(o.lit(2.0), o.div(h, shelf)))
        } else {
            h
        };
        let underwater = o.max(o.mul(h, o.lit(UNDERWATER_GAIN)), o.lit(UNDERWATER_FLOOR_M));
        if h < 0.0 { underwater } else { shelved }
    }

    fn apply_relief_scale(&self, h: f64) -> f64 {
        let o = self.ops;
        let relief = o.lit(self.params.relief_scale);
        o.mul(h, if relief > 0.0 { relief } else { o.lit(1.0) })
    }

    fn carve_terrain(&self, dir: V3, h: f64) -> f64 {
        if self.carves.is_empty() {
            return h;
        }
        let o = self.ops;
        let dir = o.normalize(dir);
        self.carves.iter().fold(h, |h_prev, c| {
            let offset = [0, 1, 2].map(|i| o.sub(dir[i], c.dir[i]));
            let t = o.clamp(
                o.div(o.sub(o.dot(offset, offset), c.inner_chord2), o.sub(c.outer_chord2, c.inner_chord2)),
                o.lit(0.0),
                o.lit(1.0),
            );
            let w = o.mul(o.mul(t, t), o.sub(o.lit(3.0), o.mul(o.lit(2.0), t)));
            o.mix(c.target_h, h_prev, w)
        })
    }

    fn natural_height(&self, dir: V3) -> f64 {
        self.shape_height(self.fractal_terrain_h(dir), self.hpf_x(dir))
    }
}
