use crate::spec::{self, ObjectProperties, ObjectSpec};
use crate::runtime::{
    AnimState, AudioState, CameraView, CollisionQueue, InputState, JointLinks, TerrainData,
    WorldState,
};
use crate::world::{
    self, Behaviors, ModelRef, ObjectId, ObjectMetrics, ObjectState, PlaceTag, PrimitiveInfo,
    RemainingLifetime, ScriptedPrimitive, Tags, WorldData, WorldRoot,
};
use avian3d::prelude::*;
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use rquickjs::loader::{ImportAttributes, Loader, Resolver};
use rquickjs::{Array, Context, Ctx, Function, IntoJs, Module, Object, Persistent, Runtime, Value};
use serde_json::{json, Map, Value as Json};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;

const PRELUDE: &str = include_str!("prelude.js");

thread_local! {
    static ENGINE: RefCell<Option<ScriptEngine>> = const { RefCell::new(None) };
    static ENGINE_FAILED: Cell<bool> = const { Cell::new(false) };
}

pub fn run_scripts(world: &mut World) {
    if ENGINE_FAILED.with(Cell::get) {
        return;
    }
    if ENGINE.with(|cell| cell.borrow().is_none()) {
        let root = world
            .get_resource::<WorldRoot>()
            .map(|root| root.0.clone())
            .unwrap_or_default();
        match ScriptEngine::new(&root) {
            Ok(engine) => ENGINE.with(|cell| *cell.borrow_mut() = Some(engine)),
            Err(error) => {
                ENGINE_FAILED.with(|flag| flag.set(true));
                println!("{{\"kind\":\"script-engine-error\",\"error\":{}}}", quote(&error));
                return;
            }
        }
    }
    ENGINE.with(|cell| {
        let mut guard = cell.borrow_mut();
        if let Some(engine) = guard.as_mut() {
            engine.tick(world);
        }
    });
}

struct ScriptedInfo {
    entity: Entity,
    script: String,
}

struct Timer {
    id: String,
    entity: Entity,
    script: String,
    due_ms: f64,
    hook: String,
    payload: String,
}

struct Listener {
    id: u64,
    entity: Entity,
    name: String,
    index: u64,
}

struct PendingEvent {
    name: String,
    payload: String,
}

struct Host {
    world: *mut World,
    root: PathBuf,
    place: String,
    now_ms: f64,
    dt: f64,
    tick: u64,
    rng: u64,
    next_object: u64,
    index: HashMap<String, Entity>,
    ids: HashMap<Entity, String>,
    scripts: HashMap<String, Option<Persistent<Object<'static>>>>,
    failed: HashSet<String>,
    schedules: HashMap<String, Option<f64>>,
    initialized: HashSet<(Entity, String)>,
    scripted_done: HashSet<Entity>,
    sleep_until: HashMap<Entity, f64>,
    next_schedule: HashMap<(Entity, String), f64>,
    sessions: HashMap<String, Map<String, Json>>,
    views: HashMap<String, Map<String, Json>>,
    extras: HashMap<String, Map<String, Json>>,
    world_state: Map<String, Json>,
    place_states: HashMap<String, Map<String, Json>>,
    timers: Vec<Timer>,
    next_timer: u64,
    listeners: Vec<Listener>,
    next_listener: u64,
    events: Vec<PendingEvent>,
    current_entity: Option<Entity>,
    current_script: Option<String>,
    next_sound: u64,
    hooks_called: u64,
    errors: u64,
    host_calls: BTreeMap<String, u64>,
    api_provenance: BTreeMap<String, String>,
    policy_done: String,
    destroyed: Vec<(String, String, Vec<String>)>,
    ui_signature: String,
    ui_html: String,
    state_before: HashMap<String, Json>,
    changed_log: Vec<Json>,
    pending_leave: Option<(String, String, String)>,
}

impl Host {
    fn new(root: &Path) -> Self {
        Self {
            world: std::ptr::null_mut(),
            root: root.to_path_buf(),
            place: String::new(),
            now_ms: 0.0,
            dt: 1.0 / 64.0,
            tick: 0,
            rng: 0x9e37_79b9_7f4a_7c15,
            next_object: 0,
            index: HashMap::new(),
            ids: HashMap::new(),
            scripts: HashMap::new(),
            failed: HashSet::new(),
            schedules: HashMap::new(),
            initialized: HashSet::new(),
            scripted_done: HashSet::new(),
            sleep_until: HashMap::new(),
            next_schedule: HashMap::new(),
            sessions: HashMap::new(),
            views: HashMap::new(),
            extras: HashMap::new(),
            world_state: Map::new(),
            place_states: HashMap::new(),
            timers: Vec::new(),
            next_timer: 0,
            listeners: Vec::new(),
            next_listener: 0,
            events: Vec::new(),
            current_entity: None,
            current_script: None,
            next_sound: 0,
            hooks_called: 0,
            errors: 0,
            host_calls: BTreeMap::new(),
            api_provenance: BTreeMap::new(),
            policy_done: String::new(),
            destroyed: Vec::new(),
            ui_signature: String::new(),
            ui_html: String::new(),
            state_before: HashMap::new(),
            changed_log: Vec::new(),
            pending_leave: None,
        }
    }

    fn refresh(&mut self) {
        let world = unsafe { &mut *self.world };
        let mut query = world.query::<(Entity, &ObjectId)>();
        let rows = query
            .iter(world)
            .map(|(entity, id)| (entity, id.0.clone()))
            .collect::<Vec<_>>();
        self.index.clear();
        self.ids.clear();
        for (entity, id) in rows {
            self.index.insert(id.clone(), entity);
            self.ids.insert(entity, id);
        }
    }

    fn behavior_entities(&self) -> Vec<(Entity, Vec<String>)> {
        let world = unsafe { &mut *self.world };
        let mut query = world.query::<(Entity, &Behaviors)>();
        query
            .iter(world)
            .filter(|(_, behaviors)| !behaviors.0.is_empty())
            .map(|(entity, behaviors)| (entity, behaviors.0.clone()))
            .collect()
    }

    fn scripted_entities(&self) -> Vec<ScriptedInfo> {
        let world = unsafe { &mut *self.world };
        let mut query = world.query::<(Entity, &ScriptedPrimitive)>();
        query
            .iter(world)
            .map(|(entity, scripted)| ScriptedInfo {
                entity,
                script: scripted.script.clone(),
            })
            .collect()
    }

    fn id_of(&self, entity: Entity) -> Option<String> {
        self.ids.get(&entity).cloned()
    }

    fn world(&self) -> &World {
        assert!(!self.world.is_null(), "script host world pointer cleared");
        unsafe { &*self.world }
    }

    fn world_mut(&mut self) -> &mut World {
        assert!(!self.world.is_null(), "script host world pointer cleared");
        unsafe { &mut *self.world }
    }

    fn session_map(&mut self, id: &str) -> &mut Map<String, Json> {
        self.sessions.entry(id.to_string()).or_default()
    }

    fn view_map(&mut self, id: &str) -> &mut Map<String, Json> {
        self.views.entry(id.to_string()).or_default()
    }

    fn extras_map(&mut self, id: &str) -> &mut Map<String, Json> {
        self.extras.entry(id.to_string()).or_default()
    }

    fn place_state_map(&mut self, place: &str) -> &mut Map<String, Json> {
        self.place_states.entry(place.to_string()).or_default()
    }

    fn get_json(&mut self, path: &str, key: &str) -> String {
        if path == "world" {
            return self.get_world(key);
        }
        if path == "world:state" {
            return self.world_state.get(key).map(json_string).unwrap_or_default();
        }
        if let Some(id) = path.strip_prefix("state:") {
            return self.get_state(id, key);
        }
        if let Some(id) = path.strip_prefix("session:") {
            return self.session_map(id).get(key).map(json_string).unwrap_or_default();
        }
        if let Some(id) = path.strip_prefix("anim:") {
            return if key.is_empty() {
                self.anim_json(id)
            } else {
                self.world()
                    .get::<AnimState>(self.index.get(id).copied().unwrap_or(Entity::PLACEHOLDER))
                    .and_then(|state| state.0.get(key))
                    .map(json_string)
                    .unwrap_or_default()
            };
        }
        if let Some(id) = path.strip_prefix("view:") {
            return self.get_view(id, key);
        }
        if let Some(id) = path.strip_prefix("params:") {
            return self.get_params(id, key);
        }
        if let Some(id) = path.strip_prefix("obj:") {
            return self.get_object(id, key);
        }
        if let Some(rest) = path.strip_prefix("place:") {
            if let Some(place) = rest.strip_suffix(":state") {
                return self.place_state_map(place).get(key).map(json_string).unwrap_or_default();
            }
            return self.get_place(rest, key);
        }
        String::new()
    }

    fn set_json(&mut self, path: &str, key: &str, text: &str) {
        let value = parse_json(text);
        if path == "world" {
            let mut state = self.world_mut().resource_mut::<WorldState>();
            match key {
                "state" => state.state = value.as_object().cloned().unwrap_or_default(),
                "ui" => state.ui = value.as_object().cloned().unwrap_or_default(),
                "camera" => state.camera = value.as_object().cloned().unwrap_or_default(),
                "inputs" => state.inputs = value.as_object().cloned().unwrap_or_default(),
                "mode" => state.mode = value.as_str().unwrap_or("dev").to_string(),
                "settling" => state.settling = value.as_bool().unwrap_or(false),
                "defaultPlace" => state.default_place = value.as_str().unwrap_or("").to_string(),
                _ => {
                    state.extra.insert(key.to_string(), value);
                }
            }
            return;
        }
        if path == "world:state" {
            self.world_state.insert(key.to_string(), value);
            return;
        }
        if let Some(id) = path.strip_prefix("state:") {
            self.set_state(id, key, value);
            return;
        }
        if let Some(id) = path.strip_prefix("session:") {
            self.session_map(id).insert(key.to_string(), value);
            return;
        }
        if let Some(id) = path.strip_prefix("anim:") {
            self.set_anim(id, key, value);
            return;
        }
        if let Some(id) = path.strip_prefix("view:") {
            self.view_map(id).insert(key.to_string(), value);
            return;
        }
        if let Some(id) = path.strip_prefix("obj:") {
            self.set_object(id, key, value);
            return;
        }
        if let Some(rest) = path.strip_prefix("place:") {
            if let Some(place) = rest.strip_suffix(":state") {
                self.place_state_map(place).insert(key.to_string(), value);
            }
        }
    }

    fn delete_json(&mut self, path: &str, key: &str) {
        if path == "world" {
            let mut state = self.world_mut().resource_mut::<WorldState>();
            match key {
                "state" => state.state.remove(key),
                "ui" => state.ui.remove(key),
                "camera" => state.camera.remove(key),
                "inputs" => state.inputs.remove(key),
                _ => state.extra.remove(key),
            };
            return;
        }
        if path == "world:state" {
            self.world_state.remove(key);
            return;
        }
        if let Some(id) = path.strip_prefix("state:") {
            if let Some(entity) = self.index.get(id).copied() {
                if let Some(mut state) = self.world_mut().get_mut::<ObjectState>(entity) {
                    state.0.remove(key);
                }
            }
            return;
        }
        if let Some(id) = path.strip_prefix("session:") {
            self.session_map(id).remove(key);
            return;
        }
        if let Some(id) = path.strip_prefix("anim:") {
            self.set_anim(id, key, Json::Null);
            return;
        }
        if let Some(id) = path.strip_prefix("view:") {
            self.view_map(id).remove(key);
            return;
        }
        if let Some(id) = path.strip_prefix("obj:") {
            self.extras_map(id).remove(key);
            return;
        }
        if let Some(rest) = path.strip_prefix("place:") {
            if let Some(place) = rest.strip_suffix(":state") {
                self.place_state_map(place).remove(key);
            }
        }
    }

    fn has_json(&mut self, path: &str, key: &str) -> bool {
        if path == "world" {
            return !self.get_world(key).is_empty();
        }
        if path == "world:state" {
            return self.world_state.contains_key(key);
        }
        if let Some(id) = path.strip_prefix("state:") {
            return self
                .index
                .get(id)
                .and_then(|entity| self.world().get::<ObjectState>(*entity))
                .is_some_and(|state| state.0.contains_key(key));
        }
        if let Some(id) = path.strip_prefix("session:") {
            return self.session_map(id).contains_key(key);
        }
        if let Some(id) = path.strip_prefix("anim:") {
            return self
                .index
                .get(id)
                .and_then(|entity| self.world().get::<AnimState>(*entity))
                .is_some_and(|state| state.0.contains_key(key));
        }
        if let Some(id) = path.strip_prefix("obj:") {
            return !self.get_object(id, key).is_empty();
        }
        if let Some(rest) = path.strip_prefix("place:") {
            if let Some(place) = rest.strip_suffix(":state") {
                return self.place_state_map(place).contains_key(key);
            }
        }
        false
    }

    fn keys_json(&mut self, path: &str) -> String {
        let keys = if path == "world:state" {
            self.world_state.keys().cloned().collect::<Vec<_>>()
        } else if let Some(id) = path.strip_prefix("state:") {
            self.index
                .get(id)
                .and_then(|entity| self.world().get::<ObjectState>(*entity))
                .map(|state| state.0.keys().cloned().collect::<Vec<_>>())
                .unwrap_or_default()
        } else if let Some(id) = path.strip_prefix("session:") {
            self.session_map(id).keys().cloned().collect::<Vec<_>>()
        } else if let Some(id) = path.strip_prefix("anim:") {
            self.index
                .get(id)
                .and_then(|entity| self.world().get::<AnimState>(*entity))
                .map(|state| state.0.keys().cloned().collect::<Vec<_>>())
                .unwrap_or_default()
        } else if let Some(rest) = path.strip_prefix("place:") {
            rest.strip_suffix(":state")
                .map(|place| self.place_state_map(place).keys().cloned().collect::<Vec<_>>())
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        json_string(&Json::Array(keys.into_iter().map(Json::String).collect()))
    }

    fn get_state(&mut self, id: &str, key: &str) -> String {
        self.index
            .get(id)
            .and_then(|entity| self.world().get::<ObjectState>(*entity))
            .and_then(|state| state.0.get(key))
            .map(json_string)
            .unwrap_or_default()
    }

    fn set_state(&mut self, id: &str, key: &str, value: Json) {
        if let Some(entity) = self.index.get(id).copied() {
            if let Some(mut state) = self.world_mut().get_mut::<ObjectState>(entity) {
                state.0.insert(key.to_string(), value);
            }
        }
    }

    fn replace_state(&mut self, id: &str, text: &str) {
        let value = parse_json(text);
        let map = value.as_object().cloned().unwrap_or_default();
        if let Some(entity) = self.index.get(id).copied() {
            if let Some(mut state) = self.world_mut().get_mut::<ObjectState>(entity) {
                state.0 = map;
            }
        }
    }

    fn get_params(&mut self, id: &str, key: &str) -> String {
        self.index
            .get(id)
            .and_then(|entity| self.world().get::<ScriptedPrimitive>(*entity))
            .and_then(|scripted| scripted.params.as_object())
            .and_then(|params| params.get(key))
            .map(json_string)
            .unwrap_or_default()
    }

    fn get_view(&mut self, id: &str, key: &str) -> String {
        if let Some(value) = self.view_map(id).get(key) {
            return json_string(value);
        }
        let camera = self
            .world()
            .get_resource::<WorldState>()
            .map(|state| state.camera.clone())
            .unwrap_or_default();
        let state = camera.get("state").and_then(Json::as_object);
        if let Some(value) = state.and_then(|state| state.get(key)).or_else(|| camera.get(key)) {
            return json_string(value);
        }
        match key {
            "yaw" => "0".to_string(),
            "pitch" => "-0.25".to_string(),
            "camera" => "null".to_string(),
            _ => String::new(),
        }
    }

    fn get_world(&mut self, key: &str) -> String {
        let (mode, default_place, settling, ui, engine, state_map, inputs) = {
            let world = self.world();
            let place = world.get_resource::<WorldData>();
            let state = world.get_resource::<WorldState>();
            (
                state.map(|state| state.mode.clone()).unwrap_or_else(|| "dev".to_string()),
                state
                    .map(|state| state.default_place.clone())
                    .or_else(|| place.map(|place| place.0.name.clone()))
                    .unwrap_or_else(|| "main".to_string()),
                state.map(|state| state.settling).unwrap_or(false),
                state.map(|state| state.ui.clone()).unwrap_or_default(),
                state.map(|state| state.extra.clone()).unwrap_or_default(),
                state.map(|state| state.state.clone()).unwrap_or_default(),
                state.map(|state| state.inputs.clone()).unwrap_or_default(),
            )
        };
        match key {
            "mode" => json_string(&Json::String(mode)),
            "version" => "null".to_string(),
            "id" => "null".to_string(),
            "address" => "null".to_string(),
            "players" => self.player_objects_json(),
            "playerCount" | "onlineCount" => parse_json(&self.player_objects_json())
                .as_array()
                .map(|list| list.len())
                .unwrap_or(0)
                .to_string(),
            "date" => json_string(&Json::String(String::new())),
            "places" => self.places_json(),
            "engine" => engine
                .get("engine")
                .map(json_string)
                .unwrap_or_else(|| json_string(&Json::Object(engine.clone()))),
            "passThrough" => "{}".to_string(),
            "routing" => "{}".to_string(),
            "ui" => json_string(&Json::Object(ui)),
            "player" => engine
                .get("player")
                .cloned()
                .map(|value| json_string(&value))
                .unwrap_or_else(|| "{}".to_string()),
            "godMode" => "{}".to_string(),
            "creatorTabs" => "[]".to_string(),
            "defaultPlace" => json_string(&Json::String(default_place)),
            "settling" => settling.to_string(),
            "files" => self.file_list(),
            "assets" => "{}".to_string(),
            "camera" => self.camera_json(),
            "state" => json_string(&Json::Object(state_map)),
            "inputs" => json_string(&Json::Object(inputs)),
            _ => String::new(),
        }
    }

    fn get_place(&mut self, name: &str, key: &str) -> String {
        let config = {
            let world = self.world();
            world.get_resource::<WorldData>().map(|place| place.0.config.clone())
        };
        match key {
            "id" | "name" => json_string(&Json::String(name.to_string())),
            "players" => self.players_json("place"),
            "playerCount" => parse_json(&self.players_json("place"))
                .as_array()
                .map(|list| list.len())
                .unwrap_or(0)
                .to_string(),
            "gravity" => config
                .as_ref()
                .and_then(|config| config.gravity.clone())
                .map(|value| json_string(&value))
                .unwrap_or_else(|| json_string(&json!({"x":0.0,"y":-9.81,"z":0.0}))),
            "tickBase" => config
                .as_ref()
                .and_then(|config| config.tick_base)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "64".to_string()),
            "damping" => config
                .as_ref()
                .and_then(|config| config.damping)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "0".to_string()),
            "mode" => config
                .as_ref()
                .and_then(|config| config.mode.clone())
                .map(|value| json_string(&Json::String(value)))
                .unwrap_or_else(|| "\"3d\"".to_string()),
            "timeScale" => "1".to_string(),
            "atmosphere" => config
                .as_ref()
                .and_then(|config| config.atmosphere.clone())
                .map(|value| json_string(&value))
                .unwrap_or_else(|| "{}".to_string()),
            "terrain" => config
                .as_ref()
                .and_then(|config| config.terrain.clone())
                .map(|value| json_string(&value))
                .unwrap_or_else(|| "{}".to_string()),
            "fields" => self.fields_json(Some(name)),
            "objects" => self.place_objects_json(name),
            "state" => self
                .place_states
                .get(name)
                .map(|state| json_string(&Json::Object(state.clone())))
                .unwrap_or_else(|| "{}".to_string()),
            "voxels" | "tiles" => "{}".to_string(),
            "policy" => config
                .as_ref()
                .and_then(|config| config.policy.clone())
                .map(|value| json_string(&value))
                .unwrap_or_else(|| "null".to_string()),
            "defaultJayzeePoint" => config
                .as_ref()
                .and_then(|config| config.default_jayzee_point.clone())
                .map(|value| json_string(&value))
                .unwrap_or_else(|| "null".to_string()),
            "defaultJayzeeRotation" => config
                .as_ref()
                .and_then(|config| config.default_jayzee_rotation.clone())
                .map(|value| json_string(&value))
                .unwrap_or_else(|| "null".to_string()),
            _ => String::new(),
        }
    }

    fn get_object(&mut self, id: &str, key: &str) -> String {
        let Some(entity) = self.index.get(id).copied() else {
            return String::new();
        };
        let world = self.world();
        let position = world.get::<Position>(entity).copied().unwrap_or_default();
        let rotation = world.get::<Rotation>(entity).copied().unwrap_or_default();
        let transform = world.get::<Transform>(entity).copied().unwrap_or_default();
        let metrics = world.get::<ObjectMetrics>(entity).copied().unwrap_or_default();
        let euler = rotation.0.to_euler(EulerRot::YXZ);
        let yaw = euler.0.to_degrees();
        match key {
            "id" | "objectId" | "userId" => json_string(&Json::String(id.to_string())),
            "name" | "displayName" => json_string(&Json::String(id.to_string())),
            "isLocal" => "true".to_string(),
            "isGodMode" | "isEditor" => "false".to_string(),
            "arrival" => json_string(&json!({"kind": "join"})),
            "pocket" => self
                .extras
                .get(id)
                .and_then(|map| map.get("pocket"))
                .map(json_string)
                .unwrap_or_else(|| "{}".to_string()),
            "tags" => world
                .get::<Tags>(entity)
                .map(|tags| json_string(&Json::Array(tags.0.iter().cloned().map(Json::String).collect())))
                .unwrap_or_else(|| "[]".to_string()),
            "state" => "{}".to_string(),
            "feetPosition" | "worldFeetPosition" => {
                let feet = position.0 - Vec3::Y * metrics.half_height * transform.scale.y;
                json_string(&vec3_json(feet))
            }
            "rotation" | "worldRotation" => json_string(&json!({
                "yaw": yaw,
                "pitch": euler.1.to_degrees(),
                "roll": euler.2.to_degrees(),
            })),
            "yaw" => yaw.to_string(),
            "scale" => json_string(&vec3_json(transform.scale)),
            "effectiveScale" => json_string(&vec3_json(transform.scale)),
            "velocity" => world
                .get::<LinearVelocity>(entity)
                .map(|velocity| json_string(&vec3_json(velocity.0)))
                .unwrap_or_else(|| json_string(&vec3_json(Vec3::ZERO))),
            "angularVelocity" => world
                .get::<AngularVelocity>(entity)
                .map(|velocity| json_string(&vec3_json(velocity.0)))
                .unwrap_or_else(|| json_string(&vec3_json(Vec3::ZERO))),
            "behavior" => world
                .get::<Behaviors>(entity)
                .map(|behaviors| json_string(&Json::Array(behaviors.0.iter().cloned().map(Json::String).collect())))
                .unwrap_or_else(|| "[]".to_string()),
            "model" => world
                .get::<ModelRef>(entity)
                .map(|model| json_string(&model.0))
                .unwrap_or_else(|| "null".to_string()),
            "place" => world
                .get::<PlaceTag>(entity)
                .map(|place| json_string(&Json::String(place.0.clone())))
                .unwrap_or_else(|| "null".to_string()),
            "lifetime" => world
                .get::<RemainingLifetime>(entity)
                .map(|lifetime| lifetime.0.to_string())
                .unwrap_or_else(|| "null".to_string()),
            "grounded" => world
                .get::<crate::runtime::GroundInfo>(entity)
                .map(|ground| ground.grounded.to_string())
                .unwrap_or_else(|| "false".to_string()),
            "touching" => world
                .get::<crate::runtime::GroundInfo>(entity)
                .map(|ground| json_string(&json!({
                    "wallLeft": ground.wall_left,
                    "wallRight": ground.wall_right,
                    "ceiling": ground.ceiling,
                })))
                .unwrap_or_else(|| json_string(&json!({"wallLeft":false,"wallRight":false,"ceiling":false}))),
            "overlapping" => world
                .get::<crate::runtime::OverlapSet>(entity)
                .map(|overlap| json_string(&Json::Array(overlap.0.iter().cloned().map(Json::String).collect())))
                .unwrap_or_else(|| "[]".to_string()),
            "liquid" => world
                .get::<crate::runtime::LiquidInfo>(entity)
                .map(|liquid| json_string(&json!({"in": liquid.inside, "level": liquid.level})))
                .unwrap_or_else(|| json_string(&json!({"in":false,"level":0.0}))),
            "anim" => world
                .get::<crate::runtime::AnimState>(entity)
                .map(|anim| json_string(&Json::Object(anim.0.clone())))
                .unwrap_or_else(|| "{}".to_string()),
            "mixer" => world
                .get::<crate::runtime::AnimState>(entity)
                .map(|anim| json_string(&Json::Object(anim.0.clone())))
                .unwrap_or_else(|| "{}".to_string()),
            "audio" => world
                .get::<crate::runtime::AudioState>(entity)
                .map(|audio| json_string(&audio.0))
                .unwrap_or_else(|| "null".to_string()),
            "joints" => self
                .extras
                .get(id)
                .and_then(|map| map.get("joints"))
                .map(json_string)
                .unwrap_or_else(|| "{}".to_string()),
            "ragdoll" => self
                .extras
                .get(id)
                .and_then(|map| map.get("ragdoll"))
                .map(json_string)
                .unwrap_or_else(|| "null".to_string()),
            "spline" => self
                .extras
                .get(id)
                .and_then(|map| map.get("spline"))
                .map(json_string)
                .unwrap_or_else(|| "null".to_string()),
            "bounds" => world
                .get::<PrimitiveInfo>(entity)
                .and_then(|primitive| primitive.0.as_ref())
                .map(|primitive| {
                    let scale = transform.scale;
                    let size = primitive.size * scale;
                    let center = position.0;
                    let min = center - size * 0.5;
                    let max = center + size * 0.5;
                    json_string(&json!({
                        "min": vec3_json(min),
                        "max": vec3_json(max),
                        "size": vec3_json(size),
                        "center": vec3_json(center),
                        "obb": {"center": vec3_json(center), "size": vec3_json(size)},
                    }))
                })
                .unwrap_or_else(|| "null".to_string()),
            "light" => world
                .get::<crate::world::LightValue>(entity)
                .map(|light| json_string(&light.0))
                .unwrap_or_else(|| "null".to_string()),
            "visible" | "render" | "visibleInFirstPerson" | "castShadow" | "receiveShadow" => {
                self.extras.get(id).and_then(|map| map.get(key)).map(json_string).unwrap_or_else(|| "true".to_string())
            }
            "sleeping" => world
                .get::<Sleeping>(entity)
                .map(|_| "true".to_string())
                .unwrap_or_else(|| "false".to_string()),
            "awake" => world
                .get::<Sleeping>(entity)
                .map(|_| "false".to_string())
                .unwrap_or_else(|| "true".to_string()),
            "controller" | "controlling" | "destination" | "measure" | "morph" => self
                .extras
                .get(id)
                .and_then(|map| map.get(key))
                .map(json_string)
                .unwrap_or_else(|| "null".to_string()),
            "triangles" => "null".to_string(),
            "skeleton" => "[]".to_string(),
            "env" | "gains" => "{}".to_string(),
            "forces" => world
                .get::<crate::runtime::ForceSet>(entity)
                .map(|forces| json_string(&Json::Object(forces.0.clone())))
                .unwrap_or_else(|| "{}".to_string()),
            "spectating" => "false".to_string(),
            "simulatedBy" => "\"server\"".to_string(),
            "audience" => "\"all\"".to_string(),
            "visibility" => "null".to_string(),
            _ => self
                .extras
                .get(id)
                .and_then(|map| map.get(key))
                .map(json_string)
                .unwrap_or_default(),
        }
    }

    fn wake(&mut self, entity: Entity) {
        self.sleep_until.remove(&entity);
    }

    fn set_object(&mut self, id: &str, key: &str, value: Json) {
        let Some(entity) = self.index.get(id).copied() else {
            return;
        };
        self.wake(entity);
        match key {
            "feetPosition" => {
                let feet = spec::vec3(Some(&value));
                let metrics = self
                    .world()
                    .get::<ObjectMetrics>(entity)
                    .copied()
                    .unwrap_or_default();
                let scale = self
                    .world()
                    .get::<Transform>(entity)
                    .map(|transform| transform.scale.y)
                    .unwrap_or(1.0);
                let center = feet + Vec3::Y * metrics.half_height * scale;
                let mut transform = self
                    .world()
                    .get::<Transform>(entity)
                    .copied()
                    .unwrap_or_default();
                transform.translation = center;
                let mut entity_mut = self.world_mut().entity_mut(entity);
                entity_mut.insert((Position::new(center), transform));
            }
            "rotation" | "yaw" => {
                let rotation = if let Some(look) = value.get("lookAt") {
                    let target = if let Some(id) = look.as_str() {
                        self.index
                            .get(id)
                            .and_then(|target| self.world().get::<Position>(*target))
                            .map(|position| position.0)
                    } else {
                        Some(spec::vec3(Some(look)))
                    };
                    let origin = self
                        .world()
                        .get::<Position>(entity)
                        .map(|position| position.0)
                        .unwrap_or_default();
                    target
                        .map(|target| {
                            Quat::from_rotation_arc(Vec3::NEG_Z, (target - origin).normalize_or_zero())
                        })
                        .unwrap_or(Quat::IDENTITY)
                } else {
                    spec::rotation(Some(&value))
                };
                let mut transform = self
                    .world()
                    .get::<Transform>(entity)
                    .copied()
                    .unwrap_or_default();
                transform.rotation = rotation;
                let mut entity_mut = self.world_mut().entity_mut(entity);
                entity_mut.insert((Rotation(rotation), transform));
            }
            "velocity" => {
                let velocity = spec::vec3(Some(&value));
                self.world_mut().entity_mut(entity).insert(LinearVelocity(velocity));
            }
            "angularVelocity" => {
                let velocity = spec::vec3(Some(&value));
                self.world_mut().entity_mut(entity).insert(AngularVelocity(velocity));
            }
            "behavior" => {
                let behaviors = value
                    .as_array()
                    .map(|array| array.iter().filter_map(|value| value.as_str().map(str::to_string)).collect())
                    .unwrap_or_default();
                self.world_mut().entity_mut(entity).insert(Behaviors(behaviors));
            }
            "model" => {
                if value.is_null() {
                    self.world_mut().entity_mut(entity).remove::<ModelRef>();
                } else {
                    self.world_mut().entity_mut(entity).insert(ModelRef(value));
                }
            }
            "scale" => {
                let scale = spec::scale(Some(&value));
                if let Some(mut transform) = self.world_mut().get_mut::<Transform>(entity) {
                    transform.scale = scale;
                }
            }
            "parent" => {
                if value.is_null() {
                    self.world_mut().entity_mut(entity).remove::<ChildOf>();
                } else if let Some(parent) = value.as_str().and_then(|parent| self.index.get(parent).copied()) {
                    self.world_mut().entity_mut(entity).insert(ChildOf(parent));
                }
            }
            "physics" => {
                let properties = ObjectProperties {
                    physics: Some(value.clone()),
                    ..Default::default()
                };
                if let Some(physics) = spec::physics(&properties) {
                    world::replace_physics(self.world_mut(), entity, &properties, &physics);
                } else {
                    self.world_mut().entity_mut(entity).remove::<(
                        RigidBody,
                        Collider,
                        LinearVelocity,
                        AngularVelocity,
                        LockedAxes,
                        Friction,
                        Restitution,
                        LinearDamping,
                        AngularDamping,
                        GravityScale,
                        Mass,
                        Sensor,
                        ColliderDensity,
                    )>();
                }
            }
            "state" => self.replace_state(id, &json_string(&value)),
            "anim" => {
                if let Some(channels) = value.as_object() {
                    let mut state = self
                        .world()
                        .get::<crate::runtime::AnimState>(entity)
                        .cloned()
                        .unwrap_or_default();
                    for (channel, entry) in channels {
                        if entry.is_null() {
                            state.0.remove(channel);
                        } else {
                            let mut entry = entry.clone();
                            if let Some(object) = entry.as_object_mut() {
                                object.entry("time").or_insert(Json::from(0.0));
                                object.entry("finished").or_insert(Json::Bool(false));
                            }
                            state.0.insert(channel.clone(), entry);
                        }
                    }
                    self.world_mut().entity_mut(entity).insert(state);
                }
            }
            "mixer" => {
                if let Some(channels) = value.as_object() {
                    let state = AnimState(channels.clone());
                    self.world_mut().entity_mut(entity).insert(state);
                }
            }
            "audio" => {
                self.world_mut().entity_mut(entity).insert(crate::runtime::AudioState(value));
            }
            "forces" => {
                self.world_mut().entity_mut(entity).insert(crate::runtime::ForceSet(
                    value.as_object().cloned().unwrap_or_default(),
                ));
            }
            "joints" => {
                self.extras_map(id).insert("joints".into(), value.clone());
                self.sync_joints(id);
            }
            "ragdoll" => {
                self.extras_map(id).insert("ragdoll".into(), value.clone());
                self.sync_joints(id);
            }
            "spline" => {
                self.extras_map(id).insert("spline".into(), value.clone());
                if value.is_null() {
                    self.world_mut().entity_mut(entity).remove::<crate::runtime::SplinePlacement>();
                } else {
                    let placement = crate::runtime::SplinePlacement {
                        id: value.get("id").and_then(Json::as_str).unwrap_or("").to_string(),
                        at: value.get("at").and_then(Json::as_f64).unwrap_or(0.0) as f32,
                        distance: value.get("space").and_then(Json::as_str) == Some("distance"),
                        orient: value.get("orient").is_some_and(|orient| !orient.is_null()),
                        offset: value
                            .get("offset")
                            .map(|offset| spec::vec3(Some(offset)))
                            .unwrap_or(Vec3::ZERO),
                    };
                    self.world_mut().entity_mut(entity).insert(placement);
                }
            }
            "controller" | "controlling" | "target" | "frame" | "destination"
            | "selection" | "visible" | "render" | "castShadow" | "receiveShadow"
            | "visibleInFirstPerson" | "visibleRange" | "layers" | "up" | "view" => {
                self.extras_map(id).insert(key.to_string(), value);
            }
            "tags" => {
                let tags = value
                    .as_array()
                    .map(|array| array.iter().filter_map(|value| value.as_str().map(str::to_string)).collect())
                    .unwrap_or_default();
                self.world_mut().entity_mut(entity).insert(Tags(tags));
            }
            "place" => {
                if let Some(place) = value.as_str() {
                    self.world_mut().entity_mut(entity).insert(PlaceTag(place.to_string()));
                }
            }
            "lifetime" => {
                if let Some(seconds) = value.as_f64() {
                    self.world_mut().entity_mut(entity).insert(RemainingLifetime(seconds as f32));
                }
            }
            "gravityScale" => {
                if let Some(scale) = value.as_f64() {
                    self.world_mut().entity_mut(entity).insert(GravityScale(scale as f32));
                }
            }
            "sleeping" => {
                if value.as_bool().unwrap_or(false) {
                    self.world_mut().entity_mut(entity).insert(Sleeping);
                } else {
                    self.world_mut().entity_mut(entity).remove::<Sleeping>();
                }
            }
            _ => {
                self.extras_map(id).insert(key.to_string(), value);
            }
        }
    }

    fn place_objects_json(&mut self, place: &str) -> String {
        let world = self.world_mut();
        let mut query = world.query::<(&ObjectId, &PlaceTag)>();
        let mut ids = query
            .iter(world)
            .filter(|(_, tag)| tag.0 == place)
            .map(|(id, _)| Json::String(id.0.clone()))
            .collect::<Vec<_>>();
        ids.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
        json_string(&Json::Array(ids))
    }

    fn player_objects_json(&mut self) -> String {
        let world = unsafe { &mut *self.world };
        let mut query = world.query::<(&ObjectId, &PlaceTag, &Tags)>();
        let players = query
            .iter(world)
            .filter(|(_, _, tags)| tags.0.iter().any(|tag| tag == "player"))
            .map(|(id, place, _)| {
                json!({
                    "id": id.0,
                    "userId": id.0,
                    "objectId": id.0,
                    "place": place.0,
                    "displayName": id.0,
                    "isLocal": true,
                })
            })
            .collect::<Vec<_>>();
        json_string(&Json::Array(players))
    }

    fn players_json(&mut self, _path: &str) -> String {
        let world = self.world_mut();
        let mut query = world.query::<(&ObjectId, &Tags)>();
        let mut ids = query
            .iter(world)
            .filter(|(_, tags)| tags.0.iter().any(|tag| tag == "player"))
            .map(|(id, _)| Json::String(id.0.clone()))
            .collect::<Vec<_>>();
        ids.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
        json_string(&Json::Array(ids))
    }

    fn iterate_json(&mut self, path: &str) -> String {
        if let Some(place) = path.strip_prefix("place:") {
            return self.place_objects_json(place.trim_end_matches(":objects"));
        }
        if path.ends_with(":players") {
            return self.players_json(path);
        }
        self.keys_json(path)
    }

    fn parent_of(&self, path: &str) -> Option<String> {
        let id = path.strip_prefix("obj:")?;
        let entity = self.index.get(id)?;
        let parent = self.world().get::<ChildOf>(*entity)?.parent();
        self.ids.get(&parent).cloned()
    }

    fn children_of(&self, path: &str) -> String {
        let Some(id) = path.strip_prefix("obj:") else {
            return "[]".to_string();
        };
        let Some(entity) = self.index.get(id) else {
            return "[]".to_string();
        };
        let children = self
            .world()
            .get::<Children>(*entity)
            .map(|children| {
                children
                    .iter()
                    .filter_map(|child| self.ids.get(&child).cloned())
                    .map(Json::String)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        json_string(&Json::Array(children))
    }

    fn target_of(&self, path: &str) -> Option<String> {
        let id = path.strip_prefix("obj:")?;
        if let Some(target) = self
            .extras
            .get(id)
            .and_then(|map| map.get("target"))
            .and_then(Json::as_str)
        {
            return Some(target.to_string());
        }
        self.index.get(id).and_then(|entity| {
            self.world()
                .get::<crate::runtime::CameraTarget>(*entity)
                .map(|target| target.0.clone())
        })
    }

    fn query_json(&mut self, options: &Json) -> String {
        let options = options.as_object().cloned().unwrap_or_default();
        let center = spec::vec3(options.get("center"));
        let radius = options.get("radius").and_then(Json::as_f64);
        let tags = string_list(options.get("tags"));
        let any_tags = string_list(options.get("anyTags"));
        let exclude_tags = string_list(options.get("excludeTags"));
        let place = options.get("place").and_then(Json::as_str).map(str::to_string);
        let object_id = options.get("objectId").and_then(Json::as_str).map(str::to_string);
        let ignore_self = options.get("ignoreSelf").and_then(Json::as_bool).unwrap_or(true);
        let self_id = self.current_entity.and_then(|entity| self.id_of(entity));
        let world = self.world_mut();
        let mut query = world.query::<(&ObjectId, &Tags, &PlaceTag, &Position)>();
        let mut ids = Vec::new();
        for (id, entity_tags, entity_place, position) in query.iter(world) {
            if ignore_self && self_id.as_deref() == Some(id.0.as_str()) {
                continue;
            }
            if let Some(place) = &place {
                if &entity_place.0 != place {
                    continue;
                }
            }
            if let Some(object_id) = &object_id {
                if &id.0 != object_id {
                    continue;
                }
            }
            if !tags.is_empty() && !tags.iter().all(|tag| entity_tags.0.contains(tag)) {
                continue;
            }
            if !any_tags.is_empty() && !any_tags.iter().any(|tag| entity_tags.0.contains(tag)) {
                continue;
            }
            if exclude_tags.iter().any(|tag| entity_tags.0.contains(tag)) {
                continue;
            }
            if let Some(radius) = radius {
                if position.0.distance(center) > radius as f32 {
                    continue;
                }
            }
            ids.push(Json::String(id.0.clone()));
        }
        ids.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
        json_string(&Json::Array(ids))
    }

    fn jayzee(&mut self, explicit: &str, spec_text: &str) -> String {
        let mut object: ObjectSpec = match serde_json::from_str(spec_text) {
            Ok(object) => object,
            Err(error) => {
                println!("{{\"kind\":\"jayzee-error\",\"error\":{}}}", quote(&error.to_string()));
                return String::new();
            }
        };
        self.next_object += 1;
        let parent = object.parent.clone();
        let id = if explicit.is_empty() {
            object
                .id
                .clone()
                .unwrap_or_else(|| format!("dyn-{}", self.next_object))
        } else {
            explicit.to_string()
        };
        object.id = Some(id.clone());
        let place = object.place.clone().unwrap_or_else(|| self.place.clone());
        let world = self.world_mut();
        let entity = world::spawn_into_world(world, &place, &id, &object);
        if let Some(parent) = parent.and_then(|parent| self.index.get(&parent).copied()) {
            self.world_mut().entity_mut(entity).insert(ChildOf(parent));
        }
        self.index.insert(id.clone(), entity);
        self.ids.insert(entity, id.clone());
        id
    }

    fn destroy(&mut self, value: &Json) {
        let ids = match value {
            Json::String(id) => vec![id.clone()],
            Json::Array(array) => array.iter().filter_map(|value| value.as_str().map(str::to_string)).collect(),
            Json::Object(object) => object
                .get("id")
                .and_then(Json::as_str)
                .map(|id| vec![id.to_string()])
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        for id in ids {
            if let Some(entity) = self.index.remove(&id) {
                let behaviors = self
                    .world()
                    .get::<Behaviors>(entity)
                    .map(|behaviors| behaviors.0.clone())
                    .unwrap_or_default();
                let place = self
                    .world()
                    .get::<PlaceTag>(entity)
                    .map(|place| place.0.clone())
                    .unwrap_or_else(|| self.place.clone());
                if !behaviors.is_empty() {
                    self.destroyed.push((id.clone(), place, behaviors));
                }
                self.ids.remove(&entity);
                self.initialized.retain(|(owner, _)| *owner != entity);
                self.scripted_done.remove(&entity);
                self.sessions.remove(&id);
                self.views.remove(&id);
                self.extras.remove(&id);
                let mut stack = vec![entity];
                while let Some(current) = stack.pop() {
                    if let Some(children) = self.world().get::<Children>(current) {
                        stack.extend(children.iter());
                    }
                    if self.world().get_entity(current).is_ok() {
                        self.world_mut().entity_mut(current).despawn();
                    }
                }
            }
        }
    }

    fn add_timer(&mut self, seconds: f64, hook: String, payload: String) -> String {
        self.next_timer += 1;
        let id = format!("t{}", self.next_timer);
        if let Some(entity) = self.current_entity {
            let script = self.current_script.clone().unwrap_or_default();
            self.timers.push(Timer {
                id: id.clone(),
                entity,
                script,
                due_ms: self.now_ms + seconds * 1000.0,
                hook,
                payload,
            });
        }
        id
    }

    fn add_listener(&mut self, name: String, index: u64, _options: String) -> u64 {
        self.next_listener += 1;
        let id = self.next_listener;
        let entity = self.current_entity.unwrap_or(Entity::PLACEHOLDER);
        self.listeners.push(Listener {
            id,
            entity,
            name,
            index,
        });
        id
    }

    fn off(&mut self, id: u64) {
        self.listeners.retain(|listener| listener.id != id);
    }

    fn accept_event(&mut self, name: &str, payload: String) -> String {
        if name == "playSound" {
            self.next_sound += 1;
            return format!("sound-{}", self.next_sound);
        }
        self.events.push(PendingEvent {
            name: name.to_string(),
            payload,
        });
        String::new()
    }

    fn state_maps(&mut self) -> HashMap<String, Json> {
        let world = unsafe { &mut *self.world };
        let mut query = world.query::<(&ObjectId, &ObjectState)>();
        query
            .iter(world)
            .map(|(id, state)| (id.0.clone(), Json::Object(state.0.clone())))
            .collect()
    }

    fn changed_json(&mut self, glob: &str) -> String {
        let pattern = glob.trim_matches('*');
        let matched = self
            .changed_log
            .iter()
            .filter(|entry| {
                pattern.is_empty()
                    || entry
                        .get("path")
                        .and_then(Json::as_str)
                        .is_some_and(|path| path.contains(pattern))
            })
            .cloned()
            .collect::<Vec<_>>();
        self.changed_log.clear();
        json_string(&Json::Array(matched))
    }

    fn diff_states(&mut self) {
        let before = std::mem::take(&mut self.state_before);
        let after = self.state_maps();
        for (id, state) in &after {
            let previous = before.get(id);
            let prev_map = previous.and_then(Json::as_object);
            let Some(map) = state.as_object() else { continue; };
            for (key, value) in map {
                let prev = prev_map.and_then(|prev| prev.get(key));
                if prev == Some(value) {
                    continue;
                }
                self.changed_log.push(json!({
                    "path": format!("{id}.state.{key}"),
                    "value": value,
                    "prev": prev.cloned().unwrap_or(Json::Null),
                }));
            }
            if let Some(prev_map) = prev_map {
                for key in prev_map.keys() {
                    if !map.contains_key(key) {
                        self.changed_log.push(json!({
                            "path": format!("{id}.state.{key}"),
                            "value": Json::Null,
                            "prev": prev_map.get(key).cloned().unwrap_or(Json::Null),
                        }));
                    }
                }
            }
        }
        self.state_before = after;
    }

    fn host_op(&mut self, op: &str, payload: &str) -> String {
        let payload: Json = parse_json(payload);
        if op == "apiCall" {
            if let Some(name) = payload.get("op").and_then(Json::as_str) {
                *self.host_calls.entry(name.to_string()).or_insert(0) += 1;
                self.api_provenance
                    .entry(name.to_string())
                    .or_insert_with(|| host_provenance(name).to_string());
            }
            return String::new();
        }
        match op {
            "raycast" => self.query_ray(&payload, false),
            "raycastAll" => self.query_ray(&payload, true),
            "overlapPoint" => self.query_point(&payload),
            "overlapSegment" => self.query_segment(&payload),
            "traverseCheck" => self.traverse_check(&payload),
            "jumpCheck" => self.jump_check(&payload),
            "inputRay" => self.input_ray(),
            "setJoint" => {
                let id = payload.get("id").and_then(Json::as_str).unwrap_or("");
                self.extras_map(id).insert("joints".into(), payload.get("joints").cloned().unwrap_or(Json::Null));
                self.sync_joints(id);
                String::new()
            }
            "setRagdoll" => {
                let id = payload.get("id").and_then(Json::as_str).unwrap_or("");
                let value = payload.get("value").cloned().unwrap_or(Json::Null);
                self.extras_map(id).insert("ragdoll".into(), value);
                self.sync_joints(id);
                String::new()
            }
            "setAnim" => {
                let id = payload.get("id").and_then(Json::as_str).unwrap_or("");
                let channel = payload.get("channel").and_then(Json::as_str).unwrap_or("");
                let value = payload.get("value").cloned().unwrap_or(Json::Null);
                self.set_anim(id, channel, value);
                String::new()
            }
            "getAnim" => {
                let id = payload.get("id").and_then(Json::as_str).unwrap_or("");
                self.anim_json(id)
            }
            "setAudio" => {
                let id = payload.get("id").and_then(Json::as_str).unwrap_or("");
                if let Some(entity) = self.index.get(id).copied() {
                    let value = payload.get("value").cloned().unwrap_or(Json::Null);
                    self.world_mut().entity_mut(entity).insert(AudioState(value.clone()));
                    if let Some(object) = value.as_object() {
                        if object.get("bus").and_then(Json::as_str) != Some("Sfx") {
                            self.world_mut().resource_mut::<crate::runtime::AudioTransport>().music = Some(value);
                        }
                    }
                }
                String::new()
            }
            "getAudio" => {
                let id = payload.get("id").and_then(Json::as_str).unwrap_or("");
                self.index
                    .get(id)
                    .and_then(|entity| self.world().get::<AudioState>(*entity))
                    .map(|audio| json_string(&audio.0))
                    .unwrap_or_else(|| "null".to_string())
            }
            "enterPlace" => self.enter_place(
                payload.get("id").and_then(Json::as_str),
                payload.get("place").and_then(Json::as_str).unwrap_or(""),
            ),
            "cross" => {
                let from = payload.get("id").and_then(Json::as_str).unwrap_or("");
                let place = payload.get("place").and_then(Json::as_str).unwrap_or("");
                self.cross(from, place)
            }
            "readField" => self.read_field(&payload),
            "terrainHeight" => {
                let mut query = payload.clone();
                if let Some(object) = query.as_object_mut() {
                    if !object.contains_key("name") {
                        object.insert("name".into(), Json::String("terrain".into()));
                    }
                }
                self.read_field(&query)
            }
            "waterLevel" => self.water_level(&payload),
            "terrainNormal" => self.terrain_normal(&payload),
            "fieldsJson" => self.fields_json(payload.get("place").and_then(Json::as_str)),
            "worldToScreen" => self.world_to_screen(&payload),
            "cameraJson" => self.camera_json(),
            "placesJson" => self.places_json(),
            "placeConfigJson" => self.place_config_json(payload.get("place").and_then(Json::as_str)),
            "splineLength" => self.spline_length(payload.get("id").and_then(Json::as_str).unwrap_or("")),
            "sampleSpline" => self.sample_spline(&payload),
            "projectToSpline" => self.project_spline(&payload),
            "fileRead" => self.file_read(payload.get("path").and_then(Json::as_str).unwrap_or("")),
            "fileWrite" => self.file_write(payload.get("path").and_then(Json::as_str).unwrap_or(""), payload.get("text").and_then(Json::as_str).unwrap_or("")),
            "fileDelete" => self.file_delete(payload.get("path").and_then(Json::as_str).unwrap_or("")),
            "fileList" => self.file_list(),
            "commit" => {
                let message = payload.get("message").and_then(Json::as_str).unwrap_or("");
                self.extras_map("__world").insert("lastCommit".into(), Json::String(message.to_string()));
                Json::Bool(true).to_string()
            }
            "setInput" => self.set_input(&payload),
            "placeObjects" => {
                let place = payload
                    .get("place")
                    .and_then(Json::as_str)
                    .map(str::to_string)
                    .unwrap_or_else(|| self.place.clone());
                self.place_objects_json(&place)
            }
            "spawnPlayer" => self.spawn_player(&payload),
            "featureProbe" => self.native_feature_probe(&payload),
            "physicsWorldCreate" => self.native_physics_create(&payload),
            "physicsWorldAdd" => self.native_physics_add(&payload),
            "physicsWorldStep" => self.native_physics_step(&payload),
            "physicsWorldSave" => self.native_physics_save(&payload),
            "physicsWorldRestore" => self.native_physics_restore(&payload),
            "voxelPart" => self.native_voxel_part(),
            "voxelSet" => self.native_voxel_set(&payload),
            "voxelJson" => self.native_voxel_json(&payload),
            "fxCreate" => self.native_fx_create(&payload),
            "fxState" => self.native_fx_state(&payload),
            "tradeReceipt" => self.native_trade_receipt(&payload),
            "dmNotify" => self.native_dm_notify(&payload),
            "jobRecord" => self.native_job_record(&payload),
            "timing" => self.native_timing(),
            "viewConstant" => self.native_view_constant(&payload),
            "featureRandom" => json_string(&Json::from(self.world_mut().resource_mut::<crate::features::FeatureState>().random())),
            "nativeRandom" => json_string(&Json::from(self.world_mut().resource_mut::<crate::features_ecs::NativeRandom>().next())),
            "nativeMath" => self.native_math(&payload),
            "nativeVector" => self.native_vector(&payload),
            "nativeNoise" => self.native_noise(&payload),
            "nativeFeatureCounts" => self.native_feature_counts(),
            "vibeSnapshot" => self.native_vibe_snapshot(&payload),
            "globeConfigure" | "globeInfo" | "globeToCartesian" | "globeToSpherical" | "globeElevation" | "globeChunk" | "globeChunks" | "globeGravity" | "globeOrientation" | "globeNavigation" => self.native_globe(op, &payload),
            _ => String::new(),
        }
    }

    fn native_globe(&mut self, op: &str, payload: &Json) -> String {
        let command = op.strip_prefix("globe").unwrap_or("info");
        let data = payload.as_object().cloned().unwrap_or_default();
        let registry = self.world().resource::<crate::globe::GlobeRegistry>().clone();
        let value = {
            let mut globe = self.world_mut().resource_mut::<crate::globe::GlobeWorld>();
            crate::globe::dispatch(&mut globe, &registry, command, &data)
        };
        json_string(&value)
    }

    fn native_feature_probe(&mut self, payload: &Json) -> String {
        let name = payload.get("name").and_then(Json::as_str).unwrap_or("feature");
        let value = self.world_mut().resource_mut::<crate::features::FeatureState>().probe(name, payload.clone());
        json_string(&value)
    }

    fn native_physics_create(&mut self, payload: &Json) -> String {
        let gravity = payload.get("gravity").map(|value| json_vec3_array(value)).unwrap_or([0.0, -9.81, 0.0]);
        json_string(&self.world_mut().resource_mut::<crate::features::FeatureState>().physics_create(gravity))
    }

    fn native_physics_add(&mut self, payload: &Json) -> String {
        let id = payload.get("world").and_then(Json::as_str).unwrap_or("");
        let spec = payload.get("spec").and_then(Json::as_object).cloned().unwrap_or_default();
        json_string(&self.world_mut().resource_mut::<crate::features::FeatureState>().physics_add(id, &spec))
    }

    fn native_physics_step(&mut self, payload: &Json) -> String {
        let id = payload.get("world").and_then(Json::as_str).unwrap_or("");
        let dt = payload.get("dt").and_then(Json::as_f64).unwrap_or(1.0 / 64.0) as f32;
        json_string(&self.world_mut().resource_mut::<crate::features::FeatureState>().physics_step(id, dt))
    }

    fn native_physics_save(&mut self, payload: &Json) -> String {
        let id = payload.get("world").and_then(Json::as_str).unwrap_or("");
        json_string(&self.world_mut().resource_mut::<crate::features::FeatureState>().physics_save(id))
    }

    fn native_physics_restore(&mut self, payload: &Json) -> String {
        let id = payload.get("world").and_then(Json::as_str).unwrap_or("");
        let bytes = payload.get("bytes").and_then(Json::as_array).map(|items| items.iter().filter_map(|item| item.as_u64()).map(|byte| byte as u8).collect::<Vec<_>>()).unwrap_or_default();
        json_string(&self.world_mut().resource_mut::<crate::features::FeatureState>().physics_restore(id, &bytes))
    }

    fn native_voxel_part(&mut self) -> String {
        json_string(&self.world_mut().resource_mut::<crate::features::FeatureState>().voxel_part())
    }

    fn native_voxel_set(&mut self, payload: &Json) -> String {
        let id = payload.get("part").and_then(Json::as_str).unwrap_or("");
        let data = payload.as_object().cloned().unwrap_or_default();
        json_string(&self.world_mut().resource_mut::<crate::features::FeatureState>().voxel_set(id, &data))
    }

    fn native_voxel_json(&mut self, payload: &Json) -> String {
        let id = payload.get("part").and_then(Json::as_str).unwrap_or("");
        self.world().resource::<crate::features::FeatureState>().voxel_json(id).map(|value| json_string(&value)).unwrap_or_else(|| "null".to_string())
    }

    fn native_fx_create(&mut self, payload: &Json) -> String {
        let data = payload.as_object().cloned().unwrap_or_default();
        json_string(&self.world_mut().resource_mut::<crate::features::FeatureState>().fx_create(&data))
    }

    fn native_fx_state(&mut self, payload: &Json) -> String {
        let id = payload.get("id").and_then(Json::as_str).unwrap_or("");
        self.world().resource::<crate::features::FeatureState>().fx_state_json(id).map(|value| json_string(&value)).unwrap_or_else(|| "null".to_string())
    }

    fn native_trade_receipt(&mut self, payload: &Json) -> String {
        let mine = payload.get("mine").and_then(Json::as_str).unwrap_or("").to_string();
        let with = payload.get("with").and_then(Json::as_str).unwrap_or("").to_string();
        let for_item = payload.get("for").and_then(Json::as_str).unwrap_or("").to_string();
        let accepted = with.is_empty() || for_item.is_empty();
        let receipt = crate::features::TradeReceipt { mine: mine.clone(), with: with.clone(), for_item: for_item.clone(), accepted };
        self.world_mut().resource_mut::<crate::features::FeatureState>().trades.push(receipt);
        json_string(&json!({ "mine": mine, "with": with, "for": for_item, "accepted": accepted }))
    }

    fn native_dm_notify(&mut self, payload: &Json) -> String {
        self.world_mut().resource_mut::<crate::features::FeatureState>().dm.push(payload.clone());
        Json::Bool(true).to_string()
    }

    fn native_job_record(&mut self, payload: &Json) -> String {
        let id = payload.get("id").and_then(Json::as_str).unwrap_or("job").to_string();
        let job = payload.get("job").and_then(Json::as_str).unwrap_or("local").to_string();
        let args = payload.get("args").cloned().unwrap_or(Json::Null);
        let state = crate::features::JobState { id: id.clone(), job, args, ok: true, data: Json::Null };
        self.world_mut().resource_mut::<crate::features::FeatureState>().jobs.insert(id.clone(), state);
        json_string(&json!({ "id": id, "ok": true, "data": null }))
    }

    fn native_timing(&mut self) -> String {
        let state = self.world().resource::<crate::features::FeatureState>();
        json_string(&json!({ "now": state.clock_ms, "dtMs": state.dt_ms, "gapMs": state.gap_ms, "tick": state.tick }))
    }

    fn native_view_constant(&mut self, payload: &Json) -> String {
        let name = payload.get("name").and_then(Json::as_str).unwrap_or("");
        self.world().resource::<crate::features::FeatureState>().view_constants.get(name).cloned().map(|value| json_string(&value)).unwrap_or_else(|| "null".to_string())
    }

    fn native_vibe_snapshot(&mut self, payload: &Json) -> String {
        let id = payload.get("id").and_then(Json::as_str).unwrap_or("");
        self.world().resource::<crate::features::FeatureState>().vibe_snapshot(id).map(|value| json_string(&value)).unwrap_or_else(|| "null".to_string())
    }

    fn native_math(&mut self, payload: &Json) -> String {
        let op = payload.get("op").and_then(Json::as_str).unwrap_or("identity");
        let a = payload.get("a").and_then(Json::as_f64).unwrap_or(0.0);
        let b = payload.get("b").and_then(Json::as_f64).unwrap_or(0.0);
        let c = payload.get("c").and_then(Json::as_f64).unwrap_or(0.0);
        json_string(&self.world_mut().resource_mut::<crate::features_ecs::NativeMathService>().math(op, a, b, c))
    }

    fn native_vector(&mut self, payload: &Json) -> String {
        let op = payload.get("op").and_then(Json::as_str).unwrap_or("identity");
        let a = payload.get("a").map(json_vec3).unwrap_or(Vec3::ZERO);
        let b = payload.get("b").map(json_vec3).unwrap_or(Vec3::ZERO);
        let scale = payload.get("scale").and_then(Json::as_f64).unwrap_or(1.0);
        json_string(&self.world_mut().resource_mut::<crate::features_ecs::NativeMathService>().vector(op, a, b, scale))
    }

    fn native_noise(&mut self, payload: &Json) -> String {
        let op = payload.get("op").and_then(Json::as_str).unwrap_or("white2");
        let x = payload.get("x").and_then(Json::as_f64).unwrap_or(0.0);
        let z = payload.get("z").and_then(Json::as_f64).unwrap_or(0.0);
        let seed = payload.get("seed").and_then(Json::as_f64).unwrap_or(0.0);
        json_string(&self.world_mut().resource_mut::<crate::features_ecs::NativeMathService>().noise(op, x, z, seed))
    }

    fn native_feature_counts(&mut self) -> String {
        let world = self.world();
        json_string(&crate::features_ecs::entity_report(
            world.resource::<crate::features_ecs::NativeFeatureRegistry>(),
            world.resource::<crate::features_ecs::NativeClock>(),
            world.resource::<crate::features_ecs::NativeRandom>(),
            world.resource::<crate::features_ecs::NativeMathService>(),
        ))
    }

    fn anim_json(&mut self, id: &str) -> String {
        let Some(entity) = self.index.get(id).copied() else {
            return "null".to_string();
        };
        self.world()
            .get::<AnimState>(entity)
            .map(|state| json_string(&Json::Object(state.0.clone())))
            .unwrap_or_else(|| "{}".to_string())
    }

    fn set_anim(&mut self, id: &str, channel: &str, value: Json) {
        let Some(entity) = self.index.get(id).copied() else {
            return;
        };
        let mut state = self.world().get::<AnimState>(entity).cloned().unwrap_or_default();
        if value.is_null() {
            state.0.remove(channel);
        } else {
            let mut value = value;
            if let Some(object) = value.as_object_mut() {
                object.insert("time".into(), Json::from(0.0));
                object.insert("finished".into(), Json::Bool(false));
            }
            state.0.insert(channel.to_string(), value);
        }
        self.world_mut().entity_mut(entity).insert(state);
    }

    fn sync_joints(&mut self, id: &str) {
        let Some(entity) = self.index.get(id).copied() else {
            return;
        };
        let old = self
            .world()
            .get::<JointLinks>(entity)
            .cloned()
            .unwrap_or_default();
        for (_, joint) in &old.0 {
            self.world_mut().entity_mut(*joint).despawn();
        }
        let mut links = Vec::new();
        let joints = self
            .extras
            .get(id)
            .and_then(|map| map.get("joints"))
            .and_then(Json::as_object)
            .cloned()
            .unwrap_or_default();
        for (name, value) in joints {
            if let Some(joint) = self.build_joint(entity, &value) {
                links.push((name, joint));
            }
        }
        if let Some(ragdoll) = self.extras.get(id).and_then(|map| map.get("ragdoll")).cloned() {
            if let Some(object) = ragdoll.as_object() {
                let bones = object
                    .get("bones")
                    .and_then(Json::as_array)
                    .cloned()
                    .unwrap_or_default();
                for (index, bone) in bones.iter().enumerate() {
                    let name = bone.as_str().unwrap_or("bone").to_string();
                    let offset = object
                        .get("offsets")
                        .and_then(Json::as_array)
                        .and_then(|offsets| offsets.get(index))
                        .map(|value| spec::vec3(Some(value)))
                        .unwrap_or(Vec3::new(0.0, 0.3 * (index as f32 + 1.0), 0.0));
                    let parent = links.last().map(|(_, joint)| *joint);
                    let radius = object
                        .get("radius")
                        .and_then(Json::as_f64)
                        .map(|value| value as f32)
                        .unwrap_or(0.18);
                    let position = self
                        .world()
                        .get::<Position>(entity)
                        .map(|position| position.0 + offset)
                        .unwrap_or(offset);
                    let bone_entity = self.world_mut().spawn((
                        RigidBody::Dynamic,
                        Collider::sphere(radius),
                        Position::new(position),
                        Rotation::default(),
                        LinearVelocity::default(),
                        AngularVelocity::default(),
                    )).id();
                    let joint_entity = self.world_mut().spawn(SphericalJoint::new(entity, bone_entity)).id();
                    links.push((format!("bone-{name}"), joint_entity));
                    if let Some(parent) = parent {
                        let _ = parent;
                    }
                }
            }
        }
        self.world_mut().entity_mut(entity).insert(JointLinks(links));
    }

    fn build_joint(&mut self, body: Entity, value: &Json) -> Option<Entity> {
        let to = value.get("to").and_then(Json::as_str)?;
        let target = self.index.get(to).copied()?;
        let kind = value.get("type").and_then(Json::as_str).unwrap_or("fixed");
        let anchor = value
            .get("anchor")
            .or_else(|| value.get("position"))
            .and_then(spec::vec3_opt);
        let hinge = value.get("axis").and_then(spec::vec3_opt).unwrap_or(Vec3::Z);
        let limits = value.get("limits").and_then(Json::as_array);
        let motor = value.get("motor").and_then(Json::as_object);
        let world = self.world_mut();
        let joint = match kind {
            "distance" => {
                let mut joint = DistanceJoint::new(body, target);
                if let Some(anchor) = anchor {
                    joint = joint.with_anchor(anchor);
                }
                world.spawn(joint).id()
            }
            "revolute" => {
                let mut joint = RevoluteJoint::new(body, target);
                joint.hinge_axis = hinge;
                if let Some(limits) = limits {
                    if limits.len() >= 2 {
                        joint.angle_limit = Some(AngleLimit::new(
                            limits[0].as_f64().unwrap_or(0.0) as f32,
                            limits[1].as_f64().unwrap_or(0.0) as f32,
                        ));
                    }
                }
                if let Some(motor) = motor {
                    joint.motor.target_velocity = motor.get("velocity").and_then(Json::as_f64).unwrap_or(0.0) as f32;
                    joint.motor.max_torque = motor.get("maxForce").and_then(Json::as_f64).unwrap_or(f32::MAX as f64) as f32;
                }
                world.spawn(joint).id()
            }
            "prismatic" => {
                let mut joint = PrismaticJoint::new(body, target);
                joint.slider_axis = hinge;
                world.spawn(joint).id()
            }
            "spherical" => {
                let mut joint = SphericalJoint::new(body, target);
                if let Some(anchor) = anchor {
                    joint.frame1 = JointFrame::local(Isometry3d::from_translation(anchor));
                }
                world.spawn(joint).id()
            }
            _ => {
                let mut joint = FixedJoint::new(body, target);
                if let Some(anchor) = anchor {
                    joint.frame1 = JointFrame::local(Isometry3d::from_translation(anchor));
                }
                world.spawn(joint).id()
            }
        };
        Some(joint)
    }

    fn query_ray(&mut self, payload: &Json, multiple: bool) -> String {
        let origin = spec::vec3(payload.get("origin"));
        let direction = spec::vec3(payload.get("direction")).normalize_or_zero();
        if direction == Vec3::ZERO {
            return "null".to_string();
        }
        let distance = payload.get("distance").and_then(Json::as_f64).unwrap_or(200.0) as f32;
        let mut filter = SpatialQueryFilter::default();
        self.add_exclusions(&mut filter, payload);
        let world = self.world_mut();
        let mut state = SystemState::<(
            SpatialQuery,
            Query<&ObjectId>,
            Query<&ColliderOf>,
            Query<&RigidBody>,
            Query<&Tags>,
        )>::new(world);
        let (spatial, ids, colliders, bodies, tags) = state.get(world).unwrap();
        let mut results = Vec::new();
        let mut push = |entity: Entity, point: Vec3, normal: Vec3, toi: f32| {
            let body = colliders.get(entity).map(|link| link.body).unwrap_or(entity);
            if let Some(kind) = payload.get("bodies").and_then(Json::as_str) {
                let rigid = bodies.get(body).ok();
                let matches = match kind {
                    "static" => matches!(rigid, Some(RigidBody::Static)),
                    "dynamic" => matches!(rigid, Some(RigidBody::Dynamic) | Some(RigidBody::Kinematic)),
                    _ => true,
                };
                if !matches {
                    return;
                }
            }
            let id = ids.get(body).or_else(|_| ids.get(entity)).map(|id| id.0.clone()).unwrap_or_default();
            let entity_tags = tags
                .get(body)
                .or_else(|_| tags.get(entity))
                .map(|tags| tags.0.clone())
                .unwrap_or_default();
            if let Some(required) = payload.get("tags").and_then(Json::as_array) {
                if !required.iter().filter_map(Json::as_str).all(|tag| entity_tags.iter().any(|value| value == tag)) {
                    return;
                }
            }
            if let Some(any) = payload.get("anyTags").and_then(Json::as_array) {
                if !any.iter().filter_map(Json::as_str).any(|tag| entity_tags.iter().any(|value| value == tag)) {
                    return;
                }
            }
            if let Some(excluded) = payload.get("excludeTags").and_then(Json::as_array) {
                if excluded.iter().filter_map(Json::as_str).any(|tag| entity_tags.iter().any(|value| value == tag)) {
                    return;
                }
            }
            results.push(json!({
                "id": id,
                "entity": id,
                "point": vec3_json(point),
                "normal": vec3_json(normal),
                "distance": toi,
                "toi": toi,
            }));
        };
        if multiple {
            let hits = spatial.ray_hits(origin, Dir3::new(direction).unwrap_or(Dir3::X), distance, 32, true, &filter);
            for hit in hits {
                let point = origin + direction * hit.distance;
                push(hit.entity, point, hit.normal, hit.distance);
            }
        } else if let Some(hit) = spatial.cast_ray(origin, Dir3::new(direction).unwrap_or(Dir3::X), distance, true, &filter) {
            let point = origin + direction * hit.distance;
            push(hit.entity, point, hit.normal, hit.distance);
        }
        drop(state);
        if multiple {
            json_string(&Json::Array(results))
        } else {
            results.into_iter().next().map(|value| value.to_string()).unwrap_or_else(|| "null".to_string())
        }
    }

    fn add_exclusions(&self, filter: &mut SpatialQueryFilter, payload: &Json) {
        let mut excluded = Vec::new();
        if let Some(list) = payload.get("ignoreEntities").and_then(Json::as_array) {
            for id in list.iter().filter_map(Json::as_str) {
                if let Some(entity) = self.index.get(id) {
                    excluded.push(*entity);
                }
            }
        }
        if payload.get("ignoreSelf").and_then(Json::as_bool).unwrap_or(true) {
            if let Some(entity) = self.current_entity {
                excluded.push(entity);
            }
        }
        filter.excluded_entities = excluded.into_iter().collect();
    }

    fn query_point(&mut self, payload: &Json) -> String {
        let point = spec::vec3(payload.get("point"));
        let mut filter = SpatialQueryFilter::default();
        self.add_exclusions(&mut filter, payload);
        let world = self.world_mut();
        let mut state = SystemState::<(SpatialQuery, Query<&ObjectId>, Query<&ColliderOf>)>::new(world);
        let (spatial, ids, colliders) = state.get(world).unwrap();
        let mut results = Vec::new();
        for hit in spatial.point_intersections(point, &filter) {
            let body = colliders.get(hit).map(|link| link.body).unwrap_or(hit);
            if let Ok(id) = ids.get(body) {
                results.push(Json::String(id.0.clone()));
            }
        }
        json_string(&json!({"hits": results}))
    }

    fn query_segment(&mut self, payload: &Json) -> String {
        let a = spec::vec3(payload.get("a").or_else(|| payload.get("from")));
        let b = spec::vec3(payload.get("b").or_else(|| payload.get("to")));
        let delta = b - a;
        let distance = delta.length();
        if distance <= 0.0001 {
            return self.query_point(payload);
        }
        let mut ray = payload.clone();
        if let Some(object) = ray.as_object_mut() {
            object.insert("origin".into(), vec3_json(a));
            object.insert("direction".into(), vec3_json(delta / distance));
            object.insert("distance".into(), Json::from(distance));
            object.insert("multiple".into(), Json::Bool(true));
        }
        let hits = self.query_ray(&ray, true);
        let parsed = parse_json(&hits);
        json_string(&json!({"hits": parsed}))
    }

    fn traverse_check(&mut self, payload: &Json) -> String {
        let from = spec::vec3(payload.get("from"));
        let to = spec::vec3(payload.get("to"));
        let step = payload.get("step").and_then(Json::as_f64).unwrap_or(0.3) as f32;
        let mut probe = payload.clone();
        if let Some(object) = probe.as_object_mut() {
            object.insert("origin".into(), vec3_json(to + Vec3::Y * 0.4));
            object.insert("direction".into(), vec3_json(Vec3::NEG_Y));
            object.insert("distance".into(), Json::from(step + 2.0));
        }
        let hit = parse_json(&self.query_ray(&probe, false));
        let blocked = parse_json(&self.query_ray(&{
            let mut ray = payload.clone();
            if let Some(object) = ray.as_object_mut() {
                object.insert("origin".into(), vec3_json(from + Vec3::Y * 0.5));
                object.insert("direction".into(), vec3_json((to - from).normalize_or_zero()));
                object.insert("distance".into(), Json::from((to - from).length()));
            }
            ray
        }, false));
        let traversable = !blocked.is_object() && !hit.is_null();
        json_string(&json!({
            "traversable": traversable,
            "reason": if traversable { "ok" } else { "blocked" },
            "detail": {"ground": hit, "obstacle": blocked},
        }))
    }

    fn jump_check(&mut self, payload: &Json) -> String {
        let from = spec::vec3(payload.get("from"));
        let to = spec::vec3(payload.get("to"));
        let speed = payload.get("speed").and_then(Json::as_f64).unwrap_or(5.0) as f32;
        let jump = payload.get("jumpSpeed").and_then(Json::as_f64).unwrap_or(5.0) as f32;
        let gravity = self
            .world()
            .get_resource::<Gravity>()
            .map(|gravity| gravity.0)
            .unwrap_or(Vec3::NEG_Y * 9.81);
        let horizontal = Vec3::new(to.x - from.x, 0.0, to.z - from.z);
        let distance = horizontal.length();
        let time = if speed > 0.0 { distance / speed } else { f32::INFINITY };
        let rise = jump * time + 0.5 * gravity.y * time * time;
        let ok = rise >= to.y - from.y - 0.05;
        json_string(&json!({
            "ok": ok,
            "reachable": ok,
            "time": time,
            "rise": rise,
            "landing": vec3_json(from + horizontal.normalize_or_zero() * distance + Vec3::Y * rise),
        }))
    }

    fn input_ray(&mut self) -> String {
        let world = self.world();
        let camera = world.resource::<CameraView>();
        let eye = if camera.valid { camera.eye } else { Vec3::ZERO };
        let forward = if camera.valid { camera.forward } else { Vec3::NEG_Z };
        json_string(&json!({
            "origin": vec3_json(eye),
            "direction": vec3_json(forward),
            "source": if camera.valid { "camera" } else { "none" },
        }))
    }

    fn set_input(&mut self, payload: &Json) -> String {
        let mut input = self.world_mut().resource_mut::<InputState>();
        if let Some(axes) = payload.get("axes").and_then(Json::as_object) {
            for (name, value) in axes {
                input.axes.insert(name.clone(), value.clone());
            }
        }
        input.pressed = input_map(payload.get("pressed"));
        input.released = input_map(payload.get("released"));
        input.held = input_map(payload.get("held"));
        if let Some(actions) = payload.get("actionData").and_then(Json::as_object) {
            input.action_data = actions.clone();
        }
        input.dirty = true;
        String::new()
    }

    fn read_field(&mut self, payload: &Json) -> String {
        let name = payload.get("name").and_then(Json::as_str).unwrap_or("terrain");
        let x = payload.get("x").and_then(Json::as_f64).unwrap_or(0.0) as f32;
        let z = payload.get("z").and_then(Json::as_f64).unwrap_or(0.0) as f32;
        let world = self.world();
        world
            .get_resource::<TerrainData>()
            .and_then(|terrain| terrain.fields.get(name).map(|field| field.height(x, z)))
            .unwrap_or(0.0)
            .to_string()
    }

    fn water_level(&mut self, payload: &Json) -> String {
        let x = payload.get("x").and_then(Json::as_f64).unwrap_or(0.0) as f32;
        let z = payload.get("z").and_then(Json::as_f64).unwrap_or(0.0) as f32;
        let world = self.world();
        let level = world
            .get_resource::<TerrainData>()
            .and_then(|terrain| {
                terrain
                    .water
                    .iter()
                    .filter(|water| water.contains(x, z))
                    .map(|water| water.level)
                    .fold(None, |best: Option<f32>, level| Some(best.map_or(level, |best| best.max(level))))
            });
        level.map(|level| level.to_string()).unwrap_or_else(|| "null".to_string())
    }

    fn terrain_normal(&mut self, payload: &Json) -> String {
        let name = payload.get("name").and_then(Json::as_str).unwrap_or("terrain");
        let x = payload.get("x").and_then(Json::as_f64).unwrap_or(0.0) as f32;
        let z = payload.get("z").and_then(Json::as_f64).unwrap_or(0.0) as f32;
        let world = self.world();
        world
            .get_resource::<TerrainData>()
            .and_then(|terrain| terrain.fields.get(name).map(|field| json_string(&vec3_json(field.normal(x, z)))))
            .unwrap_or_else(|| "null".to_string())
    }

    fn fields_json(&mut self, _place: Option<&str>) -> String {
        let world = self.world();
        let terrain = world.get_resource::<TerrainData>();
        let mut fields = Map::new();
        if let Some(terrain) = terrain {
            for (name, field) in &terrain.fields {
                fields.insert(
                    name.clone(),
                    json!({
                        "grid": {"min": vec3_json(Vec3::new(field.min.x, 0.0, field.min.y)), "max": vec3_json(Vec3::new(field.max.x, 0.0, field.max.y)), "cell": field.cell},
                        "runtime": {},
                        "authored": {},
                    }),
                );
            }
        }
        json_string(&Json::Object(fields))
    }

    fn world_to_screen(&mut self, payload: &Json) -> String {
        let point = spec::vec3(payload.get("position").or_else(|| payload.get("point")));
        let world = self.world();
        let camera = world.resource::<CameraView>();
        if !camera.valid {
            return "null".to_string();
        }
        let relative = point - camera.eye;
        let forward = camera.forward.normalize_or_zero();
        let depth = relative.dot(forward);
        if depth <= 0.0 {
            return json_string(&json!({"x": 0.0, "y": 0.0, "depth": depth, "onScreen": false}));
        }
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        let up = right.cross(forward);
        let fov = (camera.fov.max(1.0)).to_radians();
        let tan = (fov * 0.5).tan().max(0.0001);
        let aspect = camera.aspect.max(0.0001);
        let ndc_x = relative.dot(right) / (depth * tan * aspect);
        let ndc_y = relative.dot(up) / (depth * tan);
        json_string(&json!({
            "x": (ndc_x * 0.5 + 0.5),
            "y": (ndc_y * 0.5 + 0.5),
            "depth": depth,
            "onScreen": ndc_x.abs() <= 1.0 && ndc_y.abs() <= 1.0,
        }))
    }

    fn camera_json(&mut self) -> String {
        let world = self.world();
        let camera = world.resource::<CameraView>();
        let state = world.get_resource::<WorldState>();
        let mut value = state.map(|state| state.camera.clone()).unwrap_or_default();
        value.insert("eye".into(), vec3_json(camera.eye));
        value.insert("aim".into(), vec3_json(camera.aim));
        value.insert("forward".into(), vec3_json(camera.forward));
        value.insert("fov".into(), Json::from(camera.fov));
        value.insert("aspect".into(), Json::from(camera.aspect));
        value.insert("insetRight".into(), Json::from(camera.inset_right));
        value.insert("valid".into(), Json::Bool(camera.valid));
        json_string(&Json::Object(value))
    }

    fn places_json(&mut self) -> String {
        let root = self.root.clone();
        let tree = crate::tree::WorldTree::load(&root);
        match tree {
            Ok(tree) => {
                let mut places = Map::new();
                for (name, place) in tree.places {
                    places.insert(
                        name,
                        json!({
                            "name": place.name,
                            "mode": place.config.mode,
                            "gravity": place.config.gravity,
                            "tickBase": place.config.tick_base,
                        }),
                    );
                }
                json_string(&Json::Object(places))
            }
            Err(_) => "{}".to_string(),
        }
    }

    fn place_config_json(&mut self, name: Option<&str>) -> String {
        let root = self.root.clone();
        let tree = match crate::tree::WorldTree::load(&root) {
            Ok(tree) => tree,
            Err(_) => return "{}".to_string(),
        };
        let name = name
            .map(str::to_string)
            .or_else(|| tree.config.default_place.clone())
            .unwrap_or_else(|| self.place.clone());
        tree.places
            .get(&name)
            .map(|place| {
                json_string(&json!({
                    "name": place.name,
                    "mode": place.config.mode,
                    "gravity": place.config.gravity,
                    "terrain": place.config.terrain,
                    "atmosphere": place.config.atmosphere,
                    "camera": place.config.camera,
                    "inputs": place.config.inputs,
                    "ui": place.config.ui,
                    "policy": place.config.policy,
                }))
            })
            .unwrap_or_else(|| "{}".to_string())
    }

    fn enter_place(&mut self, entity: Option<&str>, place: &str) -> String {
        if place.is_empty() {
            return "null".to_string();
        }
        let root = self.root.clone();
        let tree = match crate::tree::WorldTree::load(&root) {
            Ok(tree) => tree,
            Err(_) => return "null".to_string(),
        };
        let Some(target) = tree.places.get(place).cloned() else {
            return "null".to_string();
        };
        let previous = self.place.clone();
        let moving = entity.and_then(|id| self.index.get(id).copied());
        if let (Some(moving), Some(leave)) = (
            moving,
            crate::tree::WorldTree::load(&root)
                .ok()
                .and_then(|tree| tree.places.get(&previous).cloned())
                .and_then(|place| place.config.policy)
                .and_then(|policy| policy.get("onLeave").and_then(Json::as_str).map(str::to_string)),
        ) {
            let id = self.ids.get(&moving).cloned().unwrap_or_default();
            self.pending_leave = Some((leave, previous.clone(), id));
        }
        let ptr = self.world;
        let world: &mut World = unsafe { &mut *ptr };
        let mut query = world.query::<(Entity, &PlaceTag)>();
        let stale = query
            .iter(world)
            .filter(|(entity, tag)| tag.0 == previous && Some(*entity) != moving)
            .map(|(entity, _)| entity)
            .collect::<Vec<_>>();
        for entity in stale {
            world.entity_mut(entity).despawn();
        }
        world.resource_mut::<WorldData>().0 = target.clone();
        if let Some(value) = target.config.gravity.as_ref() {
            world.insert_resource(Gravity(spec::vec3(Some(value))));
        }

        self.policy_done.clear();
        if let Some(entity) = moving {
            world.entity_mut(entity).insert(PlaceTag(place.to_string()));
        }
        let mut entities = HashMap::new();
        let mut parents = Vec::new();
        for (index, object) in target.objects.iter().enumerate() {
            let id = object.id.clone().unwrap_or_else(|| format!("object-{index}"));
            world::spawn_tree(world, place, &id, object, &mut entities, &mut parents);
        }
        for (entity, parent) in parents {
            if let Some(parent) = entities.get(&parent) {
                world.entity_mut(entity).insert(ChildOf(*parent));
            }
        }
        let mut terrain = TerrainData::default();
        world::load_terrain(&target, &mut terrain);
        world.insert_resource(terrain);
        world::spawn_terrain_collider_now(world);
        let mut query = world.query::<(Entity, &ObjectId)>();
        let rebuilt = query
            .iter(world)
            .map(|(entity, id)| (entity, id.0.clone()))
            .collect::<Vec<_>>();
        self.place = place.to_string();
        self.index.clear();
        self.ids.clear();
        for (entity, id) in rebuilt {
            self.index.insert(id.clone(), entity);
            self.ids.insert(entity, id);
        }
        json_string(&Json::String(place.to_string()))
    }

    fn cross(&mut self, from: &str, place: &str) -> String {
        self.enter_place(Some(from), place)
    }

    fn spline_vectors(&self, id: &str) -> Vec<Vec3> {
        self.world()
            .get_resource::<crate::runtime::Splines>()
            .and_then(|splines| splines.0.get(id).cloned())
            .unwrap_or_default()
    }

    fn spline_length(&mut self, id: &str) -> String {
        let points = self.spline_vectors(id);
        let mut length = 0.0;
        for pair in points.windows(2) {
            length += pair[0].distance(pair[1]);
        }
        length.to_string()
    }

    fn sample_spline(&mut self, payload: &Json) -> String {
        let id = payload.get("id").and_then(Json::as_str).unwrap_or("");
        let at = payload.get("at").and_then(Json::as_f64).unwrap_or(0.0);
        let vectors = self.spline_vectors(id);
        if vectors.len() < 2 {
            return "null".to_string();
        }
        let lengths = vectors.windows(2).map(|pair| pair[0].distance(pair[1])).collect::<Vec<_>>();
        let total: f64 = lengths.iter().map(|value| *value as f64).sum();
        let distance = if payload.get("space").and_then(Json::as_str) == Some("distance") {
            at.clamp(0.0, total)
        } else {
            at.clamp(0.0, 1.0) * total
        };
        let mut walked = 0.0;
        for (index, length) in lengths.iter().enumerate() {
            if walked + *length as f64 >= distance || index + 2 == vectors.len() {
                let t = ((distance - walked) / (*length as f64).max(0.0001)).clamp(0.0, 1.0) as f32;
                let position = vectors[index].lerp(vectors[index + 1], t);
                let tangent = (vectors[index + 1] - vectors[index]).normalize_or_zero();
                return json_string(&json!({
                    "position": vec3_json(position),
                    "tangent": vec3_json(tangent),
                    "normal": vec3_json(Vec3::Y),
                    "lateral": vec3_json(tangent.cross(Vec3::Y).normalize_or_zero()),
                    "rotation": {"yaw": tangent.x.atan2(tangent.z).to_degrees()},
                    "distance": distance,
                    "progress": distance / total.max(0.0001),
                }));
            }
            walked += *length as f64;
        }
        "null".to_string()
    }

    fn project_spline(&mut self, payload: &Json) -> String {
        let id = payload.get("id").and_then(Json::as_str).unwrap_or("");
        let point = spec::vec3(payload.get("point"));
        let vectors = self.spline_vectors(id);
        if vectors.len() < 2 {
            return "null".to_string();
        }
        let mut best = (f32::MAX, 0.0f32, 0usize);
        let mut walked = 0.0;
        for (index, pair) in vectors.windows(2).enumerate() {
            let segment = pair[1] - pair[0];
            let length = segment.length();
            let t = ((point - pair[0]).dot(segment) / length.powi(2).max(0.0001)).clamp(0.0, 1.0);
            let distance = pair[0].lerp(pair[1], t).distance(point);
            if distance < best.0 {
                best = (distance, walked + length * t, index);
            }
            walked += length;
        }
        self.sample_spline(&json!({"id": id, "at": best.1, "space": "distance"}))
    }

    fn file_path(&self, path: &str) -> Option<PathBuf> {
        let clean = normalize("", path);
        if clean.starts_with("..") || Path::new(&clean).is_absolute() {
            return None;
        }
        Some(self.root.join(clean))
    }

    fn file_read(&mut self, path: &str) -> String {
        self.file_path(path)
            .and_then(|path| std::fs::read_to_string(path).ok())
            .map(|text| json_string(&Json::String(text)))
            .unwrap_or_else(|| "null".to_string())
    }

    fn file_write(&mut self, path: &str, text: &str) -> String {
        self.file_path(path)
            .map(|path| {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                std::fs::write(path, text).is_ok()
            })
            .unwrap_or(false)
            .to_string()
    }

    fn file_delete(&mut self, path: &str) -> String {
        self.file_path(path)
            .map(|path| std::fs::remove_file(path).is_ok())
            .unwrap_or(false)
            .to_string()
    }

    fn file_list(&mut self) -> String {
        fn walk(dir: &Path, root: &Path, out: &mut Vec<Json>) {
            let Ok(entries) = std::fs::read_dir(dir) else { return; };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, root, out);
                } else if let Ok(relative) = path.strip_prefix(root) {
                    out.push(Json::String(relative.to_string_lossy().to_string()));
                }
            }
        }
        let mut files = Vec::new();
        walk(&self.root, &self.root, &mut files);
        files.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
        json_string(&Json::Array(files))
    }

    fn spawn_player(&mut self, payload: &Json) -> String {
        let id = payload.get("id").and_then(Json::as_str).unwrap_or("local-player");
        let mut object = ObjectSpec {
            id: Some(id.to_string()),
            tags: vec!["player".to_string()],
            ..Default::default()
        };
        if let Some(value) = payload.get("properties") {
            if let Ok(parsed) = serde_json::from_value::<ObjectSpec>(value.clone()) {
                object = parsed;
                object.id = Some(id.to_string());
                if !object.tags.iter().any(|tag| tag == "player") {
                    object.tags.push("player".to_string());
                }
            }
        }
        let place = payload.get("place").and_then(Json::as_str).unwrap_or(&self.place).to_string();
        let world = self.world_mut();
        let entity = world::spawn_into_world(world, &place, id, &object);
        self.index.insert(id.to_string(), entity);
        self.ids.insert(entity, id.to_string());
        json_string(&Json::String(id.to_string()))
    }

}

static LAST_UI_HTML: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

fn host_provenance(op: &str) -> &'static str {
    match op {
        "raycast" | "raycastAll" | "overlapPoint" | "overlapSegment" | "traverseCheck" | "jumpCheck" | "inputRay" | "setJoint" | "setRagdoll" | "setAnim" | "getAnim" | "setAudio" | "getAudio" | "enterPlace" | "cross" | "readField" | "terrainHeight" | "waterLevel" | "terrainNormal" | "fieldsJson" | "worldToScreen" | "cameraJson" | "placesJson" | "placeConfigJson" | "splineLength" | "sampleSpline" | "projectToSpline" | "setInput" | "placeObjects" | "spawnPlayer" | "vibeSnapshot" => "Bevy ECS world query/state",
        "physicsWorldCreate" | "physicsWorldAdd" | "physicsWorldStep" | "physicsWorldSave" | "physicsWorldRestore" | "voxelPart" | "voxelSet" | "voxelJson" | "fxCreate" | "fxState" | "tradeReceipt" | "dmNotify" | "jobRecord" | "timing" | "viewConstant" | "featureProbe" | "featureRandom" => "native FeatureState",
        "nativeRandom" | "nativeMath" | "nativeVector" | "nativeNoise" | "nativeFeatureCounts" => "typed Bevy ECS resource",
        "globeConfigure" | "globeInfo" | "globeToCartesian" | "globeToSpherical" | "globeElevation" | "globeChunk" | "globeChunks" | "globeGravity" | "globeOrientation" | "globeNavigation" | "globeNavigate" | "globeWrap" | "globeTerrainHeight" => "Bevy ECS spherical procedural world",
        "fileRead" | "fileWrite" | "fileDelete" | "fileList" => "local filesystem shim",
        "commit" | "now" => "deterministic local shim",
        _ => "prelude local shim",
    }
}

pub fn api_report() -> String {
    ENGINE.with(|cell| {
        let guard = cell.borrow();
        let Some(engine) = guard.as_ref() else {
            return json_string(&json!({
                "ctxKeys": [],
                "modules": [],
                "hostVerbs": [],
                "hostCalls": {},
                "provenance": {},
                "errors": 0
            }));
        };
        let snapshot = {
            let host = engine.host.borrow();
            (
                host.host_calls.clone(),
                host.api_provenance.clone(),
                host.errors,
                host.scripts
                    .iter()
                    .filter(|(_, entry)| entry.is_some())
                    .map(|(name, _)| name.clone())
                    .collect::<BTreeSet<_>>(),
            )
        };
        let (calls, provenance, script_errors, modules) = snapshot;
        let mut provenance: BTreeMap<String, Json> = provenance
            .into_iter()
            .map(|(key, value)| (key, Json::String(value)))
            .collect();
        let keys = prelude_ctx_keys();
        for key in &keys {
            provenance
                .entry(format!("ctx.{key}"))
                .or_insert_with(|| Json::String(ctx_provenance(key).to_string()));
        }
        let mut verbs = calls.keys().cloned().collect::<BTreeSet<_>>();
        for name in install_bridge_names() {
            provenance
                .entry(format!("bridge.{name}"))
                .or_insert_with(|| Json::String("rquickjs global bridge".to_string()));
            verbs.insert(format!("bridge.{name}"));
        }
        for (name, mechanism) in crate::globe::provenance() {
            provenance.entry(name.to_string()).or_insert_with(|| Json::String(mechanism.to_string()));
        }
        json_string(&json!({
            "ctxKeys": keys,
            "modules": modules,
            "hostVerbs": verbs,
            "hostCalls": calls,
            "provenance": provenance,
            "errors": script_errors
        }))
    })
}

fn install_bridge_names() -> BTreeSet<String> {
    [
        "__after", "__attach", "__cancel", "__changed_json", "__children_json", "__cross", "__del", "__destroy", "__dt_ms", "__emit", "__exists", "__get_json", "__has", "__host", "__id", "__iterate_json", "__jayzee", "__keys_json", "__log", "__now", "__off", "__on_id", "__params_json", "__parent", "__place_objects_json", "__players_json", "__query_json", "__random", "__replace_state", "__set_json", "__sleep", "__target", "__timers_json",
    ].into_iter().map(str::to_string).collect()
}

fn prelude_ctx_keys() -> BTreeSet<String> {
    let factory = PRELUDE.find("function __make_ctx").unwrap_or(0);
    let relative = PRELUDE[factory..].find("const ctx = {").map(|index| factory + index + "const ctx = {".len()).unwrap_or(0);
    let rest = &PRELUDE[relative..];
    let end = rest.find("\n  };").unwrap_or(rest.len());
    let body = &rest[..end];
    body.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") { return None }
            let end = trimmed.find(':').or_else(|| trimmed.find(',')).unwrap_or(trimmed.len());
            let key = trimmed[..end].trim();
            if key.is_empty() || !key.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$') {
                return None;
            }
            Some(key.to_string())
        })
        .collect()
}

fn ctx_provenance(key: &str) -> &'static str {
    match key {
        "self" | "place" | "world" | "session" | "player" | "state" | "view" | "params" | "getObject" | "query" | "jayzee" | "destroy" | "attach" | "cross" | "enterPlace" | "worldToScreen" | "getObjectBoneNames" | "getObjectMorphTargetNames" | "getObjectSocket" | "files" => "Bevy ECS world query/state",
        "now" | "dtMs" | "gapMs" | "sleep" | "after" | "cancel" | "getTimers" => "deterministic local timing shim",
        "random" => "typed Bevy ECS resource",
        "emit" | "on" | "changed" | "music" | "notifyDm" | "notifyDmOnce" | "getVibeAudio" | "stopFx" | "pulseFx" | "getFxState" | "job" | "awaitJob" | "trade" | "sql" | "proposePurchase" | "presence" | "getRoomId" | "resolveControls" | "commit" => "native FeatureState / local shim",
        "device" | "lod" | "quality" | "seed" | "constants" | "geometry" | "portal" | "scene" | "color" | "albedo" | "roughness" | "metalness" | "emissive" | "smooth" | "flat" | "quad" | "tri" | "vertex" | "normal" | "uv" | "index" | "material" | "depth" | "data" | "additive" | "viewZ" | "viewZAt" | "normalAt" | "worldPosition" | "skyVisibility" | "previousUv" | "velocity" | "target" | "pass" | "cube" | "cover" => "deterministic local render/authoring shim",
        "globe" => "Bevy ECS spherical procedural world",
        _ => "prelude local shim",
    }
}

pub fn game16_api_paths(root: &Path) -> BTreeSet<String> {
    fn walk(root: &Path, dir: &Path, paths: &mut BTreeSet<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().and_then(|name| name.to_str()) != Some(".git") {
                    walk(root, &path, paths);
                }
                continue;
            }
            let Some(extension) = path.extension().and_then(|value| value.to_str()) else { continue };
            if !["js", "yaml", "yml", "json", "md"].contains(&extension) { continue }
            let Ok(source) = std::fs::read_to_string(&path) else { continue };
            for line in source.lines() {
                let bytes = line.as_bytes();
                let mut index = 0usize;
                while let Some(offset) = line[index..].find("ctx.") {
                    let start = index + offset;
                    let mut end = start + 4;
                    while end < bytes.len() {
                        let byte = bytes[end];
                        if byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$' || byte == b'.' {
                            end += 1;
                        } else { break }
                    }
                    let token = &line[start..end];
                    if token.ends_with('.') {
                        paths.insert(token[..token.len() - 1].to_string());
                    } else {
                        paths.insert(token.to_string());
                    }
                    index = end;
                }
            }
            let _ = root;
        }
    }
    let mut paths = BTreeSet::new();
    walk(root, root, &mut paths);
    paths
}

pub fn game16_report(root: &Path, place: &str, ticks: usize, errors: u64, hash: &str) -> String {
    let api = api_report();
    let api: Json = parse_json(&api);
    let paths = game16_api_paths(root);
    let keys = api.get("ctxKeys").and_then(Json::as_array).cloned().unwrap_or_default();
    let key_set = keys.iter().filter_map(Json::as_str).collect::<BTreeSet<_>>();
    let missing = paths
        .iter()
        .filter(|path| {
            let Some(rest) = path.strip_prefix("ctx.") else { return false };
            let top = rest.split('.').next().unwrap_or("");
            !key_set.contains(top) && !game16_dynamic_paths().contains(top)
        })
        .cloned()
        .collect::<BTreeSet<_>>();
    json_string(&json!({
        "root": root.display().to_string(),
        "place": place,
        "ticks": ticks,
        "errors": errors,
        "hash": hash,
        "ctxPaths": paths.len(),
        "missingApi": missing,
        "missingApiCount": missing.len(),
        "modules": api.get("modules").cloned().unwrap_or(Json::Array(vec![])),
        "moduleCount": api.get("modules").and_then(Json::as_array).map(Vec::len).unwrap_or(0),
        "hostVerbs": api.get("hostVerbs").cloned().unwrap_or(Json::Array(vec![])),
        "hostCalls": api.get("hostCalls").cloned().unwrap_or(Json::Object(Map::new())),
        "ctxKeys": api.get("ctxKeys").cloned().unwrap_or(Json::Array(vec![])),
        "provenance": api.get("provenance").cloned().unwrap_or(Json::Object(Map::new())),
        "ecs": api.get("provenance").and_then(Json::as_object).map(|map| map.values().filter(|value| value.as_str().unwrap_or("").contains("Bevy ECS")).count()).unwrap_or(0),
        "globe": crate::globe::GlobeWorld::default().report(&crate::globe::GlobeSummary::default())
    }))
}

fn game16_dynamic_paths() -> BTreeSet<&'static str> {
    [
        "additive", "albedo", "atlas", "attribute", "authoring", "bone", "canvas", "cover", "crossing", "cube", "cycle", "data", "depth", "falloff", "flat", "fx", "getTerrainHeight", "groundNormal", "groundY", "instances", "instance", "metalness", "noise", "normal", "portal", "purchase", "quad", "rejectedRooms", "requestedRoomId", "room", "rooms", "roughness", "scatter", "seedOffset", "slope", "smooth", "sprite", "strength", "strokeStartValue", "sway", "terrain", "texture", "tile", "tri", "wang", "x", "y", "z", "slice", "terrainBackdrop", "vegetation", "getTerrainNormal"
    ].into_iter().collect()
}

pub fn last_ui_html() -> String {
    LAST_UI_HTML.lock().map(|html| html.clone()).unwrap_or_default()
}

pub fn stats() -> String {
    ENGINE.with(|cell| {
        let guard = cell.borrow();
        let Some(engine) = guard.as_ref() else {
            return "{}".to_string();
        };
        let host = engine.host.borrow();
        json_string(&json!({
            "modules": host.scripts.values().filter(|entry| entry.is_some()).count(),
            "hooks": host.hooks_called,
            "errors": host.errors,
            "timers": host.timers.len(),
            "listeners": host.listeners.len(),
            "initialized": host.initialized.len(),
            "scripted": host.scripted_done.len(),
            "uiBytes": host.ui_html.len(),
        }))
    })
}

pub fn shutdown() {
    ENGINE.with(|cell| {
        if let Some(mut engine) = cell.borrow_mut().take() {
            engine.shutdown();
        }
    });
}

struct ScriptEngine {
    context: Context,
    _runtime: Runtime,
    host: Rc<RefCell<Host>>,
    make_ctx: Option<Persistent<Function<'static>>>,
}

impl ScriptEngine {
    fn new(root: &Path) -> Result<Self, String> {
        let runtime = Runtime::new().map_err(|error| error.to_string())?;
        runtime.set_loader(WorldResolver, WorldLoader { root: root.to_path_buf() });
        let context = Context::full(&runtime).map_err(|error| error.to_string())?;
        let host = Rc::new(RefCell::new(Host::new(root)));
        let make_ctx = context
            .with(|ctx| -> rquickjs::Result<Persistent<Function<'static>>> {
                install(&ctx, host.clone())?;
                ctx.eval::<(), _>(PRELUDE)?;
                let function: Function = ctx.globals().get("__make_ctx")?;
                Ok(Persistent::save(&ctx, function))
            })
            .map_err(|error| error.to_string())?;
        Ok(Self {
            context,
            _runtime: runtime,
            host,
            make_ctx: Some(make_ctx),
        })
    }

    fn shutdown(&mut self) {
        let context = self.context.clone();
        context.with(|ctx| {
            let scripts = {
                let mut host = self.host.borrow_mut();
                host.world = std::ptr::null_mut();
                std::mem::take(&mut host.scripts)
            };
            for (_, persistent) in scripts {
                if let Some(persistent) = persistent {
                    if let Ok(value) = persistent.restore(&ctx) {
                        drop(value);
                    }
                }
            }
            if let Some(make_ctx) = self.make_ctx.take() {
                if let Ok(value) = make_ctx.restore(&ctx) {
                    drop(value);
                }
            }
            let globals = ctx.globals();
            for name in install_bridge_names() {
                let _ = globals.set::<_, Value>(name.as_str(), Value::new_null(ctx.clone()));
            }
            let _ = globals.set::<_, Value>("__active_ctx", Value::new_null(ctx.clone()));
            let _ = globals.set::<_, Value>("__make_ctx", Value::new_null(ctx.clone()));
            let _ = globals.set::<_, Value>("__make", Value::new_null(ctx.clone()));
            let _ = ctx.eval::<(), _>("globalThis.__handlers = []; globalThis.__jobs = {}; globalThis.__fx_registry = {}; globalThis.__dm_once_keys = undefined");
        });
    }

    fn tick(&mut self, world: &mut World) {
        let dt = world.resource::<Time<Fixed>>().delta_secs() as f64;
        let place = world
            .get_resource::<WorldData>()
            .map(|data| data.0.name.clone())
            .unwrap_or_default();
        {
            let mut host = self.host.borrow_mut();
            host.world = world as *mut World;
            host.dt = dt;
            host.now_ms += dt * 1000.0;
            host.tick += 1;
            host.place = place;
            host.refresh();
        }
        let context = self.context.clone();
        let result = context.with(|ctx| self.run_tick(&ctx, dt));
        if let Err(error) = result {
            println!("{{\"kind\":\"script-tick-error\",\"error\":{}}}", quote(&error));
        }
        self.host.borrow_mut().world = std::ptr::null_mut();
    }

    fn run_tick(&self, ctx: &Ctx<'_>, dt: f64) -> Result<(), String> {
        self.run_policy(ctx);
        self.dispatch_destroyed(ctx);
        self.render_ui(ctx);
        {
            let mut host = self.host.borrow_mut();
            host.state_before = host.state_maps();
        }
        let behaviors = self.host.borrow().behavior_entities();
        let pending = {
            let host = self.host.borrow();
            behaviors
                .iter()
                .flat_map(|(entity, scripts)| {
                    scripts
                        .iter()
                        .filter(|script| !host.initialized.contains(&(*entity, (*script).clone())))
                        .map(|script| (*entity, script.clone()))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        };
        for (entity, script) in &pending {
            self.call_hook(ctx, *entity, script, "onJayzee", None)?;
        }
        let scripted = self.host.borrow().scripted_entities();
        for info in &scripted {
            let done = self.host.borrow().scripted_done.contains(&info.entity);
            if !done {
                self.attach_scripted(ctx, info);
                self.host.borrow_mut().scripted_done.insert(info.entity);
            }
        }
        {
            let current = behaviors
                .iter()
                .flat_map(|(entity, scripts)| scripts.iter().map(|script| (*entity, script.clone())))
                .collect::<HashSet<_>>();
            let mut host = self.host.borrow_mut();
            host.initialized.retain(|entry| current.contains(entry));
            for (entity, script) in &pending {
                host.initialized.insert((*entity, script.clone()));
            }
        }
        let input = self
            .host
            .borrow()
            .world()
            .get_resource::<InputState>()
            .cloned()
            .unwrap_or_default();
        if input.dirty {
            let value = json!({
                "axes": input.axes,
                "pressed": input.pressed,
                "released": input.released,
                "held": input.held,
                "actionData": input.action_data,
            });
            for (entity, scripts) in &behaviors {
                self.host.borrow_mut().sleep_until.remove(entity);
                for script in scripts {
                    self.call_hook(ctx, *entity, script, "onInput", Some(value.clone()))?;
                }
            }
            if let Some(mut input) = self
                .host
                .borrow_mut()
                .world_mut()
                .get_resource_mut::<InputState>()
            {
                input.pressed.clear();
                input.released.clear();
                input.dirty = false;
            }
        }
        self.dispatch_collisions(ctx);
        self.dispatch_liquids(ctx);
        let now = self.host.borrow().now_ms;
        for (entity, scripts) in &behaviors {
            let sleeping = self
                .host
                .borrow()
                .sleep_until
                .get(entity)
                .is_some_and(|until| now < *until);
            if sleeping {
                continue;
            }
            for script in scripts {
                let interval = self.update_schedule(ctx, script);
                match interval {
                    Some(interval) if interval > 0.0 => {
                        let due = self
                            .host
                            .borrow()
                            .next_schedule
                            .get(&(*entity, script.clone()))
                            .copied()
                            .unwrap_or(0.0);
                        if now >= due {
                            self.host
                                .borrow_mut()
                                .next_schedule
                                .insert((*entity, script.clone()), now + interval * 1000.0);
                            self.call_hook(ctx, *entity, script, "update", Some(json!(interval)))?;
                        }
                    }
                    _ => self.call_hook(ctx, *entity, script, "update", Some(json!(dt)))?,
                }
                self.call_hook(ctx, *entity, script, "lateUpdate", Some(json!(dt)))?;
            }
        }
        let globals = ctx.globals();
        if let Ok(tick) = globals.get::<_, Function>("__tick") {
            if let Err(error) = tick.call::<_, ()>((dt,)) {
                println!("{{\"kind\":\"script-tick-error\",\"error\":{}}}", quote(&format!("{error}")));
            }
        }
        self.run_timers(ctx)?;
        self.dispatch_events(ctx)?;
        self.host.borrow_mut().diff_states();
        Ok(())
    }

    fn dispatch_liquids(&self, ctx: &Ctx<'_>) {
        let records = {
            let mut host = self.host.borrow_mut();
            let Some(mut queue) = host.world_mut().get_resource_mut::<crate::runtime::LiquidQueue>() else {
                return;
            };
            std::mem::take(&mut queue.0)
        };
        for (entity, inside, level) in records {
            let (scripts, id) = {
                let host = self.host.borrow();
                (
                    host.world()
                        .get::<Behaviors>(entity)
                        .map(|behaviors| behaviors.0.clone())
                        .unwrap_or_default(),
                    host.id_of(entity).unwrap_or_default(),
                )
            };
            let other = json!({"id": "terrain/water", "liquidKind": "water", "level": level, "inside": inside});
            for script in scripts {
                let _ = self.call_hook2(
                    ctx,
                    entity,
                    &script,
                    if inside { "onLiquidEnter" } else { "onLiquidExit" },
                    other.clone(),
                    Json::Null,
                );
            }
            let _ = id;
        }
    }

    fn dispatch_collisions(&self, ctx: &Ctx<'_>) {
        let (started, ended) = {
            let mut host = self.host.borrow_mut();
            let Some(mut queue) = host.world_mut().get_resource_mut::<CollisionQueue>() else {
                return;
            };
            (std::mem::take(&mut queue.started), std::mem::take(&mut queue.ended))
        };
        for record in started {
            let (a, b) = (record.a, record.b);
            let contact = json!({
                "normal": vec3_json(record.normal),
                "point": vec3_json(record.point),
                "impulse": record.impulse,
                "penetration": record.penetration,
            });
            self.call_collision_hook(ctx, a, b, if record.sensor { "onTriggerEnter" } else { "onCollide" }, &contact);
            let mut flipped = contact.clone();
            if let Some(object) = flipped.as_object_mut() {
                object.insert(
                    "normal".into(),
                    vec3_json(-record.normal),
                );
            }
            self.call_collision_hook(ctx, b, a, if record.sensor { "onTriggerEnter" } else { "onCollide" }, &flipped);
        }
        for record in ended {
            let (a, b) = (record.a, record.b);
            self.call_collision_hook(ctx, a, b, "onTriggerExit", &Json::Null);
            self.call_collision_hook(ctx, b, a, "onTriggerExit", &Json::Null);
        }
    }

    fn call_collision_hook(&self, ctx: &Ctx<'_>, entity: Entity, other: Entity, hook: &str, contact: &Json) {
        let (scripts, other_id) = {
            let host = self.host.borrow();
            let scripts = host
                .world()
                .get::<Behaviors>(entity)
                .map(|behaviors| behaviors.0.clone())
                .unwrap_or_default();
            (scripts, host.id_of(other).unwrap_or_default())
        };
        if scripts.is_empty() || other_id.is_empty() {
            return;
        }
        let other_json = json!({"id": other_id, "objectId": other_id});
        for script in scripts {
            let _ = self.call_hook2(ctx, entity, &script, hook, other_json.clone(), contact.clone());
        }
    }

    fn ctx_for_id<'js>(&self, ctx: &Ctx<'js>, id: &str, place: &str) -> Option<Value<'js>> {
        let function = self.make_ctx.as_ref()?.clone().restore(ctx).ok()?;
        function.call::<_, Value>((id.to_string(), place.to_string())).ok()
    }

    fn handle_for_id<'js>(&self, ctx: &Ctx<'js>, id: &str) -> Option<Value<'js>> {
        let globals = ctx.globals();
        let make: Function = globals.get("__make").ok()?;
        make.call::<_, Value>((format!("obj:{id}"),)).ok()
    }

    fn run_policy(&self, ctx: &Ctx<'_>) {
        if let Some((script, place, id)) = self.host.borrow_mut().pending_leave.take() {
            if let Some(namespace) = self.module(ctx, &script) {
                if let Ok(value) = namespace.get::<_, Value>("onLeave") {
                    if let Some(function) = value.as_function().cloned() {
                        if let (Some(ctx_value), Some(handle)) = (
                            self.ctx_for_id(ctx, &id, &place),
                            self.handle_for_id(ctx, &id),
                        ) {
                            self.host.borrow_mut().hooks_called += 1;
                            let globals = ctx.globals();
                            if let Ok(invoke) = globals.get::<_, Function>("__invoke1") {
                                let _ = invoke.call::<_, String>((function, ctx_value, handle));
                            }
                        }
                    }
                }
            }
        }
        let (script, id, place) = {
            let mut host = self.host.borrow_mut();
            let world: &mut World = unsafe { &mut *host.world };
            let Some(data) = world.get_resource::<crate::world::WorldData>() else {
                return;
            };
            let place_name = data.0.name.clone();
            if host.policy_done == place_name {
                return;
            }
            let script = data
                .0
                .config
                .policy
                .as_ref()
                .and_then(Json::as_object)
                .and_then(|policy| policy.get("onArrive"))
                .and_then(Json::as_str)
                .map(str::to_string);
            let Some(script) = script else {
                return;
            };
            let mut query = world.query::<(&ObjectId, &Tags)>();
            let id = query
                .iter(world)
                .find(|(_, tags)| tags.0.iter().any(|tag| tag == "player"))
                .map(|(id, _)| id.0.clone());
            let Some(id) = id else {
                return;
            };
            (script, id, place_name)
        };
        let Some(namespace) = self.module(ctx, &script) else {
            return;
        };
        let Ok(value) = namespace.get::<_, Value>("onArrive") else {
            return;
        };
        let Some(function) = value.as_function().cloned() else {
            return;
        };
        let Some(ctx_value) = self.ctx_for_id(ctx, &id, &place) else {
            return;
        };
        let Some(handle) = self.handle_for_id(ctx, &id) else {
            return;
        };
        self.host.borrow_mut().hooks_called += 1;
        if let Err(error) = function.call::<_, ()>((ctx_value, handle)) {
            self.host.borrow_mut().errors += 1;
            println!(
                "{{\"kind\":\"policy-error\",\"script\":{},\"hook\":\"onArrive\",\"error\":{}}}",
                quote(&script),
                quote(&format!("{error}"))
            );
        }
        self.host.borrow_mut().policy_done = place;
    }

    fn render_ui(&self, ctx: &Ctx<'_>) {
        let (script, id, place, signature) = {
            let mut host = self.host.borrow_mut();
            let world: &mut World = unsafe { &mut *host.world };
            let (script, place_name) = {
                let Some(data) = world.get_resource::<crate::world::WorldData>() else {
                    return;
                };
                let script = data
                    .0
                    .config
                    .ui
                    .as_ref()
                    .and_then(Json::as_object)
                    .and_then(|ui| ui.get("render"))
                    .and_then(Json::as_str)
                    .map(str::to_string)
                    .or_else(|| {
                        world
                            .get_resource::<WorldState>()
                            .and_then(|state| state.ui.get("render"))
                            .and_then(Json::as_str)
                            .map(str::to_string)
                    });
                (script, data.0.name.clone())
            };
            let Some(script) = script else {
                return;
            };
            let mut query = world.query::<(Entity, &ObjectId, &Tags, &ObjectState)>();
            let player = query
                .iter(world)
                .find(|(_, _, tags, _)| tags.0.iter().any(|tag| tag == "player"))
                .map(|(entity, id, _, state)| (entity, id.0.clone(), state.0.clone()));
            let Some((_entity, id, state)) = player else {
                return;
            };
            let signature = json_string(&Json::Object(state));
            if host.ui_signature == signature && !host.ui_html.is_empty() {
                return;
            }
            host.ui_signature = signature;
            (script, id, place_name, host.ui_signature.clone())
        };
        let _ = signature;
        let Some(namespace) = self.module(ctx, &script) else {
            return;
        };
        let Ok(value) = namespace.get::<_, Value>("default") else {
            return;
        };
        let Some(function) = value.as_function().cloned() else {
            return;
        };
        let Some(ctx_value) = self.ctx_for_id(ctx, &id, &place) else {
            return;
        };
        let Some(handle) = self.handle_for_id(ctx, &id) else {
            return;
        };
        let globals = ctx.globals();
        if let Ok(invoke) = globals.get::<_, Function>("__call1") {
            let result = invoke.call::<_, String>((function, ctx_value, handle)).unwrap_or_default();
            if let Some(error) = result.strip_prefix("__error:") {
                self.host.borrow_mut().errors += 1;
                println!(
                    "{{\"kind\":\"script-ui-error\",\"script\":{},\"error\":{}}}",
                    quote(&script),
                    quote(error)
                );
            } else {
                if let Ok(mut last) = LAST_UI_HTML.lock() {
                    last.clone_from(&result);
                }
                self.host.borrow_mut().ui_html = result;
            }
        }
        self.host.borrow_mut().hooks_called += 1;
    }

    fn dispatch_destroyed(&self, ctx: &Ctx<'_>) {
        let records = {
            let mut host = self.host.borrow_mut();
            let queued = host
                .world_mut()
                .get_resource_mut::<crate::runtime::DespawnQueue>()
                .map(|mut queue| std::mem::take(&mut queue.0))
                .unwrap_or_default();
            let mut records = std::mem::take(&mut host.destroyed);
            records.extend(queued);
            records
        };
        for (id, place, scripts) in records {
            for script in scripts {
                let Some(namespace) = self.module(ctx, &script) else {
                    continue;
                };
                let Ok(value) = namespace.get::<_, Value>("onDestroy") else {
                    continue;
                };
                let Some(function) = value.as_function().cloned() else {
                    continue;
                };
                let Some(ctx_value) = self.ctx_for_id(ctx, &id, &place) else {
                    continue;
                };
                self.host.borrow_mut().hooks_called += 1;
                if let Err(error) = function.call::<_, ()>((ctx_value,)) {
                    self.host.borrow_mut().errors += 1;
                    println!(
                        "{{\"kind\":\"script-hook-error\",\"script\":{},\"hook\":\"onDestroy\",\"error\":{}}}",
                        quote(&script),
                        quote(&format!("{error}"))
                    );
                }
            }
        }
    }

    fn update_schedule(&self, ctx: &Ctx<'_>, script: &str) -> Option<f64> {
        if let Some(cached) = self.host.borrow().schedules.get(script) {
            return *cached;
        }
        let seconds = self.module(ctx, script).and_then(|namespace| {
            let value = namespace.get::<_, Value>("updateSchedule").ok()?;
            let json = js_to_json(&value);
            let every = json.get("every")?;
            let ticks = every.as_f64().or_else(|| every.get("seconds").and_then(Json::as_f64));
            ticks.map(|value| {
                if every.is_number() {
                    value * self.host.borrow().dt
                } else {
                    value
                }
            })
        });
        self.host.borrow_mut().schedules.insert(script.to_string(), seconds);
        seconds
    }

    fn module<'js>(&self, ctx: &Ctx<'js>, script: &str) -> Option<Object<'js>> {
        {
            let host = self.host.borrow();
            if let Some(entry) = host.scripts.get(script) {
                return entry.as_ref().and_then(|persistent| persistent.clone().restore(ctx).ok());
            }
            if host.failed.contains(script) {
                return None;
            }
        }
        let path = self.host.borrow().root.join(script);
        let source = match std::fs::read_to_string(&path) {
            Ok(source) => source,
            Err(error) => {
                self.host.borrow_mut().failed.insert(script.to_string());
                println!(
                    "{{\"kind\":\"script-load-error\",\"script\":{},\"error\":{}}}",
                    quote(script),
                    quote(&error.to_string())
                );
                return None;
            }
        };
        let evaluated = Module::declare(ctx.clone(), script.to_string(), source).and_then(|module| {
            let (module, promise) = module.eval()?;
            promise.finish::<()>()?;
            module.namespace()
        });
        match evaluated {
            Ok(namespace) => {
                let persistent = Persistent::save(ctx, namespace.clone());
                self.host
                    .borrow_mut()
                    .scripts
                    .insert(script.to_string(), Some(persistent));
                Some(namespace)
            }
            Err(error) => {
                let detail = if let Some(exception) = ctx.catch().as_exception() {
                    let message = exception.message().unwrap_or_default();
                    let stack = exception.stack().unwrap_or_default();
                    if stack.is_empty() { message } else { format!("{message}\n{stack}") }
                } else {
                    format!("{error}")
                };
                self.host.borrow_mut().failed.insert(script.to_string());
                println!(
                    "{{\"kind\":\"script-load-error\",\"script\":{},\"error\":{}}}",
                    quote(script),
                    quote(&detail)
                );
                None
            }
        }
    }

    fn make_ctx_value<'js>(&self, ctx: &Ctx<'js>, entity: Entity) -> Option<Value<'js>> {
        let (id, place) = {
            let host = self.host.borrow();
            (host.id_of(entity)?, host.place.clone())
        };
        let function = self.make_ctx.as_ref()?.clone().restore(ctx).ok()?;
        function.call::<_, Value>((id, place)).ok()
    }

    fn call_hook(
        &self,
        ctx: &Ctx<'_>,
        entity: Entity,
        script: &str,
        hook: &str,
        argument: Option<Json>,
    ) -> Result<(), String> {
        let Some(namespace) = self.module(ctx, script) else {
            return Ok(());
        };
        let Ok(value) = namespace.get::<_, Value>(hook) else {
            return Ok(());
        };
        let Some(function) = value.as_function().cloned() else {
            return Ok(());
        };
        let Some(ctx_value) = self.make_ctx_value(ctx, entity) else {
            return Ok(());
        };
        self.host.borrow_mut().hooks_called += 1;
        {
            let mut host = self.host.borrow_mut();
            host.current_entity = Some(entity);
            host.current_script = Some(script.to_string());
        }
        let globals = ctx.globals();
        let result = match argument {
            Some(argument) => {
                let value = json_to_js(ctx, &argument).map_err(|error| error.to_string())?;
                let invoke: Function = globals.get("__invoke1").map_err(|error| error.to_string())?;
                invoke.call::<_, String>((function, ctx_value, value)).unwrap_or_default()
            }
            None => {
                let invoke: Function = globals.get("__invoke0").map_err(|error| error.to_string())?;
                invoke.call::<_, String>((function, ctx_value)).unwrap_or_default()
            }
        };
        if !result.is_empty() {
            self.host.borrow_mut().errors += 1;
            println!(
                "{{\"kind\":\"script-hook-error\",\"script\":{},\"hook\":{},\"error\":{}}}",
                quote(script),
                quote(hook),
                quote(&result)
            );
        }
        {
            let mut host = self.host.borrow_mut();
            host.current_entity = None;
            host.current_script = None;
        }
        Ok(())
    }

    fn call_hook2(
        &self,
        ctx: &Ctx<'_>,
        entity: Entity,
        script: &str,
        hook: &str,
        first: Json,
        second: Json,
    ) -> Result<(), String> {
        let Some(namespace) = self.module(ctx, script) else {
            return Ok(());
        };
        let Ok(value) = namespace.get::<_, Value>(hook) else {
            return Ok(());
        };
        let Some(function) = value.as_function().cloned() else {
            return Ok(());
        };
        let Some(ctx_value) = self.make_ctx_value(ctx, entity) else {
            return Ok(());
        };
        self.host.borrow_mut().hooks_called += 1;
        {
            let mut host = self.host.borrow_mut();
            host.current_entity = Some(entity);
            host.current_script = Some(script.to_string());
        }
        let first = json_to_js(ctx, &first).map_err(|error| error.to_string())?;
        let second = json_to_js(ctx, &second).map_err(|error| error.to_string())?;
        let globals = ctx.globals();
        let invoke: Function = globals.get("__invoke2").map_err(|error| error.to_string())?;
        let result = invoke.call::<_, String>((function, ctx_value, first, second)).unwrap_or_default();
        if !result.is_empty() {
            self.host.borrow_mut().errors += 1;
            println!(
                "{{\"kind\":\"script-hook-error\",\"script\":{},\"hook\":{},\"error\":{}}}",
                quote(script),
                quote(hook),
                quote(&result)
            );
        }
        {
            let mut host = self.host.borrow_mut();
            host.current_entity = None;
            host.current_script = None;
        }
        Ok(())
    }

    fn call_return<'js>(
        &self,
        ctx: &Ctx<'js>,
        entity: Entity,
        script: &str,
        hook: &str,
    ) -> Option<Value<'js>> {
        let namespace = self.module(ctx, script)?;
        let value = namespace.get::<_, Value>(hook).ok()?;
        let function = value.as_function()?.clone();
        let ctx_value = self.make_ctx_value(ctx, entity)?;
        {
            let mut host = self.host.borrow_mut();
            host.current_entity = Some(entity);
            host.current_script = Some(script.to_string());
        }
        let result = function.call::<_, Value>((ctx_value,));
        {
            let mut host = self.host.borrow_mut();
            host.current_entity = None;
            host.current_script = None;
        }
        result.ok()
    }

    fn attach_scripted(&self, ctx: &Ctx<'_>, info: &ScriptedInfo) {
        let Some(value) = self.call_return(ctx, info.entity, &info.script, "collider") else {
            return;
        };
        let json = js_to_json(&value);
        let Some(array) = json.as_array() else {
            return;
        };
        let world_ptr = self.host.borrow().world;
        let world = unsafe { &mut *world_ptr };
        let scale = world
            .get::<Transform>(info.entity)
            .map(|transform| transform.scale)
            .unwrap_or(Vec3::ONE);
        for item in array {
            let kind = item.get("kind").and_then(Json::as_str).unwrap_or("box");
            let center = spec::vec3(item.get("center"));
            let radius = item
                .get("radius")
                .or_else(|| item.get("radiusTop"))
                .or_else(|| item.get("radiusBottom"))
                .and_then(Json::as_f64)
                .unwrap_or(0.5) as f32;
            let size = Vec3::new(
                item.get("width").and_then(Json::as_f64).unwrap_or(radius as f64 * 2.0) as f32,
                item.get("height").and_then(Json::as_f64).unwrap_or(radius as f64 * 2.0) as f32,
                item.get("depth").and_then(Json::as_f64).unwrap_or(radius as f64 * 2.0) as f32,
            );
            if kind == "heightfield" {
                let heights = item
                    .get("heights")
                    .and_then(Json::as_array)
                    .map(|rows| {
                        rows.iter()
                            .map(|row| {
                                row.as_array()
                                    .map(|values| values.iter().map(|value| value.as_f64().unwrap_or(0.0) as f32).collect::<Vec<_>>())
                                    .unwrap_or_default()
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let field_scale = item
                    .get("scale")
                    .map(|value| spec::vec3(Some(value)))
                    .unwrap_or(size);
                world::add_scripted_heightfield(world, info.entity, center, heights, field_scale);
            } else {
                world::add_scripted_collider(world, info.entity, kind, center, radius, size, scale);
            }
        }
    }

    fn run_timers(&self, ctx: &Ctx<'_>) -> Result<(), String> {
        let now = self.host.borrow().now_ms;
        let due = {
            let mut host = self.host.borrow_mut();
            let mut due = Vec::new();
            let mut remaining = Vec::new();
            for timer in host.timers.drain(..) {
                if timer.due_ms <= now {
                    due.push(timer);
                } else {
                    remaining.push(timer);
                }
            }
            host.timers = remaining;
            due
        };
        for timer in due {
            let payload = parse_json(&timer.payload);
            self.call_hook(ctx, timer.entity, &timer.script, &timer.hook, Some(payload))?;
        }
        Ok(())
    }

    fn dispatch_events(&self, ctx: &Ctx<'_>) -> Result<(), String> {
        let events = {
            let mut host = self.host.borrow_mut();
            std::mem::take(&mut host.events)
        };
        for event in events {
            let listeners = {
                let host = self.host.borrow();
                host.listeners
                    .iter()
                    .filter(|listener| listener.name == event.name)
                    .map(|listener| (listener.entity, listener.index))
                    .collect::<Vec<_>>()
            };
            let Some(call) = ctx.globals().get::<_, Function>("__call_handler").ok() else {
                continue;
            };
            for (entity, index) in listeners {
                let Some(ctx_value) = self.make_ctx_value(ctx, entity) else {
                    continue;
                };
                let payload = json_to_js(ctx, &parse_json(&event.payload)).map_err(|error| error.to_string())?;
                let meta = json_to_js(
                    ctx,
                    &json!({
                        "name": event.name,
                        "sourceId": entity.to_bits().to_string(),
                        "placeId": self.host.borrow().place,
                    }),
                )
                .map_err(|error| error.to_string())?;
                let _ = call.call::<_, ()>((index, payload, meta, ctx_value));
            }
        }
        Ok(())
    }
}

fn install<'js>(ctx: &Ctx<'js>, host: Rc<RefCell<Host>>) -> rquickjs::Result<()> {
    let globals = ctx.globals();
    let h = host.clone();
    globals.set(
        "__get_json",
        Function::new(ctx.clone(), move |path: String, key: String| -> String {
            h.borrow_mut().get_json(&path, &key)
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__set_json",
        Function::new(ctx.clone(), move |path: String, key: String, value: String| {
            h.borrow_mut().set_json(&path, &key, &value);
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__del",
        Function::new(ctx.clone(), move |path: String, key: String| {
            h.borrow_mut().delete_json(&path, &key);
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__has",
        Function::new(ctx.clone(), move |path: String, key: String| -> bool {
            h.borrow_mut().has_json(&path, &key)
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__keys_json",
        Function::new(ctx.clone(), move |path: String| -> String {
            h.borrow_mut().keys_json(&path)
        })?,
    )?;
    globals.set(
        "__id",
        Function::new(ctx.clone(), move |path: String| -> Option<String> {
            path.strip_prefix("obj:").map(str::to_string)
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__parent",
        Function::new(ctx.clone(), move |path: String| -> Option<String> {
            h.borrow().parent_of(&path)
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__children_json",
        Function::new(ctx.clone(), move |path: String| -> String {
            h.borrow().children_of(&path)
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__target",
        Function::new(ctx.clone(), move |path: String| -> Option<String> {
            h.borrow().target_of(&path)
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__place_objects_json",
        Function::new(ctx.clone(), move |place: String| -> String {
            h.borrow_mut().place_objects_json(&place)
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__players_json",
        Function::new(ctx.clone(), move |path: String| -> String {
            h.borrow_mut().players_json(&path)
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__iterate_json",
        Function::new(ctx.clone(), move |path: String| -> String {
            h.borrow_mut().iterate_json(&path)
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__exists",
        Function::new(ctx.clone(), move |id: String| -> bool {
            h.borrow().index.contains_key(&id)
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__jayzee",
        Function::new(ctx.clone(), move |explicit: String, spec: String| -> String {
            h.borrow_mut().jayzee(&explicit, &spec)
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__destroy",
        Function::new(ctx.clone(), move |value: String| {
            h.borrow_mut().destroy(&parse_json(&value));
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__query_json",
        Function::new(ctx.clone(), move |options: String| -> String {
            h.borrow_mut().query_json(&parse_json(&options))
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__now",
        Function::new(ctx.clone(), move || -> f64 {
            h.borrow().now_ms
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__random",
        Function::new(ctx.clone(), move || -> f64 {
            let mut host = h.borrow_mut();
            host.rng ^= host.rng << 13;
            host.rng ^= host.rng >> 7;
            host.rng ^= host.rng << 17;
            (host.rng >> 11) as f64 / 9007199254740992.0
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__dt_ms",
        Function::new(ctx.clone(), move || -> f64 {
            h.borrow().dt * 1000.0
        })?,
    )?;
    globals.set(
        "__log",
        Function::new(ctx.clone(), move |level: String, message: String| {
            println!(
                "{{\"kind\":\"script-log\",\"level\":{},\"message\":{}}}",
                quote(&level),
                quote(&message)
            );
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__replace_state",
        Function::new(ctx.clone(), move |id: String, value: String| {
            h.borrow_mut().replace_state(&id, &value);
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__after",
        Function::new(ctx.clone(), move |seconds: f64, hook: String, payload: String| -> String {
            h.borrow_mut().add_timer(seconds, hook, payload)
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__cancel",
        Function::new(ctx.clone(), move |id: String| {
            let mut host = h.borrow_mut();
            host.timers.retain(|timer| timer.id != id);
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__host",
        Function::new(ctx.clone(), move |op: String, payload: String| -> String {
            h.borrow_mut().host_op(&op, &payload)
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__emit",
        Function::new(ctx.clone(), move |name: String, payload: String, _options: String| -> String {
            h.borrow_mut().accept_event(&name, payload)
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__on_id",
        Function::new(ctx.clone(), move |name: String, index: u64, options: String| -> u64 {
            h.borrow_mut().add_listener(name, index, options)
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__off",
        Function::new(ctx.clone(), move |id: u64| {
            h.borrow_mut().off(id);
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__changed_json",
        Function::new(ctx.clone(), move |glob: String| -> String {
            h.borrow_mut().changed_json(&glob)
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__sleep",
        Function::new(ctx.clone(), move |seconds: f64| {
            let mut host = h.borrow_mut();
            if let Some(entity) = host.current_entity {
                let until = if seconds < 0.0 { f64::INFINITY } else { host.now_ms + seconds * 1000.0 };
                host.sleep_until.insert(entity, until);
            }
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__attach",
        Function::new(ctx.clone(), move |a: String, b: String, offset: String| {
            let mut host = h.borrow_mut();
            let body = if a.is_empty() {
                host.current_entity
            } else {
                host.index.get(&a).copied()
            };
            let Some(body) = body else { return; };
            let value = parse_json(&b);
            if value.is_null() {
                host.world_mut().entity_mut(body).remove::<ChildOf>();
                return;
            }
            let parent_id = value
                .as_str()
                .map(str::to_string)
                .or_else(|| value.get("id").and_then(Json::as_str).map(str::to_string));
            let Some(parent) = parent_id.and_then(|parent| host.index.get(&parent).copied()) else {
                return;
            };
            host.world_mut().entity_mut(body).insert(ChildOf(parent));
            let offset = parse_json(&offset);
            if let Some(position) = offset.get("position").and_then(crate::spec::vec3_opt) {
                if let Some(mut transform) = host.world_mut().get_mut::<Transform>(body) {
                    transform.translation = position;
                }
            }
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__cross",
        Function::new(ctx.clone(), move |_a: String, _b: String| -> Option<String> {
            let _ = &h;
            None
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__timers_json",
        Function::new(ctx.clone(), move |owner: String| -> String {
            let host = h.borrow();
            let timers = host
                .timers
                .iter()
                .filter(|timer| owner.is_empty() || host.id_of(timer.entity).as_deref() == Some(owner.as_str()))
                .map(|timer| {
                    json!({
                        "id": timer.id,
                        "ownerId": host.id_of(timer.entity).unwrap_or_default(),
                        "dueInTicks": ((timer.due_ms - host.now_ms) / 1000.0 * 64.0).max(0.0).round(),
                        "dueInSeconds": ((timer.due_ms - host.now_ms) / 1000.0).max(0.0),
                        "engineOwned": false,
                        "hook": timer.hook,
                    })
                })
                .collect::<Vec<_>>();
            json_string(&Json::Array(timers))
        })?,
    )?;
    let h = host.clone();
    globals.set(
        "__params_json",
        Function::new(ctx.clone(), move |id: String| -> String {
            let host = h.borrow();
            let Some(entity) = host.index.get(&id).copied() else {
                return "{}".to_string();
            };
            host.world()
                .get::<ScriptedPrimitive>(entity)
                .map(|scripted| json_string(&scripted.params))
                .unwrap_or_else(|| "{}".to_string())
        })?,
    )?;
    Ok(())
}

struct WorldResolver;

impl Resolver for WorldResolver {
    fn resolve<'js>(
        &mut self,
        _ctx: &Ctx<'js>,
        base: &str,
        name: &str,
        _attributes: Option<ImportAttributes<'js>>,
    ) -> rquickjs::Result<String> {
        if name.starts_with("builtin/") || !name.starts_with('.') {
            return Ok(name.to_string());
        }
        Ok(normalize(base, name))
    }
}

struct WorldLoader {
    root: PathBuf,
}

impl Loader for WorldLoader {
    fn load<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        name: &str,
        _attributes: Option<ImportAttributes<'js>>,
    ) -> rquickjs::Result<Module<'js>> {
        if let Some(key) = name.strip_prefix("builtin/") {
            return Module::declare(ctx.clone(), name.to_string(), builtin_source(key));
        }
        let path = self.root.join(name);
        let text = std::fs::read_to_string(&path).map_err(|error| {
            rquickjs::Error::new_loading_message(name, error.to_string())
        })?;
        if name.ends_with(".yml") || name.ends_with(".yaml") {
            let value: Json = serde_yaml::from_str(&text)
                .map_err(|error| rquickjs::Error::new_loading_message(name, error.to_string()))?;
            let source = format!("export default {};", json_string(&value));
            return Module::declare(ctx.clone(), name.to_string(), source);
        }
        Module::declare(ctx.clone(), name.to_string(), text)
    }
}

fn normalize(base: &str, name: &str) -> String {
    let mut parts = base.split('/').collect::<Vec<_>>();
    parts.pop();
    for part in name.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

fn builtin_source(key: &str) -> String {
    let exports = builtin_exports(key);
    let mut source = format!("const m = globalThis.__builtins[{}];\n", quote(key));
    for name in exports {
        source.push_str(&format!("export const {name} = m.{name};\n"));
    }
    source
}

fn builtin_exports(key: &str) -> &'static [&'static str] {
    match key {
        "math" => &["clamp", "clamp01", "lerp", "smoothstep", "inverseLerp", "remap", "shortestArc", "stepAngle", "degToRad", "radToDeg"],
        "vec3" | "vec" => &["add", "sub", "scale", "dot", "length", "normalize", "distance", "cross", "directionFromYawPitch", "rotate", "rotationFromDirection", "rotationFromEuler", "eulerFromRotation"],
        "easing" => &["linear", "easeOutBounce", "easeInQuad", "easeOutQuad", "easeInOutQuad", "easeInCubic", "easeOutCubic", "easeInOutCubic", "easeInQuart", "easeOutQuart", "easeInOutQuart", "easeInQuint", "easeOutQuint", "easeInOutQuint", "easeInSine", "easeOutSine", "easeInOutSine", "easeInExpo", "easeOutExpo", "easeInOutExpo", "easeInCirc", "easeOutCirc", "easeInOutCirc"],
        "noise" => &["simplex2", "simplex3", "fbm2", "ridged", "billowed2", "gradient2", "domainWarp2", "voronoi2", "white2", "makeSeededRng"],
        "path" => &["createPath"],
        "ballistics" => &["solveShot", "elevation"],
        "physics" => &["raycast", "overlapPoint", "overlapSegment", "traverseCheck", "jumpCheck", "getInputRay", "createWorld"],
        "tween" => &["animate"],
        "spline" => &["splineLength", "sampleSpline", "projectToSpline"],
        "nav" => &["findPath"],
        "room_routing" | "room-routing" => &["pickRoom", "firstOpenRoom", "mostFullWithSpace", "nextRoomName", "roomFor"],
        "terrain" => &["shore"],
        "voxels" => &["part", "set", "fill", "hollowBox", "sphere", "cylinder", "cone", "pyramid", "torus", "line", "add", "bounds", "serialize", "material", "checker", "random", "noise"],
        "vibe" => &["note", "s", "stack", "seq", "silence", "mini", "arrange", "scale", "progression", "arp", "chord", "clamp01", "lerp", "smoothstep", "above", "below", "between", "mapRange"],
        "fx" => &["emit", "pop", "rgb", "vec3", "range", "curve", "noise", "curl", "cone", "sphereDir", "time", "floorHeight", "attr", "count", "entity", "wind", "field", "when", "disc", "sphere", "box", "arc", "line", "point", "path", "sprite", "mesh", "ribbon", "light", "flicker", "firelight", "spin", "envelope", "gravity", "up", "buoyancy", "drag", "blast", "radial", "attract", "vortex", "hdr", "force", "orbit", "seek", "along", "parent"],
        "postfx" => &["grade", "vignette", "grain", "film", "tint", "scanlines", "dotScreen", "chromaticAberration", "dof", "pixelate", "blur", "bloom", "viewZAt", "scene", "depth", "viewZ", "normal", "worldPosition", "skyVisibility", "previousUv", "velocity", "target", "pass", "cube", "cover", "data", "additive"],
        "lighting" => &["sunDirection", "sunRadiance", "ambientRadiance", "wet", "rain", "windDirection", "windSpeed", "windTravel", "fogColor", "sunVisibility", "lightsAt", "lightCount", "lightRadiance", "lightPosition"],
        "water" => &["waterRippleSlope", "waterFoam", "waterFoamField", "waterFoamFieldLod", "waterFoamCoverage", "waterTransmittance", "waterMedium", "waterUnderwaterDistortion", "waterRayBoxDistance", "waterCaustics", "waterCausticsAt", "waterCausticsRefracted", "waterFresnel"],
        "env" => &["envAt", "wind", "rain", "light", "charge", "water", "cloud"],
        "three" => &["MeshStandardNodeMaterial", "MeshPhysicalNodeMaterial", "MeshSSSNodeMaterial", "MeshBasicNodeMaterial", "MeshPhongNodeMaterial", "MeshLambertNodeMaterial", "MeshToonNodeMaterial", "MeshMatcapNodeMaterial", "LineBasicNodeMaterial", "Line2NodeMaterial", "SpriteNodeMaterial", "PointsNodeMaterial", "FrontSide", "BackSide", "DoubleSide", "NoBlending", "NormalBlending", "AdditiveBlending", "SubtractiveBlending", "MultiplyBlending", "CustomBlending", "AddEquation", "SubtractEquation", "ReverseSubtractEquation", "MinEquation", "MaxEquation", "ZeroFactor", "OneFactor", "SrcColorFactor", "OneMinusSrcColorFactor", "SrcAlphaFactor", "OneMinusSrcAlphaFactor", "DstColorFactor", "OneMinusDstColorFactor", "DstAlphaFactor", "OneMinusDstAlphaFactor", "SrcAlphaSaturateFactor", "NeverDepth", "AlwaysDepth", "LessDepth", "LessEqualDepth", "EqualDepth", "GreaterEqualDepth", "GreaterDepth", "NotEqualDepth", "Plane", "RepeatWrapping", "ClampToEdgeWrapping", "MirroredRepeatWrapping", "NearestFilter", "LinearFilter", "NearestMipmapNearestFilter", "NearestMipmapLinearFilter", "LinearMipmapNearestFilter", "LinearMipmapLinearFilter", "NoColorSpace", "SRGBColorSpace", "RGBAFormat", "RGFormat", "RedFormat", "RGBAIntegerFormat", "RGIntegerFormat", "RedIntegerFormat", "UnsignedByteType", "HalfFloatType", "FloatType", "UnsignedIntType", "Color", "Vector2", "Vector3", "Vector4", "StorageTexture", "Storage3DTexture", "DataTexture", "Data3DTexture"],
        "tsl" => &["float", "vec2", "vec3", "vec4", "color", "int", "mix", "smoothstep", "step", "clamp", "min", "max", "abs", "floor", "ceil", "fract", "sign", "mod", "negate", "pow", "sqrt", "exp", "log", "sin", "cos", "tan", "atan", "atan2", "length", "distance", "dot", "cross", "normalize", "reflect", "refract", "oneMinus", "saturate", "remap", "remapClamp", "mx_noise_float", "mx_fractal_noise_float", "mx_worley_noise_float", "mx_noise_vec3", "hash", "rand", "oscSine", "oscTriangle", "oscSquare", "oscSawtooth", "uv", "positionLocal", "positionMorphed", "positionWorld", "positionView", "normalLocal", "normalWorld", "cameraPosition", "screenUV", "screenSize", "time", "vertexColor", "attribute", "instanceIndex", "vertexIndex", "frontFacing", "sceneDepth", "sceneDepthAt", "depthFade", "viewportSharedTexture", "materialColor", "materialOpacity", "materialEmissive", "audioLevel", "audioSpectrum", "billboarding", "Fn", "select", "If", "Loop", "texture", "textureSize", "textureLoad", "textureLevel", "texture3D", "convertToTexture", "PI", "PI2", "EPSILON", "wgslFn", "instancedArray", "deltaTime", "textureStore", "minFilter", "workgroupArray", "workgroupBarrier", "invocationLocalIndex", "workgroupId", "localId", "atomicAdd", "atomicSub", "atomicMax", "atomicMin", "atomicAnd", "atomicOr", "atomicXor", "atomicStore", "atomicLoad", "objectWorldMatrix", "objectPosition", "objectScale", "objectDirection"],
        "block-shape" => &["block", "shape"],
        "authoring" => &["define", "tool"],
        "globe" | "planet" | "spherical" => &["configure", "info", "toCartesian", "toSpherical", "elevation", "terrainHeight", "chunk", "chunks", "gravity", "orientation", "navigation", "navigate", "wrap"],
        _ => &[],
    }
}

fn parse_json(text: &str) -> Json {
    if text.is_empty() {
        Json::Null
    } else {
        serde_json::from_str(text).unwrap_or(Json::Null)
    }
}

fn json_string(value: &Json) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
}

fn quote(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

fn json_vec3(value: &Json) -> Vec3 {
    let [x, y, z] = json_vec3_array(value);
    Vec3::new(x, y, z)
}

fn json_vec3_array(value: &Json) -> [f32; 3] {
    match value {
        Json::Array(items) => [scalar(items.get(0), 0.0), scalar(items.get(1), 0.0), scalar(items.get(2), 0.0)],
        Json::Object(map) => [scalar(map.get("x"), 0.0), scalar(map.get("y"), 0.0), scalar(map.get("z"), 0.0)],
        _ => [0.0, 0.0, 0.0],
    }
}

fn scalar(value: Option<&Json>, fallback: f32) -> f32 {
    value.and_then(Json::as_f64).unwrap_or(fallback as f64) as f32
}

fn vec3_json(value: Vec3) -> Json {
    json!({ "x": value.x, "y": value.y, "z": value.z })
}

fn input_map(value: Option<&Json>) -> Map<String, Json> {
    match value {
        Some(Json::Array(items)) => items
            .iter()
            .filter_map(Json::as_str)
            .map(|name| (name.to_string(), Json::Bool(true)))
            .collect(),
        Some(Json::Object(map)) => map.clone(),
        _ => Map::new(),
    }
}

fn string_list(value: Option<&Json>) -> Vec<String> {
    value
        .and_then(Json::as_array)
        .map(|array| array.iter().filter_map(|value| value.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

fn js_to_json<'js>(value: &Value<'js>) -> Json {
    if value.is_undefined() || value.is_null() {
        return Json::Null;
    }
    if let Some(value) = value.as_bool() {
        return Json::Bool(value);
    }
    if let Some(value) = value.as_number() {
        return Json::from(value);
    }
    if let Some(value) = value.as_string() {
        return Json::String(value.to_string().unwrap_or_default());
    }
    if let Some(value) = value.as_array() {
        return Json::Array(value.iter().flatten().map(|value| js_to_json(&value)).collect());
    }
    if let Some(value) = value.as_object() {
        let mut map = Map::new();
        for key in value.keys::<String>().flatten() {
            if let Ok(value) = value.get::<_, Value>(key.as_str()) {
                map.insert(key, js_to_json(&value));
            }
        }
        return Json::Object(map);
    }
    Json::Null
}

fn json_to_js<'js>(ctx: &Ctx<'js>, value: &Json) -> rquickjs::Result<Value<'js>> {
    match value {
        Json::Null => Ok(Value::new_null(ctx.clone())),
        Json::Bool(value) => value.into_js(ctx),
        Json::Number(value) => value.as_f64().unwrap_or(0.0).into_js(ctx),
        Json::String(value) => value.as_str().into_js(ctx),
        Json::Array(values) => {
            let array = Array::new(ctx.clone())?;
            for (index, value) in values.iter().enumerate() {
                array.set(index, json_to_js(ctx, value)?)?;
            }
            Ok(array.into_value())
        }
        Json::Object(values) => {
            let object = Object::new(ctx.clone())?;
            for (key, value) in values {
                object.set(key.as_str(), json_to_js(ctx, value)?)?;
            }
            Ok(object.into_value())
        }
    }
}
