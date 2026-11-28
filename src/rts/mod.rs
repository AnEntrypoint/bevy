pub mod net;
pub mod wire;
#[cfg(test)]
mod loopback;
#[cfg(test)]
pub mod client;
pub mod script;
#[cfg(test)]
mod witness;

use bevy::prelude::*;
use sim::Sim;
use std::path::{Path, PathBuf};

pub const RTS_TICKS_PER_SECOND: f64 = 16.0;
pub const RTS_MAX_UNITS: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    Order { unit: usize, x: i32, y: i32 },
    OrderAll { player: u8, x: i32, y: i32 },
}

impl Order {
    pub fn apply(&self, sim: &mut Sim) {
        match *self {
            Order::Order { unit, x, y } => sim.order(unit, x, y),
            Order::OrderAll { player, x, y } => sim.order_all(player, x, y),
        }
    }
}

#[derive(Resource)]
pub struct RtsSim(pub Sim);

#[derive(Resource, Default)]
pub struct RtsOrders {
    next: usize,
    list: Vec<(u32, Order)>,
}

impl RtsOrders {
    pub fn new(list: Vec<(u32, Order)>) -> RtsOrders {
        RtsOrders { next: 0, list }
    }
}

#[derive(Resource, Default)]
pub struct RtsHashes(pub Vec<(u32, u64)>);

pub struct RtsSimPlugin {
    pub image: PathBuf,
}

impl Plugin for RtsSimPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_hz(RTS_TICKS_PER_SECOND))
            .insert_resource(RtsSim(load_image(&self.image)))
            .init_resource::<RtsOrders>()
            .init_resource::<RtsHashes>()
            .add_systems(FixedUpdate, step_rts_sim);
    }
}

pub fn load_image(image: &Path) -> Sim {
    sim::load_file(image, RTS_MAX_UNITS).unwrap_or_else(|error| panic!("rts image {}: {error:?}", image.display()))
}

pub(crate) fn step_rts_sim(mut rts: ResMut<RtsSim>, mut orders: ResMut<RtsOrders>, mut hashes: ResMut<RtsHashes>) {
    let tick = rts.0.ticks();
    let orders = &mut *orders;
    while let Some((at, order)) = orders.list.get(orders.next) {
        if *at > tick {
            break;
        }
        order.apply(&mut rts.0);
        orders.next += 1;
    }
    rts.0.tick();
    hashes.0.push((rts.0.ticks(), rts.0.state_hash()));
}

#[cfg(test)]
pub fn native_hashes(mut sim: Sim, orders: &[(u32, Order)], ticks: usize) -> Vec<(u32, u64)> {
    let mut next = 0;
    let mut hashes = Vec::with_capacity(ticks);
    for _ in 0..ticks {
        let tick = sim.ticks();
        while next < orders.len() && orders[next].0 <= tick {
            orders[next].1.apply(&mut sim);
            next += 1;
        }
        sim.tick();
        hashes.push((sim.ticks(), sim.state_hash()));
    }
    hashes
}
