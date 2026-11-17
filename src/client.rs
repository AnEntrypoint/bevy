use avian3d::prelude::*;
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use lightyear::prelude::client::*;
use lightyear::prelude::*;

use crate::protocol::*;

#[derive(Resource, Default)]
struct Script {
    start: Option<f32>,
}

pub fn run() {
    let mut app = base_app(Some(core::time::Duration::from_nanos(16_666_667)));
    app.add_plugins(ClientPlugins { tick_duration: TICK });
    app.add_plugins(SharedPlugin);
    app.insert_resource(
        InputTimelineConfig::default().with_input_delay(InputDelayConfig::fixed_input_delay(0)),
    );
    app.insert_resource(Script::default());
    app.insert_resource(PredictionManager::default());
    app.add_systems(Startup, connect);
    app.add_observer(add_input_map);
    app.add_systems(Update, attach_physics);
    app.add_systems(First, scripted_keys);
    app.add_systems(FixedUpdate, (apply_player_input, report_and_exit));
    app.run();
}

fn connect(mut commands: Commands) {
    let client = commands
        .spawn((
            Client,
            LocalAddr(CLIENT_ADDR),
            PeerAddr(SERVER_ADDR),
            Link::default(),
            ReplicationReceiver,
            RawClient,
            UdpIo::default(),
        ))
        .id();
    commands.trigger(Connect { entity: client });
}

fn add_input_map(trigger: On<Add<Controlled>>, mut commands: Commands) {
    commands.entity(trigger.entity).insert(
        InputMap::default().with_dual_axis(PlayerAction::Move, VirtualDPad::wasd()),
    );
}

fn attach_physics(
    mut commands: Commands,
    players: Query<Entity, (Added<Predicted>, With<Player>)>,
    floors: Query<Entity, (Added<Remote>, With<Floor>)>,
) {
    for entity in &players {
        commands.entity(entity).insert(PlayerPhysicsBundle::default());
    }
    for entity in &floors {
        commands.entity(entity).insert(FloorPhysicsBundle::default());
    }
}

fn scripted_keys(
    time: Res<Time>,
    controlled: Query<(), With<Controlled>>,
    mut script: ResMut<Script>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
) {
    if script.start.is_none() && !controlled.is_empty() {
        script.start = Some(time.elapsed_secs());
    }
    let moving = script.start.is_some_and(|start| {
        let elapsed = time.elapsed_secs() - start;
        elapsed > 0.25 && elapsed < 2.75
    });
    if moving {
        keys.press(KeyCode::KeyD);
    } else {
        keys.release(KeyCode::KeyD);
    }
}

fn report_and_exit(
    time: Res<Time>,
    script: Res<Script>,
    players: Query<
        (
            Entity,
            &Position,
            &LinearVelocity,
            &PredictionHistory<Position>,
            &ConfirmedHistory<Position>,
        ),
        With<Player>,
    >,
    predicted: Query<(), With<Predicted>>,
    controlled: Query<(), With<Controlled>>,
    remote: Query<(), With<Remote>>,
    mut exit: MessageWriter<AppExit>,
    mut reported: Local<bool>,
) {
    if *reported {
        return;
    }
    let Some(start) = script.start else {
        return;
    };
    if time.elapsed_secs() - start < 5.0 {
        return;
    }
    *reported = true;
    let mut error = f32::INFINITY;
    let mut confirmed_tick = 0u32;
    let mut current = Vec3::ZERO;
    let mut predicted_at_confirmed = Vec3::ZERO;
    let mut confirmed_value = Vec3::ZERO;
    let mut predicted_len = 0usize;
    let mut confirmed_len = 0usize;
    for (entity, position, velocity, prediction, confirmation) in &players {
        predicted_len = prediction.len();
        confirmed_len = confirmation.len();
        println!(
            "{{\"kind\":\"client-debug\",\"entity\":{entity:?},\"pos\":[{:.4},{:.4},{:.4}],\"vel\":[{:.4},{:.4},{:.4}],\"predictionHistory\":{},\"confirmedHistory\":{},\"newestConfirmed\":{:?}}}",
            position.0.x,
            position.0.y,
            position.0.z,
            velocity.0.x,
            velocity.0.y,
            velocity.0.z,
            prediction.len(),
            confirmation.len(),
            confirmation.newest_present().map(|(tick, _)| tick.0)
        );
        let Some((tick, value)) = confirmation.newest_present() else {
            continue;
        };
        let Some(value_at_tick) = prediction.get(tick) else {
            continue;
        };
        let distance = value_at_tick.0.distance(value.0);
        if distance < error {
            error = distance;
            confirmed_tick = tick.0;
            current = position.0;
            predicted_at_confirmed = value_at_tick.0;
            confirmed_value = value.0;
        }
    }
    let error_json = if error.is_finite() { format!("{error:.6}") } else { "null".to_string() };
    println!(
        "{{\"kind\":\"client-report\",\"players\":{},\"predicted\":{},\"controlled\":{},\"remote\":{},\"confirmedTick\":{},\"errorM\":{error_json},\"current\":[{:.4},{:.4},{:.4}],\"predictedPos\":[{:.4},{:.4},{:.4}],\"confirmedPos\":[{:.4},{:.4},{:.4}],\"predictionHistory\":{},\"confirmedHistory\":{}}}",
        players.iter().count(),
        predicted.iter().count(),
        controlled.iter().count(),
        remote.iter().count(),
        confirmed_tick,
        current.x,
        current.y,
        current.z,
        predicted_at_confirmed.x,
        predicted_at_confirmed.y,
        predicted_at_confirmed.z,
        confirmed_value.x,
        confirmed_value.y,
        confirmed_value.z,
        predicted_len,
        confirmed_len
    );
    exit.write(AppExit::Success);
}
