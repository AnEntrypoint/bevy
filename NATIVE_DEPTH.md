# Native depth audit

This file classifies how deeply each feature area uses Bevy before and after the native-depth/browser increment. It is additive to `FEATURE_MATRIX.md`: that file tracks documented API coverage, this file tracks implementation depth.

Classes:

- **A. Bevy ECS**: real Bevy `Component`/`Resource`/systems/plugins, or Avian/Lightyear components and systems.
- **B. QuickJS/host bridge**: script-facing compatibility implemented in `src/script/mod.rs` and `src/script/prelude.js`.
- **C. JSON state shim**: plain `serde_json::Value`/maps in a data-only resource, without ECS entity identity or systems.
- **D. Render/window**: Bevy render/winit/window/camera/UI systems and entities.

## Baseline after restart

Before this increment the tree had only `bevy` features `multi_threaded`, `bevy_log`, `bevy_state`, `bevy_asset`, `std`, `async_executor`. `world`, `bench`, `det`, `server` and `client` were headless. There was no `Camera`, `Camera3d`, `Window`, `WinitPlugin` runner, mesh/material asset path, Bevy UI, or browser artifact.

| Area | Baseline | Gap / target |
|---|---|---|
| Core world objects, transforms, physics | **A** in `src/world.rs`, `src/runtime.rs`; Avian `Position`, `Rotation`, `Collider`, `RigidBody`, fixed-step systems | Keep; expose these entities to a viewer as mesh proxies. |
| Terrain, water, fields, splines, animation, forces, camera view, audio transport | **A** in `src/runtime.rs` resources/components and fixed systems | Keep; add render-side camera and UI display. |
| Bench/determinism/network | **A** in `src/bench.rs`, `src/client.rs`, `src/server.rs`, `src/protocol.rs` | Keep headless and unchanged. |
| Script behaviors / ctx / builtin imports | **B** in `src/script/mod.rs` + `src/script/prelude.js` | Keep API compatibility, but move selected calculations/state into Rust host verbs. |
| `FeatureState` physics worlds, voxels, FX, vibe, jobs, trades, DM, RNG, timing | **C**: one `#[derive(Resource)]` data bag of `BTreeMap`/`Vec<Value>` in `src/features.rs`; one system advances its tick | Add true ECS components/entities/resources where practical, while keeping the existing `feature-report` and host verbs stable. |
| Math/vector/easing/noise/spline/tween builtins | **B/C**: mostly JavaScript formulas in `prelude.js` | Add native Rust service verbs for representative calculations and route builtins through them without changing return shapes. |
| Camera/UI/rendering | **C/B**: JSON camera state, HTML string `uiBytes`, no window or camera entity | Add Bevy `DefaultPlugins`-style render features, native window `viewer` mode, `Camera3d`, light, ground/object mesh proxies, and `Text` overlay. |
| Browser/WebGPU | historical static page only | Build a separate wasm-safe Bevy WebGPU app with real `.wasm` and JS glue; keep the static page only as fallback. |

## Implemented in this increment

| Area | After | Evidence |
|---|---|---|
| Feature state | **A + C**: `FeatureState` remains for report compatibility, but `NativeFeatureRegistry` owns typed ECS components spawned as real entities: `PhysicsWorldEntity`, `PhysicsBodyComponent`, `VoxelPartEntity`, `FxEntity`, `VibeEntity`, `JobEntity`, `TradeEntity`, `DmMessageEntity`; `sync_feature_entities` and `tick_feature_entities` are Bevy systems. | `src/features.rs`, `src/runtime.rs`, `feature-report` now includes entity counts. |
| Timing/random | **A**: `NativeClock` and `NativeRandom` resources with systems; host verbs read them. | `src/features.rs`, `src/runtime.rs`, `src/script/mod.rs`. |
| Native calculation services | **A/B**: `NativeMathService` resource and Rust host verbs for math/vector/easing/noise/spline/tween-compatible calculations; prelude keeps API shape and delegates selected calculations. | `src/features.rs`, `src/script/mod.rs`, `src/script/prelude.js`. |
| Viewer/renderer | **D**: `viewer` mode uses Bevy `DefaultPlugins` subset with `bevy_window`, `bevy_winit`, `bevy_render`, `bevy_core_pipeline`, `bevy_pbr`, `bevy_ui`, `bevy_ui_render`, `x11`; spawns `Camera3d`, directional light, PBR ground/object proxies and `Text` overlay driven by a loaded world tree. | `src/viewer.rs`, `src/main.rs`, `Cargo.toml`. |
| Browser-visible page | **D-lite** in the earlier static increment: generated `web/status.json` and a canvas status scene. The current browser path is the real WebGPU WASM app; the static report is retained only as fallback. | `web/`, `scripts/serve_viewer.sh`, `scripts/generate_viewer_report.sh`, `web/wasm/`. |
| Headless compatibility | **A/B/C** unchanged where needed; all prior report lines preserved. | `cargo check`, `world` conformance, `scripts/verify.sh`. |

## Remaining limits

Historical note: the earlier static-viewer increment had no wasm/WebGPU artifact. The current increment adds a real wasm build through `wasm-viewer/`, `scripts/build-wasm.sh`, and local `wasm-bindgen` CLI 0.2.129; see the WebGPU WASM section below.
- Deliberately retained QuickJS as the scripting API boundary and JSON report shapes for compatibility. Replacing every JS helper would require changing the public script surface and is not needed for native engine depth.
- The viewer uses simple mesh proxies from `ObjectSpec`/collider-like dimensions rather than full asset/model rendering; it is a native Bevy visibility path, not a full production renderer.

## WebGPU WASM increment

`wasm-viewer/` is an excluded standalone crate with its own `Cargo.lock`, so the native root dependency graph and lock are preserved. It links Bevy 0.20.1 with `web`, `webgpu`, render/PBR/UI/window features and wasm-only `web-sys`/`js-sys`/`getrandom` dependencies. No `x11` feature, native-only rquickjs loader, Lightyear UDP network transport, Avian physics, or server/client/filesystem mode is compiled into the wasm binary. `bevy_winit` is present for Bevy's wasm canvas/event-loop integration and mounts `#bevy-canvas`; this is not the x11 native window backend.

| Area | After | Evidence |
|---|---|---|
| Browser renderer | **D**: real Bevy WebGPU WASM app with `Camera3d`, directional light, PBR plane/cube/sphere meshes, rotating meshes, orbiting camera and Bevy `Text` overlay. | `wasm-viewer/src/lib.rs`, `web/wasm/`, console `backend: BrowserWebGpu`. |
| Browser readiness | **D**: canvas mount plus `window.__BEVY_WASM_READY` / `data-app-ready="true"` set from a Bevy update system after camera and window entities exist. | `web/wasm/wasm-boot.js`, X11-backed Chromium CDP state and screenshot. |
| Native compatibility | **A/B/C/D** unchanged: native root workspace keeps all previous modes and dependencies. | `cargo check`, two world runs, `scripts/verify.sh` all exit 0. |

Headless Chromium loads the wasm and creates the canvas but cannot finalize this container's Vulkan surface; the explicit fallback text names that blocker, and current finalization does not claim `app-ready=true` from headless DOM. An earlier X11-backed Chromium run with WebGPU flags reached `app-ready=true` on an NVIDIA RTX 3060 `BrowserWebGpu` adapter and produced `/tmp/bevy-wasm-playwright.png`; repeat that in a normal browser for final visual confirmation.

Final headless browser rerun after boot-error handling: `chromium --headless=new --no-sandbox --disable-gpu-sandbox --enable-unsafe-webgpu --enable-features=Vulkan,UseSkiaRenderer --use-angle=vulkan --use-vulkan=swiftshader --virtual-time-budget=12000 --dump-dom` exit 0. DOM title `bevy-jayzee WebGPU WASM`; `#bevy-canvas` present; `#app-status` has `data-app-ready="false"`, `data-blocker="headless WebGPU runtime failed"`, text `WebGPU WASM blocked: headless WebGPU runtime failed`; `#boot-error` text `headless WebGPU runtime failed: Uncaught RuntimeError: unreachable; use a normal browser with a working WebGPU surface`. Console: `Failed to initialize vulkan surface`, `ContextResult::kTransientFailure`, Bevy renderer panic and `Uncaught RuntimeError: unreachable`. Screenshot `/tmp/bevy-wasm-finalization2.png`, PNG 1280x800, 31841 bytes. Do not claim headless `data-app-ready=true`; the only `data-app-ready="true"` occurrence before this explicit marker was a CSS selector, not the live status element.

## Render Inspector

The WebGPU WASM app has a compact browser Render Inspector fed by Bevy’s native diagnostics. Open it with `http://127.0.0.1:8092/web/wasm/?inspector=1`, the visible `Inspector: I` button, or the `I` key; `_` collapses it and `×` closes it. The overlay exposes `data-inspector`, `data-ready`, 17 tab buttons, all requested performance metrics (`DRAWS`, `SHADOW-MAP DRAWS`, `TRIANGLES`, `SHADOW TRIS`, `SHADOW PASSES`, `CASCADE SKIPS`, `ENTITIES`, `MESHES`, `SPRITES`, `TERRAIN`) and top-strip buckets (`FPS`, `MEMQ`, `MEMO`, `DRAW`, `SHOW`, `SIM`, `LAUNCH`) with rolling sparklines.

Bevy-native pieces: `DiagnosticsStore`, `DiagnosticsPlugin` from `DefaultPlugins`, `FrameTimeDiagnosticsPlugin`, `EntityCountDiagnosticsPlugin`, `LogDiagnosticsPlugin`, and `bevy_ui` text in both viewers. The wasm app publishes `fps`, `frame_time`, `frame_count`, `entity_count`, ECS/asset counts, and any `render/*` diagnostics to `window.__BEVY_INSPECTOR_SNAPSHOT` and the hidden `#inspector-data` DOM node. `RenderDiagnosticsPlugin` is present in Bevy 0.20 but is only automatically installed with `tracing-tracy`; WebGPU reports CPU spans only when that feature is enabled, so the inspector labels GPU pass timestamps/frame graph/draw calls/triangles/shadow counters/texture bytes/terrain/physics/multiplayer/jobs/vibes/parameters `unavailable` rather than inventing values. Browser-only fields are clearly labelled: `performance.memory` if exposed, `navigator.gpu`, and `performance.now` from `js_sys::Date::now`.

Canonical checker:

```sh
PORT=8092 ./scripts/serve-wasm.sh
node scripts/check-inspector.cjs 'http://127.0.0.1:8092/web/wasm/?inspector=1' /tmp/bevy-inspector.png
```

Use an X11-backed Chromium with a working WebGPU surface; headless Chromium in this container can load the wasm but its Vulkan surface fails before renderer initialization. The checker uses `playwright-core` in non-headless mode on `DISPLAY=:1.0` and records DOM markers, `window.__BEVY_INSPECTOR_CHECK()`, diagnostics snapshot, console tail, and a screenshot. `window.__BEVY_INSPECTOR_CHECK` is also available for browser automation.


## Game-16 executable depth
The parity witness is executable, not just a documentation claim. `src/script/mod.rs` classifies every `ctx.*` token in `/workspace/extra/game-16` against the `__make_ctx` surface, dynamic Bevy-backed object/place/world/state proxies, typed `FeatureState`/ECS resources, and provenance-labeled deterministic local shims. `src/world.rs` emits `game16-report` after `world`, `script-stats`, and `feature-report`; `scripts/check-game16-api.cjs` asserts both `hall` and `main` run 120 ticks with zero script errors and zero missing API. Bevy ECS remains the owner of object state, transforms, physics components, timing, terrain, input, animation and native feature mirrors; JS is the compatibility boundary only.
