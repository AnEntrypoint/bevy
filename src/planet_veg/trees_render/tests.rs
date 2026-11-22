use super::gpu::TreeGpu;
use super::*;
use crate::offscreen::{Frame, HEIGHT, SKY_COLOUR, SKY_TEXEL, WIDTH, add_target, capture, headless_app, image_stats, mean_change, pump, render_lock, spawn_view, texel};
use crate::planet_lod::origin::FloatingOrigin;
use crate::planet_veg::gpu::VegGpuPlugin;
use crate::planet_veg::gpu::buffers::{EncodedRecords, TreeRecord, VegKind, tint_rgb8};
use crate::planet_veg::gpu::cull::VegCullSettings;
use crate::planet_veg::gpu::draw::{VegCamera, VegEnvironment};
use crate::planet_veg::gpu::store::{ChunkRef, VegStore};
use assets::species_directory;
use bevy::math::DVec3;
use bevy::render::RenderApp;
use bevy::render::render_resource::{
    BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Extent3d, MapMode, Origin3d, PollType, TexelCopyBufferInfo, TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect, TextureFormat,
};
use bevy::render::renderer::{RenderDevice, RenderQueue};
use std::collections::HashSet;

const PUMP_LIMIT: usize = 900;
const ATLAS_PX: u32 = 2048;
const ATLAS_MIPS: u32 = 12;
const SPRITES_PER_SIDE: u32 = 8;
const SPRITE_PX: u32 = 64;
const DISSOLVE_BAND_M: f32 = 3.0;
const NO_LIMIT_M: f32 = 1.0e9;
const IMAGE_CHANGE_FLOOR: f64 = 0.002;
const BUDGET_CHANGE_FLOOR: f64 = 1.0e-4;

struct Rig {
    app: App,
    camera: Entity,
    target: Handle<Image>,
    witness: TreesWitness,
    origin: FloatingOrigin,
}

struct Pose {
    species: u8,
    position: Vec3,
    scale: f32,
    yaw: f32,
    wind_phase: f32,
    tint: [f32; 3],
}

#[derive(Clone, Copy, Debug)]
enum Band {
    Tier0,
    Tier1,
    Tier2,
    Impostor,
}

fn plugin() -> TreesRenderPlugin {
    TreesRenderPlugin { directory: species_directory(), ..default() }
}

fn unbaked() -> TreesRenderPlugin {
    TreesRenderPlugin { bake_atlas: false, ..plugin() }
}

fn rig_with(plugin: TreesRenderPlugin, format: TextureFormat, clear: Color) -> Rig {
    let mut app = headless_app();
    let origin = FloatingOrigin { position: DVec3::new(6_360_000.0, 345.0, -2_100.0) };
    let bake = plugin.bake_atlas;
    app.add_plugins((VegGpuPlugin::default(), plugin));
    app.insert_resource(origin);
    app.finish();
    app.cleanup();
    let target = add_target(&mut app, format);
    let frame = Frame { position: DVec3::ZERO, forward: DVec3::NEG_Z, up: DVec3::Y, sun_to: DVec3::new(0.3, 0.9, 0.2).normalize() };
    spawn_view(&mut app, &target, clear, &frame, 1.0);
    let camera = app.world_mut().query_filtered::<Entity, With<Camera3d>>().single(app.world()).unwrap();
    app.world_mut().entity_mut(camera).insert(VegCamera);
    let witness = app.world().resource::<TreesWitness>().clone();
    let mut rig = Rig { app, camera, target, witness, origin };
    rig.wait_for(|witness| witness.loaded.load(Ordering::SeqCst) || witness.refusal.lock().unwrap().is_some(), "the species load");
    assert!(rig.witness.refusal.lock().unwrap().is_none(), "{:?}", rig.witness.refusal.lock().unwrap());
    if bake {
        rig.wait_for(|witness| witness.baked.load(Ordering::SeqCst), "the atlas bake");
    }
    rig
}

fn rig(plugin: TreesRenderPlugin) -> Rig {
    rig_with(plugin, TextureFormat::Rgba8UnormSrgb, SKY_COLOUR)
}

impl Rig {
    fn wait_for(&mut self, done: impl Fn(&TreesWitness) -> bool, what: &str) {
        for _ in 0..PUMP_LIMIT {
            if done(&self.witness) {
                return;
            }
            pump(&mut self.app);
        }
        panic!("{what} did not finish in {PUMP_LIMIT} frames");
    }

    fn plant(&mut self, poses: &[Pose]) -> ChunkRef {
        let records: Vec<_> = poses
            .iter()
            .enumerate()
            .map(|(index, pose)| TreeRecord {
                local_position: pose.position.to_array(),
                scale: pose.scale,
                tilt: [0.0, 0.0, 0.0, 1.0],
                yaw: pose.yaw,
                wind_phase: pose.wind_phase,
                tint_rgb8: tint_rgb8(pose.tint),
                trunk_id: index as u32,
                species: pose.species,
                shape: 0,
            })
            .collect();
        let mut store = self.app.world_mut().resource_mut::<VegStore>();
        let chunk = store.add_chunk(self.origin.position, Quat::IDENTITY).unwrap();
        store.upload(chunk, 10.0, EncodedRecords::new(&records)).unwrap();
        for _ in 0..PUMP_LIMIT {
            if self.app.world().resource::<VegStore>().pending_bytes(VegKind::Tree) == 0 {
                break;
            }
            pump(&mut self.app);
        }
        chunk
    }

    fn uproot(&mut self, chunk: ChunkRef) {
        assert!(self.app.world_mut().resource_mut::<VegStore>().remove_chunk(chunk));
    }

    fn look(&mut self, eye: Vec3, towards: Vec3) {
        *self.app.world_mut().get_mut::<Transform>(self.camera).unwrap() = Transform::from_translation(eye).looking_to(towards, Vec3::Y);
    }

    fn wind(&mut self, time_s: f32, wind_strength: f32) {
        let mut environment = self.app.world_mut().resource_mut::<VegEnvironment>();
        environment.time_s = time_s;
        environment.wind_strength = wind_strength;
    }

    fn shoot(&mut self, band: Band) -> Image {
        self.force(band);
        capture(&mut self.app, self.target.clone(), &format!("{band:?}"))
    }

    fn force(&mut self, band: Band) {
        let mut settings = self.app.world_mut().resource_mut::<VegCullSettings>();
        let tree = &mut settings.tree;
        tree.ring_enter_m = NO_LIMIT_M;
        tree.ring_drop_m = NO_LIMIT_M;
        tree.falloff_near_m = NO_LIMIT_M;
        tree.d1_m = if matches!(band, Band::Tier0) { NO_LIMIT_M } else { 0.0 };
        tree.d2_m = if matches!(band, Band::Tier0 | Band::Tier1) { NO_LIMIT_M } else { 0.0 };
        tree.mesh_far_m = if matches!(band, Band::Impostor) { 0.0 } else { NO_LIMIT_M };
        tree.mesh_reentry_m = tree.mesh_far_m;
        tree.impostor_start_m = if matches!(band, Band::Impostor) { 0.0 } else { NO_LIMIT_M };
    }

    fn atlas(&self, albedo: bool, level: u32) -> Vec<u8> {
        let world = self.app.sub_app(RenderApp).world();
        let gpu = world.resource::<TreeGpu>();
        let texture = if albedo { &gpu.atlas.albedo } else { &gpu.atlas.normal_depth };
        assert_eq!((texture.width(), texture.height(), texture.mip_level_count()), (ATLAS_PX, ATLAS_PX, ATLAS_MIPS));
        let size = ATLAS_PX >> level;
        let (device, queue) = (world.resource::<RenderDevice>(), world.resource::<RenderQueue>());
        let staging = device.create_buffer(&BufferDescriptor { label: None, size: u64::from(size * size * 4), usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST, mapped_at_creation: false });
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());
        encoder.copy_texture_to_buffer(
            TexelCopyTextureInfo { texture, mip_level: level, origin: Origin3d::ZERO, aspect: TextureAspect::All },
            TexelCopyBufferInfo { buffer: &staging, layout: TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(size * 4), rows_per_image: Some(size) } },
            Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        );
        queue.submit(std::iter::once(encoder.finish()));
        staging.slice(..).map_async(MapMode::Read, |result| result.unwrap());
        device.wgpu_device().poll(PollType::Wait { submission_index: None, timeout: None }).unwrap();
        let data = staging.slice(..).get_mapped_range().unwrap().to_vec();
        staging.unmap();
        data
    }
}

struct Lcg(u64);

impl Lcg {
    fn range(&mut self, low: f32, high: f32) -> f32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        low + (high - low) * ((self.0 >> 40) as f32 / 16777216.0)
    }
}

fn forest() -> Vec<Pose> {
    let mut random = Lcg(0x7ee5);
    (0..300)
        .map(|index| Pose {
            species: (index % 15) as u8,
            position: Vec3::new(random.range(-40.0, 40.0), 0.0, random.range(-95.0, -9.0)),
            scale: random.range(0.8, 1.4),
            yaw: random.range(0.0, 6.28),
            wind_phase: random.range(0.0, 6.28),
            tint: [random.range(0.6, 1.4), random.range(0.6, 1.4), random.range(0.6, 1.4)],
        })
        .collect()
}

fn forest_rig(plugin: TreesRenderPlugin) -> Rig {
    let mut rig = rig(plugin);
    rig.plant(&forest());
    rig.look(Vec3::new(0.0, 2.0, 0.0), Vec3::new(0.0, -0.04, -1.0));
    rig
}

fn difference(a: &Image, b: &Image) -> f64 {
    mean_change(a, b).max(mean_change(b, a))
}

fn assert_changes(label: &str, a: &Image, b: &Image) {
    let change = difference(a, b);
    assert!(change > IMAGE_CHANGE_FLOOR, "{label}: the image changes by only {change}");
}

fn assert_rendered(label: &str, image: &Image) {
    let stats = image_stats(image);
    assert!(stats.covered > 0.02 && stats.distinct_colours > 40, "{label}: covered {} with {} colours", stats.covered, stats.distinct_colours);
}

#[test]
fn the_forest_renders_at_every_tier_and_as_impostors_and_each_band_changes_the_image() {
    let _serial = render_lock();
    let mut rig = forest_rig(plugin());
    let bands = [Band::Tier0, Band::Tier1, Band::Tier2, Band::Impostor];
    let images: Vec<_> = bands.iter().map(|band| rig.shoot(*band)).collect();
    for (band, image) in bands.iter().zip(&images) {
        assert_rendered(&format!("{band:?}"), image);
    }
    for at in 1..bands.len() {
        assert_changes(&format!("{:?} to {:?}", bands[at - 1], bands[at]), &images[at - 1], &images[at]);
    }
    rig.wind(2.0, 1.0);
    assert_changes("mesh wind at a later time", &images[0], &rig.shoot(Band::Tier0));
    assert_changes("impostor wind at a later time", &images[3], &rig.shoot(Band::Impostor));
    rig.wind(2.0, 0.0);
    let still = rig.shoot(Band::Tier0);
    rig.wind(0.0, 0.0);
    assert_eq!(difference(&still, &rig.shoot(Band::Tier0)), 0.0, "without wind the time does not move the trees");
    assert_changes("wind off", &images[0], &still);
}

fn sprite_pixels(data: &[u8], species: u32, row: u32, column: u32) -> impl Iterator<Item = [u8; 4]> + '_ {
    let x0 = ((species % 4) * SPRITES_PER_SIDE + column) * SPRITE_PX;
    let y0 = ATLAS_PX - ((species / 4) * SPRITES_PER_SIDE + row + 1) * SPRITE_PX;
    (0..SPRITE_PX * SPRITE_PX).map(move |at| {
        let offset = (((y0 + at / SPRITE_PX) * ATLAS_PX + x0 + at % SPRITE_PX) * 4) as usize;
        [data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]
    })
}

#[test]
fn the_bake_fills_every_sprite_of_every_species_tile_and_its_mips() {
    let _serial = render_lock();
    let rig = rig(plugin());
    let (albedo, normal) = (rig.atlas(true, 0), rig.atlas(false, 0));
    let mut tiles = HashSet::new();
    for species in 0..15 {
        let (mut sprites, mut directions) = (HashSet::new(), HashSet::new());
        for row in 0..SPRITES_PER_SIDE {
            for column in 0..SPRITES_PER_SIDE {
                let opaque = sprite_pixels(&albedo, species, row, column).filter(|texel| texel[3] > 102).count();
                let coverage = opaque as f64 / f64::from(SPRITE_PX * SPRITE_PX);
                assert!(coverage > 0.005 && coverage < 0.95, "species {species} sprite ({column}, {row}) coverage {coverage}");
                assert!(sprite_pixels(&normal, species, row, column).any(|texel| texel[3] > 0), "species {species} sprite ({column}, {row}) has no normal-depth");
                sprites.insert(sprite_pixels(&albedo, species, row, column).collect::<Vec<_>>());
                directions.insert(octahedral_direction(column as f32, row as f32).to_array().map(|axis| (axis * 1.0e4).round() as i32));
            }
        }
        assert!(sprites.len() >= directions.len(), "species {species}: {} distinct sprites for {} distinct directions", sprites.len(), directions.len());
        tiles.insert(sprite_pixels(&albedo, species, 4, 3).map(|texel| u64::from(texel[0]) * 3 + u64::from(texel[1])).sum::<u64>());
    }
    assert_eq!(tiles.len(), 15, "each species has its own tile");
    assert!(sprite_pixels(&albedo, 15, 3, 3).all(|texel| texel == [0; 4]), "the spare tile stays clear");
    let mip = rig.atlas(true, 3);
    assert!(mip.chunks_exact(4).any(|texel| texel[3] > 0), "the mip chain is filled");
}

fn octahedral_direction(column: f32, row: f32) -> Vec3 {
    let (px, pz) = ((column / 7.0 - 0.5) * 2.0, (row / 7.0 - 0.5) * 2.0);
    let py = 1.0 - px.abs() - pz.abs();
    let fold = |a: f32, b: f32| if py < 0.0 { if a >= 0.0 { 1.0 - b.abs() } else { b.abs() - 1.0 } } else { a };
    Vec3::new(fold(px, pz), py, fold(pz, px)).normalize()
}

const SILHOUETTE_BLOCK_PX: u32 = 3;
const ROUND_TRIP_CELLS: [(f32, f32); 5] = [(4.0, 4.0), (3.0, 1.0), (6.0, 3.0), (1.0, 1.0), (5.0, 6.0)];
const ROUND_TRIP_SPECIES: [u8; 2] = [0, 14];
const ROUND_TRIP_RADII: f32 = 8.0;
const ROUND_TRIP_OVERLAP_FLOOR: f64 = 0.55;
const ROUND_TRIP_COVERAGE_RATIO: (f64, f64) = (0.7, 1.45);
const ROUND_TRIP_COLOUR_CEILING: f64 = 0.03;

fn silhouette(image: &Image) -> HashSet<(u32, u32)> {
    (0..HEIGHT).flat_map(|y| (0..WIDTH).map(move |x| (x, y))).filter(|(x, y)| texel(image, *x, *y) != SKY_TEXEL).map(|(x, y)| (x / SILHOUETTE_BLOCK_PX, y / SILHOUETTE_BLOCK_PX)).collect()
}

fn silhouette_overlap(a: &Image, b: &Image) -> f64 {
    let (a, b) = (silhouette(a), silhouette(b));
    a.intersection(&b).count() as f64 / a.union(&b).count().max(1) as f64
}

fn mean_colour(image: &Image) -> [f64; 3] {
    let texels: Vec<_> = (0..HEIGHT).flat_map(|y| (0..WIDTH).map(move |x| (x, y))).map(|(x, y)| texel(image, x, y)).filter(|texel| *texel != SKY_TEXEL).collect();
    let count = texels.len().max(1) as f64;
    [0, 1, 2].map(|channel| texels.iter().map(|texel| f64::from(texel[channel])).sum::<f64>() / count / 255.0)
}

fn colour_gap(a: &Image, b: &Image) -> f64 {
    let (a, b) = (mean_colour(a), mean_colour(b));
    (0..3).map(|channel| (a[channel] - b[channel]).abs()).fold(0.0, f64::max)
}

fn single(species: u8, position: Vec3) -> Pose {
    Pose { species, position, scale: 1.0, yaw: 0.0, wind_phase: 0.0, tint: [1.0; 3] }
}

#[test]
fn an_impostor_matches_the_mesh_of_the_same_tree_seen_from_the_same_direction() {
    let _serial = render_lock();
    let mut rig = rig(plugin());
    rig.wind(0.0, 0.0);
    let assets = rig.app.world().resource::<TreeRenderData>().assets.clone();
    for (column, row) in ROUND_TRIP_CELLS {
        let direction = octahedral_direction(column, row);
        let views: Vec<_> = ROUND_TRIP_SPECIES
            .iter()
            .map(|species| {
                let sphere = assets.spheres[usize::from(*species)];
                let chunk = rig.plant(&[single(*species, -direction * sphere[3] * ROUND_TRIP_RADII - Vec3::new(sphere[0], sphere[1], sphere[2]))]);
                rig.look(Vec3::ZERO, -direction);
                let view = (rig.shoot(Band::Tier0), rig.shoot(Band::Impostor));
                rig.uproot(chunk);
                view
            })
            .collect();
        for (at, species) in ROUND_TRIP_SPECIES.iter().enumerate() {
            let (mesh, impostor) = &views[at];
            let other = &views[1 - at].1;
            let (overlap, control) = (silhouette_overlap(mesh, impostor), silhouette_overlap(mesh, other));
            let (coverage, gap) = (image_stats(impostor).covered / image_stats(mesh).covered.max(1e-9), colour_gap(mesh, impostor));
            let context = format!("species {species} cell ({column}, {row}): overlap {overlap:.3}, other species {control:.3}, coverage ratio {coverage:.3}, colour gap {gap:.4}");
            assert!(image_stats(mesh).covered > 5e-4, "{context}: the mesh is not in view");
            assert!(overlap > ROUND_TRIP_OVERLAP_FLOOR && overlap > control, "{context}");
            assert!((ROUND_TRIP_COVERAGE_RATIO.0..ROUND_TRIP_COVERAGE_RATIO.1).contains(&coverage), "{context}");
            assert!(gap < ROUND_TRIP_COLOUR_CEILING, "{context}");
        }
    }
}

fn probe_texel(image: &Image, x: u32, y: u32) -> [f32; 4] {
    let at = ((y * image.width() + x) * 16) as usize;
    let data = image.data.as_ref().unwrap();
    [0, 1, 2, 3].map(|channel| f32::from_le_bytes(data[at + channel * 4..at + channel * 4 + 4].try_into().unwrap()))
}

fn probe_quad() -> Mesh {
    veg_mesh(vec![[-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 10.0, 0.0], [-1.0, 10.0, 0.0]], vec![[0.0, 0.0, 1.0]; 4], vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]], vec![0, 1, 2, 0, 2, 3])
}

#[test]
fn the_wind_probe_equals_the_reference_sway_at_every_covered_pixel() {
    let _serial = render_lock();
    let mut rig = rig_with(unbaked(), TextureFormat::Rgba32Float, Color::NONE);
    let world = rig.app.world_mut();
    let sets: Vec<Entity> = world.query_filtered::<Entity, With<VegDrawSet>>().iter(world).collect();
    sets.into_iter().for_each(|entity| {
        world.despawn(entity);
    });
    let quad = world.resource_mut::<Assets<Mesh>>().add(probe_quad());
    let config = world.resource::<VegGpuConfig>().clone();
    world.spawn(VegDrawSet::tree(&config, 0, [quad.clone(), quad.clone(), quad], tree_probe_material()));
    let phases = [1.7f32, 4.0];
    rig.plant(&phases.map(|wind_phase| Pose { wind_phase, ..single(0, Vec3::new(if wind_phase < 3.0 { -3.0 } else { 3.0 }, -5.0, -12.0)) }));
    rig.look(Vec3::ZERO, Vec3::NEG_Z);
    for (time_s, wind_strength) in [(0.0f32, 1.0f32), (2.5, 1.0), (7.1, 0.5), (3.3, 0.0)] {
        rig.wind(time_s, wind_strength);
        let image = rig.shoot(Band::Tier0);
        let mut checked = [0usize; 2];
        let mut top = [0.0f32; 2];
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let [dx, dz, height, coverage] = probe_texel(&image, x, y);
                if coverage != 1.0 {
                    continue;
                }
                let side = usize::from(x >= WIDTH / 2);
                let phase = time_s * 1.3 + phases[side];
                let sway = height * 0.06 * wind_strength;
                let context = format!("time {time_s} wind {wind_strength} phase {} pixel ({x}, {y}) height {height}", phases[side]);
                assert!((dx - phase.sin() * sway).abs() < 1e-3, "{context}: x sway {dx} against {}", phase.sin() * sway);
                assert!((dz - (phase * 0.8).cos() * sway * 0.6).abs() < 1e-3, "{context}: z sway {dz} against {}", (phase * 0.8).cos() * sway * 0.6);
                checked[side] += 1;
                top[side] = top[side].max(height);
            }
        }
        assert!(checked.iter().all(|count| *count > 300), "time {time_s}: probe pixels {checked:?}");
        assert!(top.iter().all(|height| *height > 9.0), "time {time_s}: the top vertex reached {top:?}");
    }
}

#[test]
fn knocking_out_the_atlas_alpha_test_tint_or_texture_budget_changes_the_image() {
    let _serial = render_lock();
    let baseline = forest_rig(unbaked()).shoot(Band::Tier0);
    assert_rendered("baseline", &baseline);
    let unbaked_impostors = forest_rig(unbaked()).shoot(Band::Impostor);
    assert!(image_stats(&unbaked_impostors).covered < 1e-4, "impostors without a baked atlas still cover {}", image_stats(&unbaked_impostors).covered);
    let no_alpha = forest_rig(TreesRenderPlugin { look: TreeLook { alpha_test: false, ..default() }, ..unbaked() }).shoot(Band::Tier0);
    assert!(image_stats(&no_alpha).covered > image_stats(&baseline).covered, "leaf cards without the alpha test cover more");
    assert_changes("alpha test removed", &baseline, &no_alpha);
    let no_tint = forest_rig(TreesRenderPlugin { look: TreeLook { tint: false, ..default() }, ..unbaked() }).shoot(Band::Tier0);
    assert_changes("tint ignored", &baseline, &no_tint);
    let close_up = |plugin: TreesRenderPlugin| {
        let mut rig = rig(plugin);
        rig.plant(&[single(0, Vec3::new(0.0, -1.5, -1.6))]);
        rig.look(Vec3::ZERO, Vec3::NEG_Z);
        rig.shoot(Band::Tier0)
    };
    let budget_change = difference(&close_up(unbaked()), &close_up(TreesRenderPlugin { budget: TextureBudget::new(1024, 1024).unwrap(), ..unbaked() }));
    assert!(budget_change > BUDGET_CHANGE_FLOOR, "the texture budget changes the close-up by {budget_change}");
}



#[test]
fn the_impostor_dissolves_in_over_three_metres_before_the_near_cutoff() {
    let _serial = render_lock();
    let mut rig = rig(plugin());
    let assets = rig.app.world().resource::<TreeRenderData>().assets.clone();
    let sphere = assets.spheres[0];
    let distance_m = sphere[3] * ROUND_TRIP_RADII;
    rig.plant(&[single(0, Vec3::new(-sphere[0], -sphere[1], -sphere[2] - distance_m))]);
    rig.wind(0.0, 0.0);
    rig.look(Vec3::ZERO, Vec3::NEG_Z);
    let mut coverage_at = |fade: f32| {
        rig.force(Band::Impostor);
        rig.app.world_mut().resource_mut::<VegCullSettings>().tree.impostor_start_m = distance_m - fade * DISSOLVE_BAND_M;
        image_stats(&capture(&mut rig.app, rig.target.clone(), "dissolve")).covered
    };
    let (beyond, full, half, none) = (coverage_at(4.0), coverage_at(1.05), coverage_at(0.5), coverage_at(0.0));
    assert!(full > 5e-4, "the impostor is in view: {full}");
    assert!(full > 0.95 * beyond, "just past the band the impostor is whole: {full} against {beyond}");
    assert!((0.3..0.7).contains(&(half / full)), "half way through the band the dither keeps {} of the impostor", half / full);
    assert!(none < 0.1 * full, "at the start of the band the impostor is dissolved: {none} of {full}");
}
