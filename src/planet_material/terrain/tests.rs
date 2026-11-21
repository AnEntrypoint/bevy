use super::*;
use crate::planet_material::layers::Layers;
use crate::height::climate::HpfFaces;
use crate::height::height_spec::{HashVersion as NoiseHash, HeightSpec, Params};
use crate::height::ops_jsgen::Precision;
use crate::height::{HashVersion, Planet, PlanetConfig};
use crate::planet_lod::patch::{PatchMesh, build_patch};
use crate::planet_lod::quadtree::{QuadtreeConfig, select_leaves};
use crate::planet_material::splat::{SplatParams, SplatWeights, band_warp, splat_weights};
use crate::offscreen::{FOV_Y, Frame, HEIGHT, SKY_COLOUR, WIDTH, add_target, capture, headless_app, image_stats, mean_change, render_lock, spawn_view, texel, write_png};
use bevy::mesh::{Indices, PrimitiveTopology};
use std::sync::{Mutex, OnceLock};

const SEED: i32 = 0x5eed1234;
const HPF_RES: usize = 128;
const HPF_MAX_LEVEL: u32 = 7;
const RADIUS: f64 = PLANET_RADIUS_M;
const REFERENCE_RELIEF: f64 = 0.1;
const SAMPLE_STEP_PX: u32 = 5;
const WARP_PROBE_RANGE: f64 = 7.5;
const WARP_PROBE_TOLERANCE: f64 = 1.5;
const CENSUS_AGREEMENT_MIN: f64 = 0.995;
const REMOVAL_SHARE_MIN: f64 = 0.05;
const DECODE_FRAME_LIMIT: usize = 2400;
const DECODE_POLL_MS: u64 = 50;
const CHANGE_MIN: f64 = 0.004;
const COVERED_MIN: f64 = 0.4;
const LUMA_STD_MIN: f64 = 0.02;
const DISTINCT_COLOURS_MIN: usize = 64;
fn climate() -> &'static HpfFaces {
    static CLIMATE: OnceLock<HpfFaces> = OnceLock::new();
    CLIMATE.get_or_init(|| HpfFaces::bake(i64::from(SEED), HPF_RES, HPF_MAX_LEVEL))
}

fn planet(relief: f64) -> &'static Planet {
    static PLANETS: Mutex<Vec<(f64, &'static Planet)>> = Mutex::new(Vec::new());
    let mut planets = PLANETS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((_, known)) = planets.iter().find(|(r, _)| *r == relief) {
        return known;
    }
    let config = PlanetConfig { seed: SEED, hpf_res: HPF_RES, hash: HashVersion::Integer, relief_scale: Some(relief), anchor_dir: [0.0, 1.0, 0.0], radius: RADIUS, carves: Vec::new() };
    let built: &'static Planet = Box::leak(Box::new(Planet::new(&config)));
    planets.push((relief, built));
    built
}

#[derive(Clone, Copy, Debug)]
struct Pose {
    name: &'static str,
    dir: [f64; 3],
    altitude_m: f64,
    pitch_down_deg: f64,
    relief: f64,
}

impl Pose {
    fn frame(&self) -> Frame {
        let dir = DVec3::from_array(self.dir).normalize();
        let ground = planet(self.relief).height(dir.to_array(), RADIUS);
        Frame::above(dir, RADIUS + ground, self.altitude_m, self.pitch_down_deg)
    }

    fn patches(&self) -> Vec<PatchMesh> {
        let frame = self.frame();
        let config = QuadtreeConfig::reference(RADIUS, f64::from(HEIGHT), FOV_Y);
        let ground = planet(self.relief).height(frame.position.normalize().to_array(), RADIUS);
        let keys = select_leaves(&config.at(frame.position, ground), frame.position, frame.position.length() - RADIUS);
        let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
        let per_thread = keys.len().div_ceil(threads).max(1);
        std::thread::scope(|scope| {
            let workers: Vec<_> = keys
                .chunks(per_thread)
                .map(|chunk| scope.spawn(|| chunk.iter().map(|&key| build_patch(planet(self.relief), climate(), &config, key)).collect::<Vec<_>>()))
                .collect();
            workers.into_iter().flat_map(|worker| worker.join().unwrap()).collect()
        })
    }
}

fn patch_mesh(patch: &PatchMesh) -> Mesh {
    let heights: Vec<[f32; 2]> = patch.height.iter().map(|&h| [h, 0.0]).collect();
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, patch.positions.clone())
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, patch.normals.clone())
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, patch.climate.clone())
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, heights)
        .with_inserted_indices(Indices::U32(patch.indices.clone()))
}

fn decoded_layers() -> &'static Layers {
    static LAYERS: OnceLock<Layers> = OnceLock::new();
    LAYERS.get_or_init(|| decode_layers(Path::new(TEXTURE_DIR)).unwrap())
}

#[derive(Clone, Copy)]
struct Shot {
    key: TerrainKey,
    textures: bool,
    tweak: fn(&mut TerrainParams),
    sun_side: f64,
}

fn untouched(_: &mut TerrainParams) {}

impl Shot {
    fn beauty() -> Self {
        Self { key: TerrainKey::default(), textures: true, tweak: untouched, sun_side: 1.0 }
    }

    fn census(key: TerrainKey) -> Self {
        Self { key, textures: false, tweak: untouched, sun_side: 1.0 }
    }

    fn tweaked(tweak: fn(&mut TerrainParams)) -> Self {
        Self { tweak, ..Self::beauty() }
    }

    fn sun_behind_the_planet() -> Self {
        Self { sun_side: -1.0, ..Self::beauty() }
    }
}

fn render(pose: &Pose, patches: &[PatchMesh], shot: Shot) -> Image {
    let Shot { key, textures: real_textures, tweak, sun_side } = shot;
    let _serial = render_lock();
    let frame = pose.frame();
    let census = key.census.is_some();
    let format = if census { TextureFormat::Rgba8Unorm } else { TextureFormat::Rgba8UnormSrgb };
    let clear = if census { Color::srgb(1.0, 0.0, 0.0) } else { SKY_COLOUR };
    let mut app = headless_app();
    app.add_plugins(TerrainPipelinePlugin);
    app.insert_resource(FloatingOrigin { position: frame.position });
    app.insert_resource(TerrainFrame { centre: DVec3::ZERO, radius: RADIUS, relief_scale: planet(pose.relief).relief(RADIUS) });
    app.finish();
    app.cleanup();
    app.update();
    let handle = app.world().resource::<TerrainMaterialHandle>().0.clone();
    let decoded = real_textures.then(decoded_layers);
    let arrays = decoded.map(|layers| {
        let (albedo, normals) = build_images(layers);
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        (images.add(albedo), images.add(normals))
    });
    {
        let mut materials = app.world_mut().resource_mut::<Assets<TerrainMaterial>>();
        let mut material = materials.get_mut(&handle).unwrap();
        material.extension.key = key;
        material.extension.params.surf_ready = 1.0;
        if let (Some(layers), Some((albedo, normals))) = (decoded, arrays) {
            material.extension.albedo = albedo;
            material.extension.normals = normals;
            material.extension.params.bc_rock = layers.rock_mean.into();
            material.extension.params.mean_l = layers.mean_l.into();
        }
        tweak(&mut material.extension.params);
    }
    let target = add_target(&mut app, format);
    let meshes: Vec<_> = patches.iter().map(|patch| app.world_mut().resource_mut::<Assets<Mesh>>().add(patch_mesh(patch))).collect();
    for (patch, mesh) in patches.iter().zip(meshes) {
        let offset = (patch.origin - frame.position).as_vec3();
        app.world_mut().spawn((Mesh3d(mesh), MeshMaterial3d(handle.clone()), Transform::from_translation(offset)));
    }
    spawn_view(&mut app, &target, clear, &frame, sun_side);
    capture(&mut app, target, &format!("{pose:?} {key:?}"))
}

struct Triangle {
    corners: [usize; 3],
}

struct PatchGeometry {
    vertices: Vec<DVec3>,
    normals: Vec<DVec3>,
    climate: Vec<[f32; 2]>,
    height: Vec<f32>,
    triangles: Vec<Triangle>,
    centre: DVec3,
    reach: f64,
}

fn geometry(patch: &PatchMesh, camera: DVec3) -> PatchGeometry {
    let shift = patch.origin - camera;
    let vertices: Vec<DVec3> = patch.positions.iter().map(|p| shift + DVec3::from_array(p.map(f64::from))).collect();
    let centre = vertices.iter().copied().sum::<DVec3>() / vertices.len() as f64;
    let reach = vertices.iter().map(|v| v.distance(centre)).fold(0.0, f64::max);
    PatchGeometry {
        normals: patch.normals.iter().map(|n| DVec3::from_array(n.map(f64::from))).collect(),
        climate: patch.climate.clone(),
        height: patch.height.clone(),
        triangles: patch.indices.chunks_exact(3).map(|t| Triangle { corners: [t[0] as usize, t[1] as usize, t[2] as usize] }).collect(),
        vertices,
        centre,
        reach,
    }
}

fn ray_hits_sphere(direction: DVec3, centre: DVec3, reach: f64) -> bool {
    let along = direction.dot(centre);
    along > -reach && centre.length_squared() - along * along <= reach * reach
}

fn ray_triangle(direction: DVec3, a: DVec3, b: DVec3, c: DVec3) -> Option<(f64, [f64; 3])> {
    let (edge1, edge2) = (b - a, c - a);
    let p = direction.cross(edge2);
    let det = edge1.dot(p);
    if det > -1e-18 {
        return None;
    }
    let inverse = 1.0 / det;
    let to_origin = -a;
    let u = to_origin.dot(p) * inverse;
    let q = to_origin.cross(edge1);
    let v = direction.dot(q) * inverse;
    let t = edge2.dot(q) * inverse;
    (u >= 0.0 && v >= 0.0 && u + v <= 1.0 && t > 0.0).then_some((t, [1.0 - u - v, u, v]))
}

struct SurfacePoint {
    position: DVec3,
    normal: DVec3,
    temp: f64,
    humid: f64,
    height: f64,
}

fn surface_at(patches: &[PatchGeometry], direction: DVec3) -> Option<SurfacePoint> {
    let mut best: Option<(f64, &PatchGeometry, [usize; 3], [f64; 3])> = None;
    for patch in patches.iter().filter(|patch| ray_hits_sphere(direction, patch.centre, patch.reach)) {
        for triangle in &patch.triangles {
            let [a, b, c] = triangle.corners;
            if let Some((t, weights)) = ray_triangle(direction, patch.vertices[a], patch.vertices[b], patch.vertices[c]) {
                if best.as_ref().is_none_or(|(nearest, ..)| t < *nearest) {
                    best = Some((t, patch, triangle.corners, weights));
                }
            }
        }
    }
    best.map(|(t, patch, corners, weights)| {
        let blend = |value: &dyn Fn(usize) -> f64| corners.iter().zip(weights).map(|(&i, w)| value(i) * w).sum::<f64>();
        let normal = corners.iter().zip(weights).map(|(&i, w)| patch.normals[i] * w).sum::<DVec3>();
        SurfacePoint {
            position: direction * t,
            normal,
            temp: blend(&|i| f64::from(patch.climate[i][0])),
            humid: blend(&|i| f64::from(patch.climate[i][1])),
            height: blend(&|i| f64::from(patch.height[i])),
        }
    })
}

fn noise() -> HeightSpec<'static> {
    let params = Params { hpf_res: 8.0, land_bias: 0.0, beach_shelf_m: 0.0, relief_scale: 0.0 };
    HeightSpec::new(NoiseHash::Integer, Precision::F32, params, Vec::new(), &[]).unwrap()
}

fn cpu_layer(weights: SplatWeights, removed: Option<u32>) -> u8 {
    let mut ranked = [weights.grass, weights.rock, weights.sand, weights.snow];
    if let Some(layer) = removed {
        ranked[layer as usize] = -1.0;
    }
    let mut best = 0;
    for layer in 1..ranked.len() {
        if ranked[layer] > ranked[best] {
            best = layer;
        }
    }
    best as u8
}

fn cpu_pick(spec: &HeightSpec, surface: &SurfacePoint, camera: DVec3, removed: Option<u32>) -> u8 {
    let params = SplatParams::reference();
    let dir = (camera + surface.position).normalize();
    let facing = surface.normal.normalize().lerp(dir, 0.05).normalize();
    let slope = 1.0 - facing.dot(dir).max(0.0);
    let warp = band_warp(|x, y, z| spec.snoise3([x, y, z]), dir.to_array(), params.band_warp);
    cpu_layer(splat_weights(&params, warp, surface.height, slope, surface.temp, surface.humid), removed)
}

fn warp_probe_agrees(spec: &HeightSpec, surface: &SurfacePoint, camera: DVec3, probe: u8) -> bool {
    let dir = (camera + surface.position).normalize();
    let warp = band_warp(|x, y, z| spec.snoise3([x, y, z]), dir.to_array(), SplatParams::reference().band_warp);
    let expected = (warp / WARP_PROBE_RANGE * 0.5 + 0.5) * 255.0;
    (f64::from(probe) - expected).abs() <= WARP_PROBE_TOLERANCE
}

struct CensusReport {
    counts: [usize; 4],
    compared: usize,
    agreed: usize,
    gpu_counts: [usize; 4],
    warp_agreed: usize,
}

impl CensusReport {
    fn agreement(&self) -> f64 {
        self.agreed as f64 / self.compared.max(1) as f64
    }

    fn warp_agreement(&self) -> f64 {
        self.warp_agreed as f64 / self.compared.max(1) as f64
    }

    fn share(&self, layer: usize) -> f64 {
        self.counts[layer] as f64 / self.compared.max(1) as f64
    }
}

fn census(pose: &Pose, patches: &[PatchMesh], cpu_removed: Option<u32>, gpu: &Image) -> CensusReport {
    let frame = pose.frame();
    let geometries: Vec<PatchGeometry> = patches.iter().map(|patch| geometry(patch, frame.position)).collect();
    let spec = noise();
    let mut result = CensusReport { counts: [0; 4], compared: 0, agreed: 0, gpu_counts: [0; 4], warp_agreed: 0 };
    for y in (0..HEIGHT).step_by(SAMPLE_STEP_PX as usize) {
        for x in (0..WIDTH).step_by(SAMPLE_STEP_PX as usize) {
            let Some(surface) = surface_at(&geometries, frame.pixel_ray(x, y)) else { continue };
            let want = cpu_pick(&spec, &surface, frame.position, cpu_removed);
            let got = texel(gpu, x, y)[0];
            result.compared += 1;
            result.counts[want as usize] += 1;
            if let Some(count) = result.gpu_counts.get_mut(got as usize) {
                *count += 1;
            }
            result.agreed += usize::from(got == want);
            result.warp_agreed += usize::from(warp_probe_agrees(&spec, &surface, frame.position, texel(gpu, x, y)[1]));
        }
    }
    result
}

const RIDGE: Pose = Pose { name: "ridge", dir: [-0.43198, 0.7255, 0.53576], altitude_m: 1500.0, pitch_down_deg: 40.0, relief: REFERENCE_RELIEF };
const COAST: Pose = Pose { name: "coast", dir: [0.88308, 0.44483, -0.14928], altitude_m: 1500.0, pitch_down_deg: 40.0, relief: REFERENCE_RELIEF };
const GRASSLAND: Pose = Pose { name: "grassland", dir: [-0.37963, 0.54283, -0.74914], altitude_m: 1500.0, pitch_down_deg: 40.0, relief: REFERENCE_RELIEF };
const STEEP_MIXED: Pose = Pose { name: "steep-mixed", dir: [-0.94192, 0.15, -0.30049], altitude_m: 2500.0, pitch_down_deg: 40.0, relief: 0.6 };
const STEEP_EXTREME: Pose = Pose { name: "steep-extreme", dir: [-0.94192, 0.15, -0.30049], altitude_m: 2500.0, pitch_down_deg: 40.0, relief: 1.0 };
const STEEP_ROCK: Pose = Pose { name: "steep-rock", dir: [0.2852, 0.88333, 0.372], altitude_m: 2500.0, pitch_down_deg: 40.0, relief: 0.6 };
const STEEP_SNOW: Pose = Pose { name: "steep-snow", dir: [0.48876, 0.18333, 0.85294], altitude_m: 2500.0, pitch_down_deg: 40.0, relief: 1.0 };
const STEEP_RIDGE: Pose = Pose { name: "steep-ridge", dir: [0.54016, -0.05, -0.84008], altitude_m: 2500.0, pitch_down_deg: 40.0, relief: 0.6 };
const ARID_SLOPE: Pose = Pose { name: "arid-slope", dir: [-0.57726, 0.52667, 0.62401], altitude_m: 2500.0, pitch_down_deg: 40.0, relief: 0.45 };
const SHORE: Pose = Pose { name: "shore", dir: [-0.40822, 0.81695, 0.40737], altitude_m: 250.0, pitch_down_deg: 55.0, relief: REFERENCE_RELIEF };
const CENSUS_POSES: [Pose; 10] = [RIDGE, COAST, GRASSLAND, STEEP_MIXED, STEEP_EXTREME, STEEP_ROCK, STEEP_SNOW, STEEP_RIDGE, ARID_SLOPE, SHORE];
const REMOVAL_POSES: [Pose; 2] = [RIDGE, GRASSLAND];
const ORBIT: Pose = Pose { name: "orbit", dir: RIDGE.dir, altitude_m: 9.0e6, pitch_down_deg: 90.0, relief: REFERENCE_RELIEF };
const HIGH: Pose = Pose { name: "high", dir: RIDGE.dir, altitude_m: 40_000.0, pitch_down_deg: 50.0, relief: REFERENCE_RELIEF };
const LOW: Pose = Pose { name: "low", dir: GRASSLAND.dir, altitude_m: 300.0, pitch_down_deg: 35.0, relief: REFERENCE_RELIEF };

fn census_key(remove_layer: Option<u32>) -> TerrainKey {
    TerrainKey { hash_float: false, census: Some(Census { remove_layer }) }
}

#[test]
fn the_pipeline_compiles_and_renders_a_non_trivial_image_at_orbit_high_and_low_poses() {
    let mut images = Vec::new();
    for pose in [ORBIT, HIGH, LOW] {
        let image = render(&pose, &pose.patches(), Shot::beauty());
        let measured = image_stats(&image);
        eprintln!("{} covered {:.3} luminance spread {:.3} distinct colours {}", pose.name, measured.covered, measured.luma_std, measured.distinct_colours);
        write_png(&image, &format!("planet-terrain-{}", pose.name));
        assert!(measured.covered >= COVERED_MIN, "{} covered {}", pose.name, measured.covered);
        assert!(measured.luma_std >= LUMA_STD_MIN, "{} luminance spread {}", pose.name, measured.luma_std);
        assert!(measured.distinct_colours >= DISTINCT_COLOURS_MIN, "{} distinct colours {}", pose.name, measured.distinct_colours);
        images.push(image.data);
    }
    assert!(images[0] != images[1] && images[1] != images[2] && images[0] != images[2], "poses rendered identical images");
}

#[test]
fn the_gpu_layer_census_matches_the_cpu_mirror_and_witnesses_every_layer() {
    let mut witnessed = [false; 4];
    for pose in CENSUS_POSES {
        let patches = pose.patches();
        let report = census(&pose, &patches, None, &render(&pose, &patches, Shot::census(census_key(None))));
        eprintln!("{} compared {} agreement {:.4} cpu {:?} gpu {:?}", pose.name, report.compared, report.agreement(), report.counts, report.gpu_counts);
        assert!(report.agreement() >= CENSUS_AGREEMENT_MIN, "{} agreement {} cpu {:?} gpu {:?}", pose.name, report.agreement(), report.counts, report.gpu_counts);
        assert!(report.warp_agreement() >= CENSUS_AGREEMENT_MIN, "{} band warp probe agreement {}", pose.name, report.warp_agreement());
        for layer in 0..4 {
            witnessed[layer] |= report.share(layer) >= REMOVAL_SHARE_MIN;
        }
    }
    assert_eq!(witnessed, [true; 4], "every layer must hold at least {REMOVAL_SHARE_MIN} of some pose");
}

#[test]
fn removing_a_layer_in_the_shader_removes_it_from_the_gpu_census_and_breaks_the_unmodified_comparison() {
    let mut removals = 0;
    for pose in REMOVAL_POSES {
        let patches = pose.patches();
        let baseline = census(&pose, &patches, None, &render(&pose, &patches, Shot::census(census_key(None))));
        for layer in (0..4).filter(|&layer| baseline.share(layer as usize) >= REMOVAL_SHARE_MIN) {
            let image = render(&pose, &patches, Shot::census(census_key(Some(layer))));
            let against_full = census(&pose, &patches, None, &image);
            let against_removed = census(&pose, &patches, Some(layer), &image);
            eprintln!("{} remove {layer}: vs full {:.4} vs removed {:.4} gpu {:?}", pose.name, against_full.agreement(), against_removed.agreement(), against_full.gpu_counts);
            assert!(against_full.agreement() < CENSUS_AGREEMENT_MIN, "{} remove {layer} still agrees with the full census", pose.name);
            assert_eq!(against_full.gpu_counts[layer as usize], 0, "{} layer {layer} survived its removal", pose.name);
            assert!(against_removed.agreement() >= CENSUS_AGREEMENT_MIN, "{} remove {layer} disagrees with the mirror that removes it", pose.name);
            removals += 1;
        }
    }
    assert!(removals >= 6, "removals {removals}");
}

#[test]
fn the_float_hash_on_an_integer_planet_breaks_the_census() {
    let mut worst: f64 = 1.0;
    for pose in CENSUS_POSES {
        let patches = pose.patches();
        let key = TerrainKey { hash_float: true, census: Some(Census::default()) };
        let report = census(&pose, &patches, None, &render(&pose, &patches, Shot::census(key)));
        eprintln!("{} float hash agreement {:.4}", pose.name, report.agreement());
        worst = worst.min(report.agreement());
    }
    assert!(worst < CENSUS_AGREEMENT_MIN, "worst agreement {worst}");
}

#[test]
fn interior_patch_triangles_are_wound_for_front_face_culling_on_every_face() {
    use crate::planet_lod::patch::GRID_CELLS;
    use crate::planet_lod::quadtree::NodeKey;
    let config = QuadtreeConfig::reference(RADIUS, f64::from(HEIGHT), FOV_Y);
    let last = GRID_CELLS + 2;
    let side = GRID_CELLS + 3;
    let interior = |vertex: u32| {
        let (x, y) = (vertex as usize % side, vertex as usize / side);
        (1..last).contains(&x) && (1..last).contains(&y)
    };
    for face in 0..6u8 {
        let patch = build_patch(planet(REFERENCE_RELIEF), climate(), &config, NodeKey { face, level: 2, x: 1, y: 2 });
        let mut checked = 0;
        for t in patch.indices.chunks_exact(3).filter(|t| t.iter().all(|&v| interior(v))) {
            let at = |i: u32| DVec3::from_array(patch.positions[i as usize].map(f64::from));
            let winding = (at(t[1]) - at(t[0])).cross(at(t[2]) - at(t[0]));
            let outward = DVec3::from_array(patch.normals[t[0] as usize].map(f64::from));
            assert!(winding.dot(outward) < 0.0, "face {face} triangle {t:?} is counter-clockwise from outside");
            checked += 1;
        }
        assert_eq!(checked, GRID_CELLS * GRID_CELLS * 2);
    }
}

fn reference_frame() -> TerrainFrame {
    TerrainFrame { centre: DVec3::ZERO, radius: RADIUS, relief_scale: 0.1 }
}

#[test]
fn the_frame_params_follow_the_origin_the_camera_and_the_sun() {
    let origin = FloatingOrigin { position: DVec3::new(6_360_000.0, 5.0, -7.0) };
    let params = framed(TerrainParams::default(), &reference_frame(), &origin, Vec3::new(10.0, 20.0, -30.0), Vec3::NEG_Y);
    assert_eq!(params.planet_center, Vec3::new(-6_360_000.0, -5.0, 7.0));
    assert_eq!(params.sun_dir, Vec3::Y);
    assert_eq!(params.tex_cam_frac, Vec3::new(4810.0, 25.0, 19163.0));
    assert_eq!(params.def_radius, RADIUS as f32);
    assert_eq!(params.relief_scale, 0.1);
}

#[test]
fn the_texture_camera_fraction_wraps_every_eight_tiles_and_scales_with_the_radius() {
    let small = tex_cam_frac(DVec3::new(2000.0, -1.0, 1920.0), RADIUS / 10.0);
    assert_eq!(small, Vec3::new(80.0, 1919.0, 0.0));
    assert_eq!(tex_tile_m(RADIUS / 10.0), 240.0);
}

#[test]
fn the_shader_defs_follow_the_terrain_key() {
    assert!(TerrainKey::default().shader_defs().is_empty());
    let hash = TerrainKey { hash_float: true, census: None };
    assert_eq!(hash.shader_defs(), vec![ShaderDefVal::Bool("HASH_FLOAT".into(), true)]);
    let census = TerrainKey { hash_float: false, census: Some(Census { remove_layer: None }) };
    assert_eq!(census.shader_defs(), vec![ShaderDefVal::Bool("LAYER_CENSUS".into(), true)]);
    let removed = TerrainKey { hash_float: false, census: Some(Census { remove_layer: Some(0) }) };
    assert_eq!(removed.shader_defs(), vec![ShaderDefVal::Bool("LAYER_CENSUS".into(), true), ShaderDefVal::UInt("CENSUS_REMOVE_LAYER".into(), 0)]);
}

#[test]
fn the_default_params_are_the_reference_terrain_defaults() {
    let params = TerrainParams::default();
    assert_eq!(params.pool_lo, Vec4::new(0.455, 0.408, 0.275, 0.404));
    assert_eq!(params.pool_hi, Vec4::new(0.498, 0.545, 0.388, 0.486));
    assert_eq!(params.pool_spec, Vec4::new(24.0, 256.0, 0.08, 0.25));
    assert_eq!(params.pool_cover, 0.18);
    assert_eq!(params.tex_nrm_k, 0.4);
    assert_eq!(params.relief_shade, 6.0);
    assert_eq!(params.flat_normal, 0.0);
    assert_eq!(params.wetness, 0.0);
    assert_eq!(params.bc_rock, Vec3::new(0.48, 0.40, 0.30));
    assert_eq!(params.mean_l, Vec4::new(0.2, 0.2, 0.2, 0.5));
    assert_eq!(params.surf_ready, 0.0);
    assert_eq!((TEX_TILE_M, TEX_WRAP_TILES, PLANET_RADIUS_M), (2400.0, 8.0, 6_360_000.0));
}

#[test]
fn the_decode_task_swaps_the_placeholders_and_sets_the_surface_ready_flag() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.init_asset::<Image>().init_asset::<TerrainMaterial>();
    app.add_systems(Startup, insert_material);
    app.add_plugins(TerrainLayersPlugin);
    app.update();
    let handle = app.world().resource::<TerrainMaterialHandle>().0.clone();
    let before = app.world().resource::<Assets<TerrainMaterial>>().get(&handle).unwrap().extension.clone();
    assert_eq!(before.params.surf_ready, 0.0);
    for _ in 0..DECODE_FRAME_LIMIT {
        app.update();
        if app.world().resource::<Assets<TerrainMaterial>>().get(&handle).unwrap().extension.params.surf_ready == 1.0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(DECODE_POLL_MS));
    }
    let after = app.world().resource::<Assets<TerrainMaterial>>().get(&handle).unwrap().extension.clone();
    assert_eq!(after.params.surf_ready, 1.0);
    assert!(after.albedo != before.albedo && after.normals != before.normals);
    assert_ne!(after.params.bc_rock, before.params.bc_rock);
    assert_ne!(after.params.mean_l, before.params.mean_l);
    let images = app.world().resource::<Assets<Image>>();
    for handle in [&after.albedo, &after.normals] {
        let image = images.get(handle).unwrap();
        assert_eq!((image.width(), image.height(), image.texture_descriptor.size.depth_or_array_layers), (1024, 1024, LAYER_COUNT));
    }
}

#[test]
fn every_surface_input_changes_the_low_pose_image() {
    let patches = LOW.patches();
    let baseline = render(&LOW, &patches, Shot::beauty());
    let tweaks: [(&str, fn(&mut TerrainParams)); 6] = [
        ("placeholder textures", |p| p.surf_ready = 0.0),
        ("no detail normals", |p| p.tex_nrm_k = 0.0),
        ("no relief shade", |p| p.relief_shade = 1.0),
        ("wet ground", |p| p.wetness = 1.0),
        ("albedo override", |p| p.albedo_override = Vec4::new(1.0, 0.0, 0.0, 1.0)),
        ("flat normal", |p| p.flat_normal = 1.0),
    ];
    let mut shots: Vec<(&str, Shot)> = tweaks.into_iter().map(|(name, tweak)| (name, Shot::tweaked(tweak))).collect();
    shots.push(("sun behind the planet", Shot::sun_behind_the_planet()));
    for (name, shot) in shots {
        let difference = mean_change(&baseline, &render(&LOW, &patches, shot));
        eprintln!("{name}: mean difference {difference:.4}");
        assert!(difference >= CHANGE_MIN, "{name} changed the image by only {difference}");
    }
}

#[test]
fn the_frame_system_writes_the_material_each_frame_from_origin_camera_and_sun() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), TransformPlugin));
    app.init_asset::<Image>().init_asset::<TerrainMaterial>();
    app.insert_resource(FloatingOrigin { position: DVec3::new(6_360_000.0, 5.0, -7.0) });
    app.insert_resource(reference_frame());
    app.add_systems(Startup, insert_material);
    app.add_systems(Update, update_frame.run_if(resource_exists::<TerrainFrame>).run_if(resource_exists::<TerrainMaterialHandle>));
    app.world_mut().spawn((Camera3d::default(), Transform::from_xyz(10.0, 20.0, -30.0)));
    app.world_mut().spawn((DirectionalLight::default(), Transform::IDENTITY.looking_to(Vec3::NEG_Y, Vec3::Z)));
    for _ in 0..3 {
        app.update();
    }
    let handle = app.world().resource::<TerrainMaterialHandle>().0.clone();
    let params = app.world().resource::<Assets<TerrainMaterial>>().get(&handle).unwrap().extension.params;
    assert_eq!(params.planet_center, Vec3::new(-6_360_000.0, -5.0, 7.0));
    assert!((params.sun_dir - Vec3::Y).length() < 1e-6, "sun_dir {:?}", params.sun_dir);
    assert_eq!(params.tex_cam_frac, Vec3::new(4810.0, 25.0, 19163.0));
}

