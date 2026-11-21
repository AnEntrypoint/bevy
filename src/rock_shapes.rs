#![cfg_attr(not(test), allow(dead_code))]

use std::f64::consts::PI;

use crate::cave_sdf::{polygonize_cave_sdf, CaveMesh};

struct Lcg(f64);

impl Lcg {
    fn new(seed: i32) -> Self {
        Self(f64::from(seed))
    }

    fn next_unit(&mut self) -> f64 {
        self.0 = (self.0 * 1103515245.0 + 12345.0).rem_euclid(2147483648.0);
        self.0 / 2147483647.0
    }
}

struct Cut {
    dir: [f64; 3],
    radius: f64,
    ratio: f64,
    k: f64,
}

pub fn make_rock_sdf(seed: i32) -> impl Fn(f64, f64, f64) -> f64 {
    let mut rng = Lcg::new(seed);
    let num_cuts = 14 + (rng.next_unit() * 8.0).floor() as usize;
    let mut cuts = Vec::with_capacity(num_cuts);
    for _ in 0..num_cuts {
        let theta = rng.next_unit() * PI * 2.0;
        let phi = (2.0 * rng.next_unit() - 1.0).acos();
        cuts.push(Cut {
            dir: [phi.sin() * theta.cos(), phi.sin() * theta.sin(), phi.cos()],
            radius: 1.4 + rng.next_unit() * 1.1,
            ratio: 0.6 + rng.next_unit() * 0.25,
            k: 0.02 + rng.next_unit() * 0.06,
        });
    }
    let sx = 0.75 + rng.next_unit() * 0.5;
    let sy = 0.6 + rng.next_unit() * 0.4;
    let sz = 0.75 + rng.next_unit() * 0.5;
    move |x, y, z| {
        let (px, py, pz) = (x / sx, y / sy, z / sz);
        let mut d = (px * px + py * py + pz * pz).sqrt() - 1.05;
        for c in &cuts {
            let (ax, ay, az) = (px + c.dir[0] * c.radius, py + c.dir[1] * c.radius, pz + c.dir[2] * c.radius);
            let sd = (ax * ax + ay * ay + az * az).sqrt() - c.radius * c.ratio;
            let h = (0.5 + (0.5 * (-sd - d)) / c.k).clamp(0.0, 1.0);
            d = d * (1.0 - h) + (-sd) * h + c.k * h * (1.0 - h);
        }
        d.min(0.3)
    }
}

pub struct RockHull {
    pub positions: Vec<f32>,
    pub indices: Vec<u32>,
    pub v_count: usize,
    pub avg_radius: f64,
    pub squash_ratio: f64,
    pub bottom_y: f64,
    pub height: f64,
}

pub fn generate_rock_hull_data(num_shapes: usize, base_seed: i32, hull_res: usize) -> Vec<RockHull> {
    (0..num_shapes)
        .map(|s| {
            let CaveMesh { positions, indices } = polygonize_cave_sdf(make_rock_sdf(base_seed + s as i32 * 7919), hull_res);
            let v_count = positions.len() / 3;
            let n = v_count as f64;
            let (mut cx, mut cy, mut cz) = (0.0, 0.0, 0.0);
            for p in positions.chunks_exact(3) {
                cx += f64::from(p[0]);
                cy += f64::from(p[1]);
                cz += f64::from(p[2]);
            }
            let (cx, cy, cz) = (cx / n, cy / n, cz / n);
            let (mut total, mut min_y, mut max_y) = (0.0, f64::INFINITY, f64::NEG_INFINITY);
            for p in positions.chunks_exact(3) {
                let (dx, dy, dz) = (f64::from(p[0]) - cx, f64::from(p[1]) - cy, f64::from(p[2]) - cz);
                total += (dx * dx + dy * dy + dz * dz).sqrt();
                min_y = min_y.min(f64::from(p[1]));
                max_y = max_y.max(f64::from(p[1]));
            }
            let avg_radius = total / n;
            RockHull {
                positions,
                indices,
                v_count,
                avg_radius,
                squash_ratio: ((max_y - min_y) / 2.0) / avg_radius,
                bottom_y: min_y,
                height: max_y - min_y,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    const SDF_TOLERANCE: f64 = 1e-12;
    const POSITION_TOLERANCE: f64 = 1e-5;
    const METRIC_TOLERANCE: f64 = 1e-5;

    fn fixture() -> Value {
        serde_json::from_str(include_str!("../testdata/parity/spoint-rock-shapes.json")).unwrap()
    }

    fn num(v: &Value, key: &str) -> f64 {
        v[key].as_f64().unwrap()
    }

    fn u32s(v: &Value) -> Vec<u32> {
        v.as_array().unwrap().iter().map(|i| i.as_u64().unwrap() as u32).collect()
    }

    fn assert_positions(got: &[f32], want: &Value, what: &str) {
        let want: Vec<f64> = want.as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
        assert_eq!(got.len(), want.len(), "{what}: position count");
        for (i, (g, w)) in got.iter().zip(&want).enumerate() {
            assert!((f64::from(*g) - w).abs() <= POSITION_TOLERANCE, "{what}: position {i} = {g} against spoint {w}");
        }
    }

    #[test]
    fn sdf_samples_match_spoint_make_rock_sdf() {
        let fixture = fixture();
        let points = fixture["points"].as_array().unwrap();
        for sample in fixture["sdfSamples"].as_array().unwrap() {
            let seed = sample["seed"].as_i64().unwrap() as i32;
            let sdf = make_rock_sdf(seed);
            for (p, want) in points.iter().zip(sample["values"].as_array().unwrap()) {
                let got = sdf(p[0].as_f64().unwrap(), p[1].as_f64().unwrap(), p[2].as_f64().unwrap());
                let want = want.as_f64().unwrap();
                assert!((got - want).abs() <= SDF_TOLERANCE, "seed {seed} point {p}: {got} against spoint {want}");
            }
        }
    }

    #[test]
    fn polygonizer_matches_spoint_march_rock_surface() {
        let fixture = fixture();
        for mesh in fixture["meshes"].as_array().unwrap() {
            let seed = mesh["seed"].as_i64().unwrap() as i32;
            let res = mesh["res"].as_u64().unwrap() as usize;
            let got = polygonize_cave_sdf(make_rock_sdf(seed), res);
            let what = format!("seed {seed} res {res}");
            assert_eq!(got.vertex_count() as u64, mesh["vc"].as_u64().unwrap(), "{what}: vertex count");
            assert_eq!(got.indices.len() as u64, mesh["ic"].as_u64().unwrap(), "{what}: index count");
            assert_eq!(got.indices, u32s(&mesh["indices"]), "{what}: indices");
            assert_positions(&got.positions, &mesh["positions"], &what);
        }
    }

    #[test]
    fn hull_data_matches_spoint_generate_rock_hull_data() {
        let fixture = fixture();
        let hull = &fixture["hull"];
        let got = generate_rock_hull_data(
            hull["numShapes"].as_u64().unwrap() as usize,
            hull["baseSeed"].as_i64().unwrap() as i32,
            hull["hullRes"].as_u64().unwrap() as usize,
        );
        let shapes = hull["shapes"].as_array().unwrap();
        assert_eq!(got.len(), shapes.len(), "shape count");
        for (i, (g, w)) in got.iter().zip(shapes).enumerate() {
            let what = format!("shape {i}");
            assert_eq!(g.v_count as u64, w["vCount"].as_u64().unwrap(), "{what}: vertex count");
            assert_eq!(g.indices.len() as u64, w["ic"].as_u64().unwrap(), "{what}: index count");
            assert_eq!(g.indices, u32s(&w["indices"]), "{what}: indices");
            assert_positions(&g.positions, &w["positions"], &what);
            for (got, key) in [(g.avg_radius, "avgRadius"), (g.squash_ratio, "squashRatio"), (g.bottom_y, "bottomY"), (g.height, "height")] {
                let want = num(w, key);
                assert!((got - want).abs() <= METRIC_TOLERANCE, "{what}: {key} {got} against spoint {want}");
            }
        }
    }
}
