use super::net::{next_message, RtsMessage, UnitState};
use super::wire::{RtsChannel, RtsWirePlugin};
use super::{RtsHashes, RtsOrders, RtsSim, RtsSimPlugin, Order};
use super::script;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use lightyear::prelude::client::*;
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::time::Duration;

const IMAGE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/rts/duel-routes.osdt");
const ORDERS: &str = include_str!("../../testdata/rts/duel-routes.orders");
const TICK: Duration = Duration::from_nanos(62_500_000);
const TICKS: usize = 16 * 8;

fn address(port: u16) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)
}

#[derive(Resource, Default)]
struct Received(Vec<(u32, u64)>);

fn orders() -> Vec<(u32, Order)> {
    script::parse(ORDERS).expect("orders")
}

fn base(orders: Vec<(u32, Order)>) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin, ServerPlugins { tick_duration: TICK }))
        .add_plugins((RtsWirePlugin, RtsSimPlugin { image: PathBuf::from(IMAGE), rules: PathBuf::from(crate::rts::RULES) }))
        .insert_resource(RtsOrders::new(orders))
        .insert_resource(TimeUpdateStrategy::ManualDuration(TICK));
    app
}

fn server() -> App {
    let mut app = base(orders());
    app.add_systems(Startup, start_server)
        .add_systems(FixedUpdate, broadcast.after(super::step_rts_sim));
    app
}

fn start_server(mut commands: Commands) {
    let server = commands
        .spawn((RawServer, LocalAddr(address(5100)), ServerUdpIo::default()))
        .id();
    commands.trigger(Start { entity: server });
}

fn broadcast(
    rts: Res<RtsSim>,
    mut previous: Local<Vec<UnitState>>,
    mut sender: ServerMultiMessageSender,
    servers: Query<&Endpoint, With<Server>>,
) {
    let message = next_message(&rts.0, &mut previous, None);
    for endpoint in &servers {
        sender
            .send::<RtsMessage, RtsChannel>(&message, endpoint, &NetworkTarget::All)
            .expect("broadcast");
    }
}

fn client(port: u16) -> App {
    let mut app = base(orders());
    app.add_plugins(ClientPlugins { tick_duration: TICK })
        .insert_resource(PredictionManager::default())
        .init_resource::<Received>()
        .add_observer(|trigger: On<RemoteEvent<SenderMetadata>>| eprintln!("SENDER_METADATA {:?}", trigger.trigger))
        .add_systems(Startup, move |mut commands: Commands| {
            let client = commands
                .spawn((
                    Client,
                    LocalAddr(address(port)),
                    PeerAddr(address(5100)),
                    Link::default(),
                    RawClient,
                    UdpIo::default(),
                ))
                .id();
            commands.trigger(Connect { entity: client });
        })
        .add_systems(Update, receive);
    app
}

fn receive(mut receivers: Query<&mut MessageReceiver<RtsMessage>>, mut received: ResMut<Received>) {
    for mut receiver in &mut receivers {
        for message in receiver.receive() {
            if let RtsMessage::Snapshot { tick, hash, .. } | RtsMessage::Delta { tick, hash, .. } = message {
                received.0.push((tick, hash));
            }
        }
    }
}

fn pump(apps: &mut [&mut App], updates: usize) {
    for _ in 0..updates {
        for app in apps.iter_mut() {
            app.update();
        }
    }
}

#[test]
fn two_loopback_clients_match_the_server_hash_every_tick() {
    let mut server = server();
    let mut first = client(5101);
    let mut second = client(5102);
    for app in [&mut server, &mut first, &mut second] {
        app.finish();
        app.cleanup();
    }
    pump(&mut [&mut server, &mut first, &mut second], TICKS + 64);
    let server_hashes = server.world().resource::<RtsHashes>().0.clone();
    assert_eq!(server_hashes.len(), TICKS + 63, "the first update only starts the fixed clock");
    for client in [&first, &second] {
        let local = &client.world().resource::<RtsHashes>().0;
        assert_eq!(local.as_slice(), server_hashes.as_slice(), "local sim differs from the server");
        let received = &client.world().resource::<Received>().0;
        assert!(received.len() >= TICKS, "only {} messages arrived", received.len());
        for (tick, hash) in received {
            let expected = server_hashes.iter().find(|(t, _)| t == tick).map(|(_, h)| *h);
            assert_eq!(Some(*hash), expected, "desync at tick {tick}");
        }
    }
}
