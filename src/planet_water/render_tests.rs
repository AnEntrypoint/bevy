use super::surface::{WaterFrame, WaterMaterial, WaterMaterialHandle, WaterParams, WaterPipelinePlugin, WaterPlugin, WaterSurface, build_water, framed, water_mesh_asset};
use super::waves::*;
use crate::height::{HashVersion, Planet, PlanetConfig};
use crate::offscreen::{FOV_Y, Frame, HEIGHT, SKY_COLOUR, WIDTH, add_target, capture, headless_app, image_stats, mean_change, render_lock, spawn_view, texel, write_png};
use crate::planet_lod::guard::{builds_on_main_thread, mark_main_thread};
use crate::planet_lod::origin::FloatingOrigin;
use crate::planet_lod::quadtree::{QuadtreeConfig, select_leaves};
use crate::testkit::{close, parse};
use bevy::camera::visibility::NoFrustumCulling;
use bevy::math::DVec3;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::tasks::{AsyncComputeTaskPool, block_on};
use std::time::Duration;
use std::sync::{Arc, OnceLock};

const SEED: i32 = 0x5eed1234;
const HPF_RES: usize = 128;
const RADIUS: f64 = 6_360_000.0;
const RELIEF: f64 = 0.1;
const SEARCH_POINTS: usize = 40_000;
const SHORE_STEP_RAD: f64 = 0.002;
const SHORE_STEPS: usize = 3000;
const SHORE_BISECTIONS: usize = 40;
const WET_M: f64 = -20.0;
const CENSUS_STEP_PX_M: f32 = 1.9;
const CENSUS_RANGE: f64 = 8.0;
const CENSUS_SAMPLE_STRIDE: u32 = 3;
const CENSUS_MODES: u32 = 16;
const WAVE_MODES: u32 = 10;
const BASIS_TOLERANCE: f64 = 1e-4;
const CENSUS_TIMES: [f32; 2] = [0.0, 3.7];
const COVERED_MIN: f64 = 0.3;
const DISCARD_SHARE_MIN: f64 = 0.03;
const FLOODED_LAND_M: f32 = -10.0;
const LUMA_STD_MIN: f64 = 0.01;
const DISTINCT_COLOURS_MIN: usize = 32;
const CHANGE_MIN: f64 = 0.002;
const ORBIT_ALTITUDE_M: f64 = 12_000_000.0;
const LOW_ALTITUDE_M: f64 = 40.0;
const SHORE_ALTITUDE_M: f64 = 150.0;

fn planet_arc() -> Arc<Planet> {
    static PLANET: OnceLock<Arc<Planet>> = OnceLock::new();
    PLANET
        .get_or_init(|| {
            let config = PlanetConfig { seed: SEED, hpf_res: HPF_RES, hash: HashVersion::Integer, relief_scale: Some(RELIEF), anchor_dir: [0.0, 1.0, 0.0], radius: RADIUS, carves: Vec::new() };
            Arc::new(Planet::new(&config))
        })
        .clone()
}

fn planet() -> &'static Planet {
    static HELD: OnceLock<Arc<Planet>> = OnceLock::new();
    HELD.get_or_init(planet_arc)
}

fn height(dir: DVec3) -> f64 {
    planet().height(dir.normalize().to_array(), RADIUS)
}

fn fibonacci(index: usize) -> DVec3 {
    let golden = std::f64::consts::PI * (3.0 - 5f64.sqrt());
    let y = 1.0 - 2.0 * (index as f64 + 0.5) / SEARCH_POINTS as f64;
    let ring = (1.0 - y * y).sqrt();
    DVec3::new(ring * (golden * index as f64).cos(), y, ring * (golden * index as f64).sin())
}

fn deep_ocean() -> DVec3 {
    (0..SEARCH_POINTS).map(fibonacci).min_by(|a, b| height(*a).total_cmp(&height(*b))).unwrap()
}

fn north_of(dir: DVec3) -> DVec3 {
    dir.cross(DVec3::Y.cross(dir).normalize())
}

fn shoreline() -> (DVec3, DVec3) {
    let wet = deep_ocean();
    let east = DVec3::Y.cross(wet).normalize();
    let north = north_of(wet);
    for heading in 0..16 {
        let angle = f64::from(heading) * std::f64::consts::TAU / 16.0;
        let course = east * angle.cos() + north * angle.sin();
        let along = |step: f64| (wet * step.cos() + course * step.sin()).normalize();
        let Some(land) = (1..SHORE_STEPS).map(|n| n as f64 * SHORE_STEP_RAD).find(|&step| height(along(step)) > 0.0) else { continue };
        let (mut low, mut high) = (land - SHORE_STEP_RAD, land);
        for _ in 0..SHORE_BISECTIONS {
            let middle = (low + high) / 2.0;
            if height(along(middle)) > 0.0 { high = middle } else { low = middle }
        }
        let shore = along(low);
        return (shore, (course * low.cos() - wet * low.sin()).normalize());
    }
    panic!("no coastline found from the deepest ocean");
}

fn sea_mean() -> f32 {
    static MEAN: OnceLock<f64> = OnceLock::new();
    *MEAN.get_or_init(|| sea_height_mean(DEFAULT_CHOPPINESS, DEFAULT_AMPLITUDE)) as f32
}

#[derive(Clone, Copy, Debug)]
struct Pose {
    name: &'static str,
    site: fn() -> (DVec3, Option<DVec3>),
    altitude_m: f64,
    pitch_down_deg: f64,
}

fn open_sea() -> (DVec3, Option<DVec3>) {
    (deep_ocean(), None)
}

fn shoreline_towards_land() -> (DVec3, Option<DVec3>) {
    let (shore, heading) = shoreline();
    (shore, Some(heading))
}

const ORBIT: Pose = Pose { name: "orbit", site: open_sea, altitude_m: ORBIT_ALTITUDE_M, pitch_down_deg: 90.0 };
const LOW: Pose = Pose { name: "low", site: open_sea, altitude_m: LOW_ALTITUDE_M, pitch_down_deg: 35.0 };
const NADIR: Pose = Pose { name: "nadir", site: open_sea, altitude_m: LOW_ALTITUDE_M, pitch_down_deg: 90.0 };
const SHORE: Pose = Pose { name: "shoreline", site: shoreline_towards_land, altitude_m: SHORE_ALTITUDE_M, pitch_down_deg: 10.0 };

impl Pose {
    fn frame(&self) -> Frame {
        let (dir, heading) = (self.site)();
        let base = Frame::above(dir, RADIUS, self.altitude_m, self.pitch_down_deg);
        let Some(heading) = heading else { return base };
        let heading = (heading - dir * heading.dot(dir)).normalize();
        let pitch = self.pitch_down_deg.to_radians();
        Frame { forward: heading * pitch.cos() - dir * pitch.sin(), up: heading * pitch.sin() + dir * pitch.cos(), ..base }
    }

    fn leaves(&self, frame: &Frame) -> Vec<crate::planet_lod::quadtree::NodeKey> {
        let config = QuadtreeConfig::reference(RADIUS, f64::from(HEIGHT), FOV_Y);
        let ground = height(frame.position);
        select_leaves(&config.at(frame.position, ground), frame.position, frame.position.length() - RADIUS)
    }
}

#[derive(Clone, Copy)]
struct Shot {
    census: Option<u32>,
    time: f32,
    flooded: bool,
}

fn beauty(time: f32) -> Shot {
    Shot { census: None, time, flooded: false }
}

fn render(pose: &Pose, shot: Shot) -> Image {
    let _serial = render_lock();
    let frame = pose.frame();
    let census = shot.census.is_some();
    let (format, clear) = if census { (TextureFormat::Rgba8Unorm, Color::srgba(0.0, 0.0, 0.0, 0.0)) } else { (TextureFormat::Rgba8UnormSrgb, SKY_COLOUR) };
    let mut app = headless_app();
    app.add_plugins(WaterPipelinePlugin);
    let origin = FloatingOrigin { position: frame.position };
    app.insert_resource(origin);
    app.finish();
    app.cleanup();
    app.update();
    let handle = app.world().resource::<WaterMaterialHandle>().0.clone();
    let mut mesh = build_water(planet(), RADIUS, &pose.leaves(&frame), frame.position);
    if shot.flooded {
        mesh.land.iter_mut().for_each(|land| land[0] = FLOODED_LAND_M);
    }
    let sun_forward = (-frame.sun_to).as_vec3();
    let mut params = framed(WaterParams::default(), &WaterFrame { centre: DVec3::ZERO, radius: RADIUS }, &origin, Vec3::ZERO, sun_forward, mesh.origin, Vec2::new(WIDTH as f32, HEIGHT as f32));
    params.time = shot.time;
    params.sea_mean = sea_mean();
    params.census_mode = shot.census.unwrap_or(0) as f32;
    {
        let mut materials = app.world_mut().resource_mut::<Assets<WaterMaterial>>();
        let mut material = materials.get_mut(&handle).unwrap();
        material.params = params;
        material.key.census = census;
    }
    let target = add_target(&mut app, format);
    let mesh_handle = app.world_mut().resource_mut::<Assets<Mesh>>().add(water_mesh_asset(mesh));
    app.world_mut().spawn((Mesh3d(mesh_handle), MeshMaterial3d(handle), Transform::IDENTITY, NoFrustumCulling));
    spawn_view(&mut app, &target, clear, &frame, 1.0);
    capture(&mut app, target, &format!("{} {:?}", pose.name, shot.census))
}

fn assert_water_picture(pose: &Pose, image: &Image) {
    write_png(image, &format!("water-{}", pose.name));
    let stats = image_stats(image);
    assert!(stats.covered >= COVERED_MIN, "{}: water covers {}", pose.name, stats.covered);
    assert!(stats.luma_std >= LUMA_STD_MIN, "{}: luma std {}", pose.name, stats.luma_std);
    assert!(stats.distinct_colours >= DISTINCT_COLOURS_MIN, "{}: {} colours", pose.name, stats.distinct_colours);
}

#[test]
fn water_compiles_and_draws_at_orbit_and_low() {
    for pose in [ORBIT, LOW] {
        assert_water_picture(&pose, &render(&pose, beauty(0.0)));
    }
}

#[test]
fn the_shoreline_shows_water_and_discarded_land() {
    let shore = render(&SHORE, beauty(0.0));
    let flooded = render(&SHORE, Shot { flooded: true, ..beauty(0.0) });
    write_png(&shore, "water-shoreline");
    write_png(&flooded, "water-shoreline-flooded");
    let (dry, wet) = (image_stats(&shore).covered, image_stats(&flooded).covered);
    assert!(wet - dry > DISCARD_SHARE_MIN, "land discard removes {} of the picture", wet - dry);
}

#[test]
fn the_water_changes_between_two_times() {
    let (before, after) = (render(&LOW, beauty(0.0)), render(&LOW, beauty(2.5)));
    let change = mean_change(&before, &after);
    assert!(change > CHANGE_MIN, "mean change {change}");
}

fn decode(texel: [u8; 4]) -> Option<f64> {
    let fixed = f64::from(u32::from(texel[0]) * 65536 + u32::from(texel[1]) * 256 + u32::from(texel[2]));
    (texel[3] == 255).then(|| fixed / 16_777_215.0 * 2.0 * CENSUS_RANGE - CENSUS_RANGE)
}

fn expected(mode: u32, p: Point, t: f64, amp: f64, choppy: f64, up: DVec3) -> f64 {
    let slope = || sea_slope(p, t, 0..SEA_OCTAVES, amp, choppy);
    let basis = tangent_basis(up.to_array());
    match mode {
        0 => sea_height(p, t, 0..SEA_OCTAVES, amp, choppy),
        1 => sea_height(p, t, 2..6, amp, choppy),
        2 => sea_height_lf(p, t, amp, choppy),
        3 => slope()[0],
        4 => slope()[1],
        5 => sea_slope_lf(p, t, amp, choppy)[0],
        6 => sea_slope_lf(p, t, amp, choppy)[1],
        7..=9 => tangent_normal(slope())[mode as usize - 7],
        10..=12 => basis[0][mode as usize - 10],
        _ => basis[1][mode as usize - 13],
    }
}

const WAVE_TOLERANCE: [f64; WAVE_MODES as usize] = [3e-4, 3e-4, 1e-4, 1.2e-3, 1.2e-3, 1e-4, 1e-4, 1.2e-3, 1.2e-3, 3e-4];

#[test]
fn shader_waves_equal_the_rust_waves_at_every_probe_pixel() {
    let ocean = &super::tests::fixture()["constants"]["tsl/water-tsl.js"]["DEFAULT_OCEAN"];
    let (amp, choppy) = (ocean["amplitude"].as_f64().unwrap(), ocean["choppiness"].as_f64().unwrap());
    let frame = NADIR.frame();
    for time in CENSUS_TIMES {
        for mode in 0..CENSUS_MODES {
            let image = render(&NADIR, Shot { census: Some(mode), time, flooded: false });
            let (mut worst, mut seen, mut spread) = (0.0f64, 0usize, (f64::MAX, f64::MIN));
            for y in (0..HEIGHT).step_by(CENSUS_SAMPLE_STRIDE as usize) {
                for x in (0..WIDTH).step_by(CENSUS_SAMPLE_STRIDE as usize) {
                    let Some(got) = decode(texel(&image, x, y)) else { continue };
                    let p = [f64::from((x as f32 + 0.5) * CENSUS_STEP_PX_M), f64::from((y as f32 + 0.5) * CENSUS_STEP_PX_M)];
                    let want = expected(mode, p, f64::from(time), amp, choppy, frame.position.normalize());
                    worst = worst.max((got - want).abs());
                    spread = (spread.0.min(want), spread.1.max(want));
                    seen += 1;
                }
            }
            assert!(seen > 1000, "mode {mode}: only {seen} water pixels");
            let tolerance = if mode < WAVE_MODES {
                assert!(spread.1 - spread.0 > 0.02, "mode {mode}: probe values do not vary: {spread:?}");
                WAVE_TOLERANCE[mode as usize]
            } else {
                BASIS_TOLERANCE
            };
            close(worst, 0.0, tolerance, &format!("mode {mode} time {time}"));
        }
    }
}

#[test]
fn the_water_mesh_is_built_off_the_main_thread() {
    mark_main_thread();
    let frame = LOW.frame();
    let leaves = LOW.leaves(&frame);
    let before = builds_on_main_thread();
    let pooled = block_on(AsyncComputeTaskPool::get_or_init(Default::default).spawn(async move { build_water(planet(), RADIUS, &leaves, frame.position).positions.len() }));
    assert!(pooled > 0);
    assert_eq!(builds_on_main_thread(), before);
    build_water(planet(), RADIUS, &[], frame.position);
    assert_eq!(builds_on_main_thread(), before + 1);
}

const STREAM_FRAME_LIMIT: usize = 600;
const STREAM_POLL: Duration = Duration::from_millis(20);

#[test]
fn the_plugin_streams_the_water_off_the_main_thread_and_draws_it() {
    let _serial = render_lock();
    let frame = LOW.frame();
    let mut app = headless_app();
    let config = QuadtreeConfig::reference(RADIUS, f64::from(HEIGHT), FOV_Y);
    app.add_plugins(WaterPlugin { planet: planet_arc(), config, centre: DVec3::ZERO });
    app.insert_resource(FloatingOrigin { position: frame.position });
    app.finish();
    app.cleanup();
    let target = add_target(&mut app, TextureFormat::Rgba8UnormSrgb);
    spawn_view(&mut app, &target, SKY_COLOUR, &frame, 1.0);
    let mut waters = app.world_mut().query::<&Mesh3d>();
    let mut frames = 0;
    while waters.iter(app.world()).count() == 0 && frames < STREAM_FRAME_LIMIT {
        crate::offscreen::pump(&mut app);
        std::thread::sleep(STREAM_POLL);
        frames += 1;
    }
    assert_eq!(waters.iter(app.world()).count(), 1, "the streamed water mesh after {frames} frames");
    assert!((app.world().resource::<WaterSurface>().origin - frame.position).length() < 1e-6);
    let image = capture(&mut app, target, "plugin");
    write_png(&image, "water-plugin");
    assert!(image_stats(&image).covered >= COVERED_MIN);
}
