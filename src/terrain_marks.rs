#![cfg_attr(not(test), allow(dead_code))]

use serde_json::Value;

pub const TERRAIN_MARK_KINDS: [&str; 12] = [
    "flatten", "pond", "lake", "sea", "river", "path", "road", "clear", "hole", "grade", "spline", "liquid",
];

const DEFAULT_FALLOFF: f64 = 1e-9;

type Point = [f64; 2];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainMarkError {
    pub reason_code: &'static str,
    pub index: isize,
}

fn refusal(reason_code: &'static str, index: isize) -> TerrainMarkError {
    TerrainMarkError { reason_code, index }
}

#[derive(Debug, Clone, Copy)]
struct Vertex {
    x: f64,
    z: f64,
    level: f64,
}

#[derive(Debug, Clone)]
enum Shape {
    Flatten { center: Point, radius: f64, falloff: f64, height: f64 },
    Pond { center: Point, radius: f64, falloff: f64, level: f64 },
    River { points: Vec<Vertex>, half_width: f64, falloff: f64, depth: f64 },
    Polyline { points: Vec<Vertex>, half_width: f64, falloff: f64 },
    Grade { center: Point, radius: f64, falloff: f64, level: f64, slope: f64 },
    Clear { center: Point, radius: f64 },
}

#[derive(Debug, Clone, Default)]
pub struct TerrainMarks {
    shapes: Vec<Shape>,
}

fn smooth(t: f64) -> f64 {
    if t <= 0.0 {
        0.0
    } else if t >= 1.0 {
        1.0
    } else {
        t * t * (3.0 - 2.0 * t)
    }
}

fn weight(distance: f64, edge: f64, falloff: f64) -> f64 {
    1.0 - smooth((distance - edge) / falloff)
}

fn planar_distance(x: f64, z: f64, center: Point) -> f64 {
    (x - center[0]).hypot(z - center[1])
}

fn segment(x: f64, z: f64, a: Vertex, b: Vertex) -> (f64, f64) {
    let dx = b.x - a.x;
    let dz = b.z - a.z;
    let len2 = dx * dx + dz * dz;
    let t = if len2 > 0.0 {
        (((x - a.x) * dx + (z - a.z) * dz) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let px = a.x + dx * t - x;
    let pz = a.z + dz * t - z;
    (px.hypot(pz), t)
}

fn nearest(x: f64, z: f64, points: &[Vertex]) -> (f64, f64, usize) {
    let mut best = (f64::INFINITY, 0.0, 0);
    for (index, pair) in points.windows(2).enumerate() {
        let (distance, t) = segment(x, z, pair[0], pair[1]);
        if distance < best.0 {
            best = (distance, t, index);
        }
    }
    best
}

impl Shape {
    fn clears(&self, x: f64, z: f64) -> bool {
        match self {
            Shape::Clear { center, radius } => {
                let dx = x - center[0];
                let dz = z - center[1];
                dx * dx + dz * dz <= radius * radius
            }
            _ => false,
        }
    }

    fn apply(&self, x: f64, z: f64, h: f64) -> f64 {
        match self {
            Shape::Flatten { center, radius, falloff, height } => {
                let w = weight(planar_distance(x, z, *center), *radius, *falloff);
                if w <= 0.0 {
                    h
                } else {
                    h + (height - h) * w
                }
            }
            Shape::Pond { center, radius, falloff, level } => {
                let w = weight(planar_distance(x, z, *center), *radius, *falloff);
                if w <= 0.0 {
                    h
                } else {
                    h + (h.min(*level) - h) * w
                }
            }
            Shape::River { points, half_width, falloff, depth } => {
                let (distance, _, _) = nearest(x, z, points);
                let w = weight(distance, *half_width, *falloff);
                if w <= 0.0 {
                    h
                } else {
                    h - depth * w
                }
            }
            Shape::Polyline { points, half_width, falloff } => {
                let (distance, t, index) = nearest(x, z, points);
                let w = weight(distance, *half_width, *falloff);
                if w <= 0.0 {
                    return h;
                }
                let (a, b) = (points[index], points[index + 1]);
                let level = a.level + (b.level - a.level) * t;
                h + (level - h) * w
            }
            Shape::Grade { center, radius, falloff, level, slope } => {
                let w = weight(planar_distance(x, z, *center), *radius, *falloff);
                if w <= 0.0 {
                    h
                } else {
                    let target = level + slope * (x - center[0]);
                    h + (target - h) * w
                }
            }
            Shape::Clear { .. } => h,
        }
    }
}

fn finite(value: Option<&Value>, index: isize) -> Result<f64, TerrainMarkError> {
    value
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite())
        .ok_or(refusal("bad-number", index))
}

fn at_least(value: Option<&Value>, index: isize, min: f64) -> Result<f64, TerrainMarkError> {
    let v = finite(value, index)?;
    if v < min {
        Err(refusal("bad-number", index))
    } else {
        Ok(v)
    }
}

fn positive(value: Option<&Value>, index: isize) -> Result<f64, TerrainMarkError> {
    let v = finite(value, index)?;
    if v > 0.0 {
        Ok(v)
    } else {
        Err(refusal("bad-number", index))
    }
}

fn falloff(value: Option<&Value>, index: isize) -> Result<f64, TerrainMarkError> {
    at_least(value, index, 0.0).map(|v| if v > 0.0 { v } else { DEFAULT_FALLOFF })
}

fn point(items: Option<&[Value]>, index: isize) -> Result<Point, TerrainMarkError> {
    let refused = refusal("bad-point", index);
    match items {
        Some([x, z]) => {
            let x = x.as_f64().filter(|v| v.is_finite()).ok_or(refused)?;
            let z = z.as_f64().filter(|v| v.is_finite()).ok_or(refused)?;
            Ok([x, z])
        }
        _ => Err(refused),
    }
}

fn array<'a>(spec: &'a Value, key: &str) -> Option<&'a [Value]> {
    spec.get(key)?.as_array().map(Vec::as_slice)
}

fn center_of(spec: &Value, index: isize) -> Result<Point, TerrainMarkError> {
    point(array(spec, "center"), index)
}

fn polyline(spec: &Value, index: isize, with_levels: bool) -> Result<Vec<Vertex>, TerrainMarkError> {
    let entries = array(spec, "points")
        .filter(|entries| entries.len() >= 2)
        .ok_or(refusal("short-polyline", index))?;
    let mut vertices = Vec::with_capacity(entries.len());
    for entry in entries {
        let head = entry.as_array().map(|items| &items[..items.len().min(2)]);
        let [x, z] = point(head, index)?;
        let level = if with_levels {
            finite(entry.as_array().filter(|items| items.len() == 3).map(|items| &items[2]), index)?
        } else {
            0.0
        };
        vertices.push(Vertex { x, z, level });
    }
    Ok(vertices)
}

fn compile_kind(kind: &str, spec: &Value, index: isize) -> Result<Option<Shape>, TerrainMarkError> {
    let shape = match kind {
        "flatten" => Some(Shape::Flatten {
            center: center_of(spec, index)?,
            radius: positive(spec.get("radius"), index)?,
            falloff: falloff(spec.get("falloff"), index)?,
            height: finite(spec.get("height"), index)?,
        }),
        "pond" | "lake" | "sea" => Some(Shape::Pond {
            center: center_of(spec, index)?,
            radius: positive(spec.get("radius"), index)?,
            falloff: falloff(spec.get("falloff"), index)?,
            level: finite(spec.get("level"), index)?,
        }),
        "river" => {
            let points = polyline(spec, index, false)?;
            let width = positive(spec.get("width"), index)?;
            Some(Shape::River {
                points,
                half_width: width / 2.0,
                falloff: falloff(spec.get("falloff"), index)?,
                depth: at_least(spec.get("depth"), index, 0.0)?,
            })
        }
        "path" | "road" => {
            let points = polyline(spec, index, true)?;
            let width = positive(spec.get("width"), index)?;
            Some(Shape::Polyline {
                points,
                half_width: width / 2.0,
                falloff: falloff(spec.get("falloff"), index)?,
            })
        }
        "spline" => {
            let points = polyline(spec, index, true)?;
            let width = at_least(spec.get("width"), index, 0.0)?;
            Some(Shape::Polyline {
                points,
                half_width: width / 2.0,
                falloff: positive(spec.get("falloff"), index)?,
            })
        }
        "clear" | "hole" => Some(Shape::Clear {
            center: center_of(spec, index)?,
            radius: positive(spec.get("radius"), index)?,
        }),
        "grade" => {
            let center = center_of(spec, index)?;
            let radius = positive(spec.get("radius"), index)?;
            let falloff = falloff(spec.get("falloff"), index)?;
            let level = finite(spec.get("level"), index)?;
            let slope = finite(spec.get("slope"), index)?;
            let max_grade = at_least(spec.get("maxGrade"), index, 0.0)?;
            if slope.abs() > max_grade {
                return Err(refusal("grade-too-steep", index));
            }
            Some(Shape::Grade {
                center,
                radius,
                falloff,
                level,
                slope,
            })
        }
        _ => {
            center_of(spec, index)?;
            positive(spec.get("radius"), index)?;
            finite(spec.get("level"), index)?;
            None
        }
    };
    Ok(shape)
}

pub fn compile_terrain_marks(specs: &Value) -> Result<TerrainMarks, TerrainMarkError> {
    let items = specs.as_array().ok_or(refusal("not-a-list", -1))?;
    let mut shapes = Vec::new();
    for (position, spec) in items.iter().enumerate() {
        let index = position as isize;
        if !spec.is_object() && !spec.is_array() {
            return Err(refusal("not-an-object", index));
        }
        let kind = spec
            .get("kind")
            .and_then(Value::as_str)
            .filter(|kind| TERRAIN_MARK_KINDS.contains(kind))
            .ok_or(refusal("unknown-kind", index))?;
        shapes.extend(compile_kind(kind, spec, index)?);
    }
    Ok(TerrainMarks { shapes })
}

impl TerrainMarks {
    fn clears_at(&self, x: f64, z: f64) -> bool {
        self.shapes.iter().any(|shape| shape.clears(x, z))
    }

    fn height(&self, x: f64, z: f64, h: f64) -> f64 {
        if self.clears_at(x, z) {
            return h;
        }
        self.shapes.iter().fold(h, |out, shape| shape.apply(x, z, out))
    }
}

pub fn wrap_height_with_marks(
    specs: Option<&Value>,
    height: impl Fn(f64, f64, f64) -> f64,
) -> Result<impl Fn(f64, f64, f64) -> f64, TerrainMarkError> {
    let marks = match specs {
        None | Some(Value::Null) => TerrainMarks::default(),
        Some(specs) => compile_terrain_marks(specs)?,
    };
    Ok(move |x: f64, z: f64, y_guess: f64| marks.height(x, z, height(x, z, y_guess)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const FIXTURE: &str = include_str!("../testdata/parity/spoint-terrain-marks.json");
    const HEIGHT_TOLERANCE: f64 = 1e-9;

    fn fixture() -> Value {
        serde_json::from_str(FIXTURE).expect("spoint-terrain-marks.json parses")
    }

    fn num(value: &Value) -> f64 {
        value.as_f64().expect("number")
    }

    fn base_height(x: f64, z: f64, y_guess: f64) -> f64 {
        1.5 + 0.5 * x - 0.25 * z + 0.125 * y_guess
    }

    #[test]
    fn kind_set_matches_spoint() {
        let document = fixture();
        let kinds: Vec<&str> = document["kinds"]
            .as_array()
            .expect("kinds")
            .iter()
            .map(|kind| kind.as_str().expect("kind string"))
            .collect();
        assert_eq!(TERRAIN_MARK_KINDS.to_vec(), kinds);
    }

    #[test]
    fn refusals_match_spoint() {
        let document = fixture();
        let mut checked = 0;
        for case in document["cases"].as_array().expect("cases") {
            let name = case["name"].as_str().expect("name");
            let Some(expected) = case["error"].as_object() else {
                continue;
            };
            let err = match wrap_height_with_marks(Some(&case["specs"]), base_height) {
                Ok(_) => panic!("case {name}: spoint refuses but bevy accepts"),
                Err(err) => err,
            };
            let expected_code = expected["reasonCode"].as_str().expect("reasonCode");
            let expected_index = expected["index"].as_i64().expect("index") as isize;
            assert_eq!((err.reason_code, err.index), (expected_code, expected_index), "case {name}");
            checked += 1;
        }
        assert!(checked >= 57, "only {checked} refusal cases checked");
    }

    #[test]
    fn heights_match_spoint() {
        let document = fixture();
        let mut checked = 0;
        for case in document["cases"].as_array().expect("cases") {
            let name = case["name"].as_str().expect("name");
            if case["error"].is_object() {
                continue;
            }
            let height = wrap_height_with_marks(Some(&case["specs"]), base_height)
                .unwrap_or_else(|err| panic!("case {name}: bevy refuses {err:?}"));
            for point in case["points"].as_array().expect("points") {
                let (x, z, y_guess) = (num(&point["x"]), num(&point["z"]), num(&point["yGuess"]));
                let got = height(x, z, y_guess);
                let want = num(&point["height"]);
                assert!(
                    (got - want).abs() <= HEIGHT_TOLERANCE,
                    "case {name} at ({x}, {z}): bevy {got} spoint {want}"
                );
                checked += 1;
            }
        }
        assert!(checked >= 21 * 12, "only {checked} height points checked");
    }
}
