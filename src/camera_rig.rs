#![cfg_attr(not(test), allow(dead_code))]
use serde_json::Value;

#[derive(Debug, PartialEq)]
pub struct Camera {
    pub layer: String,
    pub position: [f64; 3],
    pub target: [f64; 3],
    pub done: bool,
}

struct Frame {
    t: f64,
    position: [f64; 3],
    target: [f64; 3],
}

#[derive(Clone, Copy, Default)]
struct Look {
    yaw: f64,
    pitch: f64,
}

enum Kind {
    Orbit {
        distance: f64,
        min_pitch: f64,
        max_pitch: f64,
        sensitivity: f64,
        target_height: f64,
        look: Look,
    },
    Chase {
        distance: f64,
        height: f64,
        target_height: f64,
    },
    FirstPerson {
        eye_height: f64,
        sensitivity: f64,
        max_pitch: f64,
        look: Look,
    },
    Cutscene {
        frames: Vec<Frame>,
        looped: bool,
        t: f64,
        done: bool,
    },
}

pub struct CameraRig {
    layers: Vec<(String, Kind)>,
    default: usize,
}

fn finite(value: Option<&Value>) -> Option<f64> {
    value.and_then(Value::as_f64).filter(|x| x.is_finite())
}

fn finite_or(value: Option<&Value>, fallback: f64) -> f64 {
    finite(value).unwrap_or(fallback)
}

fn vec3(value: Option<&Value>) -> Option<[f64; 3]> {
    let items = value?.as_array()?;
    if items.len() != 3 {
        return None;
    }
    Some([finite(items.get(0))?, finite(items.get(1))?, finite(items.get(2))?])
}

fn lerp(a: [f64; 3], b: [f64; 3], k: f64) -> [f64; 3] {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * k)
}

fn cutscene(raw: &Value) -> Result<Kind, &'static str> {
    let items = raw.get("frames").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]);
    if items.is_empty() {
        return Err("cutscene-no-frames");
    }
    let mut frames = Vec::with_capacity(items.len());
    for item in items {
        match (finite(item.get("t")), vec3(item.get("position")), vec3(item.get("target"))) {
            (Some(t), Some(position), Some(target)) => frames.push(Frame { t, position, target }),
            _ => return Err("cutscene-bad-frame"),
        }
    }
    if frames.windows(2).any(|pair| pair[1].t < pair[0].t) {
        return Err("cutscene-unsorted");
    }
    Ok(Kind::Cutscene {
        frames,
        looped: raw.get("loop") == Some(&Value::Bool(true)),
        t: 0.0,
        done: false,
    })
}

impl Kind {
    fn compile(kind: &str, raw: &Value) -> Result<Self, &'static str> {
        Ok(match kind {
            "orbit" => Kind::Orbit {
                distance: finite_or(raw.get("distance"), 4.0),
                min_pitch: finite_or(raw.get("minPitch"), -1.2),
                max_pitch: finite_or(raw.get("maxPitch"), 1.2),
                sensitivity: finite_or(raw.get("sensitivity"), 0.003),
                target_height: finite_or(raw.get("targetHeight"), 1.6),
                look: Look::default(),
            },
            "chase" => Kind::Chase {
                distance: finite_or(raw.get("distance"), 5.0),
                height: finite_or(raw.get("height"), 2.0),
                target_height: finite_or(raw.get("targetHeight"), 1.2),
            },
            "firstPerson" => Kind::FirstPerson {
                eye_height: finite_or(raw.get("eyeHeight"), 1.6),
                sensitivity: finite_or(raw.get("sensitivity"), 0.002),
                max_pitch: finite_or(raw.get("maxPitch"), 1.5),
                look: Look::default(),
            },
            "cutscene" => cutscene(raw)?,
            "custom" => return Err("custom-not-ported"),
            _ => return Err("layer-unknown-kind"),
        })
    }

    fn on_input(&mut self, input: &Value) -> bool {
        match self {
            Kind::Orbit { sensitivity, min_pitch, max_pitch, look, .. } => {
                look.yaw -= finite_or(input.get("lookX"), 0.0) * *sensitivity;
                look.pitch = (look.pitch - finite_or(input.get("lookY"), 0.0) * *sensitivity).max(*min_pitch).min(*max_pitch);
                true
            }
            Kind::FirstPerson { sensitivity, max_pitch, look, .. } => {
                look.yaw -= finite_or(input.get("lookX"), 0.0) * *sensitivity;
                look.pitch = (look.pitch - finite_or(input.get("lookY"), 0.0) * *sensitivity).max(-*max_pitch).min(*max_pitch);
                true
            }
            Kind::Chase { .. } | Kind::Cutscene { .. } => false,
        }
    }

    fn update(&mut self, position: [f64; 3], yaw: f64, dt: f64) -> ([f64; 3], [f64; 3], bool) {
        let [x, y, z] = position;
        match self {
            Kind::Orbit { distance, target_height, look, .. } => {
                let target = [x, y + *target_height, z];
                let cp = look.pitch.cos();
                let eye = [
                    target[0] + *distance * cp * look.yaw.sin(),
                    target[1] + *distance * look.pitch.sin(),
                    target[2] + *distance * cp * look.yaw.cos(),
                ];
                (eye, target, false)
            }
            Kind::Chase { distance, height, target_height } => {
                let (s, c) = (yaw.sin(), yaw.cos());
                let eye = [x - *distance * s, y + *height, z - *distance * c];
                (eye, [x, y + *target_height, z], false)
            }
            Kind::FirstPerson { eye_height, look, .. } => {
                let eye = [x, y + *eye_height, z];
                let cp = look.pitch.cos();
                let target = [
                    eye[0] + look.yaw.sin() * cp,
                    eye[1] + look.pitch.sin(),
                    eye[2] + look.yaw.cos() * cp,
                ];
                (eye, target, false)
            }
            Kind::Cutscene { frames, looped, t, done } => {
                *t += dt;
                let span = frames[frames.len() - 1].t;
                let mut at = *t;
                if *looped && span > 0.0 {
                    at %= span;
                } else if at >= span {
                    *done = true;
                    at = span;
                }
                let mut i = 0;
                while i + 2 < frames.len() && frames[i + 1].t <= at {
                    i += 1;
                }
                let a = &frames[i];
                let b = &frames[(i + 1).min(frames.len() - 1)];
                let k = if b.t > a.t { ((at - a.t) / (b.t - a.t)).max(0.0).min(1.0) } else { 0.0 };
                (lerp(a.position, b.position, k), lerp(a.target, b.target, k), *done)
            }
        }
    }
}

fn compile(raw: &Value) -> Result<(String, Kind), &'static str> {
    if !(raw.is_object() || raw.is_array()) {
        return Err("layer-not-object");
    }
    let id = raw.get("id").and_then(Value::as_str).filter(|id| !id.is_empty()).ok_or("layer-no-id")?;
    let kind = raw.get("kind").and_then(Value::as_str).ok_or("layer-unknown-kind")?;
    Ok((id.to_string(), Kind::compile(kind, raw)?))
}

impl CameraRig {
    pub fn define(spec: &Value) -> Result<Self, &'static str> {
        let obj = spec.as_object().ok_or("spec-not-object")?;
        let raw_layers = obj
            .get("layers")
            .and_then(Value::as_array)
            .filter(|layers| !layers.is_empty())
            .ok_or("no-layers")?;
        let mut layers: Vec<(String, Kind)> = Vec::with_capacity(raw_layers.len());
        for raw in raw_layers {
            let (id, kind) = compile(raw)?;
            if layers.iter().any(|(existing, _)| *existing == id) {
                return Err("layer-duplicate-id");
            }
            layers.push((id, kind));
        }
        let default_id = match obj.get("default") {
            None | Some(Value::Null) => layers[0].0.clone(),
            Some(value) => value.as_str().unwrap_or_default().to_string(),
        };
        let default = layers.iter().position(|(id, _)| *id == default_id).ok_or("default-unknown")?;
        Ok(Self { layers, default })
    }

    pub fn on_input(&mut self, input: &Value) -> bool {
        self.layers[self.default].1.on_input(input)
    }

    pub fn update(&mut self, pose: &Value, dt: f64) -> Result<Camera, &'static str> {
        let position = vec3(pose.get("position")).ok_or("pose-invalid")?;
        let yaw = finite(pose.get("yaw")).ok_or("pose-invalid")?;
        if !dt.is_finite() || dt < 0.0 {
            return Err("dt-invalid");
        }
        let (layer, kind) = &mut self.layers[self.default];
        let (position, target, done) = kind.update(position, yaw, dt);
        Ok(Camera {
            layer: layer.clone(),
            position,
            target,
            done,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f64 = 1e-9;

    fn assert_close(label: &str, got: [f64; 3], want: &Value) {
        let want = vec3(Some(want)).unwrap();
        for i in 0..3 {
            assert!(
                (got[i] - want[i]).abs() <= TOLERANCE,
                "{label}[{i}]: bevy {} against spoint {}",
                got[i],
                want[i]
            );
        }
    }

    #[test]
    fn camera_rig_matches_spoint() {
        let fixture: Value = serde_json::from_str(include_str!("../testdata/parity/spoint-camera-rig.json")).unwrap();
        for case in fixture["define"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            assert_eq!(CameraRig::define(&case["spec"]).err(), case["error"].as_str(), "define {name}");
        }
        for case in fixture["cases"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let mut rig = CameraRig::define(&case["spec"])
                .unwrap_or_else(|code| panic!("{name}: spoint accepts, bevy refuses {code}"));
            for (i, step) in case["steps"].as_array().unwrap().iter().enumerate() {
                let label = format!("{name} step {i}");
                if let Some(input) = step.get("input") {
                    assert_eq!(rig.on_input(input), step["returned"].as_bool().unwrap(), "{label} returned");
                    continue;
                }
                match rig.update(&step["pose"], step["dt"].as_f64().unwrap()) {
                    Err(code) => assert_eq!(Some(code), step["error"].as_str(), "{label} refusal"),
                    Ok(camera) => {
                        assert_eq!(None, step["error"].as_str(), "{label}: bevy accepts, spoint refuses");
                        assert_eq!(camera.layer, step["camera"]["layer"].as_str().unwrap(), "{label} layer");
                        assert_close(&format!("{label} position"), camera.position, &step["camera"]["position"]);
                        assert_close(&format!("{label} target"), camera.target, &step["camera"]["target"]);
                        assert_eq!(camera.done, step["camera"]["done"].as_bool().unwrap(), "{label} done");
                    }
                }
            }
        }
    }
}
