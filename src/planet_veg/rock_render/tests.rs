use super::*;
use crate::offscreen::{Frame, SKY_COLOUR, SKY_TEXEL, add_target, capture, headless_app, image_stats, mean_change, pump, render_lock, spawn_view, texel};
use crate::planet_lod::origin::FloatingOrigin;
use crate::planet_veg::gpu::VegGpuPlugin;
use crate::planet_veg::gpu::buffers::{EncodedRecords, RockRecord, VegKind};
use crate::planet_veg::gpu::cull::VegWitness;
use crate::planet_veg::gpu::draw::{VegCamera, VegEnvironment};
use crate::planet_veg::gpu::store::{ChunkRef, VegStore};
use bevy::math::DVec3;
use bevy::render::render_resource::TextureFormat;
use serde_json::Value;

const FIXTURE: &str = include_str!("../../../testdata/parity/spoint-rock-shapes.json");
const NEAR_Z: f32 = -12.0;
const PROBE_Z: f32 = -5.0;
const PROBE_SCALE: f32 = 0.5;
const PUMP_LIMIT: usize = 400;
const TIPPED_CHUNK: [f32; 3] = [0.7, 0.2, -0.3];

struct Rock {
    at: Vec3,
    scale: f32,
    squash: f32,
    variant: f32,
    rotation: Quat,
    kind: u32,
}

impl Rock {
    fn at(at: Vec3, kind: u32) -> Self {
        Self { at, scale: 2.0, squash: 1.0, variant: 0.5, rotation: Quat::IDENTITY, kind }
    }
}

fn near(kind: u32) -> Vec<Rock> {
    vec![Rock::at(Vec3::new(0.0, 0.0, NEAR_Z), kind)]
}

fn mid(kind: u32) -> Vec<Rock> {
    (0..10).map(|n| Rock { scale: 4.0, ..Rock::at(Vec3::new(-50.0 + 25.0 * (n % 5) as f32, 0.0, -150.0 - 50.0 * (n / 5) as f32), kind) }).collect()
}

struct Rig {
    app: App,
    target: Handle<Image>,
    origin: FloatingOrigin,
    chunks: Vec<ChunkRef>,
    witness: VegWitness,
}

fn rig(source: &str, with_plugin: bool) -> Rig {
    let mut app = headless_app();
    let origin = FloatingOrigin { position: DVec3::new(6_360_000.0, 345.0, -2_100.0) };
    app.add_plugins(VegGpuPlugin::default());
    if with_plugin {
        app.add_plugins(RockRenderPlugin);
    }
    app.world_mut().resource_mut::<Assets<Shader>>().insert(ROCK_SHADER.id(), Shader::from_wgsl(source.to_string(), ROCK_SHADER_PATH)).unwrap();
    app.insert_resource(origin);
    app.insert_resource(VegEnvironment { sun_direction: Vec3::new(0.3, 0.8, 0.5).normalize(), sun_irradiance: Vec3::splat(3.0), ..default() });
    app.finish();
    app.cleanup();
    let target = add_target(&mut app, TextureFormat::Rgba8UnormSrgb);
    let frame = Frame { position: DVec3::ZERO, forward: DVec3::NEG_Z, up: DVec3::Y, sun_to: DVec3::new(0.3, 0.9, 0.2).normalize() };
    spawn_view(&mut app, &target, SKY_COLOUR, &frame, 1.0);
    let camera = app.world_mut().query_filtered::<Entity, With<Camera3d>>().single(app.world()).unwrap();
    app.world_mut().entity_mut(camera).insert(VegCamera);
    let witness = app.world().resource::<VegWitness>().clone();
    let mut rig = Rig { app, target, origin, chunks: Vec::new(), witness };
    if with_plugin {
        rig.wait_for_draw_sets();
    }
    rig
}

impl Rig {
    fn wait_for_draw_sets(&mut self) {
        for _ in 0..PUMP_LIMIT {
            pump(&mut self.app);
            if self.app.world_mut().query::<&VegDrawSet>().iter(self.app.world()).count() == ROCK_TYPES {
                return;
            }
        }
        panic!("the rock meshes were never built");
    }

    fn drain_uploads(&mut self) {
        for _ in 0..PUMP_LIMIT {
            if self.app.world().resource::<VegStore>().pending_bytes(VegKind::Rock) == 0 {
                break;
            }
            pump(&mut self.app);
        }
        pump(&mut self.app);
        pump(&mut self.app);
        assert_eq!(self.app.world().resource::<VegStore>().pending_bytes(VegKind::Rock), 0, "uploads did not drain");
    }

    fn show(&mut self, rocks: &[Rock], label: &str) -> Image {
        let anchor = rocks[0].at;
        let mut store = self.app.world_mut().resource_mut::<VegStore>();
        self.chunks.drain(..).for_each(|chunk| assert!(store.remove_chunk(chunk)));
        let tipped = Quat::from_euler(EulerRot::YXZ, TIPPED_CHUNK[0], TIPPED_CHUNK[1], TIPPED_CHUNK[2]);
        let chunk = store.add_chunk(self.origin.position + anchor.as_dvec3(), tipped).unwrap();
        let records: Vec<RockRecord> = rocks
            .iter()
            .map(|rock| RockRecord {
                local_position: (tipped.inverse() * (rock.at - anchor)).to_array(),
                scale: rock.scale,
                rotation: (tipped.inverse() * rock.rotation).to_array(),
                squash: rock.squash,
                variant: rock.variant,
                kind: rock.kind,
            })
            .collect();
        store.upload(chunk, 10.0, EncodedRecords::new(&records)).unwrap();
        self.chunks.push(chunk);
        self.drain_uploads();
        capture(&mut self.app, self.target.clone(), label)
    }
}

fn extent(image: &Image) -> (f32, f32) {
    let (mut low, mut high) = ((u32::MAX, u32::MAX), (0, 0));
    for y in 0..image.height() {
        for x in 0..image.width() {
            if texel(image, x, y) != SKY_TEXEL {
                low = (low.0.min(x), low.1.min(y));
                high = (high.0.max(x), high.1.max(y));
            }
        }
    }
    ((high.0 - low.0 + 1) as f32, (high.1 - low.1 + 1) as f32)
}

fn replaced(from: &str, to: &str) -> String {
    assert_eq!(ROCK_SOURCE.matches(from).count(), 1, "{from} must occur once in the rock shader");
    ROCK_SOURCE.replace(from, to)
}

#[test]
fn rock_meshes_are_the_reference_shapes_with_outward_unit_normals_and_enclosing_bounds() {
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    let hull = &fixture["hull"];
    assert_eq!((hull["numShapes"].as_u64().unwrap() as usize, hull["baseSeed"].as_i64().unwrap() as i32, hull["hullRes"].as_u64().unwrap() as usize), (ROCK_TYPES, ROCK_BASE_SEED, ROCK_MESH_RES));
    let meshes = build_rock_meshes();
    assert_eq!(meshes.len(), ROCK_TYPES);
    for (kind, (mesh, shape)) in meshes.iter().zip(hull["shapes"].as_array().unwrap()).enumerate() {
        assert_eq!(mesh.positions.len() as u64, shape["vCount"].as_u64().unwrap(), "type {kind} vertex count");
        let wanted: Vec<u32> = shape["indices"].as_array().unwrap().iter().map(|i| i.as_u64().unwrap() as u32).collect();
        assert_eq!(mesh.indices, wanted, "type {kind} indices");
        let flat: Vec<f64> = shape["positions"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
        for (vertex, point) in mesh.positions.iter().enumerate() {
            for axis in 0..3 {
                assert!((f64::from(point[axis]) - flat[vertex * 3 + axis]).abs() < 1e-5, "type {kind} vertex {vertex}");
            }
        }
        let centre = Vec3::new(mesh.bounds[0], mesh.bounds[1], mesh.bounds[2]);
        assert!(mesh.bounds[3] > 0.5 && mesh.bounds[3] < 2.0, "type {kind} radius {}", mesh.bounds[3]);
        let used: std::collections::HashSet<u32> = mesh.indices.iter().copied().collect();
        let mut outward = 0;
        for (point, normal) in mesh.positions.iter().zip(&mesh.normals).enumerate().filter(|(vertex, _)| used.contains(&(*vertex as u32))).map(|(_, pair)| pair) {
            let (point, normal) = (Vec3::from(*point), Vec3::from(*normal));
            assert!((normal.length() - 1.0).abs() < 1e-4, "type {kind} normal length");
            assert!(point.distance(centre) <= mesh.bounds[3] + 1e-4, "type {kind} bound");
            outward += usize::from(normal.dot(point - centre) > 0.0);
        }
        assert!(outward * 10 >= used.len() * 9, "type {kind}: {outward} of {} normals point outward", used.len());
    }
}

#[test]
fn rock_types_all_render_at_near_and_mid_distance_and_each_type_changes_the_image() {
    let _serial = render_lock();
    let mut rig = rig(ROCK_SOURCE, true);
    let bounds = rig.app.world().resource::<VegBounds>().rock_kind.clone();
    assert_eq!(bounds.len(), ROCK_TYPES);
    let mut near_images = Vec::new();
    let mut mid_images = Vec::new();
    for kind in 0..ROCK_TYPES as u32 {
        let image = rig.show(&near(kind), &format!("rock {kind} near"));
        let stats = image_stats(&image);
        assert!(stats.covered > 0.012 && stats.distinct_colours > 12, "type {kind} near: covered {} colours {}", stats.covered, stats.distinct_colours);
        assert!(rig.witness.draws() > 0);
        near_images.push(image);
        let image = rig.show(&mid(kind), &format!("rock {kind} mid"));
        let stats = image_stats(&image);
        assert!(stats.covered > 0.0005 && stats.distinct_colours > 4, "type {kind} mid: covered {} colours {}", stats.covered, stats.distinct_colours);
        mid_images.push(image);
    }
    for (label, images, least) in [("near", &near_images, 0.08), ("mid", &mid_images, 0.08)] {
        for a in 0..ROCK_TYPES {
            for b in a + 1..ROCK_TYPES {
                let change = mean_change(&images[a], &images[b]);
                println!("rock {label} {a} vs {b}: {change:.4}");
                assert!(change > least, "{label}: types {a} and {b} look alike ({change})");
            }
        }
    }
}

#[test]
fn rock_squash_scales_the_height_and_the_rotation_turns_the_rock() {
    let _serial = render_lock();
    let mut rig = rig(ROCK_SOURCE, true);
    let upright = Rock { squash: 0.55, ..Rock::at(Vec3::new(0.0, 0.0, NEAR_Z), 2) };
    let full = rig.show(&[Rock { squash: 1.0, ..Rock::at(Vec3::new(0.0, 0.0, NEAR_Z), 2) }], "squash 1");
    let flat = rig.show(&[upright], "squash 0.55");
    let turned = rig.show(&[Rock { squash: 0.55, rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2), ..Rock::at(Vec3::new(0.0, 0.0, NEAR_Z), 2) }], "turned");
    let ((full_w, full_h), (flat_w, flat_h), (turned_w, turned_h)) = (extent(&full), extent(&flat), extent(&turned));
    println!("extent full {full_w}x{full_h} flat {flat_w}x{flat_h} turned {turned_w}x{turned_h}");
    assert!((0.50..0.62).contains(&(flat_h / full_h)), "squash 0.55 gives height ratio {}", flat_h / full_h);
    assert!((flat_w / full_w - 1.0).abs() < 0.04, "squash must not change the width: {}", flat_w / full_w);
    assert!((turned_w / flat_h - 1.0).abs() < 0.12 && (turned_h / flat_w - 1.0).abs() < 0.12, "a quarter turn about z swaps width and height");
}

fn hash(point: Vec3) -> f32 {
    let scaled = point * 0.3183099 + Vec3::splat(0.1);
    let p = (scaled - scaled.floor()) * 17.0;
    let product = p.x * p.y * p.z * (p.x + p.y + p.z);
    product - product.floor()
}

fn noise(x: Vec3) -> f32 {
    let (i, raw) = (x.floor(), x - x.floor());
    let f = raw * raw * (Vec3::splat(3.0) - 2.0 * raw);
    let corner = |dx: f32, dy: f32, dz: f32| hash(i + Vec3::new(dx, dy, dz));
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let row = |dy: f32, dz: f32| lerp(corner(0.0, dy, dz), corner(1.0, dy, dz), f.x);
    lerp(lerp(row(0.0, 0.0), row(1.0, 0.0), f.y), lerp(row(0.0, 1.0), row(1.0, 1.0), f.y), f.z)
}

fn reference_albedo(local: Vec3, variant: f32) -> Vec3 {
    let n = (noise(local * 1.7) * 0.6 + noise(local * 6.5) * 0.3 + noise(local * 23.0) * 0.1).clamp(0.0, 1.0);
    let ramp = Vec3::new(0.30, 0.28, 0.25).lerp(Vec3::new(0.66, 0.62, 0.56), n) * 2.05;
    ramp * (0.78 + variant * 0.4).min(1.0)
}

fn srgb_level(linear: f32) -> f32 {
    (if linear <= 0.0031308 { 12.92 * linear } else { 1.055 * linear.powf(1.0 / 2.4) - 0.055 }) * 255.0
}

const TILTED_NORMAL: [f32; 3] = [0.0, 0.6, 0.8];

fn plane_image(source: &str, fragment_entry: &'static str, rock: Rock, environment: Option<VegEnvironment>) -> Image {
    let mut rig = rig(source, false);
    if let Some(environment) = environment {
        rig.app.insert_resource(environment);
    }
    rig.app.world_mut().resource_mut::<VegBounds>().rock_kind = vec![[0.0, 0.0, 0.0, 12.0]; ROCK_TYPES];
    let plane = Mesh::from(Rectangle::new(16.0, 16.0)).with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![TILTED_NORMAL; 4]);
    let mesh = rig.app.world_mut().resource_mut::<Assets<Mesh>>().add(plane);
    rig.app.world_mut().spawn(VegDrawSet::rock(0, mesh, VegMaterial { fragment_entry, ..rock_material() }));
    rig.show(&[rock], fragment_entry)
}

fn probe_image(source: &str) -> Image {
    plane_image(source, "fs_rock_probe", Rock { scale: PROBE_SCALE, ..Rock::at(Vec3::new(0.0, 0.0, PROBE_Z), 0) }, None)
}

fn quantile(mut errors: Vec<f32>, q: f32) -> f32 {
    errors.sort_by(f32::total_cmp);
    errors[((errors.len() - 1) as f32 * q) as usize]
}

fn probe_errors(image: &Image) -> Vec<f32> {
    let frame = Frame { position: DVec3::ZERO, forward: DVec3::NEG_Z, up: DVec3::Y, sun_to: DVec3::Y };
    let mut errors = Vec::new();
    for y in (10..170).step_by(10) {
        for x in (10..310).step_by(10) {
            let ray = frame.pixel_ray(x, y).as_vec3();
            let hit = ray * (PROBE_Z / ray.z);
            let local = Vec3::new(hit.x, hit.y, 0.0) / PROBE_SCALE;
            let want = reference_albedo(local, 0.5).clamp(Vec3::ZERO, Vec3::ONE);
            let got = texel(image, x, y);
            errors.push((0..3).map(|c| (f32::from(got[c]) - srgb_level(want[c])).abs()).fold(0.0, f32::max));
        }
    }
    errors
}

#[test]
fn rock_albedo_probe_matches_the_reference_noise_ramp_and_variant_tint() {
    let _serial = render_lock();
    let image = probe_image(ROCK_SOURCE);
    assert!(image_stats(&image).covered > 0.99, "the probe plane fills the frame");
    let p90 = quantile(probe_errors(&image), 0.9);
    println!("probe: 90th percentile error {p90}");
    assert!(p90 <= PIXEL_TOLERANCE, "the probe differs from the reference albedo by {p90} levels at the 90th percentile");
}

#[test]
fn rock_red_checks_each_change_the_image_or_the_probe() {
    let _serial = render_lock();
    let baseline = rig(ROCK_SOURCE, true).show(&near(1), "red baseline");
    let ramp = replaced("vec3<f32>(0.66, 0.62, 0.56)", "vec3<f32>(0.30, 0.62, 0.90)");
    let variant = replaced("const VARIANT_TINT_SPAN: f32 = 0.4;", "const VARIANT_TINT_SPAN: f32 = 0.0;");
    let ambient = replaced("let ambient = albedo * light.ambient_mul + light.ambient_add;", "let ambient = vec3<f32>(0.0);");
    for (name, source) in [("tint ramp changed", ramp.clone()), ("variant tint ignored", variant), ("ambient dropped", ambient)] {
        let changed = rig(&source, true).show(&near(1), name);
        let change = mean_change(&baseline, &changed);
        println!("{name}: {change:.4}");
        assert!(change > 0.01, "{name} left the image unchanged ({change})");
    }
    let p90 = quantile(probe_errors(&probe_image(&ramp)), 0.9);
    assert!(p90 > 10.0 * PIXEL_TOLERANCE, "the changed ramp must break the probe, 90th percentile error {p90}");
    let squashed = Rock { squash: 0.55, ..Rock::at(Vec3::new(0.0, 0.0, NEAR_Z), 1) };
    let unsquashed = replaced("vec3<f32>(rock.scale, rock.scale * rock.squash, rock.scale)", "vec3<f32>(rock.scale, rock.scale, rock.scale)");
    let honoured = extent(&rig(ROCK_SOURCE, true).show(std::slice::from_ref(&squashed), "squash honoured")).1;
    let ignored = extent(&rig(&unsquashed, true).show(std::slice::from_ref(&squashed), "squash ignored")).1;
    assert!(ignored / honoured > 1.5, "ignoring squash must stretch the rock: {honoured} against {ignored}");
}

const LIT_SQUASH: f32 = 0.5;
const LIT_TURN: f32 = -0.7;
const PIXEL_TOLERANCE: f32 = 1.5;

fn lit_rock() -> Rock {
    Rock { scale: PROBE_SCALE, squash: LIT_SQUASH, rotation: Quat::from_rotation_z(LIT_TURN), ..Rock::at(Vec3::new(0.0, 0.0, PROBE_Z), 0) }
}

fn lit_environment() -> VegEnvironment {
    VegEnvironment { sun_direction: Vec3::new(0.3, 0.8, 0.5).normalize(), sun_irradiance: Vec3::splat(3.0), ..default() }
}

fn graded_environment() -> VegEnvironment {
    VegEnvironment { grade_mode: 1, day_shade: 0.9, night_fill: Vec3::splat(0.02), exposure: 0.5, look_saturation: 1.2, look_contrast: 1.1, ..lit_environment() }
}

fn reference_grade(lit: Vec3, environment: &VegEnvironment) -> Vec3 {
    if environment.grade_mode == 0 {
        return lit;
    }
    let exposed = (lit * environment.day_shade + environment.night_fill) * environment.exposure;
    let aces = (exposed * (exposed * 2.51 + 0.03) / (exposed * (exposed * 2.43 + 0.59) + 0.14)).clamp(Vec3::ZERO, Vec3::ONE);
    let saturated = Vec3::splat(aces.dot(Vec3::new(0.2126, 0.7152, 0.0722))).lerp(aces, environment.look_saturation);
    ((saturated - 0.5) * environment.look_contrast + 0.5).clamp(Vec3::ZERO, Vec3::ONE).powf(1.0 / 2.2)
}

fn lit_errors(image: &Image, environment: &VegEnvironment) -> Vec<f32> {
    let frame = Frame { position: DVec3::ZERO, forward: DVec3::NEG_Z, up: DVec3::Y, sun_to: DVec3::Y };
    let rock = lit_rock();
    let scale = Vec3::new(rock.scale, rock.scale * rock.squash, rock.scale);
    let normal = (rock.rotation * (Vec3::from(TILTED_NORMAL) / scale)).normalize();
    let direct = environment.sun_irradiance * std::f32::consts::FRAC_1_PI * normal.dot(environment.sun_direction).max(0.0);
    let mut errors = Vec::new();
    for y in (60..120).step_by(6) {
        for x in (100..220).step_by(8) {
            let ray = frame.pixel_ray(x, y).as_vec3();
            let hit = ray * (PROBE_Z / ray.z);
            let local = (rock.rotation.inverse() * Vec3::new(hit.x, hit.y, 0.0)) / scale;
            if local.x.abs() > 7.5 || local.y.abs() > 7.5 {
                continue;
            }
            let albedo = reference_albedo(local, 0.5);
            let want = reference_grade(albedo * direct + albedo * environment.ambient_mul + environment.ambient_add, environment).clamp(Vec3::ZERO, Vec3::ONE);
            let got = texel(image, x, y);
            errors.push((0..3).map(|c| (f32::from(got[c]) - srgb_level(want[c])).abs()).fold(0.0, f32::max));
        }
    }
    errors
}

#[test]
fn rock_lighting_matches_lambert_ambient_and_the_reference_grade_with_the_squash_corrected_normal() {
    let _serial = render_lock();
    let behind = VegEnvironment { sun_direction: -lit_environment().sun_direction, ..lit_environment() };
    for (label, environment) in [("lit", lit_environment()), ("graded", graded_environment()), ("sun behind", behind)] {
        let image = plane_image(ROCK_SOURCE, "fs_rock", lit_rock(), Some(environment));
        let errors = lit_errors(&image, &environment);
        assert!(errors.len() > 30, "{label}: only {} samples fall on the rock", errors.len());
        let p90 = quantile(errors, 0.9);
        println!("{label}: 90th percentile error {p90}");
        assert!(p90 <= PIXEL_TOLERANCE, "{label}: the lit rock differs from the reference by {p90} levels at the 90th percentile");
    }
}

#[test]
fn rock_bounds_keep_a_rock_whose_centre_is_outside_the_view() {
    let _serial = render_lock();
    let mut rig = rig(ROCK_SOURCE, true);
    let image = rig.show(&[Rock { scale: 4.0, ..Rock::at(Vec3::new(-7.5, 0.0, -10.0), 0) }], "edge rock");
    assert!(image_stats(&image).covered > 0.005, "the rock reaching into the view was culled: {}", image_stats(&image).covered);
}

#[test]
fn rocks_render_the_same_with_the_camera_far_from_the_render_origin() {
    let _serial = render_lock();
    let mut rig = rig(ROCK_SOURCE, true);
    let at_origin = rig.show(&near(3), "rock at origin");
    for eye in [Vec3::new(180.0, 0.0, 0.0), Vec3::new(6.0e4, 0.0, -6.0e4)] {
        let camera = rig.app.world_mut().query_filtered::<Entity, With<Camera3d>>().single(rig.app.world()).unwrap();
        *rig.app.world_mut().get_mut::<Transform>(camera).unwrap() = Transform::from_translation(eye).looking_to(Vec3::NEG_Z, Vec3::Y);
        let far = rig.show(&[Rock::at(eye + Vec3::new(0.0, 0.0, NEAR_Z), 3)], "rock far");
        let (covered, change) = (image_stats(&far).covered, mean_change(&at_origin, &far));
        println!("rock at {eye}: covered {covered} change {change}");
        assert!((covered / image_stats(&at_origin).covered - 1.0).abs() < 0.05, "a rock {eye} out covers {covered}");
        assert!(change < 0.02, "a rock {eye} out differs from the same view at the origin by {change}");
    }
}
