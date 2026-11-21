#![cfg_attr(not(test), allow(dead_code))]

pub const STRENGTH_REQUIRES: [&str; 1] = ["strength clamps above zero"];

#[derive(Clone, Debug, PartialEq)]
pub struct BrushOutcome<'a> {
    pub rejected: bool,
    pub requires: Option<Vec<&'a str>>,
    pub violates: Option<&'a str>,
    pub touched: u64,
}

pub fn brush_precondition<'a>(requires: &[&'a str], holds: &[bool]) -> Result<Option<BrushOutcome<'a>>, String> {
    if holds.len() != requires.len() {
        return Err(format!("[terrain] brush contract: {} checks for {} clauses", holds.len(), requires.len()));
    }
    for (clause, held) in requires.iter().zip(holds) {
        if !*held {
            return Ok(Some(BrushOutcome {
                rejected: true,
                requires: Some(requires.to_vec()),
                violates: Some(*clause),
                touched: 0,
            }));
        }
    }
    Ok(None)
}

pub fn brush_applied<T>(strokes: &mut Vec<T>, touched: f64, stroke: Option<T>) -> Result<BrushOutcome<'static>, String> {
    if !touched.is_finite() || touched.fract() != 0.0 || touched < 0.0 {
        return Err(format!("[terrain] brush postcondition: touched {touched} is not a whole non-negative cell count"));
    }
    if touched > 0.0 {
        let Some(stroke) = stroke else {
            return Err(format!("[terrain] brush postcondition: {touched} cells touched with no stroke to record"));
        };
        strokes.push(stroke);
    }
    Ok(BrushOutcome {
        rejected: false,
        requires: None,
        violates: None,
        touched: touched as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
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

    fn outcome_json(outcome: &BrushOutcome<'_>) -> Value {
        json!({
            "rejected": outcome.rejected,
            "requires": outcome.requires,
            "violates": outcome.violates,
            "touched": outcome.touched,
        })
    }

    fn precondition_json(result: Result<Option<BrushOutcome<'_>>, String>) -> Value {
        match result {
            Ok(outcome) => json!({"threw": false, "value": outcome.as_ref().map(outcome_json)}),
            Err(message) => json!({"threw": true, "message": message}),
        }
    }

    fn applied_json(result: Result<BrushOutcome<'_>, String>) -> Value {
        match result {
            Ok(outcome) => json!({"threw": false, "value": outcome_json(&outcome)}),
            Err(message) => json!({"threw": true, "message": message}),
        }
    }

    #[test]
    fn brush_result_matches_spoint() {
        let fixture: Value = serde_json::from_str(include_str!("../testdata/parity/spoint-brush-result.json")).unwrap();
        same("strengthRequires", &json!(STRENGTH_REQUIRES), &fixture["strengthRequires"]);
        let preconditions = fixture["preconditions"].as_array().unwrap();
        assert!(!preconditions.is_empty());
        for case in preconditions {
            let name = case["name"].as_str().unwrap();
            let requires: Vec<&str> = case["input"]["requires"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
            let holds: Vec<bool> = case["input"]["holds"].as_array().unwrap().iter().map(|v| v.as_bool().unwrap()).collect();
            same(&format!("precondition {name}"), &precondition_json(brush_precondition(&requires, &holds)), &case["result"]);
        }
        let applied = fixture["applied"].as_array().unwrap();
        assert!(!applied.is_empty());
        for case in applied {
            let name = case["name"].as_str().unwrap();
            let mut strokes: Vec<String> = case["input"]["strokes"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
            let touched = case["input"]["touched"].as_f64().unwrap();
            let stroke = case["input"]["stroke"].as_str().map(str::to_string);
            same(&format!("applied {name}: result"), &applied_json(brush_applied(&mut strokes, touched, stroke)), &case["result"]);
            same(&format!("applied {name}: strokes"), &json!(strokes), &case["strokesAfter"]);
        }
    }
}
