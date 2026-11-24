#![allow(dead_code)]

use bevy::asset::{AssetMetaCheck, AssetPlugin, LoadState};
use bevy::image::{CompressedImageFormatSupport, CompressedImageFormats};
use bevy::render::renderer::RenderAdapterInfo;
use bevy::prelude::*;
use bevy::render::settings::{RenderCreation, WgpuFeatures, WgpuSettings};
use bevy::render::RenderPlugin;
use bevy::diagnostic::{
    DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin,
    LogDiagnosticsPlugin,
};
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy::world_serialization::{WorldAsset, WorldAssetRoot};
use wasm_bindgen::prelude::*;
use crate::globe::GlobeWorld;
use crate::offscreen::Frame;
use crate::planet_lod::collider::NearBody;
use crate::planet_lod::stream::PlanetLodStats;
use crate::planet_poses::Pose;
use crate::planet_scene::{SETTLED_STREAK, unsettled};
use crate::planet_veg::stream::VegStreamStats;
use crate::planet_veg::trees_render::TreesWitness;

#[path = "../../src/render_stats.rs"]
mod render_stats;
mod planet_lod;
mod planet_veg;
mod planet_water;
mod planet_sky;
mod planet_material;
mod height;
mod globe;
mod planet_scene;
mod planet_poses;
mod frame_clock;
mod vec3;
mod js_math;
mod refusal;
mod offscreen;
mod spec;
mod rock_shapes;
mod cave_sdf;
mod rock_tri_table;
mod planet_assets;
mod script;
use render_stats::{is_gpu_pass, RenderStatsPlugin, SharedFrameStats};
use bevy::render::diagnostic::RenderDiagnosticsPlugin;

#[derive(Component)]
struct OrbitCamera;

#[derive(Component)]
struct Spinner;

#[derive(Resource)]
struct ReadySent(bool);

#[derive(Resource)]
struct ModelRequest(Option<String>);

const BC7_CACHE: &str = "target/gltf-cache/";
const RGBA8_CACHE: &str = "target/gltf-cache-rgba8/";
const ETC2_CACHE: &str = "target/gltf-cache-etc2/";
const ASTC_CACHE: &str = "target/gltf-cache-astc/";

#[derive(Resource)]
struct ModelHandle(Handle<WorldAsset>);

#[derive(Resource, Default)]
struct ModelReported(bool);

#[derive(Resource)]
struct Still(bool);

#[derive(Resource)]
struct InspectorState {
    enabled: bool,
    last_update: f64,
}

#[wasm_bindgen]
pub fn start() {
    let inspector_enabled = js_sys::Reflect::get(
        &web_sys::window().expect("window"),
        &JsValue::from_str("__BEVY_INSPECTOR_ENABLED"),
    )
    .ok()
    .and_then(|value| value.as_bool())
    .unwrap_or(false);
    let model = model_request();
    let planet = model.is_none();
    let pose = if planet { pose_request() } else { None };
    let asset_root = if model.is_some() { "/" } else { "assets" };
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(AssetPlugin { file_path: asset_root.to_string(), meta_check: AssetMetaCheck::Never, ..default() }).set(WindowPlugin {
        primary_window: Some(Window {
            title: "bevy-jayzee WebGPU WASM".to_string(),
            canvas: Some("#bevy-canvas".to_string()),
            fit_canvas_to_parent: true,
            resolution: WindowResolution::new(960, 540),
            ..default()
        }),
        ..default()
    })
    .set(RenderPlugin {
        render_creation: device_request(query_value("disable-bc").as_deref() == Some("1"), query_value("disable-astc").as_deref() == Some("1")),
        ..default()
    }))
    .add_plugins((
        FrameTimeDiagnosticsPlugin::default(),
        EntityCountDiagnosticsPlugin::default(),
        LogDiagnosticsPlugin::default(),
        RenderDiagnosticsPlugin,
        RenderStatsPlugin,
    ))
    .insert_resource(ClearColor(Color::srgb(0.025, 0.04, 0.07)))
    .insert_resource(ReadySent(false))
    .insert_resource(ModelRequest(model))
    .insert_resource(ModelReported(false))
    .insert_resource(Still(query_value("still").as_deref() == Some("1")))
    .insert_resource(InspectorState { enabled: inspector_enabled, last_update: 0.0 })
    .add_systems(Update, (spawn_model, orbit, spin, mark_ready, report_model, publish_diagnostics));
    if planet {
        add_planet(&mut app, pose);
    } else {
        app.add_systems(Startup, setup);
    }
    app.run();
}

fn add_planet(app: &mut App, pose: Option<Pose>) {
    app.add_plugins(avian3d::prelude::PhysicsPlugins::default())
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .insert_resource(GlobeWorld::default())
        .add_systems(Startup, spawn_planet_camera);
    match pose {
        Some(pose) => {
            app.insert_resource(PoseDriver { pose, frame: None, streak: 0 }).add_systems(Update, drive_pose);
        }
        None => {
            app.add_systems(Update, globe::frame_globe_camera);
        }
    }
    globe::register(app);
}

fn pose_request() -> Option<Pose> {
    match query_value("pose")?.as_str() {
        "orbit" => Some(Pose::Orbit),
        "high" => Some(Pose::High),
        "low" => Some(Pose::Low),
        "ground" => Some(Pose::Ground),
        "shoreline" => Some(Pose::Shoreline),
        _ => None,
    }
}

#[derive(Resource)]
struct PoseDriver {
    pose: Pose,
    frame: Option<Frame>,
    streak: usize,
}

fn drive_pose(world: &mut World) {
    let frame = match world.resource::<PoseDriver>().frame.clone() {
        Some(frame) => frame,
        None => {
            let pose = world.resource::<PoseDriver>().pose;
            let globe = world.resource::<GlobeWorld>();
            let radius = f64::from(globe.radius);
            let frame = pose.frame(radius, |dir| globe.planet.height(dir.to_array(), radius));
            world.resource_mut::<PoseDriver>().frame = Some(frame.clone());
            frame
        }
    };
    let translation = frame.position.as_vec3();
    let mut cameras = world.query_filtered::<&mut Transform, With<Camera3d>>();
    for mut transform in cameras.iter_mut(world) {
        *transform = Transform::from_translation(translation).looking_to(frame.forward.as_vec3(), frame.up.as_vec3());
    }
    let mut bodies = world.query_filtered::<&mut Transform, With<NearBody>>();
    for mut transform in bodies.iter_mut(world) {
        *transform = Transform::from_translation(translation);
    }
    let reasons = unsettled(world);
    publish_status_attribute("data-unsettled", &reasons.join(","));
    if let Some(refusal) = world.resource::<TreesWitness>().refusal.lock().unwrap().as_ref() {
        publish_status_attribute("data-tree-refusal", &format!("{}: {}", refusal.name, refusal.message));
    }
    let streak = if reasons.is_empty() { world.resource::<PoseDriver>().streak + 1 } else { 0 };
    world.resource_mut::<PoseDriver>().streak = streak;
    if streak == SETTLED_STREAK {
        let lod = world.resource::<PlanetLodStats>();
        publish_status_attribute("data-highest-level", &lod.highest_level.to_string());
        publish_status_attribute("data-live-patches", &lod.live_patches.to_string());
        publish_status_attribute("data-veg-chunks", &world.resource::<VegStreamStats>().resident_chunks.iter().sum::<usize>().to_string());
        publish_status_attribute("data-pose-settled", "true");
    }
}

fn spawn_planet_camera(mut commands: Commands) {
    commands.spawn((Camera3d::default(), Transform::default(), Name::new("planet-camera")));
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let ground = meshes.add(Plane3d::default().mesh().size(40.0, 40.0));
    let ground_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.11, 0.16, 0.19),
        perceptual_roughness: 0.9,
        ..default()
    });
    commands.spawn((
        Mesh3d(ground),
        MeshMaterial3d(ground_material),
        Transform::from_xyz(0.0, -0.75, 0.0),
        Name::new("wasm-ground"),
    ));

    let cube = meshes.add(Cuboid::new(1.6, 1.6, 1.6));
    let sphere = meshes.add(Sphere::new(0.9));
    let colors = [
        Color::srgb(0.22, 0.65, 0.98),
        Color::srgb(0.98, 0.52, 0.24),
        Color::srgb(0.42, 0.86, 0.50),
        Color::srgb(0.79, 0.42, 0.92),
    ];
    for (index, (mesh, x, z)) in [(cube, -2.4, 0.0), (sphere, 0.0, -1.4)]
        .into_iter()
        .enumerate()
    {
        let material = materials.add(StandardMaterial {
            base_color: colors[index % colors.len()],
            metallic: 0.15,
            perceptual_roughness: 0.35,
            ..default()
        });
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_xyz(x, 0.2, z),
            Spinner,
            Name::new(format!("wasm-mesh-{index}")),
        ));
    }

    commands.spawn((
        DirectionalLight {
            illuminance: 2800.0,
            shadow_maps_enabled: false,
            contact_shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.7, 0.5, 0.0)),
        Name::new("wasm-sun"),
    ));
    commands.spawn((
        Camera3d::default(),
        Camera { clear_color: ClearColorConfig::Default, ..default() },
        Transform::from_xyz(0.0, 5.2, 11.0).looking_at(Vec3::NEG_Y * 0.1, Vec3::Y),
        OrbitCamera,
        Name::new("wasm-camera"),
    ));
    commands.spawn((
        Text::new("Bevy 0.20 WebGPU WASM\nreal Bevy render / PBR / UI\nI toggles inspector"),
        TextFont { font_size: FontSize::Px(22.0), ..default() },
        TextColor(Color::srgb(0.93, 0.97, 1.0)),
        Node { position_type: PositionType::Absolute, left: Val::Px(18.0), top: Val::Px(16.0), ..default() },
        Name::new("wasm-overlay"),
    ));
}

fn model_request() -> Option<String> {
    let search = web_sys::window()?.location().search().ok()?;
    let model = web_sys::UrlSearchParams::new_with_str(&search).ok()?.get("model")?;
    let path = model.trim_start_matches('/').to_string();
    (!path.is_empty() && !path.split('/').any(|part| part == "..")).then_some(path)
}

fn query_value(name: &str) -> Option<String> {
    let search = web_sys::window()?.location().search().ok()?;
    web_sys::UrlSearchParams::new_with_str(&search).ok()?.get(name)
}

fn device_request(disable_bc: bool, disable_astc: bool) -> RenderCreation {
    let mut disabled = WgpuFeatures::empty();
    if disable_bc {
        disabled |= WgpuFeatures::TEXTURE_COMPRESSION_BC;
    }
    if disable_astc {
        disabled |= WgpuFeatures::TEXTURE_COMPRESSION_ASTC;
    }
    RenderCreation::Automatic(Box::new(WgpuSettings {
        disabled_features: (!disabled.is_empty()).then_some(disabled),
        ..default()
    }))
}

fn texture_target(support: CompressedImageFormats) -> (&'static str, &'static str) {
    if support.contains(CompressedImageFormats::BC) {
        ("bc7", BC7_CACHE)
    } else if support.contains(CompressedImageFormats::ASTC_LDR) {
        ("astc4x4", ASTC_CACHE)
    } else if support.contains(CompressedImageFormats::ETC2) {
        ("etc2", ETC2_CACHE)
    } else {
        ("rgba8", RGBA8_CACHE)
    }
}

fn compressed_names(support: CompressedImageFormats) -> String {
    let names: Vec<&str> = [(CompressedImageFormats::BC, "bc"), (CompressedImageFormats::ASTC_LDR, "astc"), (CompressedImageFormats::ETC2, "etc2")]
        .into_iter()
        .filter(|(flag, _)| support.contains(*flag))
        .map(|(_, name)| name)
        .collect();
    if names.is_empty() { "none".to_string() } else { names.join(" ") }
}

fn texture_choice(path: &str, support: CompressedImageFormats) -> (String, &'static str) {
    match path.strip_prefix(BC7_CACHE) {
        Some(rest) => {
            let (format, cache) = texture_target(support);
            (format!("{cache}{rest}"), format)
        }
        None => (path.to_string(), "source"),
    }
}

fn spawn_model(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut request: ResMut<ModelRequest>,
    support: Option<Res<CompressedImageFormatSupport>>,
    adapter: Option<Res<RenderAdapterInfo>>,
    mut spawned: Local<bool>,
) {
    if *spawned {
        return;
    }
    let Some(support) = support else {
        return;
    };
    *spawned = true;
    let Some(requested) = request.0.clone() else {
        return;
    };
    let bc_supported = support.0.contains(CompressedImageFormats::BC);
    let (path, format) = texture_choice(&requested, support.0);
    publish_status_attribute("data-bc-supported", if bc_supported { "true" } else { "false" });
    publish_status_attribute("data-compressed-formats", &compressed_names(support.0));
    if let Some(adapter) = adapter {
        publish_status_attribute("data-adapter", &adapter.name);
    }
    publish_status_attribute("data-texture-format", format);
    publish_status_attribute("data-model-path", &path);
    let handle = asset_server.load::<WorldAsset>(format!("{path}#Scene0"));
    commands.spawn((
        WorldAssetRoot(handle.clone()),
        Transform::from_xyz(0.0, 1.0, 3.0).with_scale(Vec3::splat(3.0)),
        Name::new("wasm-model"),
    ));
    commands.insert_resource(ModelHandle(handle));
    request.0 = Some(path);
}

fn publish_status_attribute(name: &str, value: &str) {
    if let Some(element) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id("app-status"))
    {
        let _ = element.set_attribute(name, value);
    }
}

fn report_model(
    asset_server: Res<AssetServer>,
    model: Option<Res<ModelHandle>>,
    request: Res<ModelRequest>,
    mut reported: ResMut<ModelReported>,
) {
    let Some(model) = model else {
        return;
    };
    if reported.0 {
        return;
    }
    let id = model.0.id();
    let path = request.0.clone().unwrap_or_default();
    match asset_server.load_state(id) {
        LoadState::Failed(error) => {
            reported.0 = true;
            publish_model_state("failed", &format!("{path}: {error}"));
        }
        _ if asset_server.is_loaded_with_dependencies(id) => {
            reported.0 = true;
            publish_model_state("loaded", &path);
        }
        _ => {}
    }
}

fn publish_model_state(state: &str, detail: &str) {
    let window = web_sys::window().expect("window");
    if let Some(element) = window.document().expect("document").get_element_by_id("app-status") {
        let _ = element.set_attribute("data-model-state", state);
    }
    let line = JsValue::from_str(&format!("bevy-jayzee model {state}: {detail}"));
    if state == "loaded" {
        web_sys::console::log_1(&line);
    } else {
        web_sys::console::error_1(&line);
    }
}

fn clock(time: &Time, still: &Still) -> f32 {
    if still.0 { 0.0 } else { time.elapsed_secs() }
}

fn orbit(time: Res<Time>, still: Res<Still>, mut cameras: Query<&mut Transform, With<OrbitCamera>>) {
    let angle = clock(&time, &still) * 0.18;
    for mut transform in &mut cameras {
        *transform = Transform::from_xyz(angle.sin() * 11.0, 5.2, angle.cos() * 11.0)
            .looking_at(Vec3::NEG_Y * 0.1, Vec3::Y);
    }
}

fn spin(time: Res<Time>, still: Res<Still>, mut meshes: Query<&mut Transform, With<Spinner>>) {
    for mut transform in &mut meshes {
        transform.rotation = Quat::from_rotation_y(clock(&time, &still) * 0.9);
    }
}

fn mark_ready(
    time: Res<Time>,
    mut sent: ResMut<ReadySent>,
    cameras: Query<Entity, With<Camera3d>>,
    windows: Query<Entity, With<PrimaryWindow>>,
) {
    if sent.0 || cameras.is_empty() || windows.is_empty() || time.elapsed_secs() < 0.05 {
        return;
    }
    sent.0 = true;
    let window = web_sys::window().expect("window");
    let document = window.document().expect("document");
    if let Some(element) = document.get_element_by_id("app-status") {
        let _ = element.set_attribute("data-app-ready", "true");
        element.set_text_content(Some("Bevy WebGPU WASM initialized"));
    }
    let _ = js_sys::Reflect::set(&window, &JsValue::from_str("__BEVY_WASM_READY"), &JsValue::TRUE);
    web_sys::console::log_1(&JsValue::from_str("bevy-jayzee WebGPU WASM app-ready"));
}

fn publish_diagnostics(
    time: Res<Time>,
    mut state: ResMut<InspectorState>,
    diagnostics: Res<DiagnosticsStore>,
    frame_stats: Res<SharedFrameStats>,
    mesh_assets: Res<Assets<Mesh>>,
    image_assets: Res<Assets<Image>>,
    material_assets: Res<Assets<StandardMaterial>>,
    cameras: Query<Entity, With<Camera3d>>,
    lights: Query<Entity, With<DirectionalLight>>,
    meshes: Query<Entity, With<Mesh3d>>,
    sprites: Query<Entity, With<Sprite>>,
    ui_nodes: Query<Entity, With<Node>>,
) {
    let enabled = js_sys::Reflect::get(
        &web_sys::window().expect("window"),
        &JsValue::from_str("__BEVY_INSPECTOR_ENABLED"),
    )
    .ok()
    .and_then(|value| value.as_bool())
    .unwrap_or(state.enabled);
    state.enabled = enabled;
    if !enabled || time.elapsed_secs_f64() - state.last_update < 0.12 {
        return;
    }
    state.last_update = time.elapsed_secs_f64();
    let stats = frame_stats.snapshot();
    let fps_value = diagnostic_value(&diagnostics, &FrameTimeDiagnosticsPlugin::FPS);
    let frame_time_value = diagnostic_value(&diagnostics, &FrameTimeDiagnosticsPlugin::FRAME_TIME);
    let entity_value = diagnostic_value(&diagnostics, &EntityCountDiagnosticsPlugin::ENTITY_COUNT);
    let gpu_passes = diagnostics
        .iter()
        .filter(|diagnostic| is_gpu_pass(diagnostic.path().as_str()))
        .filter_map(|diagnostic| diagnostic.value())
        .collect::<Vec<f64>>();
    let gpu_ms = (!gpu_passes.is_empty()).then(|| gpu_passes.iter().sum::<f64>());
    let gpu_reason = match gpu_ms {
        Some(_) => None,
        None if stats.gpu_timing_missing.is_empty() => Some("RenderDiagnosticsPlugin recorded no elapsed_gpu spans".to_string()),
        None => Some(format!("device lacks {}; device features: {}", stats.gpu_timing_missing, stats.device_features)),
    };
    let fps = diagnostic_json(&diagnostics, &FrameTimeDiagnosticsPlugin::FPS, "fps", "fps");
    let frame_time = diagnostic_json(&diagnostics, &FrameTimeDiagnosticsPlugin::FRAME_TIME, "frame_time", "ms");
    let frame_count = diagnostic_json(&diagnostics, &FrameTimeDiagnosticsPlugin::FRAME_COUNT, "frame_count", "");
    let entities = diagnostic_json(&diagnostics, &EntityCountDiagnosticsPlugin::ENTITY_COUNT, "entity_count", "");
    let mut render = String::from("[");
    let mut first = true;
    for diagnostic in diagnostics.iter() {
        if !diagnostic.path().as_str().starts_with("render/") {
            continue;
        }
        if !first { render.push(','); }
        first = false;
        render.push_str(&format!(
            "{{\"path\":\"{}\",\"value\":{},\"avg\":{},\"history\":{}}}",
            escape(diagnostic.path().as_str()),
            number(diagnostic.value()),
            number(diagnostic.average()),
            history_json(diagnostic)
        ));
    }
    render.push(']');
    let memory_js_bytes = js_heap_bytes();
    let wasm_bytes = wasm_memory_bytes();
    let mesh_count = mesh_assets.len();
    let image_count = image_assets.len();
    let image_bytes = image_assets.iter().map(|(_, image)| image.data.as_ref().map_or(0, |data| data.len())).sum::<usize>();
    let entries = vec![
        entry("FPS", "strip", fps_value, "fps", 144.0, "FrameTimeDiagnosticsPlugin/fps", Some(NO_DIAGNOSTIC.into())),
        entry("MEMQ", "strip", Some(mib(wasm_bytes)), "MiB", 0.0, "WebAssembly.Memory buffer byteLength", None),
        entry("MEMO", "strip", memory_js_bytes.map(mib), "MiB", 0.0, "performance.memory.usedJSHeapSize", Some("performance.memory is Chromium-only; this browser exposes no JS heap counter".into())),
        entry("DRAW", "strip", stats.draw_ms, "ms", 16.7, "render schedule timer around RenderSystems::Render (encode, submit, present)", Some(NO_FRAME.into())),
        entry("SHOW", "strip", None, "ms", 16.7, "present_frames", Some(SHOW_REASON.into())),
        entry("SIM", "strip", stats.sim_ms, "ms", 16.7, "First to Last schedule timer", Some(NO_FRAME.into())),
        entry("LAUNCH", "strip", stats.launch_ms, "ms", 16.7, "Last end to Render start: extract, plus any wait for the previous render frame", Some(NO_FRAME.into())),
        entry("DRAWS", "metric", Some(stats.main.draws as f64), "", 0.0, "RenderVisibleEntities<Mesh3d> of the main views, render world; batching not counted", None),
        entry("SHADOW-MAP DRAWS", "metric", Some(stats.shadow.draws as f64), "", 0.0, "RenderVisibleEntities<Mesh3d> of ViewLightEntities shadow views", None),
        entry("TRIANGLES", "metric", Some(stats.main.triangles as f64), "", 0.0, "RenderMeshInstances with RenderAssets<RenderMesh> index counts / 3, main views", None),
        entry("SHADOW TRIS", "metric", Some(stats.shadow.triangles as f64), "", 0.0, "RenderMeshInstances with RenderAssets<RenderMesh> index counts / 3, shadow views", None),
        entry("SHADOW PASSES", "metric", Some(stats.shadow_passes as f64), "", 0.0, "ViewLightEntities shadow views of the camera views", None),
        entry("ENTITIES", "metric", entity_value, "", 0.0, "EntityCountDiagnosticsPlugin/entity_count", Some(NO_DIAGNOSTIC.into())),
        entry("MESHES", "metric", Some(mesh_count as f64), "", 0.0, "Assets<Mesh>::len", None),
        entry("SPRITES", "metric", Some(sprites.iter().count() as f64), "", 0.0, "Query<Sprite>", None),
        entry("TEXTURES", "metric", Some(image_count as f64), "", 0.0, "Assets<Image>::len", None),
        entry("TEXTURE BYTES", "metric", Some(image_bytes as f64), "B", 0.0, "Assets<Image> CPU data length; GPU texture memory is not readable from Bevy", None),
        entry("frame interval", "bar", frame_time_value, "ms", 16.7, "FrameTimeDiagnosticsPlugin/frame_time", Some(NO_DIAGNOSTIC.into())),
        entry("CPU total", "bar", stats.cpu_ms, "ms", 16.7, "First start to Render end", Some(NO_FRAME.into())),
        entry("SIM", "bar", stats.sim_ms, "ms", 16.7, "First to Last schedule timer", Some(NO_FRAME.into())),
        entry("LAUNCH", "bar", stats.launch_ms, "ms", 16.7, "Last end to Render start: extract, plus any wait for the previous render frame", Some(NO_FRAME.into())),
        entry("DRAW", "bar", stats.draw_ms, "ms", 16.7, "render schedule timer around RenderSystems::Render (encode, submit, present)", Some(NO_FRAME.into())),
        entry("SHOW", "bar", None, "ms", 16.7, "present_frames", Some(SHOW_REASON.into())),
        entry("GPU", "bar", gpu_ms, "ms", 16.7, "RenderDiagnosticsPlugin elapsed_gpu spans, summed", gpu_reason.clone()),
    ];
    let entries_json = entries.iter().map(shown_json).collect::<Vec<_>>().join(",");
    let gpu_reason_json = gpu_reason.as_ref().map_or_else(|| "null".to_string(), |reason| format!("\"{}\"", escape(reason)));
    let json = format!(
        "{{\"kind\":\"bevy-diagnostics\",\"source\":\"DiagnosticsStore\",\"time\":{},\"plugins\":[\"FrameTimeDiagnosticsPlugin\",\"EntityCountDiagnosticsPlugin\",\"LogDiagnosticsPlugin\",\"RenderDiagnosticsPlugin\",\"RenderStatsPlugin\",\"DiagnosticsPlugin/DefaultPlugins\"],\"diagnostics\":[{},{},{},{}],\"renderDiagnostics\":{},\"shown\":[{}],\"gpu\":{{\"timestampQueries\":{},\"deviceFeatures\":\"{}\",\"reason\":{}}},\"memory\":{{\"wasmBytes\":{},\"jsHeapBytes\":{}}},\"ecs\":{{\"cameras\":{},\"lights\":{},\"meshEntities\":{},\"sprites\":{},\"uiNodes\":{}}},\"assets\":{{\"meshes\":{},\"images\":{},\"imageBytes\":{},\"standardMaterials\":{}}},\"unsupported\":[{}]}}",
        js_sys::Date::now(),
        fps,
        frame_time,
        frame_count,
        entities,
        render,
        entries_json,
        stats.timestamp_queries,
        escape(&stats.device_features),
        gpu_reason_json,
        number(Some(wasm_bytes)),
        number(memory_js_bytes),
        cameras.iter().count(),
        lights.iter().count(),
        meshes.iter().count(),
        sprites.iter().count(),
        ui_nodes.iter().count(),
        mesh_count,
        image_count,
        image_bytes,
        material_assets.len(),
        UNSUPPORTED
    );
    let window = web_sys::window().expect("window");
    let _ = js_sys::Reflect::set(&window, &JsValue::from_str("__BEVY_INSPECTOR_SNAPSHOT"), &JsValue::from_str(&json));
    if let Some(element) = window.document().expect("document").get_element_by_id("inspector-data") {
        element.set_text_content(Some(&json));
    }
}

const NO_FRAME: &str = "no completed frame has been timed yet";
const NO_DIAGNOSTIC: &str = "diagnostic is not registered";
const SHOW_REASON: &str = "present_frames runs inside RenderSystems::Render with no separate hook in Bevy 0.20.1; its time is counted in DRAW";
const UNSUPPORTED: &str = r#"{"name":"frameGraph","reason":"Bevy 0.20 exposes no browser-readable frame graph"},{"name":"physics","reason":"Avian is native-only in this build"},{"name":"multiplayer","reason":"the wasm viewer excludes Lightyear networking"},{"name":"jobs","reason":"no job diagnostics are registered in the wasm scene"},{"name":"vibes","reason":"native feature state is not linked into the wasm scene"},{"name":"cascadeSkips","reason":"Bevy 0.20 exposes no cascade-skip counter; the metric was removed"},{"name":"terrain","reason":"the wasm scene has no terrain plugin; the metric was removed"}"#;

struct Shown {
    name: &'static str,
    group: &'static str,
    value: Option<f64>,
    unit: &'static str,
    max: f64,
    source: &'static str,
    reason: Option<String>,
}

fn entry(name: &'static str, group: &'static str, value: Option<f64>, unit: &'static str, max: f64, source: &'static str, reason: Option<String>) -> Shown {
    let reason = match value {
        Some(_) => None,
        None => Some(reason.unwrap_or_else(|| "no source value is available".to_string())),
    };
    Shown { name, group, value, unit, max, source, reason }
}

fn shown_json(entry: &Shown) -> String {
    let reason = entry.reason.as_ref().map_or_else(|| "null".to_string(), |reason| format!("\"{}\"", escape(reason)));
    format!(
        "{{\"name\":\"{}\",\"group\":\"{}\",\"value\":{},\"unit\":\"{}\",\"max\":{},\"source\":\"{}\",\"reason\":{}}}",
        entry.name, entry.group, number(entry.value), entry.unit, entry.max, escape(entry.source), reason
    )
}

fn diagnostic_value(store: &DiagnosticsStore, path: &bevy::diagnostic::DiagnosticPath) -> Option<f64> {
    store.get(path).and_then(|diagnostic| diagnostic.value())
}

fn mib(bytes: f64) -> f64 {
    bytes / 1048576.0
}

fn js_heap_bytes() -> Option<f64> {
    let window: JsValue = web_sys::window()?.into();
    let performance = js_sys::Reflect::get(&window, &JsValue::from_str("performance")).ok()?;
    let memory = js_sys::Reflect::get(&performance, &JsValue::from_str("memory")).ok()?;
    js_sys::Reflect::get(&memory, &JsValue::from_str("usedJSHeapSize")).ok()?.as_f64()
}

fn wasm_memory_bytes() -> f64 {
    let buffer = js_sys::WebAssembly::Memory::from(wasm_bindgen::memory()).buffer();
    f64::from(js_sys::ArrayBuffer::from(buffer).byte_length())
}

fn diagnostic_json(store: &DiagnosticsStore, path: &bevy::diagnostic::DiagnosticPath, name: &str, suffix: &str) -> String {
    match store.get(path) {
        Some(diagnostic) => format!(
            "{{\"name\":\"{}\",\"path\":\"{}\",\"suffix\":\"{}\",\"value\":{},\"avg\":{},\"smoothed\":{},\"history\":{}}}",
            name, escape(path.as_str()), suffix,
            number(diagnostic.value()), number(diagnostic.average()), number(diagnostic.smoothed()), history_json(diagnostic)
        ),
        None => format!("{{\"name\":\"{}\",\"value\":null,\"state\":\"unavailable\"}}", name),
    }
}

fn history_json(diagnostic: &bevy::diagnostic::Diagnostic) -> String {
    let values = diagnostic.values().copied().collect::<Vec<_>>();
    let start = values.len().saturating_sub(64);
    let mut out = String::from("[");
    for (index, value) in values[start..].iter().enumerate() {
        if index > 0 { out.push(','); }
        out.push_str(&format!("{:.3}", value));
    }
    out.push(']');
    out
}

fn number(value: Option<f64>) -> String {
    value.map(|v| format!("{:.3}", v)).unwrap_or_else(|| "null".to_string())
}

fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
