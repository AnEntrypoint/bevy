#![cfg_attr(not(test), allow(dead_code))]

use std::f64::consts::PI;

use serde_json::Value;

use crate::height::ops_jsgen::V3;

pub const DEFAULT_MAX_DISTANCE: f64 = 3.0;
pub const DEFAULT_MAX_ANGLE_RAD: f64 = PI / 6.0;
pub const DEFAULT_LABEL: &str = "Press E";
pub const NO_TARGET: &str = "press-no-target";

#[derive(Clone, Debug, PartialEq)]
pub struct PressRefusal {
    pub reason_code: &'static str,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PressTarget {
    pub id: String,
    pub distance: f64,
    pub angle: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PressRoute {
    pub label: String,
    pub target: Option<PressTarget>,
}

fn refusal(module: &str, reason_code: &'static str, message: &str) -> PressRefusal {
    PressRefusal { reason_code, message: format!("[{module}] {reason_code}: {message}") }
}

fn read_vec3(value: Option<&Value>, name: &str) -> Result<V3, PressRefusal> {
    let parsed = match value.and_then(Value::as_array).map(Vec::as_slice) {
        Some([x, y, z]) => x.as_f64().zip(y.as_f64()).zip(z.as_f64()).map(|((x, y), z)| [x, y, z]),
        _ => None,
    };
    parsed.ok_or_else(|| refusal("vec3", "vec3-invalid", &format!("{name} must be an array of three finite numbers")))
}

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn length(a: V3) -> f64 {
    dot(a, a).sqrt()
}

fn normalize(a: V3) -> Result<V3, PressRefusal> {
    let len = length(a);
    if len == 0.0 {
        return Err(refusal("vec3", "vec3-zero-length", "cannot normalize a zero vector"));
    }
    let k = 1.0 / len;
    Ok([a[0] * k, a[1] * k, a[2] * k])
}

pub fn route_press(args: &Value) -> Result<PressRoute, PressRefusal> {
    let objects = args
        .get("objects")
        .and_then(Value::as_array)
        .ok_or_else(|| refusal("press-router", "press-objects-invalid", "objects must be an array"))?;
    let origin = read_vec3(args.get("origin"), "origin")?;
    let forward = read_vec3(args.get("forward"), "forward")?;
    let max_distance = match args.get("maxDistance") {
        None => DEFAULT_MAX_DISTANCE,
        Some(v) => v.as_f64().filter(|d| *d > 0.0).ok_or_else(|| {
            refusal("press-router", "press-distance-invalid", "maxDistance must be a positive finite number")
        })?,
    };
    let max_angle = match args.get("maxAngleRad") {
        None => DEFAULT_MAX_ANGLE_RAD,
        Some(v) => v.as_f64().filter(|a| (0.0..=PI).contains(a)).ok_or_else(|| {
            refusal("press-router", "press-angle-invalid", "maxAngleRad must be finite in [0,PI]")
        })?,
    };
    let label = args.get("label").and_then(Value::as_str).unwrap_or(DEFAULT_LABEL).to_string();
    let facing = normalize(forward)?;
    let mut best: Option<PressTarget> = None;
    for (i, obj) in objects.iter().enumerate() {
        let id = obj
            .get("id")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| refusal("press-router", "press-object-invalid", &format!("objects[{i}] needs a string id")))?;
        let position = read_vec3(obj.get("position"), &format!("objects[{i}].position"))?;
        let delta = sub(position, origin);
        let distance = length(delta);
        if distance > max_distance || distance == 0.0 {
            continue;
        }
        let cos = dot(normalize(delta)?, facing);
        let angle = cos.clamp(-1.0, 1.0).acos();
        if angle > max_angle {
            continue;
        }
        if best.as_ref().is_none_or(|b| distance < b.distance) {
            best = Some(PressTarget { id: id.to_string(), distance, angle });
        }
    }
    Ok(PressRoute { label, target: best })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn close(got: f64, want: &Value, label: &str) {
        let want = want.as_f64().unwrap();
        assert!((got - want).abs() < 1e-6, "{label}: bevy {got}, spoint {want}");
    }

    #[test]
    fn press_router_matches_spoint() {
        let f: Value = serde_json::from_str(include_str!("../testdata/parity/spoint-press-router.json")).unwrap();
        for case in f["cases"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let want = &case["expect"];
            match (route_press(&case["input"]), want["ok"].as_bool()) {
                (Ok(route), Some(true)) => {
                    let value = &want["value"];
                    assert_eq!(value["label"], json!(route.label), "{name}: label");
                    assert_eq!(value["ok"], json!(route.target.is_some()), "{name}: ok");
                    match route.target {
                        Some(hit) => {
                            assert_eq!(value["target"], json!(hit.id), "{name}: target");
                            close(hit.distance, &value["distance"], name);
                            close(hit.angle, &value["angle"], name);
                            assert_eq!(value["reasonCode"], Value::Null, "{name}: reasonCode");
                        }
                        None => {
                            assert_eq!(value["target"], Value::Null, "{name}: target");
                            assert_eq!(value["reasonCode"], json!(NO_TARGET), "{name}: reasonCode");
                        }
                    }
                }
                (Err(refusal), Some(false)) => {
                    assert_eq!(want["reasonCode"], json!(refusal.reason_code), "{name}: reasonCode");
                    assert_eq!(want["message"], json!(refusal.message), "{name}: message");
                }
                (got, _) => panic!("{name}: spoint returns {want}, bevy returns {got:?}"),
            }
        }
    }
}
