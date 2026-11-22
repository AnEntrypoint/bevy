use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use bevy::app::{First, Last};
use bevy::pbr::{RenderMeshInstances, ViewLightEntities};
use bevy::platform::time::Instant;
use bevy::prelude::*;
use bevy::render::mesh::{RenderMesh, RenderMeshBufferInfo};
use bevy::render::render_asset::RenderAssets;
use bevy::render::renderer::RenderDevice;
use bevy::render::settings::WgpuFeatures;
use bevy::render::view::RenderVisibleEntities;
use bevy::render::{ExtractSchedule, Render, RenderApp, RenderSystems};
use std::time::Duration;

#[derive(Default, Clone, Copy, Debug)]
pub struct Tally {
    pub draws: u64,
    pub triangles: u64,
}

#[derive(Default, Clone)]
pub struct FrameStats {
    pub frame_begin: Option<Instant>,
    pub sim_end: Option<Instant>,
    pub sim_ms: Option<f64>,
    pub launch_ms: Option<f64>,
    pub draw_ms: Option<f64>,
    pub cpu_ms: Option<f64>,
    pub extract_ms: Option<f64>,
    pub render_ms: Option<f64>,
    pub main: Tally,
    pub shadow: Tally,
    pub shadow_passes: u64,
    pub timestamp_queries: bool,
    pub gpu_timing_missing: String,
    pub device_features: String,
}

#[derive(Resource, Clone, Default)]
pub struct SharedFrameStats(pub Arc<Mutex<FrameStats>>);

impl SharedFrameStats {
    pub fn snapshot(&self) -> FrameStats {
        self.0.lock().map(|stats| stats.clone()).unwrap_or_default()
    }
}

pub fn is_gpu_pass(path: &str) -> bool {
    path.starts_with("render/") && path.ends_with("/elapsed_gpu")
}

fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn sim_begin(shared: Res<SharedFrameStats>) {
    if let Ok(mut stats) = shared.0.lock() {
        stats.frame_begin = Some(Instant::now());
    }
}

fn sim_end(shared: Res<SharedFrameStats>) {
    if let Ok(mut stats) = shared.0.lock() {
        let now = Instant::now();
        stats.sim_end = Some(now);
    }
}

#[derive(Resource, Default)]
struct FrameMarks {
    frame_begin: Option<Instant>,
    sim_end: Option<Instant>,
    sim_ms: Option<f64>,
    render_begin: Option<Instant>,
    launch_ms: Option<f64>,
    extract_end: Option<Instant>,
    extract_ms: Option<f64>,
}

fn capture_frame_marks(shared: Res<SharedFrameStats>, mut marks: ResMut<FrameMarks>) {
    if let Ok(stats) = shared.0.lock() {
        marks.frame_begin = stats.frame_begin;
        marks.sim_end = stats.sim_end;
        marks.sim_ms = stats.frame_begin.zip(stats.sim_end).map(|(begin, end)| millis(end - begin));
    }
}

fn extract_end(mut marks: ResMut<FrameMarks>) {
    let now = Instant::now();
    marks.extract_end = Some(now);
    marks.extract_ms = marks.sim_end.map(|end| millis(now - end));
}

fn render_close(shared: Res<SharedFrameStats>, marks: Res<FrameMarks>) {
    let now = Instant::now();
    if let Ok(mut stats) = shared.0.lock() {
        stats.extract_ms = marks.extract_ms;
        stats.render_ms = marks.extract_end.map(|begin| millis(now - begin));
    }
}

fn render_begin(mut marks: ResMut<FrameMarks>) {
    let now = Instant::now();
    marks.render_begin = Some(now);
    marks.launch_ms = marks.sim_end.map(|end| millis(now - end));
}

fn render_end(shared: Res<SharedFrameStats>, marks: Res<FrameMarks>) {
    let now = Instant::now();
    if let Ok(mut stats) = shared.0.lock() {
        stats.sim_ms = marks.sim_ms;
        stats.launch_ms = marks.launch_ms;
        stats.draw_ms = marks.render_begin.map(|begin| millis(now - begin));
        stats.cpu_ms = marks.frame_begin.map(|begin| millis(now - begin));
    }
}

fn collect_counts(
    shared: Res<SharedFrameStats>,
    device: Res<RenderDevice>,
    instances: Res<RenderMeshInstances>,
    meshes: Res<RenderAssets<RenderMesh>>,
    views: Query<(Entity, &RenderVisibleEntities)>,
    light_views: Query<&ViewLightEntities>,
) {
    let shadow_views = light_views
        .iter()
        .flat_map(|lights| lights.lights.iter().copied())
        .collect::<HashSet<Entity>>();
    let mut main = Tally::default();
    let mut shadow = Tally::default();
    for (view, visible) in &views {
        let tally = if shadow_views.contains(&view) { &mut shadow } else { &mut main };
        let Some(class) = visible.get::<Mesh3d>() else {
            continue;
        };
        for entity in class
            .entities_cpu_culling
            .iter()
            .map(|(_, entity)| *entity)
            .chain(class.entities_gpu_culling.keys().copied())
        {
            tally.draws += 1;
            let Some(mesh) = instances.mesh_asset_id(entity).and_then(|id| meshes.get(id)) else {
                continue;
            };
            tally.triangles += match &mesh.buffer_info {
                RenderMeshBufferInfo::Indexed { count, .. } => u64::from(*count) / 3,
                RenderMeshBufferInfo::NonIndexed => u64::from(mesh.vertex_count) / 3,
            };
        }
    }
    let features = device.features();
    let missing = [
        ("TIMESTAMP_QUERY", WgpuFeatures::TIMESTAMP_QUERY),
        ("TIMESTAMP_QUERY_INSIDE_ENCODERS", WgpuFeatures::TIMESTAMP_QUERY_INSIDE_ENCODERS),
        ("TIMESTAMP_QUERY_INSIDE_PASSES", WgpuFeatures::TIMESTAMP_QUERY_INSIDE_PASSES),
    ]
    .iter()
    .filter(|(_, feature)| !features.contains(*feature))
    .map(|(name, _)| *name)
    .collect::<Vec<_>>()
    .join(", ");
    if let Ok(mut stats) = shared.0.lock() {
        stats.main = main;
        stats.shadow = shadow;
        stats.shadow_passes = shadow_views.len() as u64;
        stats.timestamp_queries = features.contains(WgpuFeatures::TIMESTAMP_QUERY);
        stats.gpu_timing_missing = missing;
        stats.device_features = format!("{features:?}");
    }
}

pub struct RenderStatsPlugin;

impl Plugin for RenderStatsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SharedFrameStats>()
            .add_systems(First, sim_begin)
            .add_systems(Last, sim_end);
    }

    fn finish(&self, app: &mut App) {
        let shared = app.world().resource::<SharedFrameStats>().clone();
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .insert_resource(shared)
                .init_resource::<FrameMarks>()
                .add_systems(ExtractSchedule, capture_frame_marks)
                .add_systems(Render, extract_end.before(RenderSystems::ExtractCommands))
                .add_systems(Render, render_close.in_set(RenderSystems::PostCleanup))
                .add_systems(Render, render_begin.before(RenderSystems::Render))
                .add_systems(Render, render_end.after(RenderSystems::Render))
                .add_systems(Render, collect_counts.in_set(RenderSystems::Cleanup));
        }
    }
}
