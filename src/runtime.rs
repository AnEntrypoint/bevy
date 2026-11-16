use avian3d::prelude::*;
use bevy::prelude::*;
use crate::world::{ObjectId, ObjectMetrics};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct GroundInfo {
    pub grounded: bool,
    pub wall_left: bool,
    pub wall_right: bool,
    pub ceiling: bool,
}

#[derive(Component, Clone, Debug, Default)]
pub struct ForceSet(pub Map<String, Value>);

#[derive(Component, Clone, Debug, Default)]
pub struct JointLinks(pub Vec<(String, Entity)>);

#[derive(Component, Clone, Debug, Default)]
pub struct AnimState(pub Map<String, Value>);

#[derive(Component, Clone, Debug, Default)]
pub struct AudioState(pub Value);

#[derive(Component, Clone, Debug, Default)]
pub struct LiquidInfo {
    pub inside: bool,
    pub level: f32,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct LiquidQueue(pub Vec<(Entity, bool, f32)>);

#[derive(Resource, Clone, Debug, Default)]
pub struct DespawnQueue(pub Vec<(String, String, Vec<String>)>);

#[derive(Resource, Clone, Debug, Default)]
pub struct Splines(pub BTreeMap<String, Vec<Vec3>>);

#[derive(Component, Clone, Debug, Default)]
pub struct SplinePlacement {
    pub id: String,
    pub at: f32,
    pub distance: bool,
    pub orient: bool,
    pub offset: Vec3,
}

#[derive(Component, Clone, Debug, Default)]
pub struct OverlapSet(pub Vec<String>);

#[derive(Component, Clone, Debug, Default)]
pub struct CameraTag;

#[derive(Component, Clone, Debug, Default)]
pub struct CameraTarget(pub String);

#[derive(Resource, Clone, Debug, Default)]
pub struct WorldState {
    pub state: Map<String, Value>,
    pub ui: Map<String, Value>,
    pub camera: Map<String, Value>,
    pub inputs: Map<String, Value>,
    pub mode: String,
    pub settling: bool,
    pub default_place: String,
    pub extra: Map<String, Value>,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct InputState {
    pub axes: Map<String, Value>,
    pub pressed: Map<String, Value>,
    pub released: Map<String, Value>,
    pub held: Map<String, Value>,
    pub action_data: Map<String, Value>,
    pub dirty: bool,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct CameraView {
    pub valid: bool,
    pub eye: Vec3,
    pub aim: Vec3,
    pub fov: f32,
    pub forward: Vec3,
    pub aspect: f32,
    pub inset_right: f32,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct CollisionQueue {
    pub started: Vec<CollisionRecord>,
    pub ended: Vec<CollisionRecord>,
}

#[derive(Clone, Debug)]
pub struct CollisionRecord {
    pub a: Entity,
    pub b: Entity,
    pub sensor: bool,
    pub normal: Vec3,
    pub point: Vec3,
    pub impulse: f32,
    pub penetration: f32,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct AudioTransport {
    pub music: Option<Value>,
    pub time: f64,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct TerrainData {
    pub fields: BTreeMap<String, FieldGrid>,
    pub water: Vec<WaterArea>,
}

#[derive(Clone, Debug, Default)]
pub struct FieldGrid {
    pub min: Vec2,
    pub max: Vec2,
    pub cell: f32,
    pub cols: usize,
    pub rows: usize,
    pub heights: Vec<f32>,
}

impl FieldGrid {
    pub fn height(&self, x: f32, z: f32) -> f32 {
        if self.cols < 2 || self.rows < 2 || self.heights.len() < self.cols * self.rows {
            return 0.0;
        }
        let fx = ((x - self.min.x) / self.cell).clamp(0.0, (self.cols - 1) as f32);
        let fz = ((z - self.min.y) / self.cell).clamp(0.0, (self.rows - 1) as f32);
        let x0 = fx.floor() as usize;
        let z0 = fz.floor() as usize;
        let x1 = (x0 + 1).min(self.cols - 1);
        let z1 = (z0 + 1).min(self.rows - 1);
        let tx = fx - x0 as f32;
        let tz = fz - z0 as f32;
        let sample = |xx: usize, zz: usize| self.heights[zz * self.cols + xx];
        let a = sample(x0, z0) * (1.0 - tx) + sample(x1, z0) * tx;
        let b = sample(x0, z1) * (1.0 - tx) + sample(x1, z1) * tx;
        a * (1.0 - tz) + b * tz
    }

    pub fn normal(&self, x: f32, z: f32) -> Vec3 {
        let d = self.cell.max(0.25);
        let left = self.height(x - d, z);
        let right = self.height(x + d, z);
        let back = self.height(x, z - d);
        let front = self.height(x, z + d);
        Vec3::new(left - right, 2.0 * d, back - front).normalize_or_zero()
    }
}

#[derive(Clone, Debug, Default)]
pub struct WaterArea {
    pub kind: String,
    pub level: f32,
    pub center: Vec2,
    pub radius: f32,
    pub width: f32,
    pub points: Vec<Vec2>,
}

impl WaterArea {
    pub fn contains(&self, x: f32, z: f32) -> bool {
        let p = Vec2::new(x, z);
        match self.kind.as_str() {
            "sea" | "halfplane" => true,
            "lake" => p.distance(self.center) <= self.radius,
            "river" => self.points.windows(2).any(|pair| {
                let a = pair[0];
                let b = pair[1];
                let ab = b - a;
                let t = ((p - a).dot(ab) / ab.length_squared().max(0.0001)).clamp(0.0, 1.0);
                (a + ab * t).distance(p) <= self.width * 0.5
            }),
            _ => false,
        }
    }
}

pub fn register(app: &mut App) {
    crate::features_ecs::register(app);
    crate::globe::register(app);
    app.init_resource::<WorldState>()
        .init_resource::<InputState>()
        .init_resource::<CameraView>()
        .init_resource::<CollisionQueue>()
        .init_resource::<AudioTransport>()
        .init_resource::<TerrainData>()
        .init_resource::<LiquidQueue>()
        .init_resource::<DespawnQueue>()
        .init_resource::<Splines>()
        .add_systems(FixedUpdate, tick_features.before(crate::script::run_scripts))
        .add_systems(FixedUpdate, crate::features_ecs::advance_clock.after(tick_features).before(crate::script::run_scripts))
        .add_systems(FixedUpdate, sync_feature_entities.after(tick_features).before(crate::script::run_scripts))
        .add_systems(FixedUpdate, crate::features_ecs::read_feature_entities.after(sync_feature_entities).before(crate::script::run_scripts))
        .add_systems(FixedUpdate, update_grounding.before(crate::script::run_scripts))
        .add_systems(FixedUpdate, update_overlaps.before(crate::script::run_scripts))
        .add_systems(FixedUpdate, tick_animations.before(crate::script::run_scripts))
        .add_systems(FixedUpdate, apply_forces.after(crate::script::run_scripts).before(PhysicsSystems::StepSimulation))
        .add_systems(FixedPostUpdate, collect_collisions)
        .add_systems(FixedUpdate, update_camera_view.after(crate::script::run_scripts))
        .add_systems(FixedUpdate, follow_splines.after(crate::script::run_scripts))
        .add_systems(FixedUpdate, tick_audio);
}

fn tick_features(mut features: ResMut<crate::features::FeatureState>, time: Res<Time<Fixed>>) {
    features.advance(time.delta_secs() as f64);
}

fn sync_feature_entities(
    mut commands: Commands,
    features: Res<crate::features::FeatureState>,
    mut registry: ResMut<crate::features_ecs::NativeFeatureRegistry>,
    worlds: Query<Entity, With<crate::features_ecs::PhysicsWorldEntity>>,
    bodies: Query<Entity, With<crate::features_ecs::PhysicsBodyComponent>>,
    parts: Query<Entity, With<crate::features_ecs::VoxelPartEntity>>,
    fx: Query<Entity, With<crate::features_ecs::FxEntity>>,
    vibe: Query<Entity, With<crate::features_ecs::VibeEntity>>,
    jobs: Query<Entity, With<crate::features_ecs::JobEntity>>,
    trades: Query<Entity, With<crate::features_ecs::TradeEntity>>,
    dm: Query<Entity, With<crate::features_ecs::DmMessageEntity>>,
) {
    for entity in worlds.iter().chain(bodies.iter()).chain(parts.iter()).chain(fx.iter()).chain(vibe.iter()).chain(jobs.iter()).chain(trades.iter()).chain(dm.iter()) {
        commands.entity(entity).despawn();
    }
    registry.physics_worlds.clear();
    registry.physics_bodies.clear();
    registry.voxel_parts.clear();
    registry.fx.clear();
    registry.vibe.clear();
    registry.jobs.clear();
    registry.trades.clear();
    registry.dm.clear();
    for (id, world) in &features.physics_worlds {
        registry.physics_worlds.push(commands.spawn(crate::features_ecs::PhysicsWorldEntity {
            id: id.clone(),
            gravity: Vec3::new(world.gravity[0], world.gravity[1], world.gravity[2]),
            body_count: world.bodies.len(),
            snapshot_len: world.snapshot.len(),
        }).id());
        for (index, body) in world.bodies.iter().enumerate() {
            registry.physics_bodies.push(commands.spawn(crate::features_ecs::PhysicsBodyComponent {
                world: id.clone(),
                index,
                position: Vec3::new(body.position[0], body.position[1], body.position[2]),
                velocity: Vec3::new(body.velocity[0], body.velocity[1], body.velocity[2]),
                angular_velocity: Vec3::new(body.angular_velocity[0], body.angular_velocity[1], body.angular_velocity[2]),
                mass: body.mass,
                sleeping: body.sleeping,
            }).id());
        }
    }
    for (id, part) in &features.voxel_parts {
        registry.voxel_parts.push(commands.spawn(crate::features_ecs::VoxelPartEntity { id: id.clone(), cells: part.cells.len(), palette: part.palette.len() }).id());
    }
    for (id, state) in &features.fx {
        registry.fx.push(commands.spawn(crate::features_ecs::FxEntity { id: id.clone(), rate: state.rate, lifetime: state.lifetime, alive: state.alive, stopped: state.stopped }).id());
    }
    for (id, state) in &features.vibe {
        registry.vibe.push(commands.spawn(crate::features_ecs::VibeEntity { id: id.clone(), playing: state.playing, bpm: state.bpm, beat: state.beat }).id());
    }
    for (id, state) in &features.jobs {
        registry.jobs.push(commands.spawn(crate::features_ecs::JobEntity { id: id.clone(), job: state.job.clone(), ok: state.ok }).id());
    }
    for receipt in &features.trades {
        registry.trades.push(commands.spawn(crate::features_ecs::TradeEntity { mine: receipt.mine.clone(), with: receipt.with.clone(), for_item: receipt.for_item.clone(), accepted: receipt.accepted }).id());
    }
    for message in &features.dm {
        registry.dm.push(commands.spawn(crate::features_ecs::DmMessageEntity { message: message.clone() }).id());
    }
}

fn update_grounding(
    spatial: SpatialQuery,
    mut query: Query<(
        Entity,
        &Position,
        &Rotation,
        &Collider,
        &mut GroundInfo,
        Option<&ObjectMetrics>,
        Option<&Transform>,
        &mut LiquidInfo,
    )>,
    terrain: Res<TerrainData>,
    mut liquids: ResMut<LiquidQueue>,
) {
    for (entity, position, rotation, _collider, mut ground, metrics, transform, mut liquid) in &mut query {
        let scale = transform.map(|t| t.scale).unwrap_or(Vec3::ONE);
        let half = metrics.map(|m| m.half_height * scale.y.abs()).unwrap_or(0.5);
        let filter = SpatialQueryFilter::from_excluded_entities([entity]);
        let down = spatial.cast_ray(position.0, Dir3::NEG_Y, half * 2.0 + 0.35, true, &filter);
        ground.grounded = down.as_ref().is_some_and(|hit| hit.normal.dot(Vec3::Y) > 0.6 && hit.distance <= half + 0.18);
        let right = rotation.0 * Vec3::X;
        let left_hit = spatial.cast_ray(position.0, Dir3::new(-right).unwrap_or(Dir3::X), 0.65, true, &filter);
        let right_hit = spatial.cast_ray(position.0, Dir3::new(right).unwrap_or(Dir3::X), 0.65, true, &filter);
        let up_hit = spatial.cast_ray(position.0, Dir3::Y, half + 0.35, true, &filter);
        ground.wall_left = left_hit.as_ref().is_some_and(|hit| hit.normal.dot(right) > 0.6);
        ground.wall_right = right_hit.as_ref().is_some_and(|hit| hit.normal.dot(right) < -0.6);
        ground.ceiling = up_hit.as_ref().is_some_and(|hit| hit.normal.dot(Vec3::Y) < -0.6);
        let feet = position.0.y - half;
        let level = terrain
            .water
            .iter()
            .filter(|water| water.contains(position.0.x, position.0.z))
            .map(|water| water.level)
            .fold(None, |best: Option<f32>, level| Some(best.map_or(level, |best| best.max(level))));
        let inside = level.is_some_and(|level| feet < level);
        let was = liquid.inside;
        liquid.inside = inside;
        liquid.level = level.unwrap_or(0.0);
        if was != inside {
            liquids.0.push((entity, inside, liquid.level));
        }
    }
}

fn follow_splines(
    splines: Res<Splines>,
    mut query: Query<(&SplinePlacement, &mut Position, &mut Transform, &mut Rotation)>,
) {
    for (placement, mut position, mut transform, mut rotation) in &mut query {
        let Some(points) = splines.0.get(&placement.id) else {
            continue;
        };
        if points.len() < 2 {
            continue;
        }
        let lengths = points
            .windows(2)
            .map(|pair| pair[0].distance(pair[1]))
            .collect::<Vec<_>>();
        let total: f32 = lengths.iter().sum();
        let distance = if placement.distance {
            placement.at.clamp(0.0, total)
        } else {
            placement.at.clamp(0.0, 1.0) * total
        };
        let mut walked = 0.0;
        let mut sample = (points[0], Vec3::NEG_Z);
        for (index, length) in lengths.iter().enumerate() {
            if walked + *length >= distance || index + 2 == points.len() {
                let t = ((distance - walked) / length.max(0.0001)).clamp(0.0, 1.0);
                let point = points[index].lerp(points[index + 1], t);
                sample = (point, (points[index + 1] - points[index]).normalize_or_zero());
                break;
            }
            walked += *length;
        }
        let target = sample.0 + placement.offset;
        position.0 = target;
        transform.translation = target;
        if placement.orient {
            let q = Quat::from_rotation_arc(Vec3::NEG_Z, sample.1);
            rotation.0 = q;
            transform.rotation = q;
        }
    }
}

fn update_overlaps(
    colliders: Query<&ColliderOf>,
    ids: Query<&ObjectId>,
    mut query: Query<(&CollidingEntities, &mut OverlapSet)>,
) {
    for (colliding, mut overlap) in &mut query {
        let mut found = Vec::new();
        for entity in colliding.0.iter() {
            let body = colliders.get(*entity).map(|link| link.body).unwrap_or(*entity);
            if let Ok(id) = ids.get(body) {
                if !found.contains(&id.0) {
                    found.push(id.0.clone());
                }
            }
        }
        found.sort();
        overlap.0 = found;
    }
}

fn update_camera_view(
    query: Query<(&Position, &Rotation), With<CameraTag>>,
    mut camera: ResMut<CameraView>,
    mut input: ResMut<InputState>,
) {
    let Ok((position, rotation)) = query.single() else {
        return;
    };
    let forward = rotation.0 * Vec3::NEG_Z;
    camera.valid = true;
    camera.eye = position.0;
    camera.aim = position.0 + forward * 10.0;
    camera.forward = forward;
    camera.fov = 60.0;
    camera.aspect = 16.0 / 9.0;
    let yaw = forward.x.atan2(-forward.z);
    input.axes.insert("aimYawSin".into(), Value::from(yaw.sin() as f64));
    input.axes.insert("aimYawCos".into(), Value::from(yaw.cos() as f64));
}

fn apply_forces(mut query: Query<(&mut ForceSet, Forces)>) {
    for (mut set, mut forces) in &mut query {
        for value in set.0.values() {
            let Some(entry) = value.as_object() else { continue; };
            if let Some(force) = entry.get("force").and_then(crate::spec::vec3_opt) {
                if let Some(position) = entry.get("position").and_then(crate::spec::vec3_opt) {
                    forces.apply_force_at_point(force, position);
                } else {
                    forces.apply_force(force);
                }
            }
            if let Some(torque) = entry.get("torque").and_then(crate::spec::vec3_opt) {
                forces.apply_torque(torque);
            }
            if let Some(impulse) = entry.get("impulse").and_then(crate::spec::vec3_opt) {
                forces.apply_linear_impulse(impulse);
            }
        }
        set.0.clear();
    }
}

fn collect_collisions(
    mut started: MessageReader<CollisionStart>,
    mut ended: MessageReader<CollisionEnd>,
    contacts: ResMut<ContactGraph>,
    sensors: Query<&Sensor>,
    mut queue: ResMut<CollisionQueue>,
    colliders: Query<&ColliderOf>,
) {
    let resolve = |entity: Entity, colliders: &Query<&ColliderOf>| {
        colliders
            .get(entity)
            .map(|link| link.body)
            .unwrap_or(entity)
    };
    for event in started.read() {
        let sensor = sensors.get(event.collider1).is_ok() || sensors.get(event.collider2).is_ok();
        let mut normal = Vec3::Y;
        let mut point = Vec3::ZERO;
        let mut penetration = 0.0;
        let impulse = 0.0;
        if let Some((_, pair)) = contacts.get(event.collider1, event.collider2) {
            if let Some(manifold) = pair.manifolds.first() {
                normal = manifold.normal;
                if let Some(contact) = manifold.points.first() {
                    point = contact.point;
                    penetration = contact.penetration;
                }
            }
        }
        queue.started.push(CollisionRecord {
            a: resolve(event.collider1, &colliders),
            b: resolve(event.collider2, &colliders),
            sensor,
            normal,
            point,
            impulse,
            penetration,
        });
    }
    for event in ended.read() {
        let sensor = sensors.get(event.collider1).is_ok() || sensors.get(event.collider2).is_ok();
        queue.ended.push(CollisionRecord {
            a: resolve(event.collider1, &colliders),
            b: resolve(event.collider2, &colliders),
            sensor,
            normal: Vec3::Y,
            point: Vec3::ZERO,
            impulse: 0.0,
            penetration: 0.0,
        });
    }
}

fn tick_animations(time: Res<Time<Fixed>>, mut query: Query<&mut AnimState>) {
    let dt = time.delta_secs() as f64;
    for mut state in &mut query {
        for value in state.0.values_mut() {
            let Some(channel) = value.as_object_mut() else { continue; };
            let finished = channel.get("finished").and_then(Value::as_bool).unwrap_or(false);
            if finished {
                continue;
            }
            let speed = channel.get("speed").and_then(Value::as_f64).unwrap_or(1.0);
            let duration = channel.get("duration").and_then(Value::as_f64).unwrap_or(1.0).max(0.0001);
            let elapsed = channel.get("time").and_then(Value::as_f64).unwrap_or(0.0) + dt * speed;
            let looping = match channel.get("loop") {
                Some(Value::Bool(true)) => true,
                Some(Value::String(mode)) => mode == "loop" || mode == "pingpong",
                _ => false,
            };
            let pingpong = channel.get("loop").and_then(Value::as_str) == Some("pingpong");
            if looping {
                if pingpong {
                    let cycle = (elapsed / duration) as i64;
                    let phase = elapsed % duration;
                    channel.insert("pingpongForward".into(), Value::Bool(cycle % 2 == 0));
                    channel.insert("time".into(), Value::from(phase));
                } else {
                    channel.insert("time".into(), Value::from(elapsed % duration));
                }
            } else if elapsed >= duration {
                channel.insert("time".into(), Value::from(duration));
                channel.insert("finished".into(), Value::Bool(true));
            } else {
                channel.insert("time".into(), Value::from(elapsed));
            }
        }
    }
}

fn tick_audio(time: Res<Time<Fixed>>, mut transport: ResMut<AudioTransport>) {
    transport.time += time.delta_secs() as f64;
}

