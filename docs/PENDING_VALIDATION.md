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

## Round 7 (dev build candidates: fresh cold builds)

Each candidate is one `cargo build --timings` from a fresh, empty target directory through the cargo slot shim, run sequentially with no other build of this repo running. Then two warm rebuilds after `touch src/main.rs`. CPU seconds are user plus sys for the cargo process tree. Host load is the 1-minute average at start and end of the build, from other workloads on the same 12-core host; load moved during every run, so wall time is noisy and CPU seconds are the stronger figure.

| Candidate | Config change | Cold wall | Cold CPU | Load start to end | Warm 1 (wall / CPU) | Warm 2 (wall / CPU) | Witnesses |
|---|---|---|---|---|---|---|---|
| Baseline (committed config) | none | 1619 s | 6000 s | 5 to 21 | 11 s / 21 s | 9 s / 22 s | all pass |
| Baseline, repeat | none | 1788 s | 6089 s | 26 to 19 | 8 s / 15 s | 5 s / 13 s | all pass |
| (a) dependency `debug = 0` | `[profile.dev.package."*"] debug = 0` | 1159 s | 4483 s | 20 to 15 | 7 s / 10 s | 6 s / 10 s | all pass |
| (a), repeat | same | 1214 s | 4451 s | 20 to 11 | 8 s / 11 s | 9 s / 9 s | all pass |
| (b) `split-debuginfo = "unpacked"` | env `CARGO_PROFILE_DEV_SPLIT_DEBUGINFO=unpacked` | 1413 s | 5445 s | 15 to 11 | 10 s / 17 s | 5 s / 12 s | all pass |
| (c) `bevy/dynamic_linking` | `--features bevy/dynamic_linking` | 1898 s, then link failure | 6868 s | 12 to 19 | not run | not run | link fails |
| (a) with (b) | both | 1687 s | 5440 s | 8 to 33 | 30 s / 19 s | 10 s / 12 s | all pass |
| (d) sccache or cranelift | none | not installed | not installed | not installed | not run | not run | not tested |

Decision: keep (a), `[profile.dev.package."*"] debug = 0`. Two cold runs gave 1159 s and 1214 s wall, against 1619 s and 1788 s for the baseline, and 4451 and 4483 CPU-s against 6000 and 6089 CPU-s. The warm rebuild is 6 to 8 s and about 10 CPU-s, against 9 to 11 s and 13 to 22 CPU-s. (b) helped the cold build on its own but was not kept, because it is slower than (a) and combining it with (a) gave no further gain. The kept configuration passes every witness.

Candidate details:

- (c) `dynamic_linking`: every crate compiled, then the final link failed. `native/basislz_ktx2.cpp` calls `basist::` symbols that the bin's link line does not provide once `bevy_dylib` is used, so the link needs a build change. Not adopted. Open, if the dynamic loop is wanted.
- (d) sccache and cranelift: neither is installed on this host (no `sccache` binary on the PATH or in `/workspace` and `/usr`, no `rustc-codegen-cranelift` component in the stable, nightly, or 1.99.0 toolchains). Nothing was installed, since sudo is blocked. Not tested.

Witnesses for the kept configuration, on the in-place `target/debug/bevy-jayzee`:

- `world … "" 120`: exit 1, stderr `the place argument is empty; name a place or leave it out`.
- `world … main 120`: exit 0, `"objects":58`, `"errors":0`, hash `0x1c2b1833`.
- `check-game16-api.cjs`: `"ok": true`.
- `gltf-check`: `"bevy_loaded":64`, `"bevy_failed":0`. Exit 1 is the two PNG textures outside this work.
- `VIEWER_FOCUS_KIND=model run.sh`: `objects:50`, `modelsLoaded:9`, `modelsMissing:[]`, `uiNodes:718`, no panic.

In-place switch: the kept candidate's `debug` directory was moved into `target/` in place of the old line-tables artifacts, and a no-op build then compiled 0 crates (0.46 s). The first warm rebuild after the move cost 149 CPU-s, which looks like a one-time incremental cache rebuild after the move. Steady warm rebuilds after that: 5 to 10 s, 9 to 10 CPU-s. The `target/` directory is 6.7 GB, down from 19 GB.

Scratch: the candidate directories under `/workspace/.bevy-perf/` (about 40 GB) were removed after their numbers were recorded. None of them is used by the current build.

## Round 8 (mesh sharing, Phase A: code, docs and tests, not yet built)

Goal: objects that point at the same geometry share one mesh asset, and objects with the same mesh and material draw as one batch. Nothing here is built or run yet; each row names its Phase B witness.

| Change | File and row | Witness | Result |
|---|---|---|---|
| Shape key for primitives, markers and proxies (`MeshShape`) | src/viewer_world.rs:264-281 | `cargo test --bin bevy-jayzee viewer_world` (`shared_mesh_reuses_one_handle_per_shape`, `identical_primitives_map_to_one_shape`) | pending Phase B |
| One mesh per shape, cached across frames in a `Local` (`shapes` argument and `shared_mesh`) | src/viewer_world.rs:49, 297-299 | same test; viewer report `shapesCreated` less than `meshRequests` | pending Phase B |
| Primitive spawn uses the shared mesh and counts the request | src/viewer_world.rs:132-141 | viewer report `meshRequests` | pending Phase B |
| Marker spawn uses the shared sphere (`MARKER_RADIUS`) | src/viewer_world.rs:162-173, 282 | viewer report `meshRequests` | pending Phase B |
| Proxy spawn uses the shared cuboid keyed on clamped size | src/viewer_world.rs:174-186 | viewer report `meshRequests` | pending Phase B |
| Primitive material picked by kind (`palette_slot`) instead of entity bits | src/viewer_world.rs:134, 301-303 | `palette_slot_is_stable_and_in_range`; viewer report `uniqueMaterials` | pending Phase B |
| Placeholder spawn in `spawn_viewer` shares four palette materials instead of one material per object | src/viewer.rs:169, 179 | viewer report `uniqueMaterials` | pending Phase B |
| Report counts from live `Mesh3d` entities: `meshes` (unique mesh assets), `meshEntities`, `uniqueMaterials`, `batchKeys` | src/viewer_world.rs:305-319; src/viewer.rs:113 (chain) | `scripts/check-viewer-batching.cjs` (`batchedBelowEntities`, `handfulOfBatches`) | pending Phase B |
| Report fields `meshRequests`, `shapesCreated`, `meshEntities`, `uniqueMaterials`, `batchKeys` and the `meshes` change | src/viewer.rs:18-22, 81-85, 275-279 | `scripts/check-viewer-batching.cjs` (`meshesShared`) | pending Phase B |
| Removed the per-spawn `scene.meshes += 1` and `scene.meshes = objects + items`, which counted spawns, not assets | src/viewer.rs (old line 176), src/viewer_world.rs (old line 206) | viewer report `meshes` now equals unique mesh assets | pending Phase B |
| Draw-count witness script | scripts/check-viewer-batching.cjs | `node scripts/check-viewer-batching.cjs /tmp/ns-viewer/viewer-status-main.json` exit 0 | pending Phase B |

Not changed: the placeholder objects that `spawn_viewer` draws from the place spec are never despawned, so each object appears twice (once as a placeholder, once as its mirror). Existing behaviour, left for the owner to decide.

GLB models already share one scene asset per name (`load_model`), so their meshes were already shared; the report counts them through the `Mesh3d` query.

Phase B witnesses: `VIEWER_FOCUS_KIND=model /tmp/ns-viewer/run.sh`, then `node scripts/check-viewer-batching.cjs /tmp/ns-viewer/viewer-status-main.json`, `node scripts/check-game16-api.cjs`, `target/debug/bevy-jayzee gltf-check /workspace/neon-syndicates/assets/cdn`, and `target/debug/bevy-jayzee world /workspace/neon-syndicates/world "" 120` (exit 1, empty-place message). The longest step's wall time is recorded in Phase B.

### Round 8 Phase B results

Defects found and fixed in Phase B:

- `MeshShape` was private but used in the signature of the `pub(crate)` system `mirror_world`, so the build failed (E0446 class). Fixed by making the enum `pub(crate)`.
- `spawn_viewer` took `ResMut<ViewerScene>` but no longer wrote to it, which warned as an unused `mut`. Changed to `Res`.
- The first batching check counted every `Mesh3d` entity, which includes the internal meshes and materials of the GLB models. It reported 19 batch keys and failed a bound of 16. The owner's goal is the object draws, so the count was split: `batchKeys` and `meshes` still cover the whole scene, and `mirrorEntities` and `mirrorBatchKeys` cover the mirrored objects. The checker tests the mirror population against `shapesCreated * 4 + 1`, the palette size plus the proxy material.

Witnesses on the final tree:

- `/config/.local/bin/cargo build --timings`: `rc=0`, 107 s (the longest step), no warnings.
- `/config/.local/bin/cargo test --bin bevy-jayzee viewer_world`: `3 passed; 0 failed`.
- `VIEWER_FOCUS_KIND=model /tmp/ns-viewer/run.sh`: `objects:50`, `markers:12`, `proxies:0`, `modelsLoaded:9`, `modelsMissing:[]`, `uiNodes:718`, no panic.
- Viewer report counts: `meshRequests:49`, `shapesCreated:3`, `meshEntities:64`, `meshes:19` (unique mesh assets), `uniqueMaterials:19`, `batchKeys:19`, `mirrorEntities:41`, `mirrorBatchKeys:3`.
- `node scripts/check-viewer-batching.cjs /tmp/ns-viewer/viewer-status-main.json`: `"ok": true`; `mirrorBatchKeys 3` against a bound of 13, `shapesCreated 3` against `meshRequests 49`.
- `node scripts/check-game16-api.cjs`: `"ok": true`.
- `target/debug/bevy-jayzee gltf-check /workspace/neon-syndicates/assets/cdn`: `bevy_loaded:64`, `bevy_failed:0`, 2 s. Exit 1 is the two PNG textures `texture-concrete-panels.png` and `texture-weathered-wood-planks.png`, outside this work.
- `target/debug/bevy-jayzee world /workspace/neon-syndicates/world "" 120`: exit 1, `the place argument is empty; name a place or leave it out`.
- `target/debug/bevy-jayzee world /workspace/neon-syndicates/world main 120`: exit 0, `objects:58`, `errors:0`, hash `0x1c2b1833`.

Not measured: the report has no pre-change baseline binary. The "before" for the mesh count is the request count: the old code created one asset per request, so before this change `meshes` would equal `meshRequests`, here 49 against 3 now.

## Round 9 (placeholder retirement, Phase A: code, docs and tests, not yet built)

Decision: each object is drawn once. `spawn_viewer` placeholders carry a `ViewerPlaceholder` id and are despawned once a mirror with the same id exists. The report now counts drawn `ViewerObject` entities against the expected set (world items, place-spec objects and terrains) and lists missing and duplicate ids.

| Change | File and row | Witness | Result |
|---|---|---|---|
| `ViewerPlaceholder` component carries the object id on each placeholder | src/viewer.rs:56-57, 213 | `scripts/check-viewer-objects.cjs` (`noPlaceholderOverlap`) | pending Phase B |
| One `object_id` for both spawn paths, so placeholder and mirror ids match exactly | src/viewer.rs:59-61; used at 196 and in src/viewer_world.rs:226 | `scripts/check-viewer-objects.cjs` (`noMissing`) | pending Phase B |
| Placeholders with a repeated place-spec id are not spawned twice | src/viewer.rs:194-198 | `scripts/check-viewer-objects.cjs` (`noDuplicates`) | pending Phase B |
| `retire_placeholders` system despawns placeholders whose mirror exists; chained after `mirror_world` | src/viewer_world.rs:234-248; src/viewer.rs:137 | `scripts/check-viewer-objects.cjs` (`countMatchesExpected`), `placeholdersRetired` > 0 | pending Phase B |
| Pure `placeholders_to_retire<T>` and `duplicate_count` helpers | src/viewer_world.rs:250-256 | `cargo test --bin bevy-jayzee` (full suite, Phase B) | pending Phase B |
| Expected set: world items, place-spec ids and terrain ids, sorted | src/viewer_world.rs:224-232 (end of `mirror_world`) | `expectedObjects` in the viewer report equals `objects` | pending Phase B |
| `objects` now counts drawn `ViewerObject` entities; `expectedObjects`, `duplicateObjects`, `missingObjects`, `placeholders`, `placeholdersRetired` added to the report | src/viewer_world.rs:338-349 (`count_batches`); src/viewer.rs:ViewerScene and JSON | `scripts/check-viewer-objects.cjs` exit 0 | pending Phase B |
| Removed `scene.objects = place objects + items`, which double-counted | src/viewer_world.rs (old line 205) | `objects` equals the drawn count | pending Phase B |
| Object-count witness script | scripts/check-viewer-objects.cjs | `node scripts/check-viewer-objects.cjs /tmp/ns-viewer/viewer-status-main.json` exit 0 | pending Phase B |

Phase B also runs the full viewer unit-test suite (`cargo test --bin bevy-jayzee`, not only the helpers) and records its wall time.

## Ranked remaining work for bevy

Ranked by visible impact on the neon-syndicates viewer and by how much a regression would hide.

1. Neon terrain is not drawn (`terrains:0`). The terrain comes from `scripts/flat-starter-terrain.js`, which the Bevy host does not execute, so the ground is the largest visible gap.
2. HUD SVG drops path curves (`C`, `Q`, `A`) and `<defs>` (`uiWarnings` shows `path` and `defs` dropped). The HUD is the only text-bearing surface the owner checks, and the gap is visible.
3. Two PNG textures fail every gltf-check run (`texture-concrete-panels.png`, `texture-weathered-wood-planks.png`). Exit 1 is a permanent red witness, so a new asset failure would hide inside it.
4. GLB vertex format warnings from `bevy_gltf` (`TANGENT` is `Snorm16x4` where `Float32x4` is expected, and unsupported accessor formats) on the 9 loaded models. Normal-map shading may be wrong on those models; not checked by eye.
5. Scripted primitives are drawn at the default size (`guide-*`), because their real geometry is a render shim in the script. Visible size error on 12 objects.
6. `bevy/dynamic_linking` does not link (BasisLZ `basist::` symbols). The warm rebuild is already 6 to 8 s, so the payoff is small, but the option is dead.
7. Browser path (`wasm-viewer`, WebGPU) has not been built or witnessed since the viewer changes. A breakage would be silent.
8. Lightyear network scene (client prediction and server replication) has no witness in the current stretch.
9. The `jobs = 4` rustc cap is unconfirmed (host-wide samples include other workloads), and `verify.sh` (release build) has not run in this stretch.

Excluded by the owner: the 115 `ctx.*` figure, which is recorded as unreproduced above.

### Round 9 Phase B results

Defects found and fixed in Phase B:

- Static viewers (no `ui.render` in the world config, such as `testdata/spec-world`) never ran `count_batches`, so the report kept its initial values (`objects` 8, `expectedObjects` 0). The audit now runs in both modes, and `spawn_viewer` seeds the expected set from the place spec.
- A model whose GLB is missing (`/cdn/neutral-default-avatar.glb`, the `local-player` avatar on `game-16` `hall`) was skipped, so the object was never drawn and the audit reported it missing. It is now drawn as a marker through the shared `spawn_marker` helper, so every object is drawn once.
- Placeholder colours were picked by list index, so placeholders could not batch by kind. They now use the kind-keyed palette (`palette_slot`), the same as primitives.
- The batching witness had measured only mirrored objects, so it failed on a static viewer where there are no mirrors. It now measures all object draws (`objectDraws`, `objectBatchKeys`) and applies the mirror bound only when mirrors exist.

Neon `main` has no spec objects (`places/main` has no `cells/`), so placeholder retirement cannot be exercised there. The retirement witness uses `game-16` `hall`, which has both spec objects and a runner, and the static witness uses the fixture.

Witnesses on the final tree:

- `/config/.local/bin/cargo build --timings`: `rc=0`, 51 s, no warnings.
- Full unit suite `/config/.local/bin/cargo test --bin bevy-jayzee`: `test result: ok. 42 passed; 0 failed`, 57 s wall including the test-binary compile (0.45 s of test time). This is the longest step.
- Neon `main` viewer: `objects 50`, `expectedObjects 50`, `duplicateObjects 0`, `missingObjects []`, `objectDraws 41`, `objectBatchKeys 3`, `modelsLoaded 9`, `uiNodes 718`. `check-viewer-objects.cjs` and `check-viewer-batching.cjs` both `"ok": true`.
- `game-16` `hall` viewer (mirror mode, both spec objects and a runner): `objects 222`, `expectedObjects 222`, `duplicateObjects 0`, `missingObjects []`, `placeholdersRetired 147`, `placeholders 0`, `mirrorEntities 222`, `objectBatchKeys 6`. Both checkers `"ok": true`. The avatar model is still missing (`modelsMissing`), and its object is drawn as a marker.
- Fixture `testdata/spec-world` `test` viewer (static mode): `objects 8`, `expectedObjects 8`, `duplicateObjects 0`, `placeholders 8`, `objectBatchKeys 3`. Both checkers `"ok": true`.
- `node scripts/check-game16-api.cjs`: `"ok": true`.
- `target/debug/bevy-jayzee gltf-check /workspace/neon-syndicates/assets/cdn`: `bevy_loaded:64`, `bevy_failed:0`. Exit 1 is the two PNG textures `texture-concrete-panels.png` and `texture-weathered-wood-planks.png`, outside this work.
- `target/debug/bevy-jayzee world /workspace/neon-syndicates/world "" 120`: exit 1, `the place argument is empty; name a place or leave it out`.
- `target/debug/bevy-jayzee world /workspace/neon-syndicates/world main 120`: exit 0, `objects:58`, `errors:0`, hash `0x1c2b1833`.

Not measured: the old duplicate count. The old code spawned a placeholder and a mirror for each spec object that had a world item, so the old report would show one duplicate per such object. The `game-16` `hall` run has 147 retired placeholders, which is the number of such objects. The old binary was not run, so this is by construction, not a measurement.

Environment note: `/tmp/ns-viewer/run.sh` now takes `WORLD` and `PLACE` from the environment (defaults unchanged). It deletes its own untracked `web/status-<place>.json` on exit, so a previously generated `web/status-test.json` was removed by the fixture run. The file is gitignored and regenerated on every viewer run.

## Round 10 Phase A changes

Each row lists the commit and the Phase B witness that must pass before the change counts as done. Nothing in Phase A was built, tested or run.

| Item | Commit | Change | Phase B witness |
| --- | --- | --- | --- |
| 1 Neon terrain | 8131a6c | New `bake-terrain <dir> <place>` subcommand samples the place generator (`scripts/flat-starter-terrain.js` `heightAt`) over a grid and writes `places/<place>/terrain-fields.json`. `load_terrain` reads that file after inline `fields`. The generator is run through the existing QuickJS module loader. | `bevy-jayzee bake-terrain /workspace/neon-syndicates/world main`, then the neon viewer `terrains` >= 1. Re-run `world main 120`; the hash may move because the grid adds a static heightfield collider, which is reported as a change, not a regression. |
| 2 HUD SVG | aaa5d10 | `path` supports C/S/Q/T/A curves, flattened to polylines. `<defs>` children are registered by id and not drawn; `<use href>` draws them at their x/y offset. `svg_path_with_curves_is_reported` becomes `svg_path_with_curves_is_drawn`. | `cargo test --bin bevy-jayzee ui_html`; the neon HUD `dropped` map has no `path`, `defs` or `use` entries. |
| 3 PNG textures | (Phase B, neon assets) | `texture-concrete-panels.png` and `texture-weathered-wood-planks.png` in `/workspace/neon-syndicates/assets/cdn` are WebP (RIFF/WEBP) under a `.png` name, which the PNG decoder rejects. Planned fix: transcode to real PNG under the same names, keeping the WebP originals beside them. Not a repo change; the neon repo is not under git. | `gltf-check` exits 0. |
| 4 Tangent format | b3a5c28 | `glb::decode` converts normalized SHORT VEC4 TANGENT accessors to Float32 in an appended bufferView before bevy_gltf reads them (bevy_gltf rejects the format with `WrongFormat` and drops the attribute). Evidence from a neon scan: 12 of 64 GLBs carry this tangent format, 19 primitives, all 19 with a `normalTexture`, so normal-map shading uses the screen-derivative fallback today. | `cargo test --bin bevy-jayzee glb`; `gltf-check` shows no TANGENT WrongFormat warnings. |
| 5 Scripted primitives | c65f3f7 | Scripted collider children are drawn with their Avian shape kind (Sphere, Cylinder, Capsule, Cuboid) through the shared mesh cache. A scripted parent with collider children no longer also draws the default marker; its first child takes its id and later children take `<id>#<n>`. | Viewer on a neon place that spawns scripted primitives (`templates/props.js` with `scripts/gen/sign.js`/`prop.js`): no marker for those parents, scripted-sphere/cylinder/capsule/cuboid kinds appear, object audit still matches. `cargo test --bin bevy-jayzee scripted_colliders_keep_their_shape_kind`. |
| 6 Browser and WebGPU | c7f586e | `scripts/check-wasm-webgpu.cjs` drives the wasm viewer through playwriter on display `:1` with the adapter flags; it stops every process it starts by PID. The `gm` MCP server failed to connect, so playwriter is used directly. | `node scripts/check-wasm-webgpu.cjs`: `ok: true`, `navigatorGpu`, `bevyReady`, `BrowserWebGpu` adapter line. |
| 7 Lightyear witness | a985d44, f44a6da | `scripts/check-network.cjs` asserts the client report (one player, predicted, controlled, not remote, histories filled, confirmed tick > 0, error <= 0.05 m, moved >= 0.5 m) and a server-state line with a client. `verify.sh` calls it instead of grepping. | `bash scripts/verify.sh`; the checker's JSON with `ok: true`. |
| 7a Client report JSON | 6c8c795 | `client-report` prints `errorM` as `null` when no confirmed tick exists, instead of `inf`, so the line stays valid JSON. | Covered by item 7. |
| 8 jobs cap and verify.sh | (Phase B) | No repo change planned. `/config/.cargo/config.toml` sets `[build] jobs = 4`. Phase B confirms the cap with `cargo config get build.jobs` and samples concurrent rustc processes during one build, then runs `verify.sh` once. | `cargo config get build.jobs` prints `4`; `verify.sh` exits 0 with its checks. |

The Phase A work used six forked agents, one per item, each committing only its own files with no trailers; the lead added the `client.rs` and `check-network.cjs` fixes in 6c8c795 and f44a6da.

## Round 10 Phase B results

Contention: swap-out was 0 KB/s and 5.7 GB free before Phase B, with no cargo running. One cargo invocation ran at a time. The unit suite never overlapped a build.

Longest step: `/config/.local/bin/cargo build --release` at 17 m 35 s. Sampled concurrent `rustc` processes peaked at 4 across 523 samples, which matches `jobs = 4` in `/config/.cargo/config.toml`. `cargo config get` is unavailable on this toolchain, so the cap is confirmed from the file plus the observed peak.

Defects found in Phase B and fixed:

- **Viewer attribute lookup (359e0ac).** The HTML tag reader lowercases attribute names, but `attr` looked up `viewBox` case-sensitively, so every SVG `viewBox` was ignored and HUD percentages never applied. `attr` now compares names case-insensitively.
- **Terrain drawn twice (1a29c4a).** The baked heightfield collider was also drawn as a proxy, beside the terrain mirror, so neon reported `objects 52` against `expectedObjects 51`. Heightfield colliders are now skipped in the proxy branch.
- **Despawn warnings every tick (3d75dcc).** `sync_feature_entities` despawned each feature entity twice, once from the registry vector and once from the marker query. The registry loop is removed, and the query covers every feature entity because each registry vector has a matching marker type. The warnings were `Encountered an error in command ... Entity despawned`, about 14,000 per run with feature scripts on. They're now 0 on neon and on the fixture.
- **Network check remote count (ada7931).** The checker assumed `remote == 0`, but the report counts every `Remote` entity, including the replicated floor and the confirmed copy of the player (2 on this run). The check now requires `remote >= 2`.
- **Network check null errorM (f44a6da).** A `null` errorM satisfies `null <= 0.05` in JavaScript, so the check now requires a number.
- **WebGPU witness (b24b950).** The script saved the whole "Session N created" message as the session id, ran `-e` with the default 10 s timeout while waiting up to 90 s, and read `-e` output without the `[log] ` prefix. All three are fixed.

Witnesses on the final tree:

- Release build: `Finished release profile`, 17 m 35 s. `verify.sh`: rc 0, 7 s wall.
  - Bench 500 bodies `0.4307 ms`, 2000 `0.9275 ms`, 5000 `1.3169 ms`; determinism `pass: true`.
  - Client report: `players 1`, `predicted 1`, `controlled 1`, `remote 2`, `errorM 0`, `predictionHistory 21`, `confirmedHistory 17`, moved 14.5 m along x.
  - `check-network.cjs`: `ok: true`.
- `gltf-check /workspace/neon-syndicates/assets/cdn`: rc 0 (was rc 1, the two PNG textures), `bevy_loaded 64`, `bevy_failed 0`, stderr 0 lines. The two textures were WebP content under `.png` names. They're now real PNG, and the WebP originals are kept beside them as `.webp`. Neon isn't under git, so this is a data change, not a repo commit.
- `bake-terrain /workspace/neon-syndicates/world main`: rc 0, writes `places/main/terrain-fields.json`.
- Neon `main` viewer: `objects 51`, `expectedObjects 51`, `duplicateObjects 0`, `missingObjects []`, `terrains 1`, `modelsLoaded 9`, `markers 12`, `uiNodes 718`, `uiWarnings []`, 0 despawn warnings. Both checkers `ok: true`.
- Scratch fixture in mirror mode (a copy of `testdata/spec-world` under `/tmp` with `ui.render` added, the place policy and the `ball`'s `spec-test.js` removed, so the place stays `test`): `objects 9`, `expectedObjects 9`, `duplicateObjects 0`, `missingObjects []`, `placeholdersRetired 8`, `markers 0`, 0 despawn warnings. The scripted parent `geo` no longer draws as a marker; it draws through its cuboid child. Shape kinds sphere, cylinder and capsule are covered by the unit test only, not by a live witness.
- `world /workspace/neon-syndicates/world main 120`: rc 0, `errors 0`, `objects 59`, hash `0xa33a35c4`. **This changes the baseline** from `objects 58` and hash `0x1c2b1833`. The cause is the new static ground heightfield from the terrain bake, which is now a physics body. It's a deliberate sim change, and the owner needs to accept the new baseline.
- Empty place: rc 1, `the place argument is empty; name a place or leave it out`.
- `node scripts/check-game16-api.cjs`: `ok: true`.
- `cargo test --bin bevy-jayzee`: 47 passed, 0 failed, 0.12 s test time (about 5 s wall with the incremental test-binary compile). Includes `scripted_colliders_keep_their_shape_kind`.
- Wasm: `WASM_PROFILE=release scripts/build-wasm.sh` rc 0. The `wasm-viewer` crate was already built and does not depend on the root crate, so this rebuild was quick (4 s). Output: 28 MB `.wasm` under `web/wasm/`.
- Playwriter witness (`node scripts/check-wasm-webgpu.cjs`): `ok: true`, `navigatorGpu: true`, `appReady: true`, `bevyReady: true`, adapter `BrowserWebGpu` on `NVIDIA GeForce RTX 3060 Laptop GPU`. Chromium ran on `:1` with the adapter flags. The only logged error is `404` for `/favicon.ico`, which the page doesn't declare. The server and browser were stopped by PID.

Open for the owner:

- **Hash baseline (above).** The `world main` hash moved because terrain now exists, so the old `0x1c2b1833` no longer applies. Accept `0xa33a35c4` or ask for the terrain to be left out of the sim.
- **Sphere, cylinder and capsule scripted shapes** are covered by the unit test only. The fixture's `geo` is a cuboid.
- **Normal-map shading** is fixed at the data level. Authored tangents are now kept, but I haven't compared shading on screen.
- **HUD warnings baseline.** `uiWarnings` is empty on neon now. I didn't capture a before-value, so I can't say how many path or defs drops disappeared.
- **Compact arc flags** (`a1 1 0 011 1`) are not parsed; the tokenizer reads them as one number.
- **gm MCP server** failed to connect this session, so invariant 1 was not followed. playwriter was used directly, as the owner instructed.
- **Release profile** is unchanged (acd52c8 settings kept).

## Round 11 Phase A changes

Nothing in Phase A was built, tested or run. Each row is one local commit by lanmower with no trailers.

| Item | Commit | Change | Phase B witness |
| --- | --- | --- | --- |
| 1 Scripted sphere, cylinder and capsule | 1cfa4b0 | Adds the `shapes` place to `testdata/spec-world`: three scripted objects `shape-sphere`, `shape-cylinder` and `shape-capsule`, each with `scripts/shapes.js` returning the matching collider. Adds `ui.render` (`scripts/ui.js`) to the fixture world, because the viewer draws mirrors only when a world has a render script. | `WORLD=testdata/spec-world PLACE=shapes` through `run.sh`: `objects 3 = expectedObjects 3`, `duplicateObjects 0`, `markers 0`, and `objectKinds` contains `scripted-sphere`, `scripted-cylinder` and `scripted-capsule` with count 1 each. |
| 1 Object-kind status | 8feee8e | The viewer status reports `objectKinds`, a count of every drawn object's `kind`, so shape kinds can be witnessed from the status file. | Same run as above. |
| 2 SVG compact arc flags | d75c869 | `path_segments` reads the large-arc and sweep flags of `A`/`a` as single `0`/`1` characters, so `a1 1 0 011 1` parses. Test `compact_arc_flags_parse_as_single_digits` compares compact and spaced forms. | `cargo test --bin bevy-jayzee ui_html`. |
| 3 Normal-map fixture | 57fc47b | `scripts/make-normal-map-fixture.cjs` writes `testdata/normal-map/normal-map.glb`: one quad with SHORT normalized TANGENT and a `normalTexture` embedded PNG. Neon's 12 GLBs with this tangent format use meshopt and KTX2 textures, which the browser build can't decode, so they can't be used for the browser comparison. | Phase B runs the script, commits the GLB, and runs `gltf-check` on `testdata/normal-map`, which writes the decoded copy to `target/gltf-cache`. |
| 3 Browser loader | 6c4e927 | `wasm-viewer` loads a glTF scene when the page has `?model=<url>`, with `bevy_gltf` features added to its Cargo.toml. The default demo scene is unchanged. `scripts/check-normal-map-visual.cjs` opens the raw and decoded GLB through playwriter on `:1` and saves a screenshot for each. | `scripts/check-normal-map-visual.cjs`. The raw variant should show TANGENT warnings; the decoded one should show none. |

Phase B also needs the `wasm-viewer/Cargo.lock` update, because the new features bring crates the lock doesn't have. The Phase B witness for item 3 is a visual check, not a number: no before-value exists in the status, so the screenshots are compared by eye, and the paths are recorded in the Phase B section.

## Round 11 Phase B results

Contention: swap-out was 0 and 10 GB was free before Phase B, with no cargo running. One cargo invocation ran at a time, and the full unit suite ran alone after the builds.

Longest step: the release wasm build (`WASM_PROFILE=release scripts/build-wasm.sh`) at 217 s, which compiled `bevy_gltf`. The dev binary build was 18 to 21 s, and the unit suite ran 14 s wall.

Defect found and fixed in Phase B (e521a6d): the spec fixture's `shapes` place drew each scripted parent twice. The first copy was the parent's own `PrimitiveInfo` with kind `scripted`, drawn at default size. This is the defect from the Round 10 queue, and the Phase A children change did not stop it, because the primitive branch ran first. Scripted entities are now skipped in the item loop. A scripted parent with no collider children gets a marker after the children loop. The marker kind is now `marker`, not `scripted-marker`, because `local-player` is a marker too.

Witnesses on the final tree:

- Item 1, fixture shapes place (`WORLD=testdata/spec-world PLACE=shapes`): `objects 4 = expectedObjects 4`, `duplicateObjects 0`, `missingObjects []`, `objectKinds` = `{"marker":1,"scripted-capsule":1,"scripted-cylinder":1,"scripted-sphere":1}`, 0 despawn warnings. `local-player` is the marker, spawned by the world because the place has no player object. `check-viewer-objects.cjs` `ok: true`.
- Item 2, compact arc flags: `cargo test --bin bevy-jayzee` passes `compact_arc_flags_parse_as_single_digits`.
- Unit suite after the builds: `test result: ok. 48 passed; 0 failed`, 0.65 s of test time, 14 s wall.
- Item 3, on-screen normal-map comparison (`node scripts/check-normal-map-visual.cjs`, playwriter, display `:1`, adapter flags):
  - Raw `/testdata/normal-map/normal-map.glb`: `modelState loaded`, warning `Vertex attribute TANGENT has format Snorm16x4 but expected Float32x4`. Screenshot `target/witness/normal-map-raw.png`.
  - Decoded `/target/gltf-cache/normal-map.glb`: `modelState loaded`, no TANGENT warnings. Screenshot `target/witness/normal-map-decoded.png`.
  - Both use the BrowserWebGpu adapter, NVIDIA GeForce RTX 3060 Laptop GPU. The 404 lines match the favicon request seen in the earlier WebGPU witness, but I did not re-check them for this run.
  - **Visual check, not a number.** No before-value exists in the status, so the two screenshots were compared by eye. Both show the same bump pattern with no clear difference, and the bumps look recessed rather than raised. The sign convention of the generated normal map is therefore unverified.
  - The fixture is generated, not a neon asset. Neon's 12 GLBs with the same tangent format use meshopt and KTX2 textures, which the browser build can't decode.
- `gltf-check testdata/normal-map`: rc 0, `bevy.loaded: true`. The decoded copy has a FLOAT32 VEC4 TANGENT (componentType 5126), and the raw one keeps SHORT normalized (5122).
- Neon `main` viewer: `ok: true`, `objects 51 = expectedObjects 51`, `terrains 1`.
- `world /workspace/neon-syndicates/world main 120`: rc 0, `errors 0`, `objects 59`, hash `0xa33a35c4`. This is the accepted baseline.
- `gltf-check /workspace/neon-syndicates/assets/cdn`: rc 0, `bevy_loaded 64`, `bevy_failed 0`.
- `world testdata/spec-world test 120` ends at `objects 2`, because `spec-test.js` in the `ball` behaviour calls `enterPlace('two')`. That behaviour predates this round, and the `shapes` place has no such script.

Process check: no browser, server or build process started by this round remains running. The Xvfb on `:1` (PID 404) predates this session and was left running.
