use crate::png_check;
use bevy::asset::{AssetPlugin, LoadState};
use bevy::gltf::{Gltf, GltfPlugin};
use bevy::image::ImagePlugin;
use bevy::prelude::*;
use bevy::world_serialization::WorldSerializationPlugin;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const SETTLE_TIMEOUT: Duration = Duration::from_secs(300);
const GLB_MAGIC: &[u8; 4] = b"glTF";
const GLB_VERSION: u32 = 2;
const CHUNK_JSON: u32 = 0x4E4F_534A;
const CHUNK_BIN: u32 = 0x004E_4942;
const REFERENCE_DIRS: [&str; 3] = ["pages", "world", "values"];

pub fn run(directory: &str, project: Option<&str>) -> bool {
    let root = match std::fs::canonicalize(directory) {
        Ok(root) => root,
        Err(error) => {
            println!("{}", json!({ "kind": "gltf-load", "error": error.to_string() }));
            return false;
        }
    };
    let project = project.map(PathBuf::from).unwrap_or_else(|| {
        root.parent()
            .and_then(Path::parent)
            .unwrap_or(root.as_path())
            .to_path_buf()
    });
    let glb_files = collect_sorted(&root, &|path: &Path| has_extension(path, "glb"));
    let png_files = collect_sorted(&root, &|path: &Path| has_extension(path, "png"));
    let bevy_status = load_with_bevy(&root, &glb_files);

    let mut glb_pass = 0usize;
    let mut bevy_loaded = 0usize;
    for path in &glb_files {
        let relative = relative_name(&root, path);
        let result = std::fs::read(path)
            .map_err(|error| error.to_string())
            .and_then(|bytes| parse_glb(&bytes));
        let (ok, mut line) = report("gltf-load", &relative, result);
        let bevy = match bevy_status.get(&relative) {
            Some(Ok(())) => {
                bevy_loaded += 1;
                json!("loaded")
            }
            Some(Err(error)) => json!({ "failed": error }),
            None => json!("not-attempted"),
        };
        line["bevy"] = bevy;
        glb_pass += usize::from(ok);
        println!("{line}");
    }

    let mut png_pass = 0usize;
    for path in &png_files {
        let relative = relative_name(&root, path);
        let (ok, line) = report("png-load", &relative, png_check::check(path));
        png_pass += usize::from(ok);
        println!("{line}");
    }

    let assets = root.parent().unwrap_or(root.as_path()).to_path_buf();
    let present: BTreeSet<String> = collect_sorted(&assets, &|path: &Path| {
        has_extension(path, "glb") || has_extension(path, "png")
    })
    .iter()
    .filter_map(|path| path.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .collect();
    let referenced = referenced_names(&project);
    let referenced_glb = referenced.iter().filter(|name| name.ends_with(".glb")).count();
    let referenced_png = referenced.len() - referenced_glb;
    let mut missing_glb = 0usize;
    let mut missing_png = 0usize;
    for name in referenced.difference(&present) {
        let kind = if name.ends_with(".glb") {
            missing_glb += 1;
            "glb"
        } else {
            missing_png += 1;
            "png"
        };
        println!("{}", json!({ "kind": "missing", "type": kind, "name": name }));
    }

    let glb_fail = glb_files.len() - glb_pass;
    let png_fail = png_files.len() - png_pass;
    let summary = json!({
        "kind": "gltf-summary",
        "root": root.display().to_string(),
        "project": project.display().to_string(),
        "glb": {
            "total": glb_files.len(),
            "pass": glb_pass,
            "fail": glb_fail,
            "referenced": referenced_glb,
            "missing": missing_glb,
            "bevy_loaded": bevy_loaded,
            "bevy_failed": glb_files.len() - bevy_loaded,
        },
        "png": {
            "total": png_files.len(),
            "pass": png_pass,
            "fail": png_fail,
            "referenced": referenced_png,
            "missing": missing_png,
        },
    });
    println!("{summary}");
    glb_fail == 0 && png_fail == 0 && !(glb_files.is_empty() && png_files.is_empty())
}

pub fn parse_glb(bytes: &[u8]) -> Result<Value, String> {
    if bytes.len() < 12 {
        return Err(format!("{} bytes is shorter than the 12-byte GLB header", bytes.len()));
    }
    if &bytes[..4] != GLB_MAGIC {
        return Err("missing glTF magic".to_string());
    }
    let version = read_u32(bytes, 4);
    if version != GLB_VERSION {
        return Err(format!("GLB container version {version}, expected {GLB_VERSION}"));
    }
    let declared = read_u32(bytes, 8) as usize;
    if declared != bytes.len() {
        return Err(format!("header declares {declared} bytes, file has {}", bytes.len()));
    }
    let mut json_chunk: Option<&[u8]> = None;
    let mut bin_chunk: Option<&[u8]> = None;
    let mut offset = 12;
    while offset < bytes.len() {
        if offset + 8 > bytes.len() {
            return Err(format!("truncated chunk header at byte {offset}"));
        }
        let length = read_u32(bytes, offset) as usize;
        let kind = read_u32(bytes, offset + 4);
        let start = offset + 8;
        let end = start
            .checked_add(length)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| format!("chunk at byte {offset} runs past the end of the file"))?;
        if length % 4 != 0 {
            return Err(format!("chunk at byte {offset} has length {length}, not a multiple of 4"));
        }
        match kind {
            CHUNK_JSON if offset == 12 => json_chunk = Some(&bytes[start..end]),
            CHUNK_BIN if bin_chunk.is_none() && json_chunk.is_some() => bin_chunk = Some(&bytes[start..end]),
            _ => {}
        }
        offset = end;
    }
    let json_bytes = json_chunk.ok_or_else(|| "first chunk is not JSON".to_string())?;
    let document: Value =
        serde_json::from_slice(json_bytes).map_err(|error| format!("JSON chunk: {error}"))?;
    let asset_version = document["asset"]["version"].as_str().unwrap_or_default();
    if !asset_version.starts_with("2.") {
        return Err(format!("asset.version is {asset_version:?}, expected 2.x"));
    }
    let bin_len = bin_chunk.map_or(0, |chunk| chunk.len());
    let buffers = array(&document, "buffers");
    if let Some(first) = buffers.first() {
        if first.get("uri").is_some() {
            return Err("GLB buffer 0 must not carry a uri".to_string());
        }
        let declared = first["byteLength"]
            .as_u64()
            .ok_or_else(|| "buffer 0 has no byteLength".to_string())?;
        if declared > bin_len as u64 {
            return Err(format!("buffer 0 needs {declared} bytes, BIN chunk holds {bin_len}"));
        }
    }
    for (index, view) in array(&document, "bufferViews").iter().enumerate() {
        if view["buffer"].as_u64() == Some(0) {
            let end = view["byteOffset"].as_u64().unwrap_or(0) + view["byteLength"].as_u64().unwrap_or(0);
            if end > bin_len as u64 {
                return Err(format!(
                    "bufferView {index} ends at byte {end}, past the BIN chunk of {bin_len} bytes"
                ));
            }
        }
    }
    Ok(json!({
        "version": version,
        "bytes": bytes.len(),
        "meshes": array(&document, "meshes").len(),
        "materials": array(&document, "materials").len(),
        "animations": array(&document, "animations").len(),
        "images": array(&document, "images").len(),
        "buffers": buffers.len(),
        "bin_bytes": bin_len,
        "extensions_required": array(&document, "extensionsRequired"),
    }))
}

fn array<'a>(document: &'a Value, key: &str) -> &'a [Value] {
    document.get(key).and_then(Value::as_array).map_or(&[][..], Vec::as_slice)
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]])
}

fn report(kind: &str, relative: &str, result: Result<Value, String>) -> (bool, Value) {
    let (ok, detail) = match result {
        Ok(detail) => (true, detail),
        Err(error) => (false, json!({ "error": error })),
    };
    let mut line = json!({ "kind": kind, "path": relative, "ok": ok });
    if let (Some(target), Value::Object(fields)) = (line.as_object_mut(), detail) {
        target.extend(fields);
    }
    (ok, line)
}

fn load_with_bevy(root: &Path, files: &[PathBuf]) -> BTreeMap<String, Result<(), String>> {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            file_path: root.to_string_lossy().into_owned(),
            ..default()
        },
        ImagePlugin::default(),
        WorldSerializationPlugin,
        GltfPlugin::default(),
    ));
    app.finish();
    app.cleanup();
    let asset_server = app.world().resource::<AssetServer>().clone();
    let handles: Vec<(String, UntypedHandle)> = files
        .iter()
        .map(|path| {
            let relative = relative_name(root, path);
            let handle: Handle<Gltf> = asset_server.load(relative.clone());
            (relative, handle.untyped())
        })
        .collect();
    let deadline = Instant::now() + SETTLE_TIMEOUT;
    loop {
        app.update();
        let settled = handles.iter().all(|(_, handle)| {
            !matches!(
                asset_server.get_load_state(handle.id()),
                Some(LoadState::Loading) | Some(LoadState::NotLoaded) | None
            )
        });
        if settled || Instant::now() > deadline {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    handles
        .into_iter()
        .map(|(relative, handle)| {
            let status = match asset_server.get_load_state(handle.id()) {
                Some(LoadState::Loaded) => Ok(()),
                Some(LoadState::Failed(error)) => Err(error.to_string()),
                other => Err(format!("not settled: {other:?}")),
            };
            (relative, status)
        })
        .collect()
}

fn referenced_names(project: &Path) -> BTreeSet<String> {
    let mut sources = Vec::new();
    for dir in REFERENCE_DIRS {
        collect(&project.join(dir), &|_: &Path| true, &mut sources);
    }
    let mut names = BTreeSet::new();
    for source in &sources {
        let Ok(bytes) = std::fs::read(source) else {
            continue;
        };
        for token in bytes.split(|byte| !is_name_byte(*byte)) {
            let token = String::from_utf8_lossy(token);
            let name: &str = match token.find("value.") {
                Some(index) if index <= 2 => &token[index..],
                _ => &token,
            };
            if name.len() > 4 && (name.ends_with(".glb") || name.ends_with(".png")) {
                names.insert(name.to_string());
            }
        }
    }
    names
}

fn is_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-')
}

fn has_extension(path: &Path, extension: &str) -> bool {
    path.extension().and_then(|found| found.to_str()) == Some(extension)
}

fn relative_name(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).to_string_lossy().into_owned()
}

fn collect_sorted(directory: &Path, accept: &dyn Fn(&Path) -> bool) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect(directory, accept, &mut files);
    files.sort();
    files
}

fn collect(directory: &Path, accept: &dyn Fn(&Path) -> bool, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, accept, files);
        } else if accept(&path) {
            files.push(path);
        }
    }
}
