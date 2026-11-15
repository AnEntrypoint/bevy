use bevy::prelude::*;
use serde_json::{Map, Value, json};

#[derive(Resource, Clone, Debug, Default)]
pub struct NativeFeatureRegistry {
    pub physics_worlds: Vec<Entity>,
    pub physics_bodies: Vec<Entity>,
    pub voxel_parts: Vec<Entity>,
    pub fx: Vec<Entity>,
    pub vibe: Vec<Entity>,
    pub jobs: Vec<Entity>,
    pub trades: Vec<Entity>,
    pub dm: Vec<Entity>,
}

#[derive(Component, Clone, Debug)]
pub struct PhysicsWorldEntity {
    pub id: String,
    pub gravity: Vec3,
    pub body_count: usize,
    pub snapshot_len: usize,
}

#[derive(Component, Clone, Debug)]
pub struct PhysicsBodyComponent {
    pub world: String,
    pub index: usize,
    pub position: Vec3,
    pub velocity: Vec3,
    pub angular_velocity: Vec3,
    pub mass: f32,
    pub sleeping: bool,
}

#[derive(Component, Clone, Debug)]
pub struct VoxelPartEntity {
    pub id: String,
    pub cells: usize,
    pub palette: usize,
}

#[derive(Component, Clone, Debug, Default)]
pub struct FxEntity {
    pub id: String,
    pub rate: f32,
    pub lifetime: f32,
    pub alive: u64,
    pub stopped: bool,
}

#[derive(Component, Clone, Debug, Default)]
pub struct VibeEntity {
    pub id: String,
    pub playing: bool,
    pub bpm: f32,
    pub beat: u64,
}

#[derive(Component, Clone, Debug, Default)]
pub struct JobEntity {
    pub id: String,
    pub job: String,
    pub ok: bool,
}

#[derive(Component, Clone, Debug, Default)]
pub struct TradeEntity {
    pub mine: String,
    pub with: String,
    pub for_item: String,
    pub accepted: bool,
}

#[derive(Component, Clone, Debug, Default)]
pub struct DmMessageEntity {
    pub message: Value,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct NativeFeatureSummary {
    pub objects: usize,
    pub physics_worlds: usize,
    pub physics_bodies: usize,
    pub voxels: usize,
    pub fx: usize,
    pub vibe: usize,
    pub jobs: usize,
    pub trades: usize,
    pub dm: usize,
    pub clock_ms: f64,
    pub random_state: u64,
    pub math_calls: u64,
}

#[derive(Resource, Clone, Debug)]
pub struct NativeClock {
    pub tick: u64,
    pub elapsed_ms: f64,
    pub dt_ms: f64,
    pub gap_ms: f64,
}

impl Default for NativeClock {
    fn default() -> Self {
        Self { tick: 0, elapsed_ms: 0.0, dt_ms: 0.0, gap_ms: 0.0 }
    }
}

#[derive(Resource, Clone, Debug)]
pub struct NativeRandom {
    pub state: u64,
}

impl Default for NativeRandom {
    fn default() -> Self {
        Self { state: 0x6a09_e667_f3bc_c909 }
    }
}

impl NativeRandom {
    pub fn next(&mut self) -> f64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        (x >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[derive(Resource, Clone, Debug, Default)]
pub struct NativeMathService {
    pub calls: u64,
}

impl NativeMathService {
    fn call(&mut self) {
        self.calls = self.calls.wrapping_add(1);
    }

    pub fn math(&mut self, op: &str, a: f64, b: f64, c: f64) -> Value {
        self.call();
        let value = match op {
            "clamp" => a.clamp(b, c),
            "clamp01" => a.clamp(0.0, 1.0),
            "lerp" => a + (b - a) * c,
            "smoothstep" => {
                let t = ((a - b) / (c - b).max(1e-9)).clamp(0.0, 1.0);
                t * t * (3.0 - 2.0 * t)
            }
            "shortestArc" => {
                let mut delta = (b - a) % 360.0;
                if delta > 180.0 { delta -= 360.0; }
                if delta <= -180.0 { delta += 360.0; }
                delta
            }
            "degToRad" => a.to_radians(),
            "radToDeg" => a.to_degrees(),
            "easing" => easing(op_easing(a), a),
            _ => a,
        };
        Value::from(value)
    }

    pub fn vector(&mut self, op: &str, a: Vec3, b: Vec3, scale: f64) -> Value {
        self.call();
        let value = match op {
            "add" => a + b,
            "sub" => a - b,
            "scale" => a * scale as f32,
            "cross" => a.cross(b),
            "normalize" => a.normalize_or_zero(),
            _ => a,
        };
        let scalar = match op {
            "dot" => a.dot(b),
            "length" => a.length(),
            "distance" => a.distance(b),
            _ => 0.0,
        };
        let mut map = Map::new();
        map.insert("x".into(), Value::from(value.x));
        map.insert("y".into(), Value::from(value.y));
        map.insert("z".into(), Value::from(value.z));
        map.insert("scalar".into(), Value::from(scalar));
        Value::Object(map)
    }

    pub fn noise(&mut self, op: &str, x: f64, z: f64, seed: f64) -> Value {
        self.call();
        let base = (x * 12.9898 + z * 78.233 + seed * 0.135).sin() * 43758.5453;
        let value = base.fract();
        Value::from(if op == "white2" { value } else { value * 2.0 - 1.0 })
    }

    pub fn report(&self) -> Value {
        json!({ "calls": self.calls })
    }
}

fn op_easing(_: f64) -> &'static str {
    "linear"
}

fn easing(_: &'static str, t: f64) -> f64 {
    t
}

pub fn register(app: &mut App) {
    app.init_resource::<NativeFeatureRegistry>()
        .init_resource::<NativeClock>()
        .init_resource::<NativeRandom>()
        .init_resource::<NativeMathService>()
        .init_resource::<NativeFeatureSummary>();
}

pub fn advance_clock(mut clock: ResMut<NativeClock>, time: Res<Time<Fixed>>, features: Res<super::features::FeatureState>) {
    clock.tick = features.tick;
    clock.dt_ms = time.delta_secs() as f64 * 1000.0;
    clock.elapsed_ms += clock.dt_ms;
}

pub fn read_feature_entities(
    mut summary: ResMut<NativeFeatureSummary>,
    registry: Res<NativeFeatureRegistry>,
    clock: Res<NativeClock>,
    random: Res<NativeRandom>,
    math: Res<NativeMathService>,
    worlds: Query<&PhysicsWorldEntity>,
    bodies: Query<&PhysicsBodyComponent>,
    parts: Query<&VoxelPartEntity>,
    fx: Query<&FxEntity>,
    vibe: Query<&VibeEntity>,
    jobs: Query<&JobEntity>,
    trades: Query<&TradeEntity>,
    dm: Query<&DmMessageEntity>,
) {
    summary.objects = registry.physics_worlds.len() + registry.physics_bodies.len() + registry.voxel_parts.len() + registry.fx.len() + registry.vibe.len() + registry.jobs.len() + registry.trades.len() + registry.dm.len();
    summary.physics_worlds = worlds.iter().count();
    summary.physics_bodies = bodies.iter().map(|body| { let _ = (body.world.as_str(), body.index, body.position, body.velocity, body.angular_velocity, body.mass, body.sleeping); 1 }).sum();
    summary.voxels = parts.iter().map(|part| { let _ = (part.id.as_str(), part.cells, part.palette); 1 }).sum();
    summary.fx = fx.iter().map(|state| { let _ = (state.id.as_str(), state.rate, state.lifetime, state.alive, state.stopped); 1 }).sum();
    summary.vibe = vibe.iter().map(|state| { let _ = (state.id.as_str(), state.playing, state.bpm, state.beat); 1 }).sum();
    summary.jobs = jobs.iter().map(|job| { let _ = (job.id.as_str(), job.job.as_str(), job.ok); 1 }).sum();
    summary.trades = trades.iter().map(|trade| { let _ = (trade.mine.as_str(), trade.with.as_str(), trade.for_item.as_str(), trade.accepted); 1 }).sum();
    summary.dm = dm.iter().map(|dm| { let _ = dm.message.get("message"); 1 }).sum();
    summary.clock_ms = clock.elapsed_ms + clock.gap_ms;
    summary.random_state = random.state;
    summary.math_calls = math.calls;
    let _ = worlds.iter().map(|world| (world.id.as_str(), world.gravity, world.body_count, world.snapshot_len)).count();
}

pub fn entity_report(registry: &NativeFeatureRegistry, clock: &NativeClock, random: &NativeRandom, math: &NativeMathService) -> Value {
    let mut map = Map::new();
    map.insert("physicsWorldEntities".into(), Value::from(registry.physics_worlds.len()));
    map.insert("physicsBodyEntities".into(), Value::from(registry.physics_bodies.len()));
    map.insert("voxelPartEntities".into(), Value::from(registry.voxel_parts.len()));
    map.insert("fxEntities".into(), Value::from(registry.fx.len()));
    map.insert("vibeEntities".into(), Value::from(registry.vibe.len()));
    map.insert("jobEntities".into(), Value::from(registry.jobs.len()));
    map.insert("tradeEntities".into(), Value::from(registry.trades.len()));
    map.insert("dmEntities".into(), Value::from(registry.dm.len()));
    map.insert("clockTick".into(), Value::from(clock.tick));
    map.insert("clockDtMs".into(), Value::from(clock.dt_ms));
    map.insert("randomState".into(), Value::from(random.state));
    map.insert("nativeMath".into(), math.report());
    Value::Object(map)
}
