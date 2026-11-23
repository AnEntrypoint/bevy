use super::guard::note_build;
use crate::js_math::round_half_up;
use avian3d::prelude::{Collider, RigidBody};
use bevy::prelude::*;
use bevy::tasks::{futures::check_ready, AsyncComputeTaskPool, Task};
use std::sync::Arc;

pub const PRESET_EXTENT_M: f64 = 256.0;
pub const PRESET_RESOLUTION_M: f64 = 2.0;
pub const KEEP_FRACTION: f64 = 0.4;
const NON_FINITE_HEIGHT_M: f64 = -1000.0;
const HEIGHT_SNAP_PER_M: f64 = 1000.0;
pub const COLLIDER_BUILDS_PER_FRAME: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lattice {
    pub n: usize,
    pub spacing: f64,
    pub extent: f64,
}

impl Lattice {
    pub fn of(extent: f64, resolution: f64) -> Self {
        let rounded = round_half_up(extent / resolution).max(2.0);
        let n = if rounded % 2.0 == 0.0 { rounded } else { rounded + 1.0 };
        Self { n: n as usize, spacing: extent / (n - 1.0), extent }
    }

    pub fn keep_radius(&self) -> f64 {
        self.extent * KEEP_FRACTION
    }

    fn corner(&self, centre: f64) -> f64 {
        round_half_up((centre - self.extent / 2.0) / self.spacing) * self.spacing
    }
}

impl Default for Lattice {
    fn default() -> Self {
        Self::of(PRESET_EXTENT_M, PRESET_RESOLUTION_M)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub n: usize,
    pub spacing: f64,
    pub origin: [f64; 2],
    pub heights: Vec<f32>,
}

impl Field {
    pub fn centre(&self, lattice: &Lattice) -> [f64; 2] {
        self.origin.map(|c| c + lattice.extent / 2.0)
    }
}

fn snapped(height: f64) -> f32 {
    let finite = if height.is_finite() { height } else { NON_FINITE_HEIGHT_M };
    (round_half_up(finite * HEIGHT_SNAP_PER_M) / HEIGHT_SNAP_PER_M + 0.0) as f32
}

pub fn sample_field(lattice: &Lattice, body: [f64; 2], height: &dyn Fn(f64, f64) -> f64) -> Field {
    let origin = body.map(|c| lattice.corner(c));
    let heights = (0..lattice.n * lattice.n)
        .map(|k| snapped(height(origin[0] + (k % lattice.n) as f64 * lattice.spacing, origin[1] + (k / lattice.n) as f64 * lattice.spacing)))
        .collect();
    Field { n: lattice.n, spacing: lattice.spacing, origin, heights }
}

pub struct Chart {
    pub pose: Transform,
    pub body: [f64; 2],
    pub height: Box<dyn Fn(f64, f64) -> f64>,
}

pub struct BuiltField {
    pub collider: Collider,
    pub centre: [f64; 2],
    pub pose: Transform,
}

pub fn build_collider(lattice: &Lattice, chart: Chart) -> BuiltField {
    note_build();
    let field = sample_field(lattice, chart.body, chart.height.as_ref());
    let columns_along_x: Vec<Vec<f32>> = (0..field.n).map(|x| (0..field.n).map(|z| field.heights[z * field.n + x]).collect()).collect();
    let collider = Collider::heightfield(columns_along_x, Vec3::new(lattice.extent as f32, 1.0, lattice.extent as f32));
    BuiltField { collider, centre: field.centre(lattice), pose: chart.pose }
}

#[derive(Component)]
pub struct NearBody;

#[derive(Resource, Clone)]
pub struct ColliderGround(pub Arc<dyn Fn(Vec3) -> Chart + Send + Sync>);

#[derive(Resource, Default)]
pub struct ColliderStats {
    pub builds_started: usize,
}

#[derive(Resource, Default)]
pub struct ColliderLattice(pub Lattice);

#[derive(Component)]
struct PendingBuild(Task<BuiltField>);

#[derive(Component)]
pub struct GroundField {
    pub centre: [f64; 2],
    pub pose: Transform,
    pub collider_entity: Entity,
}

#[derive(Component)]
pub struct GroundFieldOf(pub Entity);

fn chart_xz(pose: &Transform, world: Vec3) -> [f64; 2] {
    let local = pose.compute_affine().inverse().transform_point3(world);
    [local.x as f64, local.z as f64]
}

fn chebyshev(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).abs().max((a[1] - b[1]).abs())
}

fn start_builds(
    mut commands: Commands,
    ground: Res<ColliderGround>,
    lattice: Res<ColliderLattice>,
    mut stats: ResMut<ColliderStats>,
    bodies: Query<(Entity, &Transform, Option<&GroundField>), (With<NearBody>, Without<PendingBuild>)>,
) {
    crate::frame_clock::timed(9, || {
    let outside_keep_radius = |(_, transform, field): &(Entity, &Transform, Option<&GroundField>)| {
        let body = transform.translation;
        field.is_none_or(|f| chebyshev(chart_xz(&f.pose, body), f.centre) > lattice.0.keep_radius())
    };
    for (entity, transform, _) in bodies.iter().filter(outside_keep_radius).take(COLLIDER_BUILDS_PER_FRAME) {
        let (lattice, ground, body) = (lattice.0, ground.clone(), transform.translation);
        let task = AsyncComputeTaskPool::get().spawn(async move { build_collider(&lattice, (ground.0)(body)) });
        commands.entity(entity).insert(PendingBuild(task));
        stats.builds_started += 1;
    }
});
}

fn finish_builds(mut commands: Commands, mut pending: Query<(Entity, &mut PendingBuild, Option<&GroundField>)>) {
    crate::frame_clock::timed(10, || {
    let ready = pending.iter_mut().filter_map(|(entity, mut build, field)| check_ready(&mut build.0).map(|built| (entity, built, field)));
    for (entity, built, field) in ready.take(COLLIDER_BUILDS_PER_FRAME) {
        let transform = built.pose * Transform::from_xyz(built.centre[0] as f32, 0.0, built.centre[1] as f32);
        let collider_entity = match field {
            Some(f) => {
                commands.entity(f.collider_entity).insert((built.collider, transform));
                f.collider_entity
            }
            None => commands.spawn((RigidBody::Static, built.collider, transform, GroundFieldOf(entity))).id(),
        };
        commands.entity(entity).remove::<PendingBuild>().insert(GroundField { centre: built.centre, pose: built.pose, collider_entity });
    }
});
}

fn drop_orphans(mut commands: Commands, fields: Query<(Entity, &GroundFieldOf)>, bodies: Query<(), With<NearBody>>) {
    for (entity, owner) in &fields {
        if !bodies.contains(owner.0) {
            commands.entity(entity).despawn();
        }
    }
}

pub struct ColliderStreamPlugin;

impl Plugin for ColliderStreamPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ColliderStats>()
            .init_resource::<ColliderLattice>()
            .add_systems(Update, (finish_builds, start_builds.run_if(resource_exists::<ColliderGround>), drop_orphans).chain());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::height::planet_frame::ChartFrame;
    use crate::planet_lod::guard::{builds_on_main_thread, mark_main_thread};
    use crate::testkit::{cases, close, parse};
    use serde_json::Value;
    use bevy::ecs::schedule::SingleThreadedExecutor;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    const HEIGHT_TOLERANCE_MM: f64 = 2.0;
    const RAY_HEIGHT_M: f32 = 100.0;
    const OFF_VERTEX_M: f32 = 0.01;
    const FIELD_WAIT: Duration = Duration::from_secs(30);

    thread_local! {
        static FRAMES: RefCell<HashMap<u64, ChartFrame>> = RefCell::new(HashMap::new());
    }

    fn smooth(d: [f64; 3]) -> f64 {
        40.0 + 25.0 * d[0] - 15.0 * d[1] + 10.0 * d[2] * d[2]
    }

    fn fixture() -> Value {
        parse(include_str!("../../testdata/parity/spoint-planet-collider.json"))
    }

    fn vec3(v: &Value) -> [f64; 3] {
        [0, 1, 2].map(|i| v[i].as_f64().unwrap())
    }

    fn ground_of(fixture: &Value, radius: f64) -> impl Fn(f64, f64) -> f64 + Send + Sync + 'static {
        let anchor = vec3(&fixture["anchorDir"]);
        let (offset_y, relief_scale) = (fixture["offsetY"].as_f64().unwrap(), fixture["reliefScale"].as_f64());
        move |x, z| {
            FRAMES.with(|frames| {
                let mut frames = frames.borrow_mut();
                let frame = frames.entry(radius.to_bits()).or_insert_with(|| ChartFrame::new(anchor, radius, offset_y, relief_scale, smooth));
                frame.ground_height_local(x, z, |d| Some(smooth(d))).unwrap_or(f64::NAN)
            })
        }
    }

    fn pair(v: &Value) -> [f64; 2] {
        [v[0].as_f64().unwrap(), v[1].as_f64().unwrap()]
    }

    #[test]
    fn lattice_origin_spacing_n_and_heights_match_the_spoint_collider() {
        let fixture = fixture();
        let lattice = Lattice::default();
        for case in cases(&fixture, "cases") {
            let radius = case["radius"].as_f64().unwrap();
            let body = pair(&case["body"]);
            let label = format!("radius {radius} body {body:?}");
            let ground = ground_of(&fixture, radius);
            let field = sample_field(&lattice, body, &ground);
            assert_eq!(field.n, case["n"].as_u64().unwrap() as usize, "{label} n");
            close(field.spacing, case["spacing"].as_f64().unwrap(), 1e-12, &format!("{label} spacing"));
            let origin = pair(&case["origin"]);
            close(field.origin[0], origin[0], 1e-9, &format!("{label} origin x"));
            close(field.origin[1], origin[1], 1e-9, &format!("{label} origin z"));
            let want = case["heightsMm"].as_array().unwrap();
            assert_eq!(field.heights.len(), want.len(), "{label} heights length");
            for (k, (got, want)) in field.heights.iter().zip(want).enumerate() {
                close(*got as f64 * 1000.0, want.as_f64().unwrap(), HEIGHT_TOLERANCE_MM, &format!("{label} height {k}"));
            }
        }
    }

    fn flat_ground(ground: impl Fn(f64, f64) -> f64 + Send + Sync + 'static) -> ColliderGround {
        let ground = Arc::new(ground);
        ColliderGround(Arc::new(move |body: Vec3| {
            let ground = ground.clone();
            Chart { pose: Transform::IDENTITY, body: [body.x as f64, body.z as f64], height: Box::new(move |x, z| ground(x, z)) }
        }))
    }

    fn app_with_body(ground: impl Fn(f64, f64) -> f64 + Send + Sync + 'static, body: Vec3) -> (App, Entity) {
        let mut app = App::new();
        app.edit_schedule(Update, |schedule| {
            schedule.set_executor(SingleThreadedExecutor::new());
        });
        app.add_plugins((MinimalPlugins, ColliderStreamPlugin)).insert_resource(flat_ground(ground));
        let entity = app.world_mut().spawn((NearBody, Transform::from_translation(body))).id();
        (app, entity)
    }

    fn tick_until_field(app: &mut App, entity: Entity) {
        let started = Instant::now();
        while app.world().entity(entity).contains::<PendingBuild>() || !app.world().entity(entity).contains::<GroundField>() {
            assert!(started.elapsed() < FIELD_WAIT, "no collider appeared within {FIELD_WAIT:?}");
            app.update();
            std::thread::sleep(Duration::from_millis(5));
        }
        app.update();
    }

    fn height_under(app: &App, entity: Entity, x: f32, z: f32) -> f32 {
        let field = app.world().get::<GroundField>(entity).unwrap();
        let transform = app.world().get::<Transform>(field.collider_entity).unwrap();
        let collider = app.world().get::<Collider>(field.collider_entity).unwrap();
        let local = Vec3::new(x - transform.translation.x + OFF_VERTEX_M, RAY_HEIGHT_M, z - transform.translation.z + OFF_VERTEX_M);
        let distance = collider.cast_ray(Vec3::ZERO, Quat::IDENTITY, local, Vec3::NEG_Y, 2.0 * RAY_HEIGHT_M, true).expect("ray hits the field").0;
        RAY_HEIGHT_M - distance
    }

    #[test]
    fn collider_streams_in_off_the_main_thread_with_the_reference_heights() {
        let fixture = fixture();
        let case = cases(&fixture, "cases")[0].clone();
        let radius = case["radius"].as_f64().unwrap();
        let ground = ground_of(&fixture, radius);
        mark_main_thread();
        let before = builds_on_main_thread();
        let (found, builds_started) = {
            let (mut app, entity) = app_with_body(ground, Vec3::new(0.0, 0.0, 0.0));
            tick_until_field(&mut app, entity);
            let n = case["n"].as_u64().unwrap() as usize;
            let want = case["heightsMm"].as_array().unwrap();
            let origin = pair(&case["origin"]);
            let spacing = case["spacing"].as_f64().unwrap();
            let found: Vec<(f64, f64)> = [(63, 63), (10, 20), (100, 7)]
                .into_iter()
                .map(|(i, j)| {
                    let (x, z) = (origin[0] + i as f64 * spacing, origin[1] + j as f64 * spacing);
                    (height_under(&app, entity, x as f32, z as f32) as f64, want[j * n + i].as_f64().unwrap() / 1000.0)
                })
                .collect();
            (found, app.world().resource::<ColliderStats>().builds_started)
        };
        assert_eq!(builds_started, 1);
        for (got, want) in found {
            close(got, want, 0.01, "height under the body");
        }
        assert_eq!(builds_on_main_thread() - before, 0, "collider builds on the main thread");
    }

    const CROWD_SPACING_M: f32 = 1000.0;
    const FRAMES_BEFORE_OPENING: usize = 3;
    const GATE_LIMIT: Duration = Duration::from_secs(2);

    fn pending_tasks_finished(app: &mut App) -> bool {
        app.world_mut().query::<&PendingBuild>().iter(app.world()).all(|build| build.0.is_finished())
    }

    fn fields(app: &mut App) -> usize {
        app.world_mut().query::<&GroundField>().iter(app.world()).count()
    }

    fn started(app: &App) -> usize {
        app.world().resource::<ColliderStats>().builds_started
    }

    #[test]
    fn a_crowd_of_bodies_starts_and_finishes_at_most_the_cap_per_frame() {
        let crowd = COLLIDER_BUILDS_PER_FRAME * (FRAMES_BEFORE_OPENING + 1);
        let open = Arc::new(AtomicBool::new(false));
        let gate = open.clone();
        let gate_expires = Instant::now() + GATE_LIMIT;
        let (mut app, _) = app_with_body(
            move |_, _| {
                while !gate.load(Ordering::SeqCst) && Instant::now() < gate_expires {
                    std::thread::sleep(Duration::from_millis(1));
                }
                0.0
            },
            Vec3::ZERO,
        );
        for i in 1..crowd {
            app.world_mut().spawn((NearBody, Transform::from_xyz(i as f32 * CROWD_SPACING_M, 0.0, 0.0)));
        }
        for frame in 1..=FRAMES_BEFORE_OPENING {
            app.update();
            assert_eq!(started(&app), frame * COLLIDER_BUILDS_PER_FRAME, "builds started by frame {frame}");
        }
        open.store(true, Ordering::SeqCst);
        let begun = Instant::now();
        let mut most_finished = 0;
        while fields(&mut app) < crowd {
            assert!(begun.elapsed() < FIELD_WAIT, "the crowd never got its colliders");
            while !pending_tasks_finished(&mut app) {
                std::thread::sleep(Duration::from_millis(1));
            }
            let (fields_before, started_before) = (fields(&mut app), started(&app));
            app.update();
            let finished = fields(&mut app) - fields_before;
            assert!(finished <= COLLIDER_BUILDS_PER_FRAME, "{finished} builds finished in one frame");
            assert!(started(&app) - started_before <= COLLIDER_BUILDS_PER_FRAME);
            most_finished = most_finished.max(finished);
        }
        assert_eq!(most_finished, COLLIDER_BUILDS_PER_FRAME, "the finish cap was never exercised");
        assert_eq!(started(&app), crowd);
    }

    #[test]
    fn collider_rebuilds_only_past_the_keep_radius() {
        let fixture = fixture();
        let radius = cases(&fixture, "cases")[0]["radius"].as_f64().unwrap();
        let ground = ground_of(&fixture, radius);
        let keep = Lattice::default().keep_radius() as f32;
        let (mut app, entity) = app_with_body(ground, Vec3::ZERO);
        tick_until_field(&mut app, entity);
        let first_centre = app.world().get::<GroundField>(entity).unwrap().centre;
        app.world_mut().get_mut::<Transform>(entity).unwrap().translation.x = keep * 0.9;
        for _ in 0..20 {
            app.update();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(app.world().resource::<ColliderStats>().builds_started, 1, "rebuilt inside the keep radius");
        app.world_mut().get_mut::<Transform>(entity).unwrap().translation.x = keep * 1.5;
        let started = Instant::now();
        while app.world().resource::<ColliderStats>().builds_started < 2 || app.world().entity(entity).contains::<PendingBuild>() {
            assert!(started.elapsed() < FIELD_WAIT, "no rebuild past the keep radius");
            app.update();
            std::thread::sleep(Duration::from_millis(5));
        }
        app.update();
        let second_centre = app.world().get::<GroundField>(entity).unwrap().centre;
        assert_ne!(first_centre, second_centre);
        assert_eq!(app.world().resource::<ColliderStats>().builds_started, 2);
    }
}
