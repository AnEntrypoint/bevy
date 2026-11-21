#![cfg_attr(not(test), allow(dead_code))]

use std::collections::HashMap;
use std::f64::consts::PI;

use serde_json::{json, Value};

use crate::brush_result::{brush_applied, brush_precondition, BrushOutcome, STRENGTH_REQUIRES};
use crate::brush_stroke::{biome_stroke_replay, fields_finite, number_field, replay_strokes, BiomePaint, BIOME_TARGET_FIELDS};

pub const CELL_M: f64 = 8.0;

pub const PAINT_REQUIRES: [&str; 4] = [
    "x is finite",
    "z is finite",
    "radius is finite and positive",
    "target carries a finite temp, humidity and erosion",
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BiomeTarget {
    pub temp: f64,
    pub humidity: f64,
    pub erosion: f64,
}

pub const BIOME_PRESETS: [(&str, BiomeTarget); 5] = [
    ("desert", BiomeTarget { temp: 0.85, humidity: 0.10, erosion: 0.55 }),
    ("tundra", BiomeTarget { temp: 0.10, humidity: 0.45, erosion: 0.35 }),
    ("forest", BiomeTarget { temp: 0.55, humidity: 0.75, erosion: 0.25 }),
    ("grassland", BiomeTarget { temp: 0.55, humidity: 0.40, erosion: 0.30 }),
    ("wetland", BiomeTarget { temp: 0.60, humidity: 0.95, erosion: 0.15 }),
];

pub fn biome_names() -> [&'static str; 5] {
    BIOME_PRESETS.map(|(name, _)| name)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintedCell {
    pub temp: f64,
    pub humidity: f64,
    pub erosion: f64,
    pub w: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PaintStroke {
    pub x: f64,
    pub z: f64,
    pub radius: f64,
    pub strength: f64,
    pub target: BiomeTarget,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClimateSample {
    pub temp: f64,
    pub humidity: f64,
    pub erosion: f64,
    pub sea_bias: f64,
}

pub trait ClimateField {
    fn climate_at_local(&self, x: f64, z: f64, dir: &Value) -> Option<ClimateSample>;
    fn climate_uses_local_xz(&self) -> bool;
}

#[derive(Default)]
pub struct BiomeOverride {
    cells: HashMap<i64, PaintedCell>,
    applied: Vec<PaintStroke>,
}

pub struct WrappedClimate<'a> {
    cells: &'a BiomeOverride,
    base: &'a dyn ClimateField,
}

pub(crate) fn cell_key(cx: i64, cz: i64) -> i64 {
    const KEY_AXIS_SPAN: i64 = 1 << 23;
    const KEY_AXIS_OFFSET: i64 = KEY_AXIS_SPAN >> 1;
    (cx + KEY_AXIS_OFFSET) * KEY_AXIS_SPAN + (cz + KEY_AXIS_OFFSET)
}

fn clamp01(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

impl BiomeOverride {
    pub fn apply_paint_brush(&mut self, x: f64, z: f64, radius: f64, target: &Value, strength: f64) -> Result<BrushOutcome<'static>, String> {
        let holds = [
            x.is_finite(),
            z.is_finite(),
            radius.is_finite() && radius > 0.0,
            fields_finite(target, &BIOME_TARGET_FIELDS),
        ];
        if let Some(rejection) = brush_precondition(&PAINT_REQUIRES, &holds)? {
            return Ok(rejection);
        }
        let clamped = if strength.is_finite() { strength.clamp(0.0, 1.0) } else { 1.0 };
        if let Some(weak) = brush_precondition(&STRENGTH_REQUIRES, &[clamped > 0.0])? {
            return Ok(weak);
        }
        let tgt = BiomeTarget {
            temp: clamp01(number_field(target, "temp")),
            humidity: clamp01(number_field(target, "humidity")),
            erosion: clamp01(number_field(target, "erosion")),
        };
        let cx0 = ((x - radius) / CELL_M).floor() as i64;
        let cx1 = ((x + radius) / CELL_M).ceil() as i64;
        let cz0 = ((z - radius) / CELL_M).floor() as i64;
        let cz1 = ((z + radius) / CELL_M).ceil() as i64;
        let mut touched = 0u64;
        for cz in cz0..=cz1 {
            for cx in cx0..=cx1 {
                let center_x = cx as f64 * CELL_M + CELL_M * 0.5;
                let center_z = cz as f64 * CELL_M + CELL_M * 0.5;
                let distance = (center_x - x).hypot(center_z - z);
                if distance > radius {
                    continue;
                }
                let falloff = 0.5 * (1.0 + ((distance / radius) * PI).cos());
                let t = clamped * falloff;
                if t <= 0.0 {
                    continue;
                }
                let key = cell_key(cx, cz);
                let painted = match self.cells.get(&key) {
                    None => PaintedCell {
                        temp: tgt.temp,
                        humidity: tgt.humidity,
                        erosion: tgt.erosion,
                        w: t,
                    },
                    Some(prev) => {
                        let next_w = (prev.w + t * (1.0 - prev.w)).min(1.0);
                        let mix = if next_w > 0.0 { t / next_w } else { 0.0 };
                        PaintedCell {
                            temp: prev.temp + (tgt.temp - prev.temp) * mix,
                            humidity: prev.humidity + (tgt.humidity - prev.humidity) * mix,
                            erosion: prev.erosion + (tgt.erosion - prev.erosion) * mix,
                            w: next_w,
                        }
                    }
                };
                self.cells.insert(key, painted);
                touched += 1;
            }
        }
        brush_applied(
            &mut self.applied,
            touched as f64,
            Some(PaintStroke {
                x,
                z,
                radius,
                strength: clamped,
                target: tgt,
            }),
        )
    }

    pub fn override_at(&self, x: f64, z: f64) -> Option<PaintedCell> {
        if self.cells.is_empty() || !x.is_finite() || !z.is_finite() {
            return None;
        }
        let key = cell_key((x / CELL_M).floor() as i64, (z / CELL_M).floor() as i64);
        self.cells.get(&key).copied()
    }

    pub fn wrap_climate_field<'a>(&'a self, base: &'a dyn ClimateField) -> WrappedClimate<'a> {
        WrappedClimate { cells: self, base }
    }

    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    pub fn stroke_count(&self) -> usize {
        self.applied.len()
    }

    pub fn clear(&mut self) {
        self.cells.clear();
        self.applied.clear();
    }

    pub fn to_json(&self) -> Value {
        json!({
            "version": 1,
            "cellM": CELL_M,
            "strokes": self.applied.iter().map(stroke_json).collect::<Vec<Value>>(),
        })
    }
}

fn stroke_json(stroke: &PaintStroke) -> Value {
    json!({
        "x": stroke.x,
        "z": stroke.z,
        "radius": stroke.radius,
        "strength": stroke.strength,
        "target": {"temp": stroke.target.temp, "humidity": stroke.target.humidity, "erosion": stroke.target.erosion},
    })
}

impl BiomePaint for BiomeOverride {
    fn paint(&mut self, x: f64, z: f64, radius: f64, target: &Value, strength: f64) -> Result<(), String> {
        self.apply_paint_brush(x, z, radius, target, strength).map(|_| ())
    }
}

impl ClimateField for WrappedClimate<'_> {
    fn climate_at_local(&self, x: f64, z: f64, dir: &Value) -> Option<ClimateSample> {
        let base = self.base.climate_at_local(x, z, dir);
        let Some(over) = self.cells.override_at(x, z) else {
            return base;
        };
        Some(match base {
            None => ClimateSample {
                temp: over.temp,
                humidity: over.humidity,
                erosion: over.erosion,
                sea_bias: 0.0,
            },
            Some(base) => ClimateSample {
                temp: base.temp + (over.temp - base.temp) * over.w,
                humidity: base.humidity + (over.humidity - base.humidity) * over.w,
                erosion: base.erosion + (over.erosion - base.erosion) * over.w,
                sea_bias: base.sea_bias,
            },
        })
    }

    fn climate_uses_local_xz(&self) -> bool {
        !self.cells.cells.is_empty() || self.base.climate_uses_local_xz()
    }
}

pub fn create_biome_override(strokes: &Value) -> Result<BiomeOverride, String> {
    let mut bo = BiomeOverride::default();
    replay_strokes(strokes, biome_stroke_replay(&mut bo))?;
    Ok(bo)
}

pub fn load_biome_override(json: &Value) -> Result<BiomeOverride, String> {
    create_biome_override(json.get("strokes").unwrap_or(&Value::Null))
}

#[cfg(test)]
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

    struct StubBase;

    impl ClimateField for StubBase {
        fn climate_at_local(&self, x: f64, z: f64, _dir: &Value) -> Option<ClimateSample> {
            if z < -30.0 {
                return None;
            }
            Some(ClimateSample {
                temp: 0.3 + 0.002 * x,
                humidity: 0.5 - 0.001 * z,
                erosion: 0.4,
                sea_bias: 0.25,
            })
        }

        fn climate_uses_local_xz(&self) -> bool {
            true
        }
    }

    fn number_at(value: &Value) -> f64 {
        value.as_f64().unwrap()
    }

    fn over_points(points: &Value, mut sample: impl FnMut(f64, f64) -> Value) -> Value {
        Value::Array(points.as_array().unwrap().iter().map(|p| sample(number_at(&p[0]), number_at(&p[1]))).collect())
    }

    fn cell_json(cell: Option<PaintedCell>) -> Value {
        match cell {
            None => Value::Null,
            Some(c) => json!({"temp": c.temp, "humidity": c.humidity, "erosion": c.erosion, "w": c.w}),
        }
    }

    fn climate_json(sample: Option<ClimateSample>) -> Value {
        match sample {
            None => Value::Null,
            Some(c) => json!({"temp": c.temp, "humidity": c.humidity, "erosion": c.erosion, "seaBias": c.sea_bias}),
        }
    }

    fn target_json(target: &BiomeTarget) -> Value {
        json!({"temp": target.temp, "humidity": target.humidity, "erosion": target.erosion})
    }

    fn outcome_json(outcome: &BrushOutcome<'_>) -> Value {
        json!({
            "rejected": outcome.rejected,
            "requires": outcome.requires,
            "violates": outcome.violates,
            "touched": outcome.touched,
        })
    }

    fn outcome_result_json(result: Result<BrushOutcome<'_>, String>) -> Value {
        match result {
            Ok(outcome) => outcome_json(&outcome),
            Err(message) => json!({"threw": true, "message": message}),
        }
    }

    #[test]
    fn biome_override_paint_matches_spoint() {
        let fixture: Value = serde_json::from_str(include_str!("../testdata/parity/spoint-biome-override.json")).unwrap();
        let points = &fixture["samplePoints"];
        let paint = &fixture["paint"];
        same("cellM", &json!(CELL_M), &fixture["cellM"]);
        same("paintRequires", &json!(PAINT_REQUIRES), &fixture["paintRequires"]);

        let painted = create_biome_override(&fixture["paints"]).unwrap();
        same("paint.cellCount", &json!(painted.cell_count()), &paint["cellCount"]);
        same("paint.strokeCount", &json!(painted.stroke_count()), &paint["strokeCount"]);
        same("paint.overrideAt", &over_points(points, |x, z| cell_json(painted.override_at(x, z))), &paint["overrideAt"]);
        same("paint.toJSON", &painted.to_json(), &paint["toJSON"]);

        let base = StubBase;
        let wrapped = painted.wrap_climate_field(&base);
        same("paint.wrap.climateUsesLocalXZ", &json!(wrapped.climate_uses_local_xz()), &paint["wrap"]["climateUsesLocalXZ"]);
        same(
            "paint.wrap.samples",
            &over_points(points, |x, z| climate_json(wrapped.climate_at_local(x, z, &Value::Null))),
            &paint["wrap"]["samples"],
        );

        let empty = create_biome_override(&json!([])).unwrap();
        let empty_wrapped = empty.wrap_climate_field(&base);
        same("paint.emptyWrap.climateUsesLocalXZ", &json!(empty_wrapped.climate_uses_local_xz()), &paint["emptyWrap"]["climateUsesLocalXZ"]);
        same(
            "paint.emptyWrap.samples",
            &over_points(points, |x, z| climate_json(empty_wrapped.climate_at_local(x, z, &Value::Null))),
            &paint["emptyWrap"]["samples"],
        );

        let mut direct = BiomeOverride::default();
        for call in paint["direct"]["calls"].as_array().unwrap() {
            let name = call["name"].as_str().unwrap();
            let args = call["args"].as_array().unwrap();
            let result = direct.apply_paint_brush(number_at(&args[0]), number_at(&args[1]), number_at(&args[2]), &args[3], number_at(&args[4]));
            same(&format!("paint.direct {name}"), &outcome_result_json(result), &call["result"]);
        }
        same(
            "paint.direct.after",
            &json!({"cellCount": direct.cell_count(), "strokeCount": direct.stroke_count()}),
            &paint["direct"]["after"],
        );
        direct.clear();
        same(
            "paint.direct.cleared",
            &json!({"cellCount": direct.cell_count(), "strokeCount": direct.stroke_count(), "overrideAt": cell_json(direct.override_at(0.0, 0.0))}),
            &paint["direct"]["cleared"],
        );
    }

    #[test]
    fn biome_override_load_matches_spoint() {
        let fixture: Value = serde_json::from_str(include_str!("../testdata/parity/spoint-biome-override.json")).unwrap();
        let points = &fixture["samplePoints"];
        let load = &fixture["load"];

        let presets = Value::Object(BIOME_PRESETS.iter().map(|(name, target)| (name.to_string(), target_json(target))).collect());
        same("presets", &presets, &fixture["presets"]);
        same("names", &json!(biome_names()), &fixture["names"]);

        let loaded = load_biome_override(&load["input"]).unwrap();
        same("load.cellCount", &json!(loaded.cell_count()), &load["cellCount"]);
        same("load.strokeCount", &json!(loaded.stroke_count()), &load["strokeCount"]);
        same("load.overrideAt", &over_points(points, |x, z| cell_json(loaded.override_at(x, z))), &load["overrideAt"]);

        let round_trip = load_biome_override(&loaded.to_json()).unwrap();
        same(
            "load.roundTrip",
            &json!({"cellCount": round_trip.cell_count(), "strokeCount": round_trip.stroke_count()}),
            &load["roundTrip"],
        );

        let null_loaded = load_biome_override(&Value::Null).unwrap();
        same(
            "load.nullInput",
            &json!({"cellCount": null_loaded.cell_count(), "strokeCount": null_loaded.stroke_count(), "overrideAt": over_points(points, |x, z| cell_json(null_loaded.override_at(x, z)))}),
            &load["nullInput"],
        );

        let text_loaded = load_biome_override(&json!({"strokes": "nope"})).unwrap();
        same(
            "load.nonArrayStrokes",
            &json!({"cellCount": text_loaded.cell_count(), "strokeCount": text_loaded.stroke_count()}),
            &load["nonArrayStrokes"],
        );
    }
}
