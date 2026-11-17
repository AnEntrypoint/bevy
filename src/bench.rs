use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;

use crate::protocol::{BALL_RADIUS, GRAVITY_Y, TICK, TICK_HZ};

fn next_random(state: &mut u32) -> f32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *state as f32 / 4_294_967_296.0
}

fn spawn_scene(mut commands: Commands, bodies: usize) {
    commands.spawn((
        RigidBody::Static,
        Collider::cuboid(10.0, 0.5, 10.0),
        Position::new(Vec3::new(0.0, 0.5, 0.0)),
        Friction::new(0.0),
        Restitution::new(0.2),
    ));
    commands.spawn((
        RigidBody::Static,
        Collider::half_space(Vec3::Y),
        Position::new(Vec3::ZERO),
        Friction::new(0.0),
        Restitution::new(0.2),
    ));
    let mut seed = 1u32;
    for _ in 0..bodies {
        let position = Vec3::new(
            next_random(&mut seed) * 60.0 - 30.0,
            5.0 + next_random(&mut seed) * 40.0,
            next_random(&mut seed) * 60.0 - 30.0,
        );
        commands.spawn((
            RigidBody::Dynamic,
            Collider::sphere(BALL_RADIUS),
            Position::new(position),
            LinearVelocity::default(),
            LinearDamping(0.1),
            Friction::new(0.0),
            Restitution::new(0.2),
            LockedAxes::default()
                .lock_rotation_x()
                .lock_rotation_y()
                .lock_rotation_z(),
        ));
    }
}

fn build_app(bodies: usize, diagnostics: bool) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        PhysicsPlugins::default()
            .build()
            .disable::<PhysicsTransformPlugin>()
            .disable::<PhysicsInterpolationPlugin>()
            .disable::<IslandSleepingPlugin>(),
    ));
    if diagnostics {
        app.add_plugins(PhysicsDiagnosticsPlugin);
    }
    app.insert_resource(Time::<Fixed>::from_hz(TICK_HZ));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(TICK));
    app.insert_resource(Gravity(Vec3::new(0.0, GRAVITY_Y, 0.0)));
    app.add_systems(Startup, move |commands: Commands| spawn_scene(commands, bodies));
    app
}

fn state_hash(app: &mut App) -> u32 {
    let mut query = app.world_mut().query::<(&Position, &LinearVelocity)>();
    let mut hash = 0x811c_9dc5u32;
    for (position, velocity) in query.iter(app.world()) {
        for value in [
            position.0.x,
            position.0.y,
            position.0.z,
            velocity.0.x,
            velocity.0.y,
            velocity.0.z,
        ] {
            hash = (hash ^ value.to_bits()).wrapping_mul(0x0100_0193);
        }
    }
    hash
}

fn simulate(bodies: usize, warmup: usize, ticks: usize) -> (App, Vec<f64>) {
    simulate_with(bodies, warmup, ticks, false)
}

fn simulate_with(bodies: usize, warmup: usize, ticks: usize, diagnostics: bool) -> (App, Vec<f64>) {
    let mut app = build_app(bodies, diagnostics);
    app.finish();
    app.cleanup();
    for _ in 0..warmup {
        app.update();
    }
    let mut samples = Vec::with_capacity(ticks);
    for _ in 0..ticks {
        let started = std::time::Instant::now();
        app.update();
        samples.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    (app, samples)
}

pub fn run(bodies: usize, ticks: usize) {
    let (mut app, mut samples) = simulate(bodies, 30, ticks);
    let hash = state_hash(&mut app);
    samples.sort_by(f64::total_cmp);
    let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    let pick = |q: f64| samples[((samples.len() as f64 - 1.0) * q).round() as usize];
    let threads = bevy::tasks::ComputeTaskPool::get().thread_num();
    println!(
        "{{\"kind\":\"bench\",\"engine\":\"bevy-avian\",\"bodies\":{bodies},\"ticks\":{ticks},\"threads\":{threads},\"meanMs\":{:.4},\"p50Ms\":{:.4},\"p95Ms\":{:.4},\"p99Ms\":{:.4},\"ticksPerSecondAtMean\":{},\"hash\":\"0x{:08x}\"}}",
        mean,
        pick(0.50),
        pick(0.95),
        pick(0.99),
        (1000.0 / mean).round() as u64,
        hash
    );
}

pub fn phases(bodies: usize, ticks: usize) {
    let (app, samples) = simulate_with(bodies, 30, ticks, true);
    let threads = bevy::tasks::ComputeTaskPool::get().thread_num();
    let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    let store = app.world().resource::<bevy::diagnostic::DiagnosticsStore>();
    let mut phases: Vec<(String, f64, usize)> = store
        .iter()
        .filter_map(|diagnostic| {
            let values: Vec<f64> = diagnostic.values().copied().collect();
            (!values.is_empty()).then(|| {
                let average = values.iter().sum::<f64>() / values.len() as f64;
                (diagnostic.path().to_string(), average, values.len())
            })
        })
        .collect();
    phases.sort_by(|a, b| a.0.cmp(&b.0));
    let entries: Vec<String> = phases
        .iter()
        .map(|(path, average, count)| format!("\"{path}\":{{\"meanMs\":{average:.4},\"samples\":{count}}}"))
        .collect();
    println!(
        "{{\"kind\":\"bench-phases\",\"engine\":\"bevy-avian\",\"bodies\":{bodies},\"ticks\":{ticks},\"threads\":{threads},\"tickMeanMs\":{mean:.4},\"phases\":{{{}}}}}",
        entries.join(",")
    );
}

pub fn schedules(bodies: usize, ticks: usize) {
    let mut app = build_app(bodies, false);
    app.finish();
    app.cleanup();
    for _ in 0..30 {
        app.update();
    }
    let labels = app.world().resource::<bevy::app::MainScheduleOrder>().labels.clone();
    let mut totals = vec![0.0f64; labels.len()];
    let mut whole = 0.0f64;
    for _ in 0..ticks {
        let started = std::time::Instant::now();
        for (index, label) in labels.iter().enumerate() {
            let step = std::time::Instant::now();
            let _ = app.world_mut().try_run_schedule(*label);
            totals[index] += step.elapsed().as_secs_f64() * 1000.0;
        }
        app.world_mut().clear_trackers();
        whole += started.elapsed().as_secs_f64() * 1000.0;
    }
    let entries: Vec<String> = labels
        .iter()
        .zip(&totals)
        .map(|(label, total)| format!("\"{label:?}\":{:.4}", total / ticks as f64))
        .collect();
    println!(
        "{{\"kind\":\"bench-schedules\",\"bodies\":{bodies},\"ticks\":{ticks},\"tickMeanMs\":{:.4},\"schedules\":{{{}}}}}",
        whole / ticks as f64,
        entries.join(",")
    );
}

pub fn det(bodies: usize, ticks: usize) -> bool {
    let (mut first, _) = simulate(bodies, 30, ticks);
    let (mut second, _) = simulate(bodies, 30, ticks);
    let a = state_hash(&mut first);
    let b = state_hash(&mut second);
    let pass = a == b;
    println!(
        "{{\"kind\":\"det\",\"bodies\":{bodies},\"ticks\":{ticks},\"hashA\":\"0x{a:08x}\",\"hashB\":\"0x{b:08x}\",\"pass\":{pass}}}"
    );
    pass
}
