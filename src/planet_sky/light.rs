use super::sky::SkySun;
use bevy::camera::Exposure;
use bevy::light::{CascadeShadowConfig, DirectionalLightShadowMap, GlobalAmbientLight};
use bevy::math::DVec3;
use bevy::prelude::*;
use std::f64::consts::{PI, TAU};
use std::sync::LazyLock;

pub const START_FRACTION: f64 = 0.3;
pub const DAY_LENGTH_S: f64 = 600.0;
pub const TILT_DEG: f64 = 23.0;
pub const SHADOW_MAP_SIZE: usize = 1024;
pub const CASCADE_FIRST_FAR_M: f32 = 60.0;
pub const CASCADE_SPLIT: f32 = 3.2;
pub const CASCADE_BLEND_M: f32 = 4.0;
pub const CASCADE_MIN_DISTANCE_M: f32 = 0.1;
pub const SCENE_SUN_INTENSITY: f32 = 1.5;
pub const SCENE_AMBIENT_HEX: u32 = 0xfff4d6;
pub const SCENE_AMBIENT_INTENSITY: f64 = 0.5;
pub const FILL_HEX: u32 = 0x4488ff;
pub const FILL_INTENSITY: f64 = 0.4;
pub const FILL_FROM: Vec3 = Vec3::new(-20.0, 30.0, -10.0);
const CAMERA_EXPOSURE_DIVISOR: f32 = 1.2;
const TERRAIN_AMBIENT_FLOOR_ALBEDO: f64 = 0.14;
const LUMA: [f64; 3] = [0.2126, 0.7152, 0.0722];

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Quality {
    Low,
    Medium,
    High,
    #[default]
    Ultra,
}

impl Quality {
    pub fn exposure(self) -> f32 {
        if self == Quality::Low { 0.8 } else { 1.0 }
    }

    pub fn shadows_enabled(self) -> bool {
        self != Quality::Low
    }

    pub fn cascade_count(self) -> usize {
        match self {
            Quality::Low | Quality::Medium => 1,
            Quality::High => 2,
            Quality::Ultra => 3,
        }
    }
}

pub fn camera_exposure() -> Exposure {
    Exposure { ev100: -CAMERA_EXPOSURE_DIVISOR.log2() }
}

pub fn cascade_config(count: usize) -> CascadeShadowConfig {
    let bounds = (0..count).map(|index| (f64::from(CASCADE_FIRST_FAR_M) * f64::from(CASCADE_SPLIT).powi(index as i32)) as f32).collect();
    CascadeShadowConfig { bounds, overlap_proportion: CASCADE_BLEND_M / CASCADE_FIRST_FAR_M, minimum_distance: CASCADE_MIN_DISTANCE_M }
}

fn srgb_to_linear(c: f64) -> f64 {
    if c < 0.04045 { c * 0.0773993808 } else { (c * 0.9478672986 + 0.0521327014).powf(2.4) }
}

fn linear_to_srgb(c: f64) -> f64 {
    if c < 0.0031308 { c * 12.92 } else { 1.055 * c.powf(0.41666) - 0.055 }
}

fn hex_to_linear(hex: u32) -> [f64; 3] {
    [16, 8, 0].map(|shift| srgb_to_linear(f64::from((hex >> shift) & 255) / 255.0))
}

fn linear_to_hex(linear: [f64; 3]) -> u32 {
    let byte = |c: f64| (linear_to_srgb(c) * 255.0).clamp(0.0, 255.0).add(0.5).floor() as u32;
    byte(linear[0]) << 16 | byte(linear[1]) << 8 | byte(linear[2])
}

trait Add {
    fn add(self, rhs: f64) -> f64;
}

impl Add for f64 {
    fn add(self, rhs: f64) -> f64 {
        self + rhs
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Keyframe {
    pub deg: f64,
    pub sun: [f64; 3],
    pub sun_intensity: f64,
    pub ambient: [f64; 3],
    pub ambient_intensity: f64,
    pub fill_scale: f64,
}

fn ambient_matching_terrain_floor(hex: u32) -> f64 {
    let linear = hex_to_linear(hex);
    TERRAIN_AMBIENT_FLOOR_ALBEDO * PI / (0..3).map(|c| LUMA[c] * linear[c]).sum::<f64>()
}

fn keyframe(deg: f64, sun_hex: u32, sun_intensity: f64, ambient_hex: u32, ambient_intensity: Option<f64>, fill_scale: f64) -> Keyframe {
    Keyframe {
        deg,
        sun: hex_to_linear(sun_hex),
        sun_intensity,
        ambient: hex_to_linear(ambient_hex),
        ambient_intensity: ambient_intensity.unwrap_or_else(|| ambient_matching_terrain_floor(ambient_hex)),
        fill_scale,
    }
}

pub static KEYFRAMES: LazyLock<[Keyframe; 7]> = LazyLock::new(|| {
    [
        keyframe(-90.0, 0x0a1030, 0.0, 0xb4c3e6, None, 0.44),
        keyframe(-6.0, 0x1a2550, 0.0, 0xb9c1df, None, 0.52),
        keyframe(0.0, 0xff7a3c, 0.55, 0xc8b6c8, None, 0.8),
        keyframe(8.0, 0xffa552, 1.8, 0xffcf9e, Some(0.65), 1.3),
        keyframe(30.0, 0xfff0dc, 1.45, 0xc9d4e8, Some(0.48), 0.96),
        keyframe(70.0, 0xffffff, 1.6, 0xfff4d6, Some(0.5), 1.0),
        keyframe(90.0, 0xffffff, 1.55, 0xfff4d6, Some(0.5), 1.0),
    ]
});

#[derive(Clone, Copy, Debug)]
pub struct SkyLight {
    #[cfg(test)]
    pub elevation_deg: f64,
    pub direction: DVec3,
    pub sun_hex: u32,
    pub sun_intensity: f64,
    pub ambient_hex: u32,
    pub ambient_intensity: f64,
    pub fill_scale: f64,
}

impl SkyLight {
    pub fn sun_linear(&self) -> [f64; 3] {
        hex_to_linear(self.sun_hex)
    }
}

fn mix_colour(lo: [f64; 3], hi: [f64; 3], f: f64) -> u32 {
    linear_to_hex([0, 1, 2].map(|c| lo[c] + (hi[c] - lo[c]) * f))
}

fn mix_scalar(lo: f64, hi: f64, f: f64) -> f64 {
    (1.0 - f) * lo + f * hi
}

fn bracket(elevation_deg: f64) -> (Keyframe, Keyframe) {
    let frames = &*KEYFRAMES;
    let last = frames.len() - 1;
    if elevation_deg <= frames[0].deg {
        return (frames[0], frames[0]);
    }
    if elevation_deg >= frames[last].deg {
        return (frames[last], frames[last]);
    }
    let index = (0..last).find(|&i| elevation_deg >= frames[i].deg && elevation_deg <= frames[i + 1].deg).unwrap_or(0);
    (frames[index], frames[index + 1])
}

pub fn evaluate(fraction: f64) -> SkyLight {
    let elevation_deg = ((fraction - 0.25) * TAU).sin() * (90.0 - TILT_DEG);
    let azimuth = fraction * TAU;
    let elevation = elevation_deg.to_radians();
    let direction = DVec3::new(elevation.cos() * azimuth.sin(), elevation.sin(), elevation.cos() * azimuth.cos());
    let (lo, hi) = bracket(elevation_deg);
    let span = hi.deg - lo.deg;
    let f = if span > 0.0 { ((elevation_deg - lo.deg) / span).clamp(0.0, 1.0) } else { 0.0 };
    SkyLight {
        #[cfg(test)]
        elevation_deg,
        direction,
        sun_hex: mix_colour(lo.sun, hi.sun, f),
        sun_intensity: mix_scalar(lo.sun_intensity, hi.sun_intensity, f),
        ambient_hex: mix_colour(lo.ambient, hi.ambient, f),
        ambient_intensity: mix_scalar(lo.ambient_intensity, hi.ambient_intensity, f),
        fill_scale: mix_scalar(lo.fill_scale, hi.fill_scale, f),
    }
}

pub fn advance(fraction: f64, dt_s: f64, day_length_s: f64) -> f64 {
    let next = fraction + dt_s / day_length_s;
    next - next.floor()
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct TimeOfDayClock {
    pub fraction: f64,
    pub day_length_s: f64,
    pub paused: bool,
}

impl Default for TimeOfDayClock {
    fn default() -> Self {
        Self { fraction: START_FRACTION, day_length_s: DAY_LENGTH_S, paused: false }
    }
}

#[derive(Component)]
pub struct Sun;

#[derive(Component)]
pub struct Fill;

fn colour(linear: [f64; 3]) -> Color {
    Color::linear_rgb(linear[0] as f32, linear[1] as f32, linear[2] as f32)
}

fn ambient_light(hex: u32, intensity: f64) -> GlobalAmbientLight {
    GlobalAmbientLight { color: colour(hex_to_linear(hex)), brightness: (intensity / PI) as f32, ..default() }
}

fn spawn_lights(mut commands: Commands, quality: Res<Quality>) {
    commands.spawn((
        Sun,
        DirectionalLight { illuminance: SCENE_SUN_INTENSITY, shadow_maps_enabled: quality.shadows_enabled(), ..default() },
        cascade_config(quality.cascade_count()),
        Transform::IDENTITY.looking_to(Vec3::NEG_Y, Vec3::Z),
    ));
    commands.spawn((
        Fill,
        DirectionalLight { color: colour(hex_to_linear(FILL_HEX)), illuminance: FILL_INTENSITY as f32, shadow_maps_enabled: false, ..default() },
        Transform::IDENTITY.looking_to(-FILL_FROM, Vec3::Y),
    ));
}

fn drive_lights(
    time: Res<Time>,
    mut clock: ResMut<TimeOfDayClock>,
    mut sun: Query<(&mut DirectionalLight, &mut Transform), (With<Sun>, Without<Fill>)>,
    mut fill: Query<&mut DirectionalLight, (With<Fill>, Without<Sun>)>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut direction: ResMut<SkySun>,
) {
    if !clock.paused {
        clock.fraction = advance(clock.fraction, f64::from(time.delta_secs()), clock.day_length_s);
    }
    let sky = evaluate(clock.fraction);
    let toward_sun = sky.direction.as_vec3();
    direction.0 = toward_sun;
    for (mut light, mut transform) in &mut sun {
        light.color = colour(sky.sun_linear());
        light.illuminance = sky.sun_intensity as f32;
        *transform = Transform::IDENTITY.looking_to(-toward_sun, Vec3::Y);
    }
    for mut light in &mut fill {
        light.illuminance = (FILL_INTENSITY * sky.fill_scale) as f32;
    }
    *ambient = ambient_light(sky.ambient_hex, sky.ambient_intensity);
}

pub struct LightPlugin {
    pub quality: Quality,
}

impl Plugin for LightPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.quality)
            .init_resource::<TimeOfDayClock>()
            .init_resource::<SkySun>()
            .insert_resource(DirectionalLightShadowMap { size: SHADOW_MAP_SIZE })
            .insert_resource(ambient_light(SCENE_AMBIENT_HEX, SCENE_AMBIENT_INTENSITY))
            .add_systems(Startup, spawn_lights)
            .add_systems(Update, drive_lights);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::offscreen::{FOV_Y, HEIGHT, WIDTH, add_target, capture, headless_app, render_lock, texel};
    use crate::testkit::{cases, close, parse};
    use bevy::camera::RenderTarget;
    use bevy::core_pipeline::tonemapping::Tonemapping;
    use bevy::render::render_resource::TextureFormat;
    use serde_json::Value;

    const EXACT: f64 = 1e-9;
    const LINEAR_TOLERANCE: f64 = 1e-12;
    const SHADOWED_SHARE_MAX: f64 = 0.6;
    const LIT_SHARE_MIN: f64 = 0.9;
    const CAMERA_HEIGHT: f32 = 14.0;
    const CASTER_HEIGHT: f32 = 6.0;
    const CONTROL_PIXEL: (u32, u32) = (20, 20);

    fn fixture() -> Value {
        parse(include_str!("../../testdata/parity/spoint-planet-atmosphere.json"))
    }

    fn number(value: &Value) -> f64 {
        value.as_f64().unwrap()
    }

    fn sample_matches(sample: &Value, context: &str) {
        let sky = evaluate(number(&sample["fraction"]));
        close(sky.elevation_deg, number(&sample["elevationDeg"]), EXACT, context);
        for axis in 0..3 {
            close(sky.direction[axis], number(&sample["dir"][axis]), EXACT, context);
        }
        assert_eq!(u64::from(sky.sun_hex), sample["sunColor"].as_u64().unwrap(), "{context}: sun colour");
        assert_eq!(u64::from(sky.ambient_hex), sample["ambientColor"].as_u64().unwrap(), "{context}: ambient colour");
        close(sky.sun_intensity, number(&sample["sunIntensity"]), EXACT, context);
        close(sky.ambient_intensity, number(&sample["ambientIntensity"]), EXACT, context);
        close(sky.fill_scale, number(&sample["fillScale"]), EXACT, context);
    }

    #[test]
    fn the_keyframes_are_the_references() {
        let f = fixture();
        let reference = cases(&f, "keyframes");
        assert_eq!(reference.len(), KEYFRAMES.len());
        for (frame, want) in KEYFRAMES.iter().zip(reference) {
            let context = format!("keyframe {}", frame.deg);
            close(frame.deg, number(&want["deg"]), 0.0, &context);
            close(frame.sun_intensity, number(&want["sunIntensity"]), LINEAR_TOLERANCE, &context);
            close(frame.ambient_intensity, number(&want["ambientIntensity"]), LINEAR_TOLERANCE, &context);
            close(frame.fill_scale, number(&want["fillScale"]), LINEAR_TOLERANCE, &context);
            for c in 0..3 {
                close(frame.sun[c], number(&want["sunLinear"][c]), LINEAR_TOLERANCE, &context);
                close(frame.ambient[c], number(&want["ambientLinear"][c]), LINEAR_TOLERANCE, &context);
            }
            assert_eq!(u64::from(linear_to_hex(frame.sun)), want["sunColor"].as_u64().unwrap(), "{context}: sun hex");
            assert_eq!(u64::from(linear_to_hex(frame.ambient)), want["ambientColor"].as_u64().unwrap(), "{context}: ambient hex");
        }
    }

    #[test]
    fn the_day_fraction_samples_are_the_references() {
        let f = fixture();
        let regular = f["timeOfDay"]["samples"]["regular"].as_array().unwrap();
        assert_eq!(regular.len(), 96);
        for (index, sample) in regular.iter().enumerate() {
            sample_matches(sample, &format!("regular sample {index}"));
        }
        let at_keyframes = f["timeOfDay"]["samples"]["atKeyframeElevations"].as_array().unwrap();
        assert_eq!(at_keyframes.len(), 5);
        for (index, sample) in at_keyframes.iter().enumerate() {
            sample_matches(sample, &format!("keyframe elevation sample {index}"));
        }
    }

    #[test]
    fn the_time_defaults_and_scene_lights_are_the_references() {
        let f = fixture();
        close(START_FRACTION, number(&f["timeOfDay"]["startFraction"]), 0.0, "start fraction");
        close(DAY_LENGTH_S, number(&f["timeOfDay"]["dayLengthSec"]), 0.0, "day length");
        close(TILT_DEG, number(&f["timeOfDay"]["tiltDeg"]), 0.0, "tilt");
        let lights = f["sceneSetup"]["lights"].as_array().unwrap();
        assert_eq!(u64::from(SCENE_AMBIENT_HEX), lights[0]["args"][0].as_u64().unwrap());
        close(SCENE_AMBIENT_INTENSITY, number(&lights[0]["args"][1]), 0.0, "ambient");
        assert_eq!(u64::from(FILL_HEX), lights[1]["args"][0].as_u64().unwrap());
        close(FILL_INTENSITY, number(&lights[1]["args"][1]), 0.0, "fill");
        let fill_position = &lights[1]["calls"][0][1];
        assert_eq!(FILL_FROM, Vec3::new(number(&fill_position[0]) as f32, number(&fill_position[1]) as f32, number(&fill_position[2]) as f32));
        close(f64::from(SCENE_SUN_INTENSITY), number(&lights[2]["args"][1]), 0.0, "sun");
        assert_eq!(SHADOW_MAP_SIZE as u64, lights[2]["calls"][1][1][0].as_u64().unwrap());
        assert_eq!(lights[2]["props"]["castShadow"], Value::Bool(true));
    }

    #[test]
    fn quality_presets_carry_the_reference_exposure_and_shadow_flag() {
        let f = fixture();
        for (name, quality) in [("Low", Quality::Low), ("Medium", Quality::Medium), ("High", Quality::High), ("Ultra", Quality::Ultra)] {
            close(f64::from(quality.exposure()), number(&f["qualityPresets"][name]["toneMappingExposure"]), 1e-6, name);
            assert_eq!(quality.shadows_enabled(), f["qualityPresets"][name]["shadowsEnabled"].as_bool().unwrap(), "{name}");
        }
        assert_eq!(Quality::default(), Quality::Ultra);
    }

    #[test]
    fn the_cascade_config_is_the_reference_extents_with_one_overlap_proportion() {
        let ultra = cascade_config(Quality::Ultra.cascade_count());
        assert_eq!(ultra.bounds.len(), 3);
        for (got, want) in ultra.bounds.iter().zip([60.0, 192.0, 614.4]) {
            close(f64::from(*got), want, 1e-3, "cascade far bound");
        }
        assert_eq!(cascade_config(Quality::High.cascade_count()).bounds.len(), 2);
        assert_eq!(cascade_config(Quality::Medium.cascade_count()).bounds.len(), 1);
        let blend_widths: Vec<f64> = ultra.bounds.iter().map(|far| f64::from(*far * ultra.overlap_proportion)).collect();
        close(blend_widths[0], 4.0, 1e-4, "first boundary blend is the reference 4 m");
        close(blend_widths[1], 12.8, 1e-3, "second boundary blend widens with the single proportion");
        close(blend_widths[2], 40.96, 1e-2, "third boundary blend widens with the single proportion");
    }

    #[test]
    fn the_clock_wraps_and_runs_at_the_day_length() {
        close(advance(0.3, 60.0, DAY_LENGTH_S), 0.4, 1e-12, "advance");
        close(advance(0.95, 60.0, DAY_LENGTH_S), 0.05, 1e-12, "wrap");
        close(advance(0.3, DAY_LENGTH_S, DAY_LENGTH_S), 0.3, 1e-12, "full day");
    }

    #[test]
    fn the_light_system_follows_the_day_fraction_samples() {
        let f = fixture();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(LightPlugin { quality: Quality::Ultra });
        app.world_mut().resource_mut::<TimeOfDayClock>().paused = true;
        let samples = f["timeOfDay"]["samples"]["regular"].as_array().unwrap();
        for (index, sample) in samples.iter().enumerate() {
            app.world_mut().resource_mut::<TimeOfDayClock>().fraction = number(&sample["fraction"]);
            app.update();
            let toward_sun = Vec3::new(number(&sample["dir"][0]) as f32, number(&sample["dir"][1]) as f32, number(&sample["dir"][2]) as f32);
            assert!((app.world().resource::<SkySun>().0 - toward_sun).length() < 1e-6, "sample {index} sun direction");
            let (light, transform) = app.world_mut().query_filtered::<(&DirectionalLight, &Transform), With<Sun>>().single(app.world()).unwrap();
            assert!((*transform.forward() + toward_sun).length() < 1e-5, "sample {index} light travels away from the sun");
            close(f64::from(light.illuminance), number(&sample["sunIntensity"]), 1e-6, "sun illuminance");
            let brightness = app.world().resource::<GlobalAmbientLight>().brightness;
            close(f64::from(brightness), number(&sample["ambientIntensity"]) / PI, 1e-6, "ambient brightness");
            let (fill, _) = app.world_mut().query_filtered::<(&DirectionalLight, &Transform), With<Fill>>().single(app.world()).unwrap();
            close(f64::from(fill.illuminance), FILL_INTENSITY * number(&sample["fillScale"]), 1e-6, "fill illuminance");
            assert!(!fill.shadow_maps_enabled);
        }
    }

    struct Rig {
        app: App,
        target: Handle<Image>,
    }

    fn rig(quality: Quality) -> Rig {
        let mut app = headless_app();
        app.add_plugins(LightPlugin { quality });
        app.finish();
        app.cleanup();
        app.update();
        let target = add_target(&mut app, TextureFormat::Rgba8UnormSrgb);
        let plane = app.world_mut().resource_mut::<Assets<Mesh>>().add(Cuboid::new(60.0, 0.2, 60.0));
        let ball = app.world_mut().resource_mut::<Assets<Mesh>>().add(Mesh::from(Sphere::new(1.0)));
        let grey = app.world_mut().resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial { base_color: Color::srgb(0.7, 0.7, 0.7), perceptual_roughness: 1.0, reflectance: 0.0, ..default() });
        app.world_mut().spawn((Mesh3d(plane), MeshMaterial3d(grey.clone()), Transform::from_xyz(0.0, -0.1, 0.0)));
        app.world_mut().spawn((Mesh3d(ball), MeshMaterial3d(grey), Transform::from_xyz(0.0, CASTER_HEIGHT, 0.0)));
        app.world_mut().spawn((
            Camera3d::default(),
            RenderTarget::from(target.clone()),
            Projection::Perspective(PerspectiveProjection { fov: FOV_Y as f32, ..default() }),
            Msaa::Off,
            Tonemapping::None,
            camera_exposure(),
            Transform::from_xyz(0.0, CAMERA_HEIGHT, 0.0).looking_to(Vec3::NEG_Y, Vec3::NEG_Z),
        ));
        Rig { app, target }
    }

    impl Rig {
        fn shot(&mut self, fraction: f64, label: &str) -> Image {
            *self.app.world_mut().resource_mut::<TimeOfDayClock>() = TimeOfDayClock { fraction, paused: true, ..default() };
            capture(&mut self.app, self.target.clone(), label)
        }
    }

    fn shadow_pixel(fraction: f64) -> (u32, u32) {
        let sky = evaluate(fraction);
        let on_plane = DVec3::new(0.0, f64::from(CASTER_HEIGHT), 0.0) - sky.direction * (f64::from(CASTER_HEIGHT) / sky.direction.y);
        let half_height = f64::from(CAMERA_HEIGHT) * (FOV_Y / 2.0).tan();
        let half_width = half_height * f64::from(WIDTH) / f64::from(HEIGHT);
        let x = (on_plane.x / half_width + 1.0) / 2.0 * f64::from(WIDTH);
        let y = (on_plane.z / half_height + 1.0) / 2.0 * f64::from(HEIGHT);
        (x as u32, y as u32)
    }

    fn linear_luma(image: &Image, at: (u32, u32)) -> f64 {
        let t = texel(image, at.0, at.1);
        let linear = LinearRgba::from(Color::srgb_u8(t[0], t[1], t[2]));
        f64::from(0.2126 * linear.red + 0.7152 * linear.green + 0.0722 * linear.blue)
    }

    #[test]
    fn shadows_appear_where_the_sun_says_and_move_with_the_day_fraction() {
        let _guard = render_lock();
        let (noon, afternoon) = (0.5, 0.62);
        let (noon_at, afternoon_at) = (shadow_pixel(noon), shadow_pixel(afternoon));
        assert!(noon_at.0.abs_diff(afternoon_at.0) + noon_at.1.abs_diff(afternoon_at.1) > 40, "{noon_at:?} {afternoon_at:?}");
        let mut on = rig(Quality::Ultra);
        let noon_on = on.shot(noon, "shadow noon on");
        let afternoon_on = on.shot(afternoon, "shadow afternoon on");
        let mut off = rig(Quality::Low);
        let noon_off = off.shot(noon, "shadow noon off");
        let share = |image: &Image, at| linear_luma(image, at) / linear_luma(image, CONTROL_PIXEL);
        println!("shadow shares: noon on {} afternoon on {} noon off {} moved-away on {} moved-away off {}", share(&noon_on, noon_at), share(&afternoon_on, afternoon_at), share(&noon_off, noon_at), share(&afternoon_on, noon_at), share(&noon_off, afternoon_at));
        assert!(share(&noon_on, noon_at) < SHADOWED_SHARE_MAX, "noon shadow missing: {}", share(&noon_on, noon_at));
        assert!(share(&afternoon_on, afternoon_at) < SHADOWED_SHARE_MAX, "afternoon shadow missing: {}", share(&afternoon_on, afternoon_at));
        assert!(share(&afternoon_on, noon_at) > LIT_SHARE_MIN, "the noon shadow stayed after the sun moved: {}", share(&afternoon_on, noon_at));
        assert!(share(&noon_off, noon_at) > LIT_SHARE_MIN, "shadows off still darkens the noon spot: {}", share(&noon_off, noon_at));
    }
}
