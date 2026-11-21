#![cfg_attr(not(test), allow(dead_code))]

use std::fmt;

use crate::height::ops_jsgen::V3;
use crate::hitscan::{Hitbox, DEFAULT_HITBOX};

const RESPAWN_LIFT_M: f64 = 2.0;
const MAX_AUTHORED_SURFACE_DELTA_M: f64 = 2.0;
const FALLBACK_CAPSULE_RADIUS_M: f64 = 0.6;

#[derive(Clone, Debug, PartialEq)]
pub enum BlockerKind {
    NoSurface,
    SurfaceMismatch,
    Intruding,
    Void,
    Player(u32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Player {
    pub id: u32,
    pub position: Option<V3>,
}

pub trait SpawnWorld {
    fn terrain_height_at(&self, x: f64, z: f64) -> f64;
    fn spawn_surface_y(&self, sp: V3, standing_offset: f64, headroom: f64, terrain_y: f64, radius: f64) -> Option<f64>;
    fn footprint_blockers(&self, sp: V3, feet_y: f64, hitbox: &Hitbox) -> Vec<BlockerKind>;
    fn players(&self) -> &[Player];
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpawnEvaluation {
    pub feet_y: Option<f64>,
    pub pose: V3,
    pub blockers: Vec<BlockerKind>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpawnPick {
    pub pose: V3,
    pub all_blocked: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpawnRefusal {
    pub supplied: usize,
    pub usable: usize,
}

impl fmt::Display for SpawnRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[tps-game] respawn clearance: {} spawn point(s) supplied, {} usable", self.supplied, self.usable)
    }
}

fn capsule_radius_m(hitbox: &Hitbox) -> f64 {
    if hitbox.radius_sq.is_finite() && hitbox.radius_sq > 0.0 {
        hitbox.radius_sq.sqrt()
    } else {
        FALLBACK_CAPSULE_RADIUS_M
    }
}

fn capsule_height_m(hitbox: &Hitbox) -> f64 {
    if hitbox.height.is_finite() && hitbox.height > 0.0 {
        return hitbox.height;
    }
    let center = if hitbox.center_height.is_finite() { hitbox.center_height } else { DEFAULT_HITBOX.center_height };
    let derived = 2.0 * center;
    if derived > 0.0 {
        derived
    } else {
        DEFAULT_HITBOX.height
    }
}

fn standing_offset_m(hitbox: &Hitbox) -> f64 {
    if hitbox.center_height.is_finite() && hitbox.center_height > 0.0 {
        return hitbox.center_height;
    }
    let derived = capsule_height_m(hitbox) / 2.0;
    if derived > 0.0 {
        derived
    } else {
        DEFAULT_HITBOX.center_height
    }
}

fn is_finite_point(sp: &V3) -> bool {
    sp.iter().all(|c| c.is_finite())
}

fn player_overlaps<W: SpawnWorld>(world: &W, sp: V3, feet_y: f64, hitbox: &Hitbox, exclude: &dyn Fn(u32) -> bool) -> Vec<BlockerKind> {
    let diameter = 2.0 * capsule_radius_m(hitbox);
    let height = capsule_height_m(hitbox);
    world
        .players()
        .iter()
        .filter_map(|player| {
            let position = player.position?;
            if exclude(player.id) {
                return None;
            }
            let distance = (sp[0] - position[0]).hypot(sp[2] - position[2]);
            if distance >= diameter || (position[1] - feet_y).abs() >= height {
                return None;
            }
            Some(BlockerKind::Player(player.id))
        })
        .collect()
}

pub fn evaluate_spawn_point<W: SpawnWorld>(world: &W, sp: V3, hitbox: &Hitbox, exclude: &dyn Fn(u32) -> bool) -> SpawnEvaluation {
    let radius = capsule_radius_m(hitbox);
    let standing_offset = standing_offset_m(hitbox);
    let terrain_y = world.terrain_height_at(sp[0], sp[2]);
    let pose_y = if terrain_y.is_finite() { sp[1].max(terrain_y) } else { sp[1] };
    let headroom = RESPAWN_LIFT_M + capsule_height_m(hitbox);
    let Some(feet_y) = world.spawn_surface_y(sp, standing_offset, headroom, terrain_y, radius) else {
        return SpawnEvaluation { feet_y: None, pose: sp, blockers: vec![BlockerKind::NoSurface] };
    };
    let pose = [sp[0], feet_y + RESPAWN_LIFT_M, sp[2]];
    let delta = feet_y - sp[1];
    let band_low_y = pose_y - 2.0 * standing_offset;
    if feet_y < band_low_y || feet_y > pose_y || delta.abs() > MAX_AUTHORED_SURFACE_DELTA_M {
        return SpawnEvaluation { feet_y: Some(feet_y), pose, blockers: vec![BlockerKind::SurfaceMismatch] };
    }
    let mut blockers = world.footprint_blockers(sp, feet_y, hitbox);
    blockers.extend(player_overlaps(world, sp, feet_y, hitbox, exclude));
    SpawnEvaluation { feet_y: Some(feet_y), pose, blockers }
}

fn order_by_player_distance<W: SpawnWorld>(world: &W, spawn_points: &[V3], exclude: &dyn Fn(u32) -> bool, min_safe_distance: f64) -> Vec<V3> {
    let active: Vec<V3> = world.players().iter().filter(|p| !exclude(p.id)).filter_map(|p| p.position).collect();
    if active.is_empty() {
        return spawn_points.to_vec();
    }
    let mut scored: Vec<(V3, f64)> = spawn_points
        .iter()
        .map(|sp| {
            let nearest = active.iter().map(|o| (sp[0] - o[0]).hypot(sp[2] - o[2])).fold(f64::INFINITY, f64::min);
            (*sp, nearest)
        })
        .collect();
    let safe: Vec<V3> = scored.iter().filter(|(_, d)| *d >= min_safe_distance).map(|(sp, _)| *sp).collect();
    if !safe.is_empty() {
        return safe;
    }
    scored.sort_by(|a, b| b.1.total_cmp(&a.1));
    scored.into_iter().map(|(sp, _)| sp).collect()
}

pub fn pick_clear_spawn_pose<W: SpawnWorld>(world: &W, spawn_points: &[V3], exclude: &dyn Fn(u32) -> bool, min_safe_distance: f64, hitbox: &Hitbox) -> Result<SpawnPick, SpawnRefusal> {
    let usable: Vec<V3> = spawn_points.iter().copied().filter(is_finite_point).collect();
    if usable.is_empty() {
        return Err(SpawnRefusal { supplied: spawn_points.len(), usable: 0 });
    }
    let mut least: Option<SpawnEvaluation> = None;
    for sp in order_by_player_distance(world, &usable, exclude, min_safe_distance) {
        let evaluated = evaluate_spawn_point(world, sp, hitbox, exclude);
        if evaluated.blockers.is_empty() {
            return Ok(SpawnPick { pose: evaluated.pose, all_blocked: false });
        }
        if least.as_ref().map_or(true, |l| evaluated.blockers.len() < l.blockers.len()) {
            least = Some(evaluated);
        }
    }
    least
        .map(|l| SpawnPick { pose: l.pose, all_blocked: true })
        .ok_or(SpawnRefusal { supplied: spawn_points.len(), usable: usable.len() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const TOLERANCE: f64 = 1e-9;

    fn fixture() -> Value {
        serde_json::from_str(include_str!("../testdata/parity/spoint-spawn-clearance.json")).unwrap()
    }

    fn num(v: &Value) -> f64 {
        v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())).unwrap()
    }

    fn v3(v: &Value) -> V3 {
        [num(&v[0]), num(&v[1]), num(&v[2])]
    }

    fn assert_close(got: f64, want: f64, what: &str) {
        assert!((got - want).abs() <= TOLERANCE, "{what}: {got} against spoint {want}");
    }

    fn assert_pose(got: V3, want: &Value, what: &str) {
        let want = v3(want);
        for k in 0..3 {
            assert_close(got[k], want[k], what);
        }
    }

    fn ids(v: &Value) -> Vec<u32> {
        v.as_array().unwrap().iter().map(|id| id.as_u64().unwrap() as u32).collect()
    }

    fn players(v: &Value) -> Vec<Player> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|p| Player {
                id: p["id"].as_u64().unwrap() as u32,
                position: p["position"].as_array().map(|_| v3(&p["position"])),
            })
            .collect()
    }

    fn footprint_kind(v: &Value) -> BlockerKind {
        match v.as_str().unwrap() {
            "intruding" => BlockerKind::Intruding,
            "void" => BlockerKind::Void,
            other => panic!("unknown footprint kind {other}"),
        }
    }

    fn blocker(v: &Value) -> BlockerKind {
        match v["kind"].as_str().unwrap() {
            "no-surface" => BlockerKind::NoSurface,
            "surface-mismatch" => BlockerKind::SurfaceMismatch,
            "intruding" => BlockerKind::Intruding,
            "void" => BlockerKind::Void,
            "player" => BlockerKind::Player(v["playerId"].as_u64().unwrap() as u32),
            other => panic!("unknown blocker kind {other}"),
        }
    }

    struct Candidate {
        x: f64,
        z: f64,
        terrain_y: f64,
        feet_y: Option<f64>,
        footprint: Vec<BlockerKind>,
    }

    struct Scene {
        candidates: Vec<Candidate>,
        players: Vec<Player>,
    }

    impl Scene {
        fn from_fixture(f: &Value, players: Vec<Player>) -> Scene {
            let candidates = f["scene"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| {
                    let sp = v3(&c["sp"]);
                    Candidate {
                        x: sp[0],
                        z: sp[2],
                        terrain_y: num(&c["terrainY"]),
                        feet_y: c["feetY"].as_f64(),
                        footprint: c["footprint"].as_array().unwrap().iter().map(footprint_kind).collect(),
                    }
                })
                .collect();
            Scene { candidates, players }
        }

        fn at(&self, x: f64, z: f64) -> &Candidate {
            self.candidates.iter().find(|c| c.x == x && c.z == z).unwrap()
        }
    }

    impl SpawnWorld for Scene {
        fn terrain_height_at(&self, x: f64, z: f64) -> f64 {
            self.at(x, z).terrain_y
        }

        fn spawn_surface_y(&self, sp: V3, _: f64, _: f64, _: f64, _: f64) -> Option<f64> {
            self.at(sp[0], sp[2]).feet_y
        }

        fn footprint_blockers(&self, sp: V3, _: f64, _: &Hitbox) -> Vec<BlockerKind> {
            self.at(sp[0], sp[2]).footprint.clone()
        }

        fn players(&self) -> &[Player] {
            &self.players
        }
    }

    #[test]
    fn evaluations_match_spoint_fixture() {
        let f = fixture();
        let scene = Scene::from_fixture(&f, players(&f["players"]));
        for case in f["evaluations"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let exclude_ids = ids(&case["excludeIds"]);
            let exclude = |id: u32| exclude_ids.contains(&id);
            let got = evaluate_spawn_point(&scene, v3(&case["sp"]), &DEFAULT_HITBOX, &exclude);
            let want_blockers: Vec<BlockerKind> = case["blockers"].as_array().unwrap().iter().map(blocker).collect();
            assert_eq!(got.blockers, want_blockers, "{name}: blockers against spoint");
            assert_pose(got.pose, &case["pose"], name);
            if case["feetY"].is_null() {
                assert_eq!(got.feet_y, None, "{name}: feetY against spoint null");
            } else {
                let feet = got.feet_y.unwrap_or_else(|| panic!("{name}: no feetY against spoint {}", case["feetY"]));
                assert_close(feet, num(&case["feetY"]), name);
            }
        }
    }

    #[test]
    fn picks_match_spoint_fixture() {
        let f = fixture();
        for case in f["picks"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let scene = Scene::from_fixture(&f, players(&case["players"]));
            let exclude_ids = ids(&case["excludeIds"]);
            let exclude = |id: u32| exclude_ids.contains(&id);
            let spawn: Vec<V3> = case["spawn"].as_array().unwrap().iter().map(v3).collect();
            let got = pick_clear_spawn_pose(&scene, &spawn, &exclude, num(&case["minSafeDistanceM"]), &DEFAULT_HITBOX);
            let want = &case["result"];
            match want["kind"].as_str().unwrap() {
                "pose" => {
                    let pick = got.unwrap_or_else(|e| panic!("{name}: refused ({e}) against spoint pose"));
                    assert_pose(pick.pose, &want["pose"], name);
                    assert_eq!(pick.all_blocked, want["allBlocked"].as_bool().unwrap(), "{name}: allBlocked");
                }
                "refusal" => {
                    let err = got.expect_err(&format!("{name}: spoint refused, bevy picked a pose"));
                    assert_eq!(err.to_string(), want["error"].as_str().unwrap(), "{name}: refusal message");
                }
                other => panic!("{name}: unknown spoint result {other}"),
            }
        }
    }
}
