use crate::vec3::dot;

use crate::height::ops_jsgen::V3;
use crate::height::planet_frame::Basis;
const WATER_SAMPLERS_MISSING: &str = "[fire] firebreaks.water needs both a height and a sea-level sampler: pass spec.firebreaks.heightAt and spec.firebreaks.seaLevelAt, or expose terrainHeightAt and seaLevelAt on the app ctx, or declare spec.firebreaks.water: false";
const KIND_SAMPLER_MISSING: &str = "[fire] firebreaks.kinds needs a terrain kind sampler: pass spec.firebreaks.kindAt, or expose terrainKindAt on the app ctx, or declare spec.firebreaks.kinds: []";

pub struct Cleared {
    pub center: [f64; 2],
    pub radius_m: f64,
}

#[derive(Default)]
pub struct Samplers {
    pub height_at: Option<Box<dyn Fn(f64, f64) -> f64>>,
    pub sea_level_at: Option<Box<dyn Fn(f64, f64) -> f64>>,
    pub kind_at: Option<Box<dyn Fn(f64, f64) -> Option<&'static str>>>,
}

pub struct Breaks {
    pub water: bool,
    pub kinds: Vec<&'static str>,
    pub cleared: Vec<Cleared>,
    pub rock: Option<Box<dyn Fn(f64, f64) -> bool>>,
    pub samplers: Samplers,
}

pub struct FireFrame {
    pub radius: f64,
    pub anchor_height: f64,
    pub basis: Basis,
}

pub fn create_firebreak_fuel<'a>(
    base_class_at: &'a dyn Fn(i64, i64, i64) -> u8,
    breaks: &'a Breaks,
    frame: &'a FireFrame,
    cell_centre_dir: &'a dyn Fn(i64, i64, i64) -> V3,
    terrain: &'a Samplers,
) -> Result<impl Fn(i64, i64, i64) -> u8 + 'a, String> {
    let radius = frame.radius;
    if !(radius.is_finite() && radius > 0.0) {
        return Err(format!("[fire] firebreaks need a planet radius: frame.radius must be a positive finite number, got {radius}"));
    }
    let height_at = breaks.samplers.height_at.as_deref().or(terrain.height_at.as_deref());
    let sea_level_at = breaks.samplers.sea_level_at.as_deref().or(terrain.sea_level_at.as_deref());
    let kind_at = breaks.samplers.kind_at.as_deref().or(terrain.kind_at.as_deref());
    if breaks.water && (height_at.is_none() || sea_level_at.is_none()) {
        return Err(WATER_SAMPLERS_MISSING.to_string());
    }
    if !breaks.kinds.is_empty() && kind_at.is_none() {
        return Err(KIND_SAMPLER_MISSING.to_string());
    }
    let basis = &frame.basis;
    let top = radius + frame.anchor_height;
    Ok(move |face: i64, i: i64, j: i64| {
        let base = base_class_at(face, i, j);
        if base == 0 {
            return base;
        }
        let centre = cell_centre_dir(face, i, j);
        let du = dot(centre, basis.up);
        if !(du > 0.2) {
            return base;
        }
        let x = dot(centre, basis.east) / du * top;
        let z = dot(centre, basis.north) / du * top;
        if breaks.cleared.iter().any(|c| (x - c.center[0]).hypot(z - c.center[1]) <= c.radius_m) {
            return 0;
        }
        if !breaks.kinds.is_empty() {
            if let Some(k) = kind_at.and_then(|f| f(x, z)) {
                if breaks.kinds.contains(&k) {
                    return 0;
                }
            }
        }
        if breaks.water {
            let h = height_at.map_or(f64::NAN, |f| f(x, z));
            let sea = sea_level_at.map_or(f64::NAN, |f| f(x, z));
            if h.is_finite() && sea.is_finite() && h < sea {
                return 0;
            }
        }
        if breaks.rock.as_deref().is_some_and(|rock| rock(x, z)) {
            return 0;
        }
        base
    })
}

mod tests {
    use super::*;
    use serde_json::Value;

    fn vec3(v: &Value) -> V3 {
        [v[0].as_f64().unwrap(), v[1].as_f64().unwrap(), v[2].as_f64().unwrap()]
    }

    fn ramp(x: f64, z: f64) -> f64 {
        0.5 * x - 0.25 * z
    }

    fn zero(_x: f64, _z: f64) -> f64 {
        0.0
    }

    fn band(x: f64, z: f64) -> Option<&'static str> {
        if x < -6.0 {
            Some("road")
        } else if z > 8.0 {
            Some("river")
        } else if x > 14.0 {
            Some("rock")
        } else if x > 9.0 {
            Some("grass")
        } else {
            None
        }
    }

    fn edge(x: f64, _z: f64) -> bool {
        x > 25.0
    }

    fn height(name: &Value) -> Option<Box<dyn Fn(f64, f64) -> f64>> {
        match name.as_str() {
            Some("ramp") => Some(Box::new(ramp)),
            Some(other) => panic!("unknown height sampler {other}"),
            None => None,
        }
    }

    fn sea(name: &Value) -> Option<Box<dyn Fn(f64, f64) -> f64>> {
        match name.as_str() {
            Some("zero") => Some(Box::new(zero)),
            Some(other) => panic!("unknown sea sampler {other}"),
            None => None,
        }
    }

    fn kind(name: &Value) -> Option<Box<dyn Fn(f64, f64) -> Option<&'static str>>> {
        match name.as_str() {
            Some("band") => Some(Box::new(band)),
            Some(other) => panic!("unknown kind sampler {other}"),
            None => None,
        }
    }

    fn rock(name: &Value) -> Option<Box<dyn Fn(f64, f64) -> bool>> {
        match name.as_str() {
            Some("edge") => Some(Box::new(edge)),
            Some(other) => panic!("unknown rock sampler {other}"),
            None => None,
        }
    }

    fn samplers(v: &Value) -> Samplers {
        Samplers { height_at: height(&v["heightAt"]), sea_level_at: sea(&v["seaLevelAt"]), kind_at: kind(&v["kindAt"]) }
    }

    fn kind_name(s: &str) -> &'static str {
        match s {
            "road" => "road",
            "river" => "river",
            other => panic!("unknown firebreak kind {other}"),
        }
    }

    #[test]
    fn create_firebreak_fuel_matches_spoint() {
        let fixture: Value = crate::testkit::parse(include_str!("../testdata/parity/spoint-fire-terrain.json"));
        let frame_json = &fixture["frame"];
        let base_class_at = |face: i64, i: i64, j: i64| ((i + 2 * j + face + 8) % 4) as u8;
        let cell_centre_dir = |face: i64, i: i64, j: i64| {
            let x = (i - 3) as f64;
            let y = (j - 3) as f64;
            let z = (2 + face) as f64;
            let n = (x * x + y * y + z * z).sqrt();
            [x / n, y / n, z / n]
        };
        let faces = [-3_i64, 0, 1, 2];
        let mut refused = 0;
        for case in fixture["cases"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let want = &case["expect"];
            let breaks_json = &case["breaks"];
            let breaks = Breaks {
                water: breaks_json["water"].as_bool().unwrap(),
                kinds: breaks_json["kinds"].as_array().unwrap().iter().map(|k| kind_name(k.as_str().unwrap())).collect(),
                cleared: breaks_json["cleared"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|c| Cleared { center: [c["center"][0].as_f64().unwrap(), c["center"][1].as_f64().unwrap()], radius_m: c["radiusM"].as_f64().unwrap() })
                    .collect(),
                rock: rock(&breaks_json["rock"]),
                samplers: samplers(breaks_json),
            };
            let terrain = samplers(&case["terrain"]);
            let frame = FireFrame {
                radius: case["radius"].as_f64().unwrap(),
                anchor_height: case["anchorHeight"].as_f64().unwrap(),
                basis: Basis { up: vec3(&frame_json["up"]), east: vec3(&frame_json["east"]), north: vec3(&frame_json["north"]) },
            };
            match create_firebreak_fuel(&base_class_at, &breaks, &frame, &cell_centre_dir, &terrain) {
                Err(message) => {
                    assert_eq!(want["ok"], Value::Bool(false), "{name}");
                    assert_eq!(message, want["message"].as_str().unwrap(), "{name}");
                    refused += 1;
                }
                Ok(class_at) => {
                    assert_eq!(want["ok"], Value::Bool(true), "{name}");
                    let classes = want["classes"].as_array().unwrap();
                    let mut k = 0;
                    for face in faces {
                        for i in 0..7 {
                            for j in 0..7 {
                                assert_eq!(i64::from(class_at(face, i, j)), classes[k].as_i64().unwrap(), "{name}: face {face} I {i} J {j}");
                                k += 1;
                            }
                        }
                    }
                    assert_eq!(k, classes.len(), "{name}");
                }
            }
        }
        assert_eq!(refused, 5);
    }
}
