use bevy::diagnostic::{DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowResolution};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use crate::spec::ObjectSpec;
use crate::tree::WorldTree;
use crate::ui_html::{overlapping_pairs, uncovered_glyph_count, HtmlUiPlugin, HtmlUiSource, HtmlUiStatus};
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
    pub models_missing: Vec<String>,
    pub proxies: usize,
    pub markers: usize,
    pub models_requested: Vec<String>,
    pub terrains: usize,
    pub lights: usize,
    pub camera: Vec3,
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
        .set(AssetPlugin { file_path: cache.display().to_string(), ..default() }))
    .add_plugins((FrameTimeDiagnosticsPlugin::default(), EntityCountDiagnosticsPlugin::default(), LogDiagnosticsPlugin::default()))
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
    .add_systems(Update, (update_overlay, primary_window_title, write_browser_report));
    if let Some(globe) = place_def.config.extra.get("globe") {
        app.insert_resource(crate::globe::GlobeWorld::from_value(Some(globe)));
        crate::globe::register(&mut app);
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
        TextColor(Color::srgb(0.92, 0.95, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(18.0),
            bottom: Val::Px(16.0),
            ..default()
        },
        ViewerOverlay,
        Name::new("viewer-overlay"),
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
    let eye = focus + Vec3::new(angle.sin() * 18.0, 10.0, angle.cos() * 18.0);
    scene.camera = eye;
    for mut transform in &mut query {
        *transform = Transform::from_translation(eye).looking_at(focus, Vec3::Y);
    }
}

fn update_overlay(
    scene: Res<ViewerScene>,
    diagnostics: Res<DiagnosticsStore>,
    objects: Query<&ViewerObject>,
    mut text: Query<&mut Text, With<ViewerOverlay>>,
) {
    let visible = objects.iter().map(|object| { let _ = object.kind.as_str(); 1 }).sum::<usize>();
    let fps = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS).and_then(|diagnostic| diagnostic.value());
    let frame_ms = diagnostics.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME).and_then(|diagnostic| diagnostic.value());
    let entities = diagnostics.get(&EntityCountDiagnosticsPlugin::ENTITY_COUNT).and_then(|diagnostic| diagnostic.value());
    let value = format!(
        "{}  |  place={}  |  objects={}  |  proxy meshes={}  |  light={}  |  camera=({:.1},{:.1},{:.1})\nNative Bevy render/window/PBR/UI path | headless world API preserved\nBevy DiagnosticsStore | fps={} | frame={}ms | entities={} | I toggles browser inspector in wasm viewer",
        scene.title, scene.place, scene.objects, visible, scene.lights, scene.camera.x, scene.camera.y, scene.camera.z,
        fps.map(|value| format!("{value:.1}")).unwrap_or_else(|| "unavailable".to_string()),
        frame_ms.map(|value| format!("{value:.2}")).unwrap_or_else(|| "unavailable".to_string()),
        entities.map(|value| format!("{value:.0}")).unwrap_or_else(|| "unavailable".to_string())
    );
    for mut text in &mut text {
        text.0 = value.clone();
    }
}

fn primary_window_title(scene: Res<ViewerScene>, mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    for mut window in &mut windows {
        window.title = scene.title.clone();
    }
}

fn write_browser_report(scene: Res<ViewerScene>, objects: Query<&ViewerObject>, report: Res<ViewerBrowserReport>, ui: Res<HtmlUiStatus>, overlay: Query<&Text, With<ViewerOverlay>>, labels: Query<(&Text, &ComputedNode, &UiGlobalTransform)>) {
    if report.path.is_empty() {
        return;
    }
    let mut ids = objects.iter().map(|object| object.id.clone()).collect::<Vec<_>>();
    ids.sort();
    let rects = labels
        .iter()
        .filter(|(text, node, _)| !text.0.is_empty() && node.size.x > 0.0 && node.size.y > 0.0)
        .map(|(_, node, transform)| {
            let center = transform.translation;
            let half = node.size * 0.5;
            [center.x - half.x, center.y - half.y, center.x + half.x, center.y + half.y]
        })
        .collect::<Vec<_>>();
    let value = serde_json::json!({
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
        "uiLabels": rects.len(),
        "uiLabelOverlaps": overlapping_pairs(&rects),
        "renderer": "bevy native window / winit / wgpu / pbr / ui",
    });
    let _ = std::fs::write(&report.path, serde_json::to_string_pretty(&value).unwrap_or_default());
}

pub fn browser_report_path(_directory: &str, place: Option<&str>) -> String {
    let name = place.unwrap_or("world");
    format!("web/status-{}.json", name.replace(['/', '\\'], "-"))
}
