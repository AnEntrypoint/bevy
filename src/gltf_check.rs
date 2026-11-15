use crate::glb::{self, Decoded};
use crate::png_check;
use bevy::asset::{AssetPlugin, LoadState};
use bevy::gltf::{Gltf, GltfPlugin};
use bevy::image::{CompressedImageFormatSupport, CompressedImageFormats, Image, ImagePlugin};
use bevy::mesh::MeshPlugin;
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
const CACHE_DIR: &str = "target/gltf-cache";

struct BevyLoad {
    outcome: Result<(), String>,
    images: usize,
    ktx2_images: usize,
}

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
    let cache = Path::new(env!("CARGO_MANIFEST_DIR")).join(CACHE_DIR);
    let _ = std::fs::remove_dir_all(&cache);
    let decodes = decode_to_cache(&root, &cache, &glb_files);
    let bevy_loads = load_with_bevy(&cache, &decodes);

    let glb_total = glb_files.len();
    let mut glb_parse_pass = 0usize;
    let mut glb_pass = 0usize;
    let mut decoded_ok = 0usize;
    let mut meshopt_views = 0usize;
    let mut basisu_textures = 0usize;
    let mut bevy_loaded = 0usize;
    let mut ktx2_images = 0usize;
    let mut bevy_failed_names = Vec::new();
    for path in &glb_files {
        let relative = relative_name(&root, path);
        let parse = std::fs::read(path)
            .map_err(|error| error.to_string())
            .and_then(|bytes| parse_glb(&bytes));
        let (parse_ok, mut line) = report("gltf-load", &relative, parse);
        glb_parse_pass += usize::from(parse_ok);

        let decode_ok = match decodes.get(&relative) {
            Some(Ok(decoded)) => {
                decoded_ok += 1;
                meshopt_views += decoded.meshopt_views;
                basisu_textures += decoded.basisu_textures;
                line["decode"] = json!({
                    "ok": true,
                    "meshopt_views": decoded.meshopt_views,
                    "basisu_textures": decoded.basisu_textures,
                    "ktx2_textures": decoded.ktx2_textures.len(),
                });
                true
            }
            Some(Err(error)) => {
                line["decode"] = json!({ "ok": false, "error": error });
                false
            }
            None => {
                line["decode"] = json!({ "ok": false, "error": "not decoded" });
                false
            }
        };

        let bevy_ok = match bevy_loads.get(&relative) {
            Some(load) => match &load.outcome {
                Ok(()) => {
                    bevy_loaded += 1;
                    ktx2_images += load.ktx2_images;
                    line["bevy"] = json!({ "loaded": true, "images": load.images, "ktx2_images": load.ktx2_images });
                    true
                }
                Err(error) => {
                    line["bevy"] = json!({ "loaded": false, "failed": error });
                    false
                }
            },
            None => {
                line["bevy"] = json!({ "loaded": false, "failed": "not attempted" });
                false
            }
        };
        if !bevy_ok {
            bevy_failed_names.push(relative.clone());
        }
        glb_pass += usize::from(parse_ok && decode_ok && bevy_ok);
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

    let png_fail = png_files.len() - png_pass;
    let bevy_failed = bevy_failed_names.len();
    let summary = json!({
        "kind": "gltf-summary",
        "root": root.display().to_string(),
        "project": project.display().to_string(),
        "cache": cache.display().to_string(),
        "glb": {
            "total": glb_total,
            "pass": glb_pass,
            "fail": glb_total - glb_pass,
            "parse_pass": glb_parse_pass,
            "decoded": decoded_ok,
            "decode_failed": glb_total - decoded_ok,
            "meshopt_views": meshopt_views,
            "basisu_textures": basisu_textures,
            "referenced": referenced_glb,
            "missing": missing_glb,
            "bevy_loaded": bevy_loaded,
            "bevy_failed": bevy_failed,
            "bevy_failed_names": bevy_failed_names,
            "ktx2_images": ktx2_images,
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
    glb_pass == glb_total && png_fail == 0 && !(glb_files.is_empty() && png_files.is_empty())
}

fn decode_to_cache(
    root: &Path,
    cache: &Path,
    files: &[PathBuf],
) -> BTreeMap<String, Result<Decoded, String>> {
    files
        .iter()
        .map(|path| {
            let relative = relative_name(root, path);
            let decoded = std::fs::read(path)
                .map_err(|error| error.to_string())
                .and_then(|bytes| glb::decode(&bytes))
                .and_then(|decoded| write_cached(cache, &relative, &decoded.bytes).map(|()| decoded));
            (relative, decoded)
        })
        .collect()
}

fn write_cached(cache: &Path, relative: &str, bytes: &[u8]) -> Result<(), String> {
    let target = cache.join(relative);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(&target, bytes).map_err(|error| error.to_string())
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

fn load_with_bevy(
    cache: &Path,
    decodes: &BTreeMap<String, Result<Decoded, String>>,
) -> BTreeMap<String, BevyLoad> {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            file_path: cache.to_string_lossy().into_owned(),
            ..default()
        },
        ImagePlugin::default(),
        MeshPlugin,
        WorldSerializationPlugin,
        GltfPlugin::default(),
    ));
    app.insert_resource(CompressedImageFormatSupport(
        CompressedImageFormats::BC | CompressedImageFormats::ETC2 | CompressedImageFormats::ASTC_LDR,
    ));
    app.finish();
    app.cleanup();
    let asset_server = app.world().resource::<AssetServer>().clone();
    let handles: BTreeMap<String, Handle<Gltf>> = decodes
        .iter()
        .filter(|(_, decoded)| decoded.is_ok())
        .map(|(relative, _)| (relative.clone(), asset_server.load(relative.clone())))
        .collect();
    let deadline = Instant::now() + SETTLE_TIMEOUT;
    loop {
        app.update();
        let settled = handles.values().all(|handle| {
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
    let counts = image_counts(&app, &asset_server, decodes);
    decodes
        .iter()
        .map(|(relative, decoded)| {
            let (images, ktx2_images) = counts.get(relative).copied().unwrap_or_default();
            let outcome = match decoded {
                Err(error) => Err(format!("decode: {error}")),
                Ok(_) => handles.get(relative).map_or_else(
                    || Err("not attempted".to_string()),
                    |handle| gltf_outcome(&asset_server, handle),
                ),
            };
            (relative.clone(), BevyLoad { outcome, images, ktx2_images })
        })
        .collect()
}

fn gltf_outcome(asset_server: &AssetServer, handle: &Handle<Gltf>) -> Result<(), String> {
    match asset_server.get_load_state(handle.id()) {
        Some(LoadState::Loaded) => Ok(()),
        Some(LoadState::Failed(error)) => Err(error.to_string()),
        other => Err(format!("not settled: {other:?}")),
    }
}

fn image_counts(
    app: &App,
    asset_server: &AssetServer,
    decodes: &BTreeMap<String, Result<Decoded, String>>,
) -> BTreeMap<String, (usize, usize)> {
    let mut counts: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for id in app.world().resource::<Assets<Image>>().ids() {
        let Some(path) = asset_server.get_path(id) else {
            continue;
        };
        let text = path.to_string();
        let Some((file, label)) = text.split_once('#') else {
            continue;
        };
        let Some(index) = label
            .strip_prefix("Texture")
            .and_then(|digits| digits.parse::<usize>().ok())
        else {
            continue;
        };
        if !matches!(asset_server.get_load_state(id), Some(LoadState::Loaded)) {
            continue;
        }
        let entry = counts.entry(file.to_string()).or_default();
        entry.0 += 1;
        if let Some(Ok(decoded)) = decodes.get(file) {
            if decoded.ktx2_textures.contains(&index) {
                entry.1 += 1;
            }
        }
    }
    counts
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
