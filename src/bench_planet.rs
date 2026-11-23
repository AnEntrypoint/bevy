use crate::bench::percentile;
use crate::frame_clock;
use crate::bench_host::{self, Watch};
use crate::globe::{self, GlobeWorld};
use crate::height::Planet;
use crate::offscreen::{Frame, add_target, headless_app, pump, spawn_view};
use crate::planet_lod::collider::{ColliderStats, NearBody};
use crate::planet_lod::guard;
use crate::planet_lod::stream::PlanetLodStats;
use crate::planet_poses::{CENSUS_ANCHOR, CENSUS_PITCH_DOWN_DEG, Pose, STRAIGHT_DOWN_DEG, mix, on_ground};
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
const FLIGHT_POSES: [Pose; 3] = [Pose::Orbit, Pose::High, Pose::Low];
const LEG_FRAMES: usize = 600;
const LANDED_HOLD_FRAMES: usize = 300;
const GROUND_SPEED_M_S: f64 = 10.0;
const GROUND_FRAMES: usize = 4800;
pub const IDLE_FRAMES: usize = 600;
const SETTLE_LIMIT: usize = 4000;
const SETTLED_STREAK: usize = 4;
const BUDGET_HEADROOM_MS: f64 = 1.0;
const BUDGET_IDLE_MULTIPLE: f64 = 2.0;
const BUDGET_FLOOR_MS: f64 = 2.0;
const REFERENCE_SAMPLES: usize = 3;
const REFERENCE_FRAMES: usize = 300;
const BUDGET_SWING_TOLERANCE: f64 = 0.5;
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
            Self::Flight => FLIGHT_POSES.len() * LEG_FRAMES + LANDED_HOLD_FRAMES,
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

fn flight_pose(radius: f64, ground: impl Fn(DVec3) -> f64, frame: usize) -> Frame {
    let landed = Pose::Ground.frame(radius, &ground);
    let leg = frame / LEG_FRAMES;
    let Some(from) = FLIGHT_POSES.get(leg).and_then(|pose| pose.altitude_m()) else {
        return landed;
    };
    let to = FLIGHT_POSES.get(leg + 1).and_then(|pose| pose.altitude_m()).unwrap_or(landed.position.length() - radius);
    let t = (frame % LEG_FRAMES) as f64 / LEG_FRAMES as f64;
    let pitch = if leg + 1 == FLIGHT_POSES.len() { mix(STRAIGHT_DOWN_DEG, CENSUS_PITCH_DOWN_DEG, t) } else { STRAIGHT_DOWN_DEG };
    Frame::above(CENSUS_ANCHOR.normalize(), radius, mix(from.ln(), to.ln(), t).exp(), pitch)
}

fn section_map(sections: &[f64]) -> Value {
    Value::Object(frame_clock::NAMES.iter().zip(sections).map(|(name, ms)| (name.to_string(), json!(ms))).collect())
}

fn mean_sections<'a>(frames: impl Iterator<Item = &'a FrameTimes>) -> [f64; frame_clock::SECTIONS] {
    let (sum, count) = frames.fold(([0.0; frame_clock::SECTIONS], 0usize), |(mut sum, count), frame| {
        for (total, ms) in sum.iter_mut().zip(frame.sections) {
            *total += ms;
        }
        (sum, count + 1)
    });
    sum.map(|total| total / count.max(1) as f64)
}

fn budget_ms(idle_p99_ms: f64) -> f64 {
    (idle_p99_ms + BUDGET_HEADROOM_MS).min(BUDGET_IDLE_MULTIPLE * idle_p99_ms).max(BUDGET_FLOOR_MS)
}

struct Reference {
    p99_ms: Vec<f64>,
}

impl Reference {
    fn median_ms(&self) -> f64 {
        let mut sorted = self.p99_ms.clone();
        sorted.sort_by(f64::total_cmp);
        sorted[sorted.len() / 2]
    }

    fn swing(&self) -> f64 {
        let (low, high) = self.p99_ms.iter().fold((f64::MAX, 0.0_f64), |(low, high), p99| (low.min(*p99), high.max(*p99)));
        (high - low) / self.median_ms()
    }

    fn budget_ms(&self) -> f64 {
        budget_ms(self.median_ms())
    }
}

const INCONCLUSIVE_EXIT: i32 = 77;

#[derive(Clone, Debug, PartialEq)]
pub enum Verdict {
    Pass,
    Fail,
    Inconclusive(String),
}

impl Verdict {
    pub fn exit_code(&self) -> i32 {
        match self {
            Verdict::Pass => 0,
            Verdict::Fail => 1,
            Verdict::Inconclusive(_) => INCONCLUSIVE_EXIT,
        }
    }
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
    sim_ms: f64,
    extract_ms: f64,
    render_ms: f64,
    spawned_patches: usize,
    tasks_in_flight: usize,
    veg_tasks_in_flight: usize,
    veg_queued: usize,
    sections: [f64; frame_clock::SECTIONS],
}

struct PathRun {
    path: ScriptedPath,
    settle_frames: Option<usize>,
    frames: Vec<FrameTimes>,
    idle_settle_frames: Option<usize>,
    idle_ms: Vec<f64>,
    budget_ms: f64,
}

impl PathRun {
    fn main_ms(&self) -> Vec<f64> {
        self.frames.iter().map(|frame| frame.main_ms).collect()
    }

    fn budget_ms(&self) -> f64 {
        self.budget_ms
    }

    fn over_budget(&self) -> usize {
        frames_over(&self.main_ms(), self.budget_ms())
    }

    fn over_budget_frames(&self) -> Vec<Value> {
        let budget = self.budget_ms();
        let over = self.frames.iter().enumerate().filter(|(_, frame)| frame.main_ms > budget);
        over.map(|(index, frame)| json!({ "frame": index, "mainMs": frame.main_ms, "simMs": frame.sim_ms, "extractMs": frame.extract_ms, "spawnedPatches": frame.spawned_patches, "tasksInFlight": frame.tasks_in_flight, "vegTasksInFlight": frame.veg_tasks_in_flight, "vegQueued": frame.veg_queued, "sections": section_map(&frame.sections) })).collect()
    }

    fn passes(&self) -> bool {
        self.settle_frames.is_some() && self.idle_settle_frames.is_some() && spread(&self.main_ms()).p99 <= self.budget_ms
    }

    fn json(&self) -> Value {
        let (main, idle) = (spread(&self.main_ms()), spread(&self.idle_ms));
        let render: Vec<f64> = self.frames.iter().map(|frame| frame.render_ms).collect();
        json!({
            "frames": self.frames.len(), "settleFrames": self.settle_frames,
            "p50Ms": main.p50, "p99Ms": main.p99, "maxMs": main.max, "renderP99Ms": spread(&render).p99,
            "idle": { "frames": self.idle_ms.len(), "settleFrames": self.idle_settle_frames, "p50Ms": idle.p50, "p99Ms": idle.p99, "maxMs": idle.max },
            "budgetMs": self.budget_ms(), "overBudget": self.over_budget(), "overBudgetFrames": self.over_budget_frames(), "pass": self.passes(),
            "sectionMeanMs": section_map(&mean_sections(self.frames.iter())), "overBudgetSectionMeanMs": section_map(&mean_sections(self.frames.iter().filter(|frame| frame.main_ms > self.budget_ms())))
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
    idle_before: f64,
    competitor: Option<String>,
    reference: Reference,
}

impl Report {
    fn passes(&self) -> bool {
        self.main_thread_builds == 0 && self.runs.iter().all(PathRun::passes)
    }

    fn verdict(&self) -> Verdict {
        let swung = (self.reference.swing() > BUDGET_SWING_TOLERANCE).then(|| format!("the idle p99 swung {:.0}% across {REFERENCE_SAMPLES} samples at the start (tolerance {:.0}%)", self.reference.swing() * 100.0, BUDGET_SWING_TOLERANCE * 100.0));
        match (bench_host::unquiet(self.idle_before, self.competitor.as_deref()).or(swung), self.passes()) {
            (Some(reason), _) => Verdict::Inconclusive(reason),
            (None, true) => Verdict::Pass,
            (None, false) => Verdict::Fail,
        }
    }

    fn json(&self) -> Value {
        let mut value = json!({
            "kind": "bench-planet", "stepS": STEP_S, "threads": self.threads, "idleBefore": self.idle_before, "idleMin": bench_host::IDLE_MIN, "competitor": self.competitor, "competitorCpuPct": bench_host::COMPETITOR_CPU_PCT, "referenceIdleP99Ms": self.reference.p99_ms, "referenceMedianMs": self.reference.median_ms(), "budgetSwing": self.reference.swing(), "swingTolerance": BUDGET_SWING_TOLERANCE,
            "mainThreadBuilds": self.main_thread_builds, "colliderBuildsStarted": self.collider_builds, "maxPatchesPerFrame": self.max_patches_per_frame, "vegResidentChunks": self.veg_resident_chunks,
            "pass": self.passes(), "verdict": format!("{:?}", self.verdict())
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

fn add_schedule_marks(app: &mut App) {
    app.add_systems(First, || frame_clock::restart())
        .add_systems(RunFixedMainLoop, (|| frame_clock::mark(12)).in_set(RunFixedMainLoopSystems::AfterFixedMainLoop))
        .add_systems(PostUpdate, (|| frame_clock::mark(13)).before(TransformSystems::Propagate))
        .add_systems(Last, || frame_clock::mark(14));
}

struct Bench {
    app: App,
    camera: Entity,
    body: Entity,
    planet: Arc<Planet>,
    radius: f64,
    #[cfg(test)]
    target: Handle<Image>,
}

impl Bench {
    fn new() -> Self {
        Self::with_globe(GlobeWorld { bodies: Vec::new(), ..default() })
    }

    fn with_globe(globe: GlobeWorld) -> Self {
        let (planet, radius) = (globe.planet.clone(), f64::from(globe.radius));
        let mut app = headless_app();
        app.add_plugins(RenderStatsPlugin).insert_resource(globe).insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(STEP_S)));
        globe::register(&mut app);
        add_schedule_marks(&mut app);
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
        Self { app, camera, body, planet, radius, #[cfg(test)] target }
    }

    fn pose(&self, path: ScriptedPath, frame: usize) -> Frame {
        path.pose(self.radius, |dir| self.planet.height(dir.to_array(), self.radius), frame)
    }

    fn step(&mut self, pose: &Frame) -> FrameTimes {
        let translation = pose.position.as_vec3();
        let world = self.app.world_mut();
        *world.get_mut::<Transform>(self.camera).expect("the bench camera") = Transform::from_translation(translation).looking_to(pose.forward.as_vec3(), pose.up.as_vec3());
        *world.get_mut::<Transform>(self.body).expect("the near body") = Transform::from_translation(translation);
        let before = frame_clock::totals_ms();
        pump(&mut self.app);
        let after = frame_clock::totals_ms();
        let sections = std::array::from_fn(|index| after[index] - before[index]);
        let stats = self.app.world().resource::<SharedFrameStats>().snapshot();
        let span = |ms: Option<f64>| ms.expect("the render stats time every frame");
        let lod = self.app.world().resource::<PlanetLodStats>();
        let veg = self.app.world().resource::<VegStreamStats>();
        let (sim_ms, extract_ms) = (span(stats.sim_ms), span(stats.extract_ms));
        FrameTimes { main_ms: sim_ms + extract_ms, sim_ms, extract_ms, render_ms: span(stats.render_ms), spawned_patches: lod.spawned_this_frame, tasks_in_flight: lod.tasks_in_flight, veg_tasks_in_flight: veg.tasks_in_flight, veg_queued: veg.queued_chunks.iter().sum(), sections }
    }

    fn settle(&mut self, pose: &Frame, limit: usize) -> Option<usize> {
        let mut streak = 0;
        (1..=limit).find(|_| {
            self.step(pose);
            streak = if settled(self.app.world()) { streak + 1 } else { 0 };
            streak == SETTLED_STREAK
        })
    }

    fn reference(&mut self, lengths: &Lengths) -> Reference {
        let start = self.pose(ScriptedPath::Flight, 0);
        self.settle(&start, lengths.settle_frames);
        let p99_ms = (0..REFERENCE_SAMPLES).map(|_| spread(&(0..REFERENCE_FRAMES).map(|_| self.step(&start).main_ms).collect::<Vec<_>>()).p99).collect();
        Reference { p99_ms }
    }

    fn run_path(&mut self, path: ScriptedPath, lengths: &Lengths, budget_ms: f64) -> PathRun {
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
        PathRun { path, settle_frames, frames, idle_settle_frames, idle_ms, budget_ms }
    }
}

fn measure(lengths: &Lengths) -> Report {
    let idle_before = bench_host::measure_idle();
    let watch = Watch::start(bench_host::SAMPLE_INTERVAL);
    let mut bench = Bench::new();
    let reference = bench.reference(lengths);
    let runs = ScriptedPath::ALL.map(|path| bench.run_path(path, lengths, reference.budget_ms())).into();
    let world = bench.app.world();
    Report {
        runs,
        main_thread_builds: guard::builds_on_main_thread(),
        collider_builds: world.resource::<ColliderStats>().builds_started,
        max_patches_per_frame: world.resource::<PlanetLodStats>().max_spawned_in_frame,
        veg_resident_chunks: world.resource::<VegStreamStats>().resident_chunks,
        threads: AsyncComputeTaskPool::get().thread_num(),
        idle_before,
        competitor: watch.finish(),
        reference,
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
            if let Verdict::Inconclusive(reason) = &verdict {
                eprintln!("bench-planet INCONCLUSIVE {reason}");
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
    use crate::planet_poses::TELEPORT_CLEARANCE_M;
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
    fn the_budget_is_the_smaller_of_idle_p99_plus_1_ms_and_twice_idle_p99_with_a_floor() {
        assert_eq!(budget_ms(0.5), 2.0);
        assert_eq!(budget_ms(2.2), 3.2);
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
        let frames = main_ms.iter().map(|&main_ms| FrameTimes { main_ms, sim_ms: main_ms, extract_ms: 0.0, render_ms: 0.0, spawned_patches: 0, tasks_in_flight: 0, veg_tasks_in_flight: 0, veg_queued: 0, sections: [0.0; frame_clock::SECTIONS] }).collect();
        PathRun { path: ScriptedPath::Flight, settle_frames: settled.then_some(1), frames, idle_settle_frames: Some(1), idle_ms: vec![2.0; 10], budget_ms: 3.0 }
    }

    fn report(runs: Vec<PathRun>, main_thread_builds: usize) -> Report {
        Report { runs, main_thread_builds, collider_builds: 0, max_patches_per_frame: 0, veg_resident_chunks: [0; 3], threads: 1, idle_before: 1.0, competitor: None, reference: Reference { p99_ms: vec![2.0; REFERENCE_SAMPLES] } }
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
            let budget = budget_ms(written["referenceMedianMs"].as_f64().unwrap());
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

    fn hosted(mut report: Report, idle_before: f64, competitor: Option<&str>) -> Report {
        report.idle_before = idle_before;
        report.competitor = competitor.map(str::to_string);
        report
    }

    #[test]
    fn an_unquiet_host_before_or_during_the_run_is_inconclusive_not_a_failure() {
        let failing = || report(vec![path_run(&[2.5, 3.01], true)], 0);
        let passing = || report(vec![path_run(&[2.5], true)], 0);
        assert_eq!(hosted(failing(), 0.9, None).verdict(), Verdict::Fail);
        assert_eq!(hosted(passing(), 0.9, None).verdict(), Verdict::Pass);
        assert!(matches!(hosted(failing(), 0.5, None).verdict(), Verdict::Inconclusive(_)));
        assert!(matches!(hosted(passing(), 0.9, Some("cc pid 1 at 90% cpu")).verdict(), Verdict::Inconclusive(_)));
        let codes = [Verdict::Pass, Verdict::Fail, Verdict::Inconclusive(String::new())].map(|verdict| verdict.exit_code());
        assert_eq!(codes, [0, 1, 77]);
    }

    #[test]
    fn the_budget_comes_from_the_median_of_the_start_samples_and_a_big_swing_is_inconclusive() {
        let steady = Reference { p99_ms: vec![2.0, 9.0 / 4.0, 2.2] };
        assert_eq!(steady.median_ms(), 2.2);
        assert_eq!(steady.budget_ms(), 3.2);
        let passing = || report(vec![path_run(&[2.5], true)], 0);
        let mut calm = passing();
        calm.reference = steady;
        assert_eq!(calm.verdict(), Verdict::Pass);
        let mut swinging = passing();
        swinging.reference = Reference { p99_ms: vec![2.0, 2.2, 5.0] };
        assert!(matches!(swinging.verdict(), Verdict::Inconclusive(reason) if reason.contains("swung")));
    }

    #[test]
    fn a_path_passes_when_its_p99_is_inside_the_budget_even_with_a_lone_outlier() {
        let mut samples = vec![2.5; 99];
        samples.push(9.0);
        let outlier = path_run(&samples, true);
        assert_eq!(outlier.over_budget(), 1);
        assert!(outlier.passes());
        let mut slow = vec![2.5; 98];
        slow.extend([9.0, 9.0]);
        assert!(!path_run(&slow, true).passes());
    }

    const LUMA_STD_MIN: f64 = 0.01;
    const DISTINCT_COLOURS_MIN: usize = 32;

    fn launched_globe() -> GlobeWorld {
        let launch = crate::default_launch();
        let tree = crate::tree::WorldTree::load(Path::new(&launch[1])).expect("the default world");
        let place = tree.places.get(&launch[2]).expect("the default place");
        GlobeWorld { bodies: Vec::new(), ..GlobeWorld::from_value(place.config.extra.get("globe")) }
    }

    #[test]
    fn the_default_launch_passes_the_pose_captures() {
        use crate::offscreen::{image_stats, write_png};
        let _render = render_lock();
        let mut bench = Bench::with_globe(launched_globe());
        let (planet, radius) = (bench.planet.clone(), bench.radius);
        let ground = |dir: DVec3| planet.height(dir.to_array(), radius);
        let mut lines = Vec::new();
        let mut levels = Vec::new();
        for (name, pose) in [("orbit", Pose::Orbit), ("high", Pose::High), ("low", Pose::Low), ("ground", Pose::Ground), ("shoreline", Pose::Shoreline)] {
            let frame = pose.frame(radius, &ground);
            let settle_frames = bench.settle(&frame, SETTLE_LIMIT);
            assert!(settle_frames.is_some(), "{name}: the planet never settled");
            let image = crate::offscreen::capture(&mut bench.app, bench.target.clone(), name);
            let stats = image_stats(&image);
            assert!(stats.luma_std >= LUMA_STD_MIN, "{name}: the capture is flat, luma std {}", stats.luma_std);
            assert!(stats.distinct_colours >= DISTINCT_COLOURS_MIN, "{name}: {} colours", stats.distinct_colours);
            write_png(&image, &format!("pose-native-{name}"));
            let world = bench.app.world();
            let (lod, veg) = (world.resource::<PlanetLodStats>(), world.resource::<VegStreamStats>());
            levels.push(lod.highest_level);
            lines.push(json!({ "pose": name, "settleFrames": settle_frames, "livePatches": lod.live_patches, "highestLevel": lod.highest_level, "vegResidentChunks": veg.resident_chunks, "lumaStd": stats.luma_std, "distinctColours": stats.distinct_colours }));
            if pose == Pose::Ground {
                assert!(veg.resident_chunks.iter().sum::<usize>() > 0, "ground: no vegetation streamed in");
            }
        }
        assert!(levels[0] < levels[1] && levels[1] <= levels[2], "the level of detail must deepen from orbit to high to low: {levels:?}");
        let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/witness/pose-native.json");
        std::fs::write(file, Value::Array(lines).to_string()).unwrap();
    }

    #[test]
    fn an_over_budget_frame_names_its_simulation_extraction_and_vegetation_load() {
        let mut run = path_run(&[2.5, 9.0], true);
        run.frames[1].sim_ms = 6.0;
        run.frames[1].extract_ms = 3.0;
        run.frames[1].main_ms = 9.0;
        run.frames[1].veg_queued = 7;
        let over = run.over_budget_frames();
        assert_eq!(over.len(), 1);
        assert_eq!((over[0]["simMs"].as_f64(), over[0]["extractMs"].as_f64(), over[0]["vegQueued"].as_u64()), (Some(6.0), Some(3.0), Some(7)));
    }

    #[test]
    fn the_default_launch_registers_every_planet_plugin() {
        let _render = render_lock();
        let globe = launched_globe();
        let mut app = headless_app();
        app.insert_resource(globe.clone());
        crate::planet_scene::add_planet(&mut app, &globe);
        app.finish();
        app.cleanup();
        pump(&mut app);
        let world = app.world();
        assert!(world.contains_resource::<PlanetLodStats>(), "the LOD plugin is missing");
        assert!(world.contains_resource::<TerrainMaterialHandle>(), "the terrain material is missing");
        assert!(world.contains_resource::<VegStreamStats>(), "the vegetation stream is missing");
        assert!(world.contains_resource::<VegFrame>(), "the vegetation uploads are missing");
        assert!(world.contains_resource::<TreesWitness>(), "the trees are missing");
    }
}
