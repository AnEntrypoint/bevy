use avian3d::prelude::*;
use bevy::prelude::*;
use core::net::{IpAddr, Ipv4Addr, SocketAddr};
use core::time::Duration;
use leafwing_input_manager::prelude::*;
use lightyear::avian3d::plugin::{AvianReplicationMode, LightyearAvianPlugin};
use lightyear::input::prelude::InputConfig;
use lightyear::prelude::input::leafwing;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

pub const TICK_HZ: f64 = 64.0;
pub const TICK: Duration = Duration::from_nanos(15_625_000);
pub const SERVER_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 5000);
pub const CLIENT_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 4000);
pub const GRAVITY_Y: f32 = -18.0;
pub const BALL_RADIUS: f32 = 0.35;
pub const MAX_SPEED: f32 = 6.0;
pub const MAX_ACCEL: f32 = 24.0;

#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Reflect)]
pub struct Player;

#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Reflect)]
pub struct Floor;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Reflect, Serialize, Deserialize)]
pub enum PlayerAction {
    Move,
}

impl Actionlike for PlayerAction {
    fn input_control_kind(&self) -> InputControlKind {
        InputControlKind::DualAxis
    }
}

#[derive(Bundle)]
pub struct PlayerPhysicsBundle {
    pub collider: Collider,
    pub body: RigidBody,
    pub locked_axes: LockedAxes,
    pub friction: Friction,
    pub restitution: Restitution,
    pub damping: LinearDamping,
}

impl Default for PlayerPhysicsBundle {
    fn default() -> Self {
        Self {
            collider: Collider::sphere(BALL_RADIUS),
            body: RigidBody::Dynamic,
            locked_axes: LockedAxes::default()
                .lock_rotation_x()
                .lock_rotation_y()
                .lock_rotation_z(),
            friction: Friction::new(0.0),
            restitution: Restitution::new(0.2),
            damping: LinearDamping(0.1),
        }
    }
}

#[derive(Bundle)]
pub struct FloorPhysicsBundle {
    pub collider: Collider,
    pub body: RigidBody,
    pub friction: Friction,
    pub restitution: Restitution,
}

impl Default for FloorPhysicsBundle {
    fn default() -> Self {
        Self {
            collider: Collider::cuboid(50.0, 0.5, 50.0),
            body: RigidBody::Static,
            friction: Friction::new(0.0),
            restitution: Restitution::new(0.2),
        }
    }
}

#[derive(Clone)]
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(leafwing::InputPlugin::<PlayerAction> {
            config: InputConfig::<PlayerAction> {
                rebroadcast_inputs: true,
                ..default()
            },
        });
        app.component::<Player>().replicate();
        app.component::<Floor>().replicate();
    }
}

#[derive(Clone)]
pub struct SharedPlugin;

impl Plugin for SharedPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ProtocolPlugin);
        app.add_plugins(LightyearAvianPlugin {
            replication_mode: AvianReplicationMode::Position {
                sync_to_transform: false,
            },
            ..default()
        });
        app.add_plugins(
            PhysicsPlugins::default()
                .build()
                .disable::<PhysicsTransformPlugin>()
                .disable::<PhysicsInterpolationPlugin>()
                .disable::<IslandPlugin>()
                .disable::<IslandSleepingPlugin>(),
        );
    }
}

pub fn apply_player_input(
    time: Res<Time<Fixed>>,
    mut query: Query<(&ActionState<PlayerAction>, &mut LinearVelocity), With<Player>>,
) {
    let dt = time.delta_secs();
    for (state, mut velocity) in &mut query {
        let axis = state.axis_pair(&PlayerAction::Move).clamp_length_max(1.0);
        let desired = Vec3::new(axis.x, 0.0, -axis.y) * MAX_SPEED;
        let flat = Vec3::new(velocity.0.x, 0.0, velocity.0.z)
            .move_towards(desired, MAX_ACCEL * dt);
        velocity.0.x = flat.x;
        velocity.0.z = flat.z;
    }
}

pub fn base_app(loop_wait: Option<Duration>) -> App {
    let mut app = App::new();
    let minimal = match loop_wait {
        Some(wait) => MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(wait)),
        None => MinimalPlugins.build(),
    };
    app.add_plugins((
        minimal,
        TransformPlugin,
        bevy::input::InputPlugin,
        bevy::state::app::StatesPlugin,
        bevy::diagnostic::DiagnosticsPlugin,
    ));
    app
}
