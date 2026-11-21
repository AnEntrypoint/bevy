use super::ops_jsgen::V3;

pub(crate) const FACE_FRAME: [([f64; 3], [f64; 3], [f64; 3]); 6] = [
    ([1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]),
    ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
    ([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]),
    ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
    ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
    ([0.0, 0.0, -1.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
];

const FRACTAL_CYCLES_PER_CELL: f64 = 0.06;
const FACES_AROUND_SPHERE: f64 = 4.0;
const WARP_SEED_OFFSETS: [i64; 3] = [7, 9, 13];
const WARP_STRENGTH: f64 = 0.6;

enum Shape {
    Warped,
    Plain,
    Local,
}

pub(crate) struct Band {
    pub(crate) level: u32,
    index: i64,
    shape: Shape,
    final_offset: i64,
    pub(crate) sea_bias: fn(f64) -> f64,
}

pub(crate) const BANDS: [Band; 5] = [
    Band { level: 3, index: 0, shape: Shape::Warped, final_offset: 0, sea_bias: |f| (f - 0.13) * 2600.0 },
    Band { level: 5, index: 1, shape: Shape::Warped, final_offset: 65, sea_bias: |f| f * 900.0 },
    Band { level: 6, index: 2, shape: Shape::Plain, final_offset: 0, sea_bias: |f| f * 1600.0 },
    Band { level: 7, index: 3, shape: Shape::Warped, final_offset: 91, sea_bias: |f| f * 750.0 },
    Band { level: 9, index: 4, shape: Shape::Local, final_offset: 0, sea_bias: |_| 0.0 },
];

fn to_u32(value: f64) -> u32 {
    value.rem_euclid(4294967296.0) as u32
}

pub(crate) fn hash3(a: f64, b: f64, c: f64) -> f64 {
    let sum = f64::from(a as i32) * 374761393.0 + f64::from(b as i32) * 668265263.0 + f64::from(c as i32) * 2246822519.0;
    let h = to_u32(sum);
    let h = h ^ (h >> 13);
    let h = to_u32(f64::from(h) * 1274126177.0);
    let h = h ^ (h >> 16);
    f64::from(h) / 4294967296.0
}

fn vnoise3(x: f64, y: f64, z: f64, seed: i64) -> f64 {
    let (xi, yi, zi) = (x.floor(), y.floor(), z.floor());
    let (xf, yf, zf) = (x - xi, y - yi, z - zi);
    let u = xf * xf * xf * (xf * (xf * 6.0 - 15.0) + 10.0);
    let v = yf * yf * yf * (yf * (yf * 6.0 - 15.0) + 10.0);
    let w = zf * zf * zf * (zf * (zf * 6.0 - 15.0) + 10.0);
    let hash_at = |a: f64, b: f64, c: f64| hash3(a, b, f64::from(((c * 2654435761.0) as i64 as i32) ^ (seed as i32)));
    let c000 = hash_at(xi, yi, zi);
    let c100 = hash_at(xi + 1.0, yi, zi);
    let c010 = hash_at(xi, yi + 1.0, zi);
    let c110 = hash_at(xi + 1.0, yi + 1.0, zi);
    let c001 = hash_at(xi, yi, zi + 1.0);
    let c101 = hash_at(xi + 1.0, yi, zi + 1.0);
    let c011 = hash_at(xi, yi + 1.0, zi + 1.0);
    let c111 = hash_at(xi + 1.0, yi + 1.0, zi + 1.0);
    let x00 = c000 + u * (c100 - c000);
    let x10 = c010 + u * (c110 - c010);
    let x01 = c001 + u * (c101 - c001);
    let x11 = c011 + u * (c111 - c011);
    let y0 = x00 + v * (x10 - x00);
    let y1 = x01 + v * (x11 - x01);
    (y0 + w * (y1 - y0)) * 2.0 - 1.0
}

fn fractal3(p: V3, seed: i64, octaves: u32, gain: f64, lacunarity: f64, ridged: bool) -> f64 {
    let (mut amp, mut freq, mut sum, mut norm) = (1.0, 1.0, 0.0, 0.0);
    for octave in 0..octaves {
        let mut n = vnoise3(p[0] * freq, p[1] * freq, p[2] * freq, seed + i64::from(octave) * 1013);
        if ridged {
            n = 1.0 - n.abs();
            n = n * n;
        }
        sum += n * amp;
        norm += amp;
        amp *= gain;
        freq *= lacunarity;
    }
    let r = sum / norm.max(1e-6);
    if ridged { r * 2.0 - 1.0 } else { r }
}

pub(crate) fn cycles_per_sphere(band: &Band) -> f64 {
    f64::from(1u32 << band.level) * FRACTAL_CYCLES_PER_CELL * FACES_AROUND_SPHERE
}

pub(crate) fn band_fractal(band: &Band, p: V3, seed: i64) -> f64 {
    let seed = seed + band.index * 101;
    match band.shape {
        Shape::Plain => fractal3(p, seed, 5, 0.5, 2.0, false),
        Shape::Local => fractal3(p, seed, 4, 0.55, 2.2, false).abs(),
        Shape::Warped => {
            let q = [0, 1, 2].map(|i| p[i] + WARP_STRENGTH * fractal3(p, seed + WARP_SEED_OFFSETS[i], 2, 0.5, 2.0, false));
            fractal3(q, seed + band.final_offset, 4, 0.55, 2.0, true)
        }
    }
}

pub(crate) fn face_unit(face: usize, fu: f64, fv: f64) -> V3 {
    let frame = FACE_FRAME[face];
    let (u, v) = (fu * 2.0 - 1.0, fv * 2.0 - 1.0);
    let d = [0, 1, 2].map(|i| frame.0[i] + u * frame.1[i] + v * frame.2[i]);
    let length = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    let length = if length > 0.0 { length } else { 1.0 };
    d.map(|c| c / length)
}

pub fn sea_bias_at(seed: i64, face: usize, fu: f64, fv: f64, max_level: u32) -> f32 {
    let unit = face_unit(face, fu, fv);
    let mut acc = 0.0_f32;
    for band in BANDS.iter().filter(|band| band.level <= max_level) {
        let fval = band_fractal(band, unit.map(|c| c * cycles_per_sphere(band)), seed);
        acc = (f64::from(acc) + f64::from((band.sea_bias)(fval) as f32)) as f32;
    }
    acc
}

pub fn sea_bias_faces(seed: i64, res: usize) -> Vec<Vec<f32>> {
    let denom = (res - 1) as f64;
    let max_level = (res as f64).log2().round() as u32;
    (0..6)
        .map(|face| {
            (0..res * res)
                .map(|i| sea_bias_at(seed, face, (i % res) as f64 / denom, (i / res) as f64 / denom, max_level))
                .collect()
        })
        .collect()
}
