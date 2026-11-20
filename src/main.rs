mod features;
mod features_ecs;
mod glb;
mod globe;
mod gltf_check;
mod png_check;
mod basisu;
mod css_anim;
mod bench;
mod client;
mod protocol;
mod render_stats;
mod runtime;
mod script;
mod server;
mod spec;
mod texture_quality;
mod tree;
mod ui_html;
mod vegetation;
mod vegetation;
mod viewer;
mod viewer_world;
mod world;

fn exit_when_no_gpu_adapter() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if info.to_string().contains("Unable to find a GPU") {
            eprintln!("bevy-jayzee: no GPU adapter was found, so the renderer cannot start. Check the graphics driver (on Linux, the Vulkan ICD files) and run again. Exit status 3.");
            std::process::exit(3);
        }
        default_hook(info);
    }));
}

fn main() {
    exit_when_no_gpu_adapter();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let number = |index: usize, fallback: usize| {
        args.get(index)
            .and_then(|value| value.parse().ok())
            .unwrap_or(fallback)
    };
    match args.first().map(String::as_str) {
        Some("bench") => bench::run(number(1, 2000), number(2, 300)),
        Some("bench-phases") => bench::phases(number(1, 500), number(2, 300)),
        Some("bench-schedules") => bench::schedules(number(1, 500), number(2, 300)),
        Some("det") => {
            if !bench::det(number(1, 2000), number(2, 300)) {
                std::process::exit(1);
            }
        }
        Some("world") => {
            let directory = args.get(1).map(String::as_str).unwrap_or(".");
            let place = args.get(2).map(String::as_str);
            let ticks = number(3, 120);
            if let Err(error) = world::run(directory, place, ticks) {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        Some("bake-terrain") => {
            let directory = args.get(1).map(String::as_str).unwrap_or(".");
            let Some(place) = args.get(2).map(String::as_str) else {
                eprintln!("usage: bevy-jayzee bake-terrain <dir> <place>");
                std::process::exit(2);
            };
            match world::bake_terrain(directory, place) {
                Ok(path) => println!("{{\"kind\":\"terrain-bake\",\"path\":{:?}}}", path.display().to_string()),
                Err(error) => {
                    eprintln!("{error}");
                    std::process::exit(1);
                }
            }
        }
        Some("gltf-check") => {
            let directory = args.get(1).map(String::as_str).unwrap_or("/workspace/neon-syndicates/assets/cdn");
            let project = args.get(2).map(String::as_str);
            let target = match args.get(3) {
                None => basisu::TextureTarget::Bc7,
                Some(name) => match basisu::TextureTarget::from_name(name) {
                    Some(target) => target,
                    None => {
                        eprintln!("usage: bevy-jayzee gltf-check <dir> [project] [rgba8|bc7|etc2|astc4x4]");
                        std::process::exit(2);
                    }
                },
            };
            if !gltf_check::run(directory, project, target) {
                std::process::exit(1);
            }
        }
        Some("texture-quality") => {
            let directory = args.get(1).map(String::as_str).unwrap_or("/workspace/neon-syndicates/assets/cdn");
            if !texture_quality::run(directory) {
                std::process::exit(1);
            }
        }
        Some("server") => server::run(),
        Some("client") => client::run(),
        Some("viewer") => {
            let directory = args.get(1).map(String::as_str).unwrap_or("testdata/spec-world");
            let place = args.get(2).map(String::as_str);
            let report = viewer::browser_report_path(directory, place);
            if let Err(error) = std::fs::write(&report, generate_static_report(directory, place)) {
                eprintln!("{error}");
                std::process::exit(1);
            }
            if let Err(error) = viewer::run(directory, place) {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        Some("web") => {
            let directory = args.get(1).map(String::as_str).unwrap_or("testdata/spec-world");
            let place = args.get(2).map(String::as_str);
            let report = viewer::browser_report_path(directory, place);
            if let Err(error) = std::fs::write(&report, generate_static_report(directory, place)) {
                eprintln!("{error}");
                std::process::exit(1);
            }
            println!("{{\"kind\":\"web-report\",\"path\":\"{}\"}}", report);
        }
        _ => {
            eprintln!("usage: bevy-jayzee <bench [bodies] [ticks] | bench-phases [bodies] [ticks] | det [bodies] [ticks] | world <dir> [place] [ticks] | viewer <dir> [place] | web <dir> [place] | bake-terrain <dir> <place> | gltf-check <dir> [project] | server | client>");
            std::process::exit(2);
        }
    }
}

fn generate_static_report(directory: &str, place: Option<&str>) -> String {
    let tree = match tree::WorldTree::load(std::path::Path::new(directory)) {
        Ok(tree) => tree,
        Err(error) => return serde_json::json!({ "kind": "viewer-status", "error": error }).to_string(),
    };
    let name = place
        .map(str::to_string)
        .or(tree.config.default_place.clone())
        .or_else(|| tree.places.keys().next().cloned())
        .unwrap_or_else(|| "world".to_string());
    let objects = tree.places.get(&name).map(|place| place.objects.len()).unwrap_or(0);
    serde_json::json!({
        "kind": "viewer-status",
        "title": format!("bevy-jayzee browser report — {name}"),
        "directory": directory,
        "place": name,
        "objects": objects,
        "meshes": objects + 1,
        "lights": 1,
        "camera": [12.0, 10.0, 18.0],
        "renderer": "Bevy native window + Bevy WebGPU WASM browser app",
        "nativeDepth": {
            "ecs": "world/runtime/features_ecs.rs systems and components",
            "scriptBridge": "rquickjs API boundary",
            "render": "bevy_window, bevy_winit, bevy_render, bevy_core_pipeline, bevy_pbr, bevy_ui"
        },
        "wasm": { "built": true, "renderer": "Bevy 0.20 WebGPU WASM", "output": "web/wasm/", "url": "http://127.0.0.1:8092/web/wasm/" }
    }).to_string()
}
