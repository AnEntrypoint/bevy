use crate::bench::percentile;
use crate::globe::{self, GlobeWorld};
use crate::height::Planet;
use crate::offscreen::{Frame, add_target, headless_app, pump, spawn_view};
use crate::planet_lod::collider::{ColliderStats, NearBody};
use crate::planet_lod::guard;
use crate::planet_lod::stream::PlanetLodStats;
use crate::planet_material::terrain::{TerrainMaterial, TerrainMaterialHandle};
use crate::planet_sky::lut::AtmosphereLuts;
use crate::planet_veg::gpu::store::VegFrame;
use crate::planet_veg::stream::VegStreamStats;
use crate::planet_veg::trees_render::TreesWitness;
use crate::render_stats::{RenderStatsPlugin, SharedFrameStats};
use bevy::ecs::schedule::SingleThreadedExecutor;
use bevy::math::DVec3;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::tasks::AsyncComputeTaskPool;
use bevy::time::TimeUpdateStrategy;
use serde_json::{Value, json};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

const STEP_S: f64 = 1.0 / 60.0;
const CENSUS_ANCHOR: DVec3 = DVec3::new(-40815.595807093756, 16288.03339696503, 46081.50808984984);
const FLIGHT_ALTITUDES_M: [f64; 3] = [100_000.0, 20_000.0, 5_000.0];
const LEG_FRAMES: usize = 600;
const LANDED_HOLD_FRAMES: usize = 300;
const STRAIGHT_DOWN_DEG: f64 = 90.0;
const CENSUS_PITCH_DOWN_DEG: f64 = 6.0;
const GROUND_SPEED_M_S: f64 = 10.0;
const GROUND_FRAMES: usize = 4800;
const TELEPORT_CLEARANCE_M: f64 = 2.0;
pub const IDLE_FRAMES: usize = 600;
const SETTLE_LIMIT: usize = 4000;
const SETTLED_STREAK: usize = 4;
const BUDGET_HEADROOM_MS: f64 = 1.0;
const BUDGET_IDLE_MULTIPLE: f64 = 2.0;
const RESULT_FILE: &str = "target/witness/bench-planet.json";

#[derive(Clone, Copy, Debug, PartialEq)]
enum ScriptedPath {
    Flight,
    Ground,
}

impl ScriptedPath {
    const ALL: [Self; 2] = [Self::Flight, Self::Ground];

    fn name(self) -> &'static str {
        match self {
            Self::Flight => "flight",
            Self::Ground => "ground",
        }
    }

    fn frames(self) -> usize {
        match self {
            Self::Flight => FLIGHT_ALTITUDES_M.len() * LEG_FRAMES + LANDED_HOLD_FRAMES,
            Self::Ground => GROUND_FRAMES,
        }
    }

    fn pose(self, radius: f64, ground: impl Fn(DVec3) -> f64, frame: usize) -> Frame {
        match self {
            Self::Flight => flight_pose(radius, ground, frame),
            Self::Ground => on_ground(radius, ground, GROUND_SPEED_M_S * STEP_S * (frame + 1) as f64),
        }
    }
}

fn mix(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn on_ground(radius: f64, ground: impl Fn(DVec3) -> f64, metres_north: f64) -> Frame {
    let start = CENSUS_ANCHOR.normalize();
    let north = Frame::above(start, radius, 0.0, 0.0).forward;
    let angle = metres_north / CENSUS_ANCHOR.length();
    let dir = start * angle.cos() + north * angle.sin();
    let eye = (CENSUS_ANCHOR.length() - radius - ground(start)).max(TELEPORT_CLEARANCE_M);
    Frame::above(dir, radius + ground(dir), eye, CENSUS_PITCH_DOWN_DEG)
}

fn flight_pose(radius: f64, ground: impl Fn(DVec3) -> f64, frame: usize) -> Frame {
    let landed = on_ground(radius, ground, 0.0);
    let leg = frame / LEG_FRAMES;
    let Some(&from) = FLIGHT_ALTITUDES_M.get(leg) else {
        return landed;
    };
    let to = FLIGHT_ALTITUDES_M.get(leg + 1).copied().unwrap_or(landed.position.length() - radius);
    let t = (frame % LEG_FRAMES) as f64 / LEG_FRAMES as f64;
    let pitch = if leg + 1 == FLIGHT_ALTITUDES_M.len() { mix(STRAIGHT_DOWN_DEG, CENSUS_PITCH_DOWN_DEG, t) } else { STRAIGHT_DOWN_DEG };
    Frame::above(CENSUS_ANCHOR.normalize(), radius, mix(from.ln(), to.ln(), t).exp(), pitch)
}

fn budget_ms(idle_p99_ms: f64) -> f64 {
    (idle_p99_ms + BUDGET_HEADROOM_MS).min(BUDGET_IDLE_MULTIPLE * idle_p99_ms)
}

pub const LOAD_LIMIT: f64 = 4.0;
const INCONCLUSIVE_EXIT: i32 = 77;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Verdict {
    Pass,
    Fail,
    Inconclusive(f64),
}

impl Verdict {
    pub fn exit_code(self) -> i32 {
        match self {
            Verdict::Pass => 0,
            Verdict::Fail => 1,
            Verdict::Inconclusive(_) => INCONCLUSIVE_EXIT,
        }
    }
}

fn one_minute_load(loadavg: &str) -> f64 {
    loadavg.split_whitespace().next().and_then(|load| load.parse().ok()).unwrap_or(0.0)
}

fn frames_over(samples_ms: &[f64], budget: f64) -> usize {
    samples_ms.iter().filter(|ms| **ms > budget).count()
}

struct Spread {
    p50: f64,
    p99: f64,
    max: f64,
}

fn spread(samples_ms: &[f64]) -> Spread {
    let mut sorted = samples_ms.to_vec();
    sorted.sort_by(f64::total_cmp);
    Spread { p50: percentile(&sorted, 0.50), p99: percentile(&sorted, 0.99), max: sorted[sorted.len() - 1] }
}

#[derive(Clone, Copy)]
struct FrameTimes {
    main_ms: f64,
    render_ms: f64,
    spawned_patches: usize,
    tasks_in_flight: usize,
}

struct PathRun {
    path: ScriptedPath,
    settle_frames: Option<usize>,
    frames: Vec<FrameTimes>,
    idle_settle_frames: Option<usize>,
    idle_ms: Vec<f64>,
}

impl PathRun {
    fn main_ms(&self) -> Vec<f64> {
        self.frames.iter().map(|frame| frame.main_ms).collect()
    }

    fn budget_ms(&self) -> f64 {
        budget_ms(spread(&self.idle_ms).p99)
    }

    fn over_budget(&self) -> usize {
        frames_over(&self.main_ms(), self.budget_ms())
    }

    fn over_budget_frames(&self) -> Vec<Value> {
        let budget = self.budget_ms();
        let over = self.frames.iter().enumerate().filter(|(_, frame)| frame.main_ms > budget);
        over.map(|(index, frame)| json!({ "frame": index, "mainMs": frame.main_ms, "spawnedPatches": frame.spawned_patches, "tasksInFlight": frame.tasks_in_flight })).collect()
    }

    fn passes(&self) -> bool {
        self.settle_frames.is_some() && self.idle_settle_frames.is_some() && self.over_budget() == 0
    }

    fn json(&self) -> Value {
        let (main, idle) = (spread(&self.main_ms()), spread(&self.idle_ms));
        let render: Vec<f64> = self.frames.iter().map(|frame| frame.render_ms).collect();
        json!({
            "frames": self.frames.len(), "settleFrames": self.settle_frames,
            "p50Ms": main.p50, "p99Ms": main.p99, "maxMs": main.max, "renderP99Ms": spread(&render).p99,
            "idle": { "frames": self.idle_ms.len(), "settleFrames": self.idle_settle_frames, "p50Ms": idle.p50, "p99Ms": idle.p99, "maxMs": idle.max },
            "budgetMs": self.budget_ms(), "overBudget": self.over_budget(), "overBudgetFrames": self.over_budget_frames(), "pass": self.passes()
        })
    }
}

struct Report {
    runs: Vec<PathRun>,
    main_thread_builds: usize,
    collider_builds: usize,
    max_patches_per_frame: usize,
    veg_resident_chunks: [usize; 3],
    threads: usize,
    load_before: String,
    load_after: String,
}

impl Report {
    fn passes(&self) -> bool {
        self.main_thread_builds == 0 && self.runs.iter().all(PathRun::passes)
    }

    fn verdict(&self) -> Verdict {
        let load = one_minute_load(&self.load_before).max(one_minute_load(&self.load_after));
        match (load > LOAD_LIMIT, self.passes()) {
            (true, _) => Verdict::Inconclusive(load),
            (false, true) => Verdict::Pass,
            (false, false) => Verdict::Fail,
        }
    }

    fn json(&self) -> Value {
        let mut value = json!({
            "kind": "bench-planet", "stepS": STEP_S, "threads": self.threads, "loadBefore": self.load_before, "loadAfter": self.load_after,
            "mainThreadBuilds": self.main_thread_builds, "colliderBuildsStarted": self.collider_builds, "maxPatchesPerFrame": self.max_patches_per_frame, "vegResidentChunks": self.veg_resident_chunks,
            "pass": self.passes(), "loadLimit": LOAD_LIMIT, "verdict": format!("{:?}", self.verdict())
        });
        for run in &self.runs {
            value[run.path.name()] = run.json();
        }
        value
    }
}

struct Lengths {
    path_frames: usize,
    idle_frames: usize,
    settle_frames: usize,
}

fn settled(world: &World) -> bool {
    let lod = world.resource::<PlanetLodStats>();
    let veg = world.resource::<VegStreamStats>();
    let uploads = world.resource::<VegFrame>();
    let trees = world.resource::<TreesWitness>();
    let layers = world.resource::<Assets<TerrainMaterial>>().get(&world.resource::<TerrainMaterialHandle>().0).is_some_and(|material| material.extension.params.surf_ready == 1.0);
    lod.live_patches > 0
        && lod.tasks_in_flight == 0
        && lod.waiting_on_climate == 0
        && veg.tasks_in_flight == 0
        && veg.queued_chunks == [0; 3]
        && uploads.writes.iter().all(Vec::is_empty)
        && world.contains_resource::<AtmosphereLuts>()
        && trees.loaded.load(Ordering::SeqCst)
        && trees.baked.load(Ordering::SeqCst)
        && layers
}

struct Bench {
    app: App,
    camera: Entity,
    body: Entity,
    planet: Arc<Planet>,
    radius: f64,
}

impl Bench {
    fn new() -> Self {
        let globe = GlobeWorld { bodies: Vec::new(), ..default() };
        let (planet, radius) = (globe.planet.clone(), f64::from(globe.radius));
        let mut app = headless_app();
        app.add_plugins(RenderStatsPlugin).insert_resource(globe).insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(STEP_S)));
        globe::register(&mut app);
        app.edit_schedule(Update, |update| {
            update.set_executor(SingleThreadedExecutor::new());
        });
        app.finish();
        app.cleanup();
        let target = add_target(&mut app, TextureFormat::Rgba8UnormSrgb);
        let start = ScriptedPath::Flight.pose(radius, |dir| planet.height(dir.to_array(), radius), 0);
        spawn_view(&mut app, &target, Color::BLACK, &start, 1.0);
        let camera = app.world_mut().query_filtered::<Entity, With<Camera3d>>().single(app.world()).expect("one bench camera");
        let body = app.world_mut().spawn((NearBody, Transform::default())).id();
        Self { app, camera, body, planet, radius }
    }

    fn pose(&self, path: ScriptedPath, frame: usize) -> Frame {
        path.pose(self.radius, |dir| self.planet.height(dir.to_array(), self.radius), frame)
    }

    fn step(&mut self, pose: &Frame) -> FrameTimes {
        let translation = pose.position.as_vec3();
        let world = self.app.world_mut();
        *world.get_mut::<Transform>(self.camera).expect("the bench camera") = Transform::from_translation(translation).looking_to(pose.forward.as_vec3(), pose.up.as_vec3());
        *world.get_mut::<Transform>(self.body).expect("the near body") = Transform::from_translation(translation);
        pump(&mut self.app);
        let stats = self.app.world().resource::<SharedFrameStats>().snapshot();
        let span = |ms: Option<f64>| ms.expect("the render stats time every frame");
        let lod = self.app.world().resource::<PlanetLodStats>();
        FrameTimes { main_ms: span(stats.sim_ms) + span(stats.extract_ms), render_ms: span(stats.render_ms), spawned_patches: lod.spawned_this_frame, tasks_in_flight: lod.tasks_in_flight }
    }

    fn settle(&mut self, pose: &Frame, limit: usize) -> Option<usize> {
        let mut streak = 0;
        (1..=limit).find(|_| {
            self.step(pose);
            streak = if settled(self.app.world()) { streak + 1 } else { 0 };
            streak == SETTLED_STREAK
        })
    }

    fn run_path(&mut self, path: ScriptedPath, lengths: &Lengths) -> PathRun {
        let count = path.frames().min(lengths.path_frames).max(1);
        let start = self.pose(path, 0);
        let settle_frames = self.settle(&start, lengths.settle_frames);
        let frames = (0..count).map(|frame| {
            let pose = self.pose(path, frame);
            self.step(&pose)
        }).collect();
        let end = self.pose(path, count - 1);
        let idle_settle_frames = self.settle(&end, lengths.settle_frames);
        let idle_ms = (0..lengths.idle_frames.max(1)).map(|_| self.step(&end).main_ms).collect();
        PathRun { path, settle_frames, frames, idle_settle_frames, idle_ms }
    }
}

fn load_average() -> String {
    std::fs::read_to_string("/proc/loadavg").map(|text| text.trim().to_string()).unwrap_or_default()
}

fn measure(lengths: &Lengths) -> Report {
    let load_before = load_average();
    let mut bench = Bench::new();
    let runs = ScriptedPath::ALL.map(|path| bench.run_path(path, lengths)).into();
    let world = bench.app.world();
    Report {
        runs,
        main_thread_builds: guard::builds_on_main_thread(),
        collider_builds: world.resource::<ColliderStats>().builds_started,
        max_patches_per_frame: world.resource::<PlanetLodStats>().max_spawned_in_frame,
        veg_resident_chunks: world.resource::<VegStreamStats>().resident_chunks,
        threads: AsyncComputeTaskPool::get().thread_num(),
        load_before,
        load_after: load_average(),
    }
}

fn record(lengths: &Lengths, file: &Path) -> std::io::Result<Report> {
    let report = measure(lengths);
    std::fs::create_dir_all(file.parent().unwrap_or(Path::new(".")))?;
    std::fs::write(file, report.json().to_string())?;
    Ok(report)
}

pub fn run(path_frames: usize, idle_frames: usize) -> Verdict {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join(RESULT_FILE);
    match record(&Lengths { path_frames, idle_frames, settle_frames: SETTLE_LIMIT }, &file) {
        Ok(report) => {
            println!("{}", report.json());
            if !report.passes() {
                let over: usize = report.runs.iter().map(PathRun::over_budget).sum();
                eprintln!("bench-planet FAILED: {over} frames over budget, {} main-thread builds, see {}", report.main_thread_builds, file.display());
            }
            let verdict = report.verdict();
            if let Verdict::Inconclusive(load) = verdict {
                eprintln!("bench-planet INCONCLUSIVE load={load}");
            }
            verdict
        }
        Err(error) => {
            eprintln!("bench-planet FAILED: cannot write {}: {error}", file.display());
            Verdict::Fail
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::offscreen::render_lock;
    use crate::testkit::parse;

    const RADIUS_M: f64 = 63_600.0;
    const FLAT_GROUND_M: f64 = 40.0;
    const POSE_TOLERANCE_M: f64 = 1e-6;
    const DIRECTION_TOLERANCE: f64 = 1e-9;
    const JSON_ROUND_TRIP_MS: f64 = 1e-9;

    fn flat(_: DVec3) -> f64 {
        FLAT_GROUND_M
    }

    fn altitude(pose: &Frame) -> f64 {
        pose.position.length() - RADIUS_M
    }

    fn flight(frame: usize) -> Frame {
        ScriptedPath::Flight.pose(RADIUS_M, flat, frame)
    }

    fn ground(frame: usize) -> Frame {
        ScriptedPath::Ground.pose(RADIUS_M, flat, frame)
    }

    #[test]
    fn the_census_anchor_is_the_stage_3_player_position() {
        let fixture = parse(include_str!("../testdata/parity/spoint-planet-veg-placement.json"));
        let player: Vec<f64> = fixture["thinning"]["playerWorld"].as_array().unwrap().iter().map(|value| value.as_f64().unwrap()).collect();
        assert!((CENSUS_ANCHOR - DVec3::from_slice(&player)).length() < POSE_TOLERANCE_M, "{CENSUS_ANCHOR} against {player:?}");
    }

    #[test]
    fn the_flight_drops_straight_down_over_the_anchor_through_100_20_and_5_km_to_the_census_pose() {
        let dir = CENSUS_ANCHOR.normalize();
        assert_eq!(ScriptedPath::Flight.frames(), 2100);
        for (frame, km) in [(0, 100.0), (600, 20.0), (1200, 5.0)] {
            let pose = flight(frame);
            assert!((altitude(&pose) - km * 1000.0).abs() < POSE_TOLERANCE_M, "frame {frame} altitude {}", altitude(&pose));
            assert!((pose.position.normalize() - dir).length() < DIRECTION_TOLERANCE, "frame {frame} is over the anchor");
            assert!((pose.forward + dir).length() < DIRECTION_TOLERANCE, "frame {frame} looks straight down");
        }
        let census = Frame::above(dir, CENSUS_ANCHOR.length(), 0.0, CENSUS_PITCH_DOWN_DEG);
        for frame in [1800, 2099] {
            let pose = flight(frame);
            assert!((pose.position - CENSUS_ANCHOR).length() < POSE_TOLERANCE_M, "frame {frame} is at the census anchor");
            assert!((pose.forward - census.forward).length() < DIRECTION_TOLERANCE && (pose.up - census.up).length() < DIRECTION_TOLERANCE);
        }
        let altitudes: Vec<f64> = (0..1800).map(|frame| altitude(&flight(frame))).collect();
        assert!(altitudes.windows(2).all(|pair| pair[1] < pair[0]), "the flight only descends");
        let ratio = |frame: usize| altitudes[frame + 1] / altitudes[frame];
        assert!((ratio(0) - ratio(598)).abs() < 1e-12 && (ratio(600) - ratio(1198)).abs() < 1e-12, "each leg descends at an even rate in log altitude");
    }

    #[test]
    fn the_ground_run_goes_800_m_north_at_10_m_s_at_the_census_eye() {
        let start = CENSUS_ANCHOR.normalize();
        let north = Frame::above(start, RADIUS_M, 0.0, 0.0).forward;
        let arc = |pose: &Frame| 2.0 * ((pose.position.normalize() - start).length() / 2.0).asin() * CENSUS_ANCHOR.length();
        assert_eq!(ScriptedPath::Ground.frames(), 4800);
        assert!((arc(&ground(0)) - 10.0 / 60.0).abs() < POSE_TOLERANCE_M);
        assert!((arc(&ground(4799)) - 800.0).abs() < POSE_TOLERANCE_M);
        assert!((ground(4799).position.normalize() - start).dot(north) > 0.0, "the run heads north");
        let eye = CENSUS_ANCHOR.length() - RADIUS_M - FLAT_GROUND_M;
        assert!((altitude(&ground(2400)) - FLAT_GROUND_M - eye).abs() < POSE_TOLERANCE_M, "the eye keeps the census height above the ground");
        let buried = ScriptedPath::Ground.pose(RADIUS_M, |_| 100.0, 0);
        assert!((altitude(&buried) - 100.0 - TELEPORT_CLEARANCE_M).abs() < POSE_TOLERANCE_M, "the eye keeps the teleport clearance");
    }

    #[test]
    fn the_budget_is_the_smaller_of_idle_p99_plus_1_ms_and_twice_idle_p99() {
        assert_eq!(budget_ms(0.5), 1.0);
        assert_eq!(budget_ms(1.0), 2.0);
        assert_eq!(budget_ms(3.0), 4.0);
        assert_eq!(frames_over(&[0.9, 1.0, 1.1, 2.0], 1.0), 2);
    }

    #[test]
    fn spreads_use_the_physics_bench_nearest_rank_pick() {
        let samples: Vec<f64> = (1..=100).rev().map(f64::from).collect();
        let picked = spread(&samples);
        assert_eq!((picked.p50, picked.p99, picked.max), (51.0, 99.0, 100.0));
    }

    fn path_run(main_ms: &[f64], settled: bool) -> PathRun {
        let frames = main_ms.iter().map(|&main_ms| FrameTimes { main_ms, render_ms: 0.0, spawned_patches: 0, tasks_in_flight: 0 }).collect();
        PathRun { path: ScriptedPath::Flight, settle_frames: settled.then_some(1), frames, idle_settle_frames: Some(1), idle_ms: vec![2.0; 10] }
    }

    fn report(runs: Vec<PathRun>, main_thread_builds: usize) -> Report {
        Report { runs, main_thread_builds, collider_builds: 0, max_patches_per_frame: 0, veg_resident_chunks: [0; 3], threads: 1, load_before: String::new(), load_after: String::new() }
    }

    #[test]
    fn a_frame_over_budget_an_unsettled_window_or_a_main_thread_build_fails_the_bench() {
        assert!(report(vec![path_run(&[2.5, 3.0], true)], 0).passes());
        let over = path_run(&[2.5, 3.01], true);
        assert_eq!(over.over_budget(), 1);
        assert!(!report(vec![over], 0).passes());
        assert!(!report(vec![path_run(&[2.5], false)], 0).passes());
        assert!(!report(vec![path_run(&[2.5], true)], 1).passes());
    }

    #[test]
    fn a_short_headless_run_writes_the_result_file() {
        let _render = render_lock();
        let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/witness/bench-planet-smoke.json");
        let report = record(&Lengths { path_frames: 3, idle_frames: 3, settle_frames: 3 }, &file).unwrap();
        let written = parse(&std::fs::read_to_string(&file).unwrap());
        assert_eq!(written["kind"], "bench-planet");
        assert_eq!(written["pass"], report.passes());
        assert_eq!(written["mainThreadBuilds"], 0);
        for path in ScriptedPath::ALL {
            let run = &written[path.name()];
            assert_eq!((run["frames"].as_u64(), run["idle"]["frames"].as_u64()), (Some(3), Some(3)), "{}", path.name());
            let budget = budget_ms(run["idle"]["p99Ms"].as_f64().unwrap());
            assert!((run["budgetMs"].as_f64().unwrap() - budget).abs() < JSON_ROUND_TRIP_MS, "{} budget {} against {budget}", path.name(), run["budgetMs"]);
        }
    }

    const SETTLE_FRAMES_FOR_UPLOAD_TEST: usize = 600;

    fn patches_without_a_gpu_mesh(app: &mut App) -> usize {
        use bevy::render::{RenderApp, mesh::RenderMesh, render_asset::RenderAssets};
        let handles: Vec<_> = app.world_mut().query_filtered::<&Mesh3d, With<crate::planet_lod::stream::PatchKey>>().iter(app.world()).map(|mesh| mesh.0.clone()).collect();
        let render = app.sub_app(RenderApp).world().resource::<RenderAssets<RenderMesh>>();
        handles.iter().filter(|handle| render.get(*handle).is_none()).count()
    }

    #[test]
    fn no_terrain_patch_is_without_its_gpu_mesh_while_the_planet_streams_in() {
        let _render = render_lock();
        let mut bench = Bench::new();
        let pose = bench.pose(ScriptedPath::Flight, 0);
        let worst = (0..SETTLE_FRAMES_FOR_UPLOAD_TEST).map(|_| {
            bench.step(&pose);
            patches_without_a_gpu_mesh(&mut bench.app)
        }).max();
        assert_eq!(worst, Some(0), "a terrain patch was live without its GPU mesh, leaving a hole its neighbours' skirts show");
        assert!(bench.app.world().resource::<PlanetLodStats>().live_patches > 0, "no patch streamed in");
    }

    fn loaded(mut report: Report, before: &str, after: &str) -> Report {
        report.load_before = before.to_string();
        report.load_after = after.to_string();
        report
    }

    #[test]
    fn a_load_above_the_limit_before_or_after_the_run_is_inconclusive_not_a_failure() {
        let calm = "0.50 1.00 1.00 1/100 1";
        let busy = "9.25 1.00 1.00 1/100 1";
        let failing = || report(vec![path_run(&[2.5, 3.01], true)], 0);
        let passing = || report(vec![path_run(&[2.5], true)], 0);
        assert_eq!(loaded(failing(), calm, calm).verdict(), Verdict::Fail);
        assert_eq!(loaded(passing(), calm, calm).verdict(), Verdict::Pass);
        assert_eq!(loaded(failing(), busy, calm).verdict(), Verdict::Inconclusive(9.25));
        assert_eq!(loaded(passing(), calm, busy).verdict(), Verdict::Inconclusive(9.25));
        assert_eq!(loaded(failing(), "4.00 1 1 1/1 1", "4.00 1 1 1/1 1").verdict(), Verdict::Fail, "the limit itself is quiet enough");
        assert_eq!((Verdict::Pass.exit_code(), Verdict::Fail.exit_code(), Verdict::Inconclusive(9.0).exit_code()), (0, 1, 77));
    }
}
