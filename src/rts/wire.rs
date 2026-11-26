use super::net::RtsMessage;
use bevy::prelude::*;
use lightyear::prelude::*;
use std::time::Duration;

pub struct RtsChannel;

pub struct RtsWirePlugin;

impl Plugin for RtsWirePlugin {
    fn build(&self, app: &mut App) {
        app.add_channel::<RtsChannel>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            send_frequency: Duration::default(),
            priority: 1.0,
            ..Default::default()
        })
        .add_direction(NetworkDirection::Bidirectional);
        app.register_message::<RtsMessage>()
            .add_direction(NetworkDirection::ServerToClient);
    }
}
