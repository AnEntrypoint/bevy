use bevy::prelude::*;
use bevy::render::mesh::RenderMesh;
use bevy::render::render_asset::{RenderAssets, prepare_assets};
use bevy::render::{Render, RenderApp, RenderSystems};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Shared {
    tracking: AtomicBool,
    news: AtomicBool,
    waiting: Mutex<HashSet<AssetId<Mesh>>>,
    uploaded: Mutex<HashSet<AssetId<Mesh>>>,
}

#[derive(Resource, Clone, Default)]
pub struct UploadedMeshes(Arc<Shared>);

impl UploadedMeshes {
    pub fn watch(&self, mesh: AssetId<Mesh>) {
        self.0.waiting.lock().expect("the upload watch list").insert(mesh);
    }

    pub fn is_uploaded(&self, mesh: AssetId<Mesh>) -> bool {
        !self.0.tracking.load(Ordering::SeqCst) || self.0.uploaded.lock().expect("the uploaded set").contains(&mesh)
    }

    pub fn forget(&self, mesh: AssetId<Mesh>) {
        self.0.waiting.lock().expect("the upload watch list").remove(&mesh);
        self.0.uploaded.lock().expect("the uploaded set").remove(&mesh);
    }

    pub fn take_news(&self) -> bool {
        self.0.news.swap(false, Ordering::SeqCst)
    }

    #[cfg(test)]
    pub fn track_by_hand(&self) {
        self.0.tracking.store(true, Ordering::SeqCst);
    }

    #[cfg(test)]
    pub fn pretend_every_watched_mesh_uploaded(&self) {
        let watched: Vec<AssetId<Mesh>> = self.0.waiting.lock().expect("the upload watch list").drain().collect();
        self.0.uploaded.lock().expect("the uploaded set").extend(watched);
        self.0.news.store(true, Ordering::SeqCst);
    }
}

fn publish_uploads(shared: Res<UploadedMeshes>, meshes: Res<RenderAssets<RenderMesh>>) {
    let mut waiting = shared.0.waiting.lock().expect("the upload watch list");
    let done: Vec<AssetId<Mesh>> = waiting.iter().filter(|mesh| meshes.get(**mesh).is_some()).copied().collect();
    if done.is_empty() {
        return;
    }
    let mut uploaded = shared.0.uploaded.lock().expect("the uploaded set");
    for mesh in done {
        waiting.remove(&mesh);
        uploaded.insert(mesh);
    }
    shared.0.news.store(true, Ordering::SeqCst);
}

pub struct UploadedMeshesPlugin;

impl Plugin for UploadedMeshesPlugin {
    fn build(&self, app: &mut App) {
        let shared = UploadedMeshes::default();
        app.insert_resource(shared.clone());
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            shared.0.tracking.store(true, Ordering::SeqCst);
            render.insert_resource(shared).add_systems(Render, publish_uploads.in_set(RenderSystems::PrepareAssets).after(prepare_assets::<RenderMesh>));
        }
    }
}
