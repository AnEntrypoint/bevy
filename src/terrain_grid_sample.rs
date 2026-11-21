use crate::js_math::js_round;

const NON_FINITE_HEIGHT_FALLBACK_M: f64 = -1000.0;
const HEIGHT_SNAP_PER_METRE: f64 = 1000.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridSpec {
    pub n: usize,
    pub spacing: f64,
    pub corner_x: f64,
    pub corner_z: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GridSeed {
    pub filled: Vec<bool>,
    pub samples: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GridSample {
    pub samples: Vec<f32>,
    pub reused: usize,
    pub solved: usize,
}

fn snap_height(raw: f64) -> f32 {
    let h = if raw.is_finite() { raw } else { NON_FINITE_HEIGHT_FALLBACK_M };
    (js_round(h * HEIGHT_SNAP_PER_METRE) / HEIGHT_SNAP_PER_METRE + 0.0) as f32
}

pub fn sample_terrain_grid(
    grid: GridSpec,
    height_fn: &mut dyn FnMut(f64, f64) -> f64,
    seed: Option<&GridSeed>,
    mut checkpoint: Option<&mut dyn FnMut(usize) -> bool>,
) -> Option<GridSample> {
    let total = grid.n * grid.n;
    let mut samples = vec![0.0_f32; total];
    let mut reused = 0;
    if let Some(seed) = seed {
        samples.copy_from_slice(&seed.samples);
        reused = (0..total).filter(|k| seed.filled[*k]).count();
    }
    for z in 0..grid.n {
        let wz = grid.corner_z + z as f64 * grid.spacing;
        for x in 0..grid.n {
            let k = z * grid.n + x;
            if seed.is_some_and(|s| s.filled[k]) {
                continue;
            }
            let wx = grid.corner_x + x as f64 * grid.spacing;
            samples[k] = snap_height(height_fn(wx, wz));
        }
        if let Some(check) = checkpoint.as_deref_mut() {
            if check(z) {
                return None;
            }
        }
    }
    Some(GridSample { samples, reused, solved: total - reused })
}

mod tests {
    use super::*;
    use serde_json::Value;

    fn height(x: f64, z: f64) -> f64 {
        if x < -100.0 && z < -100.0 {
            f64::NAN
        } else if x > 250.0 && z > 250.0 {
            f64::INFINITY
        } else {
            0.03 * x - 0.02 * z + 0.0004 * x * z
        }
    }

    fn number(v: &Value) -> f64 {
        v.as_f64().unwrap()
    }

    #[test]
    fn terrain_grid_sample_matches_spoint() {
        let fixture: Value = serde_json::from_str(include_str!("../testdata/parity/spoint-terrain-grid-sample.json")).unwrap();
        let geometry = &fixture["geometry"];
        let grid = GridSpec {
            n: geometry["N"].as_u64().unwrap() as usize,
            spacing: number(&geometry["spacing"]),
            corner_x: number(&geometry["cornerX"]),
            corner_z: number(&geometry["cornerZ"]),
        };
        for scenario in fixture["scenarios"].as_array().unwrap() {
            let name = scenario["name"].as_str().unwrap();
            let seed = (!scenario["seed"].is_null()).then(|| GridSeed {
                filled: scenario["seed"]["filled"].as_array().unwrap().iter().map(|v| v.as_u64() == Some(1)).collect(),
                samples: scenario["seed"]["samples"].as_array().unwrap().iter().map(|v| number(v) as f32).collect(),
            });
            let every_row = number(&scenario["budgetMs"]) == 0.0;
            let abort_after = scenario["abortAfterYield"].as_u64().map(|v| v as usize);
            let mut calls = 0_usize;
            let mut yields = 0_usize;
            let mut height_fn = |x: f64, z: f64| {
                calls += 1;
                height(x, z)
            };
            let mut checkpoint = |_row: usize| {
                yields += 1;
                abort_after == Some(yields)
            };
            let got = if every_row {
                let check: &mut dyn FnMut(usize) -> bool = &mut checkpoint;
                sample_terrain_grid(grid, &mut height_fn, seed.as_ref(), Some(check))
            } else {
                sample_terrain_grid(grid, &mut height_fn, seed.as_ref(), None)
            };
            assert_eq!(calls, scenario["heightFnCalls"].as_u64().unwrap() as usize, "{name}: height function calls");
            assert_eq!(yields, scenario["yields"].as_u64().unwrap() as usize, "{name}: yields");
            if scenario["aborted"].as_bool().unwrap() {
                assert!(got.is_none(), "{name}: bevy completed the grid where spoint aborted");
                continue;
            }
            let got = got.unwrap_or_else(|| panic!("{name}: bevy aborted where spoint completed"));
            assert_eq!(got.reused, scenario["reused"].as_u64().unwrap() as usize, "{name}: reused");
            assert_eq!(got.solved, scenario["solved"].as_u64().unwrap() as usize, "{name}: solved");
            let want = scenario["samples"].as_array().unwrap();
            assert_eq!(got.samples.len(), want.len(), "{name}: sample count");
            for (k, (g, w)) in got.samples.iter().zip(want).enumerate() {
                assert_eq!(*g, number(w) as f32, "{name}: sample {k}");
            }
        }
    }
}
