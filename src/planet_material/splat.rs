const SAND_REGION_SLOPE: [f64; 2] = [0.18, 0.55];
const SAND_SLOPE_FADE: [f64; 2] = [0.30, 0.70];
const DRY_HOT_ARIDITY: [f64; 2] = [0.60, 0.85];
const DRY_HOT_TEMP: [f64; 2] = [0.42, 0.62];
const ROCK_BAND_SNOW_FRAC: [f64; 2] = [0.7, 0.9];
const SNOW_SUPPRESSED_BY_ROCK: f64 = 0.6;
const BAND_WARP_FREQ: [f64; 2] = [1100.0, 2580.0];
const BAND_WARP_SECOND_OCTAVE_GAIN: f64 = 0.5;
const BAND_WARP_SCALE: f64 = 0.25;
const SAND_ROCK_SLOPE_FLOOR: [f64; 2] = [0.50, 0.70];
const ROCK_SLOPE_LOW_MIN: f64 = 0.05;
const ROCK_SLOPE_MIN_SPAN: f64 = 0.25;
const UNDERWATER_START_FRAC: f64 = 0.3;
const WEIGHT_EPS: f64 = 1e-4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplatWeights {
    pub grass: f64,
    pub rock: f64,
    pub sand: f64,
    pub snow: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct SplatParams {
    pub beach_top: f64,
    pub beach_width: f64,
    pub band_warp: f64,
    pub snow_edges: [f64; 2],
    pub slope_rock: [f64; 2],
}

impl SplatParams {
    pub fn reference() -> Self {
        Self { beach_top: 15.0, beach_width: 1.0, band_warp: 20.0, snow_edges: [120.0, 180.0], slope_rock: [0.25, 0.55] }
    }
}

fn smoothstep(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).max(0.0).min(1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mix(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

pub fn band_warp(snoise3: impl Fn(f64, f64, f64) -> f64, dir: [f64; 3], band_warp_amount: f64) -> f64 {
    let at = |freq: f64| snoise3(dir[0] * freq, dir[1] * freq, dir[2] * freq);
    (at(BAND_WARP_FREQ[0]) + BAND_WARP_SECOND_OCTAVE_GAIN * at(BAND_WARP_FREQ[1])) * band_warp_amount * BAND_WARP_SCALE
}

pub fn splat_weights(params: &SplatParams, band_warp: f64, h: f64, slope: f64, temp: f64, humid: f64) -> SplatWeights {
    let rock_lo = params.slope_rock[0].max(ROCK_SLOPE_LOW_MIN);
    let rock_hi = params.slope_rock[1].max(rock_lo + ROCK_SLOPE_MIN_SPAN);
    let snow = params.snow_edges;
    let dry_hot = smoothstep(DRY_HOT_ARIDITY[0], DRY_HOT_ARIDITY[1], 1.0 - humid) * smoothstep(DRY_HOT_TEMP[0], DRY_HOT_TEMP[1], temp);
    let beach_floor = band_warp.max(0.0);
    let beach = (1.0 - smoothstep(beach_floor, beach_floor + params.beach_top * params.beach_width, h))
        * (1.0 - smoothstep(SAND_REGION_SLOPE[0], SAND_REGION_SLOPE[1], slope));
    let sand_region = dry_hot.max(beach).max(0.0).min(1.0);
    let rock_slope = smoothstep(
        mix(rock_lo, SAND_ROCK_SLOPE_FLOOR[0], sand_region),
        mix(rock_hi, SAND_ROCK_SLOPE_FLOOR[1], sand_region),
        slope,
    );
    let snow_high = smoothstep(band_warp + snow[0], band_warp + snow[1], h);
    let rock_band = smoothstep(
        band_warp + snow[0] * ROCK_BAND_SNOW_FRAC[0],
        band_warp + snow[0] * ROCK_BAND_SNOW_FRAC[1],
        h,
    ) * (1.0 - snow_high);
    let rock = rock_slope.max(rock_band);
    let snow_weight = snow_high.max(0.0).min(1.0) * (1.0 - SNOW_SUPPRESSED_BY_ROCK * rock);
    let sand = sand_region * (1.0 - rock) * (1.0 - snow_weight) * (1.0 - smoothstep(SAND_SLOPE_FADE[0], SAND_SLOPE_FADE[1], slope));
    let grass = (1.0 - rock - snow_weight - sand).max(0.0);
    let on_land = 1.0 - smoothstep(params.beach_top * UNDERWATER_START_FRAC, params.beach_top, h);
    let sand_out = sand + (grass + snow_weight) * on_land;
    let grass_out = grass * (1.0 - on_land);
    let snow_out = snow_weight * (1.0 - on_land);
    let sum = grass_out + rock + sand_out + snow_out + WEIGHT_EPS;
    SplatWeights { grass: grass_out / sum, rock: rock / sum, sand: sand_out / sum, snow: snow_out / sum }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::height::height_spec::{HashVersion, HeightSpec, Params};
    use crate::height::ops_jsgen::{Precision, V3};
    use crate::testkit::{cases, close, parse};
    use serde_json::Value;

    const TOLERANCE: f64 = 1e-9;
    const NO_PARAMS: Params = Params { hpf_res: 8.0, land_bias: 0.0, beach_shelf_m: 0.0, relief_scale: 0.0 };

    fn fixture() -> Value {
        parse(include_str!("../../testdata/parity/spoint-planet-splat.json"))
    }

    fn number(value: &Value) -> f64 {
        value.as_f64().unwrap()
    }

    fn vec3(value: &Value) -> V3 {
        let a = value.as_array().unwrap();
        [number(&a[0]), number(&a[1]), number(&a[2])]
    }

    fn pair(value: &Value) -> [f64; 2] {
        [number(&value[0]), number(&value[1])]
    }

    fn spec() -> HeightSpec<'static> {
        HeightSpec::new(HashVersion::Integer, Precision::F64, NO_PARAMS, Vec::new(), &[]).unwrap()
    }

    fn fixture_params(fixture: &Value) -> SplatParams {
        let c = &fixture["constants"];
        SplatParams {
            beach_top: number(&c["beachTop"]),
            beach_width: number(&c["beachWidth"]),
            band_warp: number(&c["bandWarp"]),
            snow_edges: pair(&c["snowEdges"]),
            slope_rock: pair(&c["slopeRock"]),
        }
    }

    #[test]
    fn reference_defaults_match_the_fixture_constants() {
        let fixture = fixture();
        let want = fixture_params(&fixture);
        let got = SplatParams::reference();
        assert_eq!(
            (got.beach_top, got.beach_width, got.band_warp, got.snow_edges, got.slope_rock),
            (want.beach_top, want.beach_width, want.band_warp, want.snow_edges, want.slope_rock)
        );
    }

    #[test]
    fn band_warp_noise_terms_match_the_spoint_fixture() {
        let fixture = fixture();
        let spec = spec();
        let freqs = pair(&fixture["warpFreqs"]);
        assert_eq!(freqs, BAND_WARP_FREQ);
        let dirs = cases(&fixture, "warpDirs");
        let warp = cases(&fixture, "warp");
        assert_eq!(dirs.len(), 100);
        for (i, (dir, terms)) in dirs.iter().zip(warp).enumerate() {
            let dir = vec3(dir);
            for (k, freq) in freqs.iter().enumerate() {
                let got = spec.snoise3([dir[0] * freq, dir[1] * freq, dir[2] * freq]);
                close(got, number(&terms[k]), TOLERANCE, &format!("snoise3 dir {i} freq {freq}"));
            }
            let want = (number(&terms[0]) + 0.5 * number(&terms[1])) * 20.0 * 0.25;
            close(band_warp(|x, y, z| spec.snoise3([x, y, z]), dir, 20.0), want, TOLERANCE, &format!("band warp dir {i}"));
        }
    }

    #[test]
    fn splat_weights_match_the_spoint_fixture() {
        let fixture = fixture();
        let spec = spec();
        let params = fixture_params(&fixture);
        let warp_of = |dir: &Value| band_warp(|x, y, z| spec.snoise3([x, y, z]), vec3(dir), params.band_warp);
        let rows = cases(&fixture, "rows");
        let mut compared = 0;
        for sweep in cases(&fixture, "sweeps") {
            let name = sweep["name"].as_str().unwrap();
            let dirs = cases(&fixture, if name == "spread" { "spreadDirs" } else { "sweepDirs" });
            assert!(dirs.len() as u64 >= sweep["dirs"].as_u64().unwrap());
            let warps: Vec<f64> = dirs.iter().map(warp_of).collect();
            let start = sweep["start"].as_u64().unwrap() as usize;
            for (i, row) in rows[start..start + sweep["count"].as_u64().unwrap() as usize].iter().enumerate() {
                let r: Vec<f64> = row.as_array().unwrap().iter().map(number).collect();
                let got = splat_weights(&params, warps[r[0] as usize], r[1], r[2], r[3], r[4]);
                let context = format!("{name} row {i} dir {} h {} slope {} temp {} humid {}", r[0], r[1], r[2], r[3], r[4]);
                close(got.grass, r[5], TOLERANCE, &format!("grass {context}"));
                close(got.rock, r[6], TOLERANCE, &format!("rock {context}"));
                close(got.sand, r[7], TOLERANCE, &format!("sand {context}"));
                close(got.snow, r[8], TOLERANCE, &format!("snow {context}"));
                let sum = got.grass + got.rock + got.sand + got.snow;
                assert!(sum < 1.0 && sum >= 1.0 / (1.0 + WEIGHT_EPS) - 1e-12, "weights sum {sum} for {context}");
                compared += 1;
            }
        }
        assert_eq!(compared, rows.len());
        assert_eq!(compared, 32262);
    }
}
