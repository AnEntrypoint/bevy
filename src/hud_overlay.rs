#![allow(dead_code)]

use crate::hud::{
    crosshair_scale, damage_dealt_text, damage_taken_text, headshot_marker_on, hit_marker_color, hit_marker_on,
    kill_confirm_on, kill_text, shield_opacity, vignette_opacity,
};
use bevy::prelude::*;
use bevy::ui::{BackgroundGradient, ColorStop, Gradient, RadialGradient, RadialGradientShape, UiPosition};

const WHITE_HEX: &str = "#ffffff";
const KILL_HEX: &str = "#ffcc33";
const DMG_DEALT_HEX: &str = "#ffe27a";
const DMG_TAKEN_HEX: &str = "#ff7a7a";
const SHIELD_HEX: &str = "#50b4ff";
const VIGNETTE_HEX: &str = "#a00000";
const CROSSHAIR_BAR_ALPHA: f32 = 0.65;
const SHIELD_EDGE_ALPHA: f32 = 0.45;
const VIGNETTE_EDGE_ALPHA: f32 = 0.35;

#[derive(Clone, Copy)]
pub struct HudInputs {
    pub now: f64,
    pub rtt_ms: f64,
    pub last_shoot: f64,
    pub hit_time: f64,
    pub headshot_time: f64,
    pub kill_time: f64,
    pub streak: u32,
    pub dmg_dealt_time: f64,
    pub dmg_dealt: f64,
    pub last_hit: f64,
    pub dmg_taken: f64,
    pub hp: Option<f64>,
    pub shield_until: f64,
    pub invuln_ms: f64,
}

#[derive(Resource, Clone, Debug, PartialEq)]
pub struct HudFrame {
    pub crosshair_scale: f64,
    pub hit_visible: bool,
    pub headshot: bool,
    pub kill_text: &'static str,
    pub dmg_dealt: String,
    pub dmg_taken: String,
    pub shield: f64,
    pub vignette: f64,
}

impl Default for HudFrame {
    fn default() -> Self {
        Self {
            crosshair_scale: 1.0,
            hit_visible: false,
            headshot: false,
            kill_text: "",
            dmg_dealt: String::new(),
            dmg_taken: String::new(),
            shield: 0.0,
            vignette: 0.0,
        }
    }
}

pub fn frame_at(i: &HudInputs) -> HudFrame {
    let headshot = headshot_marker_on(i.now, i.headshot_time, i.rtt_ms);
    let hit = hit_marker_on(i.now, i.hit_time, i.rtt_ms);
    HudFrame {
        crosshair_scale: crosshair_scale(i.now, i.last_shoot),
        hit_visible: hit || headshot,
        headshot,
        kill_text: kill_text(kill_confirm_on(i.now, i.kill_time, i.rtt_ms), i.streak),
        dmg_dealt: damage_dealt_text(i.now, i.dmg_dealt_time, i.dmg_dealt),
        dmg_taken: damage_taken_text(i.now, i.last_hit, i.dmg_taken),
        shield: shield_opacity(i.now, i.shield_until, i.invuln_ms),
        vignette: vignette_opacity(i.now, i.last_hit, i.hp),
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum HudElement {
    Crosshair,
    HitMarker,
    HitBar,
    Kill,
    DmgDealt,
    DmgTaken,
    Shield,
    Vignette,
}

fn hex(code: &str) -> Color {
    let channel = |at: usize| u8::from_str_radix(&code[at..at + 2], 16).unwrap();
    Color::srgb_u8(channel(1), channel(3), channel(5))
}

fn absolute(left: Val, top: Val, width: Val, height: Val) -> Node {
    Node { position_type: PositionType::Absolute, left, top, width, height, ..default() }
}

fn centered() -> UiTransform {
    UiTransform { translation: Val2::percent(-50.0, -50.0), ..default() }
}

fn label(size: f32, code: &str) -> (Text, TextFont, TextColor) {
    (
        Text::new(""),
        TextFont { font_size: FontSize::Px(size), ..default() },
        TextColor(hex(code).with_alpha(0.0)),
    )
}

fn edge_gradient(code: &str, alpha: f32) -> BackgroundGradient {
    let edge = hex(code).with_alpha(alpha);
    BackgroundGradient(vec![Gradient::Radial(RadialGradient::new(
        UiPosition::CENTER,
        RadialGradientShape::FarthestCorner,
        vec![ColorStop::new(Color::NONE, Val::Percent(55.0)), ColorStop::new(edge, Val::Percent(100.0))],
    ))])
}

fn spawn_overlay(mut commands: Commands) {
    let bar = hex(WHITE_HEX).with_alpha(CROSSHAIR_BAR_ALPHA);
    commands
        .spawn((
            HudElement::Crosshair,
            absolute(Val::Percent(50.0), Val::Percent(50.0), Val::Px(18.0), Val::Px(18.0)),
            centered(),
        ))
        .with_children(|cross| {
            cross.spawn((absolute(Val::Px(8.0), Val::Px(0.0), Val::Px(2.0), Val::Px(18.0)), BackgroundColor(bar)));
            cross.spawn((absolute(Val::Px(0.0), Val::Px(8.0), Val::Px(18.0), Val::Px(2.0)), BackgroundColor(bar)));
        });
    commands
        .spawn((
            HudElement::HitMarker,
            absolute(Val::Percent(50.0), Val::Percent(50.0), Val::Px(0.0), Val::Px(0.0)),
            UiTransform { rotation: Rot2::degrees(45.0), ..centered() },
            Visibility::Hidden,
        ))
        .with_children(|hit| {
            for (left, top, width, height) in [(-1.0, -13.0, 2.0, 8.0), (-1.0, 5.0, 2.0, 8.0), (-13.0, -1.0, 8.0, 2.0), (5.0, -1.0, 8.0, 2.0)] {
                hit.spawn((
                    HudElement::HitBar,
                    absolute(Val::Px(left), Val::Px(top), Val::Px(width), Val::Px(height)),
                    BackgroundColor(hex(WHITE_HEX)),
                ));
            }
        });
    commands.spawn((
        HudElement::Kill,
        absolute(Val::Percent(50.0), Val::Percent(36.0), Val::Auto, Val::Auto),
        centered(),
        label(34.0, KILL_HEX),
    ));
    commands.spawn((
        HudElement::DmgDealt,
        absolute(Val::Percent(52.0), Val::Percent(47.0), Val::Auto, Val::Auto),
        label(17.0, DMG_DEALT_HEX),
    ));
    commands.spawn((
        HudElement::DmgTaken,
        absolute(Val::Percent(50.0), Val::Percent(41.0), Val::Auto, Val::Auto),
        centered(),
        label(24.0, DMG_TAKEN_HEX),
    ));
    commands.spawn((
        HudElement::Shield,
        absolute(Val::Px(0.0), Val::Px(0.0), Val::Percent(100.0), Val::Percent(100.0)),
        edge_gradient(SHIELD_HEX, 0.0),
    ));
    commands.spawn((
        HudElement::Vignette,
        absolute(Val::Px(0.0), Val::Px(0.0), Val::Percent(100.0), Val::Percent(100.0)),
        edge_gradient(VIGNETTE_HEX, 0.0),
    ));
}

fn set_label(value: &str, text: Option<&mut Text>, color: Option<&mut TextColor>, code: &str) {
    if let Some(text) = text {
        if text.0 != value {
            text.0 = value.to_string();
        }
    }
    if let Some(color) = color {
        let alpha = if value.is_empty() { 0.0 } else { 1.0 };
        color.0 = hex(code).with_alpha(alpha);
    }
}

fn update_hud(
    frame: Res<HudFrame>,
    mut parts: Query<(
        &HudElement,
        Option<&mut Visibility>,
        Option<&mut UiTransform>,
        Option<&mut Text>,
        Option<&mut TextColor>,
        Option<&mut BackgroundColor>,
        Option<&mut BackgroundGradient>,
    )>,
) {
    for (element, visibility, transform, mut text, mut color, background, gradient) in &mut parts {
        match element {
            HudElement::Crosshair => {
                if let Some(mut transform) = transform {
                    transform.scale = Vec2::splat(frame.crosshair_scale as f32);
                }
            }
            HudElement::HitMarker => {
                if let Some(mut visibility) = visibility {
                    *visibility = if frame.hit_visible { Visibility::Visible } else { Visibility::Hidden };
                }
            }
            HudElement::HitBar => {
                if let Some(mut background) = background {
                    background.0 = hex(hit_marker_color(frame.headshot));
                }
            }
            HudElement::Kill => set_label(frame.kill_text, text.as_deref_mut(), color.as_deref_mut(), KILL_HEX),
            HudElement::DmgDealt => set_label(&frame.dmg_dealt, text.as_deref_mut(), color.as_deref_mut(), DMG_DEALT_HEX),
            HudElement::DmgTaken => set_label(&frame.dmg_taken, text.as_deref_mut(), color.as_deref_mut(), DMG_TAKEN_HEX),
            HudElement::Shield => {
                if let Some(mut gradient) = gradient {
                    *gradient = edge_gradient(SHIELD_HEX, SHIELD_EDGE_ALPHA * frame.shield as f32);
                }
            }
            HudElement::Vignette => {
                if let Some(mut gradient) = gradient {
                    *gradient = edge_gradient(VIGNETTE_HEX, VIGNETTE_EDGE_ALPHA * frame.vignette as f32);
                }
            }
        }
    }
}

pub struct HudOverlayPlugin;

impl Plugin for HudOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudFrame>().add_systems(Startup, spawn_overlay).add_systems(Update, update_hud);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use serde_json::Value;

    type Part = (HudElement, Val, Val, Option<UiTransform>, Option<Visibility>, Option<String>, Option<Color>, Option<Color>);

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

    fn inputs_of(case: &Value, invuln_ms: f64) -> HudInputs {
        let i = &case["inputs"];
        let tps = &i["tps"];
        HudInputs {
            now: number(&i["now"]),
            rtt_ms: number(&i["rtt"]),
            last_shoot: field(tps, "lastShootTime", 0.0),
            hit_time: field(tps, "hitMarkerTime", 0.0),
            headshot_time: field(tps, "headshotMarkerTime", 0.0),
            kill_time: field(tps, "killTime", 0.0),
            streak: field(tps, "streak", 1.0) as u32,
            dmg_dealt_time: field(tps, "dmgDealtTime", 0.0),
            dmg_dealt: field(tps, "lastDamageDealt", 0.0),
            last_hit: field(tps, "lastHitTime", 0.0),
            dmg_taken: field(tps, "lastDamageTaken", 0.0),
            hp: i.get("hp").and_then(Value::as_f64),
            shield_until: field(tps, "spawnShieldUntil", 0.0),
            invuln_ms,
        }
    }

    fn render(frame: HudFrame) -> Vec<Part> {
        let mut world = World::new();
        world.run_system_once(spawn_overlay).unwrap();
        world.insert_resource(frame);
        world.run_system_once(update_hud).unwrap();
        let mut query = world.query::<(
            &HudElement,
            &Node,
            Option<&UiTransform>,
            Option<&Visibility>,
            Option<&Text>,
            Option<&TextColor>,
            Option<&BackgroundColor>,
        )>();
        query
            .iter(&world)
            .map(|(kind, node, transform, visibility, text, text_color, background)| {
                (
                    *kind,
                    node.left,
                    node.top,
                    transform.copied(),
                    visibility.copied(),
                    text.map(|t| t.0.clone()),
                    text_color.map(|c| c.0),
                    background.map(|b| b.0),
                )
            })
            .collect()
    }

    fn only(parts: &[Part], kind: HudElement) -> &Part {
        parts.iter().find(|p| p.0 == kind).unwrap()
    }

    fn alpha_of(color: Option<Color>) -> f64 {
        color.unwrap().to_srgba().alpha as f64
    }

    fn to_hex(color: Color) -> String {
        let s = color.to_srgba();
        format!(
            "#{:02x}{:02x}{:02x}",
            (s.red * 255.0).round() as u8,
            (s.green * 255.0).round() as u8,
            (s.blue * 255.0).round() as u8
        )
    }

    #[test]
    fn hud_overlay_nodes_match_spoint_at_fixed_inputs() {
        let fixture: Value = serde_json::from_str(include_str!("../testdata/parity/spoint-hud.json")).unwrap();
        let invuln = number(&fixture["combat"]["spawnInvulnMs"]);
        for case in fixture["cases"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let out = &case["outputs"];
            let frame = frame_at(&inputs_of(case, invuln));
            let shield = frame.shield;
            let vignette = frame.vignette;
            let parts = render(frame);

            let kill = only(&parts, HudElement::Kill);
            assert_eq!(kill.5.as_deref(), Some(out["killText"].as_str().unwrap()), "{name}: kill text");
            assert_eq!(alpha_of(kill.6), opacity(&out["killOpacity"]), "{name}: kill opacity");

            let dealt = only(&parts, HudElement::DmgDealt);
            assert_eq!(dealt.5.as_deref(), Some(out["dmgDealtText"].as_str().unwrap()), "{name}: dealt text");
            assert_eq!(alpha_of(dealt.6), opacity(&out["dmgDealtOpacity"]), "{name}: dealt opacity");

            let taken = only(&parts, HudElement::DmgTaken);
            assert_eq!(taken.5.as_deref(), Some(out["dmgTakenText"].as_str().unwrap()), "{name}: taken text");
            assert_eq!(alpha_of(taken.6), opacity(&out["dmgTakenOpacity"]), "{name}: taken opacity");

            let hit = only(&parts, HudElement::HitMarker);
            let want_visible = if opacity(&out["hitOpacity"]) == 1.0 { Visibility::Visible } else { Visibility::Hidden };
            assert_eq!(hit.4, Some(want_visible), "{name}: hit visibility");
            for bar in parts.iter().filter(|p| p.0 == HudElement::HitBar) {
                assert_eq!(to_hex(bar.7.unwrap()), out["hitColor"].as_str().unwrap(), "{name}: hit color");
            }

            let cross = only(&parts, HudElement::Crosshair);
            let scale = cross.3.unwrap().scale.x;
            assert_eq!(format!("translate(-50%,-50%) scale({scale})"), out["crosshairTransform"].as_str().unwrap(), "{name}: crosshair");

            assert!(close(shield, opacity(&out["shieldOpacity"])), "{name}: shield opacity");
            assert!(close(vignette, opacity(&out["vignetteOpacity"])), "{name}: vignette opacity");
        }
    }

    #[test]
    fn hud_overlay_positions_match_spoint_css() {
        let parts = render(HudFrame::default());
        let at = |kind: HudElement| only(&parts, kind).clone();
        let placed = [
            (HudElement::Crosshair, 50.0, 50.0, true),
            (HudElement::HitMarker, 50.0, 50.0, true),
            (HudElement::Kill, 50.0, 36.0, true),
            (HudElement::DmgDealt, 52.0, 47.0, false),
            (HudElement::DmgTaken, 50.0, 41.0, true),
        ];
        for (kind, left, top, centered_on_point) in placed {
            let part = at(kind);
            assert_eq!(part.1, Val::Percent(left), "{kind:?}: left");
            assert_eq!(part.2, Val::Percent(top), "{kind:?}: top");
            let translation = part.3.map(|t| t.translation);
            let want = if centered_on_point { Val2::percent(-50.0, -50.0) } else { Val2::default() };
            assert_eq!(translation, Some(want), "{kind:?}: translate");
        }
        assert_eq!(at(HudElement::Kill).5.as_deref(), Some(""));
    }
}
