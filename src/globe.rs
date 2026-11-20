use avian3d::prelude::{Collider, GravityScale, LinearVelocity, PhysicsSystems, Position, RigidBody, Rotation};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use serde_json::{Map, Value, json};
use crate::height::{CarveConfig, HashVersion, MAX_TERRAIN_CARVES, Planet, PlanetConfig};
use bevy::app::App;
use bevy::time::{Time, TimeUpdateStrategy, Virtual};
use std::time::Duration;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Resource, Clone, Debug)]
pub struct GlobeWorld {
    pub name: String,
    pub center: Vec3,
    pub radius: f32,
    pub sea_level: f32,
    pub amplitude: f32,
    pub seed: u64,
    pub chunk_span: f32,
    pub gravity: f32,
    pub stream_radius: f32,
    pub subdivisions: u32,
    pub body_altitude: f32,
    pub drop_height: f32,
    pub bodies: Vec<(f32, f32)>,
    pub hpf_tex_res: usize,
    pub hash_version: HashVersion,
    pub relief_scale: Option<f64>,
    pub anchor_dir: [f64; 3],
    pub carves: Vec<CarveConfig>,
    pub planet: Arc<Planet>,
}

const DEFAULT_SEED: u64 = 0x5eed_1234;
const DEFAULT_RADIUS: f32 = 6360.0;
const DEFAULT_HPF_TEX_RES: usize = 128;

impl Default for GlobeWorld {
    fn default() -> Self {
        Self {
            name: "globe".into(),
            center: Vec3::ZERO,
            radius: DEFAULT_RADIUS,
            sea_level: 0.0,
            amplitude: DEFAULT_RADIUS * 3.0 / 64.0,
            seed: DEFAULT_SEED,
            chunk_span: 0.35,
            gravity: 9.81,
            stream_radius: 40.0,
            subdivisions: 8,
            body_altitude: 0.5,
            drop_height: 0.0,
            bodies: vec![(0.0, 0.0)],
            hpf_tex_res: DEFAULT_HPF_TEX_RES,
            hash_version: HashVersion::Integer,
            relief_scale: None,
            anchor_dir: [0.0, 1.0, 0.0],
            carves: Vec::new(),
            planet: Arc::new(Planet::new(&default_planet_config())),
        }
    }
}

fn default_planet_config() -> PlanetConfig {
    PlanetConfig { seed: DEFAULT_SEED as i32, hpf_res: DEFAULT_HPF_TEX_RES, hash: HashVersion::Integer, relief_scale: None, anchor_dir: [0.0, 1.0, 0.0], radius: f64::from(DEFAULT_RADIUS), carves: Vec::new() }
}

fn vec3_of(value: &Value) -> Option<[f64; 3]> {
    let array = value.as_array()?;
    Some([array.first()?.as_f64()?, array.get(1)?.as_f64()?, array.get(2)?.as_f64()?])
}

fn carve_of(value: &Value) -> Option<CarveConfig> {
    if let Some(dir) = value.get("dir").and_then(vec3_of) {
        let inner_m = value.get("innerM")?.as_f64()?;
        let outer_m = value.get("outerM")?.as_f64()?;
        let length = dir.iter().map(|component| component * component).sum::<f64>().sqrt();
        return (length > 0.0 && inner_m >= 0.0 && outer_m > inner_m).then_some(CarveConfig::Parsed { dir, inner_m, outer_m });
    }
    let centre = value.get("center").and_then(Value::as_array)?;
    let (x, z) = (centre.first()?.as_f64()?, centre.get(1)?.as_f64()?);
    let radius_m = value.get("radius")?.as_f64()?;
    let falloff_m = value.get("falloff")?.as_f64()?;
    (x.is_finite() && z.is_finite() && radius_m >= 0.0 && falloff_m > 0.0).then_some(CarveConfig::Centre { x, z, radius_m, falloff_m })
}

#[derive(Component, Clone, Debug)]
pub struct GlobeChunk {
    pub vertices: usize,
    pub max_elevation: f32,
}

#[derive(Component, Clone, Debug, Default)]
pub struct GlobeOrientation {
    pub up: Vec3,
    pub heading: Vec3,
}

#[derive(Component, Clone, Debug, Default)]
pub struct GlobeBody {
    pub latitude: f32,
    pub longitude: f32,
    pub altitude: f32,
    pub speed: f32,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct GlobeRegistry {
    pub chunks: BTreeMap<(i32, i32), Entity>,
    pub bodies: Vec<Entity>,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct GlobeFocus(pub Vec3);

#[derive(Resource, Clone, Debug, Default)]
pub struct GlobeSummary {
    pub chunks: usize,
    pub bodies: usize,
    pub vertices: usize,
    pub max_elevation: f32,
    pub seed: u64,
    pub radius: f32,
    pub altitude: f32,
    pub body_radius: f32,
    pub gravity_at_body: Vec3,
    pub focus: Vec3,
}

struct ChunkGeometry {
    positions: Vec<Vec3>,
    colors: Vec<[f32; 4]>,
    indices: Vec<[u32; 3]>,
    max_elevation: f32,
}

impl GlobeWorld {
    fn planet_config(&self) -> PlanetConfig {
        PlanetConfig { seed: self.seed as i32, hpf_res: self.hpf_tex_res, hash: self.hash_version, relief_scale: self.relief_scale, anchor_dir: self.anchor_dir, radius: f64::from(self.radius), carves: self.carves.clone() }
    }

    pub fn from_value(value: Option<&Value>) -> Self {
        let mut globe = Self::default();
        if let Some(value) = value {
            globe.from_json(value);
        }
        globe
    }

    pub fn from_json(&mut self, value: &Value) {
        let Some(object) = value.as_object() else { return; };
        let before = self.planet_config();
        if let Some(name) = object.get("name").and_then(Value::as_str) { self.name = name.into(); }
        if object.contains_key("center") { self.center = crate::spec::vec3(object.get("center")); }
        for (field, target) in [("radius", &mut self.radius), ("seaLevel", &mut self.sea_level), ("amplitude", &mut self.amplitude), ("chunkSpan", &mut self.chunk_span), ("gravity", &mut self.gravity), ("streamRadius", &mut self.stream_radius), ("bodyAltitude", &mut self.body_altitude), ("dropHeight", &mut self.drop_height)] {
            if let Some(value) = object.get(field).and_then(Value::as_f64) { *target = value as f32; }
        }
        if let Some(value) = object.get("seed").and_then(Value::as_u64) { self.seed = value; }
        if let Some(value) = object.get("subdivisions").and_then(Value::as_u64) { self.subdivisions = value.clamp(1, 64) as u32; }
        if let Some(bodies) = object.get("bodies").and_then(Value::as_array) {
            self.bodies = bodies.iter().map(|body| {
                (number(body.get("latitude"), 0.0), self.wrap_lon(number(body.get("longitude"), 0.0)))
            }).collect();
        }
        self.radius = self.radius.max(1.0);
        self.stream_radius = self.stream_radius.max(self.radius * self.chunk_span);
        self.chunk_span = self.chunk_span.clamp(0.02, std::f32::consts::PI);
        if let Some(res) = object.get("hpfTexRes").and_then(Value::as_f64) {
            self.hpf_tex_res = (res as usize).max(2);
        }
        if let Some(hash) = object.get("hashVersion").and_then(Value::as_f64) {
            if hash == 1.0 {
                self.hash_version = HashVersion::Float;
            } else if hash == 2.0 {
                self.hash_version = HashVersion::Integer;
            } else {
                eprintln!("globe: hashVersion {hash} is not 1 or 2, keeping the current one");
            }
        }
        if let Some(scale) = object.get("reliefScale").and_then(Value::as_f64) {
            self.relief_scale = Some(scale);
        }
        let anchor: Option<Vec<f64>> = object.get("anchorDir").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_f64).collect());
        if let Some(v) = anchor.filter(|v| v.len() == 3) {
            self.anchor_dir = [v[0], v[1], v[2]];
        }
        if let Some(carves) = object.get("carves").and_then(Value::as_array) {
            self.carves = carves
                .iter()
                .filter_map(|value| {
                    let carve = carve_of(value);
                    if carve.is_none() {
                        eprintln!("globe: dropping a carve that needs dir, innerM and outerM, with innerM >= 0 and outerM > innerM: {value}");
                    }
                    carve
                })
                .collect();
            if self.carves.len() > MAX_TERRAIN_CARVES {
                eprintln!("globe: {} carves given, only the first {MAX_TERRAIN_CARVES} are used", self.carves.len());
                self.carves.truncate(MAX_TERRAIN_CARVES);
            }
        }
        if self.planet_config() != before {
            self.planet = Arc::new(Planet::new(&self.planet_config()));
        }
    }

    pub fn lat_lon_to_unit(&self, latitude: f32, longitude: f32) -> Vec3 {
        let (lat, lon) = (latitude.to_radians(), longitude.to_radians());
        Vec3::new(lat.cos() * lon.sin(), lat.sin(), lat.cos() * lon.cos())
    }

    pub fn unit_to_lat_lon(&self, direction: Vec3) -> (f32, f32) {
        let unit = direction.normalize_or_zero();
        (unit.y.asin().to_degrees(), unit.x.atan2(unit.z).to_degrees())
    }

    pub fn wrap_lon(&self, longitude: f32) -> f32 {
        ((longitude + 180.0) % 360.0 + 360.0) % 360.0 - 180.0
    }

    pub fn hash(&self, x: i64, y: i64, z: i64) -> f32 {
        let mut state = self.seed ^ (x as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ (y as u64).wrapping_mul(0xbf58_476d_1ce4_e5b9) ^ (z as u64).wrapping_mul(0x94d0_49bb_1331_11eb);
        state ^= state >> 30;
        state = state.wrapping_mul(0xbf58_476d_1ce4_e5b9);
        state ^= state >> 27;
        (state >> 11) as f32 / (1u64 << 53) as f32
    }

    pub fn elevation(&self, latitude: f32, longitude: f32) -> f32 {
        let unit = self.lat_lon_to_unit(latitude, longitude);
        self.planet.height([f64::from(unit.x), f64::from(unit.y), f64::from(unit.z)], f64::from(self.radius)) as f32
    }

    pub fn surface_position(&self, latitude: f32, longitude: f32, altitude: f32) -> Vec3 {
        let unit = self.lat_lon_to_unit(latitude, longitude);
        self.center + unit * (self.radius + self.elevation(latitude, longitude) + altitude)
    }

    pub fn gravity_vector(&self, position: Vec3) -> Vec3 {
        let toward = self.center - position;
        if toward.length_squared() < 1e-6 { return Vec3::ZERO; }
        toward.normalize() * self.gravity
    }

    pub fn orientation(&self, latitude: f32, longitude: f32, heading: f32) -> Quat {
        let (lat, lon) = (latitude.to_radians(), longitude.to_radians());
        let up = self.lat_lon_to_unit(latitude, longitude);
        let east = Vec3::new(lon.cos(), 0.0, -lon.sin());
        let north = Vec3::new(-lat.sin() * lon.sin(), lat.cos(), -lat.sin() * lon.cos());
        let forward = north * heading.to_radians().cos() + east * heading.to_radians().sin();
        let back = -forward;
        let right = up.cross(back);
        Quat::from_mat3(&Mat3::from_cols(right, up, back)).normalize()
    }

    pub(crate) fn grid(&self) -> (i32, i32) {
        let span = self.chunk_span.to_degrees();
        ((180.0 / span).ceil() as i32, (360.0 / span).ceil() as i32)
    }

    pub(crate) fn cell_bounds(&self, row: i32, column: i32) -> (f32, f32, f32, f32) {
        let span = self.chunk_span.to_degrees();
        let lat0 = -90.0 + row as f32 * span;
        let lon0 = -180.0 + column as f32 * span;
        (lat0, (lat0 + span).min(90.0), lon0, (lon0 + span).min(180.0))
    }

    fn chunk_geometry(&self, bounds: (f32, f32, f32, f32)) -> ChunkGeometry {
        let (lat0, lat1, lon0, lon1) = bounds;
        let n = self.subdivisions as usize;
        let mut positions = Vec::with_capacity((n + 1) * (n + 1));
        let mut colors = Vec::with_capacity((n + 1) * (n + 1));
        let mut max_elevation = 0.0_f32;
        for i in 0..=n {
            for j in 0..=n {
                let lat = lat0 + (lat1 - lat0) * i as f32 / n as f32;
                let lon = lon0 + (lon1 - lon0) * j as f32 / n as f32;
                let elevation = self.elevation(lat, lon);
                max_elevation = max_elevation.max(elevation);
                positions.push(self.center + self.lat_lon_to_unit(lat, lon) * (self.radius + elevation));
                colors.push(biome_color(elevation / self.amplitude.max(1e-3), elevation <= self.sea_level));
            }
        }
        let mut indices = Vec::with_capacity(n * n * 2);
        for i in 0..n {
            for j in 0..n {
                let a = (i * (n + 1) + j) as u32;
                let b = a + (n + 1) as u32;
                let c = a + 1;
                let d = b + 1;
                for mut triangle in [[a, b, d], [a, d, c]] {
                    let p0 = positions[triangle[0] as usize];
                    let edge = positions[triangle[1] as usize] - p0;
                    let other = positions[triangle[2] as usize] - p0;
                    if edge.cross(other).dot(p0 - self.center) < 0.0 { triangle.swap(1, 2); }
                    indices.push(triangle);
                }
            }
        }
        ChunkGeometry { positions, colors, indices, max_elevation }
    }

    pub fn chunk(&self, lat: i32, lon: i32) -> Map<String, Value> {
        let span = self.chunk_span.to_degrees();
        let latitude = (lat as f32 * span).clamp(-90.0, 90.0);
        let longitude = self.wrap_lon(lon as f32 * span);
        let elevation = self.elevation(latitude, longitude);
        let center = self.surface_position(latitude, longitude, 0.0);
        json!({
            "lat": lat, "lon": lon, "latitude": latitude, "longitude": longitude,
            "latitudeRange": [latitude - span * 0.5, latitude + span * 0.5],
            "longitudeRange": [longitude - span * 0.5, longitude + span * 0.5],
            "center": vec_json(center), "up": vec_json((center - self.center).normalize_or_zero()),
            "elevation": elevation, "radius": self.radius + elevation,
            "vertices": (self.subdivisions as usize + 1).pow(2), "wrap": lon.abs() as f32 * span > 180.0
        }).as_object().cloned().unwrap_or_default()
    }

    pub fn report(&self, summary: &GlobeSummary) -> Value {
        json!({
            "name": self.name, "center": vec_json(self.center), "radius": self.radius,
            "seaLevel": self.sea_level, "amplitude": self.amplitude, "seed": self.seed,
            "chunkSpan": self.chunk_span.to_degrees(), "gravity": self.gravity, "streamRadius": self.stream_radius,
            "subdivisions": self.subdivisions, "bodyAltitude": self.body_altitude, "dropHeight": self.drop_height,
            "chunks": summary.chunks, "bodies": summary.bodies, "vertices": summary.vertices,
            "maxElevation": summary.max_elevation, "altitude": summary.altitude, "bodyRadius": summary.body_radius, "gravityAtBody": vec_json(summary.gravity_at_body), "focus": vec_json(summary.focus),
            "native": ["Mesh", "StandardMaterial", "Collider::trimesh", "RigidBody", "Transform", "GlobalTransform", "Time", "Component", "Resource", "FixedUpdate"]
        })
    }
}

fn biome_color(relative: f32, water: bool) -> [f32; 4] {
    let color = if water {
        Vec3::new(0.10, 0.25, 0.55)
    } else if relative < 0.25 {
        Vec3::new(0.80, 0.74, 0.50)
    } else if relative < 0.6 {
        Vec3::new(0.25, 0.55, 0.22)
    } else if relative < 0.85 {
        Vec3::new(0.45, 0.40, 0.35)
    } else {
        Vec3::new(0.95, 0.95, 0.97)
    };
    [color.x, color.y, color.z, 1.0]
}

pub fn vec_json(value: Vec3) -> Value { json!({ "x": value.x, "y": value.y, "z": value.z }) }
pub fn quat_json(value: Quat) -> Value { json!({ "x": value.x, "y": value.y, "z": value.z, "w": value.w }) }
pub fn json_vec3(value: Option<&Value>) -> Vec3 { crate::spec::vec3(value) }
pub fn number(value: Option<&Value>, fallback: f32) -> f32 { value.and_then(Value::as_f64).map(|v| v as f32).unwrap_or(fallback) }

pub fn register(app: &mut App) {
    let globe = app.world().get_resource::<GlobeWorld>().cloned().unwrap_or_default();
    let focus = GlobeFocus(globe.surface_position(0.0, 0.0, globe.body_altitude));
    app.init_resource::<GlobeWorld>()
        .init_resource::<GlobeRegistry>()
        .init_resource::<GlobeSummary>()
        .insert_resource(focus)
        .add_systems(Startup, spawn_bodies)
        .add_systems(FixedUpdate, update_focus.before(sync_chunks).before(crate::script::run_scripts))
        .add_systems(FixedUpdate, sync_chunks.before(crate::script::run_scripts))
        .add_systems(FixedUpdate, apply_spherical_gravity.before(crate::script::run_scripts))
        .add_systems(FixedUpdate, read_summary.after(sync_chunks).before(crate::script::run_scripts))
        .add_systems(FixedPostUpdate, settle_bodies.after(PhysicsSystems::StepSimulation).before(PhysicsSystems::Writeback));
}

fn spawn_bodies(mut commands: Commands, globe: Res<GlobeWorld>, mut registry: ResMut<GlobeRegistry>) {
    for &(latitude, longitude) in &globe.bodies {
        let spawn_altitude = globe.body_altitude + globe.drop_height;
        let surface = globe.surface_position(latitude, longitude, spawn_altitude);
        let rotation = globe.orientation(latitude, longitude, 0.0);
        let entity = commands.spawn((
            GlobeBody { latitude, longitude, altitude: spawn_altitude, speed: 0.0 },
            GlobeOrientation::default(),
            RigidBody::Dynamic,
            Collider::sphere(0.5),
            GravityScale(0.0),
            LinearVelocity::default(),
            Position(surface),
            Rotation(rotation),
            Transform::from_translation(surface).with_rotation(rotation),
        )).id();
        registry.bodies.push(entity);
    }
}

const FRAMING_HEIGHT_M: f32 = 10.0;
const FRAMING_BACK_M: f32 = 14.0;

pub fn frame_globe_camera(globe: Res<GlobeWorld>, focus: Res<GlobeFocus>, mut cameras: Query<&mut Transform, With<Camera3d>>) {
    let up = (focus.0 - globe.center).normalize_or_zero();
    let eye = focus.0 + up * FRAMING_HEIGHT_M + up.any_orthogonal_vector() * FRAMING_BACK_M;
    for mut transform in &mut cameras {
        *transform = Transform::from_translation(eye).looking_at(focus.0, up);
    }
}

pub const CENSUS_ENV: &str = "BEVY_JAYZEE_CENSUS";
const CENSUS_FREEZE_S: f64 = 10.0;

#[derive(Resource)]
pub struct CensusMode;

fn census_frame() -> Duration {
    Duration::from_secs_f64(1.0 / 60.0)
}

pub fn add_census_systems(app: &mut App) {
    if std::env::var(CENSUS_ENV).as_deref() != Ok("1") {
        return;
    }
    app.insert_resource(CensusMode);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(census_frame()));
    app.add_systems(Update, census_freeze);
}

fn census_freeze(time: Res<Time<Virtual>>, mut strategy: ResMut<TimeUpdateStrategy>, mut frozen: Local<bool>) {
    if *frozen || time.elapsed_secs_f64() < CENSUS_FREEZE_S {
        return;
    }
    *strategy = TimeUpdateStrategy::ManualDuration(Duration::ZERO);
    *frozen = true;
    eprintln!("census: simulation frozen at {:.3} s of virtual time", time.elapsed_secs_f64());
}

fn update_focus(bodies: Query<&Position, With<GlobeBody>>, mut focus: ResMut<GlobeFocus>, census: Option<Res<CensusMode>>, mut latched: Local<bool>) {
    if *latched {
        return;
    }
    if let Some(position) = bodies.iter().next() {
        focus.0 = position.0;
        *latched = census.is_some();
    }
}

fn sync_chunks(
    mut commands: Commands,
    globe: Res<GlobeWorld>,
    focus: Res<GlobeFocus>,
    mut registry: ResMut<GlobeRegistry>,
    mut last_focus: Local<Option<Vec3>>,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut material: Local<Option<Handle<StandardMaterial>>>,
    vegetation: Option<Res<crate::vegetation::VegetationAssets>>,
) {
    let step = globe.radius * globe.chunk_span;
    if !registry.chunks.is_empty() && last_focus.is_some_and(|previous| previous.distance(focus.0) < step * 0.5) {
        return;
    }
    *last_focus = Some(focus.0);
    let (rows, columns) = globe.grid();
    for row in 0..rows {
        for column in 0..columns {
            let key = (row, column);
            let bounds = globe.cell_bounds(row, column);
            let center = globe.surface_position((bounds.0 + bounds.1) * 0.5, (bounds.2 + bounds.3) * 0.5, 0.0);
            let inside = center.distance(focus.0) <= globe.stream_radius;
            match (inside, registry.chunks.get(&key).copied()) {
                (true, None) => {
                    let geometry = globe.chunk_geometry(bounds);
                    let vertices = geometry.positions.len();
                    let normals: Vec<[f32; 3]> = geometry.positions.iter().map(|p| (*p - globe.center).normalize_or_zero().to_array()).collect();
                    let positions: Vec<[f32; 3]> = geometry.positions.iter().map(|p| p.to_array()).collect();
                    let mut entity = commands.spawn((
                        GlobeChunk { vertices, max_elevation: geometry.max_elevation },
                        RigidBody::Static,
                        Collider::trimesh(geometry.positions.clone(), geometry.indices.clone()),
                        Transform::default(),
                        GlobalTransform::default(),
                    ));
                    if let Some(meshes) = meshes.as_mut() {
                        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
                        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
                        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, geometry.colors.clone());
                        mesh.insert_indices(Indices::U32(geometry.indices.iter().flatten().copied().collect()));
                        entity.insert(Mesh3d(meshes.add(mesh)));
                    }
                    if let Some(materials) = materials.as_mut() {
                        let handle = material.get_or_insert_with(|| materials.add(StandardMaterial { base_color: Color::WHITE, perceptual_roughness: 0.95, ..default() })).clone();
                        entity.insert(MeshMaterial3d(handle));
                    }
                    let chunk = entity.id();
                    registry.chunks.insert(key, chunk);
                    if let Some(assets) = vegetation.as_deref() {
                        crate::vegetation::spawn_chunk_trees(&mut commands, &globe, assets, chunk, key, bounds);
                    }
                }
                (false, Some(entity)) => {
                    commands.entity(entity).despawn();
                    registry.chunks.remove(&key);
                }
                _ => {}
            }
        }
    }
}

fn apply_spherical_gravity(
    globe: Res<GlobeWorld>,
    time: Res<Time<Fixed>>,
    mut bodies: Query<(&Position, &mut LinearVelocity), With<GlobeBody>>,
) {
    let dt = time.delta_secs();
    for (position, mut velocity) in &mut bodies {
        velocity.0 += globe.gravity_vector(position.0) * dt;
    }
}

fn settle_bodies(
    globe: Res<GlobeWorld>,
    mut bodies: Query<(&mut GlobeBody, &mut GlobeOrientation, &mut Position, &mut Rotation, &mut Transform, &mut LinearVelocity)>,
) {
    for (mut body, mut orientation, mut position, mut rotation, mut transform, mut velocity) in &mut bodies {
        let offset = position.0 - globe.center;
        let unit = offset.normalize_or_zero();
        let (latitude, longitude) = globe.unit_to_lat_lon(unit);
        let floor = globe.surface_position(latitude, longitude, globe.body_altitude);
        if offset.length() < (floor - globe.center).length() {
            position.0 = floor;
            let radial = velocity.0.dot(unit);
            if radial < 0.0 { velocity.0 -= unit * radial; }
        }
        let q = globe.orientation(latitude, longitude, 0.0);
        let altitude = (position.0 - globe.center).length() - globe.radius - globe.elevation(latitude, longitude);
        body.latitude = latitude;
        body.longitude = longitude;
        body.altitude = altitude;
        body.speed = velocity.0.length();
        orientation.up = unit;
        orientation.heading = q * Vec3::NEG_Z;
        rotation.0 = q;
        transform.rotation = q;
        transform.translation = position.0;
    }
}

fn read_summary(
    globe: Res<GlobeWorld>,
    registry: Res<GlobeRegistry>,
    focus: Res<GlobeFocus>,
    chunks: Query<&GlobeChunk>,
    bodies: Query<&GlobeBody>,
    positions: Query<&Position, With<GlobeBody>>,
    mut summary: ResMut<GlobeSummary>,
) {
    summary.chunks = registry.chunks.len();
    summary.bodies = bodies.iter().count();
    summary.vertices = chunks.iter().map(|chunk| chunk.vertices).sum();
    summary.max_elevation = chunks.iter().map(|chunk| chunk.max_elevation).fold(0.0_f32, f32::max);
    summary.altitude = bodies.iter().next().map(|body| body.altitude).unwrap_or(0.0);
    summary.body_radius = positions.iter().next().map(|p| p.0.distance(globe.center)).unwrap_or(0.0);
    summary.gravity_at_body = positions.iter().next().map(|p| globe.gravity_vector(p.0)).unwrap_or(Vec3::ZERO);
    summary.focus = focus.0;
    summary.seed = globe.seed;
    summary.radius = globe.radius;
}

pub fn dispatch(globe: &mut GlobeWorld, registry: &GlobeRegistry, op: &str, payload: &Map<String, Value>) -> Value {
    match op {
        "configure" => {
            globe.from_json(&Value::Object(payload.clone()));
            json!({ "ok": true, "name": globe.name, "radius": globe.radius, "seed": globe.seed })
        }
        "info" => globe.report(&GlobeSummary::default()),
        "toCartesian" => {
            let lat = number(payload.get("latitude").or_else(|| payload.get("lat")), 0.0);
            let lon = globe.wrap_lon(number(payload.get("longitude").or_else(|| payload.get("lon")), 0.0));
            let altitude = number(payload.get("altitude"), 0.0);
            let position = globe.surface_position(lat, lon, altitude);
            json!({ "x": position.x, "y": position.y, "z": position.z, "latitude": lat, "longitude": lon, "altitude": altitude, "radius": position.distance(globe.center) })
        }
        "toSpherical" => {
            let position = json_vec3(payload.get("position"));
            let offset = position - globe.center;
            let (latitude, longitude) = globe.unit_to_lat_lon(offset);
            let distance = offset.length();
            json!({ "latitude": latitude, "longitude": longitude, "altitude": distance - globe.radius - globe.elevation(latitude, longitude), "radius": distance })
        }
        "elevation" | "terrainHeight" => {
            let lat = number(payload.get("latitude").or_else(|| payload.get("lat")).or_else(|| payload.get("z")), 0.0);
            let lon = number(payload.get("longitude").or_else(|| payload.get("lon")).or_else(|| payload.get("x")), 0.0);
            json!({ "height": globe.elevation(lat, lon), "latitude": lat, "longitude": globe.wrap_lon(lon) })
        }
        "chunk" => {
            let lat = payload.get("lat").and_then(Value::as_i64).unwrap_or(0) as i32;
            let lon = payload.get("lon").and_then(Value::as_i64).unwrap_or(0) as i32;
            Value::Object(globe.chunk(lat, lon))
        }
        "chunks" => {
            let limit = payload.get("limit").and_then(Value::as_u64).unwrap_or(16).min(256) as i32;
            let values: Vec<Value> = (-limit / 2..limit / 2).map(|i| Value::Object(globe.chunk(0, i))).collect();
            json!({ "count": registry.chunks.len(), "sample": values })
        }
        "gravity" => {
            let position = json_vec3(payload.get("position"));
            vec_json(globe.gravity_vector(position))
        }
        "orientation" => {
            let lat = number(payload.get("latitude").or_else(|| payload.get("lat")), 0.0);
            let lon = number(payload.get("longitude").or_else(|| payload.get("lon")), 0.0);
            let heading = number(payload.get("heading"), 0.0);
            let q = globe.orientation(lat, lon, heading);
            let mut value = quat_json(q).as_object().cloned().unwrap_or_default();
            value.insert("up".into(), vec_json(globe.lat_lon_to_unit(lat, lon)));
            value.insert("forward".into(), vec_json(q * Vec3::NEG_Z));
            Value::Object(value)
        }
        "navigation" | "navigate" | "wrap" => {
            let from_lat = number(payload.get("fromLatitude").or_else(|| payload.get("fromLat")), 0.0);
            let from_lon = number(payload.get("fromLongitude").or_else(|| payload.get("fromLon")), 0.0);
            let to_lat = number(payload.get("toLatitude").or_else(|| payload.get("toLat")), 0.0);
            let to_lon = globe.wrap_lon(number(payload.get("toLongitude").or_else(|| payload.get("toLon")), 0.0));
            let a = globe.lat_lon_to_unit(from_lat, from_lon);
            let b = globe.lat_lon_to_unit(to_lat, to_lon);
            let angle = a.dot(b).clamp(-1.0, 1.0).acos();
            let direction = (b - a * a.dot(b)).normalize_or_zero();
            json!({ "from": [from_lat, globe.wrap_lon(from_lon)], "to": [to_lat, to_lon], "distance": angle * globe.radius, "angle": angle.to_degrees(), "direction": vec_json(direction), "wrapped": to_lon })
        }
        _ => Value::Null,
    }
}

pub fn provenance() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        ("globeConfigure", "Bevy ECS GlobeWorld resource"),
        ("globeInfo", "Bevy ECS GlobeWorld/GlobeSummary"),
        ("globeToCartesian", "Bevy math Vec3/Quat spherical conversion"),
        ("globeToSpherical", "Bevy math Vec3 spherical conversion"),
        ("globeElevation", "Bevy ECS planet height from the spoint height spec"),
        ("globeChunk", "Bevy ECS procedural GlobeChunk entities with Bevy Mesh and Avian trimesh"),
        ("globeChunks", "Bevy ECS procedural GlobeChunk entities"),
        ("globeGravity", "Bevy ECS radial gravity system"),
        ("globeOrientation", "Bevy math Quat/Transform"),
        ("globeNavigation", "Bevy math Vec3 great-circle wraparound"),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn globe() -> GlobeWorld {
        GlobeWorld::default()
    }

    #[test]
    fn elevation_is_deterministic_and_bounded() {
        let g = globe();
        for i in 0..50 {
            let lat = -80.0 + i as f32 * 3.1;
            let lon = -170.0 + i as f32 * 6.7;
            assert_eq!(g.elevation(lat, lon).to_bits(), globe().elevation(lat, lon).to_bits());
            assert!(g.elevation(lat, lon) <= g.amplitude);
        }
    }

    #[test]
    fn default_radius_is_the_spoint_design_radius() {
        assert_eq!(globe().radius, 6360.0);
    }

    #[test]
    fn relief_scales_linearly_with_the_radius() {
        let g = globe();
        let direction = [0.3, 0.5, -0.8];
        let height = g.planet.height(direction, f64::from(g.radius));
        assert_eq!(g.planet.height(direction, 2.0 * f64::from(g.radius)), 2.0 * height);
    }

    #[test]
    fn surface_along_direction_matches_the_spoint_solve() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!("../testdata/parity/spoint-surface.json")).unwrap();
        let globe = globe();
        assert_eq!(fixture["seed"].as_u64(), Some(globe.seed));
        assert_eq!(fixture["radius"].as_str().unwrap().parse::<f32>().unwrap(), globe.radius);
        let worst = fixture["points"]
            .as_array()
            .unwrap()
            .iter()
            .map(|point| {
                let latitude: f32 = point["lat"].as_str().unwrap().parse().unwrap();
                let longitude: f32 = point["lon"].as_str().unwrap().parse().unwrap();
                let spoint: f64 = point["r"].as_str().unwrap().parse().unwrap();
                (f64::from(globe.radius) + f64::from(globe.elevation(latitude, longitude)) - spoint).abs()
            })
            .fold(0.0_f64, f64::max);
        let ulp = f32::from_bits(globe.radius.to_bits() + 1) - globe.radius;
        assert!(worst <= f64::from(ulp), "surface radius differs from spoint by up to {worst} m, one f32 ulp at the radius is {ulp} m");
    }

    fn numeric(value: &Value) -> Value {
        match value {
            Value::String(text) => match text.parse::<f64>() {
                Ok(number) => Value::from(number),
                Err(_) => value.clone(),
            },
            Value::Array(items) => Value::Array(items.iter().map(numeric).collect()),
            Value::Object(map) => Value::Object(map.iter().map(|(key, value)| (key.clone(), numeric(value))).collect()),
            _ => value.clone(),
        }
    }

    fn spoint_sampler_fixture() -> Value {
        numeric(&serde_json::from_str::<Value>(include_str!("../testdata/parity/spoint-sampler.json")).unwrap())
    }

    fn spoint_case<'a>(fixture: &'a Value, name: &str) -> &'a Value {
        fixture["cases"].as_array().unwrap().iter().find(|case| case["name"] == name).unwrap()
    }

    fn worst_elevation_difference(globe: &GlobeWorld, fixture: &Value, case: &Value) -> f64 {
        fixture["points"]
            .as_array()
            .unwrap()
            .iter()
            .zip(case["heights"].as_array().unwrap())
            .map(|(point, height)| {
                let elevation = globe.elevation(point["lat"].as_f64().unwrap() as f32, point["lon"].as_f64().unwrap() as f32);
                (f64::from(elevation) - height.as_f64().unwrap()).abs()
            })
            .fold(0.0_f64, f64::max)
    }

    #[test]
    fn default_elevation_matches_the_spoint_sampler() {
        let fixture = spoint_sampler_fixture();
        let case = spoint_case(&fixture, "default");
        let globe = GlobeWorld::default();
        assert_eq!(globe.seed, fixture["seed"].as_u64().unwrap());
        assert_eq!(f64::from(globe.radius), case["config"]["radius"].as_f64().unwrap());
        let worst = worst_elevation_difference(&globe, &fixture, case);
        let ulp = f32::from_bits(globe.radius.to_bits() + 1) - globe.radius;
        assert!(worst <= f64::from(ulp), "elevation differs from spoint by up to {worst} m, one f32 ulp at the radius is {ulp} m");
    }

    #[test]
    fn carved_elevation_matches_the_spoint_sampler() {
        let fixture = spoint_sampler_fixture();
        let case = spoint_case(&fixture, "carves");
        let globe = GlobeWorld::from_value(Some(&case["config"]));
        let worst = worst_elevation_difference(&globe, &fixture, case);
        let ulp = f32::from_bits(globe.radius.to_bits() + 1) - globe.radius;
        assert!(worst <= f64::from(ulp), "carved elevation differs from spoint by up to {worst} m, one f32 ulp at the radius is {ulp} m");
    }

    #[test]
    fn hpf_tex_res_elevation_matches_the_spoint_sampler() {
        let fixture = spoint_sampler_fixture();
        let case = spoint_case(&fixture, "hpfTexRes");
        let globe = GlobeWorld::from_value(Some(&case["config"]));
        assert_eq!(globe.hpf_tex_res as f64, case["config"]["hpfTexRes"].as_f64().unwrap());
        let worst = worst_elevation_difference(&globe, &fixture, case);
        let ulp = f32::from_bits(globe.radius.to_bits() + 1) - globe.radius;
        assert!(worst <= f64::from(ulp), "hpf elevation differs from spoint by up to {worst} m, one f32 ulp at the radius is {ulp} m");
    }

    #[test]
    fn census_clock_steps_one_frame_at_a_time_then_freezes() {
        let mut app = App::new();
        app.add_plugins(bevy::MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(census_frame()));
        app.add_systems(Update, census_freeze);
        app.finish();
        app.cleanup();
        let elapsed: Vec<f64> = (0..900)
            .map(|_| {
                app.update();
                app.world().resource::<Time<Virtual>>().elapsed_secs_f64()
            })
            .collect();
        let frame = 1.0 / 60.0;
        assert!((elapsed[60] - elapsed[0] - 60.0 * frame).abs() < 1e-6, "60 census frames advanced {} s, not 1 s", elapsed[60] - elapsed[0]);
        assert!(elapsed[899] >= CENSUS_FREEZE_S && elapsed[899] <= CENSUS_FREEZE_S + 2.0 * frame, "census froze at {} s", elapsed[899]);
        assert_eq!(elapsed[899], elapsed[800], "the census simulation kept moving after the freeze");
    }

    #[test]
    fn hash_version_one_elevation_matches_the_spoint_sampler() {
        let fixture = spoint_sampler_fixture();
        let case = spoint_case(&fixture, "hashVersion1");
        let globe = GlobeWorld::from_value(Some(&case["config"]));
        assert_eq!(globe.hash_version, HashVersion::Float);
        let worst = worst_elevation_difference(&globe, &fixture, case);
        let ulp = f32::from_bits(globe.radius.to_bits() + 1) - globe.radius;
        assert!(worst <= f64::from(ulp), "hashVersion 1 elevation differs from spoint by up to {worst} m, one f32 ulp at the radius is {ulp} m");
    }

    #[test]
    fn relief_scale_elevation_matches_the_spoint_sampler() {
        let fixture = spoint_sampler_fixture();
        let case = spoint_case(&fixture, "reliefScale");
        let globe = GlobeWorld::from_value(Some(&case["config"]));
        assert_eq!(globe.relief_scale, case["config"]["reliefScale"].as_f64());
        let worst = worst_elevation_difference(&globe, &fixture, case);
        let ulp = f32::from_bits(globe.radius.to_bits() + 1) - globe.radius;
        assert!(worst <= f64::from(ulp), "reliefScale elevation differs from spoint by up to {worst} m, one f32 ulp at the radius is {ulp} m");
    }

    #[test]
    fn carve_centre_elevation_matches_the_spoint_sampler() {
        let fixture = spoint_sampler_fixture();
        let case = spoint_case(&fixture, "carveCentre");
        let globe = GlobeWorld::from_value(Some(&case["config"]));
        assert_eq!(globe.carves.len(), 2);
        let worst = worst_elevation_difference(&globe, &fixture, case);
        let ulp = f32::from_bits(globe.radius.to_bits() + 1) - globe.radius;
        assert!(worst <= f64::from(ulp), "centre carve elevation differs from spoint by up to {worst} m, one f32 ulp at the radius is {ulp} m");
    }

    #[test]
    fn gravity_points_to_centre() {
        let g = globe();
        let position = g.surface_position(30.0, 45.0, 2.0);
        let pull = g.gravity_vector(position);
        assert!((pull.length() - g.gravity).abs() < 1e-4);
        assert!(pull.normalize().dot((g.center - position).normalize()) > 0.9999);
    }

    #[test]
    fn orientation_is_orthonormal_and_up_matches_normal() {
        let g = globe();
        let q = g.orientation(-20.0, 100.0, 35.0);
        assert!((q.length() - 1.0).abs() < 1e-5);
        let up = q * Vec3::Y;
        assert!(up.dot(g.lat_lon_to_unit(-20.0, 100.0)) > 0.9999);
        assert!((q * Vec3::NEG_Z).dot(up).abs() < 1e-4);
    }

    #[test]
    fn chunk_grid_covers_sphere_and_geometry_faces_outward() {
        let g = globe();
        let (rows, columns) = g.grid();
        assert!(rows >= 2 && columns >= 2);
        let geometry = g.chunk_geometry(g.cell_bounds(rows / 2, columns / 2));
        assert_eq!(geometry.positions.len(), (g.subdivisions as usize + 1).pow(2));
        for triangle in &geometry.indices {
            let p0 = geometry.positions[triangle[0] as usize];
            let normal = (geometry.positions[triangle[1] as usize] - p0).cross(geometry.positions[triangle[2] as usize] - p0);
            assert!(normal.dot(p0 - g.center) >= 0.0);
        }
    }
}
