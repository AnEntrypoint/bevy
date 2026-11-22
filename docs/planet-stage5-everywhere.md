# Planet stage 5 everywhere: design (Phase A)

Plan: /workspace/hyperagent/specs/bevy-planet-parity.md, Stage 5 (lines 86-88): running with no arguments opens the full-scale planet; the wasm viewer draws the same planet and vegetation through WebGPU; witness: the default launch and the wasm viewer both pass the pose captures. Done section line 100: 'The default launch and the wasm viewer show it.' Poses: docs/planet-parity.md lines 14-24. Port: /workspace/bevy at a4e0a27. Bevy 0.20.1 and wgpu-types 30.0.2 in both Cargo.lock and wasm-viewer/Cargo.lock, sources cited from /config/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/.

Everything below was read, not run: no cargo, wasm-pack, trunk or browser command was made for this design. A number is either read from the cited line or marked 'computed from' cited constants. A cost nobody timed is marked unmeasured.

## 1. The no-argument launch

### 1.1 What it does today

- Dispatch. src/main.rs:136-139: with no arguments, args becomes ['viewer', 'testdata/spec-world', 'globe']. The viewer branch (src/main.rs:207-219) writes the static browser report (generate_static_report, src/main.rs:237) and calls viewer::run.
- The place. testdata/spec-world/places/globe/config.yaml:3-13 holds a globe block: radius 63600, seed 1337, anchorDir [-0.641, 0.2558, 0.7237], reliefScale 0.001, maxLevel 13, bodyAltitude 0.5, dropHeight 6, one body at latitude 0, longitude 0. These equal the preset constants in src/globe.rs:56-61 (DEFAULT_SEED 1337, DEFAULT_RADIUS 63_600.0, DEFAULT_MAX_LEVEL 13, DEFAULT_ANCHOR_DIR) and the preset tests default_radius_is_the_preset_radius and default_planet_is_the_preset (src/globe.rs:701-712). The place became the preset in 6a7b03e (stage 1). The spec's Facts line 'testdata/spec-world/places/globe uses radius 64' is stale. testdata/spec-world/world.config.yaml:1 names defaultPlace test, so the explicit 'globe' in main.rs:138 is what selects the planet.
- viewer::run (src/viewer.rs:145-243): DefaultPlugins with a 1280 by 720 window and the asset root target/viewer-cache (src/viewer.rs:160-174). When the place has a globe key (src/viewer.rs:221-226) it inserts GlobeWorld::from_value, calls globe::register, adds frame_globe_camera and the census systems; otherwise it uses orbit_camera (src/viewer.rs:228). spawn_viewer (src/viewer.rs:282) skips the flat ground for a globe (:290) and spawns the camera (:331-336) and two text overlays (:337-362), which are hidden only in census mode (hide_live_text, :425-429, wired at :226).
- World runner. world.config.yaml:11-12 sets ui.render, so world_runner (src/viewer.rs:245-252) builds a second App with world::prepare (src/world.rs:70-152): MinimalPlugins and avian PhysicsPlugins (src/world.rs:91), GlobeWorld from the same place (src/world.rs:141) and runtime::register, which calls globe::register (src/runtime.rs:191). That App has no Assets<Mesh>, so globe::register gives it only ColliderStreamPlugin (src/globe.rs:428-430). The render App has no PhysicsPlugins.
- Every stage 1 to 4 plugin is already in the default launch. globe::register (src/globe.rs:408-438), when Assets<Mesh> exists, adds the LOD plugin (which brings ColliderStreamPlugin, src/planet_lod/stream.rs:254), TerrainMaterialPlugin, WaterPlugin, AtmosphereLutPlugin, PlanetSkyPlugin, LightPlugin, ToneMapPlugin, the stage 4 RenderAssetBytesPerFrame of 4 MiB (src/globe.rs:406, :423), equip_camera (src/globe.rs:375-379) and add_vegetation (VegGpuPlugin, TreesRenderPlugin, RockRenderPlugin, GrassRenderPlugin, VegStreamPlugin; src/globe.rs:395-398).
- Camera. frame_globe_camera (src/globe.rs:465-478) puts the eye 10 m above and 14 m behind the focus (FRAMING_HEIGHT_M, FRAMING_BACK_M, src/globe.rs:461-462), with far = 4 radii (src/globe.rs:64). The focus is the first body (update_focus, src/globe.rs:526-537), spawned at latitude 0, longitude 0. None of the five poses is reachable from the default launch.
- Census. scripts/census-run.sh:4 does cd /workspace/bevy, then :9 runs target/debug/bevy-jayzee with no arguments and BEVY_JAYZEE_CENSUS=1. add_census_systems (src/globe.rs:494-502) fixes the frame time at 1/60 s, freezes the simulation at 10 s (src/globe.rs:482, :504-514) and captures at frame 1500 once the LUTs are ready and no patch task is in flight (src/globe.rs:483, :516-524) with Screenshot::primary_window (src/globe.rs:521). That needs a window on an X display, against Architecture 9 of the spec ('Visual witnesses render to an offscreen image, never the X display').

### 1.2 What is missing, and the exact change

The launch already opens the preset planet with every plugin. Four gaps remain:

1. Working directory. 'testdata/spec-world' in src/main.rs:138 is relative. census-run.sh works only because of the cd at :4. From any other directory WorldTree::load fails and the launch exits 1 (src/viewer.rs:146). The texture and species roots already use CARGO_MANIFEST_DIR (src/globe.rs:351-355, src/planet_veg/trees_render/assets.rs:176-178), so the launch should too.
2. No witness ties the no-argument launch to the planet. Nothing asserts that the default arguments reach a place with a globe key whose GlobeWorld is the preset.
3. The census picture is a window capture at one body pose, not the five poses, and it is not offscreen.
4. The overlays are drawn in a normal launch (hidden only in census, src/viewer.rs:226). They stay: the pose capture below does not use the window.

Change (all in the main crate, no new dependency):

- src/main.rs:137-139: replace the literal with a function default_launch() -> [String; 3] that returns 'viewer', Path::new(env!('CARGO_MANIFEST_DIR')).join('testdata/spec-world') and 'globe'.
- A test the_default_launch_is_the_preset_planet next to it: load the tree from default_launch()[1], resolve the place as viewer::run does (src/viewer.rs:147-151), build GlobeWorld::from_value(place.config.extra.get('globe')) and assert radius, seed, max_level, anchor_dir and relief equal GlobeWorld::default() (the preset, src/globe.rs:66-89). This is the red check for a wrong preset radius and for a default launch without a globe.
- A test the_default_launch_registers_every_planet_plugin: build the viewer's render App parts on offscreen::headless_app with that GlobeWorld, call globe::register, and assert the resources each plugin inserts are present (PlanetLodStats, TerrainMaterialHandle, AtmosphereLuts after the bake, VegStreamStats, VegFrame, TreesWitness; the same names bench_planet::settled reads, src/bench_planet.rs:197-212).
- Optional cleanup, decided no: the place file can stay explicit rather than 'globe: {}', because the place is also read by world::prepare and the tests at src/globe.rs:701-712 already pin the defaults.

## 2. The wasm viewer

### 2.1 What it is today

- Crate. wasm-viewer/Cargo.toml is its own workspace (:1), package bevy-jayzee-wasm (:4), a cdylib (:10-11). The main crate excludes it (Cargo.toml:7-9). It does not depend on the main crate; it shares one source file by path: #[path = '../../src/render_stats.rs'] mod render_stats (wasm-viewer/src/lib.rs:15-16).
- Features. bevy 0.20.1 with default-features off (wasm-viewer/Cargo.toml:14) and bevy_log, bevy_state, bevy_asset, bevy_render, bevy_core_pipeline, bevy_pbr, bevy_ui, bevy_ui_render, bevy_window, bevy_winit, bevy_gizmos_render, default_font, png, std, async_executor, web, webgpu (:31), tonemapping_luts, zstd_rust, bevy_gltf, ktx2, bevy_world_serialization, reflect_auto_register (:15-37). No multi_threaded, no jpeg, no webgl2. wasm-bindgen =0.2.129 (:39); js-sys, web-sys and getrandom wasm_js for wasm32 (:41-44). Release: opt-level s, thin LTO, panic abort, strip (:46-52).
- Entry point. #[wasm_bindgen] pub fn start() (wasm-viewer/src/lib.rs:52-94): DefaultPlugins with the asset root 'assets' (or '/' with ?model=) and AssetMetaCheck::Never (:62-64), canvas #bevy-canvas fitted to its parent at 960 by 540 (:65-71), device_request with ?disable-bc and ?disable-astc (:75, :179-188), the diagnostics plugins and RenderStatsPlugin (:78-84).
- What it draws. setup (:96-165): a 40 by 40 plane, a spinning cube and sphere, one directional light with shadows off (:141-150), an orbit camera and a text overlay. ?model= loads a glTF scene through the AssetServer (:251) from a texture cache chosen by device support (:32-35, :193-222). ?still=1 freezes the clock (:89, :310-311). Ready: window.__BEVY_WASM_READY = true (:344) and data-app-ready='true' (:341). No planet or vegetation code.
- Build. scripts/build-wasm.sh: CARGO_TARGET_DIR=target/wasm-viewer (:10), cargo build --release --target wasm32-unknown-unknown --locked (:12), wasm-bindgen --target web into web/wasm (:20), then copies web/wasm-index.html, wasm-boot.js, favicon.svg, inspector.js and status.json (:21-25). Recorded: release build 3m48s (STATUS.md:66) and 217 s with bevy_gltf (docs/PENDING_VALIDATION.md:336). Artifacts on disk: web/wasm/bevy-jayzee-wasm_bg.wasm 31,140,873 bytes and target/wasm-viewer/.../bevy_jayzee_wasm.wasm 34,339,820 bytes; STATUS.md:56 still says 29,273,160 (stale).
- Boot. web/wasm-boot.js fetches status.json (:14-17), turns ?adapter=software into forceFallbackAdapter (:18-21), handles a missing navigator.gpu or adapter (:28-35) and calls start() (:39-42).
- Serve. scripts/serve-wasm.py serves the repo root on PORT, default 8092 (:8), with application/wasm (:13) and COOP same-origin, COEP require-corp, CORP same-origin and no-store (:22-25); scripts/serve-wasm.sh:5 runs it. scripts/check-static-headers-witness.sh checks those headers. scripts/serve_viewer.sh is the old static report server and is unrelated.
- Browser witnesses. scripts/lib/playwriter-witness.cjs drives the playwriter-project CLI (:43-48, with PLAYWRITER_PROJECT_DIR and PLAYWRITER_WEBGPU=1), refuses a busy port and starts serve-wasm.sh (:73-78, page base http://127.0.0.1:PORT/web/wasm/index.html), starts the gate (:83), opens a direct CDP session (:86-87), runs snippets (:90) and stops the browser, gate and server it started (:95, :104-106). scripts/check-neon-wasm-models.cjs uses it (:4, :123-125): adapter=software queries (:9-16), waits on __BEVY_WASM_READY for up to 120 s at 250 ms (:56-63), then takes page.screenshot (:66-67); pass = ready, loaded, adapter present, no errors (:95, :133). scripts/compare-screenshots.cjs compares two folders by PSNR with VISUAL_PSNR_MIN default 30 (:10, :72-94). check-wasm-webgpu.cjs and check-normal-map-visual.cjs, cited at docs/PENDING_VALIDATION.md:269 and :328, no longer exist.
- Headless state. STATUS.md:60 and :73-75: headless Chromium in this container reported 'Failed to initialize vulkan surface' and a renderer panic; data-app-ready=true was reached only with Chromium on DISPLAY=:1 and an NVIDIA adapter (STATUS.md:62). check-neon-wasm-models.cjs passes adapter=software through playwriter, which is the path the stage 5 witness reuses.

### 2.2 What blocks each shared module on wasm

| module | wasm blocker | evidence | plan |
|---|---|---|---|
| all task users | bevy_tasks on wasm32 is the single-threaded pool: spawn runs the future on the main thread with wasm_bindgen_futures::spawn_local. Every AsyncComputeTaskPool build therefore runs on the browser main thread. | bevy_tasks-0.20.1/src/lib.rs:21, :26, :101-103; single_threaded_task_pool.rs:33, :197. Spawns: planet_lod/stream.rs:165 (patches), :247 (climate), planet_lod/collider.rs:140, planet_material/terrain.rs:289, planet_sky/lut.rs:100, planet_water/surface.rs:240 and :320, planet_veg/stream.rs:242 and :267, trees_render.rs:138, rock_render.rs:66, grass_render.rs:243. | Accept for stage 5: the tasks are futures with check_ready polling (for example stream.rs:13), so they compile and run. The stage 4 rule 'no main-thread build' cannot hold on wasm without web-worker threads (atomics, nightly build-std), and the guard (planet_lod/guard.rs:8-20, marked at stream.rs:245) would count them. The wasm witness reports builds_on_main_thread as a measured number, not a pass condition. Open question Q2. |
| planet_lod | ColliderStreamPlugin is added unconditionally (stream.rs:254) and uses avian3d (collider.rs). The main crate's avian has the parallel feature (Cargo.toml:14). The viewer has no bodies. | as cited | The wasm crate depends on avian3d 0.8 with 3d and parry-f32 and without parallel (avian3d-0.8.0/Cargo.toml:80-82 defines parallel). With no NearBody nothing is built. Whether avian compiles for wasm32 here is unverified until the Phase B compile. |
| planet_lod climate bake | HpfFaces bake at res 128, max level 7, once, before any patch starts (stream.rs:19-20, :136-139). On wasm it runs on the main thread. | as cited | Unmeasured. The wasm capture reports the time to the first live patch. |
| planet_material | Layer textures are read with std::fs::read (planet_material/layers.rs:61) from a directory given by crate::globe::texture_dir() or CARGO_MANIFEST_DIR (terrain.rs:38, :283). There is no filesystem on wasm32-unknown-unknown. The jpg decode uses bevy::image (layers.rs:4, :60) and the wasm crate has no jpeg feature. Four 1024 by 1024 layers (layers.rs:15-16) in a 2D array (:221). Shader: terrain_material.wesl by include_str! (terrain.rs:40) and Shader::from_wesl (terrain.rs:261), which is not feature gated (bevy_shader-0.20.1/src/shader.rs:176). | as cited; assets/planet is 17 MB (du), textures 12 jpg files | Decode takes bytes, not a path: decode_layers(dir) becomes decode_layers(files) over a byte source. Native reads files; wasm fetches them through the AssetServer (the viewer already loads over HTTP with AssetMetaCheck::Never, wasm-viewer/src/lib.rs:62-64). Add jpeg to the wasm bevy features. build-wasm.sh copies assets/planet to web/wasm/assets/planet. The shader is embedded, so it needs nothing. |
| planet_sky LUT | The atmosphere tables are baked on the CPU (lut.rs:33-36; atmosphere.rs:147, :218): transmittance 256 by 128 at 500 steps, scattering 64 by 32 by 24 at 64 steps (atmosphere.rs:20-26). That is 16,384,000 and 3,145,728 integration steps (computed from those constants). Output Rgba16Float (lut.rs:79-81), scattering a D2Array (:91). On wasm it runs on the main thread. | as cited; no bake time is recorded anywhere | Measure it in the first wasm window. If it blocks the first frame for longer than the owner accepts (Q3), ship the baked bytes: 256 x 128 x 8 = 262,144 bytes and 64 x 32 x 24 x 8 = 393,216 bytes (computed, 8 bytes per Rgba16Float texel), generated by the native bake and witnessed byte-equal to it. |
| planet_sky sky, tone map | Shaders by include_str! and Shader::from_wgsl (sky.rs:29-30, :279; tonemap.rs:104). Formats Rgba8UnormSrgb and Rgba16Float only. | grep of TextureFormat in src/planet_* | No blocker found by reading. |
| planet_sky light | Cascaded shadows, DirectionalLightShadowMap size 1024 (light.rs:12, :267), cascade_config (light.rs:58-60). | as cited | Within the WebGPU default max_texture_dimension_2d 8192 (wgpu-types-30.0.2/src/limits.rs:424). No blocker found by reading. |
| planet_water | Two AsyncComputeTaskPool spawns (surface.rs:240, :320). Shader by include_str! (surface.rs:162-164). | as cited | Same as the task row. |
| planet_veg GPU core | Compute culling (cull.rs:308, dispatch at :380, :383, :386, WORKGROUP_SIZE 64 at :20), storage buffers (buffers.rs:313, :354), storage in the vertex stage (draw.rs:191-199, read-only), draw_indexed_indirect (draw.rs:361) with first_instance 0 (asserted at gpu/render_tests.rs:283) and a map_async readback (cull.rs:348). The tree cull bind group holds 8 storage buffers (cull.rs:267-277: bindings 1 to 8). | wgpu defaults(): max_storage_buffers_per_shader_stage 8 (limits.rs:421, :434); downlevel_defaults 4 (:557, :562); downlevel_webgl2 0 (:591, :644) | WebGPU only, never WebGL2. The tree cull sits exactly at the WebGPU default limit of 8, with no headroom: one more storage binding fails on a default adapter. first_instance 0 means no INDIRECT_FIRST_INSTANCE feature is needed. No WgpuFeatures are requested by the planet modules (grep). |
| planet_veg assets | Species files and tree textures by std::fs::read (species.rs:233, trees_render/assets.rs:91) from CARGO_MANIFEST_DIR (species.rs:254, assets.rs:176-178). Seventeen .veg files from 211,556 to 1,394,348 bytes plus bark and leaves (ls assets/planet/vegetation). Impostor atlas 2048 px with 12 mips (trees_render/gpu.rs:18-22, :124-125), tree texture budget MAX_PX 1024 (budget.rs:3-6), grass clump 256 (grass_render.rs:29). | as cited | Same byte-source seam as the material. The atlas is under the 8192 limit. |
| test harness | offscreen::headless_app forces the lavapipe ICD and Vulkan (offscreen.rs:18, :46-51, :58) and blocks on device.poll (offscreen.rs:78). The readback helpers are cfg(test) (offscreen.rs:104). There is no wasm test harness. | as cited | The wasm side is witnessed only in the browser through playwriter. Native and wasm captures run on different software rasterisers (lavapipe against the browser's fallback adapter), so pixel equality is not expected. Section 3. |

### 2.3 Module boundary: shared sources by path, decided against a library crate and against feature flags

Evidence:

- The main crate is a binary only: no [lib], no src/lib.rs, no [features] (Cargo.toml). The planet modules are declared in src/main.rs:15 (height) and :110-115.
- The main package's non-optional dependencies do not build for wasm32: rquickjs (C, Cargo.toml:23), lightyear with udp and raw_connection (:15), bevy with x11, multi_threaded and basis-universal (:13), and build.rs, which compiles C++ native/basislz_ktx2.cpp with cc (build.rs:1-33). Cargo builds every non-optional dependency and the build script for every target of the package, so a wasm target of this package needs all of them made optional plus cfg gates in every non-planet module. grep finds no cfg(target_arch) and no cfg(feature) anywhere in src or build.rs today. STATUS.md:29 records that the wasm viewer exists to avoid x11 and winit, UDP and lightyear, and rquickjs.
- A separate library crate means moving src/planet_* and src/height. All 28 planet mutant tables (867 lines, scripts/mutants/planet-*.tsv) name files under src/ and run cargo test --bin bevy-jayzee; so does every stage window (scripts/window-planet-stage4.sh:15). A move rewrites every red check and every witness command, and the whole red set reruns.
- The wasm viewer already shares main-crate code by #[path] (wasm-viewer/src/lib.rs:15-16).

Decision: the wasm crate compiles the same files by #[path]. Nothing moves; the native binary, its tests and all 867 mutant commands stay valid. The wasm crate root (wasm-viewer/src/lib.rs) declares, by #[path] to ../../src/..., the crate-root modules the planet code names: height, vec3, js_math, refusal, rock_shapes, rock_tri_table, planet_lod, planet_material, planet_sky, planet_veg, planet_water, and the new planet_scene below. A grep of crate:: names in src/planet_* and src/height outside tests gives exactly these, plus crate::globe (terrain.rs:283, non-test; stream.rs:430, test) and crate::offscreen and crate::testkit (tests only, not compiled in the cdylib).

Two seams in the main crate make the shared set closed:

1. src/planet_scene.rs (new, shared): the render half of globe::register, so the window launch, the pose capture and the wasm viewer use one wiring. It takes from src/globe.rs: the preset constants (:56-64), default_planet_config (:91-93), quadtree_config (:291-299), the plugin builders lod_plugin, veg_stream_plugin, water_plugin and terrain_frame (:273-289), texture_dir (:351-355), align_lights_with_focus (:357), equip_camera (:375-379), light_vegetation and add_vegetation (:381-398), and the Assets<Mesh> branch of register (:411-427). It depends only on the planet modules and height, not on avian, spec, script or GlobeWorld; it takes a small PlanetSetup value (planet, radius, centre, seed, anchor, quality, max level, focus). globe::register calls it; terrain.rs:283 calls planet_scene::texture_dir.
2. A byte source for planet files: decode functions take bytes (layers.rs:61, species.rs:233, trees_render/assets.rs:91). Native: a filesystem reader rooted at texture_dir and species_directory. Wasm: bytes fetched through the AssetServer before the decode tasks start. This is the only cfg(target_arch = 'wasm32') in the shared set.

Wasm crate changes: add jpeg to the bevy features; add avian3d 0.8 (3d, parry-f32, no parallel); add the #[path] modules; a ?planet=1 (or the default once it passes) start path that builds DefaultPlugins with the canvas, calls planet_scene, spawns the camera at a pose from ?pose=<name>, and publishes the pose state on window.__PLANET_POSE (counts below) once settled. The two locks resolve the same bevy 0.20.1 and wgpu-types 30.0.2 today.

## 3. The pose captures

### 3.1 The five poses

One pure module src/planet_poses.rs, shared by #[path], gives offscreen::Frame-like values (position, forward, up, sun) for the preset planet. It reuses the stage 4 path definitions so the poses are the ones stage 4 already witnessed:

| pose | frame | source |
|---|---|---|
| orbit | Frame::above(anchor, R, 100 km, 90 degrees down) | FLIGHT_ALTITUDES_M[0] and STRAIGHT_DOWN_DEG, src/bench_planet.rs:28, :31, :87-97; bookmark out-100km (docs/planet-parity.md:20) |
| high altitude | Frame::above(anchor, R, 20 km, 90 degrees down) | src/bench_planet.rs:28; docs/planet-parity.md:21 |
| low altitude | Frame::above(anchor, R, 5 km, 90 degrees down) | src/bench_planet.rs:28; docs/planet-parity.md:22 |
| ground in forest | the stage 3 census frame at CENSUS_ANCHOR, 6 degrees down | src/bench_planet.rs:27, :32, :78-85 (on_ground at 0 m); docs/planet-stage4-hitch.md:206 |
| shoreline | the water witness's search (deepest of 40,000 Fibonacci points, then 16 headings to the first land, bisected 40 times) run on the preset planet, at 150 m, 10 degrees down, facing land | src/planet_water/render_tests.rs:27-31, :47, :72-101, :128 |

The anchor is the preset anchor (src/globe.rs:61). Native and wasm share the module, so both targets render identical frames. The orbit, high and low poses are the stage 4 lift of the bookmarks (altitude = bookmark km), a port choice already marked (docs/planet-stage4-hitch.md:198, Q5 at :275). The shoreline search today runs on radius 6,360,000 with relief 0.1 and anchor [0, 1, 0] (render_tests.rs:25-26, :53); stage 5 runs it on the preset planet. The reference's coast and forest bookmarks are searches (docs/planet-parity.md:23-24), so these two are port choices too. The shoreline direction is computed once in a test and kept as a constant, with a test that the search still returns it within 1e-9.

### 3.2 Native: offscreen, through the default-launch wiring

Test default_launch_passes_the_pose_captures (cargo test --bin bevy-jayzee planet_poses -- --test-threads=1): GlobeWorld from default_launch()'s place (section 1.2), offscreen::headless_app, planet_scene through globe::register, an Rgba8UnormSrgb target (offscreen.rs:81-85) at 320 by 180 (offscreen.rs:21-22), the SingleThreadedExecutor and the 1/60 s step exactly as the stage 4 rig (src/bench_planet.rs:224-241). For each pose: set the camera and a NearBody to the frame, step until settled holds for 4 frames (bench_planet.rs:197-212, :258-265, SETTLED_STREAK :38, limit SETTLE_LIMIT 4000 :37), capture (offscreen.rs:148-168), write target/witness/pose-native-<pose>.png (offscreen.rs:211-218) and a JSON line with the counts below.

### 3.3 Wasm: through playwriter

scripts/check-wasm-planet-poses.cjs on scripts/lib/playwriter-witness.cjs, the same way check-neon-wasm-models.cjs works: serve-wasm.sh, one page per pose at web/wasm/index.html?planet=1&pose=<pose>&adapter=software&still=1&width=320&height=180, wait for window.__PLANET_POSE.settled with a time limit, then locator('#bevy-canvas').screenshot() to target/witness/pose-wasm-<pose>.png and read window.__PLANET_POSE. The canvas is fixed at 320 by 180 with device scale 1, so it matches the native target. ?still=1 fixes time so the wind phase and water time match the native step count (both report the frame count they captured at).

### 3.4 What 'passes' means

The reference captures cannot be taken on this host (stage 2 status, spec line 74). Each capture is checked three ways; the reference comparison stays an open row.

A. Structural, per capture (same code for both targets: a node script over the PNG plus the published counts):

| pose | check | measure source |
|---|---|---|
| all | settled within the limit; no pipeline error; live_patches > 0; LUTs ready; layers ready (surf_ready 1.0) | bench_planet.rs:197-212; offscreen.rs:163-165 |
| all | image not flat: luma standard deviation >= 0.01 and at least 32 distinct colours | LUMA_STD_MIN and DISTINCT_COLOURS_MIN, render_tests.rs:41-42; image_stats offscreen.rs:182-202 |
| orbit | the planet limb is in frame: both sky pixels and planet pixels present; no black share above 0.5 percent | BLACK_MAX_PERCENT, scripts/check-planet-screenshot.cjs:6 |
| high, low | live_patches and the deepest live level rise from orbit to high to low | PlanetLodStats (stream.rs:36, publish_stats :222); a deepest-level field is added if the struct has none (not checked) |
| ground in forest | tree resident count equals the stage 3 census (4982) and the CPU twin; visible trees equal the CPU twin (818 at that pose in stage 3); rocks and grass > 0; draw calls <= 144 (species x tiers); overflow 0 | spec stage 3 status (line 80); TreesWitness and VegStreamStats |
| shoreline | water pixels and land pixels both present (the water census idea of docs/planet-stage2-sky.md:172) | share thresholds: Q4 |

B. Native against wasm, per pose: the deterministic counts must be equal (live patch keys, tree resident count, rock and grass instance counts), because both run the same leaf selection from the same frame (QuadtreeConfig uses the fixed CAMERA_VIEWPORT_HEIGHT_PX 720, src/globe.rs:62, :293, not the canvas) and the same placement. The images are compared with scripts/compare-screenshots.cjs at its existing threshold, PSNR >= 30 dB (VISUAL_PSNR_MIN default, :10). Lavapipe and the browser fallback adapter differ in rasterisation and float precision, so 30 dB is the starting bar and Q5 asks the owner to confirm it.

C. Against the reference capture of the same pose: open. A row stage5:ref-capture is added to docs/planet-parity.md with status open and the reason (no reference capture on this host).

## 4. Red checks

Each is a mutant line in scripts/mutants/planet-s5.tsv (name, file, old text, new text, test command), applied, run to a nonzero exit, and restored with cmp.

| name | mutant | must fail |
|---|---|---|
| s5-no-planet | src/main.rs default_launch: 'globe' to 'test' | the_default_launch_is_the_preset_planet |
| s5-radius | testdata/spec-world/places/globe/config.yaml: radius 63600 to 6360 | the_default_launch_is_the_preset_planet |
| s5-relative-dir | default_launch drops the CARGO_MANIFEST_DIR join | a test that runs the resolution from another current directory |
| s5-no-veg | planet_scene: drop add_vegetation | the forest pose (resident 0) on native |
| s5-no-water | planet_scene: drop the water plugin | the shoreline structural check |
| s5-no-sky | planet_scene: drop AtmosphereLutPlugin | settled never holds (LUTs not ready) |
| s5-pose-swap | planet_poses: orbit altitude 100 km to 5 km | the orbit limb check |
| s5-wasm-no-veg | wasm start path: skip the vegetation plugins | check-wasm-planet-poses.cjs forest counts and the native-against-wasm count equality |
| s5-wasm-wrong-radius | wasm planet setup radius 63600 to 6360 | native-against-wasm patch-key equality |
| s5-wasm-not-webgpu | boot without navigator.gpu (the existing blocker path, wasm-boot.js:28-35) | the wasm witness reports blocked, not pass |

The wasm red checks need a wasm rebuild per mutant (3m48s to 217 s recorded per release build, STATUS.md:66, docs/PENDING_VALIDATION.md:336), so they are batched in one window that builds a dev profile once per mutant.

## 5. Window parts and time estimates

Estimates rest only on recorded times: the release wasm build at 217 s and 3m48s (above), the dev native build at 18 to 21 s and the unit suite at 14 s (docs/PENDING_VALIDATION.md:336, an older and smaller tree). Planet render tests and the stage 4 bench were not timed in a record I read, so their parts are marked unmeasured.

| part | work | ticket | estimate |
|---|---|---|---|
| s5-native | cargo build, warnings gates, the two default-launch tests, the pose capture test, the stage 1 to 4 core witnesses unchanged by the planet_scene move | heavy | unmeasured; the planet render tests are the long step |
| s5-wasm-build | WASM_PROFILE=release scripts/build-wasm.sh with the planet modules and assets | heavy | at least 217 s (recorded for the smaller crate); size recorded |
| s5-wasm-poses | serve-wasm, playwriter, five poses with adapter=software, the counts, compare-screenshots against the native PNGs | browser | unmeasured; per pose bounded by the 120 s wait used by check-neon-wasm-models.cjs:56-63 |
| s5-red-native | the native mutants | heavy | unmeasured |
| s5-red-wasm | the wasm mutants, one dev build each | heavy, split if over 30 minutes | number of wasm mutants x one build |
| s5-legacy | the stage 4 legacy part for files changed (globe.rs, terrain.rs, layers.rs, species.rs, trees_render/assets.rs, main.rs) | heavy | as the stage 4 legacy part |

The bench (bench-planet) reruns because globe::register moves into planet_scene; it is time-measuring, so it runs with GATE_EXCLUSIVE=1 in its own part.

## 6. Open questions

- Q1. The default launch already opens the preset planet (section 1.1). Is the stage 5 'opens the full-scale planet' satisfied by the preset (radius 63600, dec:radius, docs/planet-parity.md:9), or does the owner want the engine default 6,360,000 (planet-radius.js:1) as the no-argument planet? This design keeps the preset.
- Q2. On wasm every task runs on the main thread (bevy_tasks single_threaded_task_pool.rs:33). Is a measured main-thread build count acceptable for the wasm viewer, or must stage 5 add web-worker threads (wasm atomics and a nightly build-std toolchain), which this host has not built?
- Q3. The CPU LUT bake (16,384,000 plus 3,145,728 steps, computed) runs before the first sky frame on wasm. What first-frame delay is acceptable before the baked bytes are shipped instead?
- Q4. The shoreline water and land share thresholds: the existing 25 percent water floor (check-planet-screenshot.cjs:8) was set for the old census scene. Take it, or record the first measured shares and set the bar from them?
- Q5. PSNR 30 dB native against wasm across two software rasterisers: confirm, or set from the first measured pair.
- Q6. Can the browser fallback adapter in playwriter run the vegetation compute and indirect path at all? Headless Chromium here failed with a Vulkan surface error (STATUS.md:73-75); the adapter=software path passed for glTF models (check-neon-wasm-models.cjs:9-16) but has never run a compute pass. If it cannot, the wasm row is recorded open with the failure, as the peer rules say.
- Q7. The tree cull uses all 8 storage buffers allowed by the WebGPU default limit (cull.rs:267-277, limits.rs:434). Should Phase A merge two buffers to leave headroom, or request a higher limit from the adapter?
- Q8. The wasm artifact is 31 MB today without the planet; the planet assets add 17 MB fetched at start. Is a size budget wanted?
