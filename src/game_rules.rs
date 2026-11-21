#![cfg_attr(not(test), allow(dead_code))]

use crate::height::ops_jsgen::V3;
use serde_json::Value;

pub const ANY_TEAM: &str = "any";
pub const POWERUP_RESPAWN_MS: f64 = 15000.0;
pub const PICKUP_RADIUS_M: f64 = 1.7;
pub const MAX_HEALTH: f64 = 100.0;
pub const BUFF_REGEN_FRACTION_PER_SEC: f64 = 0.1;
pub const SHOT_GATE_MS: f64 = 100.0;
pub const RELOAD_GATE_MS: f64 = 100.0;
pub const RELOAD_TIME_MS: f64 = 2000.0;
pub const TRACER_RANGE_M: f64 = 100.0;
pub const MUZZLE_HEIGHT_M: f64 = 0.9;
pub const BOOST_FALLBACK_DURATION_S: f64 = 45.0;
pub const SHOOT_KNOCKBACK: f64 = 2.0;

#[derive(Clone, Debug, PartialEq)]
pub struct SpawnGizmo {
    pub mesh: &'static str,
    pub color: &'static str,
    pub size: V3,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpawnMarker {
    pub team: String,
    pub gizmo: Option<SpawnGizmo>,
}

pub fn spawn_marker(team: Option<&str>, show_gizmo: Option<bool>) -> SpawnMarker {
    let gizmo = if show_gizmo == Some(false) {
        None
    } else {
        Some(SpawnGizmo { mesh: "capsule", color: "#33cc88", size: [0.4, 0.9, 0.4] })
    };
    SpawnMarker { team: team.unwrap_or(ANY_TEAM).to_string(), gizmo }
}

pub fn spawn_team_on_change(team: Option<&str>) -> String {
    team.unwrap_or(ANY_TEAM).to_string()
}

pub fn team_admits(entry: Option<&str>, requested: Option<&str>) -> bool {
    let requested = requested.filter(|r| !r.is_empty() && *r != ANY_TEAM);
    let entry = entry.filter(|e| !e.is_empty() && *e != ANY_TEAM);
    match (requested, entry) {
        (Some(r), Some(e)) => r == e,
        _ => true,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorldEntity {
    pub position: V3,
    pub is_spawn: bool,
    pub spawn_team: Option<String>,
}

pub fn collect_spawn_points(entities: &[WorldEntity], requested: Option<&str>) -> Vec<V3> {
    entities
        .iter()
        .filter(|e| e.is_spawn && team_admits(e.spawn_team.as_deref(), requested))
        .map(|e| e.position)
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Buff {
    pub duration_s: f64,
    pub speed: f64,
    pub fire_rate: f64,
    pub damage: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PowerupDef {
    pub kind: &'static str,
    pub color: u32,
    pub emissive: u32,
    pub buff: Buff,
}

pub const POWERUP_DEFS: [PowerupDef; 3] = [
    PowerupDef { kind: "damage", color: 0xff3344, emissive: 0xaa0000, buff: Buff { duration_s: 20.0, speed: 1.0, fire_rate: 1.0, damage: 2.0 } },
    PowerupDef { kind: "speed", color: 0x33aaff, emissive: 0x0044aa, buff: Buff { duration_s: 20.0, speed: 1.5, fire_rate: 1.0, damage: 1.0 } },
    PowerupDef { kind: "rapid", color: 0xffcc33, emissive: 0xaa6600, buff: Buff { duration_s: 20.0, speed: 1.0, fire_rate: 2.0, damage: 1.0 } },
];

pub fn normalize_buff(duration_s: f64, speed: Option<f64>, fire_rate: Option<f64>, damage: Option<f64>) -> Buff {
    Buff {
        duration_s,
        speed: speed.unwrap_or(1.0),
        fire_rate: fire_rate.unwrap_or(1.0),
        damage: damage.unwrap_or(1.0),
    }
}

pub fn pickup_reaches(powerup: V3, player: V3) -> bool {
    let dx = player[0] - powerup[0];
    let dy = player[1] - powerup[1];
    let dz = player[2] - powerup[2];
    !(dx * dx + dy * dy + dz * dz > PICKUP_RADIUS_M * PICKUP_RADIUS_M)
}

pub fn buff_expires_ms(now: f64, duration_s: f64) -> f64 {
    now + duration_s * 1000.0
}

pub fn buff_lapsed(now: f64, expires_ms: f64) -> bool {
    now >= expires_ms
}

pub fn powerup_respawn_ms(now: f64, def_respawn_ms: Option<f64>, spec_respawn_ms: Option<f64>) -> f64 {
    now + def_respawn_ms.or(spec_respawn_ms).unwrap_or(POWERUP_RESPAWN_MS)
}

pub fn powerup_due(now: f64, respawn_at_ms: f64) -> bool {
    now >= respawn_at_ms
}

pub fn regen_health(hp: f64, max_hp: f64, regen_fraction_per_sec: f64, dt: f64) -> f64 {
    (hp + max_hp * regen_fraction_per_sec * dt).min(max_hp)
}

fn js_or(value: Option<f64>, fallback: f64) -> f64 {
    match value {
        Some(v) if v != 0.0 && !v.is_nan() => v,
        _ => fallback,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Boost {
    pub expires_ms: f64,
    pub fire_rate: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClientShot {
    pub origin: V3,
    pub direction: V3,
    pub tracer_end: V3,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClientGun {
    pub ammo: u32,
    pub magazine: u32,
    pub reloading: bool,
    pub reload_end_ms: Option<f64>,
    pub last_shot_ms: f64,
    pub last_reload_ms: f64,
    pub boost: Option<Boost>,
}

impl ClientGun {
    pub fn new(magazine: u32) -> Self {
        Self {
            ammo: magazine,
            magazine,
            reloading: false,
            reload_end_ms: None,
            last_shot_ms: 0.0,
            last_reload_ms: 0.0,
            boost: None,
        }
    }

    pub fn shot_gate_ms(&self) -> f64 {
        SHOT_GATE_MS / self.boost.map_or(1.0, |b| b.fire_rate)
    }

    pub fn can_fire(&self, now: f64) -> bool {
        !self.reloading && self.ammo > 0 && now - self.last_shot_ms > self.shot_gate_ms()
    }

    pub fn shoot(&mut self, now: f64, position: V3, aim: V3, velocity: &mut V3, shoot_knockback: f64) -> Option<ClientShot> {
        if !self.can_fire(now) {
            return None;
        }
        let origin = [position[0], position[1] + MUZZLE_HEIGHT_M, position[2]];
        self.last_shot_ms = now;
        velocity[0] -= aim[0] * shoot_knockback;
        velocity[2] -= aim[2] * shoot_knockback;
        self.ammo = self.ammo.saturating_sub(1);
        let tracer_end = [
            origin[0] + aim[0] * TRACER_RANGE_M,
            origin[1] + aim[1] * TRACER_RANGE_M,
            origin[2] + aim[2] * TRACER_RANGE_M,
        ];
        Some(ClientShot { origin, direction: aim, tracer_end })
    }

    pub fn request_reload(&mut self, now: f64) -> bool {
        if self.reloading || now - self.last_reload_ms <= RELOAD_GATE_MS {
            return false;
        }
        self.last_reload_ms = now;
        true
    }

    pub fn on_reload_start(&mut self, now: f64, duration_ms: Option<f64>) {
        self.reloading = true;
        self.reload_end_ms = Some(now + js_or(duration_ms, RELOAD_TIME_MS));
    }

    pub fn on_reload_complete(&mut self, ammo: Option<u32>) {
        self.reloading = false;
        self.reload_end_ms = None;
        self.ammo = ammo.unwrap_or(self.magazine);
    }

    pub fn on_respawn(&mut self, ammo: Option<u32>) {
        self.reloading = false;
        self.reload_end_ms = None;
        if let Some(a) = ammo {
            self.ammo = a;
        }
    }

    pub fn on_buff_applied(&mut self, now: f64, duration_s: Option<f64>, fire_rate: Option<f64>) {
        self.boost = Some(Boost {
            expires_ms: now + js_or(duration_s, BOOST_FALLBACK_DURATION_S) * 1000.0,
            fire_rate: js_or(fire_rate, 1.0),
        });
    }

    pub fn on_frame(&mut self, now: f64) {
        if self.boost.is_some_and(|b| now >= b.expires_ms) {
            self.boost = None;
        }
    }
}

pub fn knockback_on_hit(velocity: &mut V3, dir: V3, knockback: f64) {
    velocity[0] += dir[0] * knockback;
    velocity[2] += dir[2] * knockback;
}

#[derive(Clone, Debug, PartialEq)]
pub struct FireClass {
    pub fuel: f64,
    pub burn_rate: f64,
    pub ignite_heat: f64,
    pub heat_out: f64,
    pub spot_chance: f64,
    pub spot_heat: f64,
    pub smoke: f64,
    pub damage: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FireGameplay {
    pub damage_every_ticks: f64,
    pub status_damage_per_sec: f64,
    pub burn_status_seconds: f64,
    pub smoke_block_depth: f64,
    pub smoke_height_m: f64,
    pub eye_height_m: f64,
    pub explosion_impulse: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FireSpec {
    pub radius: f64,
    pub role: &'static str,
    pub step_ticks: f64,
    pub seed: f64,
    pub lead_ticks: f64,
    pub regrow_steps: f64,
    pub grass: FireClass,
    pub shrub: FireClass,
    pub forest: FireClass,
    pub gameplay: FireGameplay,
    pub firebreak_kinds: [&'static str; 2],
    pub water: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FireTuning {
    pub radius: Option<f64>,
    pub seed: Option<f64>,
    pub step_ticks: Option<f64>,
    pub lead_ticks: Option<f64>,
    pub regrow_steps: Option<f64>,
}

pub const FIRE_SPEC: FireSpec = FireSpec {
    radius: 63600.0,
    role: "authority",
    step_ticks: 30.0,
    seed: 1337.0,
    lead_ticks: 6.0,
    regrow_steps: 1200.0,
    grass: FireClass { fuel: 3000.0, burn_rate: 1500.0, ignite_heat: 90.0, heat_out: 500.0, spot_chance: 0.0, spot_heat: 0.0, smoke: 40.0, damage: 6.0 },
    shrub: FireClass { fuel: 12000.0, burn_rate: 2000.0, ignite_heat: 170.0, heat_out: 800.0, spot_chance: 40.0, spot_heat: 400.0, smoke: 90.0, damage: 10.0 },
    forest: FireClass { fuel: 45000.0, burn_rate: 2500.0, ignite_heat: 600.0, heat_out: 1200.0, spot_chance: 120.0, spot_heat: 700.0, smoke: 160.0, damage: 16.0 },
    gameplay: FireGameplay { damage_every_ticks: 15.0, status_damage_per_sec: 6.0, burn_status_seconds: 4.0, smoke_block_depth: 1.0, smoke_height_m: 30.0, eye_height_m: 1.6, explosion_impulse: 12.0 },
    firebreak_kinds: ["road", "river"],
    water: true,
};

pub fn fire_layer_enabled(flag: Option<&Value>) -> bool {
    flag.and_then(Value::as_bool) == Some(true)
}

pub fn merge_fire_spec(base: &FireSpec, tuning: &FireTuning) -> FireSpec {
    let mut spec = base.clone();
    if let Some(v) = tuning.radius {
        spec.radius = v;
    }
    if let Some(v) = tuning.seed {
        spec.seed = v;
    }
    if let Some(v) = tuning.step_ticks {
        spec.step_ticks = v;
    }
    if let Some(v) = tuning.lead_ticks {
        spec.lead_ticks = v;
    }
    if let Some(v) = tuning.regrow_steps {
        spec.regrow_steps = v;
    }
    spec
}

pub fn ignition_source(shooter_id: i64) -> i64 {
    shooter_id & 255
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const TOLERANCE: f64 = 1e-9;
    const PLAYER_ID: i64 = 1;
    const PU_Y: f64 = 10.0 + 0.6;

    fn fixture() -> Value {
        serde_json::from_str(include_str!("../testdata/parity/spoint-game-rules.json")).unwrap()
    }

    fn num(v: &Value) -> f64 {
        v.as_f64().unwrap()
    }

    fn v3(v: &Value) -> V3 {
        [num(&v[0]), num(&v[1]), num(&v[2])]
    }

    fn assert_json_close(got: &Value, want: &Value, what: &str) {
        match (got, want) {
            (Value::Number(g), Value::Number(w)) => {
                let (g, w) = (g.as_f64().unwrap(), w.as_f64().unwrap());
                assert!((g - w).abs() <= TOLERANCE, "{what}: {g} against spoint {w}");
            }
            (Value::Array(g), Value::Array(w)) => {
                assert_eq!(g.len(), w.len(), "{what}: length against spoint");
                for (i, (g, w)) in g.iter().zip(w).enumerate() {
                    assert_json_close(g, w, &format!("{what}[{i}]"));
                }
            }
            (Value::Object(g), Value::Object(w)) => {
                assert_eq!(g.len(), w.len(), "{what}: key count against spoint");
                for (k, w) in w {
                    let g = g.get(k).unwrap_or_else(|| panic!("{what}.{k}: missing, spoint has {w}"));
                    assert_json_close(g, w, &format!("{what}.{k}"));
                }
            }
            _ => assert_eq!(got, want, "{what}"),
        }
    }

    fn spawn_custom_json(m: &SpawnMarker) -> Value {
        let mut out = json!({ "_spawnPoint": true, "_spawnTeam": m.team.as_str() });
        if let Some(g) = &m.gizmo {
            out["mesh"] = json!(g.mesh);
            out["color"] = json!(g.color);
            out["sx"] = json!(g.size[0]);
            out["sy"] = json!(g.size[1]);
            out["sz"] = json!(g.size[2]);
        }
        out
    }

    #[test]
    fn spawn_marker_matches_spoint() {
        let f = fixture();
        for case in f["spawn"]["setups"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let team = case["config"].get("team").and_then(Value::as_str);
            let show = case["config"].get("showGizmo").and_then(Value::as_bool);
            let got = spawn_custom_json(&spawn_marker(team, show));
            assert_json_close(&got, &case["setup"], &format!("{name}: setup"));
            assert_eq!(spawn_team_on_change(Some("blue")), case["afterBlue"].as_str().unwrap(), "{name}: afterBlue");
            assert_eq!(spawn_team_on_change(None), case["afterNoTeam"].as_str().unwrap(), "{name}: afterNoTeam");
        }
    }

    #[test]
    fn collect_spawn_points_matches_spoint() {
        let f = fixture();
        let entities: Vec<WorldEntity> = f["spawn"]["entities"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| WorldEntity {
                position: v3(&e["position"]),
                is_spawn: e["custom"]["_spawnPoint"].as_bool() == Some(true),
                spawn_team: e["custom"]["_spawnTeam"].as_str().map(str::to_string),
            })
            .collect();
        for case in f["spawn"]["collect"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let got: Vec<V3> = collect_spawn_points(&entities, case["team"].as_str());
            assert_json_close(&json!(got), &case["positions"], &format!("collect {name}"));
        }
    }

    fn pad_id(kind: &str) -> String {
        format!("powerup_{kind}")
    }

    fn buff_json(b: Option<(f64, Buff)>) -> Value {
        match b {
            None => Value::Null,
            Some((expires, b)) => json!({ "expiresAt": expires, "speed": b.speed, "fireRate": b.fire_rate, "damage": b.damage }),
        }
    }

    #[derive(Clone, Debug)]
    struct SimDef {
        kind: &'static str,
        buff: Buff,
        respawn_ms: Option<f64>,
    }

    struct Pad {
        position: V3,
        active: bool,
        respawn_at: f64,
    }

    struct Sim {
        defs: Vec<SimDef>,
        pads: Vec<Pad>,
        spec_respawn_ms: Option<f64>,
        buff: Option<(f64, Buff)>,
        health: f64,
    }

    impl Sim {
        fn new(defs: Vec<SimDef>, spawn_points: &[V3], spec_respawn_ms: Option<f64>) -> (Sim, Vec<Value>) {
            let n = spawn_points.len();
            let m = defs.len();
            let pads: Vec<Pad> = (0..m)
                .map(|i| {
                    let p = spawn_points[((i + 1) * n) / (m + 1)];
                    Pad { position: [p[0], p[1] + 0.6, p[2]], active: true, respawn_at: 0.0 }
                })
                .collect();
            let setup = defs.iter().map(|d| json!(["spawn", pad_id(d.kind)])).collect();
            let sim = Sim { defs, pads, spec_respawn_ms, buff: None, health: MAX_HEALTH };
            (sim, setup)
        }

        fn tick(&mut self, now: f64, dt: f64, player: V3) -> Vec<Value> {
            let mut events = Vec::new();
            if let Some((expires, _)) = self.buff {
                if buff_lapsed(now, expires) {
                    self.buff = None;
                    events.push(json!(["buff_expired", PLAYER_ID]));
                } else {
                    self.health = regen_health(self.health, MAX_HEALTH, BUFF_REGEN_FRACTION_PER_SEC, dt);
                }
            }
            for i in 0..self.pads.len() {
                if self.pads[i].active {
                    if !pickup_reaches(self.pads[i].position, player) {
                        continue;
                    }
                    let def = self.defs[i].clone();
                    self.buff = Some((buff_expires_ms(now, def.buff.duration_s), def.buff));
                    events.push(json!(["buff_applied", PLAYER_ID, def.buff.duration_s, def.buff.speed, def.buff.fire_rate, def.buff.damage]));
                    events.push(json!(["destroy", pad_id(def.kind)]));
                    self.pads[i].active = false;
                    self.pads[i].respawn_at = powerup_respawn_ms(now, def.respawn_ms, self.spec_respawn_ms);
                } else if powerup_due(now, self.pads[i].respawn_at) {
                    self.pads[i].active = true;
                    events.push(json!(["spawn", pad_id(self.defs[i].kind)]));
                }
            }
            events
        }
    }

    struct Step {
        label: &'static str,
        at: f64,
        dt: f64,
        pos: Option<V3>,
        health: Option<f64>,
    }

    fn run_powerup_scenario(defs: Vec<SimDef>, spec_respawn: Option<f64>, want: &Value, steps: &[Step]) {
        let spawn_points: Vec<V3> = want["spawnPoints"].as_array().unwrap().iter().map(v3).collect();
        let start = num(&want["startAt"]);
        let (mut sim, setup) = Sim::new(defs, &spawn_points, spec_respawn);
        assert_json_close(&Value::Array(setup), &want["setup"], "setup");
        assert_eq!(steps.len(), want["trace"].as_array().unwrap().len(), "step count against spoint");
        let mut player: V3 = [0.0, 0.0, 0.0];
        for (i, step) in steps.iter().enumerate() {
            let w = &want["trace"][i];
            if let Some(p) = step.pos {
                player = p;
            }
            if let Some(h) = step.health {
                sim.health = h;
            }
            let now = start + step.at;
            let events = sim.tick(now, step.dt, player);
            let got = json!({ "label": step.label, "at": now, "health": sim.health, "buff": buff_json(sim.buff), "events": events });
            assert_json_close(&got, w, step.label);
        }
    }

    #[test]
    fn powerup_defs_match_spoint() {
        let f = fixture();
        let want = &f["powerups"]["defs"];
        for (i, d) in POWERUP_DEFS.iter().enumerate() {
            let got = json!({
                "type": d.kind,
                "color": d.color,
                "emissive": d.emissive,
                "buff": { "duration": d.buff.duration_s, "speedMultiplier": d.buff.speed, "fireRateMultiplier": d.buff.fire_rate, "damageMultiplier": d.buff.damage }
            });
            assert_json_close(&got, &want[i], &format!("powerup {}", d.kind));
        }
        assert_eq!(num(&f["powerups"]["respawnMs"]), POWERUP_RESPAWN_MS, "respawnMs against spoint");
    }

    #[test]
    fn powerup_timers_match_spoint_main_scenario() {
        let f = fixture();
        let defs: Vec<SimDef> = POWERUP_DEFS.iter().map(|d| SimDef { kind: d.kind, buff: d.buff, respawn_ms: None }).collect();
        let steps = [
            Step { label: "pickup-damage-dz1", at: 0.0, dt: 0.0, pos: Some([10.0, PU_Y, 1.0]), health: None },
            Step { label: "no-respawn-before-15s", at: 14999.0, dt: 0.0, pos: Some([-50.0, PU_Y, 0.0]), health: None },
            Step { label: "respawn-damage-at-15s", at: 15000.0, dt: 0.0, pos: None, health: None },
            Step { label: "regen-while-buffed", at: 19000.0, dt: 0.5, pos: None, health: Some(50.0) },
            Step { label: "expired-at-20s", at: 20000.0, dt: 1.0, pos: None, health: None },
            Step { label: "pickup-speed-dz1.7-boundary", at: 20001.0, dt: 0.0, pos: Some([20.0, PU_Y, 1.7]), health: None },
            Step { label: "outside-dz1.75", at: 20002.0, dt: 0.0, pos: Some([30.0, PU_Y, 1.75]), health: None },
            Step { label: "pickup-rapid-center", at: 20003.0, dt: 0.0, pos: Some([30.0, PU_Y, 0.0]), health: None },
            Step { label: "speed-not-yet-respawned", at: 35000.0, dt: 0.0, pos: Some([-50.0, PU_Y, 0.0]), health: None },
            Step { label: "speed-respawn-at-35001", at: 35001.0, dt: 0.0, pos: None, health: None },
            Step { label: "rapid-respawn-at-35003", at: 35003.0, dt: 0.0, pos: None, health: None },
            Step { label: "rapid-expired-at-40003", at: 40003.0, dt: 0.0, pos: None, health: None },
        ];
        run_powerup_scenario(defs, Some(POWERUP_RESPAWN_MS), &f["powerups"]["main"], &steps);
    }

    #[test]
    fn powerup_timers_match_spoint_default_respawn_scenario() {
        let f = fixture();
        let defs = vec![
            SimDef { kind: "quick", buff: normalize_buff(5.0, Some(2.0), None, None), respawn_ms: Some(4000.0) },
            SimDef { kind: "plain", buff: normalize_buff(3.0, None, None, None), respawn_ms: None },
        ];
        let steps = [
            Step { label: "quick-pickup", at: 0.0, dt: 0.0, pos: Some([10.0, PU_Y, 0.0]), health: None },
            Step { label: "quick-away-before-4s", at: 3999.0, dt: 0.0, pos: Some([-50.0, PU_Y, 0.0]), health: None },
            Step { label: "quick-respawn-4s", at: 4000.0, dt: 0.0, pos: None, health: None },
            Step { label: "quick-expired-5s", at: 5000.0, dt: 0.0, pos: None, health: None },
            Step { label: "plain-pickup", at: 5001.0, dt: 0.0, pos: Some([20.0, PU_Y, 0.0]), health: None },
            Step { label: "plain-away-before-15s", at: 20000.0, dt: 0.0, pos: Some([-50.0, PU_Y, 0.0]), health: None },
            Step { label: "plain-respawn-15s", at: 20001.0, dt: 0.0, pos: None, health: None },
        ];
        run_powerup_scenario(defs, None, &f["powerups"]["defaults"], &steps);
    }

    fn fire_spec_json(spec: &FireSpec) -> Value {
        let class = |c: &FireClass| json!({
            "fuel": c.fuel, "burnRate": c.burn_rate, "igniteHeat": c.ignite_heat, "heatOut": c.heat_out,
            "spotChance": c.spot_chance, "spotHeat": c.spot_heat, "smoke": c.smoke, "damage": c.damage
        });
        let g = &spec.gameplay;
        json!({
            "radius": spec.radius,
            "role": spec.role,
            "stepTicks": spec.step_ticks,
            "seed": spec.seed,
            "leadTicks": spec.lead_ticks,
            "regrowSteps": spec.regrow_steps,
            "classes": { "grass": class(&spec.grass), "shrub": class(&spec.shrub), "forest": class(&spec.forest) },
            "gameplay": {
                "damageEveryTicks": g.damage_every_ticks, "statusDamagePerSec": g.status_damage_per_sec,
                "burnStatusSeconds": g.burn_status_seconds, "smokeBlockDepth": g.smoke_block_depth,
                "smokeHeightM": g.smoke_height_m, "eyeHeightM": g.eye_height_m, "explosionImpulse": g.explosion_impulse
            },
            "firebreaks": { "kinds": spec.firebreak_kinds, "water": spec.water }
        })
    }

    fn fire_tuning(config_fire: Option<&Value>) -> FireTuning {
        let get = |k: &str| config_fire.and_then(|c| c.get(k)).and_then(Value::as_f64);
        FireTuning {
            radius: get("radius"),
            seed: get("seed"),
            step_ticks: get("stepTicks"),
            lead_ticks: get("leadTicks"),
            regrow_steps: get("regrowSteps"),
        }
    }

    #[test]
    fn fire_layer_spec_matches_spoint() {
        let f = fixture();
        assert_json_close(&fire_spec_json(&FIRE_SPEC), &f["fire"]["spec"], "FIRE_SPEC");
        for case in f["fire"]["cases"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let config_fire = case["config"].get("fire");
            let enabled = fire_layer_enabled(config_fire.and_then(|c| c.get("enabled")));
            let want = &case["fireSpec"];
            if want.is_null() {
                assert!(!enabled, "{name}: spoint has no fire layer but the gate is open");
                continue;
            }
            assert!(enabled, "{name}: spoint has a fire layer but the gate is closed");
            let merged = merge_fire_spec(&FIRE_SPEC, &fire_tuning(config_fire));
            assert_json_close(&fire_spec_json(&merged), want, &format!("{name}: fireSpec"));
            let shooters = case["shooterIds"].as_array().unwrap();
            let seeds = case["ignitionSeeds"].as_array().unwrap();
            for (s, seed) in shooters.iter().zip(seeds) {
                let got = ignition_source(s.as_i64().unwrap());
                assert_json_close(&json!(got), seed, &format!("{name}: ignition seed for shooter {s}"));
            }
        }
    }

    #[test]
    fn client_shooting_matches_spoint() {
        let f = fixture();
        let steps = f["client"]["steps"].as_array().unwrap();
        let c0 = num(&f["client"]["startAt"]);
        let aim: V3 = [0.6, 0.0, -0.8];
        let position: V3 = [0.0, 10.0, 0.0];
        let mut table: Vec<(String, f64, Act)> = vec![
            ("shoot-first".into(), 0.0, Act::Shoot),
            ("shoot-at-100ms-gate-closed".into(), 100.0, Act::Shoot),
            ("shoot-after-100ms".into(), 101.0, Act::Shoot),
            ("buff-rapid-applied".into(), 101.0, Act::Buff(Some(20.0), Some(2.0))),
            ("shoot-boosted-at-50ms-gate-closed".into(), 151.0, Act::Shoot),
            ("shoot-boosted-after-50ms".into(), 152.0, Act::Shoot),
            ("reload-request".into(), 152.0, Act::Reload),
            ("reload-request-inside-100ms".into(), 200.0, Act::Reload),
            ("reload-request-after-100ms".into(), 253.0, Act::Reload),
            ("reload-start".into(), 260.0, Act::ReloadStart(Some(2000.0))),
            ("shoot-while-reloading".into(), 261.0, Act::Shoot),
            ("reload-request-while-reloading".into(), 400.0, Act::Reload),
            ("frame-mid-reload".into(), 500.0, Act::Frame),
            ("reload-complete".into(), 2260.0, Act::ReloadComplete(Some(30))),
        ];
        for k in 0..31 {
            table.push((format!("drain-{k}"), 2300.0 + 60.0 * k as f64, Act::Shoot));
        }
        table.extend([
            ("boost-lapsed-frame".into(), 20101.0, Act::Frame),
            ("shoot-empty-after-drain".into(), 20200.0, Act::Shoot),
            ("respawn-refill".into(), 20300.0, Act::Respawn(30)),
            ("shoot-unboosted-first".into(), 20301.0, Act::Shoot),
            ("shoot-unboosted-at-100ms-gate-closed".into(), 20401.0, Act::Shoot),
            ("hit-taken-knockback".into(), 20500.0, Act::Hit([1.0, 0.0, 0.0], 4.0)),
        ]);
        assert_eq!(table.len(), steps.len(), "step count against spoint");
        let mut gun = ClientGun::new(30);
        let mut velocity: V3 = [0.0, 0.0, 0.0];
        for ((label, offset, act), want) in table.iter().zip(steps) {
            let now = c0 + offset;
            assert_eq!(want["label"].as_str().unwrap(), label.as_str());
            let mut events: Vec<Value> = Vec::new();
            match act {
                Act::Shoot => {
                    if let Some(shot) = gun.shoot(now, position, aim, &mut velocity, SHOOT_KNOCKBACK) {
                        events.push(json!(["fire", shot.origin, shot.direction]));
                        events.push(json!(["tracer", shot.origin, shot.tracer_end]));
                    }
                }
                Act::Reload => {
                    if gun.request_reload(now) {
                        events.push(json!(["reload"]));
                    }
                }
                Act::ReloadStart(d) => gun.on_reload_start(now, *d),
                Act::ReloadComplete(a) => gun.on_reload_complete(*a),
                Act::Buff(d, r) => gun.on_buff_applied(now, *d, *r),
                Act::Frame => gun.on_frame(now),
                Act::Respawn(a) => gun.on_respawn(Some(*a)),
                Act::Hit(dir, k) => {
                    knockback_on_hit(&mut velocity, *dir, *k);
                    events.push(json!(["knockback", [dir[0], 0.0, dir[2]], k, now]));
                }
            }
            let got = json!({
                "label": label,
                "now": now,
                "ammo": gun.ammo,
                "reloading": gun.reloading,
                "reloadEndTime": gun.reload_end_ms,
                "lastShootTime": gun.last_shot_ms,
                "lastReloadTime": gun.last_reload_ms,
                "boost": gun.boost.map(|b| json!({ "expiresAt": b.expires_ms, "fireRate": b.fire_rate })),
                "velocity": velocity,
                "events": events,
            });
            assert_json_close(&got, want, label);
        }
    }

    enum Act {
        Shoot,
        Reload,
        ReloadStart(Option<f64>),
        ReloadComplete(Option<u32>),
        Buff(Option<f64>, Option<f64>),
        Frame,
        Respawn(u32),
        Hit(V3, f64),
    }
}
