#![cfg_attr(not(test), allow(dead_code))]

use std::collections::{BTreeMap, BTreeSet};

pub const DEFAULT_CAPSULE_RADIUS: f64 = 0.4;
const SPAWN_POINT: [f64; 3] = [0.0, 5.0, 0.0];

pub trait CharacterWorld {
    fn add_character(&mut self, radius: f64, half_height: f64, start: [f64; 3], mass: f64) -> u64;
    fn remove_character(&mut self, id: u64);
    fn set_character_position(&mut self, id: u64, position: [f64; 3]);
    fn character_position(&self, id: u64) -> [f64; 3];
    fn character_radius(&self, id: u64) -> f64;
    fn character_count(&self) -> usize;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerBody {
    pub character: Option<u64>,
    pub on_ground: bool,
}

pub struct PlayerBodies<W: CharacterWorld> {
    world: Option<W>,
    bodies: BTreeMap<u64, PlayerBody>,
    crouch: BTreeSet<u64>,
    half_height: f64,
    mass: f64,
}

impl<W: CharacterWorld> PlayerBodies<W> {
    pub fn new(world: Option<W>, half_height: f64, mass: f64) -> Self {
        Self { world, bodies: BTreeMap::new(), crouch: BTreeSet::new(), half_height, mass }
    }

    pub fn add_player_collider(&mut self, player: u64, radius: f64) {
        let radius = if radius.is_finite() && radius > 0.0 { radius } else { DEFAULT_CAPSULE_RADIUS };
        if self.bodies.contains_key(&player) {
            self.remove_player_collider(player);
        }
        match self.world.as_mut() {
            Some(world) => {
                let id = world.add_character(radius, self.half_height, SPAWN_POINT, self.mass);
                self.bodies.insert(player, PlayerBody { character: Some(id), on_ground: false });
                self.crouch.insert(player);
            }
            None => {
                self.bodies.insert(player, PlayerBody { character: None, on_ground: false });
            }
        }
    }

    pub fn remove_player_collider(&mut self, player: u64) {
        if let Some(body) = self.bodies.remove(&player) {
            if let (Some(id), Some(world)) = (body.character, self.world.as_mut()) {
                world.remove_character(id);
            }
        }
        self.crouch.remove(&player);
    }

    pub fn set_player_position(&mut self, player: u64, position: [f64; 3]) {
        let character = self.bodies.get(&player).and_then(|body| body.character);
        if let (Some(id), Some(world)) = (character, self.world.as_mut()) {
            world.set_character_position(id, position);
        }
    }

    pub fn player_position(&self, player: u64) -> [f64; 3] {
        let character = self.bodies.get(&player).and_then(|body| body.character);
        match (character, self.world.as_ref()) {
            (Some(id), Some(world)) => world.character_position(id),
            _ => [0.0; 3],
        }
    }

    pub fn player_radius(&self, player: u64) -> Option<f64> {
        let id = self.bodies.get(&player)?.character?;
        Some(self.world.as_ref()?.character_radius(id))
    }

    pub fn bodies(&self) -> &BTreeMap<u64, PlayerBody> {
        &self.bodies
    }

    pub fn crouch_keys(&self) -> Vec<u64> {
        self.crouch.iter().copied().collect()
    }

    pub fn character_count(&self) -> usize {
        self.world.as_ref().map_or(0, |world| world.character_count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[derive(Default)]
    struct FakeWorld {
        next: u64,
        live: BTreeMap<u64, (f64, [f64; 3])>,
    }

    impl CharacterWorld for FakeWorld {
        fn add_character(&mut self, radius: f64, _half_height: f64, start: [f64; 3], _mass: f64) -> u64 {
            self.next += 1;
            self.live.insert(self.next, (radius, start));
            self.next
        }

        fn remove_character(&mut self, id: u64) {
            self.live.remove(&id);
        }

        fn set_character_position(&mut self, id: u64, position: [f64; 3]) {
            if let Some(entry) = self.live.get_mut(&id) {
                entry.1 = position;
            }
        }

        fn character_position(&self, id: u64) -> [f64; 3] {
            self.live[&id].1
        }

        fn character_radius(&self, id: u64) -> f64 {
            self.live[&id].0
        }

        fn character_count(&self) -> usize {
            self.live.len()
        }
    }

    fn id(v: &Value) -> u64 {
        v.as_u64().unwrap()
    }

    fn vec3(v: &Value) -> [f64; 3] {
        let a = v.as_array().unwrap();
        [a[0].as_f64().unwrap(), a[1].as_f64().unwrap(), a[2].as_f64().unwrap()]
    }

    fn apply(bodies: &mut PlayerBodies<FakeWorld>, message: &Value) {
        match message["type"].as_str().unwrap() {
            "spawn" => {
                let player = id(&message["playerId"]);
                bodies.add_player_collider(player, message["radius"].as_f64().unwrap());
                bodies.set_player_position(player, vec3(&message["position"]));
            }
            "reconnect" => {
                let (old, new) = (id(&message["oldId"]), id(&message["newId"]));
                if old != new {
                    bodies.remove_player_collider(old);
                }
                bodies.add_player_collider(new, message["radius"].as_f64().unwrap());
                bodies.set_player_position(new, vec3(&message["position"]));
            }
            "leave" => bodies.remove_player_collider(id(&message["playerId"])),
            other => panic!("unknown message {other}"),
        }
    }

    #[test]
    fn player_bodies_match_spoint_over_recorded_messages() {
        let fixture: Value = serde_json::from_str(include_str!("../testdata/parity/spoint-player-bodies.json")).unwrap();
        let mass = fixture["config"]["playerMass"].as_f64().unwrap();
        let messages = fixture["messages"].as_array().unwrap();
        for scenario in fixture["scenarios"].as_array().unwrap() {
            let name = scenario["name"].as_str().unwrap();
            let half_height = scenario["physicalHalfHeight"].as_f64().unwrap();
            let world = (name == "world").then(FakeWorld::default);
            let mut bodies = PlayerBodies::new(world, half_height, mass);
            for (message, want) in messages.iter().zip(scenario["steps"].as_array().unwrap()) {
                apply(&mut bodies, message);
                let mut want_bodies: Vec<&Value> = want["bodies"].as_array().unwrap().iter().collect();
                want_bodies.sort_by_key(|b| id(&b["id"]));
                assert_eq!(bodies.bodies().len(), want_bodies.len(), "{name}: body count");
                for (b, w) in bodies.bodies().iter().zip(want_bodies) {
                    assert_eq!(*b.0, id(&w["id"]), "{name}: body id");
                    assert_eq!(b.1.character.is_some(), w["hasCharacter"].as_bool().unwrap(), "{name}: hasCharacter");
                    assert_eq!(b.1.on_ground, w["onGround"].as_bool().unwrap(), "{name}: onGround");
                    if let Some(radius) = w["radius"].as_f64() {
                        assert_eq!(bodies.player_radius(*b.0), Some(radius), "{name}: radius of {}", b.0);
                    }
                }
                for entry in want["positions"].as_array().unwrap() {
                    let got = bodies.player_position(id(&entry["id"]));
                    let p = vec3(&entry["p"]);
                    assert!((0..3).all(|k| (got[k] - p[k]).abs() <= 1e-6), "{name}: position {got:?} against spoint {p:?}");
                }
                let mut crouch: Vec<u64> = want["crouch"].as_array().unwrap().iter().map(id).collect();
                crouch.sort_unstable();
                assert_eq!(bodies.crouch_keys(), crouch, "{name}: crouch keys");
                assert_eq!(bodies.character_count(), want["characterCount"].as_u64().unwrap() as usize, "{name}: character count");
            }
        }
    }
}
