use bevy::asset::{AssetPlugin, LoadState};
use bevy::gltf::{Gltf, GltfPlugin};
use bevy::image::ImagePlugin;
use bevy::prelude::*;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const SETTLE_TIMEOUT: Duration = Duration::from_secs(300);

pub fn run(directory: &str) -> bool {
    let root = match std::fs::canonicalize(directory) {
        Ok(root) => root,
        Err(error) => {
            println!("{}", json!({ "kind": "gltf-load", "error": error.to_string() }));
            return false;
        }
    };
    let mut files = Vec::new();
    collect_glb(&root.join("cdn"), &mut files);
    files.sort();
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            file_path: root.to_string_lossy().into_owned(),
            ..default()
        },
        ImagePlugin::default(),
        GltfPlugin::default(),
    ));
    app.finish();
    app.cleanup();
    let asset_server = app.world().resource::<AssetServer>().clone();
    let handles: Vec<(String, UntypedHandle)> = files
        .iter()
        .map(|path| {
            let relative = path.strip_prefix(&root).unwrap_or(path).to_string_lossy().into_owned();
            let handle: Handle<Gltf> = asset_server.load(relative.clone());
            (relative, handle.untyped())
        })
        .collect();
    let deadline = Instant::now() + SETTLE_TIMEOUT;
    loop {
        app.update();
        let settled = handles.iter().all(|(_, handle)| {
            !matches!(asset_server.get_load_state(handle.id()), Some(LoadState::Loading) | Some(LoadState::NotLoaded) | None)
        });
        if settled || Instant::now() > deadline {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let mut loaded = 0usize;
    let mut results = Vec::with_capacity(handles.len());
    for (path, handle) in &handles {
        let entry = match asset_server.get_load_state(handle.id()) {
            Some(LoadState::Loaded) => {
                loaded += 1;
                json!({ "path": path, "ok": true })
            }
            Some(LoadState::Failed(error)) => {
                json!({ "path": path, "ok": false, "error": error.to_string() })
            }
            other => json!({ "path": path, "ok": false, "error": format!("not settled: {other:?}") }),
        };
        results.push(entry);
    }
    let failed = handles.len() - loaded;
    let report: Value = json!({
        "kind": "gltf-load",
        "root": root.display().to_string(),
        "total": handles.len(),
        "loaded": loaded,
        "failed": failed,
        "files": results,
    });
    println!("{report}");
    failed == 0 && !handles.is_empty()
}

fn collect_glb(directory: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_glb(&path, files);
        } else if path.to_string_lossy().ends_with(".glb") {
            files.push(path);
        }
    }
}
