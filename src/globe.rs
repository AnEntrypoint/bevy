use crate::height::planet_frame::{ChartFrame, anchor_basis};
use crate::height::{CarveConfig, HashVersion, MAX_TERRAIN_CARVES, Planet, PlanetConfig};
use crate::planet_lod::collider::{Chart, ColliderGround, ColliderStreamPlugin, Lattice, NearBody};
use crate::planet_lod::near_field::{NearField, NearFieldOptions};
use crate::planet_lod::quadtree::QuadtreeConfig;
use crate::planet_lod::stream::{PlanetLodPlugin, PlanetLodStats};
use crate::planet_material::terrain::{TerrainFrame, TerrainMaterialPlugin};
use crate::planet_sky::light::{FILL_FROM, Fill, LightPlugin, Quality, Sun};
use crate::planet_sky::lut::{AtmosphereLutPlugin, luts_ready};
use crate::planet_sky::sky::{PlanetSkyPlugin, SkyCamera, SkyPlanet, SkySun};
use crate::planet_sky::tonemap::{ToneMapPlugin, reference_view};
use crate::planet_water::surface::WaterPlugin;
use avian3d::prelude::{Collider, GravityScale, LinearVelocity, PhysicsSystems, Position, RigidBody, Rotation};
use bevy::app::App;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::math::DVec3;
use bevy::prelude::*;
use bevy::render::view::window::screenshot::{Screenshot, save_to_disk};
use bevy::time::{Time, TimeUpdateStrategy, Virtual};
use serde_json::{Map, Value, json};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

#[derive(Resource, Clone, Debug)]
pub struct GlobeWorld {
    pub name: String,
    pub center: Vec3,
    pub radius: f32,
    pub sea_level: f32,
    pub amplitude: f32,
    pub seed: u64,
    pub max_level: u8,
    pub gravity: f32,
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

const DEFAULT_SEED: u64 = 1337;
const DEFAULT_RADIUS: f32 = 63_600.0;
const DEFAULT_HPF_TEX_RES: usize = 128;
const DEFAULT_MAX_LEVEL: u8 = 13;
const MAX_LEVEL_LIMIT: u64 = 24;
const DEFAULT_ANCHOR_DIR: [f64; 3] = [-0.641, 0.2558, 0.7237];
const CAMERA_VIEWPORT_HEIGHT_PX: f64 = 720.0;
const CAMERA_FOV_Y_RAD: f64 = std::f64::consts::FRAC_PI_4;
const CAMERA_FAR_RADII: f32 = 4.0;
const VIEWER_QUALITY: Quality = Quality::Ultra;

impl Default for GlobeWorld {
    fn default() -> Self {
        Self {
            name: "globe".into(),
            center: Vec3::ZERO,
            radius: DEFAULT_RADIUS,
            sea_level: 0.0,
            amplitude: DEFAULT_RADIUS * 3.0 / 64.0,
            seed: DEFAULT_SEED,
            max_level: DEFAULT_MAX_LEVEL,
            gravity: 9.81,
            body_altitude: 0.5,
            drop_height: 0.0,
            bodies: vec![(0.0, 0.0)],
            hpf_tex_res: DEFAULT_HPF_TEX_RES,
            hash_version: HashVersion::Integer,
            relief_scale: None,
            anchor_dir: DEFAULT_ANCHOR_DIR,
            carves: Vec::new(),
            planet: Arc::new(Planet::new(&default_planet_config())),
        }
    }
}

fn default_planet_config() -> PlanetConfig {
    PlanetConfig { seed: DEFAULT_SEED as i32, hpf_res: DEFAULT_HPF_TEX_RES, hash: HashVersion::Integer, relief_scale: None, anchor_dir: DEFAULT_ANCHOR_DIR, radius: f64::from(DEFAULT_RADIUS), carves: Vec::new() }
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
pub struct GlobeFocus(pub Vec3);

#[derive(Resource, Clone, Debug, Default)]
pub struct GlobeSummary {
    pub chunks: usize,
    pub bodies: usize,
    pub vertices: usize,
    pub min_elevation: f32,
    pub max_elevation: f32,
    pub seed: u64,
    pub radius: f32,
    pub altitude: f32,
    pub body_radius: f32,
    pub gravity_at_body: Vec3,
    pub focus: Vec3,
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
        for (field, target) in [("radius", &mut self.radius), ("seaLevel", &mut self.sea_level), ("amplitude", &mut self.amplitude), ("gravity", &mut self.gravity), ("bodyAltitude", &mut self.body_altitude), ("dropHeight", &mut self.drop_height)] {
            if let Some(value) = object.get(field).and_then(Value::as_f64) { *target = value as f32; }
        }
        if let Some(value) = object.get("seed").and_then(Value::as_u64) { self.seed = value; }
        if let Some(value) = object.get("maxLevel").and_then(Value::as_u64) { self.max_level = value.clamp(1, MAX_LEVEL_LIMIT) as u8; }
        if let Some(bodies) = object.get("bodies").and_then(Value::as_array) {
            self.bodies = bodies.iter().map(|body| {
                (number(body.get("latitude"), 0.0), self.wrap_lon(number(body.get("longitude"), 0.0)))
            }).collect();
        }
        self.radius = self.radius.max(1.0);
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

    pub fn elevation(&self, latitude: f32, longitude: f32) -> f32 {
        let (lat, lon) = (f64::from(latitude).to_radians(), f64::from(longitude).to_radians());
        let unit = [lat.cos() * lon.sin(), lat.sin(), lat.cos() * lon.cos()];
        self.planet.height(unit, f64::from(self.radius)) as f32
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

    fn lod_plugin(&self) -> PlanetLodPlugin {
        PlanetLodPlugin { planet: self.planet.clone(), config: self.quadtree_config(), centre: self.center.as_dvec3(), seed: self.seed as i32 }
    }

    fn water_plugin(&self) -> WaterPlugin {
        WaterPlugin { planet: self.planet.clone(), config: self.quadtree_config(), centre: self.center.as_dvec3() }
    }

    fn terrain_frame(&self) -> TerrainFrame {
        let radius = f64::from(self.radius);
        TerrainFrame { centre: self.center.as_dvec3(), radius, relief_scale: self.planet.relief(radius) }
    }

    pub(crate) fn quadtree_config(&self) -> QuadtreeConfig {
        let radius = f64::from(self.radius);
        let reference = QuadtreeConfig::reference(radius, CAMERA_VIEWPORT_HEIGHT_PX, CAMERA_FOV_Y_RAD);
        let collider = Lattice::default();
        let field = NearField::new(NearFieldOptions { spacing_m: Some(collider.spacing), extent_m: Some(collider.extent), ..Default::default() }).expect("the collider lattice names a positive spacing and extent");
        let spawn = self.lat_lon_to_unit(0.0, 0.0).as_dvec3();
        let ground = f64::from(self.elevation(0.0, 0.0));
        QuadtreeConfig { max_level: self.max_level, ..reference }.with_near_field(field, spawn * (radius + ground + f64::from(self.body_altitude)), ground)
    }

    pub fn report(&self, summary: &GlobeSummary) -> Value {
        json!({
            "name": self.name, "center": vec_json(self.center), "radius": self.radius,
            "seaLevel": self.sea_level, "amplitude": self.amplitude, "seed": self.seed,
            "maxLevel": self.max_level, "gravity": self.gravity,
            "bodyAltitude": self.body_altitude, "dropHeight": self.drop_height,
            "chunks": summary.chunks, "bodies": summary.bodies, "vertices": summary.vertices,
            "minElevation": summary.min_elevation, "maxElevation": summary.max_elevation, "altitude": summary.altitude, "bodyRadius": summary.body_radius, "gravityAtBody": vec_json(summary.gravity_at_body), "focus": vec_json(summary.focus),
            "native": ["Mesh", "StandardMaterial", "Collider::heightfield", "RigidBody", "Transform", "GlobalTransform", "Time", "Component", "Resource", "FixedUpdate"]
        })
    }
}

pub fn vec_json(value: Vec3) -> Value { json!({ "x": value.x, "y": value.y, "z": value.z }) }
pub fn quat_json(value: Quat) -> Value { json!({ "x": value.x, "y": value.y, "z": value.z, "w": value.w }) }
pub fn json_vec3(value: Option<&Value>) -> Vec3 { crate::spec::vec3(value) }
pub fn number(value: Option<&Value>, fallback: f32) -> f32 { value.and_then(Value::as_f64).map(|v| v as f32).unwrap_or(fallback) }

fn to_vec3(components: [f64; 3]) -> Vec3 {
    Vec3::from_array(components.map(|component| component as f32))
}

fn chart_at(planet: &Arc<Planet>, centre: Vec3, radius: f64, relief_scale: Option<f64>, body: Vec3) -> Chart {
    let basis = anchor_basis((body - centre).as_dvec3().normalize_or(DVec3::Y).to_array());
    let height_at = {
        let planet = planet.clone();
        move |direction: [f64; 3]| planet.height(direction, radius)
    };
    let anchor_height = height_at(basis.up);
    let frame = RefCell::new(ChartFrame::new(basis.up, radius, 0.0, relief_scale, &height_at));
    let rotation = Quat::from_mat3(&Mat3::from_cols(to_vec3(basis.east), to_vec3(basis.up), to_vec3(basis.north)));
    let translation = centre + to_vec3(basis.up) * (radius + anchor_height) as f32;
    Chart {
        pose: Transform { translation, rotation, scale: Vec3::ONE },
        body: [0.0, 0.0],
        height: Box::new(move |x, z| frame.borrow_mut().ground_height_local(x, z, |direction| Some(height_at(direction))).unwrap_or(f64::NAN)),
    }
}

fn insert_collider_ground(mut commands: Commands, globe: Res<GlobeWorld>) {
    let (planet, centre, radius, relief_scale) = (globe.planet.clone(), globe.center, f64::from(globe.radius), globe.relief_scale);
    commands.insert_resource(ColliderGround(Arc::new(move |body| chart_at(&planet, centre, radius, relief_scale, body))));
}

const TEXTURES_SUBDIR: &str = "assets/planet/textures";

fn first_with_textures(roots: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    roots.into_iter().map(|root| root.join(TEXTURES_SUBDIR)).find(|dir| dir.is_dir())
}

pub(crate) fn texture_dir() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok();
    let beside_build_tree = executable.iter().flat_map(|path| path.ancestors().skip(2).take(2).map(PathBuf::from));
    first_with_textures(std::iter::once(PathBuf::from(env!("CARGO_MANIFEST_DIR"))).chain(beside_build_tree).chain(std::iter::once(PathBuf::from("."))))
}

fn align_lights_with_focus(
    globe: Res<GlobeWorld>,
    focus: Res<GlobeFocus>,
    mut toward_sun: ResMut<SkySun>,
    mut sun: Query<&mut Transform, (With<Sun>, Without<Fill>)>,
    mut fill: Query<&mut Transform, (With<Fill>, Without<Sun>)>,
) {
    let up = (focus.0 - globe.center).normalize_or(Vec3::Y);
    let local = Quat::from_rotation_arc(Vec3::Y, up);
    toward_sun.0 = local * toward_sun.0;
    for mut transform in &mut sun {
        transform.rotation = local * transform.rotation;
    }
    for mut transform in &mut fill {
        *transform = Transform::from_rotation(local) * Transform::IDENTITY.looking_to(-FILL_FROM, Vec3::Y);
    }
}

fn equip_camera(mut commands: Commands, cameras: Query<Entity, Added<Camera3d>>) {
    for camera in &cameras {
        commands.entity(camera).insert((SkyCamera, reference_view(VIEWER_QUALITY)));
    }
}

fn warn_missing_textures() {
    if texture_dir().is_none() {
        warn!("planet textures not found: terrain keeps the placeholder look");
    }
}

pub fn register(app: &mut App) {
    let globe = app.world().get_resource::<GlobeWorld>().cloned().unwrap_or_default();
    let focus = GlobeFocus(globe.surface_position(0.0, 0.0, globe.body_altitude));
    if app.world().contains_resource::<Assets<Mesh>>() {
        app.add_plugins((
            globe.lod_plugin(),
            TerrainMaterialPlugin,
            globe.water_plugin(),
            AtmosphereLutPlugin,
            PlanetSkyPlugin,
            LightPlugin { quality: VIEWER_QUALITY },
            ToneMapPlugin,
        ))
        .insert_resource(globe.terrain_frame())
        .insert_resource(SkyPlanet { centre: globe.center, radius: globe.radius })
        .add_systems(Startup, warn_missing_textures)
        .add_systems(Update, equip_camera)
        .add_systems(PostUpdate, align_lights_with_focus.before(TransformSystems::Propagate));
    } else {
        app.add_plugins(ColliderStreamPlugin);
    }
    app.init_resource::<GlobeWorld>()
        .init_resource::<GlobeSummary>()
        .insert_resource(focus)
        .add_systems(Startup, (spawn_bodies, insert_collider_ground))
        .add_systems(FixedUpdate, update_focus.before(crate::script::run_scripts))
        .add_systems(FixedUpdate, apply_spherical_gravity.before(crate::script::run_scripts))
        .add_systems(FixedUpdate, read_summary.after(update_focus).before(crate::script::run_scripts))
        .add_systems(FixedPostUpdate, settle_bodies.after(PhysicsSystems::StepSimulation).before(PhysicsSystems::Writeback));
}

fn spawn_bodies(mut commands: Commands, globe: Res<GlobeWorld>) {
    for &(latitude, longitude) in &globe.bodies {
        let spawn_altitude = globe.body_altitude + globe.drop_height;
        let surface = globe.surface_position(latitude, longitude, spawn_altitude);
        let rotation = globe.orientation(latitude, longitude, 0.0);
        commands.spawn((
            GlobeBody { latitude, longitude, altitude: spawn_altitude, speed: 0.0 },
            GlobeOrientation::default(),
            NearBody,
            RigidBody::Dynamic,
            Collider::sphere(0.5),
            GravityScale(0.0),
            LinearVelocity::default(),
            Position(surface),
            Rotation(rotation),
            Transform::from_translation(surface).with_rotation(rotation),
        ));
    }
}

const FRAMING_HEIGHT_M: f32 = 10.0;
const FRAMING_BACK_M: f32 = 14.0;
const FRAMING_DIP: f32 = 0.1;

pub fn frame_globe_camera(globe: Res<GlobeWorld>, focus: Res<GlobeFocus>, mut cameras: Query<(&mut Transform, &mut Projection, &mut Tonemapping), With<Camera3d>>) {
    let up = (focus.0 - globe.center).normalize_or_zero();
    let back = up.any_orthogonal_vector();
    let altitude = (focus.0 - globe.center).length().max(globe.radius);
    let eye = globe.center + up * (altitude + FRAMING_HEIGHT_M) + back * FRAMING_BACK_M;
    let gaze = -back - up * FRAMING_DIP;
    for (mut transform, mut projection, mut tonemapping) in &mut cameras {
        *tonemapping = Tonemapping::None;
        *transform = Transform::from_translation(eye).looking_to(gaze, up);
        if let Projection::Perspective(perspective) = &mut *projection {
            perspective.far = globe.radius * CAMERA_FAR_RADII;
        }
    }
}

pub const CENSUS_ENV: &str = "BEVY_JAYZEE_CENSUS";
pub const CENSUS_OUT_ENV: &str = "BEVY_JAYZEE_CENSUS_OUT";
const CENSUS_FREEZE_S: f64 = 10.0;
const CENSUS_CAPTURE_FRAME: u32 = 1500;

#[derive(Resource)]
pub struct CensusMode {
    out: PathBuf,
}

fn census_frame() -> Duration {
    Duration::from_secs_f64(1.0 / 60.0)
}

pub fn add_census_systems(app: &mut App) {
    if std::env::var(CENSUS_ENV).as_deref() != Ok("1") {
        return;
    }
    let out = std::env::var(CENSUS_OUT_ENV).unwrap_or_else(|_| String::from("census.png"));
    app.insert_resource(CensusMode { out: PathBuf::from(out) });
    app.insert_resource(TimeUpdateStrategy::ManualDuration(census_frame()));
    app.add_systems(Update, (census_freeze, census_capture.run_if(luts_ready)));
}

fn census_freeze(time: Res<Time<Virtual>>, bodies: Query<&Position, With<GlobeBody>>, mut strategy: ResMut<TimeUpdateStrategy>, mut frozen: Local<bool>) {
    if *frozen || time.elapsed_secs_f64() < CENSUS_FREEZE_S {
        return;
    }
    *strategy = TimeUpdateStrategy::ManualDuration(Duration::ZERO);
    *frozen = true;
    eprintln!("census: simulation frozen at {:.3} s of virtual time", time.elapsed_secs_f64());
    for position in &bodies {
        eprintln!("census: body frozen at {:?}", position.0);
    }
}

fn census_capture(mut commands: Commands, census: Res<CensusMode>, lod: Option<Res<PlanetLodStats>>, mut frames: Local<u32>, mut captured: Local<bool>) {
    *frames += 1;
    let streaming = lod.is_some_and(|stats| stats.tasks_in_flight > 0);
    if *frames >= CENSUS_CAPTURE_FRAME && !streaming && !*captured {
        *captured = true;
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(census.out.clone()));
        eprintln!("census: captured frame {} to {}", *frames, census.out.display());
    }
}

fn update_focus(bodies: Query<&Position, With<GlobeBody>>, mut focus: ResMut<GlobeFocus>, census: Option<Res<CensusMode>>, mut latched: Local<bool>) {
    if *latched {
        return;
    }
    if let Some(position) = bodies.iter().next() {
        focus.0 = position.0;
        *latched = census.is_some();
        if *latched {
            eprintln!("census: camera latched at {:?}", focus.0);
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
    focus: Res<GlobeFocus>,
    lod: Option<Res<PlanetLodStats>>,
    bodies: Query<&GlobeBody>,
    positions: Query<&Position, With<GlobeBody>>,
    mut summary: ResMut<GlobeSummary>,
) {
    let lod = lod.as_deref();
    summary.chunks = lod.map_or(0, |stats| stats.live_patches);
    summary.bodies = bodies.iter().count();
    summary.vertices = lod.map_or(0, |stats| stats.vertices);
    summary.min_elevation = lod.map_or(0.0, |stats| stats.min_elevation);
    summary.max_elevation = lod.map_or(0.0, |stats| stats.max_elevation);
    summary.altitude = bodies.iter().next().map(|body| body.altitude).unwrap_or(0.0);
    summary.body_radius = positions.iter().next().map(|p| p.0.distance(globe.center)).unwrap_or(0.0);
    summary.gravity_at_body = positions.iter().next().map(|p| globe.gravity_vector(p.0)).unwrap_or(Vec3::ZERO);
    summary.focus = focus.0;
    summary.seed = globe.seed;
    summary.radius = globe.radius;
}

pub fn dispatch(globe: &mut GlobeWorld, summary: &GlobeSummary, op: &str, payload: &Map<String, Value>) -> Value {
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
        "chunks" => json!({ "count": summary.chunks, "vertices": summary.vertices, "minElevation": summary.min_elevation, "maxElevation": summary.max_elevation }),
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
        ("globeChunks", "Bevy ECS PlanetLodStats from the async quadtree patch streamer"),
        ("globeGravity", "Bevy ECS radial gravity system"),
        ("globeOrientation", "Bevy math Quat/Transform"),
        ("globeNavigation", "Bevy math Vec3 great-circle wraparound"),
    ])
}

#[cfg(test)]
mod tests {
    const BAKED_HEIGHT_TOLERANCE_M: f64 = 1e-9;

    use super::*;

    fn globe() -> GlobeWorld {
        GlobeWorld::default()
    }

    fn spoint_globe(config: Option<&Value>) -> GlobeWorld {
        let mut globe = GlobeWorld { radius: 6360.0, seed: 0x5eed_1234, anchor_dir: [0.0, 1.0, 0.0], ..GlobeWorld::default() };
        if let Some(config) = config {
            globe.from_json(config);
        }
        globe.planet = Arc::new(Planet::new(&globe.planet_config()));
        globe
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
    fn default_radius_is_the_preset_radius() {
        assert_eq!(globe().radius, 63_600.0);
    }

    #[test]
    fn default_planet_is_the_preset() {
        let globe = globe();
        assert_eq!(globe.seed, 1337);
        assert_eq!(globe.max_level, 13);
        assert_eq!(globe.anchor_dir, [-0.641, 0.2558, 0.7237]);
        assert!((globe.planet.relief(f64::from(globe.radius)) - 0.001).abs() < 1e-12);
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
        let globe = spoint_globe(None);
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
        let globe = spoint_globe(None);
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
        let globe = spoint_globe(Some(&case["config"]));
        let worst = worst_elevation_difference(&globe, &fixture, case);
        let ulp = f32::from_bits(globe.radius.to_bits() + 1) - globe.radius;
        assert!(worst <= f64::from(ulp), "carved elevation differs from spoint by up to {worst} m, one f32 ulp at the radius is {ulp} m");
    }

    #[test]
    fn hpf_tex_res_elevation_matches_the_spoint_sampler() {
        let fixture = spoint_sampler_fixture();
        let case = spoint_case(&fixture, "hpfTexRes");
        let globe = spoint_globe(Some(&case["config"]));
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
        let globe = spoint_globe(Some(&case["config"]));
        assert_eq!(globe.hash_version, HashVersion::Float);
        let worst = worst_elevation_difference(&globe, &fixture, case);
        let ulp = f32::from_bits(globe.radius.to_bits() + 1) - globe.radius;
        assert!(worst <= f64::from(ulp), "hashVersion 1 elevation differs from spoint by up to {worst} m, one f32 ulp at the radius is {ulp} m");
    }

    #[test]
    fn relief_scale_elevation_matches_the_spoint_sampler() {
        let fixture = spoint_sampler_fixture();
        let case = spoint_case(&fixture, "reliefScale");
        let globe = spoint_globe(Some(&case["config"]));
        assert_eq!(globe.relief_scale, case["config"]["reliefScale"].as_f64());
        let worst = worst_elevation_difference(&globe, &fixture, case);
        let ulp = f32::from_bits(globe.radius.to_bits() + 1) - globe.radius;
        assert!(worst <= f64::from(ulp), "reliefScale elevation differs from spoint by up to {worst} m, one f32 ulp at the radius is {ulp} m");
    }

    #[test]
    fn baked_height_field_matches_the_spoint_loader() {
        let artifact: Value = serde_json::from_str(include_str!("../testdata/parity/baked-heightfield.json")).unwrap();
        let points: Value = serde_json::from_str(include_str!("../testdata/parity/baked-heightfield-points.json")).unwrap();
        let field = crate::height::baked::BakedHeightField::from_json(&artifact).unwrap();
        let mut worst = 0.0_f64;
        for sample in points["samples"].as_array().unwrap() {
            let (x, z, expected) = (sample[0].as_f64().unwrap(), sample[1].as_f64().unwrap(), sample[2].as_f64().unwrap());
            worst = worst.max((field.height_at_local(x, z) - expected).abs());
        }
        assert!(worst <= BAKED_HEIGHT_TOLERANCE_M, "baked height differs from spoint by up to {worst} m, bound {BAKED_HEIGHT_TOLERANCE_M} m");
    }

    #[test]
    fn tps_game_hf_matches_the_spoint_loader() {
        let bytes = include_bytes!("../testdata/parity/tps-game.hf");
        let points: Value = serde_json::from_str(include_str!("../testdata/parity/tps-game-hf-points.json")).unwrap();
        let field = crate::height::baked::BakedHeightField::from_hf(bytes).unwrap();
        let mut worst = 0.0_f64;
        for sample in points["samples"].as_array().unwrap() {
            let (x, z, expected) = (sample[0].as_f64().unwrap(), sample[1].as_f64().unwrap(), sample[2].as_f64().unwrap());
            worst = worst.max((field.height_at_local(x, z) - expected).abs());
        }
        assert!(worst <= BAKED_HEIGHT_TOLERANCE_M, "tps-game.hf height differs from spoint by up to {worst} m, bound {BAKED_HEIGHT_TOLERANCE_M} m");
    }

    #[test]
    fn tps_game_hf_refusals_match_the_spoint_codec() {
        let bytes = include_bytes!("../testdata/parity/tps-game.hf");
        let points: Value = serde_json::from_str(include_str!("../testdata/parity/tps-game-hf-points.json")).unwrap();
        let mut bad_magic = bytes.to_vec();
        bad_magic[0] ^= 0xff;
        let bad = crate::height::baked::BakedHeightField::from_hf(&bad_magic).err().expect("bad magic accepted");
        assert!(bad.starts_with(points["refusals"]["badMagic"].as_str().unwrap()), "bad magic refusal {bad:?}");
        let truncated = crate::height::baked::BakedHeightField::from_hf(&bytes[..bytes.len() - 1]).err().expect("truncated accepted");
        assert!(truncated.starts_with(points["refusals"]["truncated"].as_str().unwrap()), "truncated refusal {truncated:?}");
    }

    #[test]
    fn hf_encode_matches_the_spoint_encoder() {
        let points: Value = serde_json::from_str(include_str!("../testdata/parity/spoint-hf-encode.json")).unwrap();
        for case in points["cases"].as_array().unwrap() {
            let header = case["header"].as_str().unwrap().as_bytes();
            let bits = case["bits"].as_u64().unwrap() as u32;
            let floats = |key: &str| -> Vec<f32> { case[key].as_array().unwrap().iter().map(|v| v.as_f64().unwrap() as f32).collect() };
            let q: Vec<u32> = case["q"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as u32).collect();
            let bytes = crate::height::baked::encode_hf(header, bits, &floats("sectorMin"), &floats("sectorMax"), &q);
            let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
            assert_eq!(hex, case["bytesHex"].as_str().unwrap(), "encode differs from spoint for bits {bits}");
        }
    }

    #[test]
    fn tps_game_hf_reencodes_to_the_shipped_bytes() {
        let bytes = include_bytes!("../testdata/parity/tps-game.hf");
        let (_, parts) = crate::height::baked::split_hf(bytes).unwrap();
        let again = crate::height::baked::encode_hf(&parts.header, parts.bits, &parts.sector_min, &parts.sector_max, &parts.q);
        assert!(again.as_slice() == bytes.as_slice(), "re-encoded tps-game.hf differs from the shipped bytes ({} against {})", again.len(), bytes.len());
    }

    #[test]
    fn baked_height_field_refuses_a_truncated_artifact_like_spoint() {
        let short: Value = serde_json::from_str(include_str!("../testdata/parity/baked-heightfield-short.json")).unwrap();
        let points: Value = serde_json::from_str(include_str!("../testdata/parity/baked-heightfield-points.json")).unwrap();
        let refusal = crate::height::baked::BakedHeightField::from_json(&short).err().expect("truncated artifact accepted");
        let spoint = points["refusalShort"].as_str().unwrap();
        assert!(spoint.ends_with(&refusal), "refusal {refusal:?} is not spoint's {spoint:?}");
    }

    #[test]
    fn carve_centre_elevation_matches_the_spoint_sampler() {
        let fixture = spoint_sampler_fixture();
        let case = spoint_case(&fixture, "carveCentre");
        let globe = spoint_globe(Some(&case["config"]));
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

    const BODY_SPOTS: [(f32, f32); 2] = [(55.0, 20.0), (-35.0, 140.0)];
    const DROP_HEIGHT_M: f32 = 3.0;
    const LATTICE_TOLERANCE_M: f64 = 0.25;
    const CHART_TOLERANCE_M: f64 = 0.05;
    const SETTLE_FRAMES: usize = 240;
    const COLLIDER_WAIT: std::time::Duration = std::time::Duration::from_secs(120);
    const FRAME_PAUSE: std::time::Duration = std::time::Duration::from_millis(5);

    fn physics_app(globe: GlobeWorld) -> App {
        let mut app = App::new();
        app.add_plugins((bevy::MinimalPlugins, bevy::transform::TransformPlugin, avian3d::prelude::PhysicsPlugins::default()));
        app.insert_resource(Time::<Fixed>::from_hz(60.0)).insert_resource(TimeUpdateStrategy::ManualDuration(census_frame()));
        app.insert_resource(globe);
        register(&mut app);
        app.finish();
        app.cleanup();
        app
    }

    fn viewer_app(globe: GlobeWorld) -> App {
        let mut app = App::new();
        app.add_plugins((bevy::MinimalPlugins, bevy::transform::TransformPlugin, bevy::asset::AssetPlugin::default(), avian3d::prelude::PhysicsPlugins::default()));
        app.init_asset::<Mesh>().init_asset::<Image>().init_asset::<Shader>();
        app.insert_resource(globe);
        register(&mut app);
        app.add_systems(Update, frame_globe_camera);
        app
    }

    #[test]
    fn the_viewer_adds_the_terrain_material_and_a_frame_matching_the_planet() {
        let globe = GlobeWorld { radius: 6360.0, center: Vec3::new(1.0, 2.0, 3.0), relief_scale: Some(0.25), ..GlobeWorld::default() };
        let globe = GlobeWorld { planet: Arc::new(Planet::new(&globe.planet_config())), ..globe };
        let app = viewer_app(globe);
        assert!(app.is_plugin_added::<TerrainMaterialPlugin>());
        let frame = app.world().resource::<TerrainFrame>();
        assert_eq!((frame.radius, frame.relief_scale, frame.centre), (6360.0, 0.25, DVec3::new(1.0, 2.0, 3.0)));
    }

    #[test]
    fn the_default_planet_frame_uses_the_radius_proportional_relief() {
        let mut app = viewer_app(globe());
        app.finish();
        let frame = *app.world().resource::<TerrainFrame>();
        assert_eq!((frame.radius, frame.relief_scale, frame.centre), (63_600.0, 0.001, DVec3::ZERO));
    }

    #[test]
    fn the_globe_camera_renders_without_tonemapping() {
        let mut app = viewer_app(globe());
        app.world_mut().spawn((Camera3d::default(), Tonemapping::TonyMcMapface));
        app.update();
        app.update();
        let tonemapping = app.world_mut().query_filtered::<&Tonemapping, With<Camera3d>>().single(app.world()).unwrap();
        assert_eq!(*tonemapping, Tonemapping::None);
    }

    #[test]
    fn the_viewer_adds_each_sky_water_light_and_tone_map_plugin_once() {
        let app = viewer_app(globe());
        assert!(app.is_plugin_added::<WaterPlugin>());
        assert!(app.is_plugin_added::<AtmosphereLutPlugin>());
        assert!(app.is_plugin_added::<PlanetSkyPlugin>());
        assert!(app.is_plugin_added::<LightPlugin>());
        assert!(app.is_plugin_added::<ToneMapPlugin>());
    }

    #[test]
    fn the_sky_planet_is_the_globe_centre_and_radius() {
        let globe = GlobeWorld { radius: 6360.0, center: Vec3::new(1.0, 2.0, 3.0), ..GlobeWorld::default() };
        let app = viewer_app(globe);
        assert_eq!(*app.world().resource::<SkyPlanet>(), SkyPlanet { centre: Vec3::new(1.0, 2.0, 3.0), radius: 6360.0 });
    }

    #[test]
    fn a_new_globe_camera_gets_the_sky_marker_and_the_reference_tone_map() {
        let mut app = viewer_app(globe());
        app.world_mut().spawn((Camera3d::default(), Tonemapping::TonyMcMapface));
        app.update();
        let mut cameras = app.world_mut().query_filtered::<(&Tonemapping, &crate::planet_sky::tonemap::ToneMap), (With<Camera3d>, With<SkyCamera>)>();
        let (tonemapping, tone_map) = cameras.single(app.world()).unwrap();
        assert_eq!(*tonemapping, Tonemapping::None);
        assert_eq!(*tone_map, crate::planet_sky::tonemap::ToneMap::for_quality(VIEWER_QUALITY));
    }

    #[test]
    fn the_sun_and_the_sky_sun_rise_above_the_local_horizon_of_the_focus() {
        let mut app = viewer_app(globe());
        app.update();
        app.update();
        let up = (app.world().resource::<GlobeFocus>().0 - app.world().resource::<GlobeWorld>().center).normalize();
        let toward_sun = app.world().resource::<SkySun>().0;
        let sun = app.world_mut().query_filtered::<&Transform, With<Sun>>().single(app.world()).unwrap().forward();
        assert!(toward_sun.dot(up) > 0.3, "the sky sun is below the local horizon: {toward_sun:?} against up {up:?}");
        assert!((-sun.as_vec3()).dot(toward_sun) > 0.999, "the light does not point from the sky sun: {sun:?} against {toward_sun:?}");
    }

    #[test]
    fn the_globe_camera_sits_above_the_sea_and_gazes_towards_the_horizon() {
        let mut app = viewer_app(globe());
        let camera = app.world_mut().spawn(Camera3d::default()).id();
        app.update();
        app.update();
        let (centre, radius) = (app.world().resource::<GlobeWorld>().center, app.world().resource::<GlobeWorld>().radius);
        let transform = *app.world().get::<Transform>(camera).unwrap();
        let from_centre = transform.translation - centre;
        assert!(from_centre.length() >= radius + FRAMING_HEIGHT_M - 0.01, "the eye is {} m from the centre, sea level is {radius} m", from_centre.length());
        let dip = transform.forward().dot(from_centre.normalize());
        assert!((-0.2..0.0).contains(&dip), "the gaze dips by {dip}");
    }

    #[test]
    fn exactly_one_shadow_casting_sun_exists_once_the_plugins_build() {
        let mut app = viewer_app(globe());
        app.update();
        let mut lights = app.world_mut().query::<&DirectionalLight>();
        let shadowed = lights.iter(app.world()).filter(|light| light.shadow_maps_enabled).count();
        assert_eq!(shadowed, 1);
    }

    #[test]
    fn textures_are_found_under_the_first_root_that_has_them() {
        let scratch = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/globe-texture-roots");
        let (empty, stocked, other) = (scratch.join("empty"), scratch.join("stocked"), scratch.join("other"));
        std::fs::create_dir_all(&empty).unwrap();
        std::fs::create_dir_all(stocked.join(TEXTURES_SUBDIR)).unwrap();
        std::fs::create_dir_all(other.join(TEXTURES_SUBDIR)).unwrap();
        assert_eq!(first_with_textures([stocked.clone(), other]), Some(stocked.join(TEXTURES_SUBDIR)));
        assert_eq!(first_with_textures([empty.clone(), stocked.clone()]), Some(stocked.join(TEXTURES_SUBDIR)));
        assert_eq!(first_with_textures([empty]), None);
        assert!(texture_dir().is_some());
    }

    #[test]
    fn chart_points_lie_on_the_planet_surface_wherever_the_chart_is_anchored() {
        let globe = globe();
        let radius = f64::from(globe.radius);
        for (latitude, longitude) in BODY_SPOTS {
            let body = globe.surface_position(latitude, longitude, 0.0);
            let chart = chart_at(&globe.planet, globe.center, radius, globe.relief_scale, body);
            let pose = chart.pose;
            assert!((pose.rotation * Vec3::Y).dot(globe.lat_lon_to_unit(latitude, longitude)) > 0.9999);
            for (x, z) in [(0.0, 0.0), (40.0, -25.0), (-90.0, 60.0)] {
                let local = Vec3::new(x as f32, (chart.height)(x, z) as f32, z as f32);
                let world = pose.transform_point(local) - globe.center;
                let direction = world.as_dvec3().normalize().to_array();
                let surface = globe.planet.height(direction, radius);
                let error = f64::from(world.length()) - radius - surface;
                assert!(error.abs() < CHART_TOLERANCE_M, "chart point ({x}, {z}) at {latitude}, {longitude} is {error} m off the surface");
            }
        }
    }

    #[test]
    fn bodies_dropped_at_two_latitudes_settle_on_the_planet_surface() {
        let mut app = physics_app(GlobeWorld { bodies: BODY_SPOTS.to_vec(), drop_height: DROP_HEIGHT_M, ..GlobeWorld::default() });
        let started = std::time::Instant::now();
        while app.world_mut().query::<&crate::planet_lod::collider::GroundField>().iter(app.world()).count() < BODY_SPOTS.len() {
            assert!(started.elapsed() < COLLIDER_WAIT, "no heightfield collider appeared within {COLLIDER_WAIT:?}");
            app.update();
            std::thread::sleep(FRAME_PAUSE);
        }
        for _ in 0..SETTLE_FRAMES {
            app.update();
        }
        let globe = app.world().resource::<GlobeWorld>().clone();
        let radius = f64::from(globe.radius);
        let mut settled = app.world_mut().query::<(&Position, &crate::planet_lod::collider::GroundField, &GlobeBody)>();
        for (position, field, body) in settled.iter(app.world()) {
            let (latitude, longitude) = (body.latitude, body.longitude);
            let offset = position.0 - globe.center;
            let surface = globe.planet.height(offset.as_dvec3().normalize().to_array(), radius);
            let resting_height = f64::from(offset.length() - globe.body_altitude) - radius;
            assert!((resting_height - surface).abs() < LATTICE_TOLERANCE_M, "body at {latitude}, {longitude} rests {resting_height} m above the radius, the planet surface is {surface} m");
            assert!((field.pose.rotation * Vec3::Y).dot(offset.normalize()) > 0.9999, "the heightfield at {latitude}, {longitude} is not oriented along the local up");
        }
        let colliders = app.world_mut().query_filtered::<(), (With<Collider>, Without<GlobeBody>)>().iter(app.world()).count();
        assert_eq!(colliders, BODY_SPOTS.len());
    }
}
