use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct WorldConfig {
    pub name: Option<String>,
    pub description: Option<String>,
    pub engine: Value,
    pub inputs: Value,
    pub player: Option<String>,
    pub camera: Option<Value>,
    pub default_place: Option<String>,
    pub ui: Value,
    pub nametag: Option<String>,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PlaceDef {
    pub mode: Option<String>,
    pub gravity: Option<Value>,
    pub tick_base: Option<f64>,
    pub damping: Option<f64>,
    pub terrain: Option<Value>,
    pub atmosphere: Option<Value>,
    pub camera: Option<Value>,
    pub inputs: Option<Value>,
    pub ui: Option<Value>,
    pub policy: Option<Value>,
    pub default_jayzee_point: Option<Value>,
    pub default_jayzee_rotation: Option<Value>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ObjectSpec {
    pub id: Option<String>,
    pub tags: Vec<String>,
    pub state: Map<String, Value>,
    pub behavior: Option<BehaviorRef>,
    pub parent: Option<String>,
    pub children: Vec<ObjectSpec>,
    pub place: Option<String>,
    pub audience: Option<String>,
    pub visibility: Option<String>,
    pub lifetime: Option<f64>,
    #[serde(flatten)]
    pub properties: ObjectProperties,
}

impl ObjectSpec {
    pub fn behaviors(&self) -> Vec<String> {
        match &self.behavior {
            None => Vec::new(),
            Some(BehaviorRef::One(one)) => vec![one.clone()],
            Some(BehaviorRef::Many(many)) => many.clone(),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum BehaviorRef {
    One(String),
    Many(Vec<String>),
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ObjectProperties {
    pub feet_position: Option<Value>,
    pub rotation: Option<Value>,
    pub yaw: Option<f64>,
    pub scale: Option<Value>,
    pub visible: Option<bool>,
    pub visible_in_first_person: Option<Value>,
    pub cast_shadow: Option<bool>,
    pub receive_shadow: Option<bool>,
    pub primitive: Option<Value>,
    pub sprite: Option<Value>,
    pub room: Option<Value>,
    pub path: Option<Value>,
    pub layout: Option<Value>,
    pub material: Option<Value>,
    pub physics: Option<Value>,
    pub light: Option<Value>,
    pub audio: Option<Value>,
    pub model: Option<Value>,
    pub mixer: Option<Value>,
    pub attachment: Option<Value>,
    pub spline: Option<Value>,
    pub scatter: Option<Value>,
    pub pivot: Option<Value>,
    pub render: Option<bool>,
    pub visible_range: Option<Value>,
    pub layers: Option<Value>,
    pub visible_to: Option<String>,
    pub vector: Option<Value>,
    pub jiggle: Option<Value>,
    pub morph: Option<Value>,
    pub bones: Option<Value>,
    pub wisp: Option<Value>,
    pub boxart: Option<Value>,
    pub collider2d: Option<Value>,
    pub ui: Option<Value>,
    pub air: Option<bool>,
    pub press: Option<Value>,
    pub fx: Option<Value>,
    pub voice: Option<Value>,
    pub interpolation: Option<Value>,
    pub template: Option<String>,
    pub overrides: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct PrimitiveDesc {
    pub kind: String,
    pub size: bevy::math::Vec3,
    pub half_height: f32,
    pub radius: f32,
    pub capsule_half_height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyKind {
    None,
    Static,
    Dynamic,
    Kinematic,
    Character,
}

#[derive(Clone, Debug)]
pub struct PhysicsDesc {
    pub body: BodyKind,
    pub collider: String,
}

pub fn number(value: Option<&Value>, fallback: f32) -> f32 {
    value.and_then(Value::as_f64).map(|v| v as f32).unwrap_or(fallback)
}

pub fn vec3(value: Option<&Value>) -> bevy::math::Vec3 {
    let Some(value) = value else {
        return bevy::math::Vec3::ZERO;
    };
    if let Some(array) = value.as_array() {
        if array.len() >= 3 {
            return bevy::math::Vec3::new(
                array[0].as_f64().unwrap_or(0.0) as f32,
                array[1].as_f64().unwrap_or(0.0) as f32,
                array[2].as_f64().unwrap_or(0.0) as f32,
            );
        }
    }
    if let Some(object) = value.as_object() {
        return bevy::math::Vec3::new(
            number(object.get("x"), 0.0),
            number(object.get("y"), 0.0),
            number(object.get("z"), 0.0),
        );
    }
    if let Some(value) = value.as_f64() {
        return bevy::math::Vec3::Y * value as f32;
    }
    bevy::math::Vec3::ZERO
}

pub fn vec3_opt(value: &Value) -> Option<bevy::math::Vec3> {
    if value.is_array() || value.is_object() || value.is_number() {
        Some(vec3(Some(value)))
    } else {
        None
    }
}

pub fn scale(value: Option<&Value>) -> bevy::math::Vec3 {
    let Some(value) = value else {
        return bevy::math::Vec3::ONE;
    };
    if let Some(value) = value.as_f64() {
        return bevy::math::Vec3::splat(value as f32);
    }
    let vector = vec3(Some(value));
    if vector == bevy::math::Vec3::ZERO {
        bevy::math::Vec3::ONE
    } else {
        vector
    }
}

pub fn rotation(value: Option<&Value>) -> bevy::math::Quat {
    use bevy::math::{EulerRot, Quat};
    let Some(value) = value else {
        return Quat::IDENTITY;
    };
    if let Some(yaw) = value.as_f64() {
        return Quat::from_rotation_y(yaw as f32 * std::f32::consts::PI / 180.0);
    }
    let Some(object) = value.as_object() else {
        return Quat::IDENTITY;
    };
    if object.contains_key("lookAt") {
        return Quat::IDENTITY;
    }
    if object.contains_key("x") && object.contains_key("w") {
        return Quat::from_xyzw(
            number(object.get("x"), 0.0),
            number(object.get("y"), 0.0),
            number(object.get("z"), 0.0),
            number(object.get("w"), 1.0),
        );
    }
    let yaw = number(object.get("yaw"), 0.0).to_radians();
    let pitch = number(object.get("pitch"), 0.0).to_radians();
    let roll = number(object.get("roll"), 0.0).to_radians();
    Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll)
}

pub fn primitive(props: &ObjectProperties) -> Option<PrimitiveDesc> {
    let value = props.primitive.as_ref()?;
    let (kind, object) = if let Some(kind) = value.as_str() {
        (kind.to_string(), None)
    } else {
        let object = value.as_object()?;
        let kind = object.get("kind")?.as_str()?.to_string();
        (kind, Some(object))
    };
    let object = object.cloned().unwrap_or_default();
    let mut desc = PrimitiveDesc {
        kind: kind.clone(),
        size: bevy::math::Vec3::ONE,
        half_height: 0.5,
        radius: 0.5,
        capsule_half_height: 0.5,
    };
    match kind.as_str() {
        "cube" => {
            let size = number(object.get("size"), 1.0);
            desc.size = bevy::math::Vec3::splat(size);
            desc.half_height = size * 0.5;
            desc.radius = size * 0.5;
        }
        "box" => {
            let width = number(object.get("width"), 1.0);
            let height = number(object.get("height"), 1.0);
            let depth = number(object.get("depth"), 1.0);
            desc.size = bevy::math::Vec3::new(width, height, depth);
            desc.half_height = height * 0.5;
            desc.radius = width.min(depth) * 0.5;
        }
        "sphere" | "hemisphere" => {
            let radius = number(object.get("radius"), 0.5);
            desc.size = bevy::math::Vec3::splat(radius * 2.0);
            desc.radius = radius;
            desc.half_height = radius;
        }
        "cylinder" | "frustum" | "hollowCylinder" => {
            let radius = number(object.get("radiusTop"), number(object.get("radius"), 1.0))
                .max(number(object.get("radiusBottom"), number(object.get("outerRadius"), 1.0)));
            let height = number(object.get("height"), 1.0);
            desc.size = bevy::math::Vec3::new(radius * 2.0, height, radius * 2.0);
            desc.radius = radius;
            desc.half_height = height * 0.5;
        }
        "capsule" => {
            let radius = number(object.get("radius"), 0.5);
            let half_height = number(object.get("halfHeight"), 0.5);
            desc.size = bevy::math::Vec3::new(radius * 2.0, (half_height + radius) * 2.0, radius * 2.0);
            desc.radius = radius;
            desc.capsule_half_height = half_height;
            desc.half_height = half_height + radius;
        }
        "plane" | "circle" | "ring" => {
            let width = number(object.get("width"), 1.0);
            let height = number(object.get("height"), 1.0);
            desc.size = bevy::math::Vec3::new(width, 0.0, height);
            desc.radius = width.min(height) * 0.5;
            desc.half_height = 0.0;
        }
        "cone" | "pyramid" => {
            let radius = number(object.get("radius"), 1.0);
            let height = number(object.get("height"), 1.0);
            desc.size = bevy::math::Vec3::new(radius * 2.0, height, radius * 2.0);
            desc.radius = radius;
            desc.half_height = height * 0.5;
        }
        "torus" | "torusKnot" => {
            let radius = number(object.get("radius"), 1.0);
            let tube = number(object.get("tube"), 0.4);
            desc.size = bevy::math::Vec3::splat((radius + tube) * 2.0);
            desc.radius = radius + tube;
            desc.half_height = tube;
        }
        "ellipsoid" => {
            let x = number(object.get("radiusX"), 1.0);
            let y = number(object.get("radiusY"), 1.0);
            let z = number(object.get("radiusZ"), 1.0);
            desc.size = bevy::math::Vec3::new(x * 2.0, y * 2.0, z * 2.0);
            desc.radius = x.min(y).min(z);
            desc.half_height = y;
        }
        "scripted" | "custom" | "line" | "points" => {
            desc.half_height = 0.0;
            desc.radius = 0.0;
            desc.size = bevy::math::Vec3::ZERO;
        }
        _ => {}
    }
    Some(desc)
}

pub fn physics(props: &ObjectProperties) -> Option<PhysicsDesc> {
    let value = props.physics.as_ref()?;
    if value.as_bool() == Some(false) || value.as_str() == Some("none") {
        return None;
    }
    if let Some(body) = value.as_str() {
        return Some(PhysicsDesc {
            body: body_kind(body),
            collider: "auto".to_string(),
        });
    }
    let object = value.as_object()?;
    let body = object
        .get("body")
        .and_then(Value::as_str)
        .map(body_kind)
        .unwrap_or(BodyKind::Dynamic);
    let collider = object
        .get("collider")
        .and_then(|value| value.as_str().or_else(|| value.get("kind").and_then(Value::as_str)))
        .unwrap_or("auto")
        .to_string();
    Some(PhysicsDesc { body, collider })
}

fn body_kind(value: &str) -> BodyKind {
    match value {
        "static" => BodyKind::Static,
        "kinematic" => BodyKind::Kinematic,
        "character" => BodyKind::Character,
        "dynamic" => BodyKind::Dynamic,
        _ => BodyKind::None,
    }
}

pub fn feet_position(props: &ObjectProperties) -> bevy::math::Vec3 {
    let Some(value) = props.feet_position.as_ref() else {
        return bevy::math::Vec3::ZERO;
    };
    let Some(object) = value.as_object() else {
        return vec3(Some(value));
    };
    if let Some(tile) = object.get("tile").and_then(Value::as_array) {
        let x = tile.first().and_then(Value::as_f64).unwrap_or(0.0) as f32;
        let z = tile.get(1).and_then(Value::as_f64).unwrap_or(0.0) as f32;
        let offset = vec3(object.get("offset"));
        return bevy::math::Vec3::new(x + offset.x, offset.y, z + offset.z);
    }
    let x = number(object.get("x"), 0.0);
    let z = number(object.get("z"), 0.0);
    let y = match object.get("y") {
        Some(Value::String(text)) if text == "surface" => 0.0,
        Some(Value::Object(terrain)) => number(terrain.get("terrain"), 0.0),
        other => number(other, 0.0),
    };
    bevy::math::Vec3::new(x, y, z)
}

pub fn center_height(props: &ObjectProperties, desc: Option<&PrimitiveDesc>) -> f32 {
    let half = desc.map(|desc| desc.half_height).unwrap_or(0.0);
    match props.pivot.as_ref() {
        Some(Value::String(mode)) if mode == "center" => 0.0,
        Some(Value::String(mode)) if mode == "top" => -half,
        Some(Value::Array(array)) if array.len() >= 2 => -(array[1].as_f64().unwrap_or(0.0) as f32),
        Some(Value::Object(object)) => -number(object.get("y"), 0.0),
        _ => half,
    }
}

pub fn model_path(props: &ObjectProperties) -> Option<Value> {
    props.model.clone()
}

pub fn scripted_primitive(props: &ObjectProperties) -> Option<(String, Value)> {
    let object = props.primitive.as_ref()?.as_object()?;
    if object.get("kind")?.as_str()? != "scripted" {
        return None;
    }
    let script = object.get("script")?.as_str()?.to_string();
    Some((script, object.get("params").cloned().unwrap_or(Value::Null)))
}
