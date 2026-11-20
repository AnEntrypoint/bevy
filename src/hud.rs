#![cfg_attr(not(test), allow(dead_code))]

const HIT_MARKER_WINDOW_MS: f64 = 150.0;
const HEADSHOT_MARKER_WINDOW_MS: f64 = 250.0;
const KILL_CONFIRM_WINDOW_MS: f64 = 1200.0;
const KILL_CONFIRM_WINDOW_MAX_MS: f64 = 2200.0;
const MARKER_RTT_PAD_MAX_MS: f64 = 120.0;
const CROSSHAIR_RECOIL_WINDOW_MS: f64 = 130.0;
const CROSSHAIR_RECOIL_SCALE: f64 = 1.6;
const DMG_DEALT_WINDOW_MS: f64 = 500.0;
const DMG_TAKEN_WINDOW_MS: f64 = 450.0;
const DAMAGE_FLASH_WINDOW_MS: f64 = 220.0;
const DAMAGE_FLASH_OPACITY: f64 = 0.32;
const LOW_HP_VIGNETTE_THRESHOLD: f64 = 30.0;
const VIGNETTE_MAX_OPACITY: f64 = 0.4;
const SHIELD_MAX_OPACITY: f64 = 0.6;
const FALL_PUNCH_SCALE: f64 = 0.18;
const FALL_PUNCH_MIN_VELOCITY: f64 = -9.0;
const HEADSHOT_COLOR: &str = "#ffcc33";
const DEFAULT_COLOR: &str = "#ffffff";

pub fn remaining_ms(now: f64, until: f64) -> f64 {
    until - now
}

pub fn within_ms(now: f64, at: f64, window_ms: f64) -> bool {
    remaining_ms(now, at + window_ms) > 0.0
}

fn marker_rtt_pad_ms(rtt_ms: f64) -> f64 {
    rtt_ms.min(MARKER_RTT_PAD_MAX_MS)
}

pub fn hit_marker_on(now: f64, hit_time: f64, rtt_ms: f64) -> bool {
    within_ms(now, hit_time, HIT_MARKER_WINDOW_MS + marker_rtt_pad_ms(rtt_ms))
}

pub fn headshot_marker_on(now: f64, headshot_time: f64, rtt_ms: f64) -> bool {
    within_ms(now, headshot_time, HEADSHOT_MARKER_WINDOW_MS + marker_rtt_pad_ms(rtt_ms))
}

pub fn kill_confirm_window_ms(rtt_ms: f64) -> f64 {
    (KILL_CONFIRM_WINDOW_MS + rtt_ms).min(KILL_CONFIRM_WINDOW_MAX_MS)
}

pub fn kill_confirm_on(now: f64, kill_time: f64, rtt_ms: f64) -> bool {
    within_ms(now, kill_time, kill_confirm_window_ms(rtt_ms))
}

pub fn hit_marker_color(headshot: bool) -> &'static str {
    if headshot {
        HEADSHOT_COLOR
    } else {
        DEFAULT_COLOR
    }
}

pub fn crosshair_scale(now: f64, last_shoot: f64) -> f64 {
    if within_ms(now, last_shoot, CROSSHAIR_RECOIL_WINDOW_MS) {
        CROSSHAIR_RECOIL_SCALE
    } else {
        1.0
    }
}

pub fn crosshair_transform(scale: f64) -> String {
    format!("translate(-50%,-50%) scale({scale})")
}

pub fn kill_text(on: bool, streak: u32) -> &'static str {
    match (on, streak) {
        (false, _) => "",
        (true, s) if s >= 4 => "MULTI KILL",
        (true, 3) => "TRIPLE KILL",
        (true, 2) => "DOUBLE KILL",
        (true, _) => "KILL",
    }
}

pub fn damage_dealt_text(now: f64, time: f64, dealt: f64) -> String {
    if within_ms(now, time, DMG_DEALT_WINDOW_MS) {
        format!("+{dealt}")
    } else {
        String::new()
    }
}

pub fn damage_taken_text(now: f64, time: f64, taken: f64) -> String {
    if within_ms(now, time, DMG_TAKEN_WINDOW_MS) && taken != 0.0 {
        format!("-{taken}")
    } else {
        String::new()
    }
}

pub fn opacity_of(on: bool) -> f64 {
    if on {
        1.0
    } else {
        0.0
    }
}

pub fn shield_opacity(now: f64, until: f64, invuln_ms: f64) -> f64 {
    let rem = remaining_ms(now, until);
    if rem > 0.0 {
        (rem / invuln_ms * SHIELD_MAX_OPACITY).min(SHIELD_MAX_OPACITY)
    } else {
        0.0
    }
}

pub fn vignette_opacity(now: f64, last_hit: f64, hp: Option<f64>) -> f64 {
    let damage_flash = if within_ms(now, last_hit, DAMAGE_FLASH_WINDOW_MS) { DAMAGE_FLASH_OPACITY } else { 0.0 };
    let low_hp = match hp {
        Some(hp) if hp > 0.0 && hp < LOW_HP_VIGNETTE_THRESHOLD => 0.12 + 0.06 * (now / 180.0).sin(),
        _ => 0.0,
    };
    damage_flash.max(low_hp).min(VIGNETTE_MAX_OPACITY)
}

pub fn reload_progress(now: f64, reloading: bool, reload_end: Option<f64>, reload_duration: f64) -> f64 {
    match reload_end {
        Some(end) if reloading => ((1.0 - (end - now) / reload_duration) * 100.0 + 0.5).floor().min(100.0),
        _ => 0.0,
    }
}

pub fn boost_seconds(now: f64, boost_expires: Option<f64>) -> f64 {
    boost_expires.map_or(0.0, |expires| ((expires - now) / 1000.0).ceil())
}

pub fn fall_punch(on_ground: bool, was_on_ground: Option<bool>, fall_vy: f64) -> Option<f64> {
    (on_ground && was_on_ground == Some(false) && fall_vy < FALL_PUNCH_MIN_VELOCITY).then_some(FALL_PUNCH_SCALE)
}

pub fn ammo_text(ammo: u32, magazine: u32) -> String {
    format!("{ammo}/{magazine}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn number(v: &Value) -> f64 {
        v.as_f64().unwrap()
    }

    fn opacity(v: &Value) -> f64 {
        v.as_str().unwrap().parse::<f64>().unwrap()
    }

    fn field(tps: &Value, key: &str, default: f64) -> f64 {
        tps.get(key).and_then(Value::as_f64).unwrap_or(default)
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-12
    }

    #[test]
    fn hud_math_matches_spoint_at_fixed_inputs() {
        let fixture: Value = serde_json::from_str(include_str!("../testdata/parity/spoint-hud.json")).unwrap();
        let magazine = number(&fixture["combat"]["magazineSize"]) as u32;
        let reload_time = number(&fixture["combat"]["reloadTime"]);
        let invuln_ms = number(&fixture["combat"]["spawnInvulnMs"]);
        for case in fixture["cases"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let inputs = &case["inputs"];
            let out = &case["outputs"];
            let tps = &inputs["tps"];
            let now = number(&inputs["now"]);
            let rtt = number(&inputs["rtt"]);
            let hp = number(&inputs["hp"]);
            let ammo = field(tps, "ammo", magazine as f64) as u32;
            let reloading = tps.get("reloading").and_then(Value::as_bool).unwrap_or(false);
            let reload_end = tps.get("reloadEndTime").and_then(Value::as_f64);
            let boost_expires = tps.get("boost").and_then(|b| b.get("expiresAt")).and_then(Value::as_f64);
            let streak = field(tps, "streak", 1.0) as u32;
            let was_on_ground = tps.get("_wasOnGround").and_then(Value::as_bool);
            let fall_vy = field(tps, "_fallVy", 0.0);
            let hit_time = field(tps, "hitMarkerTime", 0.0);
            let headshot_time = field(tps, "headshotMarkerTime", 0.0);
            let kill_time = field(tps, "killTime", 0.0);
            let last_hit = field(tps, "lastHitTime", 0.0);

            let hit_on = hit_marker_on(now, hit_time, rtt);
            let head_on = headshot_marker_on(now, headshot_time, rtt);
            let kill_on = kill_confirm_on(now, kill_time, rtt);
            let dmg_dealt_on = within_ms(now, field(tps, "dmgDealtTime", 0.0), DMG_DEALT_WINDOW_MS);
            let dmg_taken_on = within_ms(now, last_hit, DMG_TAKEN_WINDOW_MS);
            let punch: Vec<f64> = fall_punch(inputs["onGround"].as_bool().unwrap(), was_on_ground, fall_vy).into_iter().collect();

            assert_eq!(ammo_text(ammo, magazine), out["ammoStat"].as_str().unwrap(), "{name}: ammo");
            assert_eq!(reloading, out["reloadingStat"].as_bool().unwrap(), "{name}: reloading");
            let scale = crosshair_scale(now, field(tps, "lastShootTime", 0.0));
            assert_eq!(crosshair_transform(scale), out["crosshairTransform"].as_str().unwrap(), "{name}: crosshair");
            assert!(close(opacity_of(hit_on || head_on), opacity(&out["hitOpacity"])), "{name}: hit opacity");
            assert_eq!(hit_marker_color(head_on), out["hitColor"].as_str().unwrap(), "{name}: hit color");
            assert_eq!(kill_text(kill_on, streak), out["killText"].as_str().unwrap(), "{name}: kill text");
            assert!(close(opacity_of(kill_on), opacity(&out["killOpacity"])), "{name}: kill opacity");
            assert_eq!(damage_dealt_text(now, field(tps, "dmgDealtTime", 0.0), field(tps, "lastDamageDealt", 0.0)), out["dmgDealtText"].as_str().unwrap(), "{name}: dealt text");
            assert!(close(opacity_of(dmg_dealt_on), opacity(&out["dmgDealtOpacity"])), "{name}: dealt opacity");
            assert_eq!(damage_taken_text(now, last_hit, field(tps, "lastDamageTaken", 0.0)), out["dmgTakenText"].as_str().unwrap(), "{name}: taken text");
            assert!(close(opacity_of(dmg_taken_on), opacity(&out["dmgTakenOpacity"])), "{name}: taken opacity");
            assert!(close(shield_opacity(now, field(tps, "spawnShieldUntil", 0.0), invuln_ms), opacity(&out["shieldOpacity"])), "{name}: shield opacity");
            assert!(close(vignette_opacity(now, last_hit, Some(hp)), opacity(&out["vignetteOpacity"])), "{name}: vignette opacity");
            let want_punch: Vec<f64> = out["punch"].as_array().unwrap().iter().map(number).collect();
            assert_eq!(punch, want_punch, "{name}: punch");

            let ui = &out["ui"];
            assert!(close(hp, number(&ui["hp"])), "{name}: ui hp");
            assert_eq!(ammo, number(&ui["ammo"]) as u32, "{name}: ui ammo");
            assert_eq!(magazine, number(&ui["magazine"]) as u32, "{name}: ui magazine");
            assert_eq!(reloading, ui["reloading"].as_bool().unwrap(), "{name}: ui reloading");
            let progress = reload_progress(now, reloading, reload_end, reload_time);
            assert!(close(progress, number(&ui["reloadProgress"])), "{name}: ui reload progress {progress}");
            assert!(close(boost_seconds(now, boost_expires), number(&ui["boostSec"])), "{name}: ui boost seconds");
            assert!(close(field(tps, "kills", 0.0), number(&ui["kills"])), "{name}: ui kills");
            assert_eq!(hit_on, ui["hitMarkerActive"].as_bool().unwrap(), "{name}: ui hit marker");
            assert_eq!(head_on, ui["headshot"].as_bool().unwrap(), "{name}: ui headshot");
            assert_eq!(kill_on, ui["killConfirm"].as_bool().unwrap(), "{name}: ui kill confirm");
        }
    }
}
