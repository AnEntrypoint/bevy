# Pending validation

Each row names a committed change, the file it touches, and the witness that proves it. A row is green when its witness was run on the tree it was committed in and the output is quoted in the status.

| Commit change | File | Witness | Result |
|---|---|---|---|
| Seeded-random and feature-report unit tests | src/features.rs | `cargo test --bin bevy-jayzee features::tests` | pass: `test result: ok` for the features tests, within `tests::` 31 passed |
| Place program `sim.js`, neon-syndicates ctx keys, `ctx.spawn`, `__wrap` root sync, builtin `tsl` names | src/script/mod.rs, src/script/prelude.js, AGENTS.md | `target/debug/bevy-jayzee world /workspace/neon-syndicates/world main 120` exit 0 with `errors:0`, `missingApiCount:0`, `objects:58`, `behaviors:1` | pass, see STATUS.md section "Neon-syndicates ctx reconciliation" |
| Same change, script side | src/script/mod.rs, src/script/prelude.js | `node scripts/check-game16-api.cjs` exit 0, `"ok": true` | pass |
| Player template skip in world spawn | src/world.rs | `world ... main 120` shows the `player` template spawned (`templates/player.js` in modules) and no bare `local-player` | pass, see run log |
| BasisLZ KTX2 transcode (already on origin/main at 63700b4) | src/basisu.rs, src/glb.rs | `target/debug/bevy-jayzee gltf-check /workspace/neon-syndicates/assets/cdn`: `bevy_failed:0`, `bevy_loaded:64`, `pass:64` | pass (PNG failures outside scope) |
| Viewer HUD check on the new binary | viewer and HUD path | `/tmp/ns-viewer/run.sh` PNGs and `viewer_status` line | open: text still not drawn (see STATUS.md) |
| Globe unit test `globe::tests::elevation_is_deterministic_and_bounded` | src/globe.rs (unchanged in this stretch) | `cargo test --bin bevy-jayzee elevation_is_deterministic` | FAILS: assertion at src/globe.rs:550, not caused by this stretch but not proven pre-existing |

## Round 2 (Phase A, committed locally, not pushed)

| Commit change | File | Witness | Result |
|---|---|---|---|
| Enable Bevy's default font feature | Cargo.toml | viewer screenshot shows the overlay text (`/tmp/ns-viewer/viewer-*.png`) | pending Phase B |
| Bound globe elevation by amplitude | src/globe.rs | `cargo test --bin bevy-jayzee elevation_is_deterministic` passes | pending Phase B |
| Mirror primitive world entities into the viewer | src/viewer.rs | `viewer_status` `objects` equals the headless `objects` count of primitive-backed entities | pending Phase B |
| Palette index fix for mirrored objects | src/viewer.rs | build passes with no errors | pending Phase B |
| Globe noise uses the non-negative fraction | src/globe.rs | `cargo test --bin bevy-jayzee globe::` passes | pending Phase B |
| Viewer reports unmirrored objects | src/viewer.rs | `viewer-status` `unmirroredObjects` plus mirrored `objects` equals the headless `objects` count | pending Phase B |

## Round 2 Phase B results (final tree, dev profile)

- `bevy-jayzee gltf-check /workspace/neon-syndicates/assets/cdn`: `"bevy_failed":0`, `"bevy_loaded":64`, GLB `"pass":64,"total":64`. Exit 1 is the two PNG failures outside this work.
- `bevy-jayzee world /workspace/neon-syndicates/world main 120`: exit 0, `"objects":58`, `"behaviors":1`, `"missingApiCount":0`, hash `0x1c2b1833`.
- `node scripts/check-game16-api.cjs`: exit 0, `"ok": true`.
- `cargo test --bin bevy-jayzee`: `test result: ok. 32 passed; 0 failed`. The globe test passes after the fraction fix.
- HUD: `/tmp/ns-viewer/viewer-1.png` and `viewer-3.png` show the overlay text and the world HUD labels drawn. `viewer-status` reports `objects:29`, `unmirroredObjects:21`, `uiNodes:60`.
- The headless object count varies with the tick (`ticks=1 → 0`, `2 → 38`, `30 → 54`, `120 → 58`, `600 → 50`), so the viewer's 29 + 21 = 50 is a snapshot at its own tick, not a mismatch.

## Round 3 (Phase A, committed locally, not pushed)

| Commit change | File | Witness | Result |
|---|---|---|---|
| Draw the HUD's SVG circles, lines and rects as absolute nodes | src/ui_html.rs | `cargo test --bin bevy-jayzee ui_html` passes (10 tests); HUD `uiNodes` rises from 60 to 102 and `unsupported element <svg>` is gone | pending Phase B |

## Round 3 Phase B results

- `cargo test --bin bevy-jayzee`: `test result: ok. 34 passed; 0 failed`.
- `bevy-jayzee gltf-check /workspace/neon-syndicates/assets/cdn`: `"bevy_failed":0`, `"bevy_loaded":64`. Exit 1 is the two PNG failures outside this work.
- `bevy-jayzee world /workspace/neon-syndicates/world main 120`: exit 0, `"objects":58`, `"missingApiCount":0`, hash `0x1c2b1833`.
- `node scripts/check-game16-api.cjs`: exit 0, `"ok": true`.
- HUD: `uiNodes:102`, warnings now `defs(1)`, `path(1)`, `polygon(9)`.
- Parallelism: `/config/.cargo/config.toml` sets `jobs = 4`. The bin-only rebuild showed one rustc process at each 5-second sample, because only this crate was compiling. The cap of 4 is not confirmed from this build, only the configured value.

## Round 4 (Phase A, committed locally)

| Commit change | File | Witness | Result |
|---|---|---|---|
| Count unmirrored viewer objects that are model references | src/viewer.rs | `viewer-status` `unmirroredModels` `9` with `unmirroredObjects` `21`, and headless `models` `9`, `scripted` `11` | measured in Phase B |

## Round 4 Phase B results

- `cargo test --bin bevy-jayzee`: `test result: ok. 34 passed; 0 failed`.
- `gltf-check`: `"bevy_failed":0`, `"bevy_loaded":64`. Exit 1 is the two PNG failures outside this work.
- `world … main 120`: exit 0, `"objects":58`, `"missingApiCount":0`.
- `check-game16-api.cjs`: exit 0, `"ok": true`.
- HUD `viewer-status`: `"unmirroredModels":9`, `"unmirroredObjects":21`.

## Round 5 (Phase A, committed locally)

| Commit change | File | Witness | Result |
|---|---|---|---|
| Enable `bevy_world_serialization` and `reflect_auto_register`; without the latter the GLB scene spawn panics on unregistered `Transform` | Cargo.toml | viewer run exits 0 with no `panicked` in `viewer.err` and `modelsLoaded:9` | measured in Phase B |
| Draw the 9 GLB model references as `WorldAssetRoot` scenes from the decoded cache; draw the 12 leftover entities as markers; count markers once | src/viewer_world.rs, src/viewer.rs, src/main.rs | `viewer-status` `modelsLoaded:9`, `modelsMissing:[]`, `markers:12`, `objects:50`, `meshes:50` | measured in Phase B |
| Aim the viewer orbit at the first object of kind `VIEWER_FOCUS_KIND` so models are in frame | src/viewer.rs | `/tmp/ns-viewer/viewer-2.png` shows four GLB characters | measured in Phase B |
| Rename the leftover list to `markerIds` (it lists the entities drawn as markers, not unmirrored ones) | src/viewer.rs, src/viewer_world.rs | `viewer-status` key `markerIds` | measured in Phase B |
| SVG `polygon` and `path` as absolute nodes (scanline fill, edge strokes); `oklch()` colours; path-data parser for M/L/H/V/Z (relative and absolute); curves are reported as dropped | src/ui_html.rs | `cargo test --bin bevy-jayzee ui_html` (13 tests), `oklch_colors_convert_to_srgb`, `svg_polygon_and_path_are_drawn_not_dropped`, `svg_path_with_curves_is_reported` | measured in Phase B |

### Round 5 findings recorded, not fixed

- Neon terrain: `terrains:0`. The terrain comes from `scripts/flat-starter-terrain.js`, which Bevy does not execute. Open: it needs the generator run in the place program, not a viewer change.
- Scripted primitives (the `guide-*` ids): drawn as primitives at their correct positions with the default size. Their real geometry is a render shim in the script, so the size is a gap, not a missing draw.
- The "115 missing ctx.* APIs" figure is unreproduced. The measured missing host names are 10: `__feelRaw`, `__glass`, `__safeDestroy`, `__soundGun`, `__soundQ` (with `.length` and `.push`), `__soundRaw`, `spawn`, `uvProjection`. The 115 figure is not reproduced by `scripts/check-game16-api.cjs` or the world run; `missingApiCount` is 0.

## Round 5 Phase B results

- `VIEWER_FOCUS_KIND=model /tmp/ns-viewer/run.sh`: `viewer-status` `objects:50`, `meshes:50`, `markers:12`, `modelsLoaded:9`, `modelsMissing:[]`, `unmirroredObjects:0`, `uiNodes:718`, `uiWarnings:["unsupported element <defs> dropped (1)"]`, `terrains:0`. No `panicked` in `viewer.err`; the one `error` line is the X11 `XSETTINGS` warning. `viewer-2.png` shows four GLB characters; `viewer-1.png` shows the SVG polygon shapes at the top left.
- `bevy-jayzee gltf-check /workspace/neon-syndicates/assets/cdn`: GLB `"pass":64,"total":64`, `"bevy_loaded":64`, `"bevy_failed":0`. Exit 1 is the two PNG failures `texture-concrete-panels.png` and `texture-weathered-wood-planks.png`, outside this work.
- `bevy-jayzee world /workspace/neon-syndicates/world main 120`: exit 0, `"objects":58`, `"behaviors":1`, `"errors":0`, `"missingApiCount":0`, hash `0x1c2b1833`.
- `node scripts/check-game16-api.cjs`: exit 0, `"ok": true`.
- `cargo test --bin bevy-jayzee`: `test result: ok. 37 passed; 0 failed`.
- Parallelism: the bin rebuilds took 31 to 35 minutes and 1 minute 22 seconds when cached. The `jobs = 4` cap in `/config/.cargo/config.toml` is still unconfirmed by sampling.
- Open: neon terrain (`terrains:0`, needs the generator run), and scripted-primitive size (rendered at default size).

## Round 6 (build speed and disk, Phase A and Phase B)

Measurements use `cargo build --timings` through the cargo slot shim. Host load came from other workloads on the same 12-core host (oscraft probe, fx test, rust-lld, Chromium); it is recorded per row, and CPU seconds are the load-insensitive figure.

| Measurement | Config | Wall | CPU user+sys | Host load |
|---|---|---|---|---|
| A cold, fresh target dir | deps opt 3, full debuginfo | 1092 s | not recorded | not recorded |
| A warm no-op | same | 8 s | not recorded | not recorded |
| A warm touch of src/main.rs | same | 158 s | not recorded | not recorded |
| A warm touch, CPU timed | same | 252 s | 105 s | 28 |
| B cold after switching deps to opt 1 | deps opt 1, full debuginfo | 1862 s | not recorded | not recorded |
| B first touch after the switch | same | 880 s | not recorded | not recorded |
| B warm touch, second | same | 520 s | not recorded | 30 |
| B warm touch, CPU timed | same | 365 s | 98 s | 31 to 46 |
| C cold | deps opt 3, `debug = "line-tables-only"` | 1468 s | 5450 s | 41 to 15 |
| C warm touch 1 | same | 12 s | 24 s | 12 |
| C warm touch 2 | same | 18 s | 23 s | 12 to 18 |
| C warm touch 3 | same | 23 s | 21 s | 17 |

Decision: keep C. Deps at opt 1 (B) gave no CPU gain on the warm rebuild (98 s against 105 s) and a slower cold build, so it is not kept. Line-table debuginfo cut the warm rebuild's CPU from about 105 s to about 22 s and the bin from 2.4 GB to 945 MB. The C viewer witness shows the build is still playable: `objects:50`, `modelsLoaded:9`, 58 fps.

Rustc concurrency: 1876 host-wide samples during the cold build gave a maximum of 5 and a mean of 1.9. These counts include other users' compilers, so the `jobs = 4` cap is still not isolated.

| Change | File | Witness | Result |
|---|---|---|---|
| `debug = "line-tables-only"` in the dev profile | Cargo.toml | warm touch 12 to 23 s wall, 21 to 24 CPU s; viewer `objects:50` | measured |
| Reject an empty place argument with a message that names it | src/world.rs | `target/debug/bevy-jayzee world /workspace/neon-syndicates/world "" 120` prints `the place argument is empty; name a place or leave it out` and exits 1 | measured |

Target directory: 82 GB at the start of the session, 96 GB before pruning, 19 GB after pruning (`target/debug` 16 GB, `target/wasm-viewer` 3.5 GB kept for the wasm build, `gltf-cache` and `viewer-cache` kept). Removed: `target/release` (4.0 GB, stale profile) and 5,910 stale dependency files and 2,291 stale fingerprint directories not referenced by the current configuration.

Prune mistake: the first prune also removed fingerprint or artifact files the current build still used. The verification no-op then recompiled 274 crates in 25 min 32 s. Rebuilding restored the state, and a later no-op build took 2 s with 0 crates compiled. The prune rule was: keep only artifacts whose hash appears in the `filenames` of the `--message-format=json` no-op build. That rule did not cover build-script run units. Do not repeat the prune without including those units.

Also: a stale, empty `.git/index.lock` (2 hours old, no git process in this repo) blocked the commit and was removed.

Phase B results (final binary with the line-table setting and the patch):

- `target/debug/bevy-jayzee world /workspace/neon-syndicates/world "" 120`: exit 1, stderr `the place argument is empty; name a place or leave it out`.
- `target/debug/bevy-jayzee world /workspace/neon-syndicates/world main 120`: exit 0, `"objects":58`, `"errors":0`, hash `0x1c2b1833`.
- `target/debug/bevy-jayzee gltf-check /workspace/neon-syndicates/assets/cdn`: GLB `"bevy_loaded":64`, `"bevy_failed":0`. Exit 1 is the two PNG textures `texture-concrete-panels.png` and `texture-weathered-wood-planks.png`, outside this work.
- `node scripts/check-game16-api.cjs`: exit 0, `"ok": true`.
- `VIEWER_FOCUS_KIND=model /tmp/ns-viewer/run.sh`: `objects:50`, `modelsLoaded:9`, `modelsMissing:[]`, `uiNodes:718`, no panic.
- The unit tests were not run in this round: they would have been another cargo build, and the owner asked for one build.
