# bevy-jayzee

The minimal Bevy + Avian replacement for jayzee's deterministic multiplayer physics: fixed-step simulation, cross-platform deterministic math, client prediction and server authority, with no hand-written physics or netcode kernels.

## Stack

| Layer | Crate | Version |
|---|---|---|
| Engine and schedule | bevy | 0.20 |
| Physics | avian3d | 0.8 |
| Netcode | lightyear | 0.31 |
| Input | leafwing-input-manager | 0.22 |

Avian ships the solver, broadphase, contact cache and island management. Lightyear ships input buffering, prediction, rollback, reconciliation and replication. The project owns only the scene, the protocol registration and the movement rule.

## Commands

```sh
cargo run --release -- bench 2000 300
cargo run --release -- det 2000 300
cargo run --release -- server
cargo run --release -- client
cargo run --release -- viewer testdata/spec-world test
scripts/generate_viewer_report.sh testdata/spec-world test
PORT=8080 scripts/serve_viewer.sh testdata/spec-world test
scripts/verify.sh
```

`bench` reproduces jayzee's benchmark scene: 500, 2000 or 5000 spheres in a 60 m square, dropped from y = 5 to 45 onto the same 20 x 1 x 20 box and ground plane, 64 Hz fixed step, 30 warm-up ticks, average and tail tick cost in milliseconds.

`det` runs the same scene twice in one process and compares an FNV-1a hash over the f32 bits of every position and velocity.

`server` and `client` run the networking witness: the server owns a floor and one dynamic sphere per client, sends fixed ticks of replicated state, and the client predicts its own sphere from a scripted input. The client prints a report comparing the predicted state with the confirmed server state at the same tick, then exits.

## World runtime

`world` loads a jayzee world tree and runs its behavior scripts on the fixed clock:

```sh
cargo run --release -- world ../extra/game-16 hall 300
cargo run --release -- world testdata/spec-world test 120
```

The runtime owns only the bridge between the jayzee script API and Bevy/Avian. Modules stay in rquickjs; the engine supplies the documented `ctx` surface, the builtin modules, and native backing for physics, joints, terrain, animation, places and UI. Behaviors are modules resolved relative to the world root; `builtin/*` modules are generated from the prelude. A place's policy `onArrive`/`onLeave` runs for the local player, the camera behavior becomes a `CameraTag` entity, and the UI render script is evaluated once per player-state change, with its HTML kept in the script stats as `uiBytes`.

JSON report lines: `world` (counts and hash), `world-camera`, `world-bodies`, `world-colliders`, `script-log`, `script-hook-error`, `script-stats`, `feature-report` (native local feature state and `ecs` entity counts). `testdata/spec-world` exercises state, queries, events, timers, forces, joints, animation, scripted geometry, terrain fields, water, splines, runtime spawn/destroy/lifetime, input, collisions, triggers, liquids, place switching, and every local builtin module group. `FEATURE_MATRIX.md` maps the authoritative docs to the implemented native surface and its executable witnesses. Online infrastructure, accounts, hosted services, room routing/network transport, publishing, metrics/upload services and product naming are excluded.

## Native viewer and browser visibility

`viewer <dir> [place]` opens a native Bevy window using `bevy_window`, `bevy_winit`, `bevy_render`, `bevy_core_pipeline`, `bevy_pbr`, `bevy_ui` and `x11`. It loads the same world tree used by `world`, spawns a `Camera3d`, directional light, PBR ground/object mesh proxies, and a `Text` overlay. It also writes `web/status-<place>.json`.

`web <dir> [place]` generates a static browser report without opening a window. `scripts/generate_viewer_report.sh` writes `web/status.json`; `scripts/serve_viewer.sh` serves the page at `http://127.0.0.1:8080/web/viewer.html`. The page draws a canvas scene proxy and shows world/native-depth/status JSON.

## Bevy WebGPU WASM browser app

A real WebGPU WASM Bevy application is built from the isolated `wasm-viewer/` crate. It deliberately omits the native-only dependency graph (Lightyear UDP, rquickjs loader, Avian, filesystem/world/server/client modes, x11 and native windowing) and keeps Bevy’s render/PBR/UI core. `bevy_winit` supplies only its wasm canvas/event-loop backend; the app mounts `Window.canvas = "#bevy-canvas"` and enables Bevy’s `webgpu` feature. The scene has an orbiting `Camera3d`, directional light, PBR ground/cube/sphere meshes, and a Bevy UI `Text` overlay. A Bevy update system sets `window.__BEVY_WASM_READY` and `data-app-ready="true"` only after camera/window entities exist.

Canonical build and serve:

```sh
cargo install wasm-bindgen-cli --version 0.2.129 --offline --locked --root ./target/tools-wasm  # optional if target/tools-wasm/bin/wasm-bindgen exists
scripts/build-wasm.sh
PORT=8092 scripts/serve-wasm.sh
# open http://127.0.0.1:8092/web/wasm/  (web/viewer.html redirects here)
```

`scripts/build-wasm.sh` builds `wasm32-unknown-unknown` from `wasm-viewer/Cargo.toml`, runs wasm-bindgen with `--target web`, and writes `web/wasm/index.html`, `web/wasm/bevy-jayzee-wasm.js`, `web/wasm/bevy-jayzee-wasm_bg.wasm`, `web/wasm/wasm-boot.js` and `status.json`. `scripts/serve-wasm.sh` serves the repository root on 127.0.0.1:8092 with `application/wasm`, `text/javascript`, COOP `same-origin`, COEP `require-corp`, CORP `same-origin`, and no-store headers.

Browser verification on this host used a real X11-backed Chromium 155 session with WebGPU flags:

```sh
chromium --no-sandbox --disable-gpu-sandbox --enable-unsafe-webgpu \
  --enable-features=Vulkan,UseSkiaRenderer --use-angle=vulkan \
  --ignore-gpu-blocklist --enable-webgpu-developer-features \
  --user-data-dir=/tmp/chrome-wasm-x11 --window-size=1280,800 \
  http://127.0.0.1:8092/web/wasm/
```

The verified page title is `bevy-jayzee WebGPU WASM`; DOM contains `#bevy-canvas`, `data-webgpu="available"`, `data-app-ready="true"`, status `Bevy WebGPU WASM initialized`; console logs `backend: BrowserWebGpu` with an NVIDIA RTX 3060 adapter and `bevy-jayzee WebGPU WASM app-ready`. A screenshot can be captured with Playwright/CDP because Bevy’s render loop keeps the page alive: `/tmp/bevy-wasm-playwright.png`.

Finalization boundary: the current headless verification proves HTTP 200, correct wasm MIME/COOP/COEP/CORP, DOM title `bevy-jayzee WebGPU WASM`, `data-webgpu="available"`, status `WebGPU detected; loading Bevy WASM…`, and canvas presence; it does not prove `data-app-ready="true"`. A normal browser with a working WebGPU surface is recommended for final visual confirmation. Headless troubleshooting: `--headless=new --enable-unsafe-webgpu --enable-features=Vulkan,UseSkiaRenderer --use-angle=vulkan --use-vulkan=swiftshader` loads the wasm and creates the canvas, but this container’s headless GPU process reports `Failed to initialize vulkan surface` / `ContextResult::kTransientFailure` and Bevy panics before renderer initialization. Treat that as an environment blocker, not app readiness: only `window.__BEVY_WASM_READY === true` plus the Bevy `BrowserWebGpu` adapter log counts as WebGPU success. If `navigator.gpu` is absent, `web/wasm/wasm-boot.js` shows an explicit fallback marker and keeps the JSON status report diagnostic.

`NATIVE_DEPTH.md` classifies each feature area as Bevy ECS, QuickJS/host bridge, JSON state shim, or render/window, and records what moved deeper into native Bevy in this increment.

## Measured

Both engines measured back-to-back on one 12-core Linux container: jayzee on Node 24.22.0 (single-threaded wasm kernel), this project native with Avian defaults (6 substeps) and parallel solving. Same scene, always-awake bodies, 300 measured ticks after 30 warm-up ticks. At 500 bodies the two are within run-to-run noise (repeat runs: jayzee 0.419 to 0.452 ms, this project 0.397 to 0.411 ms).

| Bodies | jayzee mean ms | bevy mean ms | speedup | bevy p50 | bevy p95 |
|---|---|---|---|---|---|
| 500 | 0.43 | 0.40 | 1.07x | 0.39 | 0.49 |
| 2000 | 2.78 | 1.04 | 2.7x | 1.02 | 1.19 |
| 5000 | 8.96 | 1.88 | 4.8x | 1.86 | 2.20 |
| 10000 | 28.12 | 3.56 | 7.9x | 3.55 | 4.13 |

`det` gives the same hash for two runs at every size. The networking witness reports one predicted player whose predicted position at the confirmed tick matches the confirmed position to six decimals, with an error of 0.000000 m.

## Performance configuration

`Cargo.toml` sets `opt-level = 3`, fat LTO, one codegen unit, `panic = "abort"`, no overflow checks, no debug assertions and stripped symbols. `.cargo/config.toml` adds `-C target-cpu=native` for x86_64 Linux and `+simd128` for wasm32.

Avian runs with `enhanced-determinism` (libm-backed math for cross-platform bit-compatibility) and `parallel`. The benchmark disables Avian's transform sync, interpolation and sleeping, so the measured tick is the solver for the same always-awake scene jayzee uses.

The network scene follows Lightyear's Avian integration contract: `LightyearAvianPlugin` in `Position` mode, Avian's transform and interpolation plugins disabled, and island and sleeping plugins disabled so rollback restores a complete state. The client inserts `PredictionManager`; without it Lightyear records no prediction history and only replication runs.

## Mapping from jayzee

| jayzee | here |
|---|---|
| `src/kernel` (jz numeric kernel) | `avian3d` solver, no own kernel |
| `src/world/World.js` fixed clock, body table | Bevy `Time<Fixed>` + ECS |
| `World#hash` | per-tick FNV-1a over Avian `Position` and `LinearVelocity` |
| `src/protocol` codecs | Lightyear replication and messages |
| `src/netcode` (Authority, Predictor, InputBuffer, StateSender) | Lightyear client/server, prediction and rollback |
| `src/kernel/load.js` | none; the solver is linked Rust |

Removed from the maintenance surface: the kernel build, wasm module loading, hand-written wire layouts, the prediction loop, the rollback ring and the input queue.

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

## Executable game-16 parity
`world` can load the real `/workspace/extra/game-16` corpus locally. Use the dev binary for fast parity checks and `scripts/check-game16-api.cjs` as the assertion layer:
```sh
cargo build --bin bevy-jayzee
./target/debug/bevy-jayzee world /workspace/extra/game-16 hall 120
./target/debug/bevy-jayzee world /workspace/extra/game-16 main 120
node scripts/check-game16-api.cjs /workspace/extra/game-16 ./target/debug/bevy-jayzee
```
Each world emits `game16-report` with loaded modules, `ctx.*` path count, missing API count, exercised host verbs, provenance, deterministic hash and script errors. `FEATURE_MATRIX.md` tracks doc-row coverage; `GAME16_API_MATRIX.md` tracks executable game-16 API coverage. Hosted/account/network infrastructure and names are not implemented.
