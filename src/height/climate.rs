use super::anchor::{band_fractal, cycles_per_sphere, face_unit, hash3, BANDS, FACE_FRAME};
use super::ops_jsgen::V3;
use crate::vec3::dot;

const RAD_TO_DEG: f64 = 57.29577951;
const FACE_COUNT: usize = 6;
const HPF_CHANNELS: usize = 4;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Climate {
    pub sea_bias: f64,
    pub elev_amp: f64,
    pub temp: f64,
    pub humidity: f64,
    pub erosion: f64,
    pub roughness: f64,
}

fn quintic(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn vnoise2(x: f64, y: f64, seed: f64) -> f64 {
    let (xi, yi) = (x.floor(), y.floor());
    let (u, v) = (quintic(x - xi), quintic(y - yi));
    let (h00, h10) = (hash3(xi, yi, seed), hash3(xi + 1.0, yi, seed));
    let (h01, h11) = (hash3(xi, yi + 1.0, seed), hash3(xi + 1.0, yi + 1.0, seed));
    let a = h00 + u * (h10 - h00);
    let b = h01 + u * (h11 - h01);
    (a + v * (b - a)) * 2.0 - 1.0
}

fn fractal2(x: f64, y: f64, seed: f64, octaves: u32, gain: f64, ridged: bool) -> f64 {
    let (mut amp, mut freq, mut sum, mut norm) = (1.0, 1.0, 0.0, 0.0);
    for octave in 0..octaves {
        let mut n = vnoise2(x * freq, y * freq, seed + f64::from(octave) * 1013.0);
        if ridged {
            n = 1.0 - n.abs();
            n = n * n;
        }
        sum += n * amp;
        norm += amp;
        amp *= gain;
        freq *= 2.0;
    }
    let r = sum / norm.max(1e-6);
    if ridged { r * 2.0 - 1.0 } else { r }
}

fn gaussian(x: f64, centre: f64, width: f64) -> f64 {
    (-((x - centre) * (x - centre)) / (2.0 * width * width)).exp()
}

fn regional(fval: f64, lat: f64, fx: f64, fy: f64) -> Climate {
    let belt = fractal2(fval * 3.0, fval * 3.0, 31.0, 3, 0.5, true).max(0.0);
    let lat_base = lat.cos().max(0.0).powf(1.1);
    let t_noise = fractal2(fx * 2.2 + 11.0, fy * 2.2 - 7.0, 9211.0, 4, 0.55, false);
    let h_noise = fractal2(fx * 2.2 - 5.0, fy * 2.2 + 13.0, 9307.0, 4, 0.55, false);
    let inland = (fval * 0.5 + 0.5).clamp(0.0, 1.0);
    let temp = (lat_base.powf(1.4) * 1.05 + 0.28 * t_noise - 0.10 * inland - 0.04).clamp(0.0, 1.0);
    let lat_deg = lat.abs() * RAD_TO_DEG;
    let lat_humid = 0.20 * gaussian(lat_deg, 0.0, 11.0) - 0.24 * gaussian(lat_deg, 25.0, 8.0) + 0.12 * gaussian(lat_deg, 52.0, 14.0);
    let humidity = (0.62 + 0.52 * h_noise - 0.40 * inland + lat_humid - 0.22 * belt).clamp(0.0, 1.0);
    Climate { sea_bias: 0.0, elev_amp: 1.0 + 0.8 * belt, temp, humidity, erosion: 0.3 + 0.4 * belt, roughness: 0.0 }
}

pub fn band_params(band: usize, fval: f64, lat: f64, fx: f64, fy: f64) -> Climate {
    let flat = Climate { sea_bias: (BANDS[band].sea_bias)(fval), elev_amp: 1.0, ..Climate::default() };
    match band {
        0 => Climate { elev_amp: 1.0 + 0.25 * fval, ..flat },
        2 => Climate { sea_bias: flat.sea_bias, ..regional(fval, lat, fx, fy) },
        4 => Climate { elev_amp: 1.0 + 0.15 * fval, roughness: 0.4 + 0.6 * fval, ..flat },
        _ => flat,
    }
}

pub fn base_params_at(seed: i64, band: usize, face: usize, fu: f64, fv: f64) -> Climate {
    let unit = face_unit(face, fu, fv);
    let lat = unit[1].clamp(-1.0, 1.0).asin();
    let scaled = unit.map(|c| c * cycles_per_sphere(&BANDS[band]));
    band_params(band, band_fractal(&BANDS[band], scaled, seed), lat, scaled[0], scaled[2])
}

pub fn sample_uv(seed: i64, face: usize, fu: f64, fv: f64, max_level: Option<u32>) -> Climate {
    let mut sum = [0.0_f32; 4];
    let (mut amp, mut rough) = (1.0_f64, 0.0_f64);
    for band in (0..BANDS.len()).filter(|&band| max_level.is_none_or(|max| BANDS[band].level <= max)) {
        let p = base_params_at(seed, band, face, fu, fv);
        let additive = [p.sea_bias, p.temp, p.humidity, p.erosion].map(|c| c as f32);
        sum = [0, 1, 2, 3].map(|i| sum[i] + additive[i]);
        amp *= f64::from((1.0 + (f64::from((1.0 + p.elev_amp) as f32) - 1.0)) as f32);
        rough = rough.max(f64::from(p.roughness as f32));
    }
    let [sea_bias, temp, humidity, erosion] = sum.map(f64::from);
    Climate { sea_bias, elev_amp: f64::from(amp as f32), temp, humidity, erosion, roughness: f64::from(rough as f32) }
}

pub fn dir_to_face_uv(dir: V3) -> (usize, f64, f64) {
    let [ax, ay, az] = dir.map(f64::abs);
    let (face, scale) = if ax >= ay && ax >= az {
        (if dir[0] > 0.0 { 0 } else { 1 }, 1.0 / ax)
    } else if ay >= az {
        (if dir[1] > 0.0 { 2 } else { 3 }, 1.0 / ay)
    } else {
        (if dir[2] > 0.0 { 4 } else { 5 }, 1.0 / az)
    };
    let frame = FACE_FRAME[face];
    (face, dot(dir, frame.1) * scale * 0.5 + 0.5, dot(dir, frame.2) * scale * 0.5 + 0.5)
}

pub fn sample_dir(seed: i64, dir: V3) -> Climate {
    let (face, fu, fv) = dir_to_face_uv(dir);
    sample_uv(seed, face, fu, fv, None)
}

pub struct HpfFaces {
    res: usize,
    faces: Vec<Vec<f32>>,
}

impl HpfFaces {
    pub fn bake(seed: i64, res: usize, max_level: u32) -> Self {
        let denom = (res - 1) as f64;
        let faces = (0..FACE_COUNT)
            .map(|face| {
                (0..res * res)
                    .flat_map(|i| {
                        let s = sample_uv(seed, face, (i % res) as f64 / denom, (i / res) as f64 / denom, Some(max_level));
                        [s.sea_bias, s.elev_amp, s.temp, s.humidity].map(|c| c as f32)
                    })
                    .collect()
            })
            .collect();
        Self { res, faces }
    }

    pub fn sample(&self, face: usize, fu: f64, fv: f64) -> [f64; 4] {
        let denom = (self.res - 1) as f64;
        let (tx, ty) = (fu * denom, fv * denom);
        let (fx, fy) = (tx.floor(), ty.floor());
        let (x0, y0) = (fx.clamp(0.0, denom) as usize, fy.clamp(0.0, denom) as usize);
        let (x1, y1) = ((x0 + 1).min(self.res - 1), (y0 + 1).min(self.res - 1));
        let (wx, wy) = (quintic(tx - fx), quintic(ty - fy));
        let at = |x: usize, y: usize, c: usize| f64::from(self.faces[face][(y * self.res + x) * HPF_CHANNELS + c]);
        [0, 1, 2, 3].map(|c| {
            let a = at(x0, y0, c) + wx * (at(x1, y0, c) - at(x0, y0, c));
            let b = at(x0, y1, c) + wx * (at(x1, y1, c) - at(x0, y1, c));
            a + wy * (b - a)
        })
    }

    pub fn at_dir(&self, dir: V3) -> [f64; 4] {
        let (face, fu, fv) = dir_to_face_uv(dir);
        self.sample(face, fu, fv)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{cases, close, parse};
    use serde_json::Value;

    const SEED: i64 = 1337;

    fn fixture() -> Value {
        parse(include_str!("../../testdata/parity/spoint-planet-climate.json"))
    }

    fn numbers(value: &Value) -> Vec<f64> {
        value.as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect()
    }

    fn channels(c: Climate) -> [f64; 6] {
        [c.sea_bias, c.elev_amp, c.temp, c.humidity, c.erosion, c.roughness]
    }

    fn within(got: &[f64], want: &Value, relative: f64, context: &str) {
        let want = numbers(want);
        assert_eq!(got.len(), want.len(), "{context} count");
        for (i, (g, w)) in got.iter().zip(&want).enumerate() {
            close(*g, *w, relative * w.abs().max(1.0), &format!("{context}[{i}]"));
        }
    }

    fn directions(fixture: &Value) -> Vec<&Value> {
        cases(fixture, "radii").iter().flat_map(|radius| radius["directions"].as_array().unwrap()).collect()
    }

    fn dir_of(case: &Value) -> V3 {
        let d = numbers(&case["dir"]);
        [d[0], d[1], d[2]]
    }

    #[test]
    fn band_params_match_the_spoint_fixture() {
        let fixture = fixture();
        let bands = cases(&fixture, "bands");
        assert_eq!(bands.len(), 90);
        for case in bands {
            let band = case["band"].as_u64().unwrap() as usize;
            let n = f64::from(1u32 << BANDS[band].level);
            let (tx, ty) = (case["tx"].as_f64().unwrap(), case["ty"].as_f64().unwrap());
            let got = base_params_at(SEED, band, case["face"].as_u64().unwrap() as usize, (tx + 0.5) / n, (ty + 0.5) / n);
            within(&channels(got), &case["params"], 1e-9, &format!("band {band} face {} tile {tx},{ty}", case["face"]));
        }
    }

    #[test]
    fn face_selection_matches_the_spoint_fixture() {
        let fixture = fixture();
        let all = directions(&fixture);
        assert_eq!(all.len(), 182);
        for case in all {
            let (face, fu, fv) = dir_to_face_uv(dir_of(case));
            let name = case["name"].as_str().unwrap();
            assert_eq!(face as u64, case["face"].as_u64().unwrap(), "{name} face");
            close(fu, case["fu"].as_f64().unwrap(), 1e-12, &format!("{name} fu"));
            close(fv, case["fv"].as_f64().unwrap(), 1e-12, &format!("{name} fv"));
        }
    }

    #[test]
    fn continuous_channels_match_the_spoint_fixture() {
        let fixture = fixture();
        for case in directions(&fixture) {
            let name = case["name"].as_str().unwrap();
            within(&channels(sample_dir(SEED, dir_of(case))), &case["continuous"], 1e-6, name);
        }
    }

    #[test]
    fn baked_channels_match_the_spoint_fixture_and_differ_from_the_continuous_field() {
        let fixture = fixture();
        let baked = HpfFaces::bake(SEED, 128, 7);
        let mut elev_amp_gap = 0.0_f64;
        for case in directions(&fixture) {
            let name = case["name"].as_str().unwrap();
            let got = baked.at_dir(dir_of(case));
            within(&got, &case["hpf"], 1e-6, name);
            elev_amp_gap = elev_amp_gap.max((got[1] - sample_dir(SEED, dir_of(case)).elev_amp).abs());
        }
        assert!(elev_amp_gap > 1e-3, "the bake drops the level 9 band, so baked elevAmp must differ from the continuous field");
    }

    #[test]
    fn bake_nodes_match_the_spoint_fixture() {
        let fixture = fixture();
        let baked = HpfFaces::bake(SEED, 128, 7);
        let nodes: Vec<&Value> = cases(&fixture, "radii").iter().flat_map(|radius| radius["bakeNodes"].as_array().unwrap()).collect();
        assert_eq!(nodes.len(), 96);
        for case in nodes {
            let (face, x, y) = (case["face"].as_u64().unwrap() as usize, case["x"].as_u64().unwrap() as usize, case["y"].as_u64().unwrap() as usize);
            let offset = (y * 128 + x) * HPF_CHANNELS;
            let got: Vec<f64> = baked.faces[face][offset..offset + HPF_CHANNELS].iter().map(|c| f64::from(*c)).collect();
            within(&got, &case["baked"], 1e-6, &format!("bake node face {face} {x},{y}"));
            let continuous = sample_uv(SEED, face, x as f64 / 127.0, y as f64 / 127.0, Some(7));
            within(&channels(continuous), &case["continuous"], 1e-6, &format!("level 7 face {face} {x},{y}"));
        }
    }
}
