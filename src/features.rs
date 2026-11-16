use bevy::prelude::*;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

#[derive(Resource, Clone, Debug, Default)]
pub struct FeatureState {
    pub tick: u64,
    pub clock_ms: f64,
    pub dt_ms: f64,
    pub gap_ms: f64,
    pub random_state: u64,
    pub physics_worlds: BTreeMap<String, PhysicsWorldState>,
    pub voxel_parts: BTreeMap<String, VoxelPartState>,
    pub fx: BTreeMap<String, FxState>,
    pub vibe: BTreeMap<String, VibeState>,
    pub jobs: BTreeMap<String, JobState>,
    pub trades: Vec<TradeReceipt>,
    pub dm: Vec<Value>,
    pub probes: BTreeMap<String, Value>,
    pub next_id: u64,
    pub view_constants: Map<String, Value>,
    pub feature_calls: u64,
}

#[derive(Clone, Debug, Default)]
pub struct PhysicsWorldState {
    pub gravity: [f32; 3],
    pub bodies: Vec<PhysicsBodyState>,
    pub snapshot: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct PhysicsBodyState {
    pub body: String,
    pub collider: String,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub velocity: [f32; 3],
    pub angular_velocity: [f32; 3],
    pub size: [f32; 3],
    pub radius: f32,
    pub height: f32,
    pub mass: f32,
    pub friction: f32,
    pub restitution: f32,
    pub gravity_scale: f32,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub locked_axes: Vec<String>,
    pub sleeping: bool,
    pub pass_through: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct VoxelPartState {
    pub cells: BTreeMap<String, String>,
    pub palette: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct FxState {
    pub populations: Vec<Value>,
    pub params: Map<String, Value>,
    pub alive: u64,
    pub rate: f32,
    pub lifetime: f32,
    pub stopped: bool,
    pub pulses: Vec<Value>,
}

#[derive(Clone, Debug, Default)]
pub struct VibeState {
    pub playing: bool,
    pub bpm: f32,
    pub cps: f32,
    pub swing: f32,
    pub bar: u64,
    pub beat: u64,
    pub cycle: u64,
    pub voices: Vec<Value>,
    pub triggers: Vec<Value>,
}

#[derive(Clone, Debug, Default)]
pub struct JobState {
    pub id: String,
    pub job: String,
    pub args: Value,
    pub ok: bool,
    pub data: Value,
}

#[derive(Clone, Debug, Default)]
pub struct TradeReceipt {
    pub mine: String,
    pub with: String,
    pub for_item: String,
    pub accepted: bool,
}

impl FeatureState {
    pub fn seeded(seed: u64) -> Self {
        let mut state = Self::default();
        state.seed(seed);
        state
    }

    pub fn seed(&mut self, seed: u64) {
        self.random_state = seed ^ 0x9e3779b97f4a7c15;
        if self.random_state == 0 {
            self.random_state = 0x6a09e667f3bcc909;
        }
        self.view_constants.insert("quality".into(), Value::String("high".into()));
        self.view_constants.insert("device".into(), Value::String("desktop".into()));
        self.view_constants.insert("portal".into(), Value::Null);
    }

    pub fn advance(&mut self, dt: f64) {
        self.tick = self.tick.wrapping_add(1);
        self.dt_ms = dt * 1000.0;
        self.gap_ms = 0.0;
        self.clock_ms += self.dt_ms;
        for vibe in self.vibe.values_mut() {
            vibe.bar = vibe.beat / 4;
            vibe.cycle = vibe.beat / 16;
        }
    }

    pub fn random(&mut self) -> f64 {
        let mut x = self.random_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.random_state = x;
        (x >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn id(&mut self, prefix: &str) -> String {
        self.next_id = self.next_id.wrapping_add(1);
        format!("{prefix}-{}", self.next_id)
    }

    pub fn probe(&mut self, name: &str, payload: Value) -> Value {
        self.feature_calls += 1;
        let count = self.probes.entry(name.to_string()).or_insert_with(|| Value::from(0u64));
        if let Some(value) = count.as_u64() {
            *count = Value::from(value + 1);
        }
        let mut result = Map::new();
        result.insert("name".into(), Value::String(name.to_string()));
        result.insert("tick".into(), Value::from(self.tick));
        result.insert("payload".into(), payload);
        result.insert("calls".into(), count.clone());
        Value::Object(result)
    }

    pub fn physics_create(&mut self, gravity: [f32; 3]) -> Value {
        let id = self.id("physics-world");
        self.physics_worlds.insert(id.clone(), PhysicsWorldState { gravity, ..Default::default() });
        let mut value = Map::new();
        value.insert("__nativeWorld".into(), Value::String(id.clone()));
        value.insert("id".into(), Value::String(id.clone()));
        value.insert("gravity".into(), vec_json(gravity));
        value.insert("bodies".into(), Value::Array(Vec::new()));
        Value::Object(value)
    }

    pub fn physics_add(&mut self, id: &str, spec: &Map<String, Value>) -> Value {
        let body = PhysicsBodyState {
            body: string_field(spec, "body", "dynamic"),
            collider: string_field(spec, "collider", "box"),
            position: vec_field(spec, "position", [0.0, 0.0, 0.0]),
            rotation: quat_field(spec.get("rotation")),
            velocity: vec_field(spec, "velocity", [0.0, 0.0, 0.0]),
            angular_velocity: vec_field(spec, "angularVelocity", [0.0, 0.0, 0.0]),
            size: vec_field(spec, "size", [1.0, 1.0, 1.0]),
            radius: number_field(spec, "radius", 0.5),
            height: number_field(spec, "height", 1.0),
            mass: number_field(spec, "mass", 1.0),
            friction: number_field(spec, "friction", 0.5),
            restitution: number_field(spec, "restitution", 0.0),
            gravity_scale: number_field(spec, "gravityScale", 1.0),
            linear_damping: number_field(spec, "linearDamping", 0.0),
            angular_damping: number_field(spec, "angularDamping", 0.0),
            locked_axes: string_array(spec.get("lockedAxes")),
            sleeping: false,
            pass_through: Vec::new(),
        };
        let Some(world) = self.physics_worlds.get_mut(id) else {
            return Value::Null;
        };
        let index = world.bodies.len();
        world.bodies.push(body.clone());
        let mut value = Map::new();
        value.insert("__nativeWorld".into(), Value::String(id.to_string()));
        value.insert("__nativeBody".into(), Value::from(index));
        value.insert("id".into(), Value::String(format!("{id}:body-{index}")));
        value.insert("position".into(), vec_json(body.position));
        value.insert("rotation".into(), quat_json(body.rotation));
        value.insert("velocity".into(), vec_json(body.velocity));
        value.insert("angularVelocity".into(), vec_json(body.angular_velocity));
        value.insert("passThrough".into(), Value::Array(Vec::new()));
        value.insert("sleeping".into(), Value::Bool(false));
        Value::Object(value)
    }

    pub fn physics_step(&mut self, id: &str, dt: f32) -> Value {
        let Some(world) = self.physics_worlds.get_mut(id) else {
            return Value::Null;
        };
        let gravity = world.gravity;
        for body in &mut world.bodies {
            if body.body == "static" || body.sleeping {
                continue;
            }
            let damping = (1.0 - body.linear_damping * dt).max(0.0);
            for axis in 0..3 {
                if body.locked_axes.iter().any(|name| name == ["x", "y", "z"][axis]) {
                    body.velocity[axis] = 0.0;
                } else {
                    body.velocity[axis] = (body.velocity[axis] + gravity[axis] * body.gravity_scale * dt) * damping;
                    body.position[axis] += body.velocity[axis] * dt;
                }
            }
            for axis in 0..3 {
                let locked = body.locked_axes.iter().any(|name| name == ["rotX", "rotY", "rotZ"][axis]);
                if !locked {
                    body.angular_velocity[axis] *= (1.0 - body.angular_damping * dt).max(0.0);
                }
            }
        }
        Value::from(world.bodies.len())
    }

    pub fn physics_save(&mut self, id: &str) -> Value {
        let Some(world) = self.physics_worlds.get_mut(id) else {
            return Value::Array(Vec::new());
        };
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&world.bodies.len().to_le_bytes());
        for body in &world.bodies {
            for value in body.position.iter().chain(body.velocity.iter()).chain(body.angular_velocity.iter()) {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            bytes.push(u8::from(body.sleeping));
        }
        world.snapshot = bytes.clone();
        Value::Array(bytes.into_iter().map(|byte| Value::from(byte as u64)).collect())
    }

    pub fn physics_restore(&mut self, id: &str, bytes: &[u8]) -> Value {
        let Some(world) = self.physics_worlds.get_mut(id) else {
            return Value::Bool(false);
        };
        if bytes.len() < 8 {
            return Value::Bool(false);
        }
        let count = u64::from_le_bytes(bytes[0..8].try_into().unwrap()) as usize;
        let stride = 9 * 4 + 1;
        if bytes.len() < 8 + count.saturating_mul(stride) {
            return Value::Bool(false);
        }
        for (index, body) in world.bodies.iter_mut().take(count).enumerate() {
            let base = 8 + index * stride;
            for (axis, target) in body.position.iter_mut().enumerate() {
                let at = base + axis * 4;
                *target = f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
            }
            for (axis, target) in body.velocity.iter_mut().enumerate() {
                let at = base + 12 + axis * 4;
                *target = f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
            }
            for (axis, target) in body.angular_velocity.iter_mut().enumerate() {
                let at = base + 24 + axis * 4;
                *target = f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
            }
            body.sleeping = bytes[base + 36] != 0;
        }
        world.snapshot = bytes.to_vec();
        Value::Bool(true)
    }

    pub fn voxel_part(&mut self) -> Value {
        let id = self.id("voxel-part");
        self.voxel_parts.insert(id.clone(), VoxelPartState::default());
        let mut value = Map::new();
        value.insert("__nativePart".into(), Value::String(id.clone()));
        value.insert("id".into(), Value::String(id));
        Value::Object(value)
    }

    pub fn voxel_set(&mut self, id: &str, payload: &Map<String, Value>) -> Value {
        let Some(part) = self.voxel_parts.get_mut(id) else {
            return Value::Null;
        };
        let x = number_field(payload, "x", 0.0) as i32;
        let y = number_field(payload, "y", 0.0) as i32;
        let z = number_field(payload, "z", 0.0) as i32;
        let material = string_field(payload, "material", "stone");
        part.cells.insert(format!("{x},{y},{z}"), material.clone());
        if !part.palette.contains(&material) {
            part.palette.push(material);
        }
        self.voxel_json(id).unwrap_or(Value::Null)
    }

    pub fn voxel_json(&self, id: &str) -> Option<Value> {
        let part = self.voxel_parts.get(id)?;
        let mut bounds_min = [i32::MAX; 3];
        let mut bounds_max = [i32::MIN; 3];
        let mut voxels = Vec::new();
        for (key, material) in &part.cells {
            let coords = key.split(',').filter_map(|value| value.parse::<i32>().ok()).collect::<Vec<_>>();
            if coords.len() != 3 {
                continue;
            }
            for axis in 0..3 {
                bounds_min[axis] = bounds_min[axis].min(coords[axis]);
                bounds_max[axis] = bounds_max[axis].max(coords[axis]);
            }
            let palette = part.palette.iter().position(|value| value == material).unwrap_or(0);
            voxels.push(Value::Array(vec![Value::from(coords[0]), Value::from(coords[1]), Value::from(coords[2]), Value::from(palette)]));
        }
        let bounds = if voxels.is_empty() {
            Value::Null
        } else {
            Value::Array(vec![Value::Array(bounds_min.iter().copied().map(Value::from).collect()), Value::Array(bounds_max.iter().copied().map(Value::from).collect())])
        };
        let mut value = Map::new();
        value.insert("__nativePart".into(), Value::String(id.to_string()));
        value.insert("version".into(), Value::from(1));
        value.insert("bounds".into(), bounds);
        value.insert("palette".into(), Value::Array(part.palette.iter().cloned().map(Value::String).collect()));
        value.insert("voxels".into(), Value::Array(voxels));
        Some(Value::Object(value))
    }

    pub fn fx_create(&mut self, payload: &Map<String, Value>) -> Value {
        let id = payload.get("id").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| self.id("fx"));
        let rate = payload.get("rate").and_then(Value::as_f64).unwrap_or(1.0) as f32;
        let lifetime = payload.get("lifetime").and_then(Value::as_f64).unwrap_or(1.0) as f32;
        let populations = payload.get("populations").and_then(Value::as_array).cloned().unwrap_or_default();
        let params = payload.get("params").and_then(Value::as_object).cloned().unwrap_or_default();
        self.fx.insert(id.clone(), FxState { populations, params, rate, lifetime, ..Default::default() });
        self.fx_state_json(&id).unwrap_or(Value::Null)
    }

    pub fn fx_state_json(&self, id: &str) -> Option<Value> {
        let state = self.fx.get(id)?;
        let mut value = Map::new();
        value.insert("id".into(), Value::String(id.to_string()));
        value.insert("alive".into(), Value::from(state.alive));
        value.insert("rate".into(), Value::from(state.rate));
        value.insert("lifetime".into(), Value::from(state.lifetime));
        value.insert("expectedAlive".into(), Value::from((state.rate * state.lifetime) as u64));
        value.insert("stopped".into(), Value::Bool(state.stopped));
        value.insert("pulses".into(), Value::Array(state.pulses.clone()));
        Some(Value::Object(value))
    }

    pub fn vibe_snapshot(&self, id: &str) -> Option<Value> {
        let state = self.vibe.get(id)?;
        let mut value = Map::new();
        value.insert("playing".into(), Value::Bool(state.playing));
        value.insert("bpm".into(), Value::from(state.bpm));
        value.insert("bar".into(), Value::from(state.bar));
        value.insert("barPos".into(), Value::from((state.beat % 4) as f32 / 4.0));
        value.insert("beat".into(), Value::from(state.beat));
        value.insert("beatPos".into(), Value::from(0.0));
        value.insert("cycle".into(), Value::from(state.cycle));
        value.insert("cyclePos".into(), Value::from((state.beat % 16) as f32 / 16.0));
        value.insert("voices".into(), Value::Array(state.voices.clone()));
        value.insert("triggers".into(), Value::Array(state.triggers.clone()));
        Some(Value::Object(value))
    }

    pub fn report(&self) -> Value {
        let mut value = Map::new();
        value.insert("tick".into(), Value::from(self.tick));
        value.insert("featureCalls".into(), Value::from(self.feature_calls));
        value.insert("physicsWorlds".into(), Value::from(self.physics_worlds.len()));
        value.insert("voxelParts".into(), Value::from(self.voxel_parts.len()));
        value.insert("fx".into(), Value::from(self.fx.len()));
        value.insert("vibe".into(), Value::from(self.vibe.len()));
        value.insert("jobs".into(), Value::from(self.jobs.len()));
        value.insert("trades".into(), Value::from(self.trades.len()));
        value.insert("dm".into(), Value::from(self.dm.len()));
        value.insert("probes".into(), Value::Object(self.probes.iter().map(|(key, value)| (key.clone(), value.clone())).collect()));
        value.insert("randomState".into(), Value::from(self.random_state));
        value.insert("physicsBodies".into(), Value::from(self.physics_worlds.values().map(|world| world.bodies.len()).sum::<usize>()));
        value.insert("physicsSnapshots".into(), Value::from(self.physics_worlds.values().filter(|world| !world.snapshot.is_empty()).count()));
        value.insert("voxelCells".into(), Value::from(self.voxel_parts.values().map(|part| part.cells.len()).sum::<usize>()));
        value.insert("fxPopulations".into(), Value::from(self.fx.values().map(|state| state.populations.len()).sum::<usize>()));
        value.insert("fxParams".into(), Value::from(self.fx.values().map(|state| state.params.len()).sum::<usize>()));
        value.insert("vibeVoices".into(), Value::from(self.vibe.values().map(|state| state.voices.len()).sum::<usize>()));
        value.insert("vibeTriggers".into(), Value::from(self.vibe.values().map(|state| state.triggers.len()).sum::<usize>()));
        value.insert("vibePlaying".into(), Value::from(self.vibe.values().filter(|state| state.playing).count()));
        value.insert("vibeCps".into(), Value::from(self.vibe.values().map(|state| state.cps as f64).sum::<f64>()));
        value.insert("vibeSwing".into(), Value::from(self.vibe.values().map(|state| state.swing as f64).sum::<f64>()));
        value.insert("jobData".into(), Value::Array(self.jobs.values().map(|state| json!({ "id": state.id, "job": state.job, "args": state.args, "ok": state.ok, "data": state.data })).collect()));
        value.insert("tradeReceipts".into(), Value::Array(self.trades.iter().map(|receipt| json!({ "mine": receipt.mine, "with": receipt.with, "for": receipt.for_item, "accepted": receipt.accepted })).collect()));
        value.insert("bodyStates".into(), Value::Array(self.physics_worlds.values().flat_map(|world| world.bodies.iter()).map(|body| json!({ "body": body.body, "collider": body.collider, "mass": body.mass, "friction": body.friction, "restitution": body.restitution, "gravityScale": body.gravity_scale, "linearDamping": body.linear_damping, "angularDamping": body.angular_damping, "lockedAxes": body.locked_axes, "passThrough": body.pass_through, "sleeping": body.sleeping, "size": body.size, "radius": body.radius, "height": body.height, "rotation": body.rotation })).collect()));
        Value::Object(value)
    }
}

pub fn vec_json(value: [f32; 3]) -> Value {
    let mut map = Map::new();
    map.insert("x".into(), Value::from(value[0]));
    map.insert("y".into(), Value::from(value[1]));
    map.insert("z".into(), Value::from(value[2]));
    Value::Object(map)
}

fn quat_json(value: [f32; 4]) -> Value {
    let mut map = Map::new();
    map.insert("x".into(), Value::from(value[0]));
    map.insert("y".into(), Value::from(value[1]));
    map.insert("z".into(), Value::from(value[2]));
    map.insert("w".into(), Value::from(value[3]));
    Value::Object(map)
}

fn string_field(map: &Map<String, Value>, key: &str, fallback: &str) -> String {
    map.get(key).and_then(Value::as_str).unwrap_or(fallback).to_string()
}

fn number_field(map: &Map<String, Value>, key: &str, fallback: f32) -> f32 {
    map.get(key).and_then(Value::as_f64).unwrap_or(fallback as f64) as f32
}

fn vec_field(map: &Map<String, Value>, key: &str, fallback: [f32; 3]) -> [f32; 3] {
    match map.get(key) {
        Some(Value::Array(items)) => [num(items.get(0), fallback[0]), num(items.get(1), fallback[1]), num(items.get(2), fallback[2])],
        Some(Value::Object(object)) => [num(object.get("x"), fallback[0]), num(object.get("y"), fallback[1]), num(object.get("z"), fallback[2])],
        _ => fallback,
    }
}

fn quat_field(value: Option<&Value>) -> [f32; 4] {
    match value {
        Some(Value::Array(items)) => [num(items.get(0), 0.0), num(items.get(1), 0.0), num(items.get(2), 0.0), num(items.get(3), 1.0)],
        Some(Value::Object(object)) if object.contains_key("w") => [num(object.get("x"), 0.0), num(object.get("y"), 0.0), num(object.get("z"), 0.0), num(object.get("w"), 1.0)],
        Some(Value::Object(object)) => [num(object.get("x"), 0.0), num(object.get("y"), 0.0), num(object.get("z"), 0.0), 1.0],
        Some(Value::Number(number)) => {
            let yaw = number.as_f64().unwrap_or(0.0) as f32;
            [0.0, (yaw.to_radians() * 0.5).sin(), 0.0, (yaw.to_radians() * 0.5).cos()]
        }
        _ => [0.0, 0.0, 0.0, 1.0],
    }
}

fn num(value: Option<&Value>, fallback: f32) -> f32 {
    value.and_then(Value::as_f64).unwrap_or(fallback as f64) as f32
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    value.and_then(Value::as_array).map(|items| items.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::FeatureState;

    #[test]
    fn seeded_random_sequence_repeats_per_seed() {
        let first: Vec<f64> = {
            let mut state = FeatureState::seeded(7);
            (0..4).map(|_| state.random()).collect()
        };
        let again: Vec<f64> = {
            let mut state = FeatureState::seeded(7);
            (0..4).map(|_| state.random()).collect()
        };
        let other: Vec<f64> = {
            let mut state = FeatureState::seeded(8);
            (0..4).map(|_| state.random()).collect()
        };
        assert_eq!(first, again);
        assert_ne!(first, other);
    }

    #[test]
    fn feature_report_is_a_json_object() {
        let report = FeatureState::seeded(1).report();
        assert!(report.is_object());
    }
}
