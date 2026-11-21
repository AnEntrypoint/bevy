use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
pub const EMOTE_RATE_LIMIT_MS: f64 = 800.0;

pub const EMOTE_CLIPS: [(&str, &str); 8] = [
    ("wave", "Bow"),
    ("dance", "DanceLoop"),
    ("nod", "HeadNod"),
    ("victory", "Victory"),
    ("meditate", "Meditate"),
    ("jumpingjacks", "JumpingJacks"),
    ("confused", "Confused"),
    ("sit", "SittingEnter"),
];

pub fn emote_clip(code: &Value) -> Option<&'static str> {
    let code = code.as_str()?;
    EMOTE_CLIPS.iter().find(|(name, _)| *name == code).map(|(_, clip)| *clip)
}

pub fn player_slot(id: &Value) -> Option<i64> {
    id.as_f64().filter(|v| v.fract() == 0.0 && *v >= 1.0).map(|v| v as i64)
}

#[derive(Default)]
pub struct EmoteGate {
    last_at_ms: HashMap<i64, f64>,
}

impl EmoteGate {
    pub fn admit(&mut self, sender_id: Option<&Value>, code: &Value, now_ms: f64) -> Option<(i64, &'static str)> {
        let player = player_slot(sender_id?)?;
        let clip = emote_clip(code)?;
        let last = self.last_at_ms.get(&player).copied().unwrap_or(0.0);
        if now_ms - last < EMOTE_RATE_LIMIT_MS {
            return None;
        }
        self.last_at_ms.insert(player, now_ms);
        Some((player, clip))
    }
}

pub const SCOREBOARD_MAX_ENTRIES: usize = 200;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stat {
    pub kills: f64,
    pub deaths: f64,
    pub damage: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub name: String,
    pub stat: Stat,
    pub updated_at_ms: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct Live {
    key: String,
    name: String,
    stat: Stat,
}

#[derive(Default)]
pub struct Scoreboard {
    records: BTreeMap<String, Record>,
    live: HashMap<i64, Live>,
    pub ops: Vec<&'static str>,
}

pub fn stats_key_of(peer: Option<&str>, id: i64) -> String {
    match peer.filter(|p| !p.is_empty()) {
        Some(p) => format!("peer:{p}"),
        None => format!("player:{id}"),
    }
}

impl Scoreboard {
    pub fn join(&mut self, id: i64, peer: Option<&str>, name: Option<&str>) {
        let key = stats_key_of(peer, id);
        let stat = self.records.get(&key).map(|r| r.stat.clone()).unwrap_or_default();
        let name = name.filter(|n| !n.is_empty()).map_or_else(|| format!("Player {id}"), str::to_string);
        self.live.insert(id, Live { key, name, stat });
    }

    pub fn stats_of(&self, id: i64) -> Stat {
        self.live.get(&id).map(|p| p.stat.clone()).unwrap_or_default()
    }

    pub fn hit(&mut self, shooter: i64, damage: f64, now: f64) {
        self.add(shooter, 0.0, 0.0, damage);
        self.persist(shooter, now);
    }

    pub fn kill(&mut self, shooter: i64, target: i64, damage: f64, now: f64) {
        self.add(shooter, 1.0, 0.0, damage);
        self.add(target, 0.0, 1.0, 0.0);
        self.persist(shooter, now);
        self.persist(target, now);
        self.ops.push("flush");
    }

    pub fn fall(&mut self, id: i64, now: f64) {
        self.add(id, 0.0, 1.0, 0.0);
        self.persist(id, now);
        self.ops.push("flush");
    }

    pub fn leave(&mut self, id: i64, now: f64) {
        self.persist(id, now);
        self.live.remove(&id);
    }

    fn add(&mut self, id: i64, kills: f64, deaths: f64, damage: f64) {
        if let Some(p) = self.live.get_mut(&id) {
            p.stat.kills += kills;
            p.stat.deaths += deaths;
            p.stat.damage += damage;
        }
    }

    fn persist(&mut self, id: i64, now: f64) {
        let Some(p) = self.live.get(&id) else { return };
        self.records.insert(p.key.clone(), Record { name: p.name.clone(), stat: p.stat.clone(), updated_at_ms: now });
        self.trim();
        self.ops.push("save");
    }

    fn trim(&mut self) {
        if self.records.len() <= SCOREBOARD_MAX_ENTRIES {
            return;
        }
        let mut order: Vec<(f64, String)> = self.records.iter().map(|(k, r)| (r.updated_at_ms, k.clone())).collect();
        order.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        for (_, key) in order.iter().take(order.len() - SCOREBOARD_MAX_ENTRIES) {
            self.records.remove(key);
        }
    }
}

mod tests {
    use super::*;
    use serde_json::json;

    fn fixture() -> Value {
        crate::testkit::parse(include_str!("../testdata/parity/spoint-tps-game-server.json"))
    }

    fn norm(v: &Value) -> Value {
        match v {
            Value::Number(n) => json!(n.as_f64().unwrap()),
            Value::Array(a) => Value::Array(a.iter().map(norm).collect()),
            Value::Object(o) => Value::Object(o.iter().map(|(k, v)| (k.clone(), norm(v))).collect()),
            _ => v.clone(),
        }
    }

    fn stat_json(s: &Stat) -> Value {
        json!({ "kills": s.kills, "deaths": s.deaths, "damage": s.damage })
    }

    fn board_json(sb: &Scoreboard) -> Value {
        Value::Object(
            sb.records
                .iter()
                .map(|(key, r)| {
                    (key.clone(), json!({ "name": r.name, "kills": r.stat.kills, "deaths": r.stat.deaths, "damage": r.stat.damage, "updatedAtMs": r.updated_at_ms }))
                })
                .collect(),
        )
    }

    #[test]
    fn tps_game_server_matches_spoint() {
        let f = fixture();
        let emotes = crate::testkit::cases(&f, "emotes");
        assert_eq!(EMOTE_CLIPS.len(), emotes.len(), "emote table size against spoint");
        for e in emotes {
            assert_eq!(emote_clip(&json!(e["code"])), e["clip"].as_str(), "clip for {}", e["code"]);
        }
        assert_eq!(emote_clip(&json!("Wave")), None);
        for scenario in crate::testkit::cases(&f, "scenarios") {
            let name = scenario["name"].as_str().unwrap();
            let mut gate = EmoteGate::default();
            for step in scenario["steps"].as_array().unwrap() {
                let sender = Some(&step["sender"]).filter(|s| !s.is_null());
                let now = step["at"].as_f64().unwrap();
                let got: Vec<Value> = gate
                    .admit(sender, &step["code"], now)
                    .map(|(player, clip)| json!({ "playerId": player, "clip": clip, "loop": false }))
                    .into_iter()
                    .collect();
                assert_eq!(json!(got), step["played"], "{name} at {now} for {}", step["code"]);
            }
        }
    }

    #[test]
    fn scoreboard_matches_spoint() {
        let f = fixture();
        for scenario in f["scoreboard"]["scenarios"].as_array().unwrap() {
            let name = scenario["name"].as_str().unwrap();
            let mut sb = Scoreboard::default();
            for (i, row) in scenario["rows"].as_array().unwrap().iter().enumerate() {
                let event = row["event"].as_str().unwrap();
                let now = row["at"].as_f64().unwrap();
                let handlers = row["handlers"].as_array().unwrap();
                let find = |kind: &str| handlers.iter().find(|h| h["handler"] == kind);
                match event {
                    "join" => sb.join(row["id"].as_i64().unwrap(), row["peer"].as_str(), row["name"].as_str()),
                    "hit" => {
                        let hit = find("onHit").expect("hit row carries onHit");
                        let damage = hit["damage"].as_f64().unwrap();
                        match find("onKill") {
                            Some(kill) => sb.kill(kill["shooterId"].as_i64().unwrap(), kill["targetId"].as_i64().unwrap(), damage, now),
                            None => sb.hit(hit["shooterId"].as_i64().unwrap(), damage, now),
                        }
                    }
                    "fall" => {
                        let death = find("onDeath").expect("fall row carries onDeath");
                        sb.fall(death["playerId"].as_i64().unwrap(), now);
                    }
                    "leave" => sb.leave(row["id"].as_i64().unwrap(), now),
                    "tick" => assert!(handlers.is_empty(), "{name} row {i}: tick fired handlers"),
                    other => panic!("{name} row {i}: unknown event {other}"),
                }
                let ops = std::mem::take(&mut sb.ops);
                let want: Vec<&str> = row["writes"].as_array().unwrap().iter().map(|w| w["op"].as_str().unwrap()).collect();
                assert_eq!(ops, want, "{name} row {i} ({event}) writes against spoint");
                if let Some(board) = row.get("board") {
                    assert_eq!(norm(&board_json(&sb)), norm(board), "{name} row {i} ({event}) board against spoint");
                    for id in 1..=3 {
                        assert_eq!(norm(&stat_json(&sb.stats_of(id))), norm(&row["stats"][id.to_string()]), "{name} row {i} stats of {id}");
                    }
                }
            }
            assert_eq!(norm(&board_json(&sb)), norm(&scenario["finalBoard"]), "{name} final board against spoint");
        }
    }
}
