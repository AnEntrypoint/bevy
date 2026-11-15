use bevy::prelude::*;
use bevy::diagnostic::{
    DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin,
    LogDiagnosticsPlugin,
};
use bevy::window::{PrimaryWindow, WindowResolution};
use wasm_bindgen::prelude::*;

#[derive(Component)]
struct OrbitCamera;

#[derive(Component)]
struct Spinner;

#[derive(Resource)]
struct ReadySent(bool);

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
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "bevy-jayzee WebGPU WASM".to_string(),
            canvas: Some("#bevy-canvas".to_string()),
            fit_canvas_to_parent: true,
            resolution: WindowResolution::new(960, 540),
            ..default()
        }),
        ..default()
    }))
    .add_plugins((
        FrameTimeDiagnosticsPlugin::default(),
        EntityCountDiagnosticsPlugin::default(),
        LogDiagnosticsPlugin::default(),
    ))
    .insert_resource(ClearColor(Color::srgb(0.025, 0.04, 0.07)))
    .insert_resource(ReadySent(false))
    .insert_resource(InspectorState { enabled: inspector_enabled, last_update: 0.0 })
    .add_systems(Startup, setup)
    .add_systems(Update, (orbit, spin, mark_ready, publish_diagnostics));
    app.run();
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

fn orbit(time: Res<Time>, mut cameras: Query<&mut Transform, With<OrbitCamera>>) {
    let angle = time.elapsed_secs() * 0.18;
    for mut transform in &mut cameras {
        *transform = Transform::from_xyz(angle.sin() * 11.0, 5.2, angle.cos() * 11.0)
            .looking_at(Vec3::NEG_Y * 0.1, Vec3::Y);
    }
}

fn spin(time: Res<Time>, mut meshes: Query<&mut Transform, With<Spinner>>) {
    for mut transform in &mut meshes {
        transform.rotation = Quat::from_rotation_y(time.elapsed_secs() * 0.9);
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
    let json = format!(
        "{{\"kind\":\"bevy-diagnostics\",\"source\":\"DiagnosticsStore\",\"time\":{},\"plugins\":[\"FrameTimeDiagnosticsPlugin\",\"EntityCountDiagnosticsPlugin\",\"LogDiagnosticsPlugin\",\"DiagnosticsPlugin/DefaultPlugins\"],\"diagnostics\":[{fps},{frame_time},{frame_count},{entities}],\"renderDiagnostics\":{render},\"ecs\":{{\"cameras\":{},\"lights\":{},\"meshEntities\":{},\"sprites\":{},\"uiNodes\":{}}},\"assets\":{{\"meshes\":{},\"images\":{},\"standardMaterials\":{}}},\"unavailable\":[\"drawCalls\",\"triangles\",\"shadowMapDraws\",\"shadowTriangles\",\"shadowPasses\",\"cascadeSkips\",\"gpuPassTimestamps\",\"frameGraph\",\"textureBytes\",\"terrain\",\"physics\",\"multiplayer\",\"jobs\",\"vibes\",\"parameters\"]}}",
        js_sys::Date::now(),
        cameras.iter().count(),
        lights.iter().count(),
        meshes.iter().count(),
        sprites.iter().count(),
        ui_nodes.iter().count(),
        mesh_assets.len(),
        image_assets.len(),
        material_assets.len()
    );
    let window = web_sys::window().expect("window");
    let _ = js_sys::Reflect::set(&window, &JsValue::from_str("__BEVY_INSPECTOR_SNAPSHOT"), &JsValue::from_str(&json));
    if let Some(element) = window.document().expect("document").get_element_by_id("inspector-data") {
        element.set_text_content(Some(&json));
    }
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
