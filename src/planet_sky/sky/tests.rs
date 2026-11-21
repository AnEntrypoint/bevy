use super::*;
use crate::offscreen::{FOV_Y, Frame, HEIGHT, SKY_COLOUR, WIDTH, add_target, capture, headless_app, image_stats, mean_change, render_lock, spawn_view, texel, write_png};
use crate::planet_sky::atmosphere::{ATMOSPHERE, TransmittanceLut, u_from_r, v_from_mu};
use crate::planet_sky::lut::{baked, baked_through_half};
use crate::planet_sky::tonemap::display_referred_to_scene_linear;
use bevy::camera::{CameraProjection, Hdr};
use bevy::math::DVec3;

const RADIUS_M: f64 = 6_360_000.0;
const MEAN_TOLERANCE: f64 = 0.8;
const P99_TOLERANCE: f64 = 5.0;
const COVERED_MIN: f64 = 0.999;
const LUMA_STD_MIN: f64 = 0.02;
const DISTINCT_COLOURS_MIN: usize = 32;
const OFF_CHANGE_MIN: f64 = 0.2;
const TRANSMITTANCE_TOLERANCE: f64 = 1.5e-3;
const SCATTERING_RELATIVE_TOLERANCE: f64 = 5.0e-3;
const SCATTERING_ABSOLUTE_TOLERANCE: f64 = 1.0e-3;
const AERIAL_RELATIVE_TOLERANCE: f64 = 1.0e-2;
const AERIAL_ABSOLUTE_TOLERANCE: f64 = 1.0e-4;
const AERIAL_DISTANCE_KM: f64 = 20.0;
const RADIUS_TOLERANCE_RELATIVE: f64 = 1.0e-2;
const RADIUS_TOLERANCE_ABSOLUTE: f64 = 1.0e-3;
const RADIANCE_OUTSIDE_SHARE_MAX: f64 = 0.01;
const SUN_EDGE_PIXELS_MAX: usize = 2;
const SUN_NEIGHBOURHOOD_COS: f64 = 0.9995;
const DISPLAY_PROBE_TOLERANCE: f64 = 3.0e-3;
const DISPLAY_PROBE_SCALE: f64 = 3.0;
const PROBE_COLUMNS: u32 = 40;
const PROBE_MU_S_COUNT: u32 = 8;
const PROBE_MU_S_RANGE: [f64; 2] = [-0.9, 0.9];
const CENTRE_WINDOW: u32 = 10;
const BLOCKER_SIZE_M: f32 = 4.0;
const BLOCKER_DISTANCE_M: f32 = 10.0;
const BLOCKER_RED: [u8; 4] = [255, 0, 0, 255];

const RAYLEIGH_PHASE_K: f64 = 3.0 / (16.0 * std::f64::consts::PI);
const MIE_G: f64 = 0.8;
const MIE_SCATTERING: f64 = 0.003996;
const SOLAR_IRRADIANCE: DVec3 = DVec3::new(1.474, 1.8504, 1.91198);
const SUN_ANGULAR_RADIUS: f64 = 0.004675;
const SKY_TINT: DVec3 = DVec3::new(0.82, 0.95, 1.22);
const SKY_EXPOSURE_HIGH_SUN: f64 = 14.0;
const SKY_EXPOSURE_LOW_SUN: f64 = 48.0;
const SKY_SATURATION: f64 = 1.3;
const SKY_HALO_COLOUR: DVec3 = DVec3::new(0.32, 0.55, 1.0);
const SKY_HALO_GAIN: f64 = 0.03;
const SKY_DISC_GAIN: f64 = 6.0;
const SKY_MARCH_STEPS: usize = 8;
const HORIZON_BLEND_MU: f64 = 0.006;
const FADE_ALTITUDE_M: f64 = 100_000.0;
const ACES_DISPLAY_EXPOSURE: f64 = 8.0;
const LUMA: DVec3 = DVec3::new(0.2126, 0.7152, 0.0722);

fn pose(altitude_m: f64, pitch_down_deg: f64, sun_elevation_deg: f64, sun_azimuth_deg: f64) -> Frame {
    let dir = DVec3::Y;
    let frame = Frame::above(dir, RADIUS_M, altitude_m, pitch_down_deg);
    let east = DVec3::Y.cross(dir).try_normalize().unwrap_or(DVec3::X);
    let north = dir.cross(east);
    let (elevation, azimuth) = (sun_elevation_deg.to_radians(), sun_azimuth_deg.to_radians());
    let sun_to = (north * azimuth.cos() + east * azimuth.sin()) * elevation.cos() + dir * elevation.sin();
    Frame { sun_to, ..frame }
}

#[derive(Clone, Copy)]
struct Setup {
    config: SkyConfig,
    with_luts: bool,
    format: TextureFormat,
    hdr: bool,
    blocker: bool,
}

impl Default for Setup {
    fn default() -> Self {
        Self { config: SkyConfig::default(), with_luts: true, format: TextureFormat::Rgba8UnormSrgb, hdr: false, blocker: false }
    }
}

fn render_with(frame: &Frame, setup: Setup) -> Image {
    let Setup { config, with_luts, format, hdr, blocker } = setup;
    let _serial = render_lock();
    let mut app = headless_app();
    app.add_plugins(PlanetSkyPlugin);
    app.finish();
    app.cleanup();
    app.update();
    if with_luts {
        let (transmittance, scattering) = baked().images();
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        let luts = AtmosphereLuts { transmittance: images.add(transmittance), scattering: images.add(scattering) };
        app.insert_resource(luts);
    }
    app.insert_resource(config);
    app.insert_resource(SkySun(frame.sun_to.as_vec3()));
    app.insert_resource(SkyPlanet { centre: (-frame.position).as_vec3(), radius: RADIUS_M as f32 });
    if blocker {
        let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(Cuboid::from_length(BLOCKER_SIZE_M));
        let material = app.world_mut().resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial { base_color: Color::srgb(1.0, 0.0, 0.0), unlit: true, ..default() });
        app.world_mut().spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform::from_translation(frame.forward.as_vec3() * BLOCKER_DISTANCE_M)));
    }
    let target = add_target(&mut app, format);
    spawn_view(&mut app, &target, SKY_COLOUR, frame, 1.0);
    let mut cameras = app.world_mut().query_filtered::<Entity, With<Camera3d>>();
    let ids: Vec<Entity> = cameras.iter(app.world()).collect();
    for id in ids {
        app.world_mut().entity_mut(id).insert(SkyCamera);
        if hdr {
            app.world_mut().entity_mut(id).insert(Hdr);
        }
    }
    capture(&mut app, target, &format!("sky at {}", frame.position))
}

fn render(frame: &Frame) -> Image {
    render_with(frame, Setup::default())
}

fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn exp3(v: DVec3) -> DVec3 {
    DVec3::new(v.x.exp(), v.y.exp(), v.z.exp())
}

fn dist_to_top(r: f64, mu: f64) -> f64 {
    let disc = r * r * (mu * mu - 1.0) + ATMOSPHERE.top * ATMOSPHERE.top;
    if disc < 0.0 { -1.0 } else { (-r * mu + disc.sqrt()).max(0.0) }
}

fn dist_to_ground_continuous(r: f64, mu: f64) -> f64 {
    -r * mu - (r * r * (mu * mu - 1.0) + ATMOSPHERE.bottom * ATMOSPHERE.bottom).max(0.0).sqrt()
}

fn bilinear(width: usize, height: usize, fx: f64, fy: f64, fetch: impl Fn(usize, usize) -> [f64; 4]) -> [f64; 4] {
    let (x_floor, y_floor) = (fx.floor(), fy.floor());
    let clamp = |v: f64, count: usize| v.clamp(0.0, (count - 1) as f64) as usize;
    let (x0, x1, y0, y1) = (clamp(x_floor, width), clamp(x_floor + 1.0, width), clamp(y_floor, height), clamp(y_floor + 1.0, height));
    let (tx, ty) = (fx - x_floor, fy - y_floor);
    let (a, b, c, d) = (fetch(x0, y0), fetch(x1, y0), fetch(x0, y1), fetch(x1, y1));
    [0, 1, 2, 3].map(|i| (a[i] * (1.0 - tx) + b[i] * tx) * (1.0 - ty) + (c[i] * (1.0 - tx) + d[i] * tx) * ty)
}

fn transmittance_lookup(lut: &TransmittanceLut, r: f64, mu: f64) -> DVec3 {
    let (w, h) = (lut.width, lut.height);
    let fx = u_from_r(&ATMOSPHERE, r) * w as f64 - 0.5;
    let fy = v_from_mu(&ATMOSPHERE, mu, r) * h as f64 - 0.5;
    let t = bilinear(w, h, fx, fy, |x, y| {
        let at = (y * w + x) * 3;
        [f64::from(lut.data[at]), f64::from(lut.data[at + 1]), f64::from(lut.data[at + 2]), 0.0]
    });
    DVec3::new(t[0], t[1], t[2])
}

fn scattering_lookup(r: f64, mu: f64, mu_s: f64) -> [f64; 4] {
    let lut = &baked_through_half().scattering;
    let (w, h, layers) = (lut.width, lut.height, lut.layers);
    let warp = 1.4f64;
    let t = (mu_s.clamp(-0.999999, 0.999999) * warp.tanh()).atanh() / warp;
    let layer = (((t + 1.0) * 0.5).clamp(0.0, 1.0) * layers as f64 - 0.5).clamp(0.0, (layers - 1) as f64);
    let (l0, l1) = (layer.floor() as usize, (layer.floor() as usize + 1).min(layers - 1));
    let fx = u_from_r(&ATMOSPHERE, r) * w as f64 - 0.5;
    let fy = v_from_mu(&ATMOSPHERE, mu, r) * h as f64 - 0.5;
    let sample = |l: usize| bilinear(w, h, fx, fy, |x, y| {
        let at = ((l * h + y) * w + x) * 4;
        [0, 1, 2, 3].map(|i| f64::from(lut.data[at + i]))
    });
    let (s0, s1) = (sample(l0), sample(l1));
    [0, 1, 2, 3].map(|i| s0[i] + (s1[i] - s0[i]) * (layer - layer.floor()))
}

fn transmittance_to_sun(lut: &TransmittanceLut, p: DVec3, sun: DVec3) -> DVec3 {
    let r = p.length();
    let mu = p.dot(sun) / r;
    let mu_horizon = -(1.0 - ATMOSPHERE.bottom * ATMOSPHERE.bottom / (r * r)).max(0.0).sqrt();
    transmittance_lookup(lut, r, mu) * smoothstep(mu_horizon - 0.035, mu_horizon + 0.005, mu)
}

fn rayleigh_phase(nu: f64) -> f64 {
    RAYLEIGH_PHASE_K * (1.0 + nu * nu)
}

fn mie_phase(nu: f64) -> f64 {
    let g2 = MIE_G * MIE_G;
    let k = 3.0 / (8.0 * std::f64::consts::PI) * (1.0 - g2) / (2.0 + g2);
    k * (1.0 + nu * nu) / (1.0 + g2 - 2.0 * MIE_G * nu).powf(1.5)
}

fn sky_radiance(lut: &TransmittanceLut, frame: &Frame, x: u32, y: u32) -> DVec3 {
    let ray = frame.pixel_ray(x, y);
    let sun = frame.sun_to;
    let (bottom, top) = (ATMOSPHERE.bottom, ATMOSPHERE.top);
    let cam_in = frame.position * (bottom / RADIUS_M);
    let r_in = cam_in.length();
    let dt_in = dist_to_top(r_in, cam_in.dot(ray) / r_in);
    let camera = if r_in > top && dt_in > 0.0 { cam_in + ray * dt_in } else { cam_in };
    let r = camera.length();
    let mu = camera.dot(ray) / r;
    let mu_tangent = -(1.0 - bottom * bottom / (r * r)).max(0.0).sqrt();
    let w_sky = smoothstep(mu_tangent - HORIZON_BLEND_MU, mu_tangent + HORIZON_BLEND_MU, mu);
    let d_top = dist_to_top(r, mu);
    let d_end = dist_to_ground_continuous(r, mu).max(1e-3) * (1.0 - w_sky) + d_top * w_sky;
    let nu = ray.dot(sun);
    let dt = d_end / SKY_MARCH_STEPS as f64;
    let rayleigh = DVec3::from_array(ATMOSPHERE.rayleigh);
    let (mut od_r, mut od_m, mut inscat_r, mut inscat_m, mut t_view) = (0.0, 0.0, DVec3::ZERO, DVec3::ZERO, DVec3::ONE);
    for i in 0..SKY_MARCH_STEPS {
        let p = camera + ray * (dt * (i as f64 + 0.5));
        let altitude = p.length() - bottom;
        let (d_r, d_m) = ((-altitude / ATMOSPHERE.rayleigh_h).exp() * dt, (-altitude / ATMOSPHERE.mie_h).exp() * dt);
        od_r += d_r;
        od_m += d_m;
        t_view = exp3(-(rayleigh * od_r + DVec3::splat(od_m * ATMOSPHERE.mie_ext)));
        let t = t_view * transmittance_to_sun(lut, p, sun);
        inscat_r += t * d_r;
        inscat_m += t * d_m;
    }
    let radiance_base = SOLAR_IRRADIANCE * (inscat_r * rayleigh * rayleigh_phase(nu) + inscat_m * MIE_SCATTERING * mie_phase(nu));
    let sky_visible = d_top > 0.0 && (r_in <= top || dt_in >= 0.0);
    let (radiance_base, trans) = if sky_visible { (radiance_base, t_view * w_sky) } else { (DVec3::ZERO, DVec3::ONE) };
    let muc = cam_in.dot(ray) / r_in;
    let t0 = (r_in * (1.0 - muc * muc).max(0.0).sqrt() - bottom) / (top - bottom);
    let halo = if muc < 0.0 { smoothstep(0.0, 0.06, t0) * (1.0 - smoothstep(0.25, 1.6, t0)) } else { 0.0 };
    let limb_dir = (cam_in + ray * (-r_in * muc)).normalize();
    let lit = smoothstep(-0.5, 0.6, limb_dir.dot(sun)) * 0.75 + 0.25;
    let disc = if nu > SUN_ANGULAR_RADIUS.cos() { trans * SOLAR_IRRADIANCE * SKY_DISC_GAIN } else { DVec3::ZERO };
    radiance_base + SKY_HALO_COLOUR * (halo * lit * SKY_HALO_GAIN) + disc
}

fn sky_display(lut: &TransmittanceLut, frame: &Frame, x: u32, y: u32) -> DVec3 {
    let radiance = sky_radiance(lut, frame, x, y);
    let sun_elevation = frame.sun_to.dot((frame.position * (ATMOSPHERE.bottom / RADIUS_M)).normalize()).clamp(0.0, 1.0);
    let c = radiance * SKY_TINT * (SKY_EXPOSURE_LOW_SUN + (SKY_EXPOSURE_HIGH_SUN - SKY_EXPOSURE_LOW_SUN) * sun_elevation);
    let aces = (c * (c * 2.51 + DVec3::splat(0.03)) / (c * (c * 2.43 + DVec3::splat(0.59)) + DVec3::splat(0.14))).clamp(DVec3::ZERO, DVec3::ONE);
    let luma = DVec3::splat(aces.dot(LUMA));
    let mapped = (luma + (aces - luma) * SKY_SATURATION).clamp(DVec3::ZERO, DVec3::ONE);
    let fade = (1.0 - (frame.position.length() - RADIUS_M) / FADE_ALTITUDE_M).max(0.0);
    DVec3::new(mapped.x.powf(1.0 / 2.2), mapped.y.powf(1.0 / 2.2), mapped.z.powf(1.0 / 2.2)) * fade
}

fn target_bytes(display: DVec3, aces_exposure: Option<f64>) -> [f64; 3] {
    let linear = display_referred_to_scene_linear(display, aces_exposure.unwrap_or(1.0), aces_exposure.is_some()).clamp(DVec3::ZERO, DVec3::ONE);
    let encode = |l: f64| 255.0 * if l <= 0.0031308 { l * 12.92 } else { 1.055 * l.powf(1.0 / 2.4) - 0.055 };
    [encode(linear.x), encode(linear.y), encode(linear.z)]
}

struct Difference {
    mean: f64,
    p99: f64,
    max: f64,
}

fn twin_difference(image: &Image, frame: &Frame, aces_exposure: Option<f64>) -> Difference {
    let lut = &baked_through_half().transmittance;
    let mut differences = Vec::new();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let expected = target_bytes(sky_display(lut, frame, x, y), aces_exposure);
            let got = texel(image, x, y);
            differences.extend(expected.iter().enumerate().map(|(c, e)| (e - f64::from(got[c])).abs()));
        }
    }
    differences.sort_by(f64::total_cmp);
    Difference { mean: differences.iter().sum::<f64>() / differences.len() as f64, p99: differences[differences.len() * 99 / 100], max: *differences.last().unwrap() }
}

fn luma_at(image: &Image, x: u32, y: u32) -> f64 {
    let t = texel(image, x, y);
    (0.2126 * f64::from(t[0]) + 0.7152 * f64::from(t[1]) + 0.0722 * f64::from(t[2])) / 255.0
}

fn window(image: &Image) -> impl Iterator<Item = [u8; 4]> + '_ {
    let (cx, cy) = (WIDTH / 2, HEIGHT / 2);
    (cy - CENTRE_WINDOW..cy + CENTRE_WINDOW).flat_map(move |y| (cx - CENTRE_WINDOW..cx + CENTRE_WINDOW).map(move |x| texel(image, x, y)))
}

fn blue_over_red(image: &Image) -> f64 {
    let (blue, red) = window(image).fold((0.0, 0.0), |(b, r), t| (b + f64::from(t[2]), r + f64::from(t[0])));
    blue / red.max(1.0)
}

fn mean_luma(image: &Image) -> f64 {
    let sum: f64 = (0..HEIGHT).flat_map(|y| (0..WIDTH).map(move |x| (x, y))).map(|(x, y)| luma_at(image, x, y)).sum();
    sum / f64::from(WIDTH * HEIGHT)
}

struct StructureShots {
    up: Image,
    level: Image,
    toward_sun: Image,
    away_from_sun: Image,
    orbit: Image,
}

fn structure_shots(config: SkyConfig, with_luts: bool) -> StructureShots {
    let shot = |frame: Frame| render_with(&frame, Setup { config, with_luts, ..Setup::default() });
    StructureShots {
        up: shot(pose(2.0, -60.0, 20.0, 180.0)),
        level: shot(pose(2.0, 0.0, 20.0, 180.0)),
        toward_sun: shot(pose(2.0, -10.0, 12.0, 0.0)),
        away_from_sun: shot(pose(2.0, -10.0, 12.0, 180.0)),
        orbit: shot(orbit_frame()),
    }
}

fn orbit_frame() -> Frame {
    pose(ORBIT_ALTITUDE_M, ORBIT_PITCH_DOWN_DEG, 10.0, 30.0)
}

const ORBIT_ALTITUDE_M: f64 = 70_000.0;
const ORBIT_PITCH_DOWN_DEG: f64 = 8.9;

fn structure_failures(shots: &StructureShots) -> Vec<String> {
    let mut failures = Vec::new();
    let (up, level) = (blue_over_red(&shots.up), blue_over_red(&shots.level));
    if up <= level * ZENITH_BLUENESS_GAIN {
        failures.push(format!("zenith blue over red {up} is not above the horizon's {level}"));
    }
    let (toward, away) = (mean_luma(&shots.toward_sun), mean_luma(&shots.away_from_sun));
    if toward <= away * SUN_SIDE_GAIN {
        failures.push(format!("sun side luminance {toward} is not above the far side {away}"));
    }
    let profile: Vec<f64> = (0..HEIGHT).map(|y| (0..WIDTH).map(|x| luma_at(&shots.orbit, x, y)).sum::<f64>() / f64::from(WIDTH)).collect();
    let (peak_row, peak) = profile.iter().copied().enumerate().max_by(|a, b| a.1.total_cmp(&b.1)).unwrap();
    let horizon_row = HEIGHT as usize / 2;
    eprintln!("orbit profile peak {peak:.4} row {peak_row} top {:.4} bottom {:.4}", profile[0], profile[HEIGHT as usize - 1]);
    if peak_row.abs_diff(horizon_row) > HEIGHT as usize / LIMB_BAND_DIVISOR || peak < profile[0] * LIMB_RING_GAIN {
        failures.push(format!("no limb ring: peak {peak} at row {peak_row}, space row {}", profile[0]));
    }
    failures
}

const ZENITH_BLUENESS_GAIN: f64 = 1.0;
const SUN_SIDE_GAIN: f64 = 1.0;
const LIMB_BAND_DIVISOR: usize = 4;
const LIMB_RING_GAIN: f64 = 2.0;

#[test]
fn the_frame_uniform_follows_the_view_the_planet_the_sun_and_the_fade_altitude() {
    let transform = GlobalTransform::from(Transform::from_xyz(0.0, 6_410_000.0, 0.0).looking_to(Vec3::X, Vec3::Y));
    let planet = SkyPlanet { centre: Vec3::ZERO, radius: 6_360_000.0 };
    let frame = sky_frame(&transform, PerspectiveProjection { fov: FOV_Y as f32, aspect_ratio: 16.0 / 9.0, near: 1.0, ..default() }.get_clip_from_view(), &planet, &SkySun(Vec3::new(0.0, 3.0, 0.0)), &SkyConfig::default());
    assert!((frame.view_to_planet * Vec3::NEG_Z - Vec3::X).length() < 1e-6);
    assert!((frame.fade - 0.5).abs() < 1e-3, "fade {}", frame.fade);
    assert!((frame.cam_atm - Vec3::new(0.0, 6410.0, 0.0)).length() < 1e-2, "cam_atm {}", frame.cam_atm);
    assert_eq!(frame.sun_dir, Vec3::Y);
    assert!((frame.proj_diag.y - 1.0 / (FOV_Y as f32 / 2.0).tan()).abs() < 1e-5);
    assert!((frame.proj_diag.x - frame.proj_diag.y * 9.0 / 16.0).abs() < 1e-5);
    let far = sky_frame(&GlobalTransform::from_xyz(0.0, 6_500_000.0, 0.0), Mat4::IDENTITY, &planet, &SkySun::default(), &SkyConfig::default());
    assert_eq!(far.fade, 0.0);
    let moved = SkyPlanet { centre: Vec3::new(0.0, -1000.0, 0.0), radius: 6_360_000.0 };
    let shifted = sky_frame(&GlobalTransform::from_xyz(0.0, 6_359_000.0, 0.0), Mat4::IDENTITY, &moved, &SkySun::default(), &SkyConfig::default());
    assert!((shifted.cam_atm.y - 6360.0).abs() < 1e-2, "cam_atm {}", shifted.cam_atm);
}

#[test]
fn the_pipeline_compiles_and_the_sky_matches_the_cpu_twin_at_ground_high_and_orbit_poses() {
    let poses = [("ground", pose(2.0, 0.0, 12.0, 5.0)), ("high", pose(20_000.0, 4.5, 25.0, 20.0)), ("orbit", orbit_frame())];
    for (name, frame) in poses {
        let image = render(&frame);
        write_png(&image, &format!("planet-sky-{name}"));
        let measured = image_stats(&image);
        let difference = twin_difference(&image, &frame, None);
        eprintln!("{name} covered {:.4} spread {:.3} colours {} twin mean {:.3} p99 {:.3} max {:.3}", measured.covered, measured.luma_std, measured.distinct_colours, difference.mean, difference.p99, difference.max);
        assert!(measured.covered >= COVERED_MIN, "{name} covered {}", measured.covered);
        assert!(measured.luma_std >= LUMA_STD_MIN, "{name} luminance spread {}", measured.luma_std);
        assert!(measured.distinct_colours >= DISTINCT_COLOURS_MIN, "{name} distinct colours {}", measured.distinct_colours);
        assert!(difference.mean <= MEAN_TOLERANCE, "{name} mean difference {} (byte units)", difference.mean);
        assert!(difference.p99 <= P99_TOLERANCE, "{name} p99 difference {}", difference.p99);
    }
}

#[test]
fn the_sky_has_a_bluer_zenith_a_brighter_sun_side_and_a_limb_ring_at_orbit() {
    let shots = structure_shots(SkyConfig::default(), true);
    write_png(&shots.up, "planet-sky-up");
    write_png(&shots.toward_sun, "planet-sky-toward-sun");
    let failures = structure_failures(&shots);
    eprintln!("zenith {:.3} level {:.3} toward {:.3} away {:.3}", blue_over_red(&shots.up), blue_over_red(&shots.level), mean_luma(&shots.toward_sun), mean_luma(&shots.away_from_sun));
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn above_the_fade_altitude_the_sky_is_black_and_below_it_the_sky_is_lit() {
    let lit = mean_luma(&render(&pose(20_000.0, 4.5, 25.0, 20.0)));
    let gone = mean_luma(&render(&pose(FADE_ALTITUDE_M + 1_000.0, 10.0, 25.0, 20.0)));
    assert!(lit > 0.05 && gone < 0.002, "lit {lit} gone {gone}");
}

#[test]
fn without_the_atmosphere_or_its_tables_the_sky_vanishes_and_the_structure_checks_fail() {
    let reference = render(&pose(2.0, 0.0, 12.0, 5.0));
    let off = SkyConfig { enabled: false, ..SkyConfig::default() };
    let disabled = render_with(&pose(2.0, 0.0, 12.0, 5.0), Setup { config: off, ..Setup::default() });
    let unbound = render_with(&pose(2.0, 0.0, 12.0, 5.0), Setup { with_luts: false, ..Setup::default() });
    for (name, image) in [("disabled", &disabled), ("tables not bound", &unbound)] {
        assert!(image_stats(image).covered < 0.001, "{name} still drew the sky");
        let change = mean_change(&reference, image);
        assert!(change >= OFF_CHANGE_MIN, "{name} changed the image by only {change}");
    }
    let failures = structure_failures(&structure_shots(off, true));
    assert_eq!(failures.len(), 3, "{failures:?}");
}

fn probe_values(image: &Image, x: u32, y: u32) -> [f64; 4] {
    let data = image.data.as_ref().unwrap();
    let at = ((y * image.width() + x) * 16) as usize;
    [0, 1, 2, 3].map(|c| f64::from(f32::from_le_bytes(data[at + 4 * c..at + 4 * c + 4].try_into().unwrap())))
}

fn probe_parameters(x: u32, y: u32) -> (f64, f64, f64) {
    let column = f64::from(x % PROBE_COLUMNS) + 0.5;
    let r = ATMOSPHERE.bottom + (ATMOSPHERE.top - ATMOSPHERE.bottom) * column / f64::from(PROBE_COLUMNS);
    let mu = -1.0 + 2.0 * (f64::from(y) + 0.5) / f64::from(HEIGHT);
    let block = f64::from(x / PROBE_COLUMNS) / f64::from(PROBE_MU_S_COUNT - 1);
    (r, mu, PROBE_MU_S_RANGE[0] + (PROBE_MU_S_RANGE[1] - PROBE_MU_S_RANGE[0]) * block)
}

fn probe_image(view: SkyView) -> Image {
    let config = SkyConfig { view, ..SkyConfig::default() };
    render_with(&pose(2.0, 0.0, 12.0, 5.0), Setup { config, format: TextureFormat::Rgba32Float, hdr: true, ..Setup::default() })
}

#[test]
fn the_gpu_transmittance_lookup_equals_the_cpu_table_lookup() {
    assert_eq!((WIDTH, HEIGHT), (PROBE_COLUMNS * PROBE_MU_S_COUNT, 180));
    let image = probe_image(SkyView::ProbeTransmittance);
    let lut = &baked_through_half().transmittance;
    let mut worst: f64 = 0.0;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let (r, mu, _) = probe_parameters(x, y);
            let expected = transmittance_lookup(lut, r, mu);
            let got = probe_values(&image, x, y);
            worst = worst.max((0..3).map(|c| (got[c] - expected[c]).abs()).fold(0.0, f64::max));
        }
    }
    eprintln!("transmittance worst absolute difference {worst:.6}");
    assert!(worst <= TRANSMITTANCE_TOLERANCE, "worst {worst}");
}

#[test]
fn the_gpu_scattering_lookup_equals_the_cpu_table_lookup() {
    let image = probe_image(SkyView::ProbeScattering);
    let mut worst: f64 = 0.0;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let (r, mu, mu_s) = probe_parameters(x, y);
            let expected = scattering_lookup(r, mu, mu_s);
            let got = probe_values(&image, x, y);
            worst = worst.max((0..4).map(|c| (got[c] - expected[c]).abs() / (SCATTERING_RELATIVE_TOLERANCE * expected[c].abs() + SCATTERING_ABSOLUTE_TOLERANCE)).fold(0.0, f64::max));
        }
    }
    eprintln!("scattering worst difference as a multiple of the tolerance {worst:.4}");
    assert!(worst <= 1.0, "worst {worst} times the tolerance");
}

#[test]
fn the_sky_is_drawn_behind_geometry_and_never_over_it() {
    let frame = pose(2.0, 0.0, 12.0, 5.0);
    let image = render_with(&frame, Setup { blocker: true, ..Setup::default() });
    assert_eq!(texel(&image, WIDTH / 2, HEIGHT / 2), BLOCKER_RED, "the sky overdrew the geometry");
    assert_ne!(texel(&image, 2, 2), BLOCKER_RED);
    assert!(image_stats(&image).covered > 0.99, "the sky did not fill around the geometry");
}

fn aerial_cpu(r: f64, mu: f64, mu_s: f64) -> (DVec3, DVec3) {
    let lut = &baked_through_half().transmittance;
    let camera = DVec3::new(0.0, r, 0.0);
    let ray = DVec3::new((1.0 - mu * mu).max(0.0).sqrt(), mu, 0.0);
    let sun = DVec3::new((1.0 - mu_s * mu_s).max(0.0).sqrt(), mu_s, 0.0);
    let d_top = dist_to_top(r, mu);
    let full = scattering_lookup(r, mu, mu_s);
    let t_full = transmittance_lookup(lut, r, mu);
    let end = camera + ray * AERIAL_DISTANCE_KM;
    let r_end = end.length();
    let mu_end = end.dot(ray) / r_end;
    let tail = scattering_lookup(r_end, mu_end, end.dot(sun) / r_end);
    let t_end = transmittance_lookup(lut, r_end, mu_end);
    let t_to_end = t_full / t_end.max(DVec3::splat(1e-6));
    let (mut inscat_r, mut inscat_m, mut trans) = (DVec3::new(full[0], full[1], full[2]), full[3], t_full);
    if AERIAL_DISTANCE_KM < d_top - 1e-4 {
        inscat_r = (inscat_r - t_to_end * DVec3::new(tail[0], tail[1], tail[2])).max(DVec3::ZERO);
        inscat_m = (inscat_m - t_to_end.dot(DVec3::splat(1.0 / 3.0)) * tail[3]).max(0.0);
        trans = t_to_end;
    }
    if d_top <= 0.0 {
        return (DVec3::ZERO, DVec3::ONE);
    }
    let nu = ray.dot(sun);
    let rayleigh = DVec3::from_array(ATMOSPHERE.rayleigh);
    (SOLAR_IRRADIANCE * (inscat_r * rayleigh * rayleigh_phase(nu) + DVec3::splat(inscat_m * MIE_SCATTERING * mie_phase(nu))), trans)
}

#[test]
fn the_gpu_aerial_perspective_march_equals_the_cpu_twin() {
    let mut worst = [0.0f64; 2];
    for (index, view) in [SkyView::ProbeAerialRadiance, SkyView::ProbeAerialTransmittance].into_iter().enumerate() {
        let image = probe_image(view);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let (r, mu, mu_s) = probe_parameters(x, y);
                let (radiance, trans) = aerial_cpu(r, mu, mu_s);
                let expected = [radiance, trans][index];
                let got = probe_values(&image, x, y);
                let ratio = (0..3).map(|c| (got[c] - expected[c]).abs() / (AERIAL_RELATIVE_TOLERANCE * expected[c].abs() + AERIAL_ABSOLUTE_TOLERANCE)).fold(0.0, f64::max);
                worst[index] = worst[index].max(ratio);
            }
        }
    }
    eprintln!("aerial radiance and transmittance worst difference as a multiple of the tolerance {worst:?}");
    assert!(worst.iter().all(|w| *w <= 1.0), "{worst:?}");
}

#[test]
fn the_aces_display_mode_and_exposure_follow_the_tonemap_twin() {
    let frame = pose(2.0, 0.0, 12.0, 5.0);
    let config = SkyConfig { aces: true, display_exposure: ACES_DISPLAY_EXPOSURE as f32, ..SkyConfig::default() };
    let aces = render_with(&frame, Setup { config, ..Setup::default() });
    let plain = render(&frame);
    let difference = twin_difference(&aces, &frame, Some(ACES_DISPLAY_EXPOSURE));
    eprintln!("aces mode twin mean {:.3} p99 {:.3} max {:.3} against plain {:.3}", difference.mean, difference.p99, difference.max, mean_change(&plain, &aces));
    assert!(difference.mean <= MEAN_TOLERANCE && difference.p99 <= P99_TOLERANCE, "mean {} p99 {}", difference.mean, difference.p99);
    assert!(mean_change(&plain, &aces) >= OFF_CHANGE_MIN / 4.0, "the aces mode did not change the image");
}

#[test]
fn the_gpu_sky_radiance_equals_the_cpu_twin_before_the_display_curve_at_ground_high_and_orbit_poses() {
    let poses = [("ground", pose(2.0, 0.0, 12.0, 5.0)), ("high", pose(20_000.0, 4.5, 25.0, 20.0)), ("orbit", orbit_frame())];
    let lut = &baked_through_half().transmittance;
    for (name, frame) in poses {
        let config = SkyConfig { view: SkyView::ProbeRadiance, ..SkyConfig::default() };
        let image = render_with(&frame, Setup { config, format: TextureFormat::Rgba32Float, hdr: true, ..Setup::default() });
        let (mut outside, mut near_sun) = (0, 0);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let expected = sky_radiance(lut, &frame, x, y);
                let got = probe_values(&image, x, y);
                let off = (0..3).any(|c| (got[c] - [expected.x, expected.y, expected.z][c]).abs() > RADIUS_TOLERANCE_RELATIVE * [expected.x, expected.y, expected.z][c].abs() + RADIUS_TOLERANCE_ABSOLUTE);
                let by_the_sun = frame.pixel_ray(x, y).dot(frame.sun_to) > SUN_NEIGHBOURHOOD_COS;
                outside += usize::from(off && !by_the_sun);
                near_sun += usize::from(off && by_the_sun);
            }
        }
        eprintln!("{name}: {outside} pixels outside the radiance tolerance, {near_sun} of them by the sun");
        assert!(outside as f64 <= RADIANCE_OUTSIDE_SHARE_MAX * f64::from(WIDTH * HEIGHT), "{name}: {outside} pixels outside the radiance tolerance");
        assert!(near_sun <= SUN_EDGE_PIXELS_MAX, "{name}: {near_sun} pixels by the sun outside the radiance tolerance");
    }
}

#[test]
fn the_gpu_forward_display_curve_equals_the_tonemap_twin_on_a_colour_gradient() {
    let config = SkyConfig { view: SkyView::ProbeDisplay, aces: true, display_exposure: 1.0, ..SkyConfig::default() };
    let image = render_with(&pose(2.0, 0.0, 12.0, 5.0), Setup { config, format: TextureFormat::Rgba32Float, hdr: true, ..Setup::default() });
    let mut worst: f64 = 0.0;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let across = (f64::from(x) + 0.5) / f64::from(WIDTH);
            let scene = DVec3::new(across, (f64::from(y) + 0.5) / f64::from(HEIGHT), 1.0 - across) * DISPLAY_PROBE_SCALE;
            let expected = crate::planet_sky::tonemap::scene_linear_to_display_referred(scene, 1.0, true);
            let got = probe_values(&image, x, y);
            worst = worst.max((0..3).map(|c| (got[c] - [expected.x, expected.y, expected.z][c]).abs()).fold(0.0, f64::max));
        }
    }
    eprintln!("display curve worst absolute difference {worst:.5}");
    assert!(worst <= DISPLAY_PROBE_TOLERANCE, "worst {worst}");
}
