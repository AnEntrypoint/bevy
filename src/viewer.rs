use bevy::diagnostic::{DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::render::settings::{RenderCreation, WgpuFeatures, WgpuSettings};
use bevy::render::RenderPlugin;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowResolution};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::spec::ObjectSpec;
use crate::tree::WorldTree;
use crate::ui_html::{overlapping_pairs, uncovered_glyph_count, HtmlUiPlugin, HtmlUiSource, HtmlUiStatus, HtmlAnimationStatus, HtmlTranslated, WorldAnchor};
use crate::{script, viewer_world, world};

#[derive(Resource, Clone, Debug)]
pub struct ViewerScene {
    pub title: String,
    pub directory: String,
    pub place: String,
    pub objects: usize,
    pub meshes: usize,
    pub mesh_requests: usize,
    pub shapes_created: usize,
    pub mesh_entities: usize,
    pub unique_materials: usize,
    pub batch_keys: usize,
    pub mirror_entities: usize,
    pub mirror_batch_keys: usize,
    pub object_draws: usize,
    pub object_batch_keys: usize,
    pub object_kinds: BTreeMap<String, usize>,
    pub expected_objects: usize,
    pub expected_ids: Vec<String>,
    pub duplicate_objects: usize,
    pub missing_objects: Vec<String>,
    pub placeholders: usize,
    pub placeholders_retired: usize,
    pub unmirrored: usize,
    pub unmirrored_models: usize,
    pub marker_ids: Vec<String>,
    pub models_loaded: usize,
    pub texture_target: &'static str,
    pub bc_supported: bool,
    pub models_missing: Vec<String>,
    pub proxies: usize,
    pub markers: usize,
    pub models_requested: Vec<String>,
    pub terrains: usize,
    pub lights: usize,
    pub camera: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnchorPlacement {
    Placed(Vec2),
    Unresolved,
    Behind,
    Offscreen,
}

pub fn classify_anchor(view_depth: f32, point: Option<Vec2>, viewport: Vec2) -> AnchorPlacement {
    match point {
        _ if view_depth >= 0.0 => AnchorPlacement::Behind,
        None => AnchorPlacement::Behind,
        Some(point) if (0.0..=viewport.x).contains(&point.x) && (0.0..=viewport.y).contains(&point.y) => AnchorPlacement::Placed(point),
        Some(_) => AnchorPlacement::Offscreen,
    }
}

#[derive(Resource, Clone, Debug, Default)]
pub struct AnchorReport {
    pub total: usize,
    pub placed: Vec<(String, Vec2)>,
    pub unresolved: Vec<String>,
    pub behind: usize,
    pub offscreen: usize,
    pub viewport: Vec2,
    pub world: Vec<(String, Vec3)>,
}

fn place_world_anchors(
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    objects: Query<(&ViewerObject, &GlobalTransform)>,
    mut anchors: Query<(&WorldAnchor, &mut Node, &mut Visibility)>,
    mut report: ResMut<AnchorReport>,
) {
    let Some((camera, camera_transform)) = cameras.iter().find(|(camera, _)| camera.is_active) else {
        return;
    };
    let viewport = camera.logical_viewport_size().unwrap_or(Vec2::ZERO);
    let positions = objects
        .iter()
        .map(|(object, transform)| (object.id.as_str(), transform.translation()))
        .collect::<HashMap<_, _>>();
    let view = camera_transform.affine().inverse();
    let mut next = AnchorReport { viewport, ..default() };
    for (anchor, mut node, mut visibility) in &mut anchors {
        next.total += 1;
        let placement = match positions.get(anchor.id.as_str()) {
            None => AnchorPlacement::Unresolved,
            Some(position) => {
                let world = *position + anchor.offset;
                next.world.push((anchor.id.clone(), world));
                classify_anchor(view.transform_point3(world).z, camera.world_to_viewport(camera_transform, world).ok(), viewport)
            }
        };
        match placement {
            AnchorPlacement::Placed(point) => {
                node.left = Val::Px(point.x);
                node.top = Val::Px(point.y);
                *visibility = Visibility::Inherited;
                next.placed.push((anchor.id.clone(), point));
            }
            other => {
                *visibility = Visibility::Hidden;
                match other {
                    AnchorPlacement::Unresolved => next.unresolved.push(anchor.id.clone()),
                    AnchorPlacement::Behind => next.behind += 1,
                    _ => next.offscreen += 1,
                }
            }
        }
    }
    *report = next;
}

#[derive(Resource, Clone, Debug, Default)]
pub struct ViewerBrowserReport {
    pub path: String,
}

#[derive(Component, Clone, Debug)]
pub struct ViewerObject {
    pub id: String,
    pub kind: String,
}

#[derive(Component, Clone, Debug)]
pub struct ViewerPlaceholder(pub String);

pub fn object_id(object: &ObjectSpec, index: usize) -> String {
    object.id.clone().unwrap_or_else(|| format!("object-{index}"))
}

pub fn run(directory: &str, place: Option<&str>) -> Result<(), String> {
    let tree = WorldTree::load(Path::new(directory))?;
    let name = place
        .map(str::to_string)
        .or(tree.config.default_place.clone())
        .or_else(|| tree.places.keys().next().cloned())
        .ok_or_else(|| format!("{directory}: no places"))?;
    let place_def = tree
        .places
        .get(&name)
        .cloned()
        .ok_or_else(|| format!("{directory}: no place {name}"))?;
    let runner = world_runner(directory, &name, &tree)?;
    let title = format!("bevy-jayzee viewer - {name}");
    let mut app = App::new();
    let cache = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/viewer-cache");
    app.add_plugins(DefaultPlugins
        .set(WindowPlugin {
            primary_window: Some(Window {
                title: title.clone(),
                resolution: WindowResolution::new(1280, 720),
                ..default()
            }),
            ..default()
        })
        .set(AssetPlugin { file_path: cache.display().to_string(), ..default() })
        .set(RenderPlugin {
            render_creation: device_request(std::env::var("VIEWER_DISABLE_BC").is_ok_and(|value| value == "1"), std::env::var("VIEWER_DISABLE_ASTC").is_ok_and(|value| value == "1")),
            ..default()
        }))
    .add_plugins((FrameTimeDiagnosticsPlugin::default(), EntityCountDiagnosticsPlugin::default(), LogDiagnosticsPlugin::default(), bevy::render::diagnostic::RenderDiagnosticsPlugin, crate::render_stats::RenderStatsPlugin))
    .insert_resource(ClearColor(Color::srgb(0.035, 0.045, 0.065)))
    .insert_resource(ViewerScene {
        title,
        directory: directory.to_string(),
        place: name.clone(),
        objects: place_def.objects.len(),
        meshes: 0,
        mesh_requests: 0,
        shapes_created: 0,
        mesh_entities: 0,
        unique_materials: 0,
        batch_keys: 0,
        mirror_entities: 0,
        mirror_batch_keys: 0,
        object_draws: 0,
        object_batch_keys: 0,
        object_kinds: BTreeMap::new(),
        expected_objects: 0,
        expected_ids: Vec::new(),
        duplicate_objects: 0,
        missing_objects: Vec::new(),
        placeholders: 0,
        placeholders_retired: 0,
        unmirrored: 0,
        unmirrored_models: 0,
        marker_ids: Vec::new(),
        models_loaded: 0,
        texture_target: "",
        bc_supported: false,
        models_missing: Vec::new(),
        proxies: 0,
        markers: 0,
        models_requested: Vec::new(),
        terrains: 0,
        lights: 1,
        camera: Vec3::new(12.0, 10.0, 18.0),
    })
    .insert_resource(ViewerObjects(place_def.objects))
    .insert_resource(ViewerCache(cache))
    .insert_resource(ViewerBrowserReport { path: browser_report_path(directory, place) })
    .init_resource::<HtmlUiSource>()
    .add_plugins(HtmlUiPlugin)
    .add_systems(Startup, spawn_viewer)
    .init_resource::<AnchorReport>()
    .add_systems(Update, (update_overlay, primary_window_title, place_world_anchors, write_browser_report.after(place_world_anchors)));
    if let Some(globe) = place_def.config.extra.get("globe") {
        app.insert_resource(crate::globe::GlobeWorld::from_value(Some(globe)));
        crate::globe::register(&mut app);
        app.add_systems(Startup, crate::vegetation::prepare);
        app.add_systems(Update, crate::globe::orbit_globe_camera);
    } else {
        app.add_systems(Update, orbit_camera);
    }
    if let Some(runner) = runner {
        app.insert_non_send(runner).add_systems(Update, (tick_world, viewer_world::mirror_world, viewer_world::retire_placeholders, viewer_world::count_batches, feed_ui_html).chain());
    } else {
        app.add_systems(Update, (viewer_world::retire_placeholders, viewer_world::count_batches).chain());
    }
    app.run();
    crate::script::shutdown();
    Ok(())
}

pub struct WorldRunner(pub App);

#[derive(Resource, Clone, Debug)]
pub struct ViewerCache(pub PathBuf);

fn world_runner(directory: &str, place: &str, tree: &WorldTree) -> Result<Option<WorldRunner>, String> {
    let has_render = tree.config.ui.get("render").is_some_and(|render| render.is_string());
    if !has_render {
        return Ok(None);
    }
    let (app, _) = world::prepare(directory, Some(place))?;
    Ok(Some(WorldRunner(app)))
}

fn tick_world(mut runner: NonSendMut<WorldRunner>) {
    runner.0.update();
}

fn feed_ui_html(mut source: ResMut<HtmlUiSource>) {
    source.update(&script::last_ui_html());
}

#[derive(Component)]
struct ViewerOverlay;

#[derive(Component)]
struct ViewerStatsOverlay;

#[derive(Resource, Clone, Debug, Default)]
pub struct ViewerObjects(pub Vec<ObjectSpec>);

fn spawn_viewer(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    objects: Res<ViewerObjects>,
    mut scene: ResMut<ViewerScene>,
    globe: Option<Res<crate::globe::GlobeWorld>>,
) {
    if globe.is_none() {
        let ground_mesh = meshes.add(Plane3d::default().mesh().size(80.0, 80.0));
        let ground_material = materials.add(StandardMaterial { base_color: Color::srgb(0.12, 0.16, 0.18), ..default() });
        commands.spawn((Mesh3d(ground_mesh), MeshMaterial3d(ground_material), Transform::from_xyz(0.0, -0.55, 0.0), Name::new("viewer-ground")));
    }
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let sphere = meshes.add(Sphere::new(0.5));
    let colors = [
        Color::srgb(0.24, 0.62, 0.95),
        Color::srgb(0.95, 0.55, 0.25),
        Color::srgb(0.42, 0.82, 0.45),
        Color::srgb(0.78, 0.42, 0.88),
    ];
    let palette: Vec<Handle<StandardMaterial>> = colors.iter().map(|color| materials.add(StandardMaterial { base_color: *color, ..default() })).collect();
    let mut expected: Vec<String> = objects.0.iter().enumerate().map(|(index, object)| object_id(object, index)).collect();
    expected.sort();
    expected.dedup();
    scene.expected_ids = expected;
    let mut spawned_ids = HashSet::new();
    for (index, object) in objects.0.iter().enumerate() {
        let id = object_id(object, index);
        if !spawned_ids.insert(id.clone()) {
            continue;
        }
        let position = object.properties.feet_position.as_ref().map(|value| crate::spec::vec3(Some(value))).unwrap_or(Vec3::ZERO);
        let kind = object.properties.primitive.as_ref().and_then(|value| value.get("kind")).and_then(|value| value.as_str()).unwrap_or("entity").to_string();
        let (mesh, size) = if kind.contains("sphere") || kind.contains("ball") {
            (sphere.clone(), Vec3::splat(1.0))
        } else {
            (cube.clone(), Vec3::new(1.2, 1.2, 1.2))
        };
        let material = palette[viewer_world::palette_slot(&kind, palette.len())].clone();
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(position + Vec3::Y * 0.6).with_scale(size),
            ViewerObject { id: id.clone(), kind },
            ViewerPlaceholder(id.clone()),
            Name::new(id),
        ));
    }
    commands.spawn((
        DirectionalLight { illuminance: 2600.0, shadow_maps_enabled: false, contact_shadows_enabled: false, ..default() },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.7, 0.4, 0.0)),
        Name::new("viewer-sun"),
    ));
    commands.spawn((
        Camera3d::default(),
        Camera { clear_color: ClearColorConfig::Default, ..default() },
        Transform::from_xyz(scene.camera.x, scene.camera.y, scene.camera.z).looking_at(Vec3::ZERO, Vec3::Y),
        Name::new("viewer-camera"),
    ));
    commands.spawn((
        Text::new("bevy-jayzee native viewer\nloading world objects..."),
        TextFont { font_size: FontSize::Px(18.0), ..default() },
        TextColor(Color::srgb(0.910, 0.945, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(18.0),
            bottom: Val::Px(16.0),
            max_width: Val::Percent(65.0),
            ..default()
        },
        ViewerOverlay,
        Name::new("viewer-overlay"),
    ));
    commands.spawn((
        Text::new("render stats"),
        TextFont { font_size: FontSize::Px(14.0), ..default() },
        TextColor(Color::srgb(0.910, 0.945, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(18.0),
            bottom: Val::Px(200.0),
            max_width: Val::Percent(65.0),
            ..default()
        },
        ViewerStatsOverlay,
        Name::new("viewer-stats-overlay"),
    ));
}

fn orbit_camera(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<Camera3d>>,
    objects: Query<(&ViewerObject, &Transform), Without<Camera3d>>,
    mut scene: ResMut<ViewerScene>,
) {
    let focus_kind = std::env::var("VIEWER_FOCUS_KIND").unwrap_or_default();
    let focus = objects
        .iter()
        .find(|(object, _)| object.kind == focus_kind)
        .map(|(_, transform)| transform.translation)
        .unwrap_or(Vec3::ZERO);
    let angle = time.elapsed_secs() * 0.12;
    let eye = fixed_eye(std::env::var("VIEWER_EYE").ok().as_deref())
        .unwrap_or_else(|| focus + Vec3::new(angle.sin() * 18.0, 10.0, angle.cos() * 18.0));
    scene.camera = eye;
    for mut transform in &mut query {
        *transform = Transform::from_translation(eye).looking_at(focus, Vec3::Y);
    }
}

pub fn device_request(disable_bc: bool, disable_astc: bool) -> RenderCreation {
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

pub fn fixed_eye(value: Option<&str>) -> Option<Vec3> {
    let parts = value?.split(',').map(|part| part.trim().parse::<f32>().ok()).collect::<Option<Vec<_>>>()?;
    match parts.as_slice() {
        [x, y, z] => Some(Vec3::new(*x, *y, *z)),
        _ => None,
    }
}

fn ms(value: Option<f64>) -> String {
    value.map_or_else(|| "unsupported".to_string(), |value| format!("{value:.2}ms"))
}

fn process_rss_kib() -> Option<u64> {
    std::fs::read_to_string("/proc/self/statm")
        .ok()
        .and_then(|statm| statm.split_whitespace().nth(1).and_then(|pages| pages.parse::<u64>().ok()))
        .map(|pages| pages * 4)
}

fn process_rss_mib() -> String {
    process_rss_kib().map_or_else(|| "unsupported (no /proc/self/statm)".to_string(), |kib| format!("{:.1}MiB", kib as f64 / 1024.0))
}

fn update_overlay(
    scene: Res<ViewerScene>,
    diagnostics: Res<DiagnosticsStore>,
    frame_stats: Res<crate::render_stats::SharedFrameStats>,
    objects: Query<&ViewerObject>,
    mut text: Query<&mut Text, (With<ViewerOverlay>, Without<ViewerStatsOverlay>)>,
    mut stats_text: Query<&mut Text, (With<ViewerStatsOverlay>, Without<ViewerOverlay>)>,
) {
    let visible = objects.iter().map(|object| { let _ = object.kind.as_str(); 1 }).sum::<usize>();
    let fps = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS).and_then(|diagnostic| diagnostic.value());
    let frame_ms = diagnostics.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME).and_then(|diagnostic| diagnostic.value());
    let entities = diagnostics.get(&EntityCountDiagnosticsPlugin::ENTITY_COUNT).and_then(|diagnostic| diagnostic.value());
    let stats = frame_stats.snapshot();
    let gpu_passes = diagnostics
        .iter()
        .filter(|diagnostic| crate::render_stats::is_gpu_pass(diagnostic.path().as_str()))
        .filter_map(|diagnostic| diagnostic.value())
        .collect::<Vec<f64>>();
    let gpu = if gpu_passes.is_empty() {
        let reason = if stats.gpu_timing_missing.is_empty() { "enabled, no spans recorded".to_string() } else { format!("device lacks {}", stats.gpu_timing_missing) };
        format!("unsupported ({reason}; device features {})", stats.device_features)
    } else {
        format!("{:.3}ms over {} passes", gpu_passes.iter().sum::<f64>(), gpu_passes.len())
    };
    let value = format!(
        "{}  |  place={}  |  objects={}  |  proxy meshes={}  |  light={}  |  camera=({:.1},{:.1},{:.1})\nNative Bevy render/window/PBR/UI path | headless world API preserved\nBevy DiagnosticsStore | fps={} | frame={}ms | entities={} | I toggles browser inspector in wasm viewer",
        scene.title, scene.place, scene.objects, visible, scene.lights, scene.camera.x, scene.camera.y, scene.camera.z,
        fps.map(|value| format!("{value:.1}")).unwrap_or_else(|| "unavailable".to_string()),
        frame_ms.map(|value| format!("{value:.2}")).unwrap_or_else(|| "unavailable".to_string()),
        entities.map(|value| format!("{value:.0}")).unwrap_or_else(|| "unavailable".to_string()),
    );
    for mut text in &mut text {
        text.0 = value.clone();
    }
    let stats_value = format!(
        "Render world | draws={} | triangles={} | shadow draws={} | shadow tris={} | passes={}\nFrame phases | SIM={} | LAUNCH={} | DRAW={} | SHOW=unsupported | CPU total={}\nGPU passes | {} | RSS={}",
        stats.main.draws, stats.main.triangles, stats.shadow.draws, stats.shadow.triangles, stats.shadow_passes,
        ms(stats.sim_ms), ms(stats.launch_ms), ms(stats.draw_ms), ms(stats.cpu_ms),
        gpu, process_rss_mib()
    );
    for mut text in &mut stats_text {
        text.0 = stats_value.clone();
    }
}

fn primary_window_title(scene: Res<ViewerScene>, mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    for mut window in &mut windows {
        window.title = scene.title.clone();
    }
}

fn write_browser_report(scene: Res<ViewerScene>, objects: Query<&ViewerObject>, report: Res<ViewerBrowserReport>, ui: Res<HtmlUiStatus>, overlay: Query<&Text, With<ViewerOverlay>>, labels: Query<(&Text, &ComputedNode, &UiGlobalTransform, &InheritedVisibility)>, windows: Query<&Window, With<PrimaryWindow>>, anchors: Res<AnchorReport>, translated: Query<(&ComputedNode, &UiGlobalTransform, &InheritedVisibility), With<HtmlTranslated>>, animations: Res<HtmlAnimationStatus>, time: Res<Time>, diagnostics: Res<DiagnosticsStore>, images: Res<Assets<Image>>) {
    if report.path.is_empty() {
        return;
    }
    let mut ids = objects.iter().map(|object| object.id.clone()).collect::<Vec<_>>();
    ids.sort();
    let boxes = labels
        .iter()
        .filter(|(text, node, _, visible)| visible.get() && !text.0.is_empty() && node.size.x > 0.0 && node.size.y > 0.0)
        .map(|(text, node, transform, _)| {
            let center = transform.translation;
            let half = node.size * 0.5;
            (text.0.clone(), [center.x - half.x, center.y - half.y, center.x + half.x, center.y + half.y])
        })
        .collect::<Vec<_>>();
    let rects = boxes.iter().map(|(_, rect)| *rect).collect::<Vec<_>>();
    let overlaps = overlapping_pairs(&rects);
    let window_width = windows.single().map(|window| window.physical_width() as f32).unwrap_or(0.0);
    let full_width = rects.iter().filter(|rect| window_width > 0.0 && rect[2] - rect[0] >= window_width * 0.95).count();
    let label_boxes = boxes
        .iter()
        .take(60)
        .map(|(text, rect)| serde_json::json!({ "text": text.chars().take(40).collect::<String>(), "rect": rect }))
        .collect::<Vec<_>>();
    let overlap_pairs = overlaps
        .iter()
        .take(40)
        .map(|&(a, b)| serde_json::json!({ "a": boxes[a].0, "b": boxes[b].0, "aRect": boxes[a].1, "bRect": boxes[b].1 }))
        .collect::<Vec<_>>();
    let mut value = serde_json::json!({
        "kind": "viewer-status",
        "title": scene.title,
        "directory": scene.directory,
        "place": scene.place,
        "objects": scene.objects,
        "meshes": scene.meshes,
        "expectedObjects": scene.expected_objects,
        "duplicateObjects": scene.duplicate_objects,
        "missingObjects": scene.missing_objects,
        "placeholders": scene.placeholders,
        "placeholdersRetired": scene.placeholders_retired,
        "meshRequests": scene.mesh_requests,
        "shapesCreated": scene.shapes_created,
        "meshEntities": scene.mesh_entities,
        "uniqueMaterials": scene.unique_materials,
        "batchKeys": scene.batch_keys,
        "mirrorEntities": scene.mirror_entities,
        "mirrorBatchKeys": scene.mirror_batch_keys,
        "objectDraws": scene.object_draws,
        "objectBatchKeys": scene.object_batch_keys,
        "objectKinds": scene.object_kinds,
        "unmirroredObjects": scene.unmirrored,
        "unmirroredModels": scene.unmirrored_models,
        "markerIds": scene.marker_ids,
        "modelsLoaded": scene.models_loaded,
        "modelsMissing": scene.models_missing,
        "proxies": scene.proxies,
        "markers": scene.markers,
        "modelsRequested": scene.models_requested,
        "terrains": scene.terrains,
        "lights": scene.lights,
        "camera": [scene.camera.x, scene.camera.y, scene.camera.z],
        "visibleIds": ids,
        "uiNodes": ui.nodes,
        "uiWarnings": ui.warnings,
        "uiUncoveredGlyphs": ui.uncovered + overlay.iter().map(|text| uncovered_glyph_count(&text.0)).sum::<usize>(),
        "hud": overlay.iter().map(|text| text.0.clone()).collect::<Vec<_>>(),
        "uiLabels": rects.len(),
        "uiLabelOverlaps": overlaps.len(),
        "uiLabelOverlapPairs": overlap_pairs,
        "renderer": "bevy native window / winit / wgpu / pbr / ui",
    });
    value["textureTarget"] = serde_json::json!(scene.texture_target);
    value["bcSupported"] = serde_json::json!(scene.bc_supported);
    value["processRssKib"] = serde_json::json!(process_rss_kib());
    value["frameMsAverage"] = serde_json::json!(diagnostics.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME).and_then(|diagnostic| diagnostic.average()));
    value["fpsAverage"] = serde_json::json!(diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS).and_then(|diagnostic| diagnostic.average()));
    value["imageCount"] = serde_json::json!(images.iter().count());
    value["imageBytes"] = serde_json::json!(images.iter().map(|(_, image)| image.data.as_ref().map_or(0, Vec::len)).sum::<usize>());
    value["uiWindowWidth"] = serde_json::json!(window_width);
    value["uiFullWidthLabels"] = serde_json::json!(full_width);
    value["uiLabelBoxes"] = serde_json::json!(label_boxes);
    value["uiTranslatedBoxes"] = serde_json::json!(translated
        .iter()
        .filter(|(node, _, visible)| visible.get() && node.size.x > 0.0)
        .map(|(node, transform, _)| {
            let half = node.size * 0.5;
            [transform.translation.x - half.x, transform.translation.y - half.y, transform.translation.x + half.x, transform.translation.y + half.y]
        })
        .collect::<Vec<_>>());
    value["uiClock"] = serde_json::json!(time.elapsed_secs());
    value["uiAnimations"] = serde_json::json!(animations
        .samples
        .iter()
        .map(|sample| serde_json::json!({ "key": sample.key, "names": sample.names, "elapsed": sample.elapsed, "opacity": sample.opacity, "visible": sample.visible, "paused": sample.paused, "animationElapsed": sample.animation_elapsed, "transitionElapsed": sample.transition_elapsed, "transitionsStarted": sample.transitions_started, "reversedAt": sample.reversed_at, "color": sample.color, "background": sample.background, "transform": sample.transform }))
        .collect::<Vec<_>>());
    value["uiAnchorsInScene"] = serde_json::json!(ui.anchors);
    value["uiAnchorsSeen"] = serde_json::json!(anchors.total);
    value["uiAnchorsPlaced"] = serde_json::json!(anchors.placed.len());
    value["uiAnchorsUnresolved"] = serde_json::json!(anchors.unresolved);
    value["uiAnchorsBehind"] = serde_json::json!(anchors.behind);
    value["uiAnchorsOffscreen"] = serde_json::json!(anchors.offscreen);
    value["uiAnchorWorld"] = serde_json::json!(anchors.world.iter().map(|(id, world)| serde_json::json!({ "id": id, "world": [world.x, world.y, world.z] })).collect::<Vec<_>>());
    value["uiAnchorViewport"] = serde_json::json!([anchors.viewport.x, anchors.viewport.y]);
    value["uiAnchorPositions"] = serde_json::json!(anchors.placed.iter().map(|(id, point)| serde_json::json!({ "id": id, "x": point.x, "y": point.y })).collect::<Vec<_>>());
    let _ = std::fs::write(&report.path, serde_json::to_string_pretty(&value).unwrap_or_default());
}

pub fn browser_report_path(_directory: &str, place: Option<&str>) -> String {
    let name = place.unwrap_or("world");
    format!("web/status-{}.json", name.replace(['/', '\\'], "-"))
}

#[cfg(test)]
mod tests {
    use super::{AnchorPlacement, classify_anchor, device_request, fixed_eye};
    use bevy::prelude::{Vec2, Vec3};
    use bevy::render::settings::{RenderCreation, WgpuFeatures};

    #[test]
    fn disabling_each_compressed_format_removes_only_that_feature_from_the_device_request() {
        let requested = |disable_bc, disable_astc| match device_request(disable_bc, disable_astc) {
            RenderCreation::Automatic(settings) => settings.disabled_features,
            RenderCreation::Manual(_) => None,
        };
        assert_eq!(requested(true, false), Some(WgpuFeatures::TEXTURE_COMPRESSION_BC));
        assert_eq!(requested(false, true), Some(WgpuFeatures::TEXTURE_COMPRESSION_ASTC));
        assert_eq!(requested(true, true), Some(WgpuFeatures::TEXTURE_COMPRESSION_BC | WgpuFeatures::TEXTURE_COMPRESSION_ASTC));
        assert_eq!(requested(false, false), None);
    }

    #[test]
    fn anchors_are_placed_only_in_front_of_the_camera_and_inside_the_viewport() {
        let viewport = Vec2::new(800.0, 600.0);
        assert_eq!(classify_anchor(-5.0, Some(Vec2::new(10.0, 20.0)), viewport), AnchorPlacement::Placed(Vec2::new(10.0, 20.0)));
        assert_eq!(classify_anchor(-5.0, Some(Vec2::new(800.0, 600.0)), viewport), AnchorPlacement::Placed(Vec2::new(800.0, 600.0)));
        assert_eq!(classify_anchor(2.0, Some(Vec2::new(10.0, 20.0)), viewport), AnchorPlacement::Behind);
        assert_eq!(classify_anchor(-5.0, None, viewport), AnchorPlacement::Behind);
        assert_eq!(classify_anchor(-5.0, Some(Vec2::new(-1.0, 20.0)), viewport), AnchorPlacement::Offscreen);
        assert_eq!(classify_anchor(-5.0, Some(Vec2::new(10.0, 601.0)), viewport), AnchorPlacement::Offscreen);
    }

    #[test]
    fn fixed_eye_parses_three_numbers_only() {
        assert_eq!(fixed_eye(Some("0,300,1")), Some(Vec3::new(0.0, 300.0, 1.0)));
        assert_eq!(fixed_eye(Some(" 1.5, 2 ,-3")), Some(Vec3::new(1.5, 2.0, -3.0)));
        assert_eq!(fixed_eye(Some("1,2")), None);
        assert_eq!(fixed_eye(Some("a,b,c")), None);
        assert_eq!(fixed_eye(None), None);
    }
}
