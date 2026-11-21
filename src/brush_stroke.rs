
use serde_json::Value;

const STROKE_BASE_FIELDS: [&str; 3] = ["x", "z", "strength"];
pub const STROKE_KINDS: [(&str, &[&str]); 5] = [
    ("raise", &["radius"]),
    ("lower", &["radius"]),
    ("smooth", &["radius"]),
    ("flatten", &["radius", "targetHeight"]),
    ("rectangularCarve", &["halfWidth", "halfDepth", "targetDepth"]),
];
pub const BIOME_STROKE_FIELDS: [&str; 4] = ["x", "z", "radius", "strength"];
pub const BIOME_TARGET_FIELDS: [&str; 3] = ["temp", "humidity", "erosion"];

pub fn number_field(value: &Value, field: &str) -> f64 {
    value.get(field).and_then(Value::as_f64).unwrap_or(f64::NAN)
}

pub fn fields_finite(value: &Value, fields: &[&str]) -> bool {
    fields.iter().all(|field| number_field(value, field).is_finite())
}

pub fn height_stroke_is_loadable(stroke: &Value) -> bool {
    if !stroke.is_object() {
        return false;
    }
    let Some(brush) = stroke.get("brush").and_then(Value::as_str) else {
        return false;
    };
    let Some((_, kind_fields)) = STROKE_KINDS.iter().find(|(name, _)| *name == brush) else {
        return false;
    };
    fields_finite(stroke, &STROKE_BASE_FIELDS) && fields_finite(stroke, kind_fields)
}

pub fn biome_stroke_is_loadable(stroke: &Value) -> bool {
    stroke.is_object()
        && fields_finite(stroke, &BIOME_STROKE_FIELDS)
        && stroke
            .get("target")
            .is_some_and(|target| fields_finite(target, &BIOME_TARGET_FIELDS))
}

pub trait HeightField {
    fn apply_smooth(&mut self, x: f64, z: f64, radius: f64, strength: f64);
    fn apply_flatten(&mut self, base: &dyn Fn(f64, f64) -> f64, x: f64, z: f64, radius: f64, target_height: f64, strength: f64);
    fn apply_rectangular_carve(
        &mut self,
        base: &dyn Fn(f64, f64) -> f64,
        x: f64,
        z: f64,
        half_width: f64,
        half_depth: f64,
        target_depth: f64,
        strength: f64,
    );
    fn apply_raise(&mut self, x: f64, z: f64, radius: f64, strength: f64);
}

pub trait BiomePaint {
    fn paint(&mut self, x: f64, z: f64, radius: f64, target: &Value, strength: f64) -> Result<(), String>;
}

pub fn height_stroke_replay<'a>(hd: &'a mut dyn HeightField, base: Option<&'a dyn Fn(f64, f64) -> f64>) -> impl FnMut(&Value) -> bool + 'a {
    move |stroke: &Value| {
        if !height_stroke_is_loadable(stroke) {
            return false;
        }
        let x = number_field(stroke, "x");
        let z = number_field(stroke, "z");
        let radius = number_field(stroke, "radius");
        let strength = number_field(stroke, "strength");
        match stroke.get("brush").and_then(Value::as_str) {
            Some("smooth") => hd.apply_smooth(x, z, radius, strength),
            Some("flatten") => match base {
                Some(base) => hd.apply_flatten(base, x, z, radius, number_field(stroke, "targetHeight"), strength),
                None => return false,
            },
            Some("rectangularCarve") => match base {
                Some(base) => hd.apply_rectangular_carve(
                    base,
                    x,
                    z,
                    number_field(stroke, "halfWidth"),
                    number_field(stroke, "halfDepth"),
                    number_field(stroke, "targetDepth"),
                    strength,
                ),
                None => return false,
            },
            _ => hd.apply_raise(x, z, radius, strength),
        }
        true
    }
}

pub fn biome_stroke_replay<'a>(bo: &'a mut dyn BiomePaint) -> impl FnMut(&Value) -> Result<bool, String> + 'a {
    move |stroke: &Value| {
        if !biome_stroke_is_loadable(stroke) {
            return Ok(false);
        }
        let target = stroke.get("target").unwrap_or(&Value::Null);
        bo.paint(
            number_field(stroke, "x"),
            number_field(stroke, "z"),
            number_field(stroke, "radius"),
            target,
            number_field(stroke, "strength"),
        )?;
        Ok(true)
    }
}

pub fn replay_strokes(strokes: &Value, mut replay_one: impl FnMut(&Value) -> Result<bool, String>) -> Result<usize, String> {
    let Some(items) = strokes.as_array() else {
        return Ok(0);
    };
    let mut replayed = 0;
    for stroke in items {
        if replay_one(stroke)? {
            replayed += 1;
        }
    }
    Ok(replayed)
}

mod tests {
    use super::*;
    use serde_json::json;
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

    #[derive(Default)]
    struct RecordingHeightField {
        calls: Vec<Value>,
    }

    impl HeightField for RecordingHeightField {
        fn apply_smooth(&mut self, x: f64, z: f64, radius: f64, strength: f64) {
            self.calls.push(json!({"call": "applySmoothBrush", "args": [x, z, radius, strength]}));
        }
        fn apply_flatten(&mut self, _base: &dyn Fn(f64, f64) -> f64, x: f64, z: f64, radius: f64, target_height: f64, strength: f64) {
            self.calls.push(json!({"call": "applyFlattenBrush", "args": [x, z, radius, target_height, strength]}));
        }
        fn apply_rectangular_carve(
            &mut self,
            _base: &dyn Fn(f64, f64) -> f64,
            x: f64,
            z: f64,
            half_width: f64,
            half_depth: f64,
            target_depth: f64,
            strength: f64,
        ) {
            self.calls.push(json!({"call": "applyRectangularCarve", "args": [x, z, half_width, half_depth, target_depth, strength]}));
        }
        fn apply_raise(&mut self, x: f64, z: f64, radius: f64, strength: f64) {
            self.calls.push(json!({"call": "applyRaiseBrush", "args": [x, z, radius, strength]}));
        }
    }

    #[derive(Default)]
    struct RecordingBiomePaint {
        calls: Vec<Value>,
    }

    impl BiomePaint for RecordingBiomePaint {
        fn paint(&mut self, x: f64, z: f64, radius: f64, target: &Value, strength: f64) -> Result<(), String> {
            self.calls.push(json!({"call": "applyPaintBrush", "args": [x, z, radius, target, strength]}));
            Ok(())
        }
    }

    fn replay_height(mixed: &Value, base: Option<&dyn Fn(f64, f64) -> f64>) -> (usize, Vec<Value>) {
        let mut hd = RecordingHeightField::default();
        let mut replay = height_stroke_replay(&mut hd, base);
        let count = replay_strokes(mixed, |stroke| Ok(replay(stroke))).unwrap();
        drop(replay);
        (count, hd.calls)
    }

    #[test]
    fn brush_stroke_loadability_matches_spoint() {
        let fixture: Value = serde_json::from_str(include_str!("../testdata/parity/spoint-brush-stroke.json")).unwrap();
        assert_eq!(fixture["strokeKinds"].as_object().unwrap().len(), STROKE_KINDS.len());
        for (brush, fields) in STROKE_KINDS {
            same(&format!("strokeKinds.{brush}"), &json!(fields), &fixture["strokeKinds"][brush]);
        }
        same("biomeStrokeFields", &json!(BIOME_STROKE_FIELDS), &fixture["biomeStrokeFields"]);
        let height = fixture["loadability"]["height"].as_array().unwrap();
        assert!(!height.is_empty());
        for case in height {
            let name = case["name"].as_str().unwrap();
            assert_eq!(height_stroke_is_loadable(&case["input"]), case["result"].as_bool().unwrap(), "height {name}");
        }
        let biome = fixture["loadability"]["biome"].as_array().unwrap();
        assert!(!biome.is_empty());
        for case in biome {
            let name = case["name"].as_str().unwrap();
            assert_eq!(biome_stroke_is_loadable(&case["input"]), case["result"].as_bool().unwrap(), "biome {name}");
        }
    }

    #[test]
    fn brush_stroke_replay_matches_spoint() {
        let fixture: Value = serde_json::from_str(include_str!("../testdata/parity/spoint-brush-stroke.json")).unwrap();
        let replay = &fixture["replay"];
        let mixed = &replay["mixed"];
        let base_fn = |x: f64, _z: f64| 0.25 + 0.1 * x;
        let base: &dyn Fn(f64, f64) -> f64 = &base_fn;

        let (count, calls) = replay_height(mixed, Some(base));
        same("heightWithBase.count", &json!(count), &replay["heightWithBase"]["count"]);
        same("heightWithBase.calls", &Value::Array(calls), &replay["heightWithBase"]["calls"]);

        let (count, calls) = replay_height(mixed, None);
        same("heightWithoutBase.count", &json!(count), &replay["heightWithoutBase"]["count"]);
        same("heightWithoutBase.calls", &Value::Array(calls), &replay["heightWithoutBase"]["calls"]);

        let mut bo = RecordingBiomePaint::default();
        let count = replay_strokes(mixed, biome_stroke_replay(&mut bo)).unwrap();
        same("biome.count", &json!(count), &replay["biome"]["count"]);
        same("biome.calls", &Value::Array(bo.calls), &replay["biome"]["calls"]);

        same("nonArray.nullInput", &json!(replay_strokes(&Value::Null, |_| Ok(true)).unwrap()), &replay["nonArray"]["nullInput"]);
        same("nonArray.stringInput", &json!(replay_strokes(&json!("nope"), |_| Ok(true)).unwrap()), &replay["nonArray"]["stringInput"]);
    }
}
