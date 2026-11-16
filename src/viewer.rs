use bevy::diagnostic::{DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResolution};
use std::path::Path;

use crate::spec::{ObjectSpec, PrimitiveDesc};
use crate::tree::WorldTree;
use crate::ui_html::{HtmlUiPlugin, HtmlUiSource, HtmlUiStatus};
use crate::{script, world};

#[derive(Resource, Clone, Debug)]
pub struct ViewerScene {
    pub title: String,
    pub directory: String,
    pub place: String,
    pub objects: usize,
    pub meshes: usize,
    pub unmirrored: usize,
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
    let title = format!("bevy-jayzee viewer — {name}");
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: title.clone(),
            resolution: WindowResolution::new(1280, 720),
            ..default()
        }),
        ..default()
    }))
    .add_plugins((FrameTimeDiagnosticsPlugin::default(), EntityCountDiagnosticsPlugin::default(), LogDiagnosticsPlugin::default()))
    .insert_resource(ClearColor(Color::srgb(0.035, 0.045, 0.065)))
    .insert_resource(ViewerScene {
        title,
        directory: directory.to_string(),
        place: name.clone(),
        objects: place_def.objects.len(),
        meshes: 0,
        unmirrored: 0,
        lights: 1,
        camera: Vec3::new(12.0, 10.0, 18.0),
    })
    .insert_resource(ViewerObjects(place_def.objects))
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
        app.insert_non_send(runner).add_systems(Update, (tick_world, mirror_world_objects, feed_ui_html).chain());
    }
    app.run();
    crate::script::shutdown();
    Ok(())
}

struct WorldRunner(App);

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
struct WorldMirror(Entity);

fn mirror_world_objects(
    mut commands: Commands,
    mut runner: NonSendMut<WorldRunner>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut scene: ResMut<ViewerScene>,
    objects: Res<ViewerObjects>,
    mut mirrors: Query<(Entity, &WorldMirror, &mut Transform)>,
    mut palette: Local<Vec<Handle<StandardMaterial>>>,
) {
    let world = runner.0.world_mut();
    let mut query = world.query::<(Entity, &crate::world::ObjectId, &Transform, Option<&crate::world::PrimitiveInfo>)>();
    let mut unmirrored = 0;
    let items: Vec<(Entity, String, Transform, PrimitiveDesc)> = query
        .iter(world)
        .filter_map(|(entity, id, transform, primitive)| match primitive.and_then(|primitive| primitive.0.clone()) {
            Some(desc) => Some((entity, id.0.clone(), *transform, desc)),
            None => {
                unmirrored += 1;
                None
            }
        })
        .collect();
    scene.unmirrored = unmirrored;
    if palette.is_empty() {
        let colors = [
            Color::srgb(0.24, 0.62, 0.95),
            Color::srgb(0.95, 0.55, 0.25),
            Color::srgb(0.42, 0.82, 0.45),
            Color::srgb(0.78, 0.42, 0.88),
        ];
        palette.extend(colors.into_iter().map(|color| materials.add(StandardMaterial { base_color: color, ..default() })));
    }
    let by_entity: std::collections::HashMap<Entity, &(Entity, String, Transform, PrimitiveDesc)> = items.iter().map(|item| (item.0, item)).collect();
    let mut placed = std::collections::HashSet::new();
    for (viewer, mirror, mut transform) in &mut mirrors {
        match by_entity.get(&mirror.0) {
            Some(item) => {
                *transform = item.2;
                placed.insert(mirror.0);
            }
            None => commands.entity(viewer).despawn(),
        }
    }
    for (entity, id, transform, desc) in &items {
        if placed.contains(entity) {
            continue;
        }
        let mesh = primitive_mesh(&mut meshes, desc);
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(palette[entity.to_bits() as usize % palette.len()].clone()),
            *transform,
            ViewerObject { id: id.clone(), kind: desc.kind.clone() },
            WorldMirror(*entity),
            Name::new(id.clone()),
        ));
    }
    scene.objects = objects.0.len() + items.len();
    scene.meshes = objects.0.len() + items.len();
}

fn primitive_mesh(meshes: &mut Assets<Mesh>, desc: &PrimitiveDesc) -> Handle<Mesh> {
    let kind = desc.kind.as_str();
    if kind.contains("sphere") || kind.contains("ball") {
        meshes.add(Sphere::new(desc.radius))
    } else if kind.contains("capsule") {
        meshes.add(Capsule3d::new(desc.radius, desc.capsule_half_height * 2.0))
    } else if kind.contains("cylinder") {
        meshes.add(Cylinder::new(desc.radius, desc.half_height * 2.0))
    } else {
        meshes.add(Cuboid::new(desc.size.x, desc.size.y, desc.size.z))
    }
}

#[derive(Component)]
struct ViewerOverlay;

#[derive(Resource, Clone, Debug, Default)]
struct ViewerObjects(Vec<ObjectSpec>);

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
    for (index, object) in objects.0.iter().enumerate() {
        let id = object.id.clone().unwrap_or_else(|| format!("object-{index}"));
        let position = object.properties.feet_position.as_ref().map(|value| crate::spec::vec3(Some(value))).unwrap_or(Vec3::ZERO);
        let kind = object.properties.primitive.as_ref().and_then(|value| value.get("kind")).and_then(|value| value.as_str()).unwrap_or("entity").to_string();
        let (mesh, size) = if kind.contains("sphere") || kind.contains("ball") {
            (sphere.clone(), Vec3::splat(1.0))
        } else {
            (cube.clone(), Vec3::new(1.2, 1.2, 1.2))
        };
        let material = materials.add(StandardMaterial { base_color: colors[index % colors.len()], ..default() });
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(position + Vec3::Y * 0.6).with_scale(size),
            ViewerObject { id: id.clone(), kind },
            Name::new(id),
        ));
        scene.meshes += 1;
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
        Text::new("bevy-jayzee native viewer\nloading world objects…"),
        TextFont { font_size: FontSize::Px(18.0), ..default() },
        TextColor(Color::srgb(0.92, 0.95, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(18.0),
            top: Val::Px(16.0),
            ..default()
        },
        ViewerOverlay,
        Name::new("viewer-overlay"),
    ));
}

fn orbit_camera(time: Res<Time>, mut query: Query<&mut Transform, With<Camera3d>>, mut scene: ResMut<ViewerScene>) {
    let angle = time.elapsed_secs() * 0.12;
    let eye = Vec3::new(angle.sin() * 18.0, 10.0, angle.cos() * 18.0);
    scene.camera = eye;
    for mut transform in &mut query {
        *transform = Transform::from_xyz(eye.x, eye.y, eye.z).looking_at(Vec3::ZERO, Vec3::Y);
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
        "{}  •  place={}  •  objects={}  •  proxy meshes={}  •  light={}  •  camera=({:.1},{:.1},{:.1})\nNative Bevy render/window/PBR/UI path • headless world API preserved\nBevy DiagnosticsStore • fps={} • frame={}ms • entities={} • I toggles browser inspector in wasm viewer",
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

fn write_browser_report(scene: Res<ViewerScene>, objects: Query<&ViewerObject>, report: Res<ViewerBrowserReport>, ui: Res<HtmlUiStatus>) {
    if report.path.is_empty() {
        return;
    }
    let mut ids = objects.iter().map(|object| object.id.clone()).collect::<Vec<_>>();
    ids.sort();
    let value = serde_json::json!({
        "kind": "viewer-status",
        "title": scene.title,
        "directory": scene.directory,
        "place": scene.place,
        "objects": scene.objects,
        "meshes": scene.meshes,
        "unmirroredObjects": scene.unmirrored,
        "lights": scene.lights,
        "camera": [scene.camera.x, scene.camera.y, scene.camera.z],
        "visibleIds": ids,
        "uiNodes": ui.nodes,
        "uiWarnings": ui.warnings,
        "renderer": "bevy native window / winit / wgpu / pbr / ui",
    });
    let _ = std::fs::write(&report.path, serde_json::to_string_pretty(&value).unwrap_or_default());
}

pub fn browser_report_path(_directory: &str, place: Option<&str>) -> String {
    let name = place.unwrap_or("world");
    format!("web/status-{}.json", name.replace(['/', '\\'], "-"))
}
