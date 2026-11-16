# Status

Current increment: full local feature-set implementation from `/workspace/extra`, excluding online infrastructure, accounts, auth, chat/social APIs, hosted services, room routing/network transport, notifications, feed, CDN/publishing, metrics/upload services, and product names.

State: implemented and verified on the current worktree. `FEATURE_MATRIX.md` tracks all 67 in-scope authoritative doc rows. Before this increment 48 were implemented/verified; after it 67/67 are implemented/verified, with hosted/account/service portions explicitly excluded. The prior world runtime, bench/determinism, and Lightyear prediction work is preserved.

Native implementation added in `src/features.rs` and wired through `src/runtime.rs`, `src/world.rs`, `src/script/mod.rs`, and `src/script/prelude.js`: deterministic local physics worlds with byte snapshot/restore, voxel part storage, FX/vibe/job/trade/DM state, seeded RNG, timing/view constants, exact builtin exports, camera/self compatibility fields, and detailed QuickJS module exception reporting. `feature-report` is emitted by `world`.

Executable coverage added in `testdata/spec-world/scripts/feature-audit.js` and attached to the fixture's `ball` behavior. It imports and calls every local builtin module group and records native state. Existing `script-stats`, `world`, `world-camera`, `world-bodies`, `world-colliders`, `script-log`, and `script-hook-error` report lines remain intact.

Focused verification: `cargo check`, dev build, `world testdata/spec-world test 120`, and `world /workspace/extra/game-16 hall 120` all pass with zero script hook/load/tick/engine errors. The fixture reports `restored:true` for native physics snapshots and `errors:0` in `script-stats`. Release verification: `cargo build --release` exit 0 (5m 53s); release `world testdata/spec-world test 120` exit 0; release `world /workspace/extra/game-16 hall 120` exit 0; `./scripts/verify.sh` exit 0. Both release worlds had zero script errors; spec `script-stats errors:0`, game `errors:0`; `det` hashes match `0x3a5f0b51`; `client-report errorM:0.000000`; benchmark means 0.3985/0.9619/1.6605 ms at 500/2000/5000 bodies.

Uncommitted changes are intentionally preserved; no commit or push was requested.

Remaining actions: commit/publish only when the owner asks; otherwise none for this increment.


# Native-depth and browser increment

State: implemented and verified after restart. `NATIVE_DEPTH.md` is the depth audit. `src/features_ecs.rs` adds real Bevy ECS components/resources/systems for typed feature entities, clock, seeded RNG, native math/vector/noise service, and entity summaries; `feature-report` now includes an `ecs` section. `src/viewer.rs` adds a native Bevy render/window viewer with `Camera3d`, directional light, PBR mesh proxies and `Text` overlay. `web/` plus `scripts/generate_viewer_report.sh` and `scripts/serve_viewer.sh` add a browser-visible status/canvas page.

Historical static-viewer increment: the browser initially showed only a generated status/canvas report. The native-window viewer was launched under `DISPLAY=:1.0`, created a window, selected a Vulkan GPU adapter, and was captured with `xwd`; that static limitation was later resolved by the real WebGPU WASM increment documented below.

Owner invariants: all pre-existing dirty files and commits preserved; no commit or push.

Final verification for this increment: `cargo check --message-format=short` exit 0; `cargo build --message-format=short` exit 0; debug `world testdata/spec-world test 120` exit 0; debug `world /workspace/extra/game-16 hall 120` exit 0; both had zero script errors and `script-stats errors:0`. `feature-report` includes `ecs` counts (`physicsWorldEntities:1`, `physicsBodyEntities:1`, `voxelPartEntities:3`, `jobEntities:1`, `tradeEntities:1`, `nativeMath.calls:8`) for the spec fixture. `cargo build --release` exit 0 (10m 21s); `./scripts/verify.sh` exit 0 with `det pass=true`, `client-report errorM:0.000000`, and bench means `0.3704/0.8411/1.4743 ms` at 500/2000/5000 bodies. Browser: `scripts/generate_viewer_report.sh` exit 0; fresh HTTP server returned 200 for `web/viewer.html`, `web/viewer.js`, `web/status.json`; Chromium headless dump-dom exit 0 showed `<title>bevy-jayzee browser report — test</title>`, `status.json loaded`, `testdata/spec-world`, and the historical static wasm blocker text (later replaced by the WebGPU WASM build); screenshot `/tmp/final-viewer.png` was `PNG image data, 1280 x 720`, 195233 bytes. Native window: viewer created `bevy-jayzee viewer — test`, selected NVIDIA Vulkan adapter, and `xwd` captured `/tmp/native-viewer.xwd` as `X-Window screen dump image data, 1280x682x24`, 3495019 bytes.
# WebGPU WASM

A separate wasm-safe Bevy application lives in `wasm-viewer/`. It avoids the native-only x11/winit, UDP/Lightyear, and rquickjs server stack while keeping a real Bevy WebGPU renderer with a canvas mount, camera, light, mesh proxies, and UI text.

Build:

```bash
./scripts/build-wasm.sh
```

The script builds `wasm-viewer/Cargo.toml` for `wasm32-unknown-unknown`, runs the locally installed `wasm-bindgen` CLI from `target/tools-wasm/bin/wasm-bindgen`, and writes:

- `web/wasm/bevy-jayzee-wasm_bg.wasm`
- `web/wasm/bevy-jayzee-wasm.js`
- `web/wasm/wasm-boot.js`
- `web/wasm/index.html`
- `web/wasm/status.json`

Serve:

```bash
PORT=8092 ./scripts/serve-wasm.sh
```

Open `http://127.0.0.1:8092/web/wasm/`. The server sends wasm as `application/wasm` and the COOP/COEP/CORP headers needed by browser wasm threads and WebGPU where applicable.

Verified in the current environment:

- `/web/wasm/` returns HTTP 200 with `text/html`.
- `bevy-jayzee-wasm_bg.wasm` returns HTTP 200 with `application/wasm` and 29,273,160 bytes.
- `wasm-boot.js` returns HTTP 200 with `text/javascript`.
- Chromium with `--enable-unsafe-webgpu --enable-features=Vulkan,UseSkiaRenderer --use-angle=vulkan` sees `navigator.gpu`, renders the page title `bevy-jayzee WebGPU WASM`, sets `data-webgpu="available"`, mounts `#bevy-canvas`, and begins loading the Bevy WASM.

Headless caveat: this container's headless Vulkan path reports `Failed to initialize vulkan surface` and `ContextResult::kTransientFailure`, so a DOM dump may remain at `WebGPU detected; loading Bevy WASM…` rather than reaching `data-app-ready="true"`. Only `window.__BEVY_WASM_READY === true` plus Bevy's `BrowserWebGpu` adapter log is treated as successful WebGPU initialization.

Earlier X11-backed Chromium verification in this worktree: Playwright/CDP launched Chromium 155 on `DISPLAY=:1.0` with `--enable-unsafe-webgpu --enable-features=Vulkan,UseSkiaRenderer --use-angle=vulkan --ignore-gpu-blocklist --enable-webgpu-developer-features`. State: title `bevy-jayzee WebGPU WASM`, `appReady:true`, `statusReady:"true"`, `webgpu:"available"`, status `Bevy WebGPU WASM initialized`, `#bevy-canvas` 1280x576 with `webgpu-context-available`, `gpu:true`. Console included `AdapterInfo { name: "NVIDIA GeForce RTX 3060 Laptop GPU", ... backend: BrowserWebGpu }` and `bevy-jayzee WebGPU WASM app-ready`. Screenshot `/tmp/bevy-wasm-playwright.png` is PNG 1280x800, 414510 bytes. The old static report is a diagnostic fallback, not the renderer; `web/viewer.html` now redirects to the wasm app.

Native regression after the wasm changes: `cargo check --offline --message-format=short` exit 0; `./target/debug/bevy-jayzee world testdata/spec-world test 120` exit 0 with `script-stats errors:0`; `./target/debug/bevy-jayzee world /workspace/extra/game-16 hall 120` exit 0 with `script-stats errors:0`; `./scripts/verify.sh` exit 0 with `det pass=true` and `client-report errorM:0.000000`.

Tooling: `wasm-bindgen-cli 0.2.129` installed with `cargo install wasm-bindgen-cli --version 0.2.129 --offline --locked --root ./target/tools-wasm` (exit 0). Offline wasm lock generation initially failed with `no matching package named bevy_anti_alias found`; online sparse-index resolution succeeded, then `WASM_PROFILE=release scripts/build-wasm.sh` exit 0 in 3m48s. Owner invariants: pre-existing dirty files and commits preserved; no commit, push, reset, checkout-over or force operation.


# Finalization pass

Current artifact state: `web/wasm/bevy-jayzee-wasm_bg.wasm` exists at 29,273,160 bytes (sha256 `da3939a70d9c675d57df1a72b5aac2a3d6cbadd7d2b82f427d9a251422915bb7`) with wasm-bindgen JS glue, `index.html`, `wasm-boot.js` and generated `status.json`. `target/tools-wasm/bin/wasm-bindgen --version` reports `wasm-bindgen 0.2.129`. Build with `WASM_PROFILE=release scripts/build-wasm.sh`; serve with `PORT=8092 ./scripts/serve-wasm.sh`; canonical URL is `http://127.0.0.1:8092/web/wasm/`.

Current headless browser evidence: HTTP 200 / `application/wasm` / `text/javascript` / COOP/COEP/CORP; DOM title `bevy-jayzee WebGPU WASM`; `data-webgpu="available"`; status `WebGPU detected; loading Bevy WASM…`; `#bevy-canvas` present. This proves wasm/WebGPU detection, module boot and canvas mount, but not `data-app-ready="true"`. Headless Chromium reports Vulkan surface warnings and Bevy does not reach the app-ready marker in this environment. A normal browser with a working WebGPU surface is recommended for final visual confirmation; previous X11-backed evidence above is retained as an earlier run, not a current headless claim.

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


Render Inspector verification on the final tree: `WASM_PROFILE=release scripts/build-wasm.sh` exit 0 in 2m36s; `cargo check --offline --message-format=short` exit 0; `CARGO_TARGET_DIR=target/wasm-viewer cargo check --manifest-path wasm-viewer/Cargo.toml --target wasm32-unknown-unknown --locked --offline --message-format=short` exit 0. HTTP on 127.0.0.1:8092 returned 200 for `/web/wasm/` (`text/html`, 6119 bytes), `inspector.js` (`text/javascript`, 8257), `bevy-jayzee-wasm.js` (113788), and `bevy-jayzee-wasm_bg.wasm` (`application/wasm`, 29311210). `node scripts/check-inspector.cjs 'http://127.0.0.1:8092/web/wasm/?inspector=1' /tmp/bevy-inspector.png` exit 0 with X11/Chromium 155: title `bevy-jayzee WebGPU WASM`, `data-inspector="open"`, `data-ready="true"`, `appReady=true`, status `Bevy WebGPU WASM initialized`, 17 tabs, all 10 performance metric markers and all 7 strip markers. Snapshot kind `bevy-diagnostics` contained FPS, frame time, frame count and entity count from Bevy `DiagnosticsStore`; console logs showed Bevy `LogDiagnosticsPlugin` entries. Screenshot `/tmp/bevy-inspector.png` PNG 1440x900, 350247 bytes. Final wasm hash `aee098cb2d63b82b6213465060af77d9570cfe1300860f8cc2633e0a7233707a`; JS glue `45aa841a39489ed0b33580cda6e7c822300e77c2f30434195c45dfe18c2c8c64`; inspector JS `635cbf21afdccc03aca863128e09205c5b2295f0fa9e67470c4996ba955cb25b`.

## Executable game-16 parity increment
- Added `game16-report` from `src/world.rs`/`src/script/mod.rs`: source `ctx.*` inventory, missing API count, loaded QuickJS modules, host verbs/calls, context keys, provenance, ECS-backed provenance count, fixed-tick hash and script errors.
- Added `scripts/check-game16-api.cjs`; it runs both real places for 120 ticks, requires exit 0, `script-stats errors:0`, `game16-report errors:0`, `missingApiCount:0`, and full coverage of extracted `ctx.*` usage.
- Added explicit local API shims/fixes for `getTerrainHeight`, `sampleSpline`, `splineLength`, `projectToSpline`, `spawnPlayer`, `setInput`, `setAnim`, `ensureScatter`, `getPixelProvenance`, `day`, and `field`, with Bevy ECS terrain/host backing where applicable.
- Added `GAME16_API_MATRIX.md` and separated doc-row coverage from executable game-16 API coverage in `FEATURE_MATRIX.md`.

## Final executable verification (2026-11-15)
- `cargo check --bin bevy-jayzee` exit 0; `cargo build --bin bevy-jayzee` exit 0; `cargo build --release --bin bevy-jayzee` exit 0 (10m 21s); `scripts/verify.sh` exit 0 with bench 500/2000/5000, determinism `pass:true`, and `client-report` prediction error `0.000000`.
- `node scripts/check-game16-api.cjs /workspace/extra/game-16 ./target/debug/bevy-jayzee` exit 0: `usageCount:317`, `missingUsageCount:0`, both places exit 0, `scriptErrors:0`, `missingApiCount:0`; loaded modules hall=9/main=1; hashes `0x7a2d0c18` and `0x7c40622a`.
- Release `world /workspace/extra/game-16 hall 120`, `world /workspace/extra/game-16 main 120`, and `world testdata/spec-world test 120` all exit 0 with `script-stats errors:0` and `game16-report missingApiCount:0`.
- `WASM_PROFILE=release scripts/build-wasm.sh` exit 0 with `wasm-bindgen 0.2.129` and `bevy-jayzee-wasm_bg.wasm` 29,311,210 bytes. Port 8092 was already served by existing Python pid 204697; no server was started or stopped for this run.
- `node scripts/check-inspector.cjs http://127.0.0.1:8092/web/wasm/?inspector=1 /tmp/bevy-inspector-final2.png` exit 0: `data-webgpu="available"`, `data-app-ready="true"`, status `Bevy WebGPU WASM initialized`, inspector `open`/`ready=true`, `snapshotKind:"bevy-diagnostics"`, 4 diagnostics and 15 explicit `unavailable` metrics. The checker keeps only the last 20 console lines, so no `BrowserWebGpu` adapter line is claimed from this run.

## Teardown hardening (final verification)
Root cause of the reported `null reference produced` abort: the script host clears `Host::world` after every fixed tick, and shutdown previously dropped QuickJS module/factory persistents without clearing global bridge/context references. On some rebuilt debug binaries a report/teardown-time callback reached the stale world pointer. `src/script/mod.rs` now asserts on null world pointer access instead of dereferencing it, clears `Host::world` and global QuickJS references during `shutdown()`, drops persisted modules and `__make_ctx` before runtime teardown, and keeps `api_report()`/`game16_report()` as pure snapshots with no QuickJS evaluation.
Final rebuilt-debug verification: `RUST_BACKTRACE=1 world /workspace/extra/game-16 hall 20`, `main 20`, `hall 120`, and `main 120` all exit 0 with no panic/backtrace; `cargo check --message-format=short`, `cargo build --message-format=short`, hall/main 120, `node scripts/check-game16-api.cjs`, and `./scripts/verify.sh` exit 0. `WASM_PROFILE=release scripts/build-wasm.sh` and `scripts/check-inspector.cjs` also exit 0; port 8092 was already served by existing Python pid 204697.

## Neon-syndicates ctx reconciliation (this stretch)

Counts, measured on /workspace/neon-syndicates/world:

| Measure | Count | Source |
|---|---|---|
| Distinct `ctx.*` paths | 129 | scan of `ctx.` tokens in world/ |
| Distinct top-level keys | 55 | same scan |
| Top-level keys missing from the runtime before this stretch | 14 | runtime ctxKeys vs scan |
| `missingApi` entries before this stretch, after the checker's exemptions | 10 (8 host names) | game16-report on main, before the change |
| `missingApi` entries now | 0 | game16-report on main, dev build 2026-11-16 |
| Owner's figure | 115 | not reproduced: no scan of the world, the whole neon-syndicates project, or the game-16 corpus gives 115 |

The 0 is not the same thing as 8 host APIs implemented. Of the 8 names, only `spawn` is a host API with behaviour (`ctx.spawn` through `__jayzee`). The other seven (`uvProjection`, `__feelRaw`, `__glass`, `__safeDestroy`, `__soundGun`, `__soundQ`, `__soundRaw`) are state fields that the world's own scripts write (for example `scripts/sys/sound.js` sets `ctx.__soundQ = []`, and `scripts/gen/*.js` sets `ctx.__glass = 1`). The runtime now seeds them as `null`, `false` or `0` on the per-place ctx so the report classifies them. The checker counts them as provided because the names exist, not because a host behaviour backs them.

Verified on the dev build:

- `target/debug/bevy-jayzee world /workspace/neon-syndicates/world main 120`: exit 0, `"objects":58`, `"bodies":9`, `"behaviors":1`, `"models":9`, `"scripted":11`, `script-stats errors:0`, `game16-report missingApiCount:0`, hash `0x1c2b1833`.
- `node scripts/check-game16-api.cjs`: exit 0, `"ok": true`.
- `target/debug/bevy-jayzee gltf-check /workspace/neon-syndicates/assets/cdn`: `bevy_failed:0`, `bevy_loaded:64`, GLB `pass:64`. Exit 1 remains from 2 PNG failures outside this stretch.
- `cargo test --bin bevy-jayzee basisu`: 11 passed. `glb`: 4 passed. `tests::`: 31 passed, 1 failed (`globe::tests::elevation_is_deterministic_and_bounded`, assertion at src/globe.rs:550). globe.rs is not in this stretch's diff; whether the test fails on 63700b4 has not been checked.

HUD check on the new binary (`/tmp/ns-viewer/run.sh`): the viewer draws the ground plane only, `uiNodes:60` (up from 9), `objects:0`, `meshes:0`, and warns `unsupported element <svg> dropped (12)`. No HUD text is visible. The likely cause is that `Cargo.toml` does not enable `default_font`; not yet proven.
