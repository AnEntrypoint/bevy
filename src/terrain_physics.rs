#![cfg_attr(not(test), allow(dead_code))]

use std::collections::HashMap;

pub const AIRBORNE_GROUND_REUSE_SAFETY: f64 = 8.0;
pub const AIRBORNE_GROUND_RELEASE_MIN_M: f64 = 1.0;
pub const AIRBORNE_GROUND_RELEASE_TICKS: f64 = 3.0;
pub const AIRBORNE_GROUND_RELEASE_HZ: f64 = 60.0;
const AIRBORNE_GROUND_MIN_DESCENT_MPS: f64 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AirbornePlayer {
    pub key: u64,
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    pub on_ground: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReuseStats {
    pub served: u64,
    pub exact: u64,
    pub reused: u64,
    pub grounded_served: u64,
    pub grounded_exact: u64,
    pub max_reuse_m: f64,
    pub sum_reuse_m: f64,
    pub max_reuse_ratio: f64,
    pub limit_min_m: f64,
    pub limit_max_m: f64,
    pub verify_samples: u64,
    pub max_served_vs_exact_m: f64,
    pub sum_served_vs_exact_m: f64,
    pub verify_exact_solves: u64,
    pub min_above_m: f64,
    pub max_ratio_to_derived: f64,
}

impl Default for ReuseStats {
    fn default() -> Self {
        Self {
            served: 0,
            exact: 0,
            reused: 0,
            grounded_served: 0,
            grounded_exact: 0,
            max_reuse_m: 0.0,
            sum_reuse_m: 0.0,
            max_reuse_ratio: 0.0,
            limit_min_m: f64::INFINITY,
            limit_max_m: 0.0,
            verify_samples: 0,
            max_served_vs_exact_m: 0.0,
            sum_served_vs_exact_m: 0.0,
            verify_exact_solves: 0,
            min_above_m: f64::INFINITY,
            max_ratio_to_derived: 0.0,
        }
    }
}

#[derive(Clone, Copy)]
struct Cached {
    x: f64,
    z: f64,
    y: f64,
    epoch: u64,
}

pub struct AirborneGroundReuse {
    cache: HashMap<u64, Cached>,
    stats: ReuseStats,
    enabled: bool,
    verify_reuse: bool,
    distance_override_m: Option<f64>,
}

fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() { f64::NAN } else { a.max(b) }
}

fn release_clearance_m(p: &AirbornePlayer) -> f64 {
    let descent = if p.velocity[1].is_finite() { p.velocity[1].abs() } else { 0.0 };
    js_max(AIRBORNE_GROUND_RELEASE_MIN_M, descent * AIRBORNE_GROUND_RELEASE_TICKS / AIRBORNE_GROUND_RELEASE_HZ)
}

fn derived_reuse_m(p: &AirbornePlayer, ground_y: f64) -> f64 {
    let above = p.position[1] - ground_y;
    if !above.is_finite() || above <= 0.0 {
        return 0.0;
    }
    let descent = js_max(p.velocity[1].abs(), AIRBORNE_GROUND_MIN_DESCENT_MPS);
    let horizontal = p.velocity[0].hypot(p.velocity[2]);
    horizontal * (above / descent) / AIRBORNE_GROUND_REUSE_SAFETY
}

fn ratio_of(part: f64, whole: f64, moved: f64) -> f64 {
    if whole > 0.0 {
        part / whole
    } else if moved > 0.0 {
        f64::INFINITY
    } else {
        0.0
    }
}

impl AirborneGroundReuse {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            stats: ReuseStats::default(),
            enabled: true,
            verify_reuse: false,
            distance_override_m: None,
        }
    }

    pub fn stats(&self) -> ReuseStats {
        self.stats
    }

    pub fn reset_stats(&mut self) {
        self.stats = ReuseStats::default();
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn set_verify_reuse(&mut self, verify: bool) {
        self.verify_reuse = verify;
    }

    pub fn set_distance_override_m(&mut self, metres: Option<f64>) {
        self.distance_override_m = metres;
    }

    fn solve(&mut self, x: f64, z: f64, height: &mut dyn FnMut(f64, f64) -> f64) -> f64 {
        self.stats.exact += 1;
        height(x, z)
    }

    pub fn height_at(&mut self, players: &[AirbornePlayer], epoch: u64, x: f64, z: f64, height: &mut dyn FnMut(f64, f64) -> f64) -> f64 {
        self.stats.served += 1;
        let found = if self.enabled {
            players.iter().find(|p| p.position[0] == x && p.position[2] == z).copied()
        } else {
            None
        };
        let Some(p) = found else {
            return self.solve(x, z, height);
        };
        if p.on_ground {
            self.stats.grounded_served += 1;
            self.stats.grounded_exact += 1;
            return self.solve(x, z, height);
        }
        if let Some(entry) = self.cache.get(&p.key).copied() {
            if entry.epoch == epoch && entry.y.is_finite() {
                let above = p.position[1] - entry.y;
                let derived_limit = derived_reuse_m(&p, entry.y);
                let limit = self.distance_override_m.unwrap_or(derived_limit);
                let moved = (x - entry.x).hypot(z - entry.z);
                let clearance = if self.distance_override_m.is_none() { release_clearance_m(&p) } else { 0.0 };
                if moved <= limit && above > clearance {
                    self.stats.reused += 1;
                    self.stats.sum_reuse_m += moved;
                    let ratio_to_derived = ratio_of(moved, derived_limit, moved);
                    if ratio_to_derived > self.stats.max_ratio_to_derived {
                        self.stats.max_ratio_to_derived = ratio_to_derived;
                    }
                    if moved > self.stats.max_reuse_m {
                        self.stats.max_reuse_m = moved;
                    }
                    let ratio = ratio_of(moved, limit, moved);
                    if ratio > self.stats.max_reuse_ratio {
                        self.stats.max_reuse_ratio = ratio;
                    }
                    if limit < self.stats.limit_min_m {
                        self.stats.limit_min_m = limit;
                    }
                    if limit > self.stats.limit_max_m {
                        self.stats.limit_max_m = limit;
                    }
                    if above.is_finite() && above < self.stats.min_above_m {
                        self.stats.min_above_m = above;
                    }
                    if self.verify_reuse {
                        let exact_y = height(x, z);
                        self.stats.verify_exact_solves += 1;
                        self.stats.verify_samples += 1;
                        let delta = (entry.y - exact_y).abs();
                        self.stats.sum_served_vs_exact_m += delta;
                        if delta > self.stats.max_served_vs_exact_m {
                            self.stats.max_served_vs_exact_m = delta;
                        }
                    }
                    return entry.y;
                }
            }
        }
        let y = self.solve(x, z, height);
        self.cache.insert(p.key, Cached { x, z, y, epoch });
        y
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn num(v: &Value) -> f64 {
        match v.as_str() {
            Some("Infinity") => f64::INFINITY,
            Some("-Infinity") => f64::NEG_INFINITY,
            _ => v.as_f64().unwrap(),
        }
    }

    fn vec3(v: &Value) -> [f64; 3] {
        let a = v.as_array().unwrap();
        [num(&a[0]), num(&a[1]), num(&a[2])]
    }

    fn close(got: f64, want: f64) -> bool {
        got == want || (got - want).abs() <= 1e-9
    }

    fn field_values(s: &ReuseStats) -> Vec<(&'static str, f64)> {
        vec![
            ("served", s.served as f64),
            ("exact", s.exact as f64),
            ("reused", s.reused as f64),
            ("groundedServed", s.grounded_served as f64),
            ("groundedExact", s.grounded_exact as f64),
            ("maxReuseM", s.max_reuse_m),
            ("sumReuseM", s.sum_reuse_m),
            ("maxReuseRatio", s.max_reuse_ratio),
            ("limitMinM", s.limit_min_m),
            ("limitMaxM", s.limit_max_m),
            ("verifySamples", s.verify_samples as f64),
            ("maxServedVsExactM", s.max_served_vs_exact_m),
            ("sumServedVsExactM", s.sum_served_vs_exact_m),
            ("verifyExactSolves", s.verify_exact_solves as f64),
            ("minAboveM", s.min_above_m),
            ("maxRatioToDerived", s.max_ratio_to_derived),
        ]
    }

    #[test]
    fn airborne_ground_reuse_matches_the_spoint_terrain_physics() {
        let fixture: Value = serde_json::from_str(include_str!("../testdata/parity/spoint-airborne-reuse.json")).unwrap();
        let ops = fixture["ops"].as_array().unwrap();
        let expect = fixture["expect"].as_array().unwrap();
        let mut reuse = AirborneGroundReuse::new();
        let mut players: Vec<(String, AirbornePlayer)> = Vec::new();
        let mut epoch = 0_u64;
        let mut reused_seen = 0_u64;
        for (i, op) in ops.iter().enumerate() {
            let kind = op["op"].as_str().unwrap();
            let mut y_got = None;
            match kind {
                "add" => {
                    let id = op["id"].as_str().unwrap();
                    let p = AirbornePlayer {
                        key: op["key"].as_u64().unwrap(),
                        position: vec3(&op["position"]),
                        velocity: vec3(&op["velocity"]),
                        on_ground: op["onGround"].as_bool().unwrap(),
                    };
                    match players.iter().position(|(k, _)| k.as_str() == id) {
                        Some(at) => players[at] = (id.to_string(), p),
                        None => players.push((id.to_string(), p)),
                    }
                }
                "move" => {
                    let id = op["id"].as_str().unwrap();
                    let at = players.iter().position(|(k, _)| k.as_str() == id).unwrap();
                    let p = &mut players[at].1;
                    p.position = vec3(&op["position"]);
                    p.velocity = vec3(&op["velocity"]);
                    p.on_ground = op["onGround"].as_bool().unwrap();
                }
                "remove" => {
                    let id = op["id"].as_str().unwrap();
                    let at = players.iter().position(|(k, _)| k.as_str() == id).unwrap();
                    players.remove(at);
                }
                "height" => {
                    let list: Vec<AirbornePlayer> = players.iter().map(|(_, p)| *p).collect();
                    let mut height = |x: f64, z: f64| 3.0 + 0.25 * x - 0.1 * z;
                    y_got = Some(reuse.height_at(&list, epoch, num(&op["x"]), num(&op["z"]), &mut height));
                }
                "epoch" => epoch = op["value"].as_u64().unwrap(),
                "enabled" => reuse.set_enabled(op["value"].as_bool().unwrap()),
                "verify" => reuse.set_verify_reuse(op["value"].as_bool().unwrap()),
                "override" => reuse.set_distance_override_m(if op["value"].is_null() { None } else { Some(num(&op["value"])) }),
                "reset" => reuse.reset_stats(),
                other => panic!("unknown op {other}"),
            }
            let want = &expect[i];
            if let Some(y) = y_got {
                let w = num(&want["y"]);
                assert!(close(y, w), "op {i} height {y} against spoint {w}");
            }
            for (name, got) in field_values(&reuse.stats()) {
                let w = num(&want["stats"][name]);
                assert!(close(got, w), "op {i} {kind} stat {name}: bevy {got} against spoint {w}");
            }
            reused_seen = reused_seen.max(reuse.stats().reused);
        }
        assert!(reused_seen > 10, "the scripted path should exercise reuse, saw {reused_seen}");
    }
}
