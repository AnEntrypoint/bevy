
use crate::brush_result::{brush_applied, brush_precondition, BrushOutcome, STRENGTH_REQUIRES};
use crate::brush_stroke::{height_stroke_replay, replay_strokes, HeightField};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::f64::consts::PI;

pub const CELL_M: f64 = 1.0;

pub const RAISE_REQUIRES: [&str; 4] = [
    "x is finite",
    "z is finite",
    "radius is finite and positive",
    "strength is finite and non-zero",
];
pub const FLATTEN_REQUIRES: [&str; 5] = [
    "baseHeightFn is a function",
    "x is finite",
    "z is finite",
    "radius is finite and positive",
    "targetHeight is finite",
];
pub const CARVE_REQUIRES: [&str; 6] = [
    "baseHeightFn is a function",
    "x is finite",
    "z is finite",
    "halfWidth is finite and positive",
    "halfDepth is finite and positive",
    "targetDepth is finite",
];
pub const SMOOTH_REQUIRES: [&str; 4] = [
    "x is finite",
    "z is finite",
    "radius is finite and positive",
    "strength is finite and positive",
];

const FLATTEN_FEATHER_FRACTION: f64 = 0.3;
const RECT_FEATHER_FRACTION: f64 = 0.15;
const SMOOTH_KERNEL_CELLS: i64 = 2;
const BRUSH_CONTRACT: &str = "brush contract holds for every call that reaches the heightfield";

#[derive(Default)]
pub struct HeightDelta {
    cells: HashMap<(i64, i64), f64>,
    strokes: Vec<Value>,
}

fn positive(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

fn clamp_strength(strength: f64) -> f64 {
    if strength.is_finite() { strength.clamp(0.0, 1.0) } else { 1.0 }
}

fn cell_span(center: f64, half: f64) -> std::ops::RangeInclusive<i64> {
    let lo = ((center - half) / CELL_M).floor() as i64;
    let hi = ((center + half) / CELL_M).ceil() as i64;
    lo..=hi
}

fn cell_center(cx: i64, cz: i64) -> (f64, f64) {
    (cx as f64 * CELL_M + CELL_M * 0.5, cz as f64 * CELL_M + CELL_M * 0.5)
}

impl HeightDelta {
    fn cell(&self, cx: i64, cz: i64) -> f64 {
        self.cells.get(&(cx, cz)).copied().unwrap_or(0.0)
    }

    pub fn raise(&mut self, x: f64, z: f64, radius: f64, strength: f64) -> Result<BrushOutcome<'static>, String> {
        let holds = [x.is_finite(), z.is_finite(), positive(radius), strength.is_finite() && strength != 0.0];
        if let Some(bad) = brush_precondition(&RAISE_REQUIRES, &holds)? {
            return Ok(bad);
        }
        let mut touched = 0.0;
        for cz in cell_span(z, radius) {
            for cx in cell_span(x, radius) {
                let (ccx, ccz) = cell_center(cx, cz);
                let d = (ccx - x).hypot(ccz - z);
                if d > radius {
                    continue;
                }
                let falloff = 0.5 * (1.0 + ((d / radius) * PI).cos());
                *self.cells.entry((cx, cz)).or_insert(0.0) += strength * falloff;
                touched += 1.0;
            }
        }
        let brush = if strength >= 0.0 { "raise" } else { "lower" };
        let stroke = json!({"x": x, "z": z, "radius": radius, "strength": strength, "brush": brush});
        brush_applied(&mut self.strokes, touched, Some(stroke))
    }

    pub fn flatten(
        &mut self,
        base: Option<&dyn Fn(f64, f64) -> f64>,
        x: f64,
        z: f64,
        radius: f64,
        target_height: f64,
        strength: f64,
    ) -> Result<BrushOutcome<'static>, String> {
        let holds = [base.is_some(), x.is_finite(), z.is_finite(), positive(radius), target_height.is_finite()];
        if let Some(bad) = brush_precondition(&FLATTEN_REQUIRES, &holds)? {
            return Ok(bad);
        }
        let Some(base) = base else {
            unreachable!("baseHeightFn checked above")
        };
        let clamped = clamp_strength(strength);
        if let Some(bad) = brush_precondition(&STRENGTH_REQUIRES, &[clamped > 0.0])? {
            return Ok(bad);
        }
        let plateau = radius * (1.0 - FLATTEN_FEATHER_FRACTION);
        let feather = radius - plateau;
        let mut touched = 0.0;
        for cz in cell_span(z, radius) {
            for cx in cell_span(x, radius) {
                let (ccx, ccz) = cell_center(cx, cz);
                let d = (ccx - x).hypot(ccz - z);
                if d > radius {
                    continue;
                }
                let base_height = base(ccx, ccz);
                if !base_height.is_finite() {
                    continue;
                }
                let target_delta = target_height - base_height;
                let current = self.cell(cx, cz);
                let falloff = if d <= plateau {
                    1.0
                } else {
                    0.5 * (1.0 + (((d - plateau) / feather) * PI).cos())
                };
                let t = clamped * falloff;
                self.cells.insert((cx, cz), current + (target_delta - current) * t);
                touched += 1.0;
            }
        }
        let stroke = json!({"x": x, "z": z, "radius": radius, "strength": clamped, "targetHeight": target_height, "brush": "flatten"});
        brush_applied(&mut self.strokes, touched, Some(stroke))
    }

    pub fn carve(
        &mut self,
        base: Option<&dyn Fn(f64, f64) -> f64>,
        x: f64,
        z: f64,
        half_width: f64,
        half_depth: f64,
        target_depth: f64,
        strength: f64,
    ) -> Result<BrushOutcome<'static>, String> {
        let holds = [
            base.is_some(),
            x.is_finite(),
            z.is_finite(),
            positive(half_width),
            positive(half_depth),
            target_depth.is_finite(),
        ];
        if let Some(bad) = brush_precondition(&CARVE_REQUIRES, &holds)? {
            return Ok(bad);
        }
        let Some(base) = base else {
            unreachable!("baseHeightFn checked above")
        };
        let clamped = clamp_strength(strength);
        if let Some(bad) = brush_precondition(&STRENGTH_REQUIRES, &[clamped > 0.0])? {
            return Ok(bad);
        }
        let plateau_w = half_width * (1.0 - RECT_FEATHER_FRACTION);
        let plateau_d = half_depth * (1.0 - RECT_FEATHER_FRACTION);
        let feather_w = half_width - plateau_w;
        let feather_d = half_depth - plateau_d;
        let mut touched = 0.0;
        for cz in cell_span(z, half_depth) {
            for cx in cell_span(x, half_width) {
                let (ccx, ccz) = cell_center(cx, cz);
                let dx = (ccx - x).abs();
                let dz = (ccz - z).abs();
                if dx > half_width || dz > half_depth {
                    continue;
                }
                let base_height = base(ccx, ccz);
                if !base_height.is_finite() {
                    continue;
                }
                let target_delta = target_depth - base_height;
                let current = self.cell(cx, cz);
                let falloff_x = if dx > plateau_w {
                    0.5 * (1.0 + (((dx - plateau_w) / feather_w) * PI).cos())
                } else {
                    1.0
                };
                let falloff_z = if dz > plateau_d {
                    0.5 * (1.0 + (((dz - plateau_d) / feather_d) * PI).cos())
                } else {
                    1.0
                };
                let t = clamped * (falloff_x * falloff_z);
                self.cells.insert((cx, cz), current + (target_delta - current) * t);
                touched += 1.0;
            }
        }
        let stroke = json!({
            "x": x,
            "z": z,
            "halfWidth": half_width,
            "halfDepth": half_depth,
            "targetDepth": target_depth,
            "strength": clamped,
            "brush": "rectangularCarve",
        });
        brush_applied(&mut self.strokes, touched, Some(stroke))
    }

    pub fn smooth(&mut self, x: f64, z: f64, radius: f64, strength: f64) -> Result<BrushOutcome<'static>, String> {
        let holds = [x.is_finite(), z.is_finite(), positive(radius), strength.is_finite() && strength > 0.0];
        if let Some(bad) = brush_precondition(&SMOOTH_REQUIRES, &holds)? {
            return Ok(bad);
        }
        let clamped = strength.min(1.0);
        let kernel = ((2 * SMOOTH_KERNEL_CELLS + 1) * (2 * SMOOTH_KERNEL_CELLS + 1)) as f64;
        let mut writes = Vec::new();
        let mut touched = 0.0;
        for cz in cell_span(z, radius) {
            for cx in cell_span(x, radius) {
                let (ccx, ccz) = cell_center(cx, cz);
                let d = (ccx - x).hypot(ccz - z);
                if d > radius {
                    continue;
                }
                let mut sum = 0.0;
                for kz in -SMOOTH_KERNEL_CELLS..=SMOOTH_KERNEL_CELLS {
                    for kx in -SMOOTH_KERNEL_CELLS..=SMOOTH_KERNEL_CELLS {
                        sum += self.cell(cx + kx, cz + kz);
                    }
                }
                let avg = sum / kernel;
                let current = self.cell(cx, cz);
                if avg == current {
                    continue;
                }
                let falloff = 0.5 * (1.0 + ((d / radius) * PI).cos());
                let t = clamped * falloff;
                writes.push(((cx, cz), current + (avg - current) * t));
                touched += 1.0;
            }
        }
        for (key, value) in writes {
            self.cells.insert(key, value);
        }
        let stroke = json!({"x": x, "z": z, "radius": radius, "strength": clamped, "brush": "smooth"});
        brush_applied(&mut self.strokes, touched, Some(stroke))
    }

    pub fn delta_at(&self, x: f64, z: f64) -> f64 {
        if self.cells.is_empty() {
            return 0.0;
        }
        let fx = x / CELL_M;
        let fz = z / CELL_M;
        let ix = fx.floor() as i64;
        let iz = fz.floor() as i64;
        let tx = fx - ix as f64;
        let tz = fz - iz as f64;
        let h00 = self.cell(ix, iz);
        let h10 = self.cell(ix + 1, iz);
        let h01 = self.cell(ix, iz + 1);
        let h11 = self.cell(ix + 1, iz + 1);
        if h00 == 0.0 && h10 == 0.0 && h01 == 0.0 && h11 == 0.0 {
            return 0.0;
        }
        (h00 * (1.0 - tx) + h10 * tx) * (1.0 - tz) + (h01 * (1.0 - tx) + h11 * tx) * tz
    }

    pub fn to_json(&self) -> Value {
        json!({"version": 1, "cellM": CELL_M, "strokes": self.strokes})
    }

    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    pub fn stroke_count(&self) -> usize {
        self.strokes.len()
    }
}

impl HeightField for HeightDelta {
    fn apply_smooth(&mut self, x: f64, z: f64, radius: f64, strength: f64) {
        self.smooth(x, z, radius, strength).expect(BRUSH_CONTRACT);
    }

    fn apply_flatten(&mut self, base: &dyn Fn(f64, f64) -> f64, x: f64, z: f64, radius: f64, target_height: f64, strength: f64) {
        self.flatten(Some(base), x, z, radius, target_height, strength).expect(BRUSH_CONTRACT);
    }

    fn apply_rectangular_carve(
        &mut self,
        base: &dyn Fn(f64, f64) -> f64,
        x: f64,
        z: f64,
        half_width: f64,
        half_depth: f64,
        target_depth: f64,
        strength: f64,
    ) {
        self.carve(Some(base), x, z, half_width, half_depth, target_depth, strength).expect(BRUSH_CONTRACT);
    }

    fn apply_raise(&mut self, x: f64, z: f64, radius: f64, strength: f64) {
        self.raise(x, z, radius, strength).expect(BRUSH_CONTRACT);
    }
}

pub fn create_height_delta(strokes: &Value, base: Option<&dyn Fn(f64, f64) -> f64>) -> HeightDelta {
    let mut hd = HeightDelta::default();
    {
        let mut replay = height_stroke_replay(&mut hd, base);
        replay_strokes(strokes, |stroke| Ok(replay(stroke))).expect("height stroke replay never fails");
    }
    hd
}

pub fn load_height_delta(json: &Value, base: Option<&dyn Fn(f64, f64) -> f64>) -> HeightDelta {
    create_height_delta(json.get("strokes").unwrap_or(&Value::Null), base)
}

mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn same(label: &str, got: &Value, want: &Value) {
        match (got, want) {
            (Value::Number(_), Value::Number(_)) => {
                let (g, w) = (got.as_f64().unwrap(), want.as_f64().unwrap());
                assert!((g - w).abs() <= 1e-9 * w.abs().max(1.0), "{label}: got {g} want {w}");
            }
            (Value::Array(g), Value::Array(w)) => {
                assert_eq!(g.len(), w.len(), "{label}: length");
                for (i, (g, w)) in g.iter().zip(w).enumerate() {
                    same(&format!("{label}[{i}]"), g, w);
                }
            }
            (Value::Object(g), Value::Object(w)) => {
                let got_keys: BTreeSet<&String> = g.keys().collect();
                let want_keys: BTreeSet<&String> = w.keys().collect();
                assert_eq!(got_keys, want_keys, "{label}: keys");
                for (key, want_value) in w {
                    same(&format!("{label}.{key}"), &g[key.as_str()], want_value);
                }
            }
            _ => assert_eq!(got, want, "{label}"),
        }
    }

    fn close(label: &str, got: f64, want: &Value) {
        let want = want.as_f64().unwrap();
        assert!((got - want).abs() <= 1e-9 * want.abs().max(1.0), "{label}: got {got} want {want}");
    }

    fn number(value: &Value, field: &str) -> f64 {
        match &value[field] {
            Value::Number(number) => number.as_f64().unwrap(),
            Value::String(text) => text.parse().unwrap(),
            other => panic!("{field} is not a number: {other}"),
        }
    }

    fn reference_base(x: f64, z: f64) -> f64 {
        if x > 16.0 {
            return f64::NAN;
        }
        2.0 + 0.5 * (0.3 * x).sin() + 0.25 * (0.4 * z).cos() + 0.005 * x * z
    }

    fn outcome_json(outcome: &BrushOutcome<'_>) -> Value {
        json!({
            "rejected": outcome.rejected,
            "requires": outcome.requires,
            "violates": outcome.violates,
            "touched": outcome.touched,
        })
    }

    fn check_state(label: &str, hd: &HeightDelta, want: &Value, points: &[Value]) {
        same(&format!("{label} cellCount"), &json!(hd.cell_count()), &want["cellCount"]);
        same(&format!("{label} strokeCount"), &json!(hd.stroke_count()), &want["strokeCount"]);
        same(&format!("{label} toJSON"), &hd.to_json(), &want["toJSON"]);
        let values = want["values"].as_array().unwrap();
        assert_eq!(points.len(), values.len(), "{label}: point count");
        for (point, want) in points.iter().zip(values) {
            let (x, z) = (number(point, "x"), number(point, "z"));
            close(&format!("{label} at x={x} z={z}"), hd.delta_at(x, z), want);
        }
    }

    #[test]
    fn height_delta_matches_spoint() {
        let fixture: Value = serde_json::from_str(include_str!("../testdata/parity/spoint-height-delta.json")).unwrap();
        same("cellM", &json!(CELL_M), &fixture["cellM"]);
        same(
            "requires",
            &json!({
                "raise": RAISE_REQUIRES,
                "flatten": FLATTEN_REQUIRES,
                "rectangularCarve": CARVE_REQUIRES,
                "smooth": SMOOTH_REQUIRES,
            }),
            &fixture["requires"],
        );
        let points = fixture["points"].as_array().unwrap();
        assert!(!points.is_empty());
        let base_values = fixture["baseValues"].as_array().unwrap();
        assert_eq!(points.len(), base_values.len());
        for (i, (point, want)) in points.iter().zip(base_values).enumerate() {
            let got = reference_base(number(point, "x"), number(point, "z"));
            if want.is_null() {
                assert!(!got.is_finite(), "baseValues[{i}]: got {got} want null");
            } else {
                close(&format!("baseValues[{i}]"), got, want);
            }
        }

        let reference: &dyn Fn(f64, f64) -> f64 = &reference_base;
        let mut hd = HeightDelta::default();
        let plan = fixture["plan"].as_array().unwrap();
        assert!(!plan.is_empty());
        for (i, record) in plan.iter().enumerate() {
            let step = &record["step"];
            let op = step["op"].as_str().unwrap();
            let base = if step["useBase"].as_bool().unwrap() { Some(reference) } else { None };
            let (x, z, strength) = (number(step, "x"), number(step, "z"), number(step, "strength"));
            let outcome = match op {
                "raise" => hd.raise(x, z, number(step, "radius"), strength),
                "smooth" => hd.smooth(x, z, number(step, "radius"), strength),
                "flatten" => hd.flatten(base, x, z, number(step, "radius"), number(step, "targetHeight"), strength),
                "rectangularCarve" => hd.carve(
                    base,
                    x,
                    z,
                    number(step, "halfWidth"),
                    number(step, "halfDepth"),
                    number(step, "targetDepth"),
                    strength,
                ),
                other => panic!("unknown op {other}"),
            };
            same(&format!("plan[{i}] {op} result"), &outcome_json(&outcome.unwrap()), &record["result"]);
            same(&format!("plan[{i}] {op} cellCount"), &json!(hd.cell_count()), &record["cellCount"]);
            same(&format!("plan[{i}] {op} strokeCount"), &json!(hd.stroke_count()), &record["strokeCount"]);
        }
        check_state("final", &hd, &fixture["final"], points);

        let reloaded = load_height_delta(&fixture["final"]["toJSON"], Some(reference));
        check_state("roundTrip", &reloaded, &fixture["roundTrip"], points);

        let cases = fixture["loadCases"].as_array().unwrap();
        assert!(!cases.is_empty());
        for case in cases {
            let name = case["name"].as_str().unwrap();
            let base = if case["useBase"].as_bool().unwrap() { Some(reference) } else { None };
            let loaded = load_height_delta(&case["json"], base);
            check_state(&format!("loadCase {name}"), &loaded, case, points);
        }
    }
}
