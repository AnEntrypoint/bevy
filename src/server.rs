use avian3d::prelude::*;
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use lightyear::prelude::server::*;
use lightyear::prelude::*;

use crate::protocol::*;

#[derive(Resource, Default)]
struct ServerClock(u32);

pub fn run() {
    let mut app = base_app(Some(TICK));
    app.add_plugins(ServerPlugins { tick_duration: TICK });
    app.add_plugins(SharedPlugin);
    app.init_resource::<ServerClock>();
    app.add_systems(Startup, start_server);
    app.add_observer(handle_new_client);
    app.add_systems(FixedUpdate, (apply_player_input, report_state));
    app.run();
}

fn start_server(mut commands: Commands) {
    commands.spawn((
        Floor,
        FloorPhysicsBundle::default(),
        Position::new(Vec3::new(0.0, 0.5, 0.0)),
        Rotation::default(),
        Replicate::to_clients(NetworkTarget::All),
    ));
    let server = commands.spawn((RawServer, LocalAddr(SERVER_ADDR), ServerUdpIo)).id();
    commands.trigger(Start { entity: server });
}

fn handle_new_client(trigger: On<Add<Connected>>, mut commands: Commands) {
    commands.entity(trigger.entity).insert(ReplicationSender);
    commands.spawn((
        Player,
        PlayerPhysicsBundle::default(),
        Position::new(Vec3::new(0.0, 1.5, 0.0)),
        Rotation::default(),
        LinearVelocity::default(),
        AngularVelocity::default(),
        ActionState::<PlayerAction>::default(),
        Replicate::to_clients(NetworkTarget::All),
        PredictionTarget::to_clients(NetworkTarget::All),
        ControlledBy {
            owner: trigger.entity,
            lifetime: Default::default(),
        },
    ));
}

fn report_state(
    mut clock: ResMut<ServerClock>,
    links: Query<(), With<Connected>>,
    players: Query<(&Position, &LinearVelocity), With<Player>>,
) {
    clock.0 += 1;
    if clock.0 % 64 != 0 {
        return;
    }
    print!("{{\"kind\":\"server-state\",\"tick\":{},\"clients\":{},\"players\":[", clock.0, links.iter().count());
    let mut first = true;
    for (position, velocity) in &players {
        if !first {
            print!(",");
        }
        first = false;
        print!(
            "{{\"p\":[{:.4},{:.4},{:.4}],\"v\":[{:.4},{:.4},{:.4}]}}",
            position.0.x, position.0.y, position.0.z, velocity.0.x, velocity.0.y, velocity.0.z
        );
    }
    println!("]}}");
}
