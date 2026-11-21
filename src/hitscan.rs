use crate::height::ops_jsgen::V3;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hitbox {
    pub center_height: f64,
    pub radius_sq: f64,
    pub height: f64,
}

pub const DEFAULT_HITBOX: Hitbox = Hitbox { center_height: 0.9, radius_sq: 0.36, height: 1.8 };

#[derive(Debug, PartialEq)]
pub struct CapsuleHit {
    pub proj: V3,
    pub dot: f64,
}

#[derive(Debug)]
pub struct HitTarget {
    pub id: String,
    pub position: V3,
    pub hp: Option<f64>,
}

pub fn normalize_shot_direction(d: V3) -> Option<V3> {
    if !d.iter().all(|c| c.is_finite()) {
        return None;
    }
    let m = d.iter().fold(0.0_f64, |m, c| m.max(c.abs()));
    if !(m > 0.0) {
        return None;
    }
    let s = d.map(|c| c / m);
    let len = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt();
    Some(s.map(|c| c / len))
}

pub fn ray_vs_capsule(origin: V3, direction: V3, range: f64, tp: V3, hitbox: &Hitbox) -> Option<CapsuleHit> {
    let unit = normalize_shot_direction(direction)?;
    let to = [tp[0] - origin[0], tp[1] + hitbox.center_height - origin[1], tp[2] - origin[2]];
    let dot = to[0] * unit[0] + to[1] * unit[1] + to[2] * unit[2];
    if dot < 0.0 || dot > range {
        return None;
    }
    let proj = [0, 1, 2].map(|i| origin[i] + unit[i] * dot);
    let dd = [proj[0] - tp[0], proj[1] - (tp[1] + hitbox.center_height), proj[2] - tp[2]];
    let d2 = dd[0] * dd[0] + dd[1] * dd[1] + dd[2] * dd[2];
    if d2 > hitbox.radius_sq {
        return None;
    }
    Some(CapsuleHit { proj, dot })
}

pub fn hit_height_ratio(proj: V3, tp: V3, hitbox: &Hitbox) -> f64 {
    (proj[1] - tp[1]) / hitbox.height
}

pub fn predict_hit(origin: V3, direction: V3, range: f64, players: &[HitTarget], self_id: &str, headshot_zone: f64) -> Option<bool> {
    if !origin.iter().all(|c| c.is_finite()) {
        return None;
    }
    let unit = normalize_shot_direction(direction)?;
    for p in players {
        if p.id == self_id || !p.position.iter().all(|c| c.is_finite()) {
            continue;
        }
        if p.hp.is_some_and(|hp| hp <= 0.0) {
            continue;
        }
        let Some(hit) = ray_vs_capsule(origin, unit, range, p.position, &DEFAULT_HITBOX) else {
            continue;
        };
        return Some(hit_height_ratio(hit.proj, p.position, &DEFAULT_HITBOX) >= headshot_zone);
    }
    None
}

mod tests {
    use super::*;
    use serde_json::Value;

    fn vec3(value: &Value) -> V3 {
        let a = value.as_array().unwrap();
        [a[0].as_f64().unwrap(), a[1].as_f64().unwrap(), a[2].as_f64().unwrap()]
    }

    #[test]
    fn ray_vs_capsule_matches_the_spoint_hitscan() {
        let fixture: Value = crate::testkit::parse(include_str!("../testdata/parity/spoint-hitscan.json"));
        let mut worst = 0.0_f64;
        for case in fixture["direct"].as_array().unwrap() {
            let got = ray_vs_capsule(vec3(&case["origin"]), vec3(&case["direction"]), case["range"].as_f64().unwrap(), vec3(&case["tp"]), &DEFAULT_HITBOX);
            match (got, case["out"].as_object()) {
                (None, None) => {}
                (Some(hit), Some(want)) => {
                    worst = worst.max((hit.dot - want["dot"].as_f64().unwrap()).abs());
                    let proj = vec3(&case["out"]["proj"]);
                    for i in 0..3 {
                        worst = worst.max((hit.proj[i] - proj[i]).abs());
                    }
                }
                (got, want) => panic!("spoint {want:?} against bevy {got:?} for {}", case["origin"]),
            }
        }
        assert!(worst <= 1e-9, "rayVsCapsule differs from spoint by up to {worst}");
    }

    #[test]
    fn predict_hit_matches_the_spoint_hitscan() {
        let fixture: Value = crate::testkit::parse(include_str!("../testdata/parity/spoint-hitscan.json"));
        let players: Vec<HitTarget> = fixture["players"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| HitTarget { id: p["id"].as_str().unwrap().to_string(), position: vec3(&p["position"]), hp: p["hp"].as_f64() })
            .collect();
        for case in fixture["predict"].as_array().unwrap() {
            let got = predict_hit(vec3(&case["origin"]), vec3(&case["direction"]), fixture["range"].as_f64().unwrap(), &players, case["selfId"].as_str().unwrap(), case["zone"].as_f64().unwrap());
            let want = case["out"]["headshot"].as_bool();
            assert_eq!(got, want, "predictHit for self {} at zone {}", case["selfId"], case["zone"]);
        }
    }
}
