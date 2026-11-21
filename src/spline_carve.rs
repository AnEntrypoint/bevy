
use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;
use crate::biome_override::cell_key;

pub const CELL_M: f64 = 8.0;

pub type Point = [f64; 2];

pub struct Band<'a> {
    pub width: usize,
    pub spacing: f64,
    pub corner: Point,
    pub control_points: &'a [Point],
    pub carve_width: f64,
    pub carve_depth: f64,
    pub bank_width: f64,
}

pub struct CarvedHeights {
    pub heights: Vec<f32>,
    pub carved_cells: HashSet<i64>,
}

#[derive(Default, Clone, Copy)]
pub struct CarveOpts {
    pub seed: i32,
    pub width_variance: f64,
    pub bank_erosion: f64,
}

#[derive(Clone, Copy)]
struct Cell {
    delta: f64,
    kind: &'static str,
}

#[derive(Default)]
pub struct SplineCarveLayer {
    cells: HashMap<i64, Cell>,
    spline_count: usize,
}

fn cell_index(v: f64) -> i64 {
    (v / CELL_M).floor() as i64
}

fn carve_hash(seed: i32, i: i64) -> f64 {
    let mut h = seed as u32;
    h = (h ^ (i as i32 as u32)).wrapping_mul(0x27d4eb2d);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a2d39);
    h ^= h >> 15;
    h as f64 / 4294967296.0
}

fn catmull_rom(p0: Point, p1: Point, p2: Point, p3: Point, t: f64) -> Point {
    let t2 = t * t;
    let t3 = t2 * t;
    let axis = |a: usize| {
        0.5 * ((2.0 * p1[a])
            + (-p0[a] + p2[a]) * t
            + (2.0 * p0[a] - 5.0 * p1[a] + 4.0 * p2[a] - p3[a]) * t2
            + (-p0[a] + 3.0 * p1[a] - 3.0 * p2[a] + p3[a]) * t3)
    };
    [axis(0), axis(1)]
}

pub fn sample_spline(points: &[Point], step_size: f64, closed: bool) -> Vec<Point> {
    if points.len() < 2 {
        return Vec::new();
    }
    let len = points.len();
    let n = if closed { len } else { len - 1 };
    let mut samples = Vec::new();
    for i in 0..n {
        let p0 = points[(i + len - 1) % len];
        let p1 = points[i];
        let p2 = points[(i + 1) % len];
        let p3 = points[(i + 2) % len];
        let dx = p2[0] - p1[0];
        let dz = p2[1] - p1[1];
        let seg_len = (dx * dx + dz * dz).sqrt();
        let steps = ((seg_len / step_size).ceil() as usize).max(1);
        for s in 0..steps {
            samples.push(catmull_rom(p0, p1, p2, p3, s as f64 / steps as f64));
        }
    }
    if !closed {
        samples.push(points[len - 1]);
    }
    samples
}

pub fn carve_spline(heights: &[f32], band: &Band) -> CarvedHeights {
    let unchanged = || CarvedHeights {
        heights: heights.to_vec(),
        carved_cells: HashSet::new(),
    };
    if band.width < 2 || !band.spacing.is_finite() || band.spacing <= 0.0 {
        return unchanged();
    }
    if band.control_points.len() < 2 {
        return unchanged();
    }
    if !band.carve_width.is_finite() || band.carve_width <= 0.0 || !band.carve_depth.is_finite() {
        return unchanged();
    }
    let n = band.width;
    let mut out = heights.to_vec();
    let mut carved_cells = HashSet::new();
    let spline = sample_spline(band.control_points, band.spacing * 0.5, false);
    if spline.is_empty() {
        return CarvedHeights {
            heights: out,
            carved_cells,
        };
    }
    let half_width = band.carve_width / 2.0;
    let total_half_width = half_width + band.bank_width;
    for iz in 0..n {
        for ix in 0..n {
            let wx = band.corner[0] + ix as f64 * band.spacing;
            let wz = band.corner[1] + iz as f64 * band.spacing;
            let mut min_dist = f64::INFINITY;
            for sp in &spline {
                let dx = wx - sp[0];
                let dz = wz - sp[1];
                let dist = (dx * dx + dz * dz).sqrt();
                if dist < min_dist {
                    min_dist = dist;
                }
                if min_dist <= total_half_width {
                    break;
                }
            }
            if min_dist > total_half_width {
                continue;
            }
            let idx = iz * n + ix;
            let Some(&h) = out.get(idx) else {
                continue;
            };
            if !h.is_finite() {
                continue;
            }
            let falloff = if min_dist > half_width && band.bank_width > 0.0 {
                0.5 * (1.0 + (((min_dist - half_width) / band.bank_width) * PI).cos())
            } else if min_dist > half_width {
                continue;
            } else {
                1.0
            };
            out[idx] = (h as f64 - band.carve_depth * falloff) as f32;
            carved_cells.insert(cell_key(cell_index(wx), cell_index(wz)));
        }
    }
    CarvedHeights {
        heights: out,
        carved_cells,
    }
}

pub fn is_carved(carved: &HashSet<i64>, cx: i64, cz: i64) -> bool {
    carved.contains(&cell_key(cx, cz))
}

pub fn create_carved_climate_filter<T>(
    carved: HashSet<i64>,
) -> Option<impl Fn(Option<T>, i64, i64) -> Option<T>> {
    if carved.is_empty() {
        return None;
    }
    Some(move |climate: Option<T>, cx: i64, cz: i64| {
        climate.filter(|_| !carved.contains(&cell_key(cx, cz)))
    })
}

impl SplineCarveLayer {
    pub fn carve(
        &mut self,
        control_points: &[Point],
        width: f64,
        depth: f64,
        kind: &str,
        base: Option<&dyn Fn(f64, f64) -> f64>,
        opts: CarveOpts,
    ) -> usize {
        if control_points.len() < 2 || !width.is_finite() || width <= 0.0 || !depth.is_finite() {
            return 0;
        }
        let kind: &'static str = match kind {
            "river" => "river",
            "road" => "road",
            _ => return 0,
        };
        let is_river = kind == "river";
        let width_variance = if is_river { opts.width_variance } else { 0.0 };
        let bank_erosion = if is_river { opts.bank_erosion } else { 0.0 };
        let spine = sample_spline(control_points, (width * 0.5).max(1.0), false);
        if spine.is_empty() {
            return 0;
        }
        let max_half_width = (width * 0.5) * (1.0 + width_variance) + bank_erosion;
        let mut touched = 0;
        let mut arc_len = 0.0;
        for (si, sp) in spine.iter().enumerate() {
            let [sx, sz] = *sp;
            if si > 0 {
                let [px, pz] = spine[si - 1];
                arc_len += (sx - px).hypot(sz - pz);
            }
            let mut half_width = width * 0.5;
            if width_variance > 0.0 {
                let t = carve_hash(opts.seed, (arc_len * 0.1).round() as i64);
                half_width *= 1.0 + (t * 2.0 - 1.0) * width_variance;
            }
            let cx0 = ((sx - max_half_width) / CELL_M).floor() as i64;
            let cx1 = ((sx + max_half_width) / CELL_M).ceil() as i64;
            let cz0 = ((sz - max_half_width) / CELL_M).floor() as i64;
            let cz1 = ((sz + max_half_width) / CELL_M).ceil() as i64;
            for cz in cz0..=cz1 {
                for cx in cx0..=cx1 {
                    let c_min_x = cx as f64 * CELL_M;
                    let c_min_z = cz as f64 * CELL_M;
                    let ccx = c_min_x + CELL_M * 0.5;
                    let ccz = c_min_z + CELL_M * 0.5;
                    let mut bank_offset = 0.0;
                    if bank_erosion > 0.0 {
                        let angle = carve_hash(opts.seed ^ 0x5bd1e995, cx * 92821 + cz) * PI * 2.0;
                        bank_offset = (carve_hash(opts.seed ^ 0x1b873593, cx * 15485863 + cz) * 2.0
                            - 1.0)
                            * bank_erosion
                            * (0.5 + 0.5 * angle.cos());
                    }
                    let cell_half_width = half_width + bank_offset;
                    if cell_half_width <= 0.0 {
                        continue;
                    }
                    let nearest_x = c_min_x.max(sx.min(c_min_x + CELL_M));
                    let nearest_z = c_min_z.max(sz.min(c_min_z + CELL_M));
                    if (nearest_x - sx).hypot(nearest_z - sz) > cell_half_width {
                        continue;
                    }
                    let d_center = (ccx - sx).hypot(ccz - sz);
                    let falloff_dist = d_center.min(cell_half_width);
                    let falloff = 0.5 * (1.0 + ((falloff_dist / cell_half_width) * PI).cos());
                    let key = cell_key(cx, cz);
                    let base_ok = base.is_some_and(|f| f(ccx, ccz).is_finite());
                    let target_delta = if base_ok { -depth * falloff } else { 0.0 };
                    let cell = match self.cells.get(&key).copied() {
                        Some(prev) => Cell {
                            delta: prev.delta.min(target_delta),
                            kind: if prev.kind == "river" { "river" } else { kind },
                        },
                        None => Cell {
                            delta: target_delta,
                            kind,
                        },
                    };
                    self.cells.insert(key, cell);
                    touched += 1;
                }
            }
        }
        if touched > 0 {
            self.spline_count += 1;
        }
        touched
    }

    pub fn delta_at(&self, x: f64, z: f64) -> f64 {
        self.cells
            .get(&cell_key(cell_index(x), cell_index(z)))
            .map_or(0.0, |c| c.delta)
    }

    pub fn kind_at(&self, x: f64, z: f64) -> Option<&'static str> {
        self.cells
            .get(&cell_key(cell_index(x), cell_index(z)))
            .map(|c| c.kind)
    }

    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    pub fn spline_count(&self) -> usize {
        self.spline_count
    }
}

mod tests {
    use super::*;
    use serde_json::Value;

    const TOLERANCE: f64 = 1e-9;
    const HEIGHT_TOLERANCE: f64 = 1e-5;

    fn fixture() -> Value {
        serde_json::from_str(include_str!("../testdata/parity/spoint-spline-carve.json")).unwrap()
    }

    fn num(v: &Value, field: &str) -> f64 {
        v[field].as_f64().unwrap()
    }

    fn point(v: &Value) -> Point {
        let a = v.as_array().unwrap();
        [a[0].as_f64().unwrap(), a[1].as_f64().unwrap()]
    }

    fn points(v: &Value) -> Vec<Point> {
        v.as_array().unwrap().iter().map(point).collect()
    }

    fn assert_close(got: f64, want: f64, tolerance: f64, what: &str) {
        assert!(
            (got - want).abs() <= tolerance,
            "{what}: {got} against spoint {want}"
        );
    }

    fn named<'a>(fx: &'a Value, group: &str, name: &str) -> &'a Value {
        fx[group]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == name)
            .unwrap()
    }

    fn carve_case(case: &Value) -> CarvedHeights {
        let width = case["width"].as_u64().unwrap() as usize;
        let mut heights: Vec<f32> = case["heightsIn"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap() as f32)
            .collect();
        for i in case["nanIndices"].as_array().unwrap() {
            heights[i.as_u64().unwrap() as usize] = f32::NAN;
        }
        let control_points = points(&case["controlPoints"]);
        let band = Band {
            width,
            spacing: num(case, "spacing"),
            corner: [num(case, "cornerX"), num(case, "cornerZ")],
            control_points: &control_points,
            carve_width: num(case, "carveWidth"),
            carve_depth: num(case, "carveDepth"),
            bank_width: num(case, "bankWidth"),
        };
        carve_spline(&heights, &band)
    }

    #[test]
    fn sample_spline_matches_spoint() {
        let fx = fixture();
        for case in fx["sampleSpline"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let got = sample_spline(
                &points(&case["controlPoints"]),
                num(case, "stepSize"),
                case["closed"].as_bool().unwrap(),
            );
            let want = points(&case["points"]);
            assert_eq!(got.len(), want.len(), "{name} sample count");
            for (k, (g, w)) in got.iter().zip(&want).enumerate() {
                assert_close(g[0], w[0], TOLERANCE, &format!("{name} sample[{k}].x"));
                assert_close(g[1], w[1], TOLERANCE, &format!("{name} sample[{k}].z"));
            }
        }
    }

    #[test]
    fn carve_spline_matches_spoint() {
        let fx = fixture();
        for case in fx["carveSpline"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let got = carve_case(case);
            let want_heights = case["heights"].as_array().unwrap();
            assert_eq!(got.heights.len(), want_heights.len(), "{name} heights length");
            for (k, (g, w)) in got.heights.iter().zip(want_heights).enumerate() {
                match w.as_f64() {
                    Some(w) => assert_close(
                        *g as f64,
                        w,
                        HEIGHT_TOLERANCE,
                        &format!("{name} heights[{k}]"),
                    ),
                    None => assert!(g.is_nan(), "{name} heights[{k}] should be NaN, got {g}"),
                }
            }
            let mut cells: Vec<i64> = got.carved_cells.into_iter().collect();
            cells.sort_unstable();
            let want_cells: Vec<i64> = case["cellKeys"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_i64().unwrap())
                .collect();
            assert_eq!(cells, want_cells, "{name} carved cells");
        }
    }

    #[test]
    fn is_carved_matches_spoint() {
        let fx = fixture();
        let carved = carve_case(named(&fx, "carveSpline", "river_bank")).carved_cells;
        let section = &fx["isCarved"];
        for c in section["cases"].as_array().unwrap() {
            let (cx, cz) = (c["cx"].as_i64().unwrap(), c["cz"].as_i64().unwrap());
            assert_eq!(
                is_carved(&carved, cx, cz),
                c["carved"].as_bool().unwrap(),
                "isCarved cx={cx} cz={cz}"
            );
        }
        assert_eq!(
            is_carved(&HashSet::new(), 0, 0),
            section["emptyCarved"].as_bool().unwrap()
        );
    }

    #[test]
    fn carved_climate_filter_matches_spoint() {
        let fx = fixture();
        let section = &fx["carvedClimateFilter"];
        let carved = carve_case(named(&fx, "carveSpline", "river_bank")).carved_cells;
        let filter = create_carved_climate_filter::<&str>(carved);
        assert_eq!(filter.is_some(), section["enabled"].as_bool().unwrap());
        assert_eq!(
            create_carved_climate_filter::<&str>(HashSet::new()).is_some(),
            section["emptyEnabled"].as_bool().unwrap()
        );
        let filter = filter.expect("spoint enables the filter for river_bank cells");
        for c in section["cases"].as_array().unwrap() {
            let (cx, cz) = (c["cx"].as_i64().unwrap(), c["cz"].as_i64().unwrap());
            let out = filter(c["climate"].as_str(), cx, cz);
            assert_eq!(out, c["out"].as_str(), "climateFilter cx={cx} cz={cz}");
        }
    }

    #[test]
    fn spline_carve_layer_matches_spoint() {
        let fx = fixture();
        let section = &fx["splineCarveLayer"];
        let base = |x: f64, z: f64| if x < -20.0 { f64::NAN } else { 10.0 + 0.1 * x - 0.05 * z };
        let mut layer = SplineCarveLayer::default();
        for c in section["carves"].as_array().unwrap() {
            let control_points = points(&c["controlPoints"]);
            let o = &c["opts"];
            let opts = CarveOpts {
                seed: o["seed"].as_i64().unwrap() as i32,
                width_variance: num(o, "widthVariance"),
                bank_erosion: num(o, "bankErosion"),
            };
            let touched = layer.carve(
                &control_points,
                num(c, "width"),
                num(c, "depth"),
                c["kind"].as_str().unwrap(),
                Some(&base),
                opts,
            );
            assert_eq!(
                touched,
                c["touched"].as_u64().unwrap() as usize,
                "carve kind={} touched",
                c["kind"]
            );
        }
        assert_eq!(
            layer.cell_count(),
            section["cellCount"].as_u64().unwrap() as usize,
            "cellCount"
        );
        assert_eq!(
            layer.spline_count(),
            section["splineCount"].as_u64().unwrap() as usize,
            "splineCount"
        );
        for s in section["samples"].as_array().unwrap() {
            let (x, z) = (num(s, "x"), num(s, "z"));
            assert_close(layer.delta_at(x, z), num(s, "delta"), TOLERANCE, &format!("deltaAt x={x} z={z}"));
            assert_eq!(layer.kind_at(x, z), s["kind"].as_str(), "kindAt x={x} z={z}");
        }
        let empty = SplineCarveLayer::default();
        assert_close(empty.delta_at(0.0, 0.0), num(&section["empty"], "delta"), TOLERANCE, "empty deltaAt");
        assert_eq!(empty.kind_at(0.0, 0.0), section["empty"]["kind"].as_str(), "empty kindAt");
    }
}
