use super::*;
use crate::offscreen::{Frame, SKY_COLOUR, SKY_TEXEL, add_target, capture, headless_app, image_stats, mean_change, pump, render_lock, spawn_view, texel};
use crate::planet_lod::origin::FloatingOrigin;
use crate::planet_veg::gpu::VegGpuPlugin;
use crate::planet_veg::gpu::buffers::{EncodedRecords, GrassRecord, VegGpu, VegGpuConfig, VegKind};
use crate::planet_veg::gpu::draw::VegCamera;
use crate::planet_veg::gpu::store::{ChunkRef, VegStore};
use bevy::math::DVec3;
use bevy::render::render_resource::{BufferDescriptor, BufferUsages, CommandEncoderDescriptor, MapMode, PollType};
use bevy::render::renderer::{RenderDevice, RenderQueue};
use serde_json::Value;

const WIND_REFERENCE: &str = include_str!("../../../testdata/parity/spoint-planet-grass-wind.json");
const PUMP_LIMIT: usize = 900;
const PROBE_TOLERANCE: f32 = 0.006;
const PROBE_COLOUR_SCALE: f32 = 1.2;
const TOP_SWAY_WEIGHT: f32 = 0.45;
const WIND_PROBE: (&str, &str) = ("vs_grass_probe", "fs_grass_probe");
const COLOUR_PROBE: (&str, &str) = ("vs_grass_colour_probe", "fs_grass_probe");
const PROBE_HEIGHT_RANGE: f32 = 4.0;
const PROBE_WIDTH_RANGE: f32 = 6.0;
const SIZE_TOLERANCE: f32 = 0.025;
const COLOUR_TOLERANCE: f32 = 0.006;

struct Rng(u64);

impl Rng {
    fn unit(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) as f32 / 2147483648.0
    }

    fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
    }
}

struct Rig {
    app: App,
    camera: Entity,
    target: Handle<Image>,
    chunk: Option<ChunkRef>,
}

fn test_config() -> VegGpuConfig {
    VegGpuConfig { tree_species: 1, instance_capacity: [16, 16, 4096], grass_slot_capacity: 4096, ..VegGpuConfig::default() }
}

fn rig(entries: (&'static str, &'static str), format: TextureFormat) -> Rig {
    let mut app = headless_app();
    app.add_plugins((VegGpuPlugin { config: test_config() }, GrassRenderPlugin { entries }));
    app.insert_resource(FloatingOrigin { position: DVec3::new(6_360_000.0, 345.0, -2_100.0) });
    app.finish();
    app.cleanup();
    let target = add_target(&mut app, format);
    let frame = Frame { position: DVec3::ZERO, forward: DVec3::NEG_Z, up: DVec3::Y, sun_to: DVec3::new(0.3, 0.9, 0.2).normalize() };
    spawn_view(&mut app, &target, SKY_COLOUR, &frame, 1.0);
    let camera = app.world_mut().query_filtered::<Entity, With<Camera3d>>().single(app.world()).unwrap();
    app.world_mut().entity_mut(camera).insert(VegCamera);
    let mut rig = Rig { app, camera, target, chunk: None };
    rig.wait_for_draw_set();
    rig
}

impl Rig {
    fn wait_for_draw_set(&mut self) {
        for _ in 0..PUMP_LIMIT {
            if self.app.world_mut().query::<&VegDrawSet>().iter(self.app.world()).count() == 1 {
                pump(&mut self.app);
                pump(&mut self.app);
                return;
            }
            pump(&mut self.app);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("the grass draw set never appeared");
    }

    fn place_camera(&mut self, eye: Vec3, direction: Vec3) {
        *self.app.world_mut().get_mut::<Transform>(self.camera).unwrap() = Transform::from_translation(eye).looking_to(direction, Vec3::Y);
    }

    fn set_records(&mut self, records: &[GrassRecord]) {
        let origin = *self.app.world().resource::<FloatingOrigin>();
        let mut store = self.app.world_mut().resource_mut::<VegStore>();
        if let Some(chunk) = self.chunk.take() {
            store.remove_chunk(chunk);
        }
        let chunk = store.add_chunk(origin.position, Quat::IDENTITY).unwrap();
        store.upload(chunk, 10.0, EncodedRecords::new(records)).unwrap();
        self.chunk = Some(chunk);
        for _ in 0..PUMP_LIMIT {
            if self.app.world().resource::<VegStore>().pending_bytes(VegKind::Grass) == 0 {
                break;
            }
            pump(&mut self.app);
        }
        pump(&mut self.app);
        assert_eq!(self.app.world().resource::<VegStore>().pending_bytes(VegKind::Grass), 0, "grass uploads did not drain");
    }

    fn environment(&mut self, edit: impl FnOnce(&mut VegEnvironment)) {
        edit(&mut self.app.world_mut().resource_mut::<VegEnvironment>());
    }

    fn look(&mut self, edit: impl FnOnce(&mut GrassLook)) {
        edit(&mut self.app.world_mut().resource_mut::<GrassLook>());
    }

    fn cull(&mut self, edit: impl FnOnce(&mut GrassCullSettings)) {
        edit(&mut self.app.world_mut().resource_mut::<VegCullSettings>().grass);
    }

    fn shot(&mut self, label: &str) -> Image {
        capture(&mut self.app, self.target.clone(), label)
    }

    fn draw_args(&self) -> Vec<u32> {
        let world = self.app.sub_app(RenderApp).world();
        let (device, queue) = (world.resource::<RenderDevice>(), world.resource::<RenderQueue>());
        let buffer = world.resource::<VegGpu>().draws[VegKind::Grass.index()].clone();
        let bytes = 40;
        let staging = device.create_buffer(&BufferDescriptor { label: None, size: bytes, usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST, mapped_at_creation: false });
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());
        encoder.copy_buffer_to_buffer(&buffer, 0, &staging, 0, bytes);
        queue.submit(std::iter::once(encoder.finish()));
        staging.slice(..).map_async(MapMode::Read, |result| result.unwrap());
        device.wgpu_device().poll(PollType::Wait { submission_index: None, timeout: None }).unwrap();
        let data = staging.slice(..).get_mapped_range().unwrap().to_vec();
        staging.unmap();
        data.chunks_exact(4).map(|word| u32::from_le_bytes(word.try_into().unwrap())).collect()
    }
}

fn clump(rng: &mut Rng, x: f32, z: f32) -> GrassRecord {
    GrassRecord {
        local_position: [x, 0.0, z],
        scale: rng.range(1.0, 2.0),
        rotation: Quat::from_rotation_y(rng.range(0.0, 6.28)).to_array(),
        ground_rgb: [0.2, 0.4, 0.1],
        tint: rng.range(0.2, 0.8),
        rank: 1.0,
        wind_phase: 0.0,
        shadow: 1.0,
    }
}

fn field(seed: u64, count: usize, x: (f32, f32), z: (f32, f32)) -> Vec<GrassRecord> {
    let mut rng = Rng(seed);
    (0..count)
        .map(|_| {
            let (px, pz) = (rng.range(x.0, x.1), rng.range(z.0, z.1));
            clump(&mut rng, px, pz)
        })
        .collect()
}

fn mean_colour(image: &Image) -> [f64; 3] {
    let mut sum = [0.0; 3];
    let mut count: f64 = 0.0;
    for y in 0..image.height() {
        for x in 0..image.width() {
            let pixel = texel(image, x, y);
            if pixel != SKY_TEXEL {
                (0..3).for_each(|channel| sum[channel] += f64::from(pixel[channel]));
                count += 1.0;
            }
        }
    }
    sum.map(|total| total / count.max(1.0))
}

fn eye_height() -> (Vec3, Vec3) {
    (Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, -0.1, -1.0))
}

#[test]
fn grass_renders_near_and_mid_with_the_core_slots_and_every_red_check_changes_the_image() {
    let _serial = render_lock();
    let mut rig = rig(GrassRenderPlugin::default().entries, TextureFormat::Rgba8UnormSrgb);
    let (eye, direction) = eye_height();
    rig.place_camera(eye, direction);
    rig.look(|look| look.wind_amplitude = 3.0);
    rig.environment(|environment| environment.time_s = 8.0);

    let near_field = field(11, 60, (-3.0, 3.0), (-7.0, -3.0));
    rig.set_records(&near_field);
    let near = rig.shot("grass near");
    let near_stats = image_stats(&near);
    assert!(near_stats.covered > 0.02, "near clumps cover {}", near_stats.covered);
    assert!(near_stats.distinct_colours > 8, "{} colours", near_stats.distinct_colours);
    let args = rig.draw_args();
    assert_eq!((args[0], args[1] > 0), (12, true), "near slot draws the 2 segment clump: {args:?}");
    assert_eq!(args[6], 0, "no clump is beyond the near distance: {args:?}");
    let mean = mean_colour(&near);
    assert!(mean[1] > mean[0] && mean[1] > mean[2], "grass is green: {mean:?}");

    rig.cull(|grass| grass.near_m = 0.0);
    let mid_lod = rig.shot("grass near_m zero");
    let args = rig.draw_args();
    assert_eq!((args[1], args[5], args[6] > 0), (0, 6, true), "a zero near distance moves every clump to the 1 segment slot: {args:?}");
    assert!(mean_change(&near, &mid_lod) > 0.002, "the LOD switch changes the image by {}", mean_change(&near, &mid_lod));
    rig.cull(|grass| grass.near_m = 400.0);
    let all_near = rig.shot("grass near_m huge");
    assert!(mean_change(&near, &all_near) < 1e-9, "a wide near distance keeps the near image");
    rig.cull(|grass| grass.near_m = 8.0);

    rig.set_records(&field(12, 480, (-14.0, 14.0), (-40.0, -12.0)));
    let mid = rig.shot("grass mid");
    let mid_stats = image_stats(&mid);
    let args = rig.draw_args();
    assert_eq!((args[1], args[6] > 0), (0, true), "mid distance clumps use the 1 segment slot only: {args:?}");
    assert!(mid_stats.covered > 0.01, "mid clumps cover {}", mid_stats.covered);
    assert!(mid_stats.distinct_colours > 6, "{} colours", mid_stats.distinct_colours);
    assert!(mean_change(&near, &mid) > 0.01);

    rig.set_records(&near_field);
    rig.look(|look| look.wind_amplitude = 1.0);
    rig.environment(|environment| environment.time_s = 0.0);
    let calm = rig.shot("wind t0");
    rig.environment(|environment| environment.time_s = 8.0);
    let moved = rig.shot("wind t8");
    assert!(mean_change(&calm, &moved) > 0.001, "wind moves the grass by {}", mean_change(&calm, &moved));
    rig.environment(|environment| environment.wind_strength = 0.0);
    let still_a = rig.shot("wind off t8");
    rig.environment(|environment| environment.time_s = 0.0);
    let still_b = rig.shot("wind off t0");
    assert!(mean_change(&still_a, &still_b) < 1e-9, "wind off keeps the grass still");
    assert!(mean_change(&moved, &still_a) > 0.001, "wind off differs from wind on");
    rig.environment(|environment| environment.wind_strength = 1.0);

    let baseline = rig.shot("baseline");
    let tinted: Vec<_> = near_field.iter().map(|record| GrassRecord { tint: 1.0 - record.tint, ..*record }).collect();
    rig.set_records(&tinted);
    assert!(mean_change(&baseline, &rig.shot("tint")) > 0.003, "the tint term changes the image");
    let shadowed: Vec<_> = near_field.iter().map(|record| GrassRecord { shadow: 0.0, ..*record }).collect();
    rig.set_records(&shadowed);
    let dark = rig.shot("shadow");
    assert!(mean_change(&baseline, &dark) > 0.003, "the shadow term changes the image");
    assert!(mean_colour(&dark)[1] < mean_colour(&baseline)[1], "a shadowed clump is darker");
    let grounded: Vec<_> = near_field.iter().map(|record| GrassRecord { ground_rgb: [0.5, 0.2, 0.1], ..*record }).collect();
    rig.set_records(&grounded);
    assert!(mean_change(&baseline, &rig.shot("ground")) > 0.003, "the ground colour changes the image");
    rig.set_records(&near_field);
    assert!(mean_change(&baseline, &rig.shot("restored")) < 1e-9, "restoring the records restores the image");

    rig.set_records(&[GrassRecord { local_position: [0.0, 0.0, -3.0], rotation: [0.0, 0.0, 0.0, 1.0], scale: 2.0, ..clump(&mut Rng(5), 0.0, -3.0) }]);
    let single = rig.shot("single clump");
    let rows: Vec<usize> = (0..single.height()).map(|y| (0..single.width()).filter(|x| texel(&single, *x, y) != SKY_TEXEL).count()).collect();
    let covered: Vec<usize> = (0..rows.len()).filter(|y| rows[*y] > 0).collect();
    let (top, bottom) = (covered[0], covered[covered.len() - 1]);
    let third = (bottom - top) / 3;
    let (upper, lower): (usize, usize) = (rows[top..=top + third].iter().sum(), rows[bottom - third..=bottom].iter().sum());
    assert!(lower * 4 > upper * 5, "blades stand on their bases: lower third {lower} upper third {upper}");
}

fn decode(channel: u8) -> f32 {
    (f32::from(channel) / 255.0 - 0.5) / PROBE_COLOUR_SCALE
}

#[test]
fn grass_wind_displacement_equals_the_reference_wind_field_at_probe_points() {
    let _serial = render_lock();
    let reference: Value = serde_json::from_str(WIND_REFERENCE).unwrap();
    let times: Vec<f32> = reference["times"].as_array().unwrap().iter().map(|time| time.as_f64().unwrap() as f32).collect();
    let cell = |time: usize, index: usize, key: &str| reference["cells"][time][index][key].as_f64().unwrap() as f32;
    let mut rig = rig(WIND_PROBE, TextureFormat::Rgba8Unorm);
    let (eye, direction) = eye_height();
    rig.place_camera(eye, direction);
    rig.cull(|grass| {
        grass.full_radius_m = 1000.0;
        grass.edge_cap_m = 1.0e5;
    });
    rig.look(|look| look.wind_amplitude = reference["amplitude"].as_f64().unwrap() as f32);
    let blades: Vec<_> = (0..64)
        .map(|index| GrassRecord {
            local_position: [cell(0, index, "x"), 0.0, cell(0, index, "z")],
            scale: 1.0,
            rotation: [0.0, 0.0, 0.0, 1.0],
            ground_rgb: [0.2, 0.4, 0.1],
            tint: 0.5,
            rank: 1.0,
            wind_phase: (cell(0, index, "col") + 8.0 * cell(0, index, "row")).round(),
            shadow: 1.0,
        })
        .collect();
    rig.set_records(&blades);
    let mut worst = 0.0f32;
    for (at, time) in times.iter().enumerate() {
        rig.environment(|environment| environment.time_s = time / GRASS_ANIM_SPEED);
        let image = rig.shot("wind probe");
        for index in 0..64 {
            let (column, row) = (cell(at, index, "col") as u32, cell(at, index, "row") as u32);
            let pixel = probe_texel(&image, (column + 8 * row) as usize);
            assert_ne!(pixel, SKY_TEXEL, "probe cell ({column}, {row}) at t {time} was culled");
            let (got_x, got_z) = (decode(pixel[0]), decode(pixel[1]));
            let (want_x, want_z) = (cell(at, index, "dx") * TOP_SWAY_WEIGHT, cell(at, index, "dz") * TOP_SWAY_WEIGHT);
            worst = worst.max((got_x - want_x).abs()).max((got_z - want_z).abs());
            assert!((got_x - want_x).abs() < PROBE_TOLERANCE && (got_z - want_z).abs() < PROBE_TOLERANCE, "t {time} cell ({column}, {row}): got ({got_x}, {got_z}) want ({want_x}, {want_z})");
        }
    }
    assert!(worst > 1e-5, "the probe compared real offsets");
    rig.environment(|environment| environment.wind_strength = 0.0);
    let off = rig.shot("wind probe off");
    assert_probe_sizes(&mut rig);
    let pixel = texel(&off, 20, 170);
    assert!((decode(pixel[0]).abs() + decode(pixel[1]).abs()) < PROBE_TOLERANCE, "wind off writes no displacement: {pixel:?}");
}

fn probe_texel(image: &Image, cell: usize) -> [u8; 4] {
    let (column, row) = ((cell % 8) as u32, (cell / 8) as u32);
    texel(image, 40 * column + 20, 180 - (22.5 * (row as f32 + 0.5)) as u32)
}

fn smoothstep(low: f32, high: f32, x: f32) -> f32 {
    let t = ((x - low) / (high - low)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn assert_probe_sizes(rig: &mut Rig) {
    rig.cull(|grass| *grass = VegCullSettings::default().grass);
    let settings = VegCullSettings::default().grass;
    let mut rng = Rng(31);
    let blades: Vec<GrassRecord> = (0..64)
        .map(|cell| {
            let distance = 3.0 + cell as f32 * 1.03;
            let angle = rng.range(-0.3, 0.3);
            GrassRecord {
                local_position: [distance * angle.sin(), 0.0, -distance * angle.cos()],
                scale: rng.range(0.5, 2.0),
                rotation: [0.0, 0.0, 0.0, 1.0],
                ground_rgb: [0.2, 0.4, 0.1],
                tint: rng.unit(),
                rank: 0.25,
                wind_phase: cell as f32,
                shadow: 1.0,
            }
        })
        .collect();
    rig.set_records(&blades);
    let image = rig.shot("size probe");
    let mut faded = 0;
    for (cell, blade) in blades.iter().enumerate() {
        let distance = Vec2::new(blade.local_position[0], blade.local_position[2]).length();
        let edge = (settings.full_radius_m / blade.rank + settings.fade_m).min(settings.edge_cap_m);
        let fade_in = ((edge - distance) / settings.fade_m).clamp(0.0, 1.0);
        let grow = (1.0 + 0.005 * (distance - settings.full_radius_m).max(0.0)).min(1.3);
        let (height, width) = (blade.scale * settings.height_scale * fade_in * grow, (settings.width_min + settings.width_span * blade.tint) * grow);
        let pixel = probe_texel(&image, cell);
        assert_ne!(pixel, SKY_TEXEL, "size probe cell {cell} at {distance} m was culled");
        let (got_height, got_width) = (f32::from(pixel[2]) / 255.0 * PROBE_HEIGHT_RANGE, f32::from(pixel[3]) / 255.0 * PROBE_WIDTH_RANGE);
        assert!((got_height - height).abs() < SIZE_TOLERANCE && (got_width - width).abs() < SIZE_TOLERANCE, "cell {cell} at {distance} m: got {got_height} x {got_width} want {height} x {width}");
        faded += usize::from(fade_in < 1.0);
    }
    assert!(faded > 0, "the size probe reached the fade band");
}

fn reference_colour(blade: &GrassRecord) -> Vec3 {
    let ground = Vec3::from(blade.ground_rgb);
    let base_root = Vec3::new(0.19, 0.25, 0.11).lerp(ground, 0.85);
    let jitter = (1.0 + 0.8 * (blade.tint * 2.0 - 1.0)).clamp(0.2, 1.25);
    let colour = base_root.lerp(ground * 1.1 * jitter, smoothstep(0.0, 0.95, 0.75)) * 1.1 * (0.62 + smoothstep(0.0, 0.3, 0.75) * 0.38);
    let environment = VegEnvironment::default();
    let backlit = 0.5;
    colour * (Vec3::new(0.32, 0.36, 0.4) + environment.sun_irradiance * blade.shadow) + colour * environment.sun_irradiance * backlit * 0.55 * blade.shadow + colour * environment.ambient_mul + environment.ambient_add
}

#[test]
fn grass_colour_equals_the_reference_blade_colour_and_the_alpha_cut_at_probe_points() {
    let _serial = render_lock();
    let mut rig = rig(COLOUR_PROBE, TextureFormat::Rgba8Unorm);
    let (eye, direction) = eye_height();
    rig.place_camera(eye, direction);
    let mut rng = Rng(47);
    let blades: Vec<GrassRecord> = (0..64)
        .map(|cell| GrassRecord {
            local_position: [rng.range(-2.0, 2.0), 0.0, rng.range(-6.0, -3.0)],
            scale: 1.0,
            rotation: [0.0, 0.0, 0.0, 1.0],
            ground_rgb: [rng.range(0.05, 0.3), rng.range(0.1, 0.35), rng.range(0.02, 0.2)],
            tint: rng.unit(),
            rank: 1.0,
            wind_phase: cell as f32,
            shadow: rng.unit(),
        })
        .collect();
    rig.set_records(&blades);
    let image = rig.shot("colour probe");
    for (cell, blade) in blades.iter().enumerate() {
        let want = reference_colour(blade);
        let pixel = probe_texel(&image, cell);
        assert_ne!(pixel, SKY_TEXEL, "colour probe cell {cell} was culled");
        let got = Vec3::new(f32::from(pixel[0]), f32::from(pixel[1]), f32::from(pixel[2])) / 255.0;
        assert!(want.max_element() < 1.0 && (got - want).abs().max_element() < COLOUR_TOLERANCE, "cell {cell}: got {got} want {want}");
        assert_eq!(pixel[3], 0, "a clump sample of 0.5 falls below the 0.6 alpha cut");
    }
}

#[test]
fn grass_renders_the_same_with_the_camera_far_from_the_render_origin() {
    let _serial = render_lock();
    let mut rig = rig(GrassRenderPlugin::default().entries, TextureFormat::Rgba8UnormSrgb);
    let (eye, direction) = eye_height();
    let near_field = field(11, 60, (-3.0, 3.0), (-7.0, -3.0));
    rig.place_camera(eye, direction);
    rig.set_records(&near_field);
    let at_origin = rig.shot("grass at origin");
    for offset in [Vec3::new(180.0, 0.0, 0.0), Vec3::new(6.0e4, 0.0, -6.0e4)] {
        let shifted: Vec<_> = near_field.iter().map(|record| GrassRecord { local_position: (Vec3::from(record.local_position) + offset).to_array(), ..*record }).collect();
        rig.place_camera(eye + offset, direction);
        rig.set_records(&shifted);
        let far = rig.shot("grass far");
        let (covered, change) = (image_stats(&far).covered, mean_change(&at_origin, &far));
        println!("grass at {offset}: covered {covered} change {change}");
        assert!((covered / image_stats(&at_origin).covered - 1.0).abs() < 0.1, "grass {offset} out covers {covered}");
        assert!(change < 0.03, "grass {offset} out differs from the same view at the origin by {change}");
    }
}
