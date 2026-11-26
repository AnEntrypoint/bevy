use super::Order;
use serde::{Deserialize, Serialize};
use sim::Sim;

pub const SNAPSHOT_EVERY: u32 = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitState {
    pub unit: u32,
    pub x: i32,
    pub y: i32,
    pub alive: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderWire {
    Order { unit: u32, x: i32, y: i32 },
    OrderAll { player: u8, x: i32, y: i32 },
}

impl OrderWire {
    pub fn command(self) -> Order {
        match self {
            OrderWire::Order { unit, x, y } => Order::Order { unit: unit as usize, x, y },
            OrderWire::OrderAll { player, x, y } => Order::OrderAll { player, x, y },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RtsMessage {
    Orders { tick: u32, orders: Vec<OrderWire> },
    Snapshot { tick: u32, hash: u64, units: Vec<UnitState> },
    Delta { tick: u32, hash: u64, changed: Vec<UnitState> },
}

#[derive(Debug, PartialEq, Eq)]
pub enum StreamError {
    Gap { expected: u32, got: u32 },
}

fn units(sim: &Sim) -> Vec<UnitState> {
    (0..sim.count())
        .map(|i| {
            let (x, y) = sim.position_fixed(i);
            UnitState { unit: i as u32, x, y, alive: sim.alive(i) }
        })
        .collect()
}

fn changed(previous: &[UnitState], now: &[UnitState]) -> Vec<UnitState> {
    now.iter().enumerate().filter(|(i, state)| previous.get(*i) != Some(state)).map(|(_, state)| *state).collect()
}

pub fn next_message(sim: &Sim, previous: &mut Vec<UnitState>) -> RtsMessage {
    let tick = sim.ticks();
    let hash = sim.state_hash();
    let now = units(sim);
    let message = if tick % SNAPSHOT_EVERY == 0 {
        RtsMessage::Snapshot { tick, hash, units: now.clone() }
    } else {
        RtsMessage::Delta { tick, hash, changed: changed(previous, &now) }
    };
    *previous = now;
    message
}

pub fn server_stream(mut sim: Sim, orders: &[(u32, OrderWire)], ticks: u32) -> Vec<RtsMessage> {
    let mut previous: Vec<UnitState> = Vec::new();
    let mut next = 0;
    let mut messages = Vec::new();
    for _ in 0..ticks {
        let tick = sim.ticks();
        let mut due = Vec::new();
        while next < orders.len() && orders[next].0 <= tick {
            due.push(orders[next].1);
            orders[next].1.command().apply(&mut sim);
            next += 1;
        }
        if !due.is_empty() {
            messages.push(RtsMessage::Orders { tick, orders: due });
        }
        sim.tick();
        messages.push(next_message(&sim, &mut previous));
    }
    messages
}

#[derive(Default)]
pub struct Mirror {
    pub tick: u32,
    pub hash: u64,
    pub units: Vec<UnitState>,
}

impl Mirror {
    pub fn accept(&mut self, message: &RtsMessage) -> Result<(), StreamError> {
        match message {
            RtsMessage::Orders { .. } => Ok(()),
            RtsMessage::Snapshot { tick, hash, units } => {
                self.tick = *tick;
                self.hash = *hash;
                self.units = units.clone();
                Ok(())
            }
            RtsMessage::Delta { tick, hash, changed } => {
                if *tick != self.tick + 1 {
                    return Err(StreamError::Gap { expected: self.tick + 1, got: *tick });
                }
                for state in changed {
                    let slot = state.unit as usize;
                    if slot >= self.units.len() {
                        self.units.resize(slot + 1, *state);
                    }
                    self.units[slot] = *state;
                }
                self.tick = *tick;
                self.hash = *hash;
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rts::load_image;
    use sim::fixed::from_int;
    use std::path::Path;

    const IMAGE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/rts/duel-routes.osdt");

    fn centre_orders(sim: &Sim) -> Vec<(u32, OrderWire)> {
        let grid = sim.grid();
        let (x, y) = (from_int((grid.w / 2) as i32), from_int((grid.h / 2) as i32));
        vec![(0, OrderWire::OrderAll { player: 0, x, y }), (0, OrderWire::OrderAll { player: 1, x, y })]
    }

    fn stream(ticks: u32) -> Vec<RtsMessage> {
        let sim = load_image(Path::new(IMAGE));
        let orders = centre_orders(&sim);
        server_stream(sim, &orders, ticks)
    }

    #[test]
    fn mirror_tracks_the_server_on_every_message() {
        let messages = stream(200);
        let mut mirror = Mirror::default();
        for message in &messages {
            mirror.accept(message).expect("no gap");
        }
        let last = messages.last().expect("a stream");
        assert!(matches!(last, RtsMessage::Snapshot { tick: 200, .. }));
        assert_eq!(mirror.tick, 200);
        if let RtsMessage::Snapshot { hash, units, .. } = last {
            assert_eq!(mirror.hash, *hash);
            assert_eq!(&mirror.units, units);
        }
    }

    #[test]
    fn red_a_dropped_delta_is_named_by_its_tick() {
        let mut messages = stream(200);
        messages.retain(|message| !matches!(message, RtsMessage::Delta { tick: 101, .. }));
        let mut mirror = Mirror::default();
        let error = messages.iter().find_map(|message| mirror.accept(message).err()).expect("a gap");
        assert_eq!(error, StreamError::Gap { expected: 101, got: 102 });
    }
}

#[cfg(test)]
mod bytes {
    use super::{server_stream, OrderWire, RtsMessage};
    use crate::rts::{load_image, script, Order};
    use std::path::Path;

    const IMAGE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/rts/duel-routes.osdt");
    const ORDERS: &str = include_str!("../../testdata/rts/oscraft-duel-routes.orders");
    const TICKS: u32 = 1280;
    const TICKS_PER_SECOND: f64 = 16.0;

    #[test]
    fn duel_routes_stream_bytes_per_second() {
        let orders: Vec<(u32, OrderWire)> = script::parse(ORDERS)
            .expect("orders")
            .into_iter()
            .map(|(tick, order)| {
                let wire = match order {
                    Order::Order { unit, x, y } => OrderWire::Order { unit: unit as u32, x, y },
                    Order::OrderAll { player, x, y } => OrderWire::OrderAll { player, x, y },
                };
                (tick, wire)
            })
            .collect();
        let messages: Vec<RtsMessage> = server_stream(load_image(Path::new(IMAGE)), &orders, TICKS);
        let bytes: usize = messages.iter().map(|message| serde_json::to_vec(message).expect("encode").len()).sum();
        let seconds = f64::from(TICKS) / TICKS_PER_SECOND;
        eprintln!("S5_BYTES_PER_SECOND {:.0} over {} messages ({} bytes in {} s)", bytes as f64 / seconds, messages.len(), bytes, seconds);
        assert!(bytes > 0);
    }
}
