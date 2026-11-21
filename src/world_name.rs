use serde_json::Value;
use crate::terrain_config::Refusal;
use crate::canonical_json::canonical_json;

pub const WORLD_DIR: &str = "apps/world";
pub const WORLD_INDEX: &str = "index";
pub const WORLD_FIXTURES_DIR: &str = "apps/world/_fixtures";

pub fn world_stem(v: &Value) -> Option<&str> {
    v.as_str()
        .filter(|s| (1..=128).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'))
}

pub fn is_world_name(v: &Value) -> bool {
    world_stem(v).is_some()
}

fn not_a_stem(name: &Value) -> Refusal {
    Refusal {
        name: "TypeError",
        message: format!("world name must be a world file stem, got {}", canonical_json(name)),
    }
}

pub fn world_file_rel(name: &Value) -> Result<String, Refusal> {
    world_stem(name).map(|stem| format!("{WORLD_DIR}/{stem}.js")).ok_or_else(|| not_a_stem(name))
}

pub fn world_file_candidates(name: &Value) -> Result<[String; 2], Refusal> {
    world_stem(name)
        .map(|stem| [format!("{WORLD_DIR}/{stem}.js"), format!("{WORLD_FIXTURES_DIR}/{stem}.js")])
        .ok_or_else(|| not_a_stem(name))
}

pub fn default_world_name_of(index_module: &Value) -> Result<String, Refusal> {
    match index_module.get("defaultWorld") {
        None => Ok(WORLD_INDEX.to_string()),
        Some(named) => world_stem(named).map(str::to_string).ok_or_else(|| Refusal {
            name: "TypeError",
            message: format!("{WORLD_DIR}/{WORLD_INDEX}.js defaultWorld must be a world file stem, got {}", canonical_json(named)),
        }),
    }
}

mod tests {
    use super::*;
    use serde_json::json;
    use std::fmt::Debug;

    fn check<T: Debug>(label: &str, got: Result<T, Refusal>, want: &Value, same: impl Fn(&T, &Value)) {
        match (got, want["ok"].as_bool()) {
            (Ok(value), Some(true)) => same(&value, &want["value"]),
            (Err(refusal), Some(false)) => {
                assert_eq!(want["name"].as_str(), Some(refusal.name), "{label}: refusal name");
                assert_eq!(want["message"].as_str(), Some(refusal.message.as_str()), "{label}: refusal message");
            }
            (got, _) => panic!("{label}: spoint returns {want}, bevy returns {got:?}"),
        }
    }

    #[test]
    fn world_name_matches_spoint() {
        let f: Value = crate::testkit::parse(include_str!("../testdata/parity/spoint-world-name.json"));
        assert_eq!(WORLD_DIR, f["worldDir"].as_str().unwrap(), "WORLD_DIR");
        assert_eq!(WORLD_INDEX, f["worldIndex"].as_str().unwrap(), "WORLD_INDEX");
        assert_eq!(WORLD_FIXTURES_DIR, f["worldFixturesDir"].as_str().unwrap(), "WORLD_FIXTURES_DIR");
        for case in crate::testkit::cases(&f, "isWorldName") {
            assert_eq!(is_world_name(&case["input"]), case["expect"].as_bool().unwrap(), "isWorldName {}", case["input"]);
        }
        for case in crate::testkit::cases(&f, "worldFileRel") {
            check("worldFileRel", world_file_rel(&case["input"]), &case["expect"], |v: &String, want: &Value| {
                assert_eq!(json!(v), *want, "worldFileRel");
            });
        }
        for case in crate::testkit::cases(&f, "worldFileCandidates") {
            check("worldFileCandidates", world_file_candidates(&case["input"]), &case["expect"], |v: &[String; 2], want: &Value| {
                assert_eq!(json!(v), *want, "worldFileCandidates");
            });
        }
        for case in crate::testkit::cases(&f, "defaultWorldNameOf") {
            check("defaultWorldNameOf", default_world_name_of(&case["input"]), &case["expect"], |v: &String, want: &Value| {
                assert_eq!(json!(v), *want, "defaultWorldNameOf");
            });
        }
    }
}
