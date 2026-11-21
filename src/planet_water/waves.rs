pub const SEA_OCTAVES: usize = 8;
pub const SEA_FREQ: f64 = 0.2;
pub const SEA_SPEED: f64 = 2.4;
pub const SEA_AMP_M: f64 = 0.6;
pub const SEA_OCTAVE_ANGLE: f64 = 1.57079633;
pub const SEA_FREQ_LF: f64 = 0.16;
pub const SLOPE_EPS_M: f64 = 0.25;
pub const SLOPE_EPS_LF_M: f64 = 1.25;
pub const WAVE_TILE_M: f64 = 1024.0;
pub const DEFAULT_AMPLITUDE: f64 = 1.0;
pub const DEFAULT_CHOPPINESS: f64 = 0.5;
pub const DEFAULT_FOAM: f64 = 0.5;
const FREQ_GAIN: f64 = 1.9;
const AMP_GAIN: f64 = 0.45;
const CHOPPY_TO_ONE: f64 = 0.3;
const OCTAVE_TIME_SKEW: f64 = 0.08;
const TIME_SCALE_X: f64 = 0.6;
const TIME_SCALE_Y: f64 = 0.4;
const LF_DIRECTION_X: [f64; 2] = [0.866, 0.5];
const LF_DIRECTION_Y: [f64; 2] = [-0.5, 0.866];
const LF_AMP_SHARE: f64 = 0.5;
const NOISE_HASH_X: u32 = 1597334673;
const NOISE_HASH_Y: u32 = 3812015801;
const NOISE_HASH_MIX: u32 = 1597334673;
const NOISE_RANGE: f64 = 4294967296.0;
const NOISE_SMOOTH: [f64; 2] = [3.0, 2.0];
const CREST_SHARPNESS: f64 = 0.65;
const BASIS_POLE_LIMIT: f64 = 0.99;
const MEAN_GRID: usize = 256;
const MEAN_GRID_STEP: f64 = 3.7013;
const MEAN_PHASE_STEP: f64 = 0.31;

pub type Point = [f64; 2];

fn hash(x: i32, y: i32) -> u32 {
    ((x as u32).wrapping_mul(NOISE_HASH_X) ^ (y as u32).wrapping_mul(NOISE_HASH_Y)).wrapping_mul(NOISE_HASH_MIX)
}

pub fn noise(p: Point) -> f64 {
    let (ix, iy) = (p[0].floor(), p[1].floor());
    let smooth = |f: f64| f * f * (NOISE_SMOOTH[0] - NOISE_SMOOTH[1] * f);
    let (fx, fy) = (smooth(p[0] - ix), smooth(p[1] - iy));
    let corner = |dx: i32, dy: i32| f64::from(hash(ix as i32 + dx, iy as i32 + dy));
    let bottom = corner(0, 0) + (corner(1, 0) - corner(0, 0)) * fx;
    let top = corner(0, 1) + (corner(1, 1) - corner(0, 1)) * fx;
    (bottom + (top - bottom) * fy) / NOISE_RANGE
}

pub fn octave(p: Point, choppy: f64) -> f64 {
    let n = noise(p);
    let (ux, uy) = (p[0] + n, p[1] + n);
    let (wx, wy) = (1.0 - ux.sin().abs(), 1.0 - uy.sin().abs());
    let (wx, wy) = (wx + (ux.cos().abs() - wx) * wx, wy + (uy.cos().abs() - wy) * wy);
    (1.0 - (wx * wy).powf(CREST_SHARPNESS)).powf(choppy)
}

pub fn sea_height(p: Point, t: f64, octaves: std::ops::Range<usize>, amplitude: f64, choppy: f64) -> f64 {
    let sea_time = [t * (SEA_SPEED * TIME_SCALE_X), t * (SEA_SPEED * TIME_SCALE_Y)];
    let (mut freq, mut amp, mut choppy, mut height) = (SEA_FREQ, amplitude * SEA_AMP_M, choppy, 0.0);
    for i in 0..SEA_OCTAVES {
        if octaves.contains(&i) {
            let (sa, ca) = (i as f64 * SEA_OCTAVE_ANGLE).sin_cos();
            let skew = 1.0 - i as f64 * OCTAVE_TIME_SKEW;
            let rotated = [ca * p[0] + sa * p[1], ca * p[1] - sa * p[0]];
            height += octave([(rotated[0] + sea_time[0] * skew) * freq, (rotated[1] + sea_time[1] * skew) * freq], choppy) * amp;
        }
        freq *= FREQ_GAIN;
        amp *= AMP_GAIN;
        choppy += (1.0 - choppy) * CHOPPY_TO_ONE;
    }
    height
}

pub fn sea_height_lf(p: Point, t: f64, amplitude: f64, choppy: f64) -> f64 {
    let q = [p[0] * SEA_FREQ_LF, p[1] * SEA_FREQ_LF];
    let shifted = |direction: [f64; 2], scale: f64| octave([q[0] + direction[0] * (t * (SEA_SPEED * scale)), q[1] + direction[1] * (t * (SEA_SPEED * scale))], choppy);
    (shifted(LF_DIRECTION_X, TIME_SCALE_X) + shifted(LF_DIRECTION_Y, TIME_SCALE_Y)) * (amplitude * (SEA_AMP_M * LF_AMP_SHARE))
}

fn slope_of(height: impl Fn(Point) -> f64, p: Point, eps: f64) -> Point {
    let h = height(p);
    [(height([p[0] + eps, p[1]]) - h) / eps, (height([p[0], p[1] + eps]) - h) / eps]
}

pub fn sea_slope(p: Point, t: f64, octaves: std::ops::Range<usize>, amplitude: f64, choppy: f64) -> Point {
    slope_of(|q| sea_height(q, t, octaves.clone(), amplitude, choppy), p, SLOPE_EPS_M)
}

pub fn sea_slope_lf(p: Point, t: f64, amplitude: f64, choppy: f64) -> Point {
    slope_of(|q| sea_height_lf(q, t, amplitude, choppy), p, SLOPE_EPS_LF_M)
}

pub fn tangent_normal(slope: Point) -> [f64; 3] {
    let length = (slope[0] * slope[0] + slope[1] * slope[1] + 1.0).sqrt();
    [-slope[0] / length, -slope[1] / length, 1.0 / length]
}

#[cfg(test)]
pub fn tangent_basis(dir: [f64; 3]) -> [[f64; 3]; 2] {
    let normalise = |v: [f64; 3]| {
        let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        v.map(|c| c / length)
    };
    let cross = |a: [f64; 3], b: [f64; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    let uz = normalise(dir);
    let helper = if uz[1].abs() < BASIS_POLE_LIMIT { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] };
    let ux = normalise(cross(helper, uz));
    [ux, cross(uz, ux)]
}

pub fn sea_height_mean(choppy: f64, amplitude: f64) -> f64 {
    let mut unit_mean = 0.0;
    let (mut amp, mut choppy) = (SEA_AMP_M, choppy);
    for i in 0..SEA_OCTAVES {
        let phase = MEAN_PHASE_STEP * i as f64;
        let sum: f64 = (0..MEAN_GRID * MEAN_GRID).map(|n| octave([(n % MEAN_GRID) as f64 * MEAN_GRID_STEP + phase, (n / MEAN_GRID) as f64 * MEAN_GRID_STEP + phase], choppy)).sum();
        unit_mean += amp * (sum / (MEAN_GRID * MEAN_GRID) as f64);
        amp *= AMP_GAIN;
        choppy += (1.0 - choppy) * CHOPPY_TO_ONE;
    }
    unit_mean * amplitude
}

pub fn wave_tile_offset(position: f64) -> f64 {
    position - (position / WAVE_TILE_M).floor() * WAVE_TILE_M
}
