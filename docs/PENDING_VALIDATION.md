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
