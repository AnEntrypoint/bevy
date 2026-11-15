use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use crate::runtime::{
    AnimState, AudioState, ForceSet, GroundInfo, JointLinks, LiquidInfo, OverlapSet, TerrainData,
    WorldState,
};
use crate::spec::{self, BodyKind, ObjectProperties, ObjectSpec, PhysicsDesc, PrimitiveDesc};
use crate::tree::{Place, WorldTree};

#[derive(Component, Clone, Debug)]
pub struct ObjectId(pub String);

#[derive(Component, Clone, Debug)]
pub struct PlaceTag(pub String);

#[derive(Component, Clone, Debug, Default)]
pub struct Tags(pub Vec<String>);

#[derive(Component, Clone, Debug, Default)]
pub struct ObjectState(pub serde_json::Map<String, Value>);

#[derive(Component, Clone, Debug, Default)]
pub struct Behaviors(pub Vec<String>);

#[derive(Component, Clone, Debug)]
pub struct ModelRef(pub Value);

#[derive(Component, Clone, Debug)]
pub struct LightValue(pub Value);

#[derive(Component, Clone, Debug)]
pub struct ScriptedPrimitive {
    pub script: String,
    pub params: Value,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct RemainingLifetime(pub f32);

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ObjectMetrics {
    pub half_height: f32,
}

#[derive(Component, Clone, Debug)]
pub struct PrimitiveInfo(pub Option<PrimitiveDesc>);

#[derive(Resource, Clone, Debug)]
pub struct WorldData(pub Place);

#[derive(Resource, Clone, Debug)]
pub struct WorldRoot(pub std::path::PathBuf);

pub fn run(directory: &str, place_name: Option<&str>, ticks: usize) -> Result<(), String> {
    let (mut app, place) = prepare(directory, place_name)?;
    for _ in 0..ticks {
        app.update();
    }
    report(&mut app, Path::new(directory), &place, ticks);
    crate::script::shutdown();
    Ok(())
}

pub fn prepare(directory: &str, place_name: Option<&str>) -> Result<(App, Place), String> {
    let tree = WorldTree::load(Path::new(directory))?;
    let name = place_name
        .map(str::to_string)
        .or(tree.config.default_place.clone())
        .or_else(|| tree.places.keys().next().cloned())
        .ok_or_else(|| format!("{directory}: no places"))?;
    let place = tree
        .places
        .get(&name)
        .cloned()
        .ok_or_else(|| format!("{directory}: no place {name}"))?;
    let hz = place.config.tick_base.unwrap_or(64.0).clamp(1.0, 1000.0);
    let gravity = match place.config.gravity.as_ref() {
        Some(value) => spec::vec3(Some(value)),
        None => Vec3::new(0.0, -9.81, 0.0),
    };
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, TransformPlugin, PhysicsPlugins::default()));
    app.insert_resource(Time::<Fixed>::from_hz(hz));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(1.0 / hz)));
    app.insert_resource(Gravity(gravity));
    app.insert_resource(WorldData(place.clone()));
    app.insert_resource(WorldRoot(Path::new(directory).to_path_buf()));
    {
        let mut state = WorldState::default();
        state.mode = "dev".to_string();
        state.default_place = tree
            .config
            .default_place
            .clone()
            .unwrap_or_else(|| place.name.clone());
        state.extra = place.config.extra.clone();
        state.extra.insert("engine".into(), tree.config.engine.clone());
        if let Some(player) = tree.config.player.clone() {
            state.extra.insert("player".into(), Value::String(player));
        }
        state.ui = tree.config.ui.as_object().cloned().unwrap_or_default();
        if let Some(ui) = place.config.ui.as_ref().and_then(Value::as_object) {
            for (key, value) in ui {
                state.ui.insert(key.clone(), value.clone());
            }
        }
        state.camera = tree
            .config
            .camera
            .as_ref()
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        if let Some(camera) = place.config.camera.as_ref().and_then(Value::as_object) {
            for (key, value) in camera {
                state.camera.insert(key.clone(), value.clone());
            }
        }
        state.inputs = tree.config.inputs.as_object().cloned().unwrap_or_default();
        if let Some(inputs) = place.config.inputs.as_ref().and_then(Value::as_object) {
            state.inputs.extend(inputs.clone());
        }
        app.insert_resource(state);
    }
    app.insert_resource(crate::features::FeatureState::seeded(0x6a09_e667_f3bc_c909));
    {
        let mut terrain = TerrainData::default();
        load_terrain(&place, &mut terrain);
        app.insert_resource(terrain);
    }
    let _ = tree;
    app.insert_resource(crate::globe::GlobeWorld::from_value(place.config.extra.get("globe")));
    crate::runtime::register(&mut app);
    app.add_systems(Startup, spawn_place);
    app.add_systems(FixedUpdate, age_lifetimes);
    app.add_systems(FixedUpdate, crate::script::run_scripts);
    app.finish();
    app.cleanup();
    Ok((app, place))
}

fn spawn_place(world: &mut World) {
    let place = world.resource::<WorldData>().0.clone();
    let mut entities = HashMap::<String, Entity>::new();
    let mut parents = Vec::<(Entity, String)>::new();
    for (index, object) in place.objects.iter().enumerate() {
        let id = object
            .id
            .clone()
            .unwrap_or_else(|| format!("object-{index}"));
        spawn_tree(world, &place.name, &id, object, &mut entities, &mut parents);
    }
    for (entity, parent) in &parents {
        if let Some(parent) = entities.get(parent) {
            world.entity_mut(*entity).insert(ChildOf(*parent));
        }
    }
    let has_player = entities.keys().any(|id| {
        world
            .get::<Tags>(*entities.get(id).unwrap())
            .is_some_and(|tags| tags.0.iter().any(|tag| tag == "player"))
    });
    if !has_player {
        let feet = place
            .config
            .default_jayzee_point
            .as_ref()
            .map(|value| spec::vec3(Some(value)))
            .unwrap_or(Vec3::ZERO);
        let object = ObjectSpec {
            id: Some("local-player".to_string()),
            tags: vec!["player".to_string()],
            place: Some(place.name.clone()),
            properties: ObjectProperties {
                feet_position: Some(serde_json::json!({"x": feet.x, "y": feet.y, "z": feet.z})),
                physics: Some(Value::String("none".to_string())),
                ..Default::default()
            },
            ..Default::default()
        };
        spawn_tree(world, &place.name, "local-player", &object, &mut entities, &mut parents);
    }
    let camera = place
        .config
        .camera
        .as_ref()
        .and_then(Value::as_object)
        .and_then(|camera| camera.get("behavior"))
        .map(|behavior| match behavior {
            Value::String(script) => vec![script.clone()],
            Value::Array(scripts) => scripts.iter().filter_map(Value::as_str).map(str::to_string).collect(),
            _ => Vec::new(),
        })
        .unwrap_or_default();
    if !camera.is_empty() {
        let state = place
            .config
            .camera
            .as_ref()
            .and_then(Value::as_object)
            .and_then(|camera| camera.get("state"))
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let point = place
            .config
            .default_jayzee_point
            .as_ref()
            .map(|value| spec::vec3(Some(value)))
            .unwrap_or(Vec3::ZERO);
        let object = ObjectSpec {
            id: Some("camera".to_string()),
            tags: vec!["camera".to_string()],
            state,
            behavior: Some(crate::spec::BehaviorRef::Many(camera)),
            place: Some(place.name.clone()),
            properties: ObjectProperties {
                feet_position: Some(serde_json::json!({"x": point.x, "y": point.y + 1.6, "z": point.z})),
                physics: Some(Value::String("none".to_string())),
                ..Default::default()
            },
            ..Default::default()
        };
        let entity = spawn_into_world(world, &place.name, "camera", &object);
        world.entity_mut(entity).insert((
            crate::runtime::CameraTag,
            crate::runtime::CameraTarget("local-player".to_string()),
        ));
    }
    spawn_terrain_collider(world);
}

pub fn spawn_terrain_collider_now(world: &mut World) {
    spawn_terrain_collider(world);
}

pub fn spawn_terrain_collider(world: &mut World) {
    let fields = world
        .get_resource::<TerrainData>()
        .map(|terrain| terrain.fields.clone())
        .unwrap_or_default();
    for (name, field) in fields {
        if field.cols < 2 || field.rows < 2 || field.heights.len() < field.cols * field.rows {
            continue;
        }
        let mut heights = Vec::with_capacity(field.rows);
        for z in 0..field.rows {
            let mut row = Vec::with_capacity(field.cols);
            for x in 0..field.cols {
                row.push(field.heights[z * field.cols + x]);
            }
            heights.push(row);
        }
        let width = (field.max.x - field.min.x).max(field.cell);
        let depth = (field.max.y - field.min.y).max(field.cell);
        let center = Vec3::new(
            (field.min.x + field.max.x) * 0.5,
            0.0,
            (field.min.y + field.max.y) * 0.5,
        );
        world.spawn((
            ObjectId(format!("terrain/{name}")),
            PlaceTag(name.clone()),
            RigidBody::Static,
            Collider::heightfield(heights, Vec3::new(width, 1.0, depth)),
            Transform::from_translation(center),
            Position::new(center),
        ));
    }
}

pub fn load_terrain(place: &Place, terrain: &mut TerrainData) {
    let Some(value) = &place.config.terrain else {
        return;
    };
    let Some(object) = value.as_object() else {
        return;
    };
    if object.get("kind").and_then(Value::as_str) == Some("off") {
        return;
    }
    if let Some(fields) = object.get("fields").and_then(Value::as_object) {
        for (name, value) in fields {
            let Some(field) = value.as_object() else {
                continue;
            };
            let grid = field.get("grid").and_then(Value::as_object);
            let size = field.get("size").and_then(Value::as_f64).unwrap_or(64.0) as f32;
            let cell = field
                .get("cell")
                .and_then(Value::as_f64)
                .map(|value| value as f32)
                .or_else(|| grid.and_then(|grid| grid.get("cell")).and_then(Value::as_f64).map(|value| value as f32))
                .unwrap_or(1.0);
            let min = grid
                .and_then(|grid| grid.get("min"))
                .map(|value| spec::vec3(Some(value)))
                .unwrap_or(Vec3::splat(-size * 0.5));
            let max = grid
                .and_then(|grid| grid.get("max"))
                .map(|value| spec::vec3(Some(value)))
                .unwrap_or(Vec3::splat(size * 0.5));
            let cols = (((max.x - min.x) / cell).round() as usize + 1).max(2);
            let rows = (((max.z - min.z) / cell).round() as usize + 1).max(2);
            let heights = field
                .get("cells")
                .or_else(|| field.get("heights"))
                .and_then(Value::as_array)
                .map(|cells| cells.iter().map(|value| value.as_f64().unwrap_or(0.0) as f32).collect::<Vec<_>>())
                .unwrap_or_default();
            terrain.fields.insert(
                name.clone(),
                crate::runtime::FieldGrid {
                    min: Vec2::new(min.x, min.z),
                    max: Vec2::new(max.x, max.z),
                    cell,
                    cols,
                    rows,
                    heights,
                },
            );
        }
    }
    if let Some(marks) = object.get("marks").and_then(Value::as_object) {
        for mark in marks.values() {
            let Some(mark) = mark.as_object() else {
                continue;
            };
            let kind = mark.get("kind").and_then(Value::as_str).unwrap_or("");
            let level = mark.get("level").and_then(Value::as_f64).unwrap_or(0.0) as f32;
            let center = mark.get("at").map(|value| spec::vec3(Some(value))).unwrap_or(Vec3::ZERO);
            let radius = mark.get("radius").and_then(Value::as_f64).unwrap_or(0.0) as f32;
            let width = mark.get("width").and_then(Value::as_f64).map(|value| value as f32).unwrap_or(0.0);
            let points = mark
                .get("points")
                .and_then(Value::as_array)
                .map(|points| {
                    points
                        .iter()
                        .filter_map(|point| point.as_array())
                        .filter(|point| point.len() >= 2)
                        .map(|point| {
                            Vec2::new(
                                point[0].as_f64().unwrap_or(0.0) as f32,
                                point[1].as_f64().unwrap_or(0.0) as f32,
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            terrain.water.push(crate::runtime::WaterArea {
                kind: kind.to_string(),
                level,
                center: Vec2::new(center.x, center.z),
                radius,
                width,
                points,
            });
        }
    }
}

pub fn spawn_tree(
    world: &mut World,
    place: &str,
    id: &str,
    object: &ObjectSpec,
    entities: &mut HashMap<String, Entity>,
    parents: &mut Vec<(Entity, String)>,
) {
    let entity = spawn_into_world(world, place, id, object);
    entities.insert(id.to_string(), entity);
    if let Some(parent) = &object.parent {
        parents.push((entity, parent.clone()));
    }
    for (index, child) in object.children.iter().enumerate() {
        let child_id = child
            .id
            .clone()
            .unwrap_or_else(|| format!("{id}-child-{index}"));
        spawn_tree(world, place, &child_id, child, entities, parents);
        if let Some(child_entity) = entities.get(&child_id) {
            parents.push((*child_entity, id.to_string()));
        }
    }
}

pub fn spawn_into_world(world: &mut World, place: &str, id: &str, object: &ObjectSpec) -> Entity {
    let properties = &object.properties;
    let primitive = spec::primitive(properties);
    let physics = spec::physics(properties);
    let feet = spec::feet_position(properties);
    let object_scale = spec::scale(properties.scale.as_ref());
    let height = match (&primitive, physics.as_ref().map(|physics| physics.body)) {
        (None, Some(BodyKind::Character)) => 0.9 * object_scale.y,
        _ => spec::center_height(properties, primitive.as_ref()) * object_scale.y,
    };
    let center = feet + Vec3::Y * height;
    let rotation = match (properties.rotation.as_ref(), properties.yaw) {
        (Some(value), _) => spec::rotation(Some(value)),
        (None, Some(yaw)) => spec::rotation(Some(&Value::from(yaw))),
        (None, None) => Quat::IDENTITY,
    };
    let transform = Transform::from_translation(center)
        .with_rotation(rotation)
        .with_scale(object_scale);
    let mut entity = world.spawn((
        ObjectId(id.to_string()),
        PlaceTag(place.to_string()),
        Tags(object.tags.clone()),
        ObjectState(object.state.clone()),
        Behaviors(object.behaviors()),
        transform,
        Position::new(center),
        Rotation(rotation),
        ObjectMetrics {
            half_height: match (&primitive, physics.as_ref().map(|physics| physics.body)) {
                (None, Some(BodyKind::Character)) => 0.9,
                _ => primitive.as_ref().map(|desc| desc.half_height).unwrap_or(0.0),
            },
        },
        PrimitiveInfo(primitive.clone()),
    ));
    entity.insert((
        GroundInfo::default(),
        ForceSet::default(),
        JointLinks::default(),
        AnimState::default(),
        AudioState(properties.audio.clone().unwrap_or(Value::Null)),
        OverlapSet::default(),
        LiquidInfo::default(),
    ));
    if let Some(mixer) = &properties.mixer {
        entity.insert(AnimState(
            mixer
                .as_object()
                .map(|map| {
                    map.iter()
                        .map(|(channel, value)| {
                            let mut value = value.clone();
                            if let Some(object) = value.as_object_mut() {
                                object.entry("time").or_insert(Value::from(0.0));
                                object.entry("finished").or_insert(Value::Bool(false));
                            }
                            (channel.clone(), value)
                        })
                        .collect()
                })
                .unwrap_or_default(),
        ));
    }
    if let Some(physics) = &physics {
        attach_physics(&mut entity, properties, physics, primitive.as_ref(), object_scale);
    }
    if let Some(model) = spec::model_path(properties) {
        entity.insert(ModelRef(model));
    }
    if let Some(light) = &properties.light {
        entity.insert(LightValue(light.clone()));
    }
    if let Some((script, params)) = spec::scripted_primitive(properties) {
        entity.insert(ScriptedPrimitive { script, params });
    }
    if let Some(lifetime) = object.lifetime {
        entity.insert(RemainingLifetime(lifetime as f32));
    }
    let points = spline_points(properties);
    let entity_id = entity.id();
    drop(entity);
    if points.len() >= 2 {
        world
            .resource_mut::<crate::runtime::Splines>()
            .0
            .insert(id.to_string(), points);
    }
    entity_id
}

pub fn spline_points(properties: &ObjectProperties) -> Vec<Vec3> {
    let points = properties
        .path
        .as_ref()
        .and_then(|path| path.get("points"))
        .and_then(Value::as_array)
        .cloned()
        .or_else(|| {
            properties
                .primitive
                .as_ref()
                .and_then(Value::as_object)
                .filter(|primitive| primitive.get("kind").and_then(Value::as_str) == Some("tube"))
                .and_then(|primitive| primitive.get("points"))
                .and_then(Value::as_array)
                .cloned()
        })
        .unwrap_or_default();
    points.iter().map(|point| spec::vec3(Some(point))).collect()
}

fn attach_physics(
    entity: &mut EntityWorldMut,
    properties: &ObjectProperties,
    physics: &PhysicsDesc,
    primitive: Option<&PrimitiveDesc>,
    object_scale: Vec3,
) {
    let body = match physics.body {
        BodyKind::None => return,
        BodyKind::Static => RigidBody::Static,
        BodyKind::Dynamic | BodyKind::Character => RigidBody::Dynamic,
        BodyKind::Kinematic => RigidBody::Kinematic,
    };
    entity.insert((
        body,
        LinearVelocity::default(),
        AngularVelocity::default(),
        CollidingEntities::default(),
        CollisionEventsEnabled,
    ));
    let settings = properties.physics.as_ref().and_then(Value::as_object);
    if physics.body == BodyKind::Character {
        entity.insert((
            LockedAxes::default()
                .lock_rotation_x()
                .lock_rotation_y()
                .lock_rotation_z(),
            GravityScale(0.0),
        ));
    }
    if let Some(settings) = settings {
        if let Some(friction) = settings.get("friction").and_then(Value::as_f64) {
            entity.insert(Friction::new(friction as f32));
        }
        if let Some(restitution) = settings.get("restitution").and_then(Value::as_f64) {
            entity.insert(Restitution::new(restitution as f32));
        }
        if let Some(damping) = settings.get("linearDamping").and_then(Value::as_f64) {
            entity.insert(LinearDamping(damping as f32));
        }
        if let Some(damping) = settings.get("angularDamping").and_then(Value::as_f64) {
            entity.insert(AngularDamping(damping as f32));
        }
        if let Some(scale) = settings.get("gravityScale").and_then(Value::as_f64) {
            entity.insert(GravityScale(scale as f32));
        } else if physics.body == BodyKind::Character {
            entity.insert(GravityScale(0.0));
        }
        if let Some(mass) = settings.get("mass").and_then(Value::as_f64) {
            entity.insert(Mass(mass as f32));
        }
        if let Some(axes) = settings.get("lockedAxes").and_then(Value::as_array) {
            let mut locked = LockedAxes::default();
            for axis in axes.iter().filter_map(Value::as_str) {
                locked = match axis {
                    "x" => locked.lock_translation_x(),
                    "y" => locked.lock_translation_y(),
                    "z" => locked.lock_translation_z(),
                    "rotX" => locked.lock_rotation_x(),
                    "rotY" => locked.lock_rotation_y(),
                    "rotZ" => locked.lock_rotation_z(),
                    _ => locked,
                };
            }
            entity.insert(locked);
        }
        if settings.get("trigger").and_then(Value::as_bool) == Some(true) {
            entity.insert(Sensor);
        }
        if let Some(density) = settings.get("density").and_then(Value::as_f64) {
            entity.insert(ColliderDensity(density as f32));
        }
    }
    if let Some(collider) = collider_for(physics, primitive, object_scale) {
        entity.insert(collider);
    }
}

fn collider_for(
    physics: &PhysicsDesc,
    primitive: Option<&PrimitiveDesc>,
    object_scale: Vec3,
) -> Option<Collider> {
    if physics.collider == "none" {
        return None;
    }
    if primitive.is_none() && physics.body == BodyKind::Character {
        return Some(Collider::capsule(0.3 * object_scale.x.max(object_scale.z), 1.2 * object_scale.y));
    }
    let desc = primitive.cloned().unwrap_or(PrimitiveDesc {
        kind: "box".to_string(),
        size: Vec3::ONE,
        half_height: 0.5,
        radius: 0.5,
        capsule_half_height: 0.5,
    });
    let kind = if physics.collider == "auto" {
        desc.kind.clone()
    } else {
        physics.collider.clone()
    };
    let scale = Vec3::new(
        object_scale.x.abs().max(0.0001),
        object_scale.y.abs().max(0.0001),
        object_scale.z.abs().max(0.0001),
    );
    match kind.as_str() {
        "box" | "cube" | "wall" => Some(Collider::cuboid(
            (desc.size.x * scale.x).max(0.001),
            (desc.size.y * scale.y).max(0.001),
            (desc.size.z * scale.z).max(0.001),
        )),
        "sphere" | "hemisphere" | "circle" => Some(Collider::sphere(
            desc.radius * scale.x.max(scale.y).max(scale.z),
        )),
        "capsule" => Some(Collider::capsule(
            desc.radius * scale.x.max(scale.z),
            desc.capsule_half_height * 2.0 * scale.y,
        )),
        "cylinder" | "frustum" | "hollowCylinder" | "cone" | "pyramid" => {
            Some(Collider::cylinder(
                desc.radius * scale.x.max(scale.z),
                (desc.size.y * scale.y).max(0.001),
            ))
        }
        "plane" => Some(Collider::cuboid(
            (desc.size.x * scale.x).max(0.001),
            0.01,
            (desc.size.z * scale.z).max(0.001),
        )),
        _ => None,
    }
}

pub fn replace_physics(
    world: &mut World,
    entity: Entity,
    properties: &ObjectProperties,
    physics: &PhysicsDesc,
) {
    {
        let mut entity_mut = world.entity_mut(entity);
        entity_mut.remove::<(
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
            CollidingEntities,
            CollisionEventsEnabled,
        )>();
    }
    let primitive = world
        .get::<PrimitiveInfo>(entity)
        .and_then(|info| info.0.clone());
    let scale = world
        .get::<Transform>(entity)
        .map(|transform| transform.scale)
        .unwrap_or(Vec3::ONE);
    let mut entity_mut = world.entity_mut(entity);
    attach_physics(&mut entity_mut, properties, physics, primitive.as_ref(), scale);
}

pub fn add_scripted_collider(
    world: &mut World,
    body: Entity,
    kind: &str,
    center: Vec3,
    radius: f32,
    size: Vec3,
    scale: Vec3,
) {
    let collider = match kind {
        "box" => Collider::cuboid(
            (size.x * scale.x).max(0.001),
            (size.y * scale.y).max(0.001),
            (size.z * scale.z).max(0.001),
        ),
        "sphere" => Collider::sphere(radius * scale.max_element()),
        "capsule" => Collider::capsule(radius * scale.x.max(scale.z), size.y * scale.y),
        "cylinder" => Collider::cylinder(radius * scale.x.max(scale.z), size.y * scale.y),
        "cone" => Collider::cone(radius * scale.x.max(scale.z), size.y * scale.y),
        "halfSpace" => Collider::half_space(Vec3::Y),
        _ => Collider::cuboid(
            (size.x * scale.x).max(0.001),
            (size.y * scale.y).max(0.001),
            (size.z * scale.z).max(0.001),
        ),
    };
    let (body_position, body_rotation, body_scale) = world
        .get::<Position>(body)
        .copied()
        .map(|position| {
            (
                position.0,
                world.get::<Rotation>(body).copied().unwrap_or_default().0,
                world.get::<Transform>(body).map(|transform| transform.scale).unwrap_or(Vec3::ONE),
            )
        })
        .unwrap_or((Vec3::ZERO, Quat::IDENTITY, Vec3::ONE));
    let world_center = body_position + body_rotation * (center * body_scale);
    world.spawn((
        ChildOf(body),
        ColliderOf { body },
        Position::new(world_center),
        Rotation(body_rotation),
        Transform::from_translation(world_center).with_rotation(body_rotation),
        collider,
    ));
}

pub fn add_scripted_heightfield(world: &mut World, body: Entity, center: Vec3, heights: Vec<Vec<f32>>, scale: Vec3) {
    if heights.len() < 2 || heights[0].len() < 2 {
        return;
    }
    let (body_position, body_rotation, body_scale) = world
        .get::<Position>(body)
        .copied()
        .map(|position| {
            (
                position.0,
                world.get::<Rotation>(body).copied().unwrap_or_default().0,
                world.get::<Transform>(body).map(|transform| transform.scale).unwrap_or(Vec3::ONE),
            )
        })
        .unwrap_or((Vec3::ZERO, Quat::IDENTITY, Vec3::ONE));
    let world_center = body_position + body_rotation * (center * body_scale);
    world.spawn((
        ChildOf(body),
        ColliderOf { body },
        Position::new(world_center),
        Rotation(body_rotation),
        Transform::from_translation(world_center).with_rotation(body_rotation),
        Collider::heightfield(heights, scale),
    ));
}

fn age_lifetimes(
    time: Res<Time<Fixed>>,
    mut query: Query<(
        Entity,
        &mut RemainingLifetime,
        Option<&ObjectId>,
        Option<&PlaceTag>,
        Option<&Behaviors>,
    )>,
    mut commands: Commands,
    mut queue: ResMut<crate::runtime::DespawnQueue>,
) {
    for (entity, mut lifetime, id, place, behaviors) in &mut query {
        lifetime.0 -= time.delta_secs();
        if lifetime.0 <= 0.0 {
            if let (Some(id), Some(behaviors)) = (id, behaviors) {
                if !behaviors.0.is_empty() {
                    queue.0.push((
                        id.0.clone(),
                        place.map(|place| place.0.clone()).unwrap_or_default(),
                        behaviors.0.clone(),
                    ));
                }
            }
            commands.entity(entity).despawn();
        }
    }
}

fn report(app: &mut App, root: &Path, place: &Place, ticks: usize) {
    let mut query = app.world_mut().query::<(
        &ObjectId,
        &Position,
        Option<&RigidBody>,
        Option<&Behaviors>,
        Option<&ModelRef>,
        Option<&ScriptedPrimitive>,
        Option<&LightValue>,
        Option<&Collider>,
        Option<&ColliderOf>,
    )>();
    let mut rows = Vec::new();
    for (id, position, body, behaviors, model, scripted, light, collider, collider_of) in query.iter(app.world()) {
        rows.push((
            id.0.clone(),
            position.0,
            body.is_some(),
            behaviors.is_some_and(|behaviors| !behaviors.0.is_empty()),
            model.is_some(),
            scripted.is_some(),
            light.is_some(),
            collider.is_some(),
            collider_of.is_some(),
        ));
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    let mut collider_query = app.world_mut().query::<(Option<&Collider>, Option<&ColliderOf>)>();
    let mut global_colliders = 0usize;
    let mut global_compound = 0usize;
    for (collider, collider_of) in collider_query.iter(app.world()) {
        if collider.is_some() {
            global_colliders += 1;
        }
        if collider_of.is_some() {
            global_compound += 1;
        }
    }
    let mut hash = 0x811c_9dc5u32;
    let mut bodies = 0usize;
    let mut scripted_count = 0usize;
    let mut model_count = 0usize;
    let mut light_count = 0usize;
    let mut behavior_count = 0usize;
    for (id, position, has_body, has_behavior, has_model, has_script, has_light, _has_collider, _is_compound) in &rows {
        if *has_body {
            bodies += 1;
        }
        if *has_behavior {
            behavior_count += 1;
        }
        if *has_model {
            model_count += 1;
        }
        if *has_script {
            scripted_count += 1;
        }
        if *has_light {
            light_count += 1;
        }
        for byte in id.bytes() {
            hash = (hash ^ byte as u32).wrapping_mul(0x0100_0193);
        }
        for value in [position.x, position.y, position.z] {
            hash = (hash ^ value.to_bits()).wrapping_mul(0x0100_0193);
        }
    }
    if let Some(row) = rows.iter().find(|row| row.0 == "camera") {
        println!(
            "{{\"kind\":\"world-camera\",\"position\":[{:.4},{:.4},{:.4}]}}",
            row.1.x, row.1.y, row.1.z
        );
    }
    println!(
        "{{\"kind\":\"world\",\"place\":\"{}\",\"objects\":{},\"bodies\":{},\"behaviors\":{},\"models\":{},\"scripted\":{},\"lights\":{},\"colliders\":{},\"compound\":{},\"ticks\":{},\"hash\":\"0x{:08x}\"}}",
        place.name,
        rows.len(),
        bodies,
        behavior_count,
        model_count,
        scripted_count,
        light_count,
        global_colliders,
        global_compound,
        ticks,
        hash
    );
    let sample = rows
        .iter()
        .filter(|row| row.2)
        .take(4)
        .map(|row| format!("[\"{}\",{:.4},{:.4},{:.4}]", row.0, row.1.x, row.1.y, row.1.z))
        .collect::<Vec<_>>()
        .join(",");
    println!("{{\"kind\":\"world-bodies\",\"sample\":[{sample}]}}");
    let mut child_query = app.world_mut().query::<(&Transform, Option<&ColliderOf>)>();
    let mut child_sample = Vec::new();
    for (transform, collider_of) in child_query.iter(app.world()) {
        if collider_of.is_some() {
            child_sample.push(format!(
                "[{:.4},{:.4},{:.4}]",
                transform.translation.x, transform.translation.y, transform.translation.z
            ));
            if child_sample.len() == 4 {
                break;
            }
        }
    }
    println!(
        "{{\"kind\":\"world-colliders\",\"sample\":[{}]}}",
        child_sample.join(",")
    );
    let script_stats: Value = serde_json::from_str(&crate::script::stats()).unwrap_or_default();
    println!("{{\"kind\":\"script-stats\",\"stats\":{}}}", script_stats);
    let features = app.world().resource::<crate::features::FeatureState>().report();
    let ecs = {
        let world = app.world();
        crate::features_ecs::entity_report(
            world.resource::<crate::features_ecs::NativeFeatureRegistry>(),
            world.resource::<crate::features_ecs::NativeClock>(),
            world.resource::<crate::features_ecs::NativeRandom>(),
            world.resource::<crate::features_ecs::NativeMathService>(),
        )
    };
    let globe = {
        let world = app.world();
        world.resource::<crate::globe::GlobeWorld>().report(world.resource::<crate::globe::GlobeSummary>())
    };
    println!("{{\"kind\":\"globe-report\",\"globe\":{}}}", globe);
    let mut merged = features.as_object().cloned().unwrap_or_default();
    merged.insert("ecs".into(), ecs);
    merged.insert("globe".into(), globe);
    println!("{{\"kind\":\"feature-report\",\"features\":{}}}", serde_json::Value::Object(merged));
    let script_errors = script_stats.get("errors").and_then(Value::as_u64).unwrap_or(0);
    let report = crate::script::game16_report(root, &place.name, ticks, script_errors, &format!("0x{hash:08x}"));
    println!("{{\"kind\":\"game16-report\",\"report\":{report}}}");
}
