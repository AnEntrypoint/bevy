use super::light::Quality;
use bevy::asset::uuid_handle;
use bevy::camera::Hdr;
use bevy::core_pipeline::fullscreen_material::{FullscreenMaterial, FullscreenMaterialPlugin};
use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::prelude::*;
use bevy::render::RenderApp;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_resource::ShaderType;
use bevy::shader::{Shader, ShaderRef};

const TONEMAP_SHADER: Handle<Shader> = uuid_handle!("3b8c5e1a-2f47-4d96-8a0e-6c1d9b7f4e23");
const TONEMAP_SHADER_PATH: &str = "shaders/planet/atmosphere_tonemap.wgsl";

#[cfg(test)]
mod twin {
    use bevy::math::{DMat3, DVec3};

    pub const DISPLAY_CLAMP: f64 = 0.995;
    pub const ACES_EXPOSURE_DIVISOR: f64 = 0.6;
    pub const ACES_INPUT_ROWS: [[f64; 3]; 3] = [[0.59719, 0.35458, 0.04823], [0.076, 0.90834, 0.01566], [0.0284, 0.13383, 0.83777]];
    pub const ACES_OUTPUT_ROWS: [[f64; 3]; 3] = [[1.60475, -0.53108, -0.07367], [-0.10208, 1.10813, -0.00605], [-0.00327, -0.07276, 1.07602]];
    pub const RRT_QUADRATIC_SHIFT: f64 = 0.0245786;
    pub const RRT_NUMERATOR_FLOOR: f64 = 0.000090537;
    pub const RRT_LINEAR_SHIFT: f64 = 0.4329510;
    pub const RRT_DENOMINATOR_SLOPE: f64 = 0.983729;
    pub const RRT_DENOMINATOR_FLOOR: f64 = 0.238081;

    pub fn matrix(rows: [[f64; 3]; 3]) -> DMat3 {
        DMat3::from_cols_array_2d(&rows).transpose()
    }

    pub fn rrt_odt_fit(v: DVec3) -> DVec3 {
        let numerator = v * (v + RRT_QUADRATIC_SHIFT) - RRT_NUMERATOR_FLOOR;
        let denominator = v * ((v + RRT_LINEAR_SHIFT) * RRT_DENOMINATOR_SLOPE) + RRT_DENOMINATOR_FLOOR;
        numerator / denominator
    }

    pub fn aces_filmic(colour: DVec3, exposure: f64) -> DVec3 {
        let fitted = rrt_odt_fit(matrix(ACES_INPUT_ROWS) * (colour * exposure / ACES_EXPOSURE_DIVISOR));
        (matrix(ACES_OUTPUT_ROWS) * fitted).clamp(DVec3::ZERO, DVec3::ONE)
    }

    pub fn invert_aces_filmic(display_linear: DVec3, exposure: f64) -> DVec3 {
        let z = (matrix(ACES_OUTPUT_ROWS).inverse() * display_linear).clamp(DVec3::ZERO, DVec3::splat(DISPLAY_CLAMP));
        let a = z * RRT_DENOMINATOR_SLOPE - 1.0;
        let b = z * (RRT_DENOMINATOR_SLOPE * RRT_LINEAR_SHIFT) - RRT_QUADRATIC_SHIFT;
        let c = z * RRT_DENOMINATOR_FLOOR + RRT_NUMERATOR_FLOOR;
        let discriminant = (b * b - a * c * 4.0).max(DVec3::ZERO);
        let y = (-b - discriminant.map(f64::sqrt)) / (a * 2.0);
        matrix(ACES_INPUT_ROWS).inverse() * y * (ACES_EXPOSURE_DIVISOR / exposure)
    }

    pub fn srgb_oetf(linear: DVec3) -> DVec3 {
        linear.map(|c| if c < 0.0031308 { c * 12.92 } else { 1.055 * c.powf(0.41666) - 0.055 })
    }

    pub fn srgb_to_linear(display: DVec3) -> DVec3 {
        display.map(|c| if c < 0.04045 { c * 0.0773993808 } else { (c * 0.9478672986 + 0.0521327014).powf(2.4) })
    }

    pub fn scene_linear_to_display_referred(linear: DVec3, exposure: f64, aces: bool) -> DVec3 {
        let mapped = if aces { aces_filmic(linear, exposure) } else { linear };
        srgb_oetf(mapped.clamp(DVec3::ZERO, DVec3::ONE))
    }

    pub fn display_referred_to_scene_linear(display: DVec3, exposure: f64, aces: bool) -> DVec3 {
        let t = srgb_to_linear(display).clamp(DVec3::ZERO, DVec3::splat(DISPLAY_CLAMP));
        if aces { invert_aces_filmic(t, exposure) } else { t }
    }
}

#[cfg(test)]
pub use twin::*;

#[derive(Component, ExtractComponent, Clone, Copy, ShaderType, Default, Debug, PartialEq)]
#[extract_app(RenderApp)]
pub struct ToneMap {
    pub exposure: f32,
    pub aces: f32,
    pub padding: Vec2,
}

impl ToneMap {
    pub fn for_quality(quality: Quality) -> Self {
        Self { exposure: quality.exposure(), aces: 1.0, padding: Vec2::ZERO }
    }
}

impl FullscreenMaterial for ToneMap {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(TONEMAP_SHADER)
    }
}

pub fn reference_view(quality: Quality) -> impl Bundle {
    (Hdr, Tonemapping::None, DebandDither::Disabled, super::light::camera_exposure(), ToneMap::for_quality(quality))
}

pub struct ToneMapPlugin;

impl Plugin for ToneMapPlugin {
    fn build(&self, app: &mut App) {
        let shader = Shader::from_wgsl(include_str!("../../assets/shaders/planet/atmosphere_tonemap.wgsl"), TONEMAP_SHADER_PATH);
        if let Some(mut shaders) = app.world_mut().get_resource_mut::<Assets<Shader>>() {
            let _ = shaders.insert(TONEMAP_SHADER.id(), shader);
        }
        app.add_plugins(FullscreenMaterialPlugin::<ToneMap>::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::offscreen::{add_target, capture, headless_app, render_lock, texel};
    use crate::testkit::{cases, close, parse};
    use bevy::math::DVec3;
    use bevy::camera::{ClearColorConfig, RenderTarget};
    use bevy::render::render_resource::TextureFormat;
    use serde_json::Value;

    const INVERSE_TOLERANCE: f64 = 1e-12;
    const ROUND_TRIP_TOLERANCE: f64 = 2e-5;
    const GPU_TOLERANCE_LEVELS: f64 = 1.0;
    const CARDS: [[f64; 3]; 5] = [[0.18, 0.18, 0.18], [2.0, 0.5, 0.05], [0.02, 0.4, 1.5], [0.9, 0.9, 0.9], [0.0, 0.0, 0.0]];

    fn fixture() -> Value {
        parse(include_str!("../../testdata/parity/spoint-planet-atmosphere.json"))
    }

    fn rows(value: &Value) -> [[f64; 3]; 3] {
        let row = |r: usize| [0, 1, 2].map(|c| value[r][c].as_f64().unwrap());
        [row(0), row(1), row(2)]
    }

    fn expected_bytes(card: [f64; 3], exposure: f64, aces: bool) -> [f64; 3] {
        let linear = DVec3::from_array(card);
        let display = if aces { aces_filmic(linear, exposure) } else { linear.clamp(DVec3::ZERO, DVec3::ONE) };
        (srgb_oetf(display) * 255.0).to_array()
    }

    #[test]
    fn display_referred_constants_are_the_references() {
        let f = fixture();
        let constants = &f["displayReferred"]["constants"];
        assert_eq!(rows(&constants["ACES_INPUT_ROWS"]), ACES_INPUT_ROWS);
        assert_eq!(rows(&constants["ACES_OUTPUT_ROWS"]), ACES_OUTPUT_ROWS);
        assert_eq!(constants["DISPLAY_CLAMP"].as_f64().unwrap(), DISPLAY_CLAMP);
        let input_inverse = rows(&f["displayReferred"]["acesInputInverseRows"]);
        let output_inverse = rows(&f["displayReferred"]["acesOutputInverseRows"]);
        for (r, row) in (0..3).zip(0..3) {
            for c in 0..3 {
                let _ = row;
                close(matrix(ACES_INPUT_ROWS).inverse().row(r)[c], input_inverse[r][c], INVERSE_TOLERANCE, "input inverse");
                close(matrix(ACES_OUTPUT_ROWS).inverse().row(r)[c], output_inverse[r][c], INVERSE_TOLERANCE, "output inverse");
            }
        }
    }

    #[test]
    fn the_curve_literals_are_the_reference_source_text() {
        let f = fixture();
        let inverse = f["displayReferred"]["invertAcesFilmicSource"].as_str().unwrap();
        for literal in [RRT_QUADRATIC_SHIFT, RRT_NUMERATOR_FLOOR, RRT_LINEAR_SHIFT, RRT_DENOMINATOR_SLOPE, RRT_DENOMINATOR_FLOOR, ACES_EXPOSURE_DIVISOR] {
            let text = format!("{literal}");
            let in_forward = f["displayReferred"]["sceneLinearToDisplayReferredSource"].as_str().unwrap().contains(&text);
            let in_back = f["displayReferred"]["displayReferredToSceneLinearSource"].as_str().unwrap().contains(&text);
            assert!(inverse.contains(&text) || in_forward || in_back, "literal {text} is in no reference source");
        }
    }

    #[test]
    fn three_reads_its_matrices_as_written_rows_so_grey_stays_grey() {
        for grey in [0.01, 0.18, 0.5, 1.0, 4.0] {
            let mapped = aces_filmic(DVec3::splat(grey), 1.0);
            close(mapped.x, mapped.y, 1e-4, "grey r against g");
            close(mapped.y, mapped.z, 1e-4, "grey g against b");
        }
    }

    #[test]
    fn display_referred_round_trips_below_the_clamp() {
        for exposure in [0.8, 1.0] {
            for step in 0..=40 {
                let d = f64::from(step) / 40.0 * 0.97;
                let display = DVec3::new(d, d * 0.8, (d * 1.3).min(0.97));
                let back = scene_linear_to_display_referred(display_referred_to_scene_linear(display, exposure, true), exposure, true);
                for c in 0..3 {
                    close(back[c], display[c], ROUND_TRIP_TOLERANCE, &format!("round trip exposure {exposure} step {step}"));
                }
            }
        }
    }

    #[test]
    fn the_clamp_caps_what_the_inverse_can_return() {
        let bright = display_referred_to_scene_linear(DVec3::ONE, 1.0, true);
        let capped = display_referred_to_scene_linear(srgb_oetf(DVec3::splat(DISPLAY_CLAMP)), 1.0, true);
        assert!(bright.x.is_finite() && (bright - capped).abs().max_element() < 1e-9, "{bright:?} against {capped:?}");
        let through = scene_linear_to_display_referred(bright, 1.0, true);
        assert!(through.x < 1.0 && through.x > 0.99, "{through:?}");
    }

    #[test]
    fn the_view_bundle_carries_the_quality_exposure() {
        assert_eq!(ToneMap::for_quality(Quality::Low).exposure, 0.8);
        assert_eq!(ToneMap::for_quality(Quality::Ultra).exposure, 1.0);
        assert_eq!(ToneMap::for_quality(Quality::High).aces, 1.0);
    }

    fn set_card(app: &mut App, card: [f64; 3], tonemap: Option<ToneMap>) {
        let colour = Color::linear_rgb(card[0] as f32, card[1] as f32, card[2] as f32);
        let mut cameras = app.world_mut().query::<(Entity, &mut Camera)>();
        let entity = cameras.iter_mut(app.world_mut()).map(|(entity, mut camera)| {
            camera.clear_color = ClearColorConfig::Custom(colour);
            entity
        }).next().unwrap();
        match tonemap {
            Some(settings) => app.world_mut().entity_mut(entity).insert(settings),
            None => app.world_mut().entity_mut(entity).remove::<ToneMap>(),
        };
    }

    fn centre_bytes(image: &Image) -> [f64; 3] {
        let t = texel(image, 160, 90);
        [f64::from(t[0]), f64::from(t[1]), f64::from(t[2])]
    }

    #[test]
    fn the_gpu_pass_matches_the_cpu_mirror_on_flat_cards_and_changes_the_image() {
        let _guard = render_lock();
        let mut app = headless_app();
        app.add_plugins(ToneMapPlugin);
        app.finish();
        app.cleanup();
        app.update();
        let target = add_target(&mut app, TextureFormat::Rgba8UnormSrgb);
        app.world_mut().spawn((Camera3d::default(), RenderTarget::from(target.clone()), Msaa::Off, reference_view(Quality::Ultra)));
        let mut worst = 0.0f64;
        let mut changed = 0usize;
        for exposure_quality in [Quality::Ultra, Quality::Low] {
            for card in CARDS {
                set_card(&mut app, card, Some(ToneMap::for_quality(exposure_quality)));
                let with_pass = capture(&mut app, target.clone(), "tone map card");
                let want = expected_bytes(card, f64::from(exposure_quality.exposure()), true);
                let got = centre_bytes(&with_pass);
                for c in 0..3 {
                    worst = worst.max((got[c] - want[c]).abs());
                    assert!((got[c] - want[c]).abs() <= GPU_TOLERANCE_LEVELS, "card {card:?} {exposure_quality:?} channel {c}: gpu {got:?} against mirror {want:?}");
                }
                set_card(&mut app, card, None);
                let without = capture(&mut app, target.clone(), "no tone map card");
                let plain = expected_bytes(card, 1.0, false);
                let got_plain = centre_bytes(&without);
                for c in 0..3 {
                    assert!((got_plain[c] - plain[c]).abs() <= GPU_TOLERANCE_LEVELS, "plain card {card:?} channel {c}: gpu {got_plain:?} against clamp {plain:?}");
                }
                if (0..3).any(|c| (got[c] - got_plain[c]).abs() > 8.0) {
                    changed += 1;
                }
            }
        }
        println!("tone map worst gpu against mirror: {worst} levels, cards changed by the pass: {changed}");
        assert!(changed >= 6, "the pass changed only {changed} of 10 cards");
        let _ = cases;
    }
}
