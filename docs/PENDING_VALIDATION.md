# Pending validation

Each row names a committed change, the file it touches, and the witness that proves it. A row is green when its witness was run on the tree it was committed in and the output is quoted in the status.

| Commit change | File | Witness | Result |
|---|---|---|---|
| Seeded-random and feature-report unit tests | src/features.rs | `cargo test --bin bevy-jayzee features::tests` | pass: `test result: ok` for the features tests, within `tests::` 31 passed |
| Place program `sim.js`, neon-syndicates ctx keys, `ctx.spawn`, `__wrap` root sync, builtin `tsl` names | src/script/mod.rs, src/script/prelude.js, AGENTS.md | `target/debug/bevy-jayzee world /workspace/neon-syndicates/world main 120` exit 0 with `errors:0`, `missingApiCount:0`, `objects:58`, `behaviors:1` | pass, see STATUS.md section "Neon-syndicates ctx reconciliation" |
| Same change, script side | src/script/mod.rs, src/script/prelude.js | `node scripts/check-game16-api.cjs` exit 0, `"ok": true` | pass |
| Player template skip in world spawn | src/world.rs | `world ... main 120` shows the `player` template spawned (`templates/player.js` in modules) and no bare `local-player` | pass, see run log |
| BasisLZ KTX2 transcode (already on origin/main at 63700b4) | src/basisu.rs, src/glb.rs | `target/debug/bevy-jayzee gltf-check /workspace/neon-syndicates/assets/cdn`: `bevy_failed:0`, `bevy_loaded:64`, `pass:64` | pass (PNG failures outside scope) |
| open until Round 16 Phase B: glyph boxes and overlap fixed in Round 16 Phase A (item 6) |
| pass, Round 2 Phase B: the test passes after the fraction fix |

## Round 2 (Phase A, committed locally, not pushed)

| Commit change | File | Witness | Result |
|---|---|---|---|
| Enable Bevy's default font feature | Cargo.toml | viewer screenshot shows the overlay text (`/tmp/ns-viewer/viewer-*.png`) | pass, Round 2 Phase B |
| Bound globe elevation by amplitude | src/globe.rs | `cargo test --bin bevy-jayzee elevation_is_deterministic` passes | pass, Round 2 Phase B |
| Mirror primitive world entities into the viewer | src/viewer.rs | `viewer_status` `objects` equals the headless `objects` count of primitive-backed entities | pass, Round 2 Phase B |
| Palette index fix for mirrored objects | src/viewer.rs | build passes with no errors | pass, Round 2 Phase B |
| Globe noise uses the non-negative fraction | src/globe.rs | `cargo test --bin bevy-jayzee globe::` passes | pass, Round 2 Phase B |
| Viewer reports unmirrored objects | src/viewer.rs | `viewer-status` `unmirroredObjects` plus mirrored `objects` equals the headless `objects` count | pass, Round 2 Phase B |

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
| Draw the HUD's SVG circles, lines and rects as absolute nodes | src/ui_html.rs | `cargo test --bin bevy-jayzee ui_html` passes (10 tests); HUD `uiNodes` rises from 60 to 102 and `unsupported element <svg>` is gone | pass, Round 3 Phase B |

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

Resolved in Round 10 (8131a6c): `bake-terrain` writes the terrain field and the viewer draws `terrains 1`.
- Scripted primitives (the `guide-*` ids): drawn as primitives at their correct positions with the default size. Their real geometry is a render shim in the script, so the size is a gap, not a missing draw.
- The "115 missing ctx.* APIs" figure is unreproduced. The measured missing host names are 10: `__feelRaw`, `__glass`, `__safeDestroy`, `__soundGun`, `__soundQ` (with `.length` and `.push`), `__soundRaw`, `spawn`, `uvProjection`. The 115 figure is not reproduced by `scripts/check-game16-api.cjs` or the world run; `missingApiCount` is 0.

## Round 5 Phase B results

- `VIEWER_FOCUS_KIND=model /tmp/ns-viewer/run.sh`: `viewer-status` `objects:50`, `meshes:50`, `markers:12`, `modelsLoaded:9`, `modelsMissing:[]`, `unmirroredObjects:0`, `uiNodes:718`, `uiWarnings:["unsupported element <defs> dropped (1)"]`, `terrains:0`. No `panicked` in `viewer.err`; the one `error` line is the X11 `XSETTINGS` warning. `viewer-2.png` shows four GLB characters; `viewer-1.png` shows the SVG polygon shapes at the top left.
- `bevy-jayzee gltf-check /workspace/neon-syndicates/assets/cdn`: GLB `"pass":64,"total":64`, `"bevy_loaded":64`, `"bevy_failed":0`. Exit 1 is the two PNG failures `texture-concrete-panels.png` and `texture-weathered-wood-planks.png`, outside this work.
- `bevy-jayzee world /workspace/neon-syndicates/world main 120`: exit 0, `"objects":58`, `"behaviors":1`, `"errors":0`, `"missingApiCount":0`, hash `0x1c2b1833`.
- `node scripts/check-game16-api.cjs`: exit 0, `"ok": true`.
- `cargo test --bin bevy-jayzee`: `test result: ok. 37 passed; 0 failed`.
- Parallelism: the bin rebuilds took 31 to 35 minutes and 1 minute 22 seconds when cached. The `jobs = 4` cap in `/config/.cargo/config.toml` is still unconfirmed by sampling.
- Neon terrain: resolved in Round 10 (8131a6c). Scripted-primitive size: still open, rendered at default size.

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
| Shape key for primitives, markers and proxies (`MeshShape`) | src/viewer_world.rs:264-281 | `cargo test --bin bevy-jayzee viewer_world` (`shared_mesh_reuses_one_handle_per_shape`, `identical_primitives_map_to_one_shape`) | pass, Round 8 Phase B |
| One mesh per shape, cached across frames in a `Local` (`shapes` argument and `shared_mesh`) | src/viewer_world.rs:49, 297-299 | same test; viewer report `shapesCreated` less than `meshRequests` | pass, Round 8 Phase B |
| Primitive spawn uses the shared mesh and counts the request | src/viewer_world.rs:132-141 | viewer report `meshRequests` | pass, Round 8 Phase B |
| Marker spawn uses the shared sphere (`MARKER_RADIUS`) | src/viewer_world.rs:162-173, 282 | viewer report `meshRequests` | pass, Round 8 Phase B |
| Proxy spawn uses the shared cuboid keyed on clamped size | src/viewer_world.rs:174-186 | viewer report `meshRequests` | pass, Round 8 Phase B |
| Primitive material picked by kind (`palette_slot`) instead of entity bits | src/viewer_world.rs:134, 301-303 | `palette_slot_is_stable_and_in_range`; viewer report `uniqueMaterials` | pass, Round 8 Phase B |
| Placeholder spawn in `spawn_viewer` shares four palette materials instead of one material per object | src/viewer.rs:169, 179 | viewer report `uniqueMaterials` | pass, Round 8 Phase B |
| Report counts from live `Mesh3d` entities: `meshes` (unique mesh assets), `meshEntities`, `uniqueMaterials`, `batchKeys` | src/viewer_world.rs:305-319; src/viewer.rs:113 (chain) | `scripts/check-viewer-batching.cjs` (`batchedBelowEntities`, `handfulOfBatches`) | pass, Round 8 Phase B |
| Report fields `meshRequests`, `shapesCreated`, `meshEntities`, `uniqueMaterials`, `batchKeys` and the `meshes` change | src/viewer.rs:18-22, 81-85, 275-279 | `scripts/check-viewer-batching.cjs` (`meshesShared`) | pass, Round 8 Phase B |
| Removed the per-spawn `scene.meshes += 1` and `scene.meshes = objects + items`, which counted spawns, not assets | src/viewer.rs (old line 176), src/viewer_world.rs (old line 206) | viewer report `meshes` now equals unique mesh assets | pass, Round 8 Phase B |
| Draw-count witness script | scripts/check-viewer-batching.cjs | `node scripts/check-viewer-batching.cjs /tmp/ns-viewer/viewer-status-main.json` exit 0 | pass, Round 8 Phase B |

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
| `ViewerPlaceholder` component carries the object id on each placeholder | src/viewer.rs:56-57, 213 | `scripts/check-viewer-objects.cjs` (`noPlaceholderOverlap`) | pass, Round 9 Phase B |
| One `object_id` for both spawn paths, so placeholder and mirror ids match exactly | src/viewer.rs:59-61; used at 196 and in src/viewer_world.rs:226 | `scripts/check-viewer-objects.cjs` (`noMissing`) | pass, Round 9 Phase B |
| Placeholders with a repeated place-spec id are not spawned twice | src/viewer.rs:194-198 | `scripts/check-viewer-objects.cjs` (`noDuplicates`) | pass, Round 9 Phase B |
| `retire_placeholders` system despawns placeholders whose mirror exists; chained after `mirror_world` | src/viewer_world.rs:234-248; src/viewer.rs:137 | `scripts/check-viewer-objects.cjs` (`countMatchesExpected`), `placeholdersRetired` > 0 | pass, Round 9 Phase B |
| Pure `placeholders_to_retire<T>` and `duplicate_count` helpers | src/viewer_world.rs:250-256 | `cargo test --bin bevy-jayzee` (full suite, Phase B) | pass, Round 9 Phase B |
| Expected set: world items, place-spec ids and terrain ids, sorted | src/viewer_world.rs:224-232 (end of `mirror_world`) | `expectedObjects` in the viewer report equals `objects` | pass, Round 9 Phase B |
| `objects` now counts drawn `ViewerObject` entities; `expectedObjects`, `duplicateObjects`, `missingObjects`, `placeholders`, `placeholdersRetired` added to the report | src/viewer_world.rs:338-349 (`count_batches`); src/viewer.rs:ViewerScene and JSON | `scripts/check-viewer-objects.cjs` exit 0 | pass, Round 9 Phase B |
| Removed `scene.objects = place objects + items`, which double-counted | src/viewer_world.rs (old line 205) | `objects` equals the drawn count | pass, Round 9 Phase B |
| Object-count witness script | scripts/check-viewer-objects.cjs | `node scripts/check-viewer-objects.cjs /tmp/ns-viewer/viewer-status-main.json` exit 0 | pass, Round 9 Phase B |

Phase B also runs the full viewer unit-test suite (`cargo test --bin bevy-jayzee`, not only the helpers) and records its wall time.

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
- **Compact arc flags** (`a1 1 0 011 1`): fixed in d75c869, see the Round 10 table.
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

## Round 12 Phase A changes

Nothing in Phase A was built, tested or run. Each row is one local commit by lanmower with no trailers.

| Item | Commit | Change | Phase B witness |
| --- | --- | --- | --- |
| 1 Fixture ends at `objects 2` | eee60c1 | The place switch (`enterPlace` to `two`) moved out of the `ball` object's `spec-test.js` into `places/test/sim.js`, a place program. It fires on the sixth run of its one-second cadence, so the 120-tick witness never reaches it. `places/two/sim.js` returns to `test` after one more second. The switch runs only once per place visit, because the counter lives in `ctx.place.state`. | `world testdata/spec-world test 120` ends in place `test` with `objects 8`. A viewer run on `test` ends with the expected object count. |
| 2 Normal-map sign | 85c28c1 | `ny` changes from `-dy * strength` to `dy * strength`. The dome heights are raised, and image rows increase downward, so G must be high on the upper slope. The bump maths moves to `scripts/lib/normal-map-bumps.cjs`, which the generator requires. | Regenerate `testdata/normal-map/normal-map.glb`, then `node scripts/check-normal-map-sign.cjs` with `ok: true`. |
| 2 Numeric sign witness | 2d60ac8 | `scripts/check-normal-map-sign.cjs` decodes the GLB's embedded PNG and checks three things: every pixel matches the reference normal within ±2; on dome slopes the R and G signs point away from the dome centre; and vertex UVs follow positions (u grows with x, v grows downward). It exits non-zero on failure. | The checker must fail on the committed GLB before regeneration and pass after. |

## Round 12 Phase B results

Contention: swap-out 0 and 10,591 MB available before the build and the browser runs. One cargo invocation ran, and it was a no-op (`cargo build --timings`, rc 0, 0 warnings). The unit suite ran alone afterwards.

Defect 1, fixture ends at `objects 2`: fixed in eee60c1. The switch to place `two` now runs from `places/test/sim.js` on the sixth one-second run, and `places/two/sim.js` returns to `test`. The `ball` object's script no longer switches places.

- `world testdata/spec-world test 120`: rc 0, `objects 8`, `errors 0`, every `place` field `test`. The 120-tick window ends before the switch, so this run shows the count on the starting place only.
- `world testdata/spec-world test 600`: rc 0, `objects 8`, `errors 0`, one `spec-enter` (switch from `test`), one `spec-two`, and the final lines in `test`. The 600-tick run covers the full switch and return.

Defect 2, normal-map sign: fixed in 85c28c1, with the numeric witness in 2d60ac8 and the regenerated GLB in b590c0d. Before the fix the numbers were wrong on almost every pixel, and this check replaces the eye check from Round 11.

- Committed GLB before regeneration, `node scripts/check-normal-map-sign.cjs testdata/normal-map/normal-map.glb`: rc 1, `encodingMismatches 34560`, `slopePixels 25408`, `conventionViolations 24864`, `geometryViolations 0`, `ok false`.
- Regenerated GLB, same check: rc 0, `encodingMismatches 0`, `slopePixels 25408`, `conventionViolations 0`, `geometryViolations 0`, `ok true`.
- `gltf-check testdata/normal-map`: rc 0, `bevy.loaded true`, `decode.ok true`, `bevy_loaded 1`. This refreshed `target/gltf-cache/normal-map.glb`, which the decoded variant loads.
- Visual check, `node scripts/check-normal-map-visual.cjs` on display `:1`, adapter BrowserWebGpu on NVIDIA GeForce RTX 3060 Laptop GPU, rc 0, `ok true`. Raw variant: `modelState loaded`, still logs `Snorm16x4` where `Float32x4` is expected, because the raw file keeps the SHORT tangent. Decoded variant: `modelState loaded`, no TANGENT warning.
- Screenshots: post-fix `target/witness/normal-map-raw.png` and `target/witness/normal-map-decoded.png`. Pre-fix copies of the decoded screenshot kept at `target/witness/pre-sign-fix/normal-map-decoded.png` (and the raw one alongside it, `target/witness/pre-sign-fix/normal-map-raw.png`).
- **The screenshots are a visual check, not a number.** The pre-fix image reads as sunken dimples, with dark crescents on the upper rims. The post-fix image reads as raised domes, with the dark edge on the lower side. This is a judgement from a small 870-pixel image. The numeric checker above is the witness for the sign.
- The two 404 lines per variant are unidentified: the console log doesn't record their URLs. The earlier WebGPU witness showed one favicon 404, so that is a guess and not confirmed. Neither 404 affects `modelState loaded`.

Unit suite after the no-op build: `test result: ok. 48 passed; 0 failed`, including `compact_arc_flags_parse_as_single_digits` and `scripted_colliders_keep_their_shape_kind`.

Timing: this round had no full rebuild, so the wasm release build from Round 11 (217 s) is still the longest measured step. The unit suite took 1 s on the cached build. The browser and fixture runs were not timed.

Process check: the visual witness reported server PID 2295693 and browser PID 2295697; both were gone when checked after the run. No process started this round remains running.

The two unidentified 404s were identified and closed in Round 13 (favicon, see its table).

## Round 13 Phase A changes

Nothing in Phase A was built, tested or run. Each row is one local commit by lanmower with no trailers.

| Item | Commit | Change | Phase B witness |
| --- | --- | --- | --- |
| 1 Unidentified 404s | 46b7de3 | `scripts/check-normal-map-visual.cjs` now records each console error's URL (`message.location().url`), each response with status 400 or more, and each failed request with its error text, per variant, in `failedRequests`. `ok` now also requires no TANGENT warning and no failed request. | Run the witness, read the `failedRequests` URLs, then fix their cause. |
| 2 Raw variant TANGENT warning | e079709 | `scripts/make-normal-map-fixture.cjs` writes the TANGENT accessor as FLOAT32 VEC4 (componentType 5126, no `normalized`), with value (1, 0, 0, 1) per vertex. The SHORT normalized form is what bevy_gltf rejects. The unused `int16Bytes` helper is removed. | Regenerate `testdata/normal-map/normal-map.glb`, then confirm the sign checker still passes and the raw variant logs no TANGENT warning. |

## Round 13 Phase B results

Contention: swap-out 0 and 10,585 MB available before the first browser run and before the wasm build. One cargo invocation ran (the wasm release build). The unit suite ran alone after it.

Item 1, the 404s. The first witness run, before the GLB and the viewer changed, logged these failing URLs:

- `http://127.0.0.1:8092/favicon.ico`, on the raw variant only. The page declared no icon, so the browser asked for the default. Last round this was only a guess; now it has its URL.
- `http://127.0.0.1:8092/testdata/normal-map/normal-map.glb.meta` (raw) and `http://127.0.0.1:8092/target/gltf-cache/normal-map.glb.meta` (decoded). Bevy's asset loader requests a `.meta` sidecar beside every asset, and the wasm viewer used the default `AssetMetaCheck::Always`.

Fixes:

- 318c3b0: `wasm-viewer/src/lib.rs` sets `AssetMetaCheck::Never`. Both fixture assets are plain GLBs with no sidecar, so Bevy's default meta is the intended behaviour.
- def6cc5: the page declares `<link rel="icon" href="favicon.svg">` in `web/wasm-index.html`, and `web/favicon.svg` is copied by `scripts/build-wasm.sh`. `web/wasm/` is generated and ignored by git, so the change lives in the tracked template.

Item 2, raw TANGENT warning. The raw GLB was regenerated with FLOAT32 tangents (e079709 for the generator, f1175a7 for the GLB). The accessor is now componentType 5126, VEC4, no `normalized`. The raw variant no longer triggers the bevy_gltf `Snorm16x4` warning, and the decoded copy is now the same size as the raw one, because `glb::decode` leaves float tangents unchanged.

Witnesses on the final tree:

- Pre-fix visual run (before the GLB and viewer changes): rc 1, `ok false`. The raw variant still logged `Snorm16x4`, and both variants had the two 404s above.
- `node scripts/check-normal-map-sign.cjs testdata/normal-map/normal-map.glb`: rc 0, `encodingMismatches 0`, `slopePixels 25408`, `conventionViolations 0`, `geometryViolations 0`, `ok true`. The generator change keeps the sign fix.
- `target/debug/bevy-jayzee gltf-check testdata/normal-map`: rc 0, `bevy_loaded 1`, `bevy_failed 0`, `ok true`.
- Wasm release build through the shim (`PATH=/config/.local/bin:$PATH WASM_PROFILE=release bash scripts/build-wasm.sh`): rc 0, 59 s wall, incremental on the Round 11 build. The bundle is 31,059,938 bytes and `web/wasm/favicon.svg` and the favicon link are present.
- Post-fix visual run, `node scripts/check-normal-map-visual.cjs` on display `:1` with the adapter flags: rc 0, `ok true`. Both variants: `ready true`, `modelState loaded`, one BrowserWebGpu adapter line each, `tangentWarnings []`, `failedRequests []`, and no console error lines. Screenshots: `target/witness/normal-map-raw.png` and `target/witness/normal-map-decoded.png`. The raw screenshot shows the same raised domes as the decoded one. This is a visual check, not a number.
- The witness's server and browser PIDs (2299012 and 2299016) were gone when checked after the run, and nothing listened on port 8092.
- Unit suite, alone after the build: `test result: ok. 48 passed; 0 failed`, including `compact_arc_flags_parse_as_single_digits`.

Timing: the longest measured step this round is the wasm release build at 59 s wall. The visual witness was not timed. The Round 11 build at 217 s compiled `bevy_gltf` from scratch, so 59 s is the incremental figure.

Caveat, unchanged: the neon GLBs use the SHORT normalized tangent form and are not fixed by this change. This fixture only covers `testdata/normal-map`. The wasm build still cannot decode the neon assets, which use meshopt and KTX2.

Process check: the witness server and browser PIDs listed above are gone, nothing listens on port 8092, and no cargo, rustc or wasm-bindgen process is running. Playwriter processes under PID 2296782 belong to a gm-core worker that started at 01:19:54, before this round's witness runs, so they were left running.

## Round 14 Phase A changes

Nothing in Phase A was built, tested or run. Each row is one local commit by lanmower with no trailers.

Gate for this run: gm connected after the dispatcher restart. Run 2026-11-17-00-09-35-1a554e, relay.log `started 2310604:28740314`, `skill://gm/SKILL.md` read, gm tools answered.

Neon models that need decoding. The scan read all 64 GLBs under `/workspace/neon-syndicates/assets/cdn`. 61 need decoding (EXT_meshopt_compression, KHR_texture_basisu or KTX2). The 12 below also use SHORT normalized TANGENT, and all carry meshopt, basisu and KTX2 together. Those 12 cannot load in the wasm viewer, because it has no basis transcoder:

- value.17c7bd864f7f18268fe20172cd1286704b1c555b18e231e818b259c091f2f09b.glb
- value.212b71b2975cb231665b84b7497be5ed35a01d2d9c2e1465c5e056433976cc12.glb
- value.23187122d2f90dd03e7e7fe0e934d25c43958afbd123479c83dea47d3664ed31.glb
- value.260c985144718b8764c86ef6a7b6f6fdb32e07d104aebcadfafe10e86ae846ee.glb
- value.279b3bbd1aa256bce199b415843ae598ec1e5800a121edcabad594ff33b6f856.glb
- value.3edb0af0d4d0c70b9abeac56a41817781bdea320d91435f0f99a7e4d6588c296.glb
- value.600eb5a37029506ef1b65af5c4e4876484e98b9fbd7ec912ba32fe33ea20622f.glb
- value.970e5de4acda1b16df5bd0f1af737af75ead94b39722ae0eaee2474a428be5b5.glb
- value.98155df60a8033bcbd60d362fd6051b690da3149ef4b39b5810416929b7ded36.glb
- value.c6e98cac813bc144e718d109d8d0415afa1a0956c847854f2c9429589c4b4e03.glb
- value.d056bb42933f2de0008cb52a064446e053072ab7853281d05866236d0c3cbc89.glb
- value.d50992a799663b0660d89ce9114cb89ea3052e357f8afd743dadf243d4ef7365.glb

Cause. `promote` turns KHR_texture_basisu into a plain KTX2 image reference. Only BasisLZ images were rewritten to RGBA8, so the UASTC images with Zstandard supercompression reached the browser unchanged. The wasm build enables Bevy `ktx2` in the native build but not `basis-universal`, so UASTC KTX2 cannot be transcoded in the browser. The 49 meshopt-only models have no basis images but still need decoding, because Bevy 0.20.1 does not read EXT_meshopt_compression.

| Item | Commit | Change | Phase B witness |
| --- | --- | --- | --- |
| 2 Route decoded copies, transcode basis images | 64a7f5a | `src/basisu.rs` renames `transcode_basislz_images` to `transcode_basis_images`, and `is_basislz` to `is_basis_ktx2`. The predicate is now any KTX2 image with vkFormat undefined (basis-encoded: BasisLZ, UASTC, Zstandard or ZLIB supercompressed), and it ignores the supercompression scheme. Images with a real vkFormat are untouched. The unit test is rewritten to check vkFormat. `src/glb.rs` uses the renamed function. | `cargo test --bin bevy-jayzee` passes basis_detection_reads_the_vk_format and images_that_are_not_basis_keep_their_buffer_view. A decode of the 12 names produces no image with a basis vkFormat. |
| 2 Keep the cache across runs | ca8cba3 | `src/gltf_check.rs` no longer calls `remove_dir_all` on `target/gltf-cache`. Before, each `gltf-check <dir>` deleted every other directory's decoded copies, so `normal-map.glb` was lost after the neon run and the reverse. | After `gltf-check` on the neon directory and on `testdata/normal-map`, both sets of copies exist in `target/gltf-cache`. |
| 2 Wasm KTX2 loader | 66ab7dd | `wasm-viewer/Cargo.toml` enables Bevy `ktx2`. `basis-universal` stays off, so the browser build needs no basis transcoder, because the decoded copies hold no basis images. Cargo.lock is not yet updated for `ktx2`; the Phase B `cargo fetch` does that before the locked wasm build. | The wasm release build passes with `--locked` after the lock is updated. |
| 2 Documentation | 15ab1ef | `AGENTS.md` now describes the basis transcoding rule and the cache rule. | none |

Phase B witnesses still to run:
- `gltf-check` on `/workspace/neon-syndicates/assets/cdn`: all 64 decoded, 0 failures, each of the 12 has no basis-encoded image afterwards.
- The wasm viewer on each of the 64 decoded copies, at `?model=target/gltf-cache/<name>`: `modelState loaded`, no console error, no failed request, no TANGENT warning, for every name.
- Neon `main` on the native viewer: `objects 51 = expectedObjects 51`.
- The unit suite, once after the build.

## Round 14 Phase B results

Contention: one cargo invocation at a time, each run alone. The gate read swap-out 0 and 10,975 MB free before the first build. A first gate check reported cargo=2, but that count was the gate's own command text matching the word cargo, and the gate was rerun with a process-name match before any build.

Defect found in Phase B, not caught in Phase A: the Phase A row for item 2 claimed that Zstandard basis images were handled. They were not. `build.rs` sets `BASISD_SUPPORT_KTX2_ZSTD=0`, so the native transcoder cannot start on Zstandard levels. The unit test `basisu_glb_decodes_to_plain_glb_with_ktx2_images` failed with `basis transcoder failed on level 0 (status -1)` on `value.260c985144718b8764c86ef6a7b6f6fdb32e07d104aebcadfafe10e86ae846ee.glb`, which has supercompression 2 (Zstandard). Fix (5ce88c5): `inflate_zstd_levels` in `src/basisu.rs` decodes each Zstandard level with `ruzstd` 0.9.1 (already in the lock through Bevy's `zstd_rust`), clears the scheme, and rewrites the level index, keeping the rest of the file layout. `transcode_basis` runs it first. Images with BasisLZ are unchanged. ZLIB (scheme 3) is not handled; no file in the data uses it, and the transcoder rejects it with an error.

Witnesses on the final tree:

- Unit suite, once after the build: `test result: ok. 49 passed; 0 failed`, including `zstd_levels_are_inflated_and_the_scheme_is_cleared`, `basis_detection_reads_the_vk_format`, `images_that_are_not_basis_keep_their_buffer_view` and `basisu_glb_decodes_to_plain_glb_with_ktx2_images`.
- `target/debug/bevy-jayzee gltf-check /workspace/neon-syndicates/assets/cdn`: rc 0. GLB `total 64, pass 64, decoded 64, bevy_loaded 64, bevy_failed 0`. The decoded copies hold 61 KTX2 images, all `vkFormat 37` with supercompression 0 (RGBA8). `basisLeftCount 0`, which is the count of images with vkFormat 0 left in any decoded copy.
- `target/debug/bevy-jayzee gltf-check testdata/normal-map`: rc 0. The cache now holds 65 entries: the 64 neon copies plus `normal-map.glb`, so the cache is not wiped between runs.
- Wasm release build, `PATH=/config/.local/bin:$PATH WASM_PROFILE=release bash scripts/build-wasm.sh`: rc 0, `Finished release in 56.11s`, `wasm-bindgen 0.2.129`. The bundle is 31,059,942 bytes, 4 bytes more than before the `ktx2` feature. The bundle barely changed, so the browser witness below is the evidence that KTX2 loading works, not the size.
- Wasm witness, 64 of 64 neon models, `?model=target/gltf-cache/<name>` on the wasm build served from `scripts/serve-wasm.sh` on port 8092, in gm's playwriter. Each model: `modelState loaded`, no console error, no page error, no failed request, no TANGENT warning, one BrowserWebGpu adapter line. The 12 Zstandard models are included (12/12 clean). Per-model rows are in `/tmp/neon-scan/r14-wasm-witness.json`, outside the repo. The first pass covered 61 models and 2 batches failed at the browser (a closed target and the 10 s default timeout); the 6 missing models were rerun with `timeout: 300000`, and all 6 loaded clean. The 3 `clip-*` animation GLBs are included in the 64.
- Wasm regression, normal map, both variants: raw `/testdata/normal-map/normal-map.glb` and decoded `/target/gltf-cache/normal-map.glb` both report `modelState loaded`, `errors []`, `tangentWarnings []`, `failedRequests []`. Screenshots `target/witness/r14-normal-map-raw.png` and `target/witness/r14-normal-map-decoded.png`.
- Native neon `main`: `objects 51, expectedObjects 51, duplicateObjects 0, missingObjects [], terrains 1, placeholdersRetired 0, objectKinds {marker 23, model 9, ring 18, terrain 1}`. This is the regression check for the native viewer, which also calls `glb::decode` (`src/viewer_world.rs`).

Timing (wall): lock fetch 0.8 s; dev build 27.2 s; unit suite 7.4 s; gltf-check on neon not timed separately; wasm release build 60.0 s; the 64-model browser witness 287.3 s. The longest step is the 64-model browser witness at 287.3 s, in the pass that produced the recorded results.

Process check: the witness server (gm job `r14-serve`) was stopped with `gm.job_stop`; port 8092 answers `000`. The witness browser tree was stopped by PID (2326182 wrapper and its 8 children, all gone after SIGTERM). gm's project gate process 2325939 remains. It is the project playwriter's gate, not something this round started on its own, so it was left running.

Not committed: `.gitignore` has an added line `.playwriter/`. The gm playwriter added it, not this round; it is left uncommitted for the owner to decide.

Open: none for item 2.

## Round 15 Phase A changes

- `.gitignore` ignores `.playwriter/`, the browser profile that gm's playwriter generates inside the repo. Committed locally as lanmower with no trailers; pushed in the Round 15 Phase B push.

## Round 15 ranked remaining work

Nothing below was started in Round 15. Each rank is the order I would work them, by the risk of a change already claimed as done and by how visible the gap is to the owner. The list is written as a local commit; it is not pushed until the next push.

| Rank | Item | Why this rank | Witness that closes it |
| --- | --- | --- | --- |
| 1 | Native viewer memory and frame cost after the RGBA8 decode change | `glb::decode` now writes RGBA8 for the 12 Zstandard models, and `src/viewer_world.rs` also calls `glb::decode`, so the native viewer loads uncompressed textures it used to load as UASTC. The decoded cache is 271 MB. Neither the memory nor the frame cost has been measured. | Viewer RSS, texture bytes and frame time for neon `main`, before and after the change. The owner then decides: keep UASTC natively and decode only for wasm, or accept RGBA8. |
| 2 | HUD text quality in the native viewer | The HUD text is drawn now (`/tmp/ns-viewer/viewer-1.png` shows the overlay line), but some glyphs render as empty boxes, such as the middle dot, and HUD labels overlap the top-left overlay. This is the owner's text surface. | A viewer PNG with no empty-box glyphs and no overlapping labels, and `uiWarnings` empty. |
| 3 | Release gate on the current tree | Every Phase B witness this stretch used the dev binary. `scripts/verify.sh` (release build, bench 500/2000/5000, det 2000, server and client, `check-network`) has not run since Round 10. | `scripts/verify.sh` exit 0, with its output quoted. Cost: one release build. |
| 4 | Wasm payload for the decoded neon copies | The 12 Zstandard models now ship RGBA8 KTX2 to the browser. Their download size and load time are not measured, and the 64-model witness took 287 s wall. | Per-model byte size and load time in the wasm viewer, and a decision on a compressed wasm format (BC or ASTC). |
| 5 | Commit the browser witnesses as a repo script | The 64-model and normal-map wasm witnesses ran as gm snippets. Their only record is `/tmp/neon-scan/r14-wasm-witness.json`, outside the repo. | A script in the repo that reproduces 64 of 64 from a clean checkout. |
| 6 | Move `scripts/check-normal-map-visual.cjs` onto gm's playwriter | The owner's browser rule is gm's playwriter setup, but this script still calls the playwriter binary directly. | The script, or its gm replacement, reproduces both normal-map variants. |
| 7 | Reconcile `docs/PENDING_VALIDATION.md` | Old rows still say "pending Phase B" (Round 2) or "open" (the Round 1 HUD row). The "Ranked remaining work" section near line 231 lists items already fixed, such as the HUD path curves. A reader cannot tell what is open. | Each row marked with its current witness, and the stale section removed. |
| 8 | ZLIB-supercompressed basis KTX2 is not handled | No file in the data uses it. The transcoder rejects it with an error, so it fails loudly rather than silently. | A ZLIB sample that decodes, or a clear error from `gltf-check`. |
| 9 | AGENTS.md size rule (under 30 KB) | The 30 KB limit is a standing rule. This stretch added lines to AGENTS.md without measuring its size. | `wc -c AGENTS.md` under 30720 after an audit. |

Blocked or not bevy items, not ranked:
- Build speed: `dynamic_linking` is skipped by decision; sccache is not installed and sudo is blocked on this host.
- The two neon texture PNG conversions exist only in `/workspace/neon-syndicates`, outside git.
- The gm playwriter gate tree still running on this host: `2313609` and its child `2327190`, with Chromium `2327217`. Gm owns these.

## Round 16 Phase A changes

- Item 6, HUD glyphs: `src/ui_html.rs` now maps every character outside printable ASCII to ASCII when it builds a text run (`push_glyph`). The font Bevy ships, `FiraMono-subset.ttf` in `bevy_text-0.20.1/src`, has 95 codepoints, which is exactly 0x20 to 0x7E, so each neon HUD character outside that range drew as a box. Before the change, the neon sources used 38 distinct non-ASCII codepoints and the viewer's own overlay used 3, all outside the 95. `uncovered_glyph_count` counts any character still outside the range. Witness, Phase B: `cargo test --bin bevy-jayzee glyph_fallback_keeps_text_inside_the_font` passes; `cargo test --bin bevy-jayzee ui_html` passes; viewer run on neon `main` writes a status with `uiUncoveredGlyphs` 0 and a screenshot `target/witness/r16-hud.png` with no box glyphs.
- Item 6, overlap: `src/viewer.rs` moves the viewer's overlay from the top-left corner (`top: 16`) to the bottom-left (`bottom: 16`), where the neon HUD draws nothing; the overlay's `•` and `…` are ASCII now. Witness, Phase B: the same screenshot shows the overlay in the bottom-left and no label overlapping it at the top-left.
- Item 1, normal-map visual script: `scripts/check-normal-map-visual.cjs` now runs through `scripts/lib/playwriter-witness.cjs`, which drives gm's project gate (`playwriter-project`, the gm playwriter setup) instead of starting its own Chromium and the playwriter CLI. The gate's browser pid is read from `.playwriter/browser.pid` before and after the run; only a pid the run started is stopped, by its pid tree. Witness, Phase B: `node scripts/check-normal-map-visual.cjs` exits 0 and prints `"ok": true`, with both variants `modelState` loaded, `failedRequests` empty, `tangentWarnings` empty, and screenshots `target/witness/normal-map-raw.png` and `normal-map-decoded.png` written.
- Item 2, browser witness scripts: `scripts/check-neon-wasm-models.cjs` (new) runs the decoded copies of every neon GLB in `assets/cdn` through the gate, in batches of 8, each batch its own playwriter call with a 300 s timeout, and writes `target/witness/neon-wasm-witness.json`. It needs `target/gltf-cache` filled by `gltf-check` first. Witness, Phase B: `node scripts/check-neon-wasm-models.cjs` exits 0 with `summary.clean` equal to `summary.total` (64), `summary.notClean` empty and `summary.missingNames` empty. Item 8's payload and load-time numbers come from the same file, under `summary.glbEncodedBytesTotal`, `summary.loadMsMedian` and `summary.wasm`.
- Item 4, reconciliation: removed the Round 9 "Ranked remaining work for bevy" section, whose items are fixed, and marked the "pending Phase B" rows of rounds 2, 3, 8 and 9 with their round's Phase B pass (27 rows). The top HUD row now names Round 16, the globe `FAILS` row is a pass, and the Round 5 terrain notes and the Round 10 compact-arc note name the commits that fixed them (terrain 8131a6c, arc flags d75c869). The Round 12 404 note now names its close in Round 13. Witness, Phase B: `grep -n 'pending Phase B' docs/PENDING_VALIDATION.md` prints no line above the `## Round 10 Phase A changes` heading, and `grep -c '^## Ranked remaining work for bevy' docs/PENDING_VALIDATION.md` prints 0.
- Item 5, AGENTS.md size: `wc -c AGENTS.md` is 10952 bytes, under the 30 KB limit (30,720 bytes), so nothing was trimmed. Witness, Phase B: the same `wc -c` line, read again on the final tree.
- Not in this stretch: ZLIB-supercompressed basis KTX2 images (scheme 3). No file in the data uses them, and the transcoder rejects them with an error rather than decoding them silently. Decision recorded here; it is not an open item.

## Round 16 Phase B results

Contention: the gate read swap-out 0 KB/s, 11,141 MB free and no cargo or rustc before each cargo invocation; the release builds were run one at a time, and no unit suite ran alongside a build.

Defects found in Phase B and fixed:

- `src/client.rs`: `report_and_exit` ran in `FixedUpdate`, so when a frame ran two fixed ticks after the 5 s mark it printed two `client-report` lines, and `check-network.cjs` requires exactly one. `verify.sh` failed on that (`singleClientReport` false, all dependent checks false) on the first release run. Fixed with a `Local` guard (ebf98da). Re-run on the final tree: pass.
- `src/viewer.rs`: the scene title in the overlay still had an em dash, which drew as a box. `uiUncoveredGlyphs` reported 1, which is that dash; the title is ASCII now (95d1b99). Re-run: 0.
- `scripts/lib/playwriter-witness.cjs` and the two witness scripts, found by running them: `session new` without `--direct` tries the Playwriter extension and fails (`extension_not_connected`); a gate lease is released between calls, so the browser stopped between batches (`ECONNREFUSED 127.0.0.1:10232`); the CLI truncates output at 10,000 characters, which cut the JSON; one tab per model made Chromium fail (`Failed to open a new tab`, then the browser closed after 7 models). Fixed by connecting with `--direct` to the gate browser's debug port, one playwriter call per witness, one page navigated per model, and results written to a file by the snippet (30d104a). Earlier failed runs are not counted; the final run is below.
- Open limitation, not fixed: `playwriter-project` must be on `PATH`. The gm shell does not have `/config/.gmnew/pw/bin` on `PATH`, so the witnesses are run with `PATH=/config/.gmnew/pw/bin:$PATH`.

Build and unit witnesses (cargo through `/config/.local/bin/cargo` only):

- `cargo build --bin bevy-jayzee`: rc 0, 21.6 s, no warnings. After the two later edits: rc 0, 5 s and 17 s.
- `cargo test --bin bevy-jayzee`: `test result: ok. 50 passed; 0 failed`, including `ui_html::tests::glyph_fallback_keeps_text_inside_the_font`.
- `WASM_PROFILE=release scripts/build-wasm.sh`: rc 0, cargo finished in 0.73 s (the wasm-viewer crate did not change), `wasm-bindgen 0.2.129`. Bundle `bevy-jayzee-wasm_bg.wasm` 31,059,942 bytes.
- `cargo build --release --bin bevy-jayzee`: rc 0 twice, 10 m 21 s (621.7 s) before the client fix and 10 m 15 s (614.9 s) on the final tree. The second is the longest step of this round.

Gltf cache: `gltf-check` on `/workspace/neon-syndicates/assets/cdn` rc 0 in 3 s and on `testdata/normal-map` rc 0. The cache holds 65 GLBs.

Witnesses:

- Item 1, `node scripts/check-normal-map-visual.cjs` (with the gate's `bin` on `PATH`): `ok: true` in 12 to 13 s. Raw and decoded variants: `modelState` loaded, one `BrowserWebGpu` adapter line each (NVIDIA GeForce RTX 3060 Laptop GPU), `tangentWarnings` empty, `errors` empty, `failedRequests` empty. Screenshots `target/witness/normal-map-raw.png` and `normal-map-decoded.png`. The script's browser (pid 2342921 in the last run) was stopped by pid.
- Item 2, `node scripts/check-neon-wasm-models.cjs`: `ok: true`, `total` 64, `clean` 64, `notClean` [], `missingNames` []. One playwriter call, 166.8 s. Rows: `target/witness/neon-wasm-witness.json`.
- Item 8, from the same run: `loadMsMedian` 1,092 ms, `loadMsMax` 2,421 ms (goto to `data-model-state` loaded). Wasm bundle transfer 31,060,242 bytes. Decoded GLB transfer total 283,092,200 bytes, largest 37,106,376 bytes. The browser and gate started by the script were stopped by pid; none of them remain.
- Item 3, `bash scripts/verify.sh` on the final tree (release binary): exit 0, 7.1 s wall. Bench 500 bodies: mean 0.3220 ms, p95 0.4543, hash 0x17ff9626. Bench 2000: mean 0.7825 ms. Bench 5000: mean 1.3336 ms. Determinism 2000 bodies, 300 ticks: `pass: true`, hash 0x3a5f0b51 on both runs. `check-network.cjs`: `ok: true`, `singleClientReport` true, `errorM` 0, `moved` 14.505 m, every one of the 11 checks true. The release build is the only run of verify.sh that counts: the first run failed on the client defect above.
- Item 6, HUD: viewer on neon `main` on display `:1` (window 898 by 579), screenshot `target/witness/r16-hud.png`. `viewer-status`: `uiUncoveredGlyphs` 0 (the earlier shot had 1, the em dash), `uiWarnings` [], `uiNodes` 718, `objects` 51 = `expectedObjects` 51, `duplicateObjects` 0, `missingObjects` [], `terrains` 1, `modelsLoaded` 9, `modelsMissing` []. The overlay draws in the bottom-left with no glyph boxes and no longer covers the neon HUD at the top-left.
- Open, HUD: the neon HUD's own top-left labels still overlap each other in the screenshot ("NEW DAWN" over "Threshold", "HELIX" over the line below). They come from absolutely positioned boxes that the HTML renderer in `src/ui_html.rs` lays out approximately. Not fixed in this round; the owner should decide whether to spend on the renderer's positioning.
- Item 7, native viewer (debug binary, release is not used by the viewer): RSS 898,224 kB at 25 s, high-water mark 898,224 kB, 896,168 kB at 35 s. Frame time from the HUD: fps 61.0, frame 16.39 ms. Cache: 283,121,864 bytes (270 MiB) for 65 GLBs; the originals in `assets/cdn` total 37,147,716 bytes, so the decoded set is 7.6 times larger. KTX2 images: 61 in the originals (51 Zstandard, 10 BasisLZ) and 61 in the cache, all `vkFormat` 37 (RGBA8) with no supercompression. Texel footprint with mips: RGBA8 242,308,356 bytes against 60,578,736 bytes if they stayed BC7, a 4 times difference on this GPU. The figures cover all 61 images; the viewer loads 9 models, so this is an upper bound for the native viewer. Decision: no cache change, because the measurement shows a trade-off and not a defect. The owner can choose BC7 for the native path later.
- Process check: no chromium, playwriter gate, witness server or viewer process started this round remains (`ps` counts 0 for each). The gm gate that was running at the start of Round 15 had already exited.

Not done in this round, recorded: ZLIB-supercompressed basis KTX2 images (scheme 3), decided in Phase A, no file in the data uses them.

Round 16 items: 1 normal-map witness through the gate (done); 2 browser witness scripts in `scripts/` (done); 3 verify.sh once on the release binary (done, after one defect fix); 4 doc reconciliation (done); 5 AGENTS.md size, 10,952 bytes under 30,720 (done, nothing trimmed); 6 HUD glyph coverage and overlap (glyphs done; the neon HUD's own top-left label overlap remains open); 7 native measurement (done); 8 wasm payload (done).

## Round 17 Phase A

Phase A ran no build, test, gate or witness. Every item below is code, docs or a witness script that Phase B runs. Each change is a local commit by lanmower with no trailers; nothing is pushed.

Changes:

- 399f533, `src/ui_html.rs`: `parse_val` reads `calc(...)` by summing its px terms, or its percent terms when any are present, because `Val` has no calc variant. The mixed case (`calc(24% + 200px)`) keeps only the percent term. This is the hypothesis for the neon top-left overlap: the squad strip `#tl` (`top: calc(18px + var(--spawn-safe-area-top,0px))`) and `#topc` used calc insets, which fell back to `auto` and piled up at the origin. Not verified until Phase B's overlap witness runs. Unit tests: `calc_insets_resolve_to_the_summed_terms`, `overlapping_pairs_counts_only_real_intersections`.
- 399f533, `src/ui_html.rs` and `src/viewer.rs`: `overlapping_pairs` counts strict rectangle intersections; `write_browser_report` writes `uiLabels` and `uiLabelOverlaps` into `web/status-<place>.json`, from every non-empty `Text` node's `ComputedNode` size and `UiGlobalTransform` centre.
- cadcf9d, `scripts/check-hud-overlap.sh` (new), `scripts/xwd2png.py` (copied from the Round 16 `/tmp` tool): runs the native viewer on a place (`PLACE`, default `main`, world `/workspace/neon-syndicates/world`), waits for `uiLabels > 0`, takes `target/witness/neon-hud.png`, and writes `target/witness/hud-overlap-main.json`. Its `ok` is true only when `uiLabelOverlaps` and `uiUncoveredGlyphs` are both 0.
- 4388b17, `native/basislz_ktx2.cpp`, `src/basisu.rs`, `src/glb.rs`, `src/gltf_check.rs`, `src/main.rs`: `TextureTarget { Rgba8, Bc7 }`. The transcoder takes a `bc7` flag and writes `cTFBC7_RGBA` sized in 4x4 blocks. `ktx2_bytes(target, ...)` writes vkFormat 37 (RGBA8) or 145 (BC7 UNORM) with a matching DFD (BC7 model 134, 4x4 texel blocks, 16 bytes per block). `transcode_basis(input, target)`, `transcode_basis_images(json, bin, target)`, and `glb::decode_with(bytes, target)`; `glb::decode` keeps the RGBA8 default, so the viewer's `load_model` is unchanged. `gltf-check <dir> <project> bc7` writes `target/gltf-cache-bc7`; with no third argument it writes `target/gltf-cache` as before. Unit test: `bc7_ktx2_declares_bc7_and_sizes_levels_in_blocks`.
- cadcf9d, `scripts/check-neon-wasm-models.cjs`: `NEON_TEXTURE=bc7` loads `target/gltf-cache-bc7` instead. Outputs are now per texture: `target/witness/neon-wasm-witness-<rgba8|bc7>.json` and `neon-rows-<texture>.json` (the old `neon-wasm-witness.json` and `neon-rows.json` names are gone). Each model also writes a screenshot to `target/witness/neon-shots-<texture>/<model>.png`.
- cadcf9d, `scripts/texture-footprint.cjs` (new): over the Zstandard models (those whose source KTX2 images use supercompression 2), it compares the cache GLB bytes and the KTX2 level payloads for both targets, reads the two neon witnesses and `target/witness/texture-visual.json`, and writes `target/witness/texture-footprint.json` with a `decision`.
- cadcf9d, `scripts/compare-screenshots.cjs` (new): decodes each pair of `neon-shots-rgba8` and `neon-shots-bc7` PNGs (8-bit RGB or RGBA, all five filters), computes PSNR per pair, and writes `target/witness/texture-visual.json`. A pair is ok when it is identical or its PSNR is at least `VISUAL_PSNR_MIN` (30 dB).

BC7 decision criteria, fixed before any measurement:

1. Footprint: over the Zstandard models, the total BC7 KTX2 level payload is below the RGBA8 total, and the total BC7 cache GLB bytes are below the RGBA8 total. Every model must exist in both caches.
2. Load time: the BC7 neon `loadMsMedian` is at most 1.10 times the RGBA8 median in the same host session (`LOAD_TOLERANCE`).
3. Clean: the BC7 neon witness has `clean == total` with no missing names.
4. Visual: `texture-visual.json` has `ok: true`, so every paired screenshot is identical or at least 30 dB PSNR.

If all four hold, Phase B switches the decoded cache to BC7: the `gltf-check` default target, the `glb::decode` default, and the viewer's `load_model` decode. If any one fails, the cache stays RGBA8, and the failing criterion is recorded in the Round 17 Phase B results. The RGBA8 numbers from Round 16 are 242,308,356 bytes of GPU texels with mips against 60,578,736 for BC7 on the same texel count (4x); the new measurement replaces that estimate with the cache's own numbers.

Phase B plan, in this order, one cargo at a time and no unit suite alongside a build:

1. `/config/.local/bin/cargo build --bin bevy-jayzee` (dev), then `cargo test --bin bevy-jayzee`.
2. `/config/.local/bin/cargo build --release --bin bevy-jayzee`, then `bash scripts/verify.sh` once.
3. `target/debug/bevy-jayzee gltf-check /workspace/neon-syndicates/assets/cdn` (RGBA8 cache) and `... gltf-check /workspace/neon-syndicates/assets/cdn <project> bc7` (BC7 cache, with the project argument before the target).
4. `scripts/check-hud-overlap.sh` on neon `main` (native viewer, display `:1`); read `hud-overlap-main.json` and `neon-hud.png`.
5. `NEON_TEXTURE=rgba8 node scripts/check-neon-wasm-models.cjs`, then `NEON_TEXTURE=bc7 node scripts/check-neon-wasm-models.cjs` (both through the gm gate; `playwriter-project` must be on `PATH`).
6. `node scripts/compare-screenshots.cjs`, then `node scripts/texture-footprint.cjs` for the decision.
7. `node scripts/check-normal-map-visual.cjs`, the Round 16 normal-map witness, to confirm the normal-map path still works after the `decode` change.

Friction item for gm (recorded, not fixed in code):

- `playwriter-project` lives in `/config/.gmnew/pw/bin`, but that directory is not on the gm shell's `PATH`. Repo witnesses (`scripts/lib/playwriter-witness.cjs`) look the binary up on `PATH`, so they fail with `spawnSync playwriter-project ENOENT` unless the caller prefixes `PATH=/config/.gmnew/pw/bin:$PATH`. The repo does not hardcode the path. gm's launcher should put the playwriter bin on `PATH` for the shell it runs repo commands in, or expose the gate binary path, so witnesses run without a manual prefix.

## Round 17 Phase B results

Contention: the gate read swap-out 0 KB/s, 10,551 MB free and no cargo before the first build, and 10,266 MB free before the release build. One cargo invocation ran at a time; no unit suite ran alongside a build.

Defects found in Phase B and fixed:

- Neon HUD overlap, root cause: the top-left compass and district tags are `data-world-anchor` elements. Their position comes from projecting a world point, which the native viewer does not do, so `ui_html` placed them all at the origin as full-width rows. The `calc()` insets from Phase A were not the cause; the first witness run still showed 23 overlapping pairs. Fix (7180a93): `convert` drops `data-world-anchor` elements and reports them as `unsupported element <div data-world-anchor> dropped (n)`. The native viewer therefore does not draw world-anchored tags; that is a recorded limitation, not an overlap.
- Viewer overlay: the viewer's own bottom-left text box spanned the full width and covered the `?` chip. It is now anchored bottom-right with `max_width: 65%` (7180a93).
- `scripts/texture-footprint.cjs`: `cleanOk` compared the 64-model BC7 witness with the 12-model row count, so it could not pass. Fixed in its own commit.

Witnesses:

- Overlap, `scripts/check-hud-overlap.sh` on neon `main` (native viewer, display `:1`): first run after the calc change: `uiLabels` 22, `uiLabelOverlaps` 23. After the anchor drop: 17 and 4 (the overlay colliding with the race block and the zone banner). After the overlay move, final: `uiLabels` 17, `uiLabelOverlaps` 0, `uiUncoveredGlyphs` 0, exit 0. Screenshot `target/witness/neon-hud.png`; numbers in `target/witness/hud-overlap-main.json`. Remaining width artifact, not an overlap: the zone banner `#dz` is a full-width box because `width: max-content` is not parsed, so its text is centred inside a 898 px box.
- Neon browser witness, RGBA8 (`NEON_TEXTURE=rgba8`): `ok: true`, 64 of 64 clean, `loadMsMedian` 1,102 ms, `loadMsMax` 3,703 ms (the last of two runs; the first was 3,454), GLB encoded total 283,092,200 bytes. Witness `target/witness/neon-wasm-witness-rgba8.json`.
- Neon browser witness, BC7 (`NEON_TEXTURE=bc7`): `ok: true`, 64 of 64 clean, `loadMsMedian` 1,112 ms, `loadMsMax` 1,933 ms, GLB encoded total 101,358,620 bytes. Witness `target/witness/neon-wasm-witness-bc7.json`. WebGPU accepted the BC7 KTX2 images in the browser (vkFormat 145 mapped to `Bc7RgbaUnorm`).
- Native `gltf-check`: RGBA8 cache and BC7 cache both `bevy_loaded` 64, `bevy_failed` 0, `decode_failed` 0, `basisu_textures` 61, `ktx2_images` 61.
- Footprint, the 12 Zstandard models (`scripts/texture-footprint.cjs`): texel payload 242,308,356 bytes RGBA8 against 60,578,736 BC7, ratio 0.2500. Cache GLB bytes 281,422,780 against 99,689,200.
- Visual check, `scripts/compare-screenshots.cjs` over the 64 paired neon screenshots: BC7 against RGBA8 median 33.3 dB, minimum 15.8 dB, below the 30 dB threshold in some pairs. Three of the 12 Zstandard pairs in the first run were below 30 dB (28.0, 28.6, 28.6).
- Control, the reason the visual check was not accepted: a second RGBA8 run compared with the first RGBA8 run gave median 29.4 dB, minimum 15.5 dB, and 36 of 64 pairs below 30 dB. The screenshots are timing-sensitive, so this metric measures run-to-run noise as much as codec error. It cannot show that BC7 holds. A texel-level comparison (decode BC7 and compare with the RGBA8 level 0) is the right measurement and is not done; it is recorded as open.
- Normal-map witness, `scripts/check-normal-map-visual.cjs` through the gate: `ok: true`, raw and decoded variants `loaded`, one adapter each, no tangent warnings, no failed requests.

BC7 decision: `rgba8`. The four criteria from Phase A: footprint wins (true), load within 1.10 times (true, 1,112 against 1,102), clean (true, 64 of 64), visual holds (false, and the control shows the check is not discriminative at 30 dB). Because the visual criterion does not hold, the decoded cache stays RGBA8. The footprint result is recorded: BC7 would cut texel memory by 4× and GLB transfer by 2.8×. Nothing was switched; `gltf-check` defaults and `glb::decode` are unchanged.

Build and unit witnesses:

- `/config/.local/bin/cargo build --bin bevy-jayzee`: rc 0, no warnings (25.5 s first build; 15.6 s and 5.6 s after edits).
- `/config/.local/bin/cargo test --bin bevy-jayzee`: `test result: ok. 54 passed; 0 failed`, including `calc_insets_resolve_to_the_summed_terms`, `overlapping_pairs_counts_only_real_intersections`, `world_anchored_elements_are_dropped_with_a_warning`, `bc7_ktx2_declares_bc7_and_sizes_levels_in_blocks`.
- `/config/.local/bin/cargo build --release --bin bevy-jayzee`: rc 0, 10 m 39 s.
- `bash scripts/verify.sh` on the release binary, once: exit 0. Bench 500 bodies mean 0.3132 ms (hash 0x17ff9626); 2000 bodies mean 0.8208 ms; 5000 bodies mean 1.4545 ms. Determinism 2000 bodies: `hashA` = `hashB` = 0x3a5f0b51, `pass: true`. Network check: `ok: true`, `errorM` 0, `confirmedTick` 354, all 11 checks true. These bench means are not compared back-to-back with the Round 16 host numbers, so they are not called a regression or an improvement.

Process check: every browser, gate and witness server started by this round's runs has exited (`ps` shows none of their PIDs). Processes belonging to gm's other run (gm worker `2026-11-17-02-11-29-7857f6`, its playwriter gate and Chromium under `/tmp/gm-first-open-*`) are running and were not touched.

Untracked, not committed: `web/status-main.json`, written by the native viewer on every run. The viewer writes it and it is not in `.gitignore`; it is left in the working tree.

Round 17 items: 1 neon HUD overlap, fixed and witnessed (0 overlaps; screenshot kept). 2 BC7 against RGBA8, measured; cache stays RGBA8 with the reason recorded; the texel-level visual measurement is open. 3 PATH, recorded as a gm friction item in Phase A; the witnesses still need `PATH=/config/.gmnew/pw/bin:$PATH`.

## Round 18 Phase A

State of `32b4d25`: the Round 17 witnesses that ran and passed are the HUD overlap witness (0 pairs), the full BC7 cache witness (64 of 64 clean, decision recomputed with the 44bc3a1 `cleanOk` fix), the RGBA8 neon witness, native `gltf-check` for both caches, the footprint measurement, the normal-map witness, `cargo test` (54 passed) and `verify.sh` (exit 0). Open: visible quality. The screenshot PSNR criterion did not measure BC7, because an RGBA8-against-RGBA8 control scored the same, so the `rgba8` decision rested on a criterion that cannot fail or pass on codec error.

Changes, each a local commit by lanmower with no trailers; no build, test or witness ran in Phase A:

| Change | Files | Witness it needs (Phase B) |
| --- | --- | --- |
| `glb::ktx2_images` returns the KTX2 image bytes of a GLB in image order | `src/glb.rs` | dev build; `texture-quality` run below reads both caches through it |
| `bevy-jayzee texture-quality <dir>`: for each Zstandard model, transcodes the same basis source to RGBA8 and BC7, decodes every BC7 block with `bcdec_rs` 0.2.1, and reports per-image PSNR (RGBA, level 0 and all levels) and the largest channel error | `src/texture_quality.rs`, `src/main.rs`, `Cargo.toml` | `target/debug/bevy-jayzee texture-quality /workspace/neon-syndicates/assets/cdn > target/witness/texture-texel.json`; 12 models listed; no errors |
| Visual criterion is the texel measurement, fixed before measuring: every image's level-0 PSNR at least 35 dB and the median at least 40 dB. The screenshot PSNR is still reported as `screenshots`, without gating | `scripts/texture-footprint.cjs` | `node scripts/texture-footprint.cjs` prints `visual` from `texture-texel.json` and a `decision` |
| If the decision is `bc7`: the decoded cache default becomes BC7 (`gltf-check` default target, `glb::decode`) | Phase B only, conditional | `gltf-check` rc 0 on the switched cache; RGBA8-to-BC7 neon witness 64 of 64 on `target/gltf-cache`; normal-map witness; HUD overlap witness; `cargo test`; `verify.sh` |

Risk recorded before measuring: a BC7 cache needs `texture-compression-bc` on the GPU. Desktop WebGPU (the witness host, NVIDIA) has it; most mobile GPUs do not, and there the BC7 images would not load. None of the witnesses cover a device without BC support.

## Round 18 Phase B results

Contention: the gate read swap-out 0 KB/s and over 10 GB free with no cargo before each build. One cargo ran at a time, and the unit suite never ran alongside a build. The release build ran in the background under `timeout 1500` with its PID in `target/witness/build-release.pid`; it was waited on with bounded `tail --pid` calls, not a Monitor.

Rows from Round 18 Phase A, with their witnesses:

| Change | Witness | Result |
| --- | --- | --- |
| `glb::ktx2_images` | dev build, then used by `texture-quality` | build rc 0, no warnings; `texture-quality` read every image with no errors |
| `texture-quality` subcommand (`bcdec_rs` 0.2.1) | `target/debug/bevy-jayzee texture-quality /workspace/neon-syndicates/assets/cdn` → `target/witness/texture-texel.json` | `ok: true`, 12 models, 61 basis images, level-0 PSNR minimum 41.70 dB (`value.279b3bbd` image 2), median 53.51 dB, largest channel error 69; `errors` empty. Values that high also show the BC7 decoder is decoding correctly |
| Texel criterion gates the decision | `node scripts/texture-footprint.cjs` | `decision: bc7`; footprint, load, clean and visual all true |
| BC7 becomes the default decoded cache (526406e): `glb::decode` and plain `gltf-check` write BC7 to `target/gltf-cache`; `gltf-check <dir> <project> rgba8` writes `target/gltf-cache-rgba8`; the neon witness defaults to BC7 and takes `NEON_TEXTURE=rgba8`; AGENTS.md updated (11,198 bytes) | the witnesses below | all pass |

BC7 measurement on the final build:

- Footprint over the 12 Zstandard models: texel payload 60,578,736 bytes BC7 against 242,308,356 RGBA8 (0.2500); cache GLB bytes 99,689,200 against 281,422,780.
- Load (neon browser witness through the gm gate, 64 models each): BC7 on `target/gltf-cache` `ok: true`, 64 of 64 clean, median 1,103 ms, max 2,800 ms, GLB transfer 101,358,620 bytes. RGBA8 on `target/gltf-cache-rgba8` `ok: true`, 64 of 64 clean, median 1,122 ms, max 3,524 ms, GLB transfer 283,092,200 bytes.
- Visible quality: texel PSNR above. The screenshot PSNR is reported and does not gate: median 32.2 dB, minimum 15.6 dB, the same range as the RGBA8-against-RGBA8 control from Round 17.
- Decision: BC7. The cache switch is in 526406e.

Other witnesses on the final tree:

- `cargo test --bin bevy-jayzee`: 54 passed, 0 failed.
- `gltf-check` on the neon CDN: default (BC7) `bevy_loaded` 64, `bevy_failed` 0, `decode_failed` 0; `rgba8` the same. `gltf-check testdata/normal-map`: `bevy_loaded` 1, `bevy_failed` 0.
- `scripts/check-normal-map-visual.cjs`: `ok: true`, raw and decoded both `loaded`, one adapter, no tangent warnings, no failed requests.
- `scripts/check-hud-overlap.sh` on neon `main`, viewer now loading BC7 models: `uiLabels` 17, `uiLabelOverlaps` 0, `uiUncoveredGlyphs` 0, `modelsLoaded` 9, `modelsMissing` [], exit 0; screenshot `target/witness/neon-hud.png`. The 15 `Unsupported vertex attribute format` warnings in the viewer log are the same count as Round 16's log and are not texture related.
- Release build: rc 0, 10 m 33 s. `bash scripts/verify.sh` once on it: exit 0; bench means 0.4168 / 0.7683 / 1.3624 ms at 500 / 2000 / 5000 bodies; determinism `hashA` = `hashB` = 0x3a5f0b51; network check `ok: true`, `errorM` 0, `singleClientReport` true. The bench means are not compared back-to-back with an earlier run, so they are not called a regression or an improvement.
- Processes: every browser (2396428, 2403629, 2406205), gate (2396407, 2403614), viewer (2406502) and the release build (2406890) started this round has exited.

Open, recorded:

- Portability: the BC7 cache needs `texture-compression-bc`. The witness host has it (NVIDIA, desktop WebGPU). A device without it (most mobile GPUs) cannot use the BC7 images; `gltf-check ... rgba8` produces the RGBA8 cache, but nothing serves it automatically to such devices.
- gm PATH: gm's fix (5f0b8ce, documented in f47d502) puts `.gmnew/pw/bin` first on the gm shell PATH. The gm host running this session is `2d72a75`, before that fix: a fresh gm shell still has no `playwriter-project` on PATH, and `command -v playwriter` names `/data/cache/npm-global/bin/playwriter`, which has no project gate. The witnesses ran with `PATH=/config/.gmnew/pw/bin:$PATH`; the repo still does not hardcode the path. The lib needs no change once the host is updated.

## Round 19 Phase A

gm check, one call: the gm worker's executable is `/config/.gmnew/versions/29e97a7-1/gm-core`, and in a fresh gm shell `command -v playwriter-project` prints `/config/.gmnew/pw/bin/playwriter-project` and `command -v playwriter` prints `/config/.gmnew/pw/bin/playwriter`. Browser witnesses from here on run in the gm shell with no PATH prefix. The Round 17/18 friction item is closed.

Decision, runtime texture format: the BC7 default is correct only on a GPU with `TEXTURE_COMPRESSION_BC`; elsewhere a BC7 image does not load. The format is now chosen at runtime from Bevy's `CompressedImageFormatSupport` (the flag Bevy's own image loader uses, filled from the device features): BC7 when it contains `BC`, RGBA8 otherwise. This is a platform limit with a runtime fix, not an owner decision.

Changes, each a local commit by lanmower with no trailers; no build, test, gate or witness ran in Phase A:

| Change | Commit | Files | Witness it needs (Phase B) |
| --- | --- | --- | --- |
| `TextureTarget::for_gpu(bc_supported)` and `name()`; unit test `gpu_without_bc_falls_back_to_rgba8` | 2ffe41b | `src/basisu.rs` | `cargo test` |
| Native viewer: `mirror_world` reads `CompressedImageFormatSupport`, decodes each model with that target, and writes `textureTarget` to `web/status-<place>.json`. `glb::decode` is now test-only, since every runtime caller names its target | 2ffe41b | `src/viewer_world.rs`, `src/viewer.rs`, `src/glb.rs` | dev build with no warnings; HUD witness reports `textureTarget: "bc7"` on this NVIDIA host and `modelsMissing` 0 |
| Wasm viewer: `spawn_model` runs in `Update` once `CompressedImageFormatSupport` exists; a `?model=` path under `target/gltf-cache/` is mapped to `target/gltf-cache-rgba8/` when `BC` is missing; `#app-status` gets `data-texture-format` (`bc7`, `rgba8` or `source`), `data-model-path` and `data-bc-supported` | 2ffe41b | `wasm-viewer/src/lib.rs` | `WASM_PROFILE=release scripts/build-wasm.sh`; the fallback witness below |
| `scripts/check-texture-fallback.cjs`: through the gm gate, a control page with BC visible must report `bc7` and fetch from `target/gltf-cache`; then, with an init script hiding `texture-compression-bc` from `navigator.gpu.requestAdapter()`, each of the 64 neon models must load, report `rgba8` and `data-bc-supported=false`, fetch its GLB from `target/gltf-cache-rgba8` and not from `target/gltf-cache`, and log no errors | 7e555fb | `scripts/check-texture-fallback.cjs` | `node scripts/check-texture-fallback.cjs`: `good` 64 of 64 and `controlOk` true |
| Viewer status also reports `uiWindowWidth`, `uiFullWidthLabels` (labels at least 95% of the window width) and up to 60 `uiLabelBoxes`; the HUD witness also requires `uiFullWidthLabels` 0 and no missing models | 2ffe41b, 60ede4c | `src/viewer.rs`, `scripts/check-hud-overlap.sh` | `scripts/check-hud-overlap.sh` exit 0 |
| Item 3, the last reachable open item (Round 17's zone banner): an absolute box with `width: max-content` (or `fit-content`, `min-content`) and auto horizontal margins drops its left and right insets and gets `align_self: center`, so it shrinks to its text and centres instead of spanning the window; unit test `max_content_boxes_with_auto_margins_shrink_and_centre` | 60ede4c | `src/ui_html.rs` | `cargo test`; HUD witness `uiFullWidthLabels` 0 and the screenshot shows the banner centred |
| `/web/status-*.json` in `.gitignore` (covers `web/status-main.json` and other places the viewer writes) | 081f14e | `.gitignore` | `git check-ignore -v web/status-main.json` names the rule; `git status` clean |

## Round 19 Phase B results

Contention: before each build the gate read swap-out 0 KB/s, over 10 GB free and no cargo. One cargo ran at a time; the unit suite ran alone. The release build ran in the background under `timeout 1500` (PID 2426285) and was waited on with bounded `tail --pid` calls. Browser witnesses ran as gm jobs (`job_start` with a 15 or 20 minute deadline, bounded `job_status` waits) in the gm shell, where `playwriter-project` resolves to `/config/.gmnew/pw/bin/playwriter-project` with no PATH prefix; the gm host was `29e97a7-511` by then (it restarted from `29e97a7-1` during the round).

Defect found and fixed: the status `json!` literal in `write_browser_report` hit the macro recursion limit once the new fields were added (`error: recursion limit reached while expanding $crate::json_internal!`). `textureTarget`, `uiWindowWidth`, `uiFullWidthLabels` and `uiLabelBoxes` are now assigned on the built value (ab26e55).

Witnesses, one per Round 19 Phase A row:

| Row | Witness | Result |
| --- | --- | --- |
| `TextureTarget::for_gpu` | `cargo test --bin bevy-jayzee` | 56 passed, 0 failed, including `gpu_without_bc_falls_back_to_rgba8` |
| Native runtime target | dev build; `scripts/check-hud-overlap.sh` | build rc 0, no warnings; `textureTarget: "bc7"` on this NVIDIA host, `modelsLoaded` 9, `modelsMissing` 0 |
| Wasm runtime target | `WASM_PROFILE=release CARGO_TARGET_DIR=target/wasm-viewer scripts/build-wasm.sh` (cargo is `/config/.local/bin/cargo`) | rc 0, 1 m 24 s, `wasm-bindgen 0.2.129`, bundle 31,062,146 bytes |
| Fallback witness, BC forced off | `node scripts/check-texture-fallback.cjs` (gm job) | `ok: true`. With `texture-compression-bc` hidden: `rows` 64, `loaded` 64, `formatRgba8` 64, `fetchedFromRgba8Cache` 64, `fetchedFromBc7Cache` 0, `bcHiddenOnAdapter` 64, `good` 64, `notGood` [], median load 1,094 ms. Control with BC visible: `format: bc7`, `bcSupported: true`, `adapterHasBc: true`, loaded from `target/gltf-cache`. Browser 2415514 and gate 2415485 exited |
| Full-width labels and the zone banner | `scripts/check-hud-overlap.sh` on neon `main` | exit 0: `uiLabels` 17, `uiLabelOverlaps` 0, `uiUncoveredGlyphs` 0, `uiFullWidthLabels` 0 at `uiWindowWidth` 898. The DOWNTOWN banner box is now `[413, 139, 485, 157]` (72 px, centred) instead of 898 px wide. Screenshot `target/witness/neon-hud.png` |
| `max_content_boxes_with_auto_margins_shrink_and_centre` | `cargo test` | passed |
| `.gitignore` | `git check-ignore -v web/status-main.json` | `.gitignore:8:/web/status-*.json`; `git status` clean |

Regression witnesses on the final tree:

- `gltf-check` on the neon CDN: default (BC7) rc 0 and `rgba8` rc 0; `testdata/normal-map` rc 0.
- `node scripts/check-neon-wasm-models.cjs` (default BC7 cache, gm job): `ok: true`, 64 of 64 clean, median 1,110 ms, GLB transfer 101,358,620 bytes. Browser 2418642 and gate 2418608 exited.
- `node scripts/check-normal-map-visual.cjs` (gm job): `ok: true`, raw and decoded both `loaded`, one adapter each, no tangent warnings, no failed requests. Browser 2425932 exited.
- Release build rc 0, 10 m 44 s. `bash scripts/verify.sh` once: exit 0; bench means 0.2946 / 0.7708 / 1.3427 ms at 500 / 2000 / 5000 bodies; determinism `hashA` = `hashB` = 0x3a5f0b51, `pass: true`; network check `ok: true`. The bench means are not compared back-to-back with an earlier run, so they are not called a regression or an improvement.
- Processes: no Chromium on the repo profile, gate, witness server, viewer or build started this round remains.

Open list after this round: none reachable. World-anchored HUD tags remain dropped in the native viewer (it has no projection for them); that is a stated limitation of the native viewer, not an open defect.

## Round 20 Phase A

Item 1, world-anchored HUD tags in the native viewer. Not a platform limit: the viewer has the 3D camera, so each tag is now placed every frame by projecting its object's world position plus `data-anchor-offset` to the screen.

Ranked remaining work (this stretch does item 1 of the queue and ranked item 1 below; the rest are not done, with the reason):

1. **CSS `transform: translate(...)` is ignored by `ui_html`.** HUD blocks positioned with `left:50%; transform:translateX(-50%)` (`#topc`, the clock and phase block) start at the window centre instead of being centred on it, and other translated boxes drift the same way. Reachable now; done in this stretch.
2. **World-anchored tags whose id is not a viewer object.** Event tags (`E.uid`), coach markers (foe and pad ids) and social pings anchor to entity ids. The native viewer can only place a tag whose id is a `ViewerObject`. Not done: whether neon `main` produces any such tag at the witness moment is what the Phase B witness shows (`uiAnchorsUnresolved`); if it does, the fix is mapping those ids to their mirrored entities, which is the next round.
3. **CSS animations and transitions.** `ui_html` ignores `animation` and `transition`, so time-limited banners (the zone banner's `dzin 4s forwards`, which ends hidden) stay visible in the native viewer. Not done: it needs keyframe parsing and a clock in `ui_html`, which is a larger change than this stretch.
4. **A smaller texture format for GPUs without BC.** Such GPUs get the RGBA8 cache, 4x the BC7 texel memory; ASTC or ETC2 would be the right formats there. Not done: this host's WebGPU adapter (NVIDIA, desktop) has no ASTC or ETC2, so the result cannot be witnessed loading here.
5. **A real non-BC device for the fallback witness.** The Round 19 witness hides `texture-compression-bc` with a script shim. Not done: no device without BC is available on this host.
6. **Bench comparison back-to-back with jayzee.** AGENTS.md says a bench result is only a regression when compared back-to-back on the same host. Not done: it needs a jayzee run outside this repo, and no regression is suspected.

Changes, each a local commit by lanmower with no trailers; no build, test, gate or witness ran in Phase A:

| Change | Commit | Files | Witness it needs (Phase B) |
| --- | --- | --- | --- |
| `ui_html`: a `data-world-anchor` element is no longer dropped. It becomes an absolute box carrying `WorldAnchor { id, offset }` (offset from `data-anchor-offset`), spawned hidden; anchored inline elements skip the inline path; `HtmlUiStatus.anchors` counts them. Test `world_anchored_elements_become_anchored_absolute_boxes` replaces the old drop test | b4ea88d | `src/ui_html.rs` | `cargo test` |
| Viewer: `place_world_anchors` projects each anchor's object position plus offset through the active 3D camera every frame and centres the box on the point; it hides the box when the id has no object (`unresolved`), the point is behind the camera, or the point is outside the viewport. `classify_anchor` holds the decision; test `anchors_are_placed_only_in_front_of_the_camera_and_inside_the_viewport`. Hidden text no longer counts as a label in the overlap check. Status fields: `uiAnchorsInScene`, `uiAnchorsSeen`, `uiAnchorsPlaced`, `uiAnchorsUnresolved`, `uiAnchorsBehind`, `uiAnchorsOffscreen`, `uiAnchorViewport`, `uiAnchorPositions` | b4ea88d | `src/viewer.rs` | `cargo test`; HUD witness below |
| HUD witness requires `uiAnchorsInScene > 0`, `uiAnchorsPlaced == uiAnchorsInScene`, and every placed position inside the viewport | b1e3866 | `scripts/check-hud-overlap.sh` | `scripts/check-hud-overlap.sh` exit 0 with the anchor fields; screenshot `target/witness/neon-hud.png` |
| Ranked item 1: `transform: translate(...)`, `translateX(...)`, `translateY(...)` (px or %) become a `UiTransform` translation (percent of the node's own size, as in CSS), with an `HtmlTranslated` marker. Test `translate_transforms_map_to_node_relative_offsets`. Status `uiTranslatedBoxes`; the HUD witness requires a translated box at the top (`#topc`) centred on the window within 2 px | the last Round 20 code commit | `src/ui_html.rs`, `src/viewer.rs`, `scripts/check-hud-overlap.sh` | `cargo test`; HUD witness `topCentreOffset` at most 2 |

## Round 20 Phase B results

Contention: before each build the gate read swap-out 0 KB/s, over 10 GB free and no cargo. One cargo ran at a time; the unit suite ran alone. The release build ran in the background under `timeout 1500` (PID 2449811), waited on with bounded `tail --pid` calls.

Defects found in Phase B and fixed (5431a91):

- Anchored tags were centred twice. The first run showed the `district-nexus` tag box at x 415 to 448 for a point at x 449. The neon CSS centres each tag itself (`.dtag` `translate(-50%,-50%)`, `.so-tag` and `.ev-tag` `translate(-50%,-100%)`), and the browser puts the anchor element's origin on the projected point. `place_world_anchors` now puts the anchor box's top-left on the point and leaves centring to the tag's own transform, which the Round 20 translate support now applies.
- The witness could not meet "placed equals the count in the scene data" with the orbit camera. It circles 18 units from the origin at height 10, so in the first run 2 of the 5 district tags were placed, 1 was behind the camera and 2 were off screen. That is the correct hide rule for that pose, but the pose changes with time, so the count was not repeatable. `VIEWER_EYE=x,y,z` now fixes the eye, looking at the focus (`fixed_eye`, test `fixed_eye_parses_three_numbers_only`). The HUD witness uses `0,300,1`, overhead of the city, and the status lists each anchor's world position (`uiAnchorWorld`) so the framing can be checked.

Witnesses:

| Row | Witness | Result |
| --- | --- | --- |
| Anchored boxes in `ui_html`, `classify_anchor`, `fixed_eye`, translate parsing | `cargo test --bin bevy-jayzee` | 59 passed, 0 failed |
| World-anchored tags placed by projection | `scripts/check-hud-overlap.sh` on neon `main`, `VIEWER_EYE=0,300,1` | exit 0. `uiAnchorsInScene` 5, `uiAnchorsPlaced` 5, `uiAnchorsUnresolved` [], `uiAnchorsBehind` 0, `uiAnchorsOffscreen` 0, `anchorsInsideViewport` 5 of 5 in 898 x 579. Positions: `district-northgate` (449, 205.5), `district-westworks` (365.0, 289.5), `district-nexus` (449, 289.5), `district-eastline` (533.0, 289.5), `district-southbank` (449, 373.5); world positions (0, 0.5, -36), (-36, 0.5, 0), (0, 0.5, 0), (36, 0.5, 0), (0, 0.5, 36), so the screen cross is symmetric about the window centre as the world cross is about the origin. Screenshot `target/witness/neon-hud.png` shows NORTHGATE, WESTWORKS, NEXUS, EASTLINE and SOUTHBANK centred on those points |
| CSS translate (ranked item 1) | the same HUD run | `topCentreOffset` 0: the translated top block `#topc` box is `[358.5, 16, 539.5, 58]`, centred on 449 |
| No regressions in the HUD | the same HUD run | `uiLabels` 22, `uiLabelOverlaps` 0, `uiUncoveredGlyphs` 0, `uiFullWidthLabels` 0, `modelsLoaded` 9, `modelsMissing` 0, `textureTarget` `bc7`; no unsupported-element warning for anchors any more |
| Regression | release build, then `bash scripts/verify.sh` once | build rc 0, 10 m 34 s; verify exit 0; bench means 0.2844 / 0.7919 / 1.3540 ms at 500 / 2000 / 5000 bodies; determinism `hashA` = `hashB` = 0x3a5f0b51, `pass: true`; network `ok: true`. Not compared back-to-back with an earlier run, so not called a regression or an improvement |

Not run this round, and why: the wasm build and the browser witnesses (neon models, texture fallback, normal-map). No file they depend on changed: `wasm-viewer/`, the decode path and the caches are untouched since Round 19's passing runs.

Processes: the viewers (one per HUD run) were stopped by PID inside `check-hud-overlap.sh`; the release build PID 2449811 exited; no viewer, server or client from this round remains.

Ranked list after this round: item 1 (translate) is done. Item 2 (anchors whose id is not a viewer object) did not occur on neon `main` at the witness moment (`uiAnchorsUnresolved` is empty), so it remains first and unproven rather than open. Items 3 to 6 are unchanged, for the reasons given in Round 20 Phase A.

## Round 21 Phase A

Dispatcher decisions applied: CSS animations and anchors on things other than scene objects are defects to fix in this stretch. ASTC/ETC2 for non-BC GPUs and a real non-BC device are recorded as open by the dispatcher (this host has neither).

Note on the banner spec: the zone banner's keyframes are neon's own `@keyframes dzin{0%{opacity:0;...}15%{opacity:1;...}75%{opacity:1}100%{opacity:0;visibility:hidden}}` with `animation:dzin 4s forwards`. By CSS, opacity is 0 at t=0, rises to 1 by 15% (0.6 s), holds to 75% (3.0 s), and is 0 and hidden from 4.0 s. The witness therefore checks 0 at t=0 rather than 1, and the browser comparison decides what is correct.

Changes, each a local commit by lanmower with no trailers; no build, test, gate or witness ran in Phase A:

| Change | Commit | Files | Witness it needs (Phase B) |
| --- | --- | --- | --- |
| `css_anim`: `@keyframes` parsing (`from`, `to`, percent lists), the `animation` shorthand (lists, duration, delay, `linear`, `ease`, `ease-in`, `ease-out`, `ease-in-out`, `cubic-bezier()`, `steps()`, iteration count, fill mode), and sampling of `opacity` (eased interpolation, underlying 1 where a keyframe omits it) and `visibility` (CSS discrete rule: visible between a visible and a hidden end). Tests: `keyframes_and_shorthand_parse`, `ease_matches_the_css_curve`, `zone_banner_fades_in_holds_and_ends_hidden` | 6484d2d | `src/css_anim.rs`, `src/main.rs` | `cargo test` |
| `ui_html`: boxes keep their `animation` list with keyframes attached and a clock key (the element `id`, else its tree path and names). `HtmlAnimationClock` keeps each key's start time across HUD rebuilds and drops keys that leave the HTML, so a re-entered banner restarts. `animate_html` sets the box's visibility and scales the text and background alpha of its whole subtree each frame (`HtmlBaseAlpha` holds the unanimated alpha). Test `animated_boxes_keep_their_keyframes_under_their_id` | 6484d2d | `src/ui_html.rs` | `cargo test`; fixture witness below |
| Viewer status: `uiClock` and `uiAnimations` (key, names, elapsed, opacity, visible) | 6484d2d | `src/viewer.rs` | fixture witness below |
| Fixture world `testdata/hud-fixture` (outside the game data): place `fixture` whose sim spawns five geometry-less entities (`fixture-district`, `fixture-event`, `fixture-foe`, `fixture-pad`, `fixture-ping`); `scripts/ui.js` renders one tag of each kind (district `.dtag`, event `.ev-tag`, coach foe `.cofoe`, coach beacon `.cobeac` with an x offset, social ping `.so-tag`) and the zone banner with neon's `dzin` keyframes; `browser.html` renders the same `ui.js` in a page | 36378d1 | `testdata/hud-fixture/*` | the two witnesses below |
| `scripts/check-hud-fixture.cjs`: runs the native viewer on the fixture with `VIEWER_EYE=0,40,1`, polls `web/status-fixture.json` every 15 to 60 ms for the banner samples until 6 s, takes `hud-fixture-1s.png` and `hud-fixture-6s.png`, stops the viewer by PID. ok when 5 anchors are in the scene, 5 are placed, all 5 are inside the viewport with the expected ids, the first sample is before 0.2 s, every sample from 0.7 s to 2.9 s is opacity 1 and visible, every sample from 4.0 s is opacity 0 and hidden, and there are 0 label overlaps | 36378d1 | `scripts/check-hud-fixture.cjs` | `node scripts/check-hud-fixture.cjs` exit 0 |
| `scripts/check-banner-browser.cjs` (gm playwriter): opens `browser.html`, pauses the banner's real CSS animation and seeks it to each native sample's elapsed time and to 0, 2, 4 and 6 s; ok when every native sample matches the browser's computed opacity within 0.02 and its visibility exactly, and the browser has opacity 0 and hidden at 4 s | 36378d1 | `scripts/check-banner-browser.cjs` | `node scripts/check-banner-browser.cjs` exit 0 (gm job) |
| Ranked item 6: `scripts/bench-vs-jayzee.sh` runs jayzee's `scripts/bench.mjs` and the release `bench` back-to-back at 500, 2000 and 5000 bodies, 300 ticks, and writes `target/witness/bench-vs-jayzee.json` | d7bbd25 | `scripts/bench-vs-jayzee.sh` | release build, then the script |

Still undone after this stretch, with reasons:

- ASTC or ETC2 textures for non-BC GPUs, and a real non-BC device: open by dispatcher decision; this host's GPU has neither format and there is no such device.
- CSS animation of properties other than `opacity` and `visibility` (for example `dzin`'s `transform: translateY(-4px)` on the first frames, or colour animations): the native path samples only those two. Not done because no witness on neon `main` or the fixture depends on them, and `transform` animation would need per-frame `UiTransform` writes that conflict with the static translate support; next candidate.
- The `animation-*` longhands (`animation-name`, `animation-duration` and so on): not parsed; neon's HUD uses the shorthand everywhere it animates.
- `transition`: not run. A transition needs the previous value of a property across HUD rebuilds, which the rebuild-from-HTML model does not keep.

## Round 21 Phase B results

Contention: before each build the gate read swap-out 0 KB/s, over 10 GB free and no cargo. One cargo ran at a time; the unit suite ran alone. The release build ran in the background under `timeout 1500` (PID 2475023), waited on with bounded `tail --pid` calls. The browser witness ran as a gm job (600 s deadline) in the gm shell.

Defect found and fixed (41f2f4f): an animated banner was drawn on an opaque black box. In Bevy 0.20 `Node` requires `BackgroundColor`, whose default is transparent black, so every text entity has one; `animate_html` scaled that background's alpha by the text's base alpha and made it opaque. The 1 s screenshot showed NEXUS on a black box (pixels `(0,0,0)` around the glyphs). Text and background now keep separate base alphas (`HtmlTextAlpha`, `HtmlBackgroundAlpha`), and only colours the HTML set are scaled. The fixture witness now also probes the screenshot: the banner box's corner pixels must match the scene just outside it within 12 per channel. After the fix the largest difference is 1.

Fixture change found in Phase B: the BEACON tag (x offset +2) overlapped the viewer's own debug overlay box by 8 px; its offset is now -2. That is fixture layout, not engine code.

Witnesses:

| Row | Witness | Result |
| --- | --- | --- |
| `css_anim`, animated boxes, anchors, fixed eye | `cargo test --bin bevy-jayzee` | 63 passed, 0 failed, including `keyframes_and_shorthand_parse`, `ease_matches_the_css_curve`, `zone_banner_fades_in_holds_and_ends_hidden`, `animated_boxes_keep_their_keyframes_under_their_id` |
| Item 2, anchors on non-scene objects | `node scripts/check-hud-fixture.cjs` | exit 0. `uiAnchorsInScene` 5, placed 5, unresolved [], behind 0, offscreen 0, inside the 898 x 579 viewport 5 of 5, ids `fixture-district`, `fixture-event`, `fixture-foe`, `fixture-pad`, `fixture-ping`. Screen positions: ping (449, 108), pad/beacon (410, 483), foe (268, 289), event (638, 288), district (449, 289). Label overlaps 0. Viewer PID 2474478 stopped |
| Item 1, native banner clock | the same run | 113 samples. First sample at 0.050 s: opacity 0.071. From 0.7 s to 2.9 s: 31 samples, all opacity 1 and visible. From 4.0 s: 34 samples, all opacity 0 and hidden; last sample before 4 s at 3.987 s, opacity 0.0001. Screenshots `target/witness/hud-fixture-1s.png` (NEXUS shown, no backdrop) and `hud-fixture-6s.png` (NEXUS gone) |
| Item 1, browser agreement | `node scripts/check-banner-browser.cjs` (gm job) | exit 0. 114 native samples compared with Chromium's own `dzin` animation seeked to the same elapsed time: 0 disagreements, largest opacity difference 0.0000018, visibility equal on every sample. Browser at fixed times: 0 s opacity 0 visible, 2 s opacity 1, 4 s opacity 0 hidden, 6 s opacity 0 hidden. Examples: 0.298 s native 0.7987 / browser 0.7987; 3.198 s 0.7099 / 0.7099; 4.098 s 0 hidden / 0 hidden. Browser 2474682 and gate 2474667 exited |
| Neon regression | `scripts/check-hud-overlap.sh` on neon `main` | exit 0: 22 labels, 0 overlaps, 0 full-width, anchors 5 of 5 placed, `topCentreOffset` 0, `textureTarget` `bc7`, `modelsMissing` 0. Neon's own animations now run natively: `dzDOWNTOWN` (`dzin`) and three `fd` fades sampled at 2.0 s, opacity 1 |
| Regression | release build, then `bash scripts/verify.sh` once | build rc 0, 11 m 02 s; verify exit 0, determinism `hashA` = `hashB` = 0x3a5f0b51 `pass: true`, network `ok: true`. Bench means 1.0718 / 1.3212 / 2.5245 ms were measured in the tail of the release build's load and are not a timing result |

Not done, with reasons:

- Ranked item 6, bench against jayzee back-to-back: the script is committed (d7bbd25) and runs, but the host was not quiet. After the release build the 1-minute load stayed between 4.8 and 10.3 for more than 25 minutes (three bounded waits of up to 570 s each for load under 1.5 and no compiler process), driven by another session on this host (a `cc1plus` compile, then `gm-core` and its claude process), none of it started by this round. A run made at load 9.85 gave jayzee 0.436 / 2.805 / 9.389 ms and bevy 0.494 / 2.341 / 4.171 ms at 500 / 2000 / 5000 bodies; bevy's solver is parallel and loses more to contention than jayzee's single thread, so those numbers are recorded only as a contended run, not as a comparison. Run `scripts/bench-vs-jayzee.sh` on a quiet host.
- ASTC or ETC2 for non-BC GPUs, and a real non-BC device: open by dispatcher decision; this host has neither.
- Animation of properties other than `opacity` and `visibility`, the `animation-*` longhands, and `transition`: reasons as listed in Round 21 Phase A.

Processes: the fixture viewers (PIDs 2473073, 2473204, 2474478), the neon viewer, the browser and gate of the browser witness, and the release build all exited; nothing started by this round remains.

## Round 22 Phase A

Dispatcher decisions applied: the `animation-*` longhands and `transition` are defects to fix (an authored animation that the native viewer ignores is a defect); the jayzee bench runs as interleaved pairs with the load recorded per run, and the verdict is the ratio.

Design for carrying a transition across HUD rebuilds: the native HUD is rebuilt from HTML on every change, so the value cannot live on the entity. Every dynamic element (one with animations or transitions) now has an `ElementClock` in `HtmlAnimationClock`, keyed by its `id` (or tree path and names). It holds the animation start, a paused-since time, and two tracks, `Track<f32>` for opacity and `Track<bool>` for visibility, each with from, to, start, duration, delay and timing. On a rebuild, a changed target starts a new track from the current tracked value at that instant, so a change in the middle of a transition continues from where it was. A dynamic element's own `opacity` is no longer baked into its colours; it is the track's target and is applied each frame to the subtree. The browser witness decides whether this matches Chromium.

Changes, each a local commit by lanmower with no trailers; no build, test, gate or witness ran in Phase A:

| Change | Commit | Files | Witness it needs (Phase B) |
| --- | --- | --- | --- |
| `css_anim`: `animation-name`, `animation-duration`, `animation-delay`, `animation-timing-function`, `animation-iteration-count`, `animation-fill-mode`, `animation-play-state` (over the shorthand list, or rebuilt from `animation-name`, cycling shorter lists as CSS does; `paused` also in the shorthand); `transition` shorthand and `transition-property`, `-duration`, `-delay`, `-timing-function` longhands with `all`; `Track` values (start value held through the delay; visibility is visible while either end is visible); `sample_over` uses the element's own underlying value instead of opacity 1. Tests `longhands_build_and_override_animations`, `transitions_parse_from_shorthand_and_longhands`, `tracks_hold_during_delay_then_ease_to_the_target`, `animations_sample_over_the_underlying_value` | dae6678 | `src/css_anim.rs` | `cargo test` |
| `ui_html`: `ElementClock` per dynamic element (above); paused elements freeze their elapsed time and resume from it; static `visibility:hidden` (and `collapse`) now hides a box; a dynamic element's own opacity is applied at runtime instead of being baked; status samples add `paused` and `transitionElapsed`. Tests `longhands_transitions_and_visibility_reach_the_box_style`, `element_clock_starts_a_transition_from_the_current_value_across_rebuilds` | dae6678 | `src/ui_html.rs`, `src/viewer.rs` | `cargo test`; the two witnesses below |
| Fixture: the sim sets the player's `state.step = 1` from tick 20 (about 2 s), which re-renders the HUD. `ui.js` adds `lhAnim` (every longhand: `pulse` 2 s, delay 0.5 s, linear, 2 iterations, fill both), `lhPaused` (`animation-play-state:paused`, `pulsein` from 0.3), `trFade` (`transition:opacity 1.5s ease-in-out`, opacity 1 to 0.25 at step 1) and `trHide` (longhands `transition-property:opacity,visibility`, 1 s linear, to opacity 0 and hidden at step 1). `browser.html` gets `__renderStep(step)`, which morphs by `id` (copies `style` and `class` onto the existing elements, as neon's DOM host does), so Chromium runs real transitions | 075b2fc | `testdata/hud-fixture/*` | the two witnesses below |
| `scripts/check-hud-fixture.cjs` samples all five elements every 15 to 40 ms until the banner's elapsed reaches 6.5 s. ok adds: `lhAnim` is 1 before its delay and 0.2 after 4.6 s; `lhPaused` is paused and 0.3 on every sample; `trFade` is 0.25 from 1.6 s after its transition starts; `trHide` is visible during its transition and 0 and hidden from 1.1 s; plus the Round 21 checks | 075b2fc | `scripts/check-hud-fixture.cjs` | `node scripts/check-hud-fixture.cjs` exit 0 |
| `scripts/check-css-browser.cjs` replaces `check-banner-browser.cjs`: Chromium's own animations are seeked to each native elapsed (banner and `lhAnim`); `lhPaused` is read as it is (Chromium keeps a paused animation at time 0 too); for the transitions it records the step-0 values, calls `__renderStep(1)`, pauses Chromium's `CSSTransition` objects and seeks them to each native `transitionElapsed`. ok when every native sample matches within 0.02 opacity and exact visibility, every element was sampled, and Chromium created transitions for both elements | 075b2fc | `scripts/check-css-browser.cjs` | `node scripts/check-css-browser.cjs` exit 0 (gm job) |
| `scripts/bench-pairs.cjs`: for 500, 2000 and 5000 bodies, `PAIRS` (8) pairs alternating which engine runs first, the 1-minute load before and after each run, the ratio jayzee mean / bevy mean per pair, and the verdict by the dispatcher's rule: conclusive only when the ratio's spread (max - min) is no wider than its median's distance from parity (1.0); otherwise inconclusive with that spread. The AGENTS.md baseline ratio is reported alongside | 08441f8 | `scripts/bench-pairs.cjs` | release build, then `node scripts/bench-pairs.cjs` |

Ranked remaining work, updated (reasons for each item left undone):

1. Animation and transition of properties other than `opacity` and `visibility` (`transform` in `dzin`'s first frames, colours). The native path samples only those two; `transform` would need per-frame `UiTransform` writes merged with the static translate. Next candidate.
2. `animation-direction` (`reverse`, `alternate`): not in the dispatcher's longhand list and not used by neon's HUD; parsed as a no-op.
3. Play state per animation: an element is paused only when all its animations are paused, and the whole element's clock freezes; a mixed list on one element is approximated.
4. Reversing a running transition: CSS shortens the reverse duration by the reversing-shortening factor; the native track restarts with the full duration from the current value.
5. ASTC or ETC2 for non-BC GPUs, and a real non-BC device: open by dispatcher decision; this host has neither.

## Round 22 Phase B results

Contention: before each build the gate read swap-out 0 KB/s, at least 6.6 GB free and no cargo; one cargo at a time; the unit suite ran alone. The release build ran in the background under `timeout 1500` (PID 2500935) and was waited on with bounded `tail --pid` calls; it took 15 m 34 s under the shared host's load. The browser witness ran as a gm job.

Defect found and fixed: three dead-code warnings (`Track::running`, `css_anim::sample` and `UiSpec::animation_keys` were only used by tests). `running` is gone; the other two are `#[cfg(test)]`. The dev build is warning-free again.

Witnesses:

| Row | Witness | Result |
| --- | --- | --- |
| `css_anim` longhands, transitions, tracks; `ui_html` box style and element clock | `cargo test --bin bevy-jayzee` | 69 passed, 0 failed |
| Native fixture | `node scripts/check-hud-fixture.cjs` | exit 0. 152 samples per element. `lhAnim` 1 before its 0.5 s delay and 0.2 after 4.6 s; `lhPaused` paused and 0.3 on all 152; `trFade` 0.25 once settled (105 samples carry a transition clock); `trHide` visible through its transition and 0 hidden once settled. Banner and anchors as in Round 21: 5 of 5 placed inside the viewport, 0 overlaps, backdrop delta 1 |
| Chromium agreement | `node scripts/check-css-browser.cjs` (gm job) | exit 0. 760 samples, 0 disagreements. Largest opacity difference per element: banner 0.0000014, `lhAnim` 0.00000056 (seeked animation), `lhPaused` 0.000000012 (Chromium `playState: paused`, `currentTime: 0`, opacity 0.3), `trFade` 0.00000068 and `trHide` 0.00000049 (Chromium `CSSTransition`s for `opacity` and for `opacity` plus `visibility`, seeked to the native transition clock). Examples: `lhAnim` 0.996 s 0.8014 / 0.8014, 3.693 s 0.5227 / 0.5227; `trFade` 0.899 s 0.4996 / 0.4996; `trHide` 0.501 s 0.4992 / 0.4992, 1.182 s 0 hidden / 0 hidden. Browser 2500591 and gate 2500576 exited |
| Neon regression | `scripts/check-hud-overlap.sh` | exit 0: 22 labels, 0 overlaps, anchors 5 of 5 inside the viewport, `topCentreOffset` 0; neon's own transition elements (`#tr`, the `#topc` spans) are now dynamic elements in the native clock |
| Regression | release build, then `bash scripts/verify.sh` once | build rc 0; verify exit 0; determinism `hashA` = `hashB` = 0x3a5f0b51 `pass: true`; network `ok: true` |
| Interleaved bench | `node scripts/bench-pairs.cjs`, 8 pairs per size, alternating first runner, 300 ticks | load 3.75 to 4.26 on every run (each pair's load is in `target/witness/bench-pairs.json`; `/proc/loadavg` refreshes every 5 s, so neighbouring runs often show the same reading). Ratio jayzee mean / bevy mean: 500 bodies median 1.057, range 0.993 to 1.133, spread 0.141 against 0.057 from parity, **inconclusive**; 2000 bodies median 2.558, range 2.402 to 2.689, spread 0.287 against 1.558, **bevy faster**; 5000 bodies median 4.084, range 3.164 to 4.817, spread 1.652 against 3.084, **bevy faster**. Against the AGENTS.md baseline ratios (1.075, 2.673, 4.766) every median lies within its own spread of the baseline, so no regression is detectable at this load. Determinism hashes match the solo bench (0x17ff9626, 0x3a5f0b51, 0x38909cb0) |

Ranked list after this round (unchanged from Round 22 Phase A, reasons there): other animated properties (`transform`, colours); `animation-direction`; per-animation play state; the reversing-shortening factor for reversed transitions; ASTC/ETC2 and a real non-BC device (open by dispatcher decision). The 500-body bench ratio stays inconclusive at this load; `scripts/bench-pairs.cjs` remains for later runs.

Processes: the fixture and neon viewers were stopped by PID inside their scripts; the browser, gate and release build exited; nothing started by this round remains.

## Round 23 Phase A

Dispatcher decisions applied: transform and colour animation, `animation-direction`, per-animation play state, and the reversing-shortening rule for transitions are CSS semantics gaps to close, each witnessed against Chromium.

Note on "compose rather than override": in CSS an animated `transform` replaces the element's static `transform` while the animation applies, and the static value is the animation's underlying value wherever a keyframe omits `transform`. The native path does the same: the static transform list (including the translate used for centring) is the underlying value, keyframes that omit `transform` interpolate to and from it (`anCompose` tests this), and when no animation affects `transform` the static `UiTransform` is left as spawned. Chromium is the judge of the result.

Changes, each a local commit by lanmower with no trailers; no build, test, gate or witness ran in Phase A:

| Change | Commit | Files | Witness it needs (Phase B) |
| --- | --- | --- | --- |
| `css_anim`: `Direction` (`normal`, `reverse`, `alternate`, `alternate-reverse`) in the shorthand and `animation-direction`; directed progress per iteration, including the final iteration's direction for a forwards fill. Typed keyframe segments: `opacity`, `visibility`, `color` and `background-color` (premultiplied sRGB interpolation), `transform` lists of `translate`, `translateX`, `translateY`, `scale`, `scaleX`, `scaleY`, `rotate` (deg, rad, grad, turn), interpolated per function when the lists match, with `none` as the identity of the other list and a translate-rotate-scale fallback otherwise; lengths keep px and % separately so percent translates resolve against the node size at runtime. `sample_over` takes one elapsed time per animation. Transition tracks carry the reversing-adjusted start value and the reversing shortening factor; `retarget` applies the CSS Transitions rule (when the new target equals the running transition's reversing-adjusted start value, the factor is `abs(output progress x old factor + (1 - old factor))` clamped to 0..1, and the duration, and a negative delay, are scaled by it). Tests `directions_follow_css_directed_progress`, `transforms_parse_and_interpolate`, `colours_interpolate_in_premultiplied_srgb` | b717c90 | `src/css_anim.rs` | `cargo test` |
| `ui_html`: one clock per animation (`AnimationClock`), so `animation-play-state` pauses each animation on its own; `animate_html` writes the text colour of the element's subtree (stopping at descendants that set their own colour, marked `HtmlOwnColor`), its background, and its `UiTransform` (translation in px after resolving %, scale, rotation) whenever an animation specifies those properties; static `scale()` and `rotate()` are now applied too. Status samples add `animationElapsed`, per-animation `paused`, `transitionsStarted`, `reversedAt`, `color`, `background`, and the resolved `transform`. Tests `reversed_transitions_are_shortened_as_css_specifies` (2 s linear fade reversed at 0.8 s: factor 0.4, duration 0.8 s, start value 0.68; reversed again at 11.0 s: factor 0.7) and `each_animation_keeps_its_own_play_state` | b717c90 | `src/ui_html.rs` | `cargo test` |
| Fixture: `anTransform` (`slide`: translateX, scale and rotate keyframes, 4 s linear forwards), `anCompose` (static `translate(-20px,0px)` with `nudge` animating only the 50% keyframe), `anColor` (`tint`: `color` and `background-color`), `dirNormal`, `dirReverse`, `dirAlternate`, `dirAltReverse` (`sweep` 1 s `ease-in`, 3 iterations, fill both), `ppMixed` (`blink` and `hue`, `animation-play-state: running, paused`), `trRev` (`transition: opacity 2s linear`, 1 to 0.2 at step 1 and back to 1 at step 2). The sim sets step 1 at tick 20 and step 2 at tick 28 (about 0.8 s later, mid-transition) | 561e043 | `testdata/hud-fixture/*` | the witnesses below |
| `scripts/check-hud-fixture.cjs`: samples all fourteen elements; adds checks that `anTransform` ends at translateX 60 px and a quarter turn, `ppMixed` stays paused on its second animation with one constant colour while its opacity runs, `trRev` records a reversal and ends at opacity 1, and every direction element is sampled | 561e043 | `scripts/check-hud-fixture.cjs` | `node scripts/check-hud-fixture.cjs` exit 0 |
| `scripts/check-css-browser.cjs`, generic: every animation of an element is paused and seeked to that animation's own native elapsed (so direction and per-animation play state are judged by Chromium); opacity, visibility, `color`, `background-color` and the transform matrix are compared (opacity 0.02, colour channel 1.5 of 255, matrix 0.002, translation 0.05 px). Transitions: step-0 rows as-is, then `__renderStep(1)` with Chromium's `CSSTransition`s seeked to the native transition clock; for `trRev` Chromium's first transition is seeked to the native reversal point before `__renderStep(2)`, so Chromium computes its own shortened reversal, whose duration is recorded | 561e043 | `scripts/check-css-browser.cjs` | `node scripts/check-css-browser.cjs` exit 0 (gm job) |

Remaining after this stretch, with reasons: transitions of `color`, `background-color` and `transform` (only `opacity` and `visibility` transition natively; no dispatcher item covered them); `transform` functions beyond translate, scale and rotate (`skew`, `matrix`, 3D), which `parse_transform` rejects so the keyframe falls back to the underlying value; non-uniform scale combined with rotation, where Bevy's `UiTransform` applies rotation and scale in a fixed order that matches CSS only for uniform scale; colours interpolated in sRGB for every syntax, while CSS interpolates `oklch()` and other modern syntaxes in Oklab. ASTC/ETC2 and a real non-BC device stay open by dispatcher decision.

## Round 23 Phase B results

Contention: the gate read swap-out 0 KB/s, at least 7.2 GB free and no cargo before each build; one cargo at a time; the unit suite ran alone. The release build ran in the background under `timeout 1500` (PID 2504744), waited on with bounded `tail --pid` calls (13 m 44 s). The browser witness ran as a gm job.

Defect found and fixed (9a74235): the first Chromium run failed before opening a page with `spawnSync playwriter-project E2BIG`. The comparison plan (14 elements x 147 samples) was inlined into the playwriter command and exceeded the OS argument limit. The plan is now written to `target/witness/css-browser-plan.json`, and the snippet reads it from there.

Witnesses:

| Row | Witness | Result |
| --- | --- | --- |
| Direction, transform, colour, reversal, per-animation play state (unit level) | `cargo test --bin bevy-jayzee` | 74 passed, 0 failed |
| Native fixture | `node scripts/check-hud-fixture.cjs` | exit 0, 147 samples per element. `anTransform` ends at `[60, 0, 1, 1, 1.5708]` (translateX 60 px, scale 1, a quarter turn). `ppMixed` has paused flags `[false, true]` on every sample, one colour (`[1, 0, 0, 1]`, the paused `hue` animation held at its 0% keyframe), and opacity 0.200 to 0.990 from the running `blink`. `trRev` reversed 0.932 s into its first transition (79 samples after the reversal) and ends at 1. All four direction elements sampled. Anchors 5 of 5 inside the viewport, 18 labels, 0 overlaps, banner backdrop delta 1 |
| Chromium agreement | `node scripts/check-css-browser.cjs` (gm job) | exit 0. 2058 samples (14 elements x 147), 0 disagreements. Colour compared on all 2058 samples, `background-color` on 147 (`anColor`), the transform matrix on 441 (`anTransform`, `anCompose`, and the banner, whose `dzin` keyframes animate `transform`). The direction elements and `ppMixed` were judged with every Chromium animation seeked to the matching native animation's own elapsed time. Reversal: Chromium's own reversed `CSSTransition` lasted 932.1 ms, which is the CSS rule's value (output progress 0.466 of a 2 s linear transition, factor 0.466, 2 s x 0.466). Its value at the reversal point was 0.627, and all 79 post-reversal samples match. Transitions Chromium created: `trFade` [opacity], `trHide` [opacity, visibility], `trRev` [opacity]. Browser 2504318 and gate 2504303 exited |
| Neon regression | `scripts/check-hud-overlap.sh` | exit 0: 22 labels, 0 overlaps, anchors 5 of 5, `topCentreOffset` 0, `textureTarget` `bc7`, `modelsMissing` 0 |
| Regression | release build, then `bash scripts/verify.sh` once | build rc 0; verify exit 0; determinism `hashA` = `hashB` = 0x3a5f0b51 `pass: true`; network `ok: true` |

Remaining, with reasons (unchanged from Round 23 Phase A): transitions of colour and transform; `skew`, `matrix` and 3D transform functions; non-uniform scale combined with rotation (Bevy's fixed order); Oklab interpolation for modern colour syntaxes; ASTC/ETC2 and a real non-BC device (open by dispatcher decision). The jayzee bench was out of this stretch.

Processes: the fixture and neon viewers were stopped by PID inside their scripts; the browser and gate of both Chromium runs, and the release build, exited; nothing started by this round remains.

## Round 24 Phase A

Dispatcher decisions applied: (1) tighten the mirror batching bound below 13 by measurement, keeping the lowest bound that passes across the static, mirror and fixture modes, without changing the cache or the mesh sharing; (2) witness the non-BC fallback through a real device request, not a script shim.

The bound is `shapesCreated x MIRROR_SLOTS + MIRROR_PROXY`. Round 8 set it to 4 and 1 (the palette size and the proxy material), so 3 shapes give 13. Round 24 measures the mirror batch keys per mode and lowers the two terms step by step. A bound that is tight for this scene also detects a regression: a new object kind that uses a palette slot the measured scene never used will fail the check, so the lowest passing value is a claim about this scene and the witness is what keeps it honest.

Changes, each a local commit by lanmower with no trailers; no build, test, gate or witness ran in Phase A:

| Change | Commit | Files | Witness it needs (Phase B) |
| --- | --- | --- | --- |
| Native device request: `device_request(disable_bc)` returns `RenderCreation::Automatic` with `WgpuSettings::disabled_features = TEXTURE_COMPRESSION_BC` when `VIEWER_DISABLE_BC=1`, so Bevy's device request leaves BC out and `CompressedImageFormatSupport` (read from `device.features()`) reports no BC. `ViewerScene.bc_supported` comes from the same resource and is written as `bcSupported` in the viewer status | e0a8213 | `src/viewer.rs`, `src/viewer_world.rs` | `cargo test --bin bevy-jayzee disabling_bc_removes_only_that_feature_from_the_device_request`; a viewer run on neon `main` with `VIEWER_DISABLE_BC=1`: `bcSupported` false, `textureTarget` rgba8, `modelsLoaded` 9, `modelsMissing` [] |
| Wasm device request: `?disable-bc=1` on the page URL gives the same `WgpuSettings` through a `RenderPlugin`, so the browser's device is requested without BC; the loader's existing check then takes the RGBA8 path | 691182e | `wasm-viewer/src/lib.rs` | wasm build (`WASM_PROFILE=release scripts/build-wasm.sh`, `CARGO_TARGET_DIR=target/wasm-viewer`), then `node scripts/check-device-bc-fallback.cjs` |
| `scripts/check-viewer-batching.cjs`: `MIRROR_SLOTS` and `MIRROR_PROXY` (defaults 4 and 1, unchanged) set the two terms of the bound; every status file on the command line is checked, and each result carries its bound and its mirror batch keys | 923f354 | `scripts/check-viewer-batching.cjs` | the ladder below, one run per step over the three status files |
| `scripts/check-device-bc-fallback.cjs`: no adapter shim. A control page without the flag must load `bc7` from `target/gltf-cache`; then each of the 64 neon models with `?disable-bc=1` must load, report `data-texture-format` rgba8 and `data-bc-supported` false, fetch from `target/gltf-cache-rgba8` and not from `target/gltf-cache`, and log no errors. The page's own `navigator.gpu` adapter is read unmodified and recorded as `adapterHasBc` per row | 6ee0772 | `scripts/check-device-bc-fallback.cjs` | `node scripts/check-device-bc-fallback.cjs` (gm job): `ok` true, `good` 64 of 64, `controlOk` true |

Phase B plan, not run in Phase A:

- Batching ladder. Each step is `MIRROR_SLOTS`, `MIRROR_PROXY`, the bound with 3 shapes, and the check's result on the three modes (static: `testdata/spec-world`, mirror: neon `main`, fixture: `testdata/hud-fixture` place `fixture`). Steps: (4,1) 13 is the current bound; then (4,0) 12, (3,1) 10, (2,1) 7, (1,1) 4, (1,0) 3. Each step records the three modes' `mirrorBatchKeys`, `shapesCreated` and `mirrorEntities`. The lowest step that passes on every mode is kept, and the constants in the checker become its defaults. A step below the measured keys fails by construction, so the ladder stops at the measurement, not at a guess.
- Adapter. The browser's adapter on this host is the NVIDIA GeForce RTX 3060 Laptop GPU, which reports `texture-compression-bc` (the control in the Round 19 witness). Phase B first probes whether a Chromium software adapter (SwiftShader through the flags gm's playwriter passes) lacks BC. If it does, the fallback runs on that real adapter with no request change, and the witness says so. If it has BC, the `disable-bc=1` device request is the mechanism and the witness says that instead.

Ranked remaining work, updated for Round 24:

1. Batching bound: the ladder is measured in Phase B; the result lands here.
2. Non-BC fallback on a real device: the device request is coded (Phase A); the witness and the adapter probe run in Phase B.
3. ASTC or ETC2 for non-BC GPUs: still open by dispatcher decision; this host's adapter has neither format and no real non-BC device is available.
4. The Round 23 list: transitions of colour and transform, `skew`, `matrix` and 3D transform functions, non-uniform scale with rotation, Oklab interpolation, and the jayzee bench on a quiet host.

## Round 24 Phase B results

Contention: the gate read swap-out 0 KB/s, 2.6 GB free and no cargo before the first build. One cargo ran at a time; the unit suite ran alone after the dev build; the wasm build and the release build each ran alone. Each viewer run was bounded by `timeout 45`, and the browser witness ran as a gm job.

Defect found and fixed: the object batching check (`objectsBatched`) failed on spec-world `test` at every bound. Its 8 draws (4 boxes of different sizes, 2 spheres, a cuboid and a scripted proxy) have 8 distinct (mesh, material) keys, and the scene makes 10 shapes from 22 mesh requests, so mesh sharing works and no object can share a batch with another. The check now applies only when the scene has geometry to share: it passes when `shapesCreated >= objectDraws`. `meshesShared` still fails when sharing breaks. This is a decision about the witness, not the mesh code, and it can be reverted.

Batching ladder. The current tree measured the mirror keys on the four status files; each step's bound is the checker's formula with that step's two terms:

| Step (MIRROR_SLOTS, MIRROR_PROXY) | Static `test` bound / keys | Mirror neon `main` bound / keys | Fixture bound / keys | Mirror `main`, BC off bound / keys | Result |
| --- | --- | --- | --- | --- | --- |
| (4, 1), the Round 8 value | 41 / 8 | 9 / 2 | 5 / 1 | 9 / 2 | pass |
| (4, 0) | 40 / 8 | 8 / 2 | 4 / 1 | 8 / 2 | pass |
| (3, 1) | 31 / 8 | 7 / 2 | 4 / 1 | 7 / 2 | pass |
| (2, 1) | 21 / 8 | 5 / 2 | 3 / 1 | 5 / 2 | pass |
| (1, 1) | 11 / 8 | 3 / 2 | 2 / 1 | 3 / 2 | pass |
| (1, 0), kept | 10 / 8 | 2 / 2 | 1 / 1 | 2 / 2 | pass |

Shapes created: static 10, mirror `main` 2, fixture 1, mirror `main` with BC off 2. The Round 8 bound of 13 was `3 x 4 + 1` on the scene of that round; the current tree has 2 shapes on `main`.

The next step down is `shapesCreated - 1`, which is not a formula in the two terms, and it fails on the measured values: `main` has 2 keys against a bound of 1, and the fixture has 1 key against 0. So (1, 0) is the lowest bound the measurement supports. It has no slack on `main` or the fixture, so a new (shape, material) pair fails the check by design. The kept defaults are `MIRROR_SLOTS` 1 and `MIRROR_PROXY` 0. The cache and the mesh-sharing code are unchanged.

Non-BC device. The browser's adapters on this host both report BC, so no real adapter lacks it:

| Adapter (`navigator.gpu` in the gate's Chromium) | Description | BC |
| --- | --- | --- |
| hardware, `requestAdapter()` | NVIDIA GeForce RTX 3060 Laptop GPU (nvidia, ampere) | true |
| software, `requestAdapter({ forceFallbackAdapter: true })` | SwiftShader Device (Subzero) (google, swiftshader) | true |

The gate's Chromium is launched by the playwriter bundle with `--use-vulkan=swiftshader`, `--enable-unsafe-swiftshader` and `--enable-unsafe-webgpu`. Since the software adapter has BC, the fallback is selected by the device request: `?disable-bc=1` on the wasm page and `VIEWER_DISABLE_BC=1` for the native viewer. The witness reads the device's BC through `data-bc-supported`, which the viewer sets from the device features, while the page's own adapter still reports BC. The loader's existing check takes the RGBA8 path.

Witnesses:

| Row | Witness | Result |
| --- | --- | --- |
| Dev build | `/config/.local/bin/cargo build --bin bevy-jayzee` | rc 0, 41.67 s, 0 warnings |
| Unit suite | `/config/.local/bin/cargo test --bin bevy-jayzee` | 75 passed, 0 failed, including `disabling_bc_removes_only_that_feature_from_the_device_request` |
| Batching, kept bound | `node scripts/check-viewer-batching.cjs` on static, mirror and fixture status | ok true, bounds 10 / 2 / 1, mirror keys 8 / 2 / 1, `objectsBatched` true on all three |
| Wasm build | `PATH=/config/.local/bin:$PATH WASM_PROFILE=debug scripts/build-wasm.sh` | rc 0, 12 m 20 s, 0 warnings, `bevy-jayzee-wasm_bg.wasm` sha256 `e124bffb...` |
| Device fallback, browser | `node scripts/check-device-bc-fallback.cjs` (gm job) | ok true. Control (no flag): `bc7`, `bcSupported` true, loaded from `target/gltf-cache`. With `?disable-bc=1`: 64 of 64 loaded, `formatRgba8` 64, `bcSupported` false 64, fetched from `target/gltf-cache-rgba8` 64, from `target/gltf-cache` 0, good 64 of 64, median load 1,267 ms. Adapters as in the table above. Browser 2515577 and gate 2515562 exited |
| Device fallback, native | `VIEWER_DISABLE_BC=1` viewer on neon `main` | `bcSupported` false, `textureTarget` rgba8, `modelsLoaded` 9, `modelsMissing` [], `objectDraws` 42, `mirrorBatchKeys` 2 |
| Neon HUD regression | `scripts/check-hud-overlap.sh` | ok true: 22 labels, 0 overlaps, 0 full-width, anchors 5 of 5 placed and inside the viewport, `topCentreOffset` 0, `textureTarget` bc7, `modelsLoaded` 9 |
| Fixture HUD regression | `node scripts/check-hud-fixture.cjs` | ok true, 2,338 samples; anchors, banner and cases checks true |
| Release and verify | `/config/.local/bin/cargo build --release`, then `bash scripts/verify.sh` once | build rc 0, 12 m 08 s; verify rc 0; determinism hashA = hashB = 0x3a5f0b51, `pass` true; network `ok` true. Bench means 0.3469 / 0.9448 / 1.8743 ms at 500 / 2000 / 5000 bodies; no load recorded, so these are not a timing comparison |

Not done, with reasons:

- A real adapter without BC: none exists on this host (both adapters report BC). The device-request path is witnessed instead, as the dispatcher's branch for an adapter that has BC. A real non-BC device is still open, as it was before.
- ASTC or ETC2 for non-BC GPUs: open by dispatcher decision.
- `scripts/check-texture-fallback.cjs` still hides BC with an adapter shim. It tests the loader's RGBA8 choice only; the device witness above is the one through the real device request.

Ranked remaining work after Round 24:

1. A real non-BC device or adapter, and ASTC or ETC2 for non-BC GPUs: open by dispatcher decision; neither is on this host.
2. The batching bound is at its measured floor on `main` and the fixture; a new kind that shares a shape with another palette slot fails the check. Raise `MIRROR_SLOTS` or `MIRROR_PROXY` only after measuring why.
3. The Round 23 list is unchanged: transitions of colour and transform, `skew`, `matrix` and 3D transform functions, non-uniform scale with rotation, Oklab interpolation, and the jayzee bench on a quiet host.

Processes: each viewer run ended on its `timeout 45` (rc 124) and the fixture and neon viewers stopped by their scripts; the browser and gate of the device witness exited; the dev and release builds, the wasm build and the unit suite exited. A `python3 -m http.server 8977` (PID 1830962) was already running before this round and was left alone.

## Round 25 Phase A

Owner notes applied: the batching bound stays at its floor (1, 0) with no slack, accepted for now. The judgement made in static mode, scoping `objectsBatched` to scenes with shared geometry (`shapesCreated >= objectDraws` passes it), is kept, and the owner can revert it: restoring the old check makes static `test` fail again, because its eight objects have eight distinct geometries and no batch can form.

Dispatcher decision applied: ETC2 and ASTC 4x4 are added as KTX2 cache targets on the same path as BC7 and RGBA8, chosen by the device's compressed formats in the order BC, ASTC, ETC2, RGBA8. Both formats are added because the probe that decides which one the software adapter exposes has not run, and the format is only claimed where it is loaded.

Changes, each a local commit by lanmower with no trailers; no build, test, gate or witness ran in Phase A:

| Change | Commit | Files | Witness it needs (Phase B) |
| --- | --- | --- | --- |
| Transcoder bridge takes a format code instead of a BC7 flag (0 RGBA32, 1 BC7, 2 ETC2_RGBA, 3 ASTC_4x4_RGBA); block formats output 16 bytes per 4x4 block. `TextureTarget` gains `Etc2` (vkFormat 151, DFD model 161, cache `target/gltf-cache-etc2`) and `Astc4x4` (vkFormat 157, DFD model 162, cache `target/gltf-cache-astc`); `for_gpu` takes `CompressedImageFormats` and prefers BC, then ASTC, then ETC2, then RGBA8; `from_name` parses `gltf-check`'s target argument, which now rejects unknown names with exit 2 | bef6ffb | `native/basislz_ktx2.cpp`, `src/basisu.rs`, `src/main.rs`, `src/viewer_world.rs` | `cargo test --bin bevy-jayzee` (tests `gpu_picks_the_first_compressed_format_it_supports_in_bc_astc_etc2_order`, `every_target_round_trips_through_its_name`, `etc2_and_astc_ktx2_declare_their_vulkan_format_and_block_size`); `gltf-check <cdn> "" etc2` and `astc4x4` write the two caches |
| Wasm viewer: `?adapter=software` sets `WgpuSettings::force_fallback_adapter`; `texture_target` picks BC7, ASTC, ETC2 or RGBA8 from the device's `CompressedImageFormatSupport`; the requested BC7 model path maps to the chosen cache; `data-compressed-formats` lists the device's formats | 69d89ac | `wasm-viewer/src/lib.rs` | wasm build, then `node scripts/check-device-format-fallback.cjs` |
| `scripts/lib/texture-cache.cjs`: GLB and KTX2 helpers (`glbParts`, `ktx2Images`, `ktx2Info`, `sourceIsZstd`, `measure`) moved out of `scripts/texture-footprint.cjs`, which requires them; behaviour unchanged | 715daa5 | `scripts/lib/texture-cache.cjs`, `scripts/texture-footprint.cjs` | `node scripts/texture-footprint.cjs` gives the same totals as before on the BC7 and RGBA8 caches |
| `scripts/check-device-format-fallback.cjs`: control on the hardware adapter must load `bc7`. It then reads both adapters' full feature lists (`forceFallbackAdapter` for the software one). Expected format = the first of ASTC 4x4 then ETC2 that the software adapter exposes, else RGBA8 and a recorded reason. If one is exposed, all 64 neon models load with `?adapter=software&disable-bc=1`; each must report that format, `bcSupported` false, fetch from its cache and not from another. The footprint of that cache against RGBA8 and BC7 is measured from the files | 715016c | `scripts/check-device-format-fallback.cjs` | gm job: `node scripts/check-device-format-fallback.cjs` |

Phase B plan, not run in Phase A:

1. Probe. The witness's own control records the software adapter's features. If it exposes neither `texture-compression-astc` nor `texture-compression-etc2`, the witness stops with `reason` set and the feature list in `adapters.software.features`, and no format is claimed. The cause for that case would be the adapter's own feature set, recorded as observed.
2. If it exposes one or both, the caches are generated with `gltf-check` for `etc2` and `astc4x4` (both are generated, so the witness can name the format the device picks), the wasm build is rebuilt, and the witness runs. Its numbers: loaded count, format count, fetches from the chosen cache and from none other, footprint against RGBA8 and BC7.
3. Quality of the ETC2 and ASTC transcodes is not measured by this round: `texture-quality` compares BC7 and RGBA8 only. The footprint is measured; the image quality of the new formats is open.

## Round 25 Phase B results

Probe: the software adapter returned by `navigator.gpu.requestAdapter({ forceFallbackAdapter: true })` is `SwiftShader Device (Subzero)` (google, swiftshader). Its feature list includes `texture-compression-astc`, `texture-compression-astc-sliced-3d`, `texture-compression-etc2`, `texture-compression-bc`, `texture-compression-bc-sliced-3d` and `texture-compression-unaligned`. The hardware adapter (NVIDIA GeForce RTX 3060 Laptop GPU) exposes BC only, with no ASTC or ETC2. So the software adapter does expose both formats, and the device-request witness applies.

Correction to the Phase A table: the row that says `?adapter=software` sets `WgpuSettings::force_fallback_adapter` was not effective. wgpu 30.0.2's web backend maps only `powerPreference` into `navigator.gpu.requestAdapter` (`src/backend/webgpu.rs`, `request_adapter`), so Bevy's flag never reached the browser. The first witness run was `ok: false`: the software query loaded all 64 models on the hardware adapter with BC removed, so each fell back to RGBA8 and none reported the ASTC or ETC2 format. The fix is in the page, not in Rust: `web/wasm-boot.js` wraps `navigator.gpu.requestAdapter` when `adapter=software` is in the query, passing `forceFallbackAdapter: true`. The wasm viewer no longer sets `force_fallback_adapter`; `device_request` takes only the BC switch. The viewer publishes the adapter name as `data-adapter`, and the witness requires it to name SwiftShader on every row.

Witness `node scripts/check-device-format-fallback.cjs` (output `target/witness/device-format-fallback.json`), `ok: true`:
- control on the hardware adapter: `bc7`, `bcSupported` true, fetched from the BC cache only.
- expected format from the software adapter's features: `astc4x4`.
- 64 of 64 models loaded with `?adapter=software&disable-bc=1` on `SwiftShader Device (Subzero)`; 64 of 64 report `astc4x4`; 64 of 64 fetch from `target/gltf-cache-astc`; 0 fetch from another cache; `bcSupported` false on all 64; `data-compressed-formats` is `astc etc2` on all 64; no console error or failed response.
- footprint (texture bytes in the KTX2 payloads): RGBA8 242,308,356; BC7 60,578,736; ASTC 4x4 60,578,736. ASTC is 0.2500 of RGBA8 and 1.0 of BC7, as expected for 8 bits per texel in both. GLB bytes: RGBA8 283,092,200; ASTC 101,358,620 (0.358 of RGBA8).
- load time median 4,075 ms on the software adapter. Not comparable with hardware loads.

ETC2: `gltf-check` wrote `target/gltf-cache-etc2` (64 of 64 decoded and loaded by Bevy, 0 failures, 97 MB) and `target/gltf-cache-astc` (64 of 64, 0 failures, 97 MB). The device-request witness exercised ASTC only, because the format order is BC, ASTC, ETC2, RGBA8 and ASTC comes first. No ETC2 device row was made, so the ETC2 path is generated and loadable but not witnessed through an adapter. It is not claimed as loaded through a device.

Regression set, all run on the final code:
- `node scripts/check-device-bc-fallback.cjs`: `ok: true`; 64 of 64 on RGBA8 with BC disabled, all fetched from `target/gltf-cache-rgba8`; control on BC7 passes.
- `node scripts/check-viewer-batching.cjs`: `ok: true`; `web/status-main.json` has 50 draws, 3 keys, 2 shapes, bound 2 (`objectsBatched` and `meshesShared` true).
- `scripts/check-hud-overlap.sh`: exit 0, `ok: true`.
- `node scripts/check-hud-fixture.cjs`: `ok: true`; 5 of 5 anchors placed, none unresolved.
- `cargo test --bin bevy-jayzee` (dev build): 77 passed, 0 failed.
- `/config/.local/bin/cargo build` (dev), `cargo build --release` and `WASM_PROFILE=debug scripts/build-wasm.sh`: all exit 0 with no warnings in their logs.
- `scripts/verify.sh`: exit 0. Bench (mean ms, 300 ticks): 500 bodies 0.403, 2000 bodies 1.247, 5000 bodies 2.304. Determinism at 2000 bodies: `0x3a5f0b51` on both runs, `pass: true`. Network check (client prediction error 0 m against a limit of 0.05 m; all 11 checks true).
- `node scripts/texture-footprint.cjs`: exit 0 and `complete` true, BC7 and RGBA8 totals unchanged by the helper move. Its decision is `rgba8`, because it reads `target/witness/texture-visual.json`, dated 05:12 on this run's day, before this round. That file reports `ok: false` with `psnrMin` 15.59 against a floor of 30. The Round 25 changes did not produce that file and this round did not regenerate it (see the ranked list).

Bench note: the baseline recorded in AGENTS.md was measured back to back with jayzee on the same host. This run was not compared with jayzee. At 500 bodies the figure matches the baseline (0.40). At 2000 and 5000 bodies it is 20% and 23% above the baseline. It is recorded and not called a regression until it is measured back to back on a quiet host.

Processes: the device regression run left four `playwriter-project` processes behind. They were stopped by PID (2526805, 2526798, 2526797, 2526648). No chromium was left running. `playwriter-ws-server` (PID 289316, 20 minutes old) and a `node` process from `playwriter-project` (PID 2354022, 6 hours old) were not started by this round's jobs and were left alone.

Ranked remaining work after Round 25:

1. ETC2 through a device request: the cache is generated and loads (64 of 64), but the SwiftShader adapter always takes ASTC first, so the device never selects ETC2. Open: needs a selector on the wasm query (for example `disable-astc=1`) and a witness row for ETC2 with the same numeric check. Reason: not in this round's queue, and ASTC took priority in the witness.
2. Image quality of the non-BC transcodes: `texture-quality` compares BC7 with RGBA8 only. ASTC and ETC2 per-texel quality is unmeasured. Reason: the Round 25 queue asked for the footprint against RGBA8, which is measured.
3. Texture visual witness: `target/witness/texture-visual.json` is from before this round and fails its PSNR floor (psnrMin 15.59 against 30), so the footprint decision stays `rgba8`. Open: regenerate through `check-neon-wasm-models.cjs` and `compare-screenshots.cjs` through the gate, then rerun the footprint decision. Reason: not run this round; it is outside the Round 25 queue and needs the gate.
4. A real non-BC device: none on this host. The RTX 3060 and SwiftShader both report BC. SwiftShader stands in for the device request, and its feature list is recorded above. Reason unchanged from Round 24.
5. The batching floor is (1, 0): on `main` and the fixture it is at the floor. The owner accepted it and can revert the `objectsBatched` scoping (see Round 25 Phase A). Raise `MIRROR_SLOTS` or `MIRROR_PROXY` only after measuring why.
6. The Round 23 list is unchanged: transitions of colour and transform, `skew`, `matrix` and 3D transform functions, non-uniform scale with rotation, Oklab interpolation, and the jayzee bench on a quiet host.

## Round 26 Phase A

Correction to Round 25. The Round 25 Phase B section says the footprint decision "still says rgba8" because `texture-visual.json` (PSNR 15.59) fails. That was wrong, and it is retracted. The footprint file as it stood before this round had `decision: "bc7"` with every reason true (`footprintWins`, `loadOk`, `cleanOk`, `visualOk`). Its `visualOk` came from `target/witness/texture-texel.json` (`ok` true, `psnrLevel0Min` 41.70), not from the screenshot file. `texture-visual.json` was never an input to the decision. Both files were written at 05:12, before Round 25, so the decision rested on a BC7-only texel run that this round had not regenerated, and on no screenshot judgement at all.

Why ETC2 and ASTC are not judged. Three causes, each checked in the code:
1. `src/texture_quality.rs` decodes only BC7 (`bcdec_rs::bc7`) and skips every image whose `vkFormat` is not BC7. An ETC2 or ASTC source therefore yields zero rows and `ok` false. No CPU ETC2 or ASTC block decoder is in this tree or in `Cargo.lock` (the only block decoder is `bcdec_rs`), so texel quality for those two formats is not measured by any CPU path.
2. `scripts/compare-screenshots.cjs` compares only `neon-shots-rgba8` with `neon-shots-bc7`. ETC2 and ASTC never had a screenshot set to compare.
3. `scripts/texture-footprint.cjs` ignored the screenshot witness: `visualOk` read the texel file only.

Fix. A format is judged by what the GPU actually renders, not by a CPU decode. Each format's screenshots are compared with an RGBA8 render taken on the same SwiftShader adapter. RGBA8 needs no selector: the viewer loads a `gltf-cache-rgba8` path as-is on any adapter. The footprint decision reads this round's files for each format and records each file's timestamp. The BC7 default now also needs its screenshot judgement (64 pairs against RGBA8, PSNR 30 or higher), which the decision previously ignored. Whether BC7 passes it is decided by Phase B's measurement, not assumed here. BC7 texel quality stays a CPU measurement; ETC2 and ASTC texel quality stays open, because no decoder exists.

Changes, each a local commit by lanmower with no trailers. Phase A ran no build, test, gate or witness; `node --check` was used only to parse the four edited scripts.

| Change | Commit | Files | Phase B witness |
| --- | --- | --- | --- |
| Selectors. Native `VIEWER_DISABLE_ASTC=1` and wasm `?disable-astc=1` put `TEXTURE_COMPRESSION_ASTC` in `WgpuSettings::disabled_features` beside BC; `device_request(disable_bc, disable_astc)`, with the unit test `disabling_each_compressed_format_removes_only_that_feature_from_the_device_request` | dc4f9ed | `src/viewer.rs`, `wasm-viewer/src/lib.rs` | `cargo test` (alone); wasm build; ETC2 witness below |
| `FORMAT=etc2 node scripts/check-device-format-fallback.cjs` writes `target/witness/device-format-fallback-etc2.json` with `?adapter=software&disable-bc=1&disable-astc=1`; the expected format is ETC2 only if SwiftShader exposes `texture-compression-etc2`. The ASTC run keeps its file name | 17032f9 | `scripts/check-device-format-fallback.cjs` | `FORMAT=etc2` run; ASTC run as a regression |
| `NEON_TEXTURE` modes `bc7`, `rgba8`, `rgba8-software`, `astc4x4`, `etc2`. An unknown value now errors; before, any value other than `rgba8` silently ran BC7. `compare-screenshots.cjs` writes `texture-visual-<right>.json` | bc2c941 | `scripts/check-neon-wasm-models.cjs`, `scripts/compare-screenshots.cjs` | five gate runs; five comparisons |
| `texture-footprint.cjs` writes a verdict per format. BC7 is usable only with footprint wins, load within 1.1 times the hardware RGBA8 witness, 64 of 64 clean, a visual file with 64 pairs and ok, and texel ok. ASTC and ETC2 are usable with footprint wins, load within 1.1 times the SwiftShader RGBA8 witness, clean, and visual ok; their texel is not required. `decision` keeps the BC7 default rule and `fallbackFormats` reports the other two. Each input's mtime is recorded | 568e608 | `scripts/texture-footprint.cjs` | `node scripts/texture-footprint.cjs` |
| AGENTS.md: the non-BC device path names both selectors, the wgpu forced-fallback caveat, the witnesses, and the visual judging rule | (uncommitted at this point, next commit) | `AGENTS.md` | none |

Phase B plan, not run in Phase A:
1. `/config/.local/bin/cargo build` (dev) and `cargo test --bin bevy-jayzee` alone; wasm build (`WASM_PROFILE=debug scripts/build-wasm.sh`).
2. `FORMAT=etc2 node scripts/check-device-format-fallback.cjs`, which writes the file the queue names, `device-format-fallback-etc2.json`.
3. Regenerate `target/witness/texture-texel.json` with `target/debug/bevy-jayzee texture-quality <cdn>`. The current file is from 05:12.
4. Regenerate the screenshot sets through the gate, one run each: `NEON_TEXTURE` = `rgba8`, `bc7`, `rgba8-software`, `astc4x4`, `etc2`. Then compare: BC7 against hardware RGBA8 (`SHOTS_LEFT=neon-shots-rgba8 SHOTS_RIGHT=neon-shots-bc7`), and ASTC and ETC2 each against `neon-shots-rgba8-software`.
5. `node scripts/texture-footprint.cjs` gives the decision from those files.
6. Bench, back to back on this host: `scripts/bench-vs-jayzee.sh` with the release binary, after one release build and `verify.sh`. Both the jayzee and the bevy numbers are recorded with the host load before and after.
7. Regressions the device path could affect: `check-device-bc-fallback.cjs`, `check-device-format-fallback.cjs` (ASTC, the default), `check-viewer-batching.cjs`, `check-hud-fixture.cjs`.

Process check for the Round 25 leftover. `ps -o pid,etime,args -p 2354022` returns no row, so PID 2354022 (the 6-hour `node` from `playwriter-project`) has exited. The same check listed `playwriter-ws-server` (PID 289316), age 20:36 at that time; it was not started by a Round 25 job that is still running, and it was not stopped.

Ranked remaining work, updated for Round 26 Phase A (reasons for each item left undone):
1. ETC2 through a device request: the Phase B witness decides it. Not claimed until `device-format-fallback-etc2.json` has `ok: true`.
2. ASTC and ETC2 texel quality: not measured, because no CPU decoder exists in the tree. Reason: adding a decoder is a new dependency or a new decoder, out of scope for this round. The visual judgement stands in for it.
3. BC7 visual judgement: the decision now requires it. Its result is recorded in Phase B; a failing BC7 visual would make the default `rgba8`, as designed.
4. A real non-BC device on this host: none. The NVIDIA adapter reports BC, and SwiftShader reports BC, ASTC and ETC2. SwiftShader is the stand-in for the device request.
5. The batching floor (1, 0) and the owner's note about reverting the `objectsBatched` scoping: unchanged.
6. The Round 23 list is unchanged: transitions of colour and transform, `skew`, `matrix` and 3D transform functions, non-uniform scale with rotation, Oklab interpolation, and the jayzee bench on a quiet host.

## Round 26 Phase B results

Builds: `cargo build --bin bevy-jayzee` (dev) and `cargo build --release --bin bevy-jayzee` both exit 0 with no warnings. `cargo test --bin bevy-jayzee` (run alone): 77 passed, 0 failed, including `disabling_each_compressed_format_removes_only_that_feature_from_the_device_request`. The wasm build (`WASM_PROFILE=debug scripts/build-wasm.sh`) was run twice and has no warnings.

### 1. ETC2 through a device request: claimed, witnessed

`VIEWER_DISABLE_ASTC=1` (native) and `?disable-astc=1` (wasm) put `TEXTURE_COMPRESSION_ASTC` in `disabled_features`. `FORMAT=etc2 node scripts/check-device-format-fallback.cjs` (page query `?adapter=software&disable-bc=1&disable-astc=1`) writes `target/witness/device-format-fallback-etc2.json`:
- `ok: true`; adapter `SwiftShader Device (Subzero)`; `compressedFormats: ["etc2"]` on all rows; `bcSupported` false on all rows.
- 64 of 64 loaded; 64 of 64 report `etc2`; 64 of 64 fetched from `target/gltf-cache-etc2`; 0 fetched from any other cache.
- Footprint: RGBA8 242,308,356 texture bytes against ETC2 60,578,736, a ratio of 0.2500 (1.0 against BC7, as both are 8 bits per texel).

The ASTC witness was re-run on the final wasm build and still passes (`device-format-fallback.json`, `ok: true`, 64 of 64, ratio 0.2500).

### 2. Visual judgement: why the new format was not judged, and the fix

Three causes, each checked in the code:
- `src/texture_quality.rs` decodes only BC7 and skips every other image, so ETC2 and ASTC produced zero texel rows.
- `scripts/compare-screenshots.cjs` compared only RGBA8 with BC7; neither new format had a screenshot set.
- `scripts/texture-footprint.cjs` never read the screenshot witness. Its `visualOk` came from `texture-texel.json` alone.

A fourth cause showed up when the screenshot judge was first run: the wasm viewer orbits its camera at `elapsed × 0.18` and spins its meshes at `elapsed × 0.9`, on wall-clock time, so two captures of the same model differ. RGBA8 against a second RGBA8 render (before the fix) gave min 16.03, median 27.80, 43 of 64 pairs under 30 dB, and no pair identical. That is the noise floor of the judge, not of any format.

Fix: `still=1` on the wasm page holds the orbit and the spin at zero (`wasm-viewer/src/lib.rs`). Every mode of `check-neon-wasm-models.cjs` passes it. With it:
- RGBA8 against a second RGBA8 render: 64 of 64 identical.
- BC7 against RGBA8: min 65.12, median 69.39; 52 identical, and the 12 that differ are the 12 textured models.
- ASTC 4x4 against RGBA8 on SwiftShader: min 51.90, median 69.76; 52 identical.
- ETC2 against RGBA8 on SwiftShader: min 50.44, median 57.25; 52 identical.
All 64 pairs pass the 30 dB floor for all three formats.

The footprint decision also had a bug of mine, fixed in `0b643da`: `visualOk` compared the 64 visual pairs with the 12-model zstd subset, so every format failed it. It now compares against the full model count.

Final decision from `target/witness/texture-footprint.json`: `decision: "bc7"`. BC7: footprint wins (0.25), load within 1.1 times RGBA8 (1280 ms and 1279 ms medians in this round's runs), 64 of 64 clean, visual ok, texel ok. ASTC 4x4 and ETC2 are both usable (`fallbackFormats` true for both). The decision comes from this round's files; each file's mtime is recorded in the footprint output.

Witness runs behind that decision, all through the gate:
- `neon-wasm-witness-{rgba8,bc7,rgba8-software,astc4x4,etc2}.json`: each 64 of 64 clean.
- `texture-visual-{bc7,astc4x4,etc2,rgba8}.json` (the last is the noise floor): the figures above.
- `texture-texel.json` regenerated by `target/debug/bevy-jayzee texture-quality`: ok, 61 images, 12 models, min 41.70, median 53.51. The same numbers as the 05:12 file, so the BC7 texel was current.

Runs that are kept on record but not the decision:
- `*-nostill.json` (witness and visual before the fix): BC7 visual min 15.42, median 31.59, 26 of 64 under 30 dB; ASTC min 12.05; ETC2 min 14.35. These measure the judge's noise, not the formats.
- `neon-wasm-witness-astc4x4-run1.json`: the first ASTC gate run, before `still=1`, had one model (`value.7e1d1aec…glb`) fail with `Unable to find a GPU` in the wasm panic hook, `ready: false`. The rerun of the same mode was 64 of 64 clean, and the `still=1` run was 64 of 64 clean. The cause is not known; it was seen once in three ASTC gate runs.
- A load gate flip: an intermediate run (after the RGBA8 witness had been re-run and before BC7 was re-run) gave `loadOk` false, because two runs with different loads were compared (1264 ms against 2091 ms). Medians from a single run each are noisy at a 1.1 tolerance; the decision above uses the final runs, which are within tolerance, and is not stable proof.

### 3. Bench: back to back, no regression

Same release binary (`target/release/bevy-jayzee`, built from this round's tree) and the same host, measured back to back by `scripts/bench-vs-jayzee.sh` (one jayzee run and one bevy run at each body count, 300 ticks). Host load (1-minute) was 1.71 before and 1.65 after, against about 1.6 during `verify.sh`.

| bodies | jayzee mean ms | bevy mean ms | speedup | recorded baseline (bevy / jayzee) |
| --- | --- | --- | --- | --- |
| 500 | 0.414 | 0.318 | 1.30 | 0.40 / 0.43 |
| 2000 | 2.040 | 0.822 | 2.48 | 1.04 / 2.78 |
| 5000 | 6.645 | 1.405 | 4.73 | 1.88 / 8.96 |

`verify.sh` on the same binary, same day: bevy 0.308 / 0.804 / 1.398 ms at 500 / 2000 / 5000; determinism at 2000 bodies `0x3a5f0b51` on both runs (`pass: true`); network check `ok: true`.

Verdict: bevy is at or below the recorded baseline at every size, and the speedup ratios match the baseline (2.48 against 2.67 at 2000, 4.73 against 4.77 at 5000). The Round 25 figures (1.247 and 2.304 ms at 2000 and 5000) were a contended run: that round recorded no host load, and this round's same-code run is 25 to 35 percent lower. Under the AGENTS rule that delta is not a regression.

### Regressions on the final build

- `scripts/check-device-bc-fallback.cjs`: `ok: true`; 64 of 64 loaded on RGBA8 with BC disabled, 64 fetched from `target/gltf-cache-rgba8`, 0 from BC7; control on BC7 passes.
- `scripts/check-viewer-batching.cjs`: `ok: true`; `web/status-main.json` 50 draws, 2 shapes, bound 2.
- `scripts/check-hud-fixture.cjs`: `ok: true`; 5 of 5 anchors placed, none unresolved.

### Processes

PID 2354022 (the Round 25 leftover, a 6-hour `node` from `playwriter-project`): gone. `ps -o pid,etime,args -p 2354022` returns no row at the start of this round and again before the bench. PID 289316 (`playwriter-ws-server`, 22 minutes old at the last check) is still running. No job of this round started it and it was not stopped. No chromium is running.

### Commits

Phase A: dc4f9ed, 17032f9, bc2c941, 568e608, c7e40c6. Phase B: 914d88f (still capture), 0b643da (footprint pair count), and the commit that holds this section.

### Ranked remaining work after Round 26

1. Load gate on single-run medians: the 1.1 tolerance flipped once when only one side was re-run. Open: take at least three runs per mode and compare medians. Reason left undone: outside this queue; the current decision rests on one final run per mode.
2. ASTC and ETC2 texel quality: not measured. No CPU ETC2 or ASTC block decoder exists in the tree, and writing one is a new decoder, outside this queue. The screenshot judgement stands in for it.
3. The one-off `Unable to find a GPU` panic in the first ASTC gate run: not reproduced in two later runs; cause not found. Open: if it recurs, record the model and the gate's adapter log.
4. A real non-BC device on this host: none. The NVIDIA adapter reports BC; SwiftShader reports BC, ASTC and ETC2 and stands in for the device request.
5. The screenshot cross-adapter caveat: BC7 is compared with a hardware RGBA8 render, and ASTC and ETC2 with a SwiftShader RGBA8 render. Each pair is on one adapter, but the references differ. Open: a SwiftShader BC7 set would remove the difference. Reason left undone: the queue asked for the new formats on the fallback adapter, and the BC7 reference already passes.
6. The batching floor (1, 0): accepted by the owner; revert note recorded in Round 25 Phase A.
7. The Round 23 list is unchanged: colour and transform transitions, `skew`, `matrix` and 3D transform functions, non-uniform scale with rotation, Oklab interpolation.


## Round 27 Phase A: performance metrics wired or removed

Phase A is code, docs and local commits only. Nothing here was built, run or witnessed. The only check was a syntax parse of web/inspector.js, which is not a test. Phase B builds and witnesses.

Commits, all as lanmower with no trailers: 1057517 (native HUD), 921c1d9 (wasm snapshot), a09b7c4 (inspector renders the snapshot only), 4b2cdca (check-inspector.cjs fails on stubs), and the commit that adds this section.

### Metric sources

| Metric | Surface | Source | Status |
|---|---|---|---|
| FPS | strip | FrameTimeDiagnosticsPlugin fps | real |
| MEMQ | strip | WebAssembly.Memory buffer byteLength | real in wasm; native has no wasm memory |
| MEMO | strip | performance.memory.usedJSHeapSize | real in Chromium; unsupported elsewhere, with reason |
| DRAW | strip, bar | timer around RenderSystems::Render in the render world (encode, submit, present) | real |
| SHOW | strip, bar | none | unsupported: present_frames runs inside render_system with no hook in Bevy 0.20.1; its time is counted in DRAW |
| SIM | strip, bar | First to Last schedule timer in the main world | real |
| LAUNCH | strip, bar | Last end to Render start (extract handoff) | real |
| CPU total | bar | First start to Render end | real |
| GPU | bar | RenderDiagnosticsPlugin elapsed_gpu spans, summed | real when TIMESTAMP_QUERY is enabled; otherwise unsupported with the device feature list |
| DRAWS | metric | RenderVisibleEntities<Mesh3d> of the main views | real; batching is not counted |
| SHADOW-MAP DRAWS | metric | RenderVisibleEntities<Mesh3d> of ViewLightEntities shadow views | real |
| TRIANGLES | metric | RenderMeshInstances mesh asset, RenderAssets<RenderMesh> index count / 3 (vertex_count / 3 when non-indexed) | real |
| SHADOW TRIS | metric | same, shadow views | real |
| SHADOW PASSES | metric | ViewLightEntities.lights count | real; 0 in the wasm scene because shadow maps are disabled |
| ENTITIES | metric | EntityCountDiagnosticsPlugin | real |
| MESHES | metric | Assets<Mesh>::len | real |
| SPRITES | metric | Query<Sprite> | real |
| TEXTURES | metric | Assets<Image>::len | new |
| TEXTURE BYTES | metric | sum of Assets<Image> data lengths (CPU side) | new; GPU texture bytes are not readable from Bevy |

### Removed, with reasons

- CASCADE SKIPS: Bevy 0.20.1 exposes no cascade-skip counter, so there is no source.
- TERRAIN: the wasm scene has no terrain plugin, so a count would always be zero.
- Frame bars adapter CPU, Lume GPU renderer, process/queue and on-demand timing: no source. Lume was another project's label.
- The FPS frame bar: a duplicate of the strip FPS.
- The static unavailable list in the snapshot: replaced by per-entry reasons and an unsupported list.

### Design

- src/render_stats.rs is the shared module. The wasm viewer includes it with a path attribute. RenderStatsPlugin takes main-world timers in First and Last, render-world timers around RenderSystems::Render, and the counters in RenderSystems::Cleanup. The state is an Arc of Mutex of FrameStats, shared between the two worlds.
- RenderDiagnosticsPlugin is added explicitly in both apps, because RenderPlugin adds it only under the tracing-tracy feature.
- Bevy's default WgpuSettings priority requests every adapter feature, so TIMESTAMP_QUERY is on whenever the adapter has it. The snapshot records the device feature list in either case.
- The snapshot entry constructor always sets a reason when the value is absent, so the inspector and the checker can rely on that.
- The native HUD prints the same values. RSS comes from /proc/self/statm and assumes a 4096-byte page, which holds on Linux x86_64.

### Not verified (Phase B must settle these)

- Names: bevy::platform::time::Instant, bevy::render::view::RenderVisibleEntities, bevy::pbr::{RenderMeshInstances, ViewLightEntities}, the RenderMesh.vertex_count and RenderMeshBufferInfo::Indexed count fields, and App::get_sub_app_mut. The compile settles each one.
- Whether the render-world visibility lists are populated when RenderSystems::Cleanup runs in both viewers.
- Whether the wasm software adapter reports TIMESTAMP_QUERY.
- Whether RenderDiagnosticsPlugin records GPU spans on this host.


## Round 27 Phase B: performance metrics witnessed

### Builds

- Native dev: /config/.local/bin/cargo build, exit 0, no warnings, on the last three builds.
- Wasm release: PATH=/config/.local/bin:$PATH WASM_PROFILE=release scripts/build-wasm.sh, exit 0. The build script calls cargo by name, so PATH selects the permitted binary.
- Gate before the first build: swap-out 0 KB/s in the one-second samples, 7.3 GB free.

### Defects found by the witness, fixed

1. Counts printed with two decimals (3.00 DRAWS) and the strip repeated its unit (fps fps). Fixed: integers print without decimals; each value sits in one value cell.
2. Bevy publishes the snapshot as a JSON string, and the inspector's 250 ms poll is the only thing that parses it. A reader that did not parse it saw no shown list. Fixed: the check hook and check-inspector.cjs both parse a string snapshot.
3. The GPU reason said "no elapsed_gpu spans" on a device that lacks two features. Bevy's pass timers (bevy_render diagnostic/internal.rs lines 296 and 298) need TIMESTAMP_QUERY_INSIDE_PASSES and TIMESTAMP_QUERY_INSIDE_ENCODERS too. This WebGPU adapter exposes TIMESTAMP_QUERY alone, so GPU passes are unsupported in the wasm viewer. Fixed: the reason lists the missing flags and the device feature list. The native viewer reports GPU passes as supported (see below).
4. The four phases came from different frames. Bevy pipelines rendering, so the render world can run during the next frame's main update, and the shared frame start was overwritten. Fixed: the render world captures each frame's marks at extract and publishes SIM, LAUNCH, DRAW and CPU total together at render end. Before the fix the native HUD read SIM 2.96 + LAUNCH 10.79 + DRAW 13.06 against CPU total 13.01 ms. After the fix it reads SIM 2.58 + LAUNCH 18.54 + DRAW 15.68 = 36.80 = CPU total 36.80 ms.
5. LAUNCH includes any wait for the previous render frame under pipelining. The label now says so.
6. check-inspector.cjs launched its own Chromium, which the owner's rule against browser work outside playwriter excludes. Changed: it attaches to the playwriter browser through CDP_URL, closes only its own page, and exits 2 without CDP_URL. The judge is exported and the script runs main only when executed directly, so the negative case can run without a browser.

### Browser witness (gm playwriter)

The page was served on 8092 by scripts/serve-wasm.sh and opened through playwriter_execute at http://127.0.0.1:8092/web/wasm/index.html?inspector=1&still=1. Witness output from the final build, produced by scripts/check-inspector.cjs with CDP_URL=http://127.0.0.1:10232 (the playwriter browser):

- ok: true, failures: [], inspector: open, appReady: true, exit 0.
- strip: FPS=32.85 fps (FrameTimeDiagnosticsPlugin/fps); MEMQ=59.50 MiB (WebAssembly.Memory buffer byteLength); MEMO=25.53 MiB (performance.memory.usedJSHeapSize); DRAW=1.94 ms (render schedule timer around RenderSystems::Render); SHOW=unsupported (present_frames runs inside RenderSystems::Render with no separate hook in Bevy 0.20.1; its time is counted in DRAW); SIM=0.85 ms (First to Last schedule timer); LAUNCH=0.47 ms (Last end to Render start, including any wait for the previous render frame).
- metrics: DRAWS=3 (RenderVisibleEntities<Mesh3d> of the main views; batching not counted); SHADOW-MAP DRAWS=0; TRIANGLES=734 (RenderMeshInstances with RenderAssets<RenderMesh> index counts / 3); SHADOW TRIS=0; SHADOW PASSES=0 (the wasm scene disables shadow maps); ENTITIES=302; MESHES=8; SPRITES=0; TEXTURES=8; TEXTURE BYTES=3850248 B (Assets<Image> CPU data length).
- bars: frame interval=30.44 ms; CPU total=3.25 ms; SIM=0.85 ms; LAUNCH=0.47 ms; DRAW=1.94 ms; SHOW=unsupported; GPU=unsupported (device lacks TIMESTAMP_QUERY_INSIDE_ENCODERS, TIMESTAMP_QUERY_INSIDE_PASSES; device features list recorded in the snapshot).

The first run of this witness (before fixes 1 to 3) and the playwriter DOM probe are the source of defects 1 to 3.

### Stub guard

The judge in scripts/check-inspector.cjs, run on a snapshot with one null value and no reason, prints:

- stub: strip/MEMO has no value and no reason (source: test)
- strip/MEMO is unsupported without a reason

### Native run

Command: DISPLAY=:1.0 target/debug/bevy-jayzee viewer testdata/spec-world shapes. The HUD text comes from web/status-shapes.json, the "hud" array that write_browser_report now records (untracked output). Final run:

- bevy-jayzee viewer - shapes | place=shapes | objects=4 | proxy meshes=4 | light=1 | camera=(4.4,10.0,-17.5)
- Native Bevy render/window/PBR/UI path | headless world API preserved
- Bevy DiagnosticsStore | fps=47.3 | frame=21.16ms | entities=313 | I toggles browser inspector in wasm viewer
- Render world | draws=5 | triangles=2590 | shadow draws=0 | shadow triangles=0 | shadow passes=0
- Frame phases | SIM=2.58ms | LAUNCH=18.54ms | DRAW=15.68ms | SHOW=unsupported (present runs inside the render system) | CPU total=36.80ms
- GPU passes | 1.528ms over 9 passes | process RSS=585.7MiB

RSS is read from /proc/self/statm with a 4096-byte page size. GPU passes are supported natively: the native adapter exposes all three timestamp features.

### Processes

- Browser: playwriter browser pid 2571207 and its gate pid 2571177 stopped by SIGTERM over their process trees (nine PIDs in all, the browser's own children included). Afterwards no playwriter or Chromium process of this round remains.
- Wasm server job (pid 2572455): stopped. Port 8092 no longer answers.
- Native viewer jobs (pids 2572910, 2573559, 2574003): stopped.
- Not stopped, not started by this round: PID 289316 (playwriter-ws-server) and the playwriter-project processes from before this round (2566897 and its children).

### Open

- SHOW has no source in Bevy 0.20.1; it stays unsupported with that reason.
- GPU passes are unsupported in the wasm viewer on this adapter, and supported natively.
- DRAWS counts visible mesh entities in the render world, not GPU draw calls. Batching is not counted, so the count is an upper bound on submitted draws.
- Each figure is one run. No median over runs was taken.


## Round 28 Phase A: web UI taste pass

Phase A is code, CSS and docs only. Nothing was built, run or witnessed. Phase B runs the taste loop.

### Palette

The flag hues are softened by keeping each one's HSL lightness and taking two thirds of its saturation. Text uses a lighter tint of the same hue, so each text colour clears 6:1 on the panel colour (#07111f). The neutrals are the existing dark base and off-white.

| Token | Hex | Source | Role |
|---|---|---|---|
| --flag-green | #146648 | flag #007A4D, softened | primary accent: ready border, bar fill |
| --flag-gold | #D8A739 | flag #FFB612, softened | selected tab and active state |
| --flag-red | #C1534E | flag #DE3831, softened | boot-error border |
| --flag-blue | #19307C | flag #002395, softened | buttons, toggle |
| --flag-green-text | #26C087 | tint of green | status text, bucket values, sparklines |
| --flag-gold-text | #C09026 | tint of gold | pending status text, sparklines |
| --flag-red-text | #CF7B77 | tint of red | boot-error text, sparklines |
| --flag-blue-text | #758FE3 | tint of blue | panel headings, links |
| --ui-base | #050b12 | neutral (black) | page background |
| --ui-panel | #07111f | neutral | panels |
| --ui-raised | #081522 | neutral | buckets, tabs |
| --ui-card | #091827 | neutral | metric cards |
| --ui-track | #14253a | neutral | bar track |
| --ui-line | #1b3148 | neutral | borders |
| --ui-edge | #52627a | neutral | pill border while not ready |
| --ui-text | #e8f1ff | neutral (white) | primary text |
| --ui-muted | #93a9c0 | neutral | secondary text, unsupported labels |
| --ui-ink | #06111d | neutral | text on gold |

All palette values are defined once, in the :root block of web/wasm-index.html. The page's inline colours were none, so nothing else needed changing there.

### Changes (each committed as lanmower, no trailers)

- c7b85ae: palette custom properties; header border, status pill (gold text while pending, green when ready), toggle (blue), boot-error box (red border, red-tint text).
- 4168366: inspector panels, tabs (gold when selected), buckets, action buttons (blue), panel headings (blue tint), metric cards, bar track and fill (green to gold), links (blue tint), details and pre text. The .unavailable class moves from red to the muted neutral, because red is reserved for errors and an unsupported metric is not an error.
- 3526ea7: sparkline series colours come from the tokens in web/inspector.js.
- 8fbef24: the native loading text (src/viewer.rs) uses the palette text colour #e8f1ff. The native HUD sets no other colour; the 3D ClearColor, ground, model materials and the neon world are not touched.

### Not changed

The 3D scene's colours, the neon world, the banner's authored colours and every model material.

## Round 28 Phase B: taste loop witnesses

Viewer: http://127.0.0.1:8092/web/wasm/index.html?inspector=1&still=1 through gm's playwriter (debug port 10232), viewport 1440x900, 20 s wait before each capture. Files were copied from web/wasm-index.html and web/inspector.js into web/wasm (both git-ignored) before each reload.

Contrast method: in the page, every visible text node under the header and inspector (82 nodes) is measured. Backgrounds are composited up the DOM from the body colour, and WCAG relative luminance gives the ratio. Limit: the boot-error box is hidden, so its ratio is a colour check only.

### Palette after the passes

Hex values are the Phase A table above. No hex value changed in the three passes. The one colour change was removing red from the sparkline cycle in web/inspector.js.

### Contrast at the last pass (every label at or above 4.5:1)

| Text | Background | Ratio |
| --- | --- | --- |
| #758fe3 (blue tint, panel headings and tab title) | #07111f | 6.13 |
| #93a9c0 (muted, metric labels and unsupported) | #091827 | 7.41 |
| #93a9c0 (muted, unselected tabs) | #081522 | 7.61 |
| #26c087 (green, bucket values) | #081522 | 7.87 |
| #26c087 (green, ready status pill) | #050b12 | 8.44 |
| #06111d (ink, selected tab on gold) | #d8a739 | 8.60 |
| #e8f1ff (text, toggle and window buttons) | #19307c | 10.52 |
| #e8f1ff (text, primary) | #091827 | 15.75 |
| #cf7b77 (red tint, boot error colour only) | #080f19 | 6.18 |

Lowest label: 6.13:1. Before the round it was 6.35:1 (#7f9dbb on #091827, metric labels). Count of text nodes under 4.5:1: 0 in every pass.

### Passes

- Before (pre-palette files from 681eea0): docs/ui-taste/before-inspector.png. Cyan and lime sparklines, cyan-to-lime bars, red unsupported labels. Minimum 6.35:1, 0 below 4.5.
- Pass 1 (Phase A palette): docs/ui-taste/pass1-inspector.png. Minimum 6.13:1. scripts/check-inspector.cjs ok:true, no failures (docs/ui-taste/pass1-check.png). scripts/check-hud-overlap.sh exit 1, uiLabelOverlaps 1: the Round 27 render-counter lines made the native overlay wider and taller, and its box crossed the EASTLINE label. Visible problem: the DRAW sparkline was red, and red is reserved for errors. Changes: (a) the native render counters and frame phases moved into their own overlay above the main overlay, commit 9288319; the first attempts (top-left, bottom-left, bottom-right at bottom 96) overlapped the HELIX, UZI, TECH, hold and ? labels or the main overlay, so the final box is right 18, bottom 200, font 14. (b) red removed from the sparkline series cycle, commit 0ecf66f.
- Pass 2: docs/ui-taste/pass2-inspector.png. Minimum 6.13:1. check-inspector ok:true (docs/ui-taste/pass2-check.png). check-hud-overlap exit 0, uiLabels 23, uiLabelOverlaps 0. The DRAW sparkline is now the neutral text colour; no other visible problem.
- Pass 3 (Memory tab selected): docs/ui-taste/pass3-inspector-memory-tab.png. Minimum 6.13:1. check-inspector ok:true (docs/ui-taste/pass3-check.png). check-hud-overlap exit 0, uiLabelOverlaps 0. Nothing visible changed, so the loop stopped at three passes.

### Open

- Layout, not palette: at 1440x900 the inspector panel (fixed, top 10) covers the page header, the h1, the app-status pill and the "Inspector: I" toggle. Their colours are measured above (status ready 8.44:1) but they are not visible in the screenshots. Left for a layout change.
- The native witness checks label rectangles, not colour contrast. The native HUD colour is the loading text only (#e8f1ff).

### Processes

Stopped by PID at the end of the round, as trees: the playwriter browser (chromium 2576749 and its children, including the two crashpad handlers 2576766 and 2576768), its gate and project processes (2576690, 2576681 and 2576697), and the serve-wasm job (2576659, 2576663, 2576664). No viewer process from check-hud-overlap remained.

## Round 29 Phase A: inspector panel below the header

Defect: the Render Inspector was position: fixed (top 10, left 10, right 10, z-index 20), so with the panel open it covered the page header, including the "Inspector: I" toggle and the app-status pill at 1440x900.

Change, in web/wasm-index.html, the #render-inspector rule (commit on main, as lanmower, no trailers): position: fixed, top, left, right and z-index are removed, and margin: 10px 10px 0 places the panel in normal flow below the header. The open panel now pushes the stage down instead of covering the header. No z-index was changed to hide anything.

Checked by reading only: no script or page script depends on the panel being fixed (grep of scripts/, web/*.js, web/*.html, src/ for render-inspector, inspector-toggle and app-status). Phase B witnesses the layout in the browser at 1440x900 and 1024x768.

## Round 29 Phase B: inspector panel below the header, witnessed

Viewer: http://127.0.0.1:8092/web/wasm/index.html?inspector=1&still=1 through gm's playwriter (debug port 10232), 20 s wait after each load, the inspector open. Measured with getBoundingClientRect and elementFromPoint at the centre of each element. Clicks use playwright's own click, which fails if another element covers the target.

Defect found in witness: after load the bevy canvas takes focus (winit calls canvas.focus() when the window is active), and that focus scrolled the page by 117 px at 1024x768, so the header and the toggle sat at y -117 and the toggle was off-screen. Fix: web/wasm-boot.js overrides HTMLElement.prototype.focus for canvas elements only, so they focus with preventScroll: true. Other elements keep the default focus. The file is copied verbatim into web/wasm by build-wasm.sh, so no wasm rebuild is needed.

### Element boxes (CSS px, scrollY 0 in every row)

| Size | Element | x | y | w | h | right | bottom |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1440x900 | header | 0 | 0 | 1425 | 63 | 1425 | 63 |
| 1440x900 | #inspector-toggle | 1091.9 | 14 | 93.2 | 34 | 1185.1 | 48 |
| 1440x900 | #app-status | 1193.1 | 15 | 211.9 | 32 | 1405 | 47 |
| 1440x900 | #render-inspector | 10 | 73 | 1405 | 405.2 | 1415 | 478.2 |
| 1024x768 | header | 0 | 0 | 1009 | 63 | 1009 | 63 |
| 1024x768 | #inspector-toggle | 675.9 | 14 | 93.2 | 34 | 769.1 | 48 |
| 1024x768 | #app-status | 777.1 | 15 | 211.9 | 32 | 989 | 47 |
| 1024x768 | #render-inspector | 10 | 73 | 989 | 468.1 | 999 | 541.1 |

Overlap of toggle and status with the panel: 0 at both sizes. elementFromPoint at the toggle centre and the status centre returns the toggle and the status at both sizes. Clicking the toggle with the panel open closes it (data-inspector closed), and clicking again reopens it (open), at both sizes.

Before (the old fixed rule injected over the new page, since the old file is no longer in the tree): panel at x 10, y 10, 1420 x 405.2 at 1440x900 and 989 x 468.1 at 1024x768. Overlap of the toggle with the panel: 3168.8 px2 at both sizes. Overlap of the status with the panel: 6780.8 px2 at both sizes. elementFromPoint at both centres returns a span inside the panel, so the toggle and status were not clickable.

Screenshots: docs/ui-layout/before-1440x900.png, before-1024x768.png (old rule injected), after-1440x900.png, after-1024x768.png.

### Other witnesses

- scripts/check-inspector.cjs with CDP_URL=http://127.0.0.1:10232: exit 0, ok true, failures []. Output screenshot: docs/ui-layout/check-inspector.png.
- scripts/check-hud-overlap.sh with DISPLAY=:1.0: exit 0, uiLabels 23, uiLabelOverlaps 0.

### Open

- The metric strip (.strip, min-width 720px) scrolls horizontally inside the panel at 1024x768. It does not cover the header, toggle or status. It is an existing narrow-width rule and was left as it is.

## Round 30 Phase A: metric strip and tabs fit the inspector panel

Defect: at 1024x768 the inspector showed a horizontal scrollbar. The strip rule had min-width 720px, and the tab row had overflow-x auto. The scrollbar in the earlier screenshot sat under the tab row. Phase B measures which element scrolls.

Changes, in web/wasm-index.html (each committed as lanmower, no trailers):

- .strip: min-width 720px removed, replaced with min-width 0 and flex-wrap: wrap, so the metric buckets wrap onto a second line instead of forcing a minimum width.
- .tabs: overflow-x auto removed, replaced with flex-wrap: wrap, so the tab buttons wrap instead of scrolling sideways.

Not changed: .bucket, .spark, .metrics and the inspector body. Phase B measures them and records any defect.

## Round 30 Phase B: strip and tabs fit the panel, witnessed

Viewer: http://127.0.0.1:8092/web/wasm/index.html?inspector=1&still=1 through gm's playwriter, 20 s wait after each load, the inspector open. scrollWidth and clientWidth are read from each element. Before values come from the old rules injected over the new page (the strip rule min-width 720px with nowrap, the tab rule overflow-x auto with nowrap), not from the old file.

Correction to the Round 29 report: at 1024x768 the metric strip did not scroll. Its scrollWidth and clientWidth were both 794. The element that scrolled was the tab row (scrollWidth 1244, clientWidth 987), and four tabs (Physics, Vibes, Parameters, Diagnostic) sat outside the panel.

### Numbers (CSS px)

| Size | Element | Before scrollWidth / clientWidth | After scrollWidth / clientWidth |
| --- | --- | --- | --- |
| 1440x900 | #inspector-strip | 1210 / 1210 | 1210 / 1210 |
| 1440x900 | #inspector-tabs | 1403 / 1403 | 1403 / 1403 |
| 1440x900 | #render-inspector height | 405.2 | 420.2 (strip wraps SIM and LAUNCH to a second row) |
| 1024x768 | #inspector-strip | 794 / 794 | 794 / 794 |
| 1024x768 | #inspector-tabs | 1244 / 987 (257 px overflow) | 987 / 987 |
| 1024x768 | #render-inspector height | 468.1 | 515.1 (tabs wrap to two rows) |

Buckets: 7 of 7 inside the panel at both sizes, before and after. None clipped at either size (each bucket scrollWidth equals clientWidth). Tabs outside the panel: before at 1024x768 four (Physics, Vibes, Parameters, Diagnostic); after none at either size. Bucket width is at least 82 px, so each bucket is readable.

Screenshots: docs/ui-layout/strip-before-1440x900.png, strip-after-1440x900.png, strip-before-1024x768.png, strip-after-1024x768.png.

### Other witnesses

- scripts/check-inspector.cjs with CDP_URL=http://127.0.0.1:10232: exit 0, ok true, failures []. Output screenshot: docs/ui-layout/check-inspector-r30.png.
- scripts/check-hud-overlap.sh with DISPLAY=:1.0: exit 0, uiLabels 23, uiLabelOverlaps 0.

### Open

- Page-level horizontal overflow at 1024x768, outside the panel: document scrollWidth 1048 against clientWidth 1009. main is min(100vw, 1280px) wide plus 12 px padding each side (1048 px), and the stage is 1026 px. It predates this round and is not fixed here. A one-line fix would be box-sizing: border-box on main and the stage, to be done in its own round.
- The inspector body (max-height 42vh) now scrolls vertically at 1024x768, because the wrapped tabs make the panel taller. That is the existing max-height rule working as designed.

## Round 31 Phase A: page no longer wider than the viewport at 1024x768

Defect: at 1024x768 the document scrollWidth was 1048 against a clientWidth of 1009. main was min(100vw, 1280px) plus 12 px padding on each side (1048 px with its content-box width), and the stage was 1026 px. Two causes: 100vw includes the 15 px vertical scrollbar that the document's client width excludes, and the stage's 2 px border was added to its width.

Changes, in web/wasm-index.html (each committed as lanmower, no trailers):

- main: box-sizing: border-box, and width min(100%, 1280px) in place of min(100vw, 1280px). The padding is now inside the width, and the width follows the parent, not the viewport.
- #stage: box-sizing: border-box, and width 100% in place of min(100vw, 1280px). The border is inside the width, and the stage fills main's content box, so it cannot pass the viewport.

Not changed: the stage height, the inspector panel, the header and the canvas. Phase B measures the document scrollWidth against clientWidth, the stage's right edge, and the header, status pill and toggle at both sizes.

## Round 31 Phase B: page fits the viewport at 1024x768 and 1440x900

Viewer: http://127.0.0.1:8092/web/wasm/index.html?inspector=1&still=1 through gm's playwriter, viewport set per size, 20 s wait after each load, the inspector open. Before values come from the old rules injected over the new page (main and #stage as content-box with min(100vw, 1280px)), not from the old file.

Defect found in witness and fixed in this phase: the Phase A entry said #stage takes width 100%. #stage is a sibling of main, not a child, so 100% made the stage the full body width (1425 px) at 1440x900, where it had been a centred 1280 px. The fix is #stage width min(100%, 1280px), committed separately. At 1024x768 the stage is still the full 1009 px, and at 1440x900 it is centred at 1280 px again.

### Numbers (CSS px; docClientWidth excludes the vertical scrollbar)

| Size | Measure | Before | After |
| --- | --- | --- | --- |
| 1024x768 | document scrollWidth / clientWidth | 1048 / 1009 | 1009 / 1009 |
| 1024x768 | main x, width | 0, 1048 | 0, 1009 |
| 1024x768 | #stage x, width, right edge | 0, 1026, 1026 (outside 1009) | 0, 1009, 1009 (inside) |
| 1024x768 | header width, right edge | 1009, 1009 | 1009, 1009 |
| 1024x768 | #inspector-toggle x, y, w, h, right | 675.9, 14, 93.2, 34, 769.1 | 675.9, 14, 93.2, 34, 769.1 |
| 1024x768 | #app-status x, y, w, h, right | 777.1, 15, 211.9, 32, 989 | 777.1, 15, 211.9, 32, 989 |
| 1440x900 | document scrollWidth / clientWidth | 1425 / 1425 | 1425 / 1425 |
| 1440x900 | main x, width | 60.5, 1304 | 72.5, 1280 |
| 1440x900 | #stage x, width, right edge | 71.5, 1282, 1353.5 | 72.5, 1280, 1352.5 |
| 1440x900 | header width, right edge | 1425, 1425 | 1425, 1425 |
| 1440x900 | #inspector-toggle x, y, w, h, right | 1091.9, 14, 93.2, 34, 1185.1 | 1091.9, 14, 93.2, 34, 1185.1 |
| 1440x900 | #app-status x, y, w, h, right | 1193.1, 15, 211.9, 32, 1405 | 1193.1, 15, 211.9, 32, 1405 |

After the fix, at both sizes the toggle and status lie inside the header's box, elementFromPoint at each centre returns the toggle and the status, scrollX is 0, and appReady is true.

At 1440x900 the stage is 2 px narrower than before (1280 against 1282), because its border is now inside the width. This is the intended border-box effect.

Screenshots: docs/ui-layout/page-before-1024x768.png, page-after-1024x768.png, page-before-1440x900.png, page-after-1440x900.png.

### Other witnesses

- scripts/check-inspector.cjs with CDP_URL=http://127.0.0.1:10232: exit 0, ok true, failures []. Output screenshot: docs/ui-layout/check-inspector-r31.png.
- scripts/check-hud-overlap.sh with DISPLAY=:1.0: exit 0, uiLabels 23, uiLabelOverlaps 0.

## Round 32 Phase A: slack in the mirror batching bound

Owner decision: allow slack in the mirror batching bound. The floor (MIRROR_SLOTS 1, MIRROR_PROXY 0) fails a new object kind that puts a second material on an existing shape. The slack must be measured, and it must not exceed what one second material needs.

The mechanism, read from the code and not yet measured: each primitive takes its material from `palette[palette_slot(desc.kind)]`, a hash of the primitive kind name modulo 4 (src/viewer_world.rs:422), and its mesh from `primitive_shape`, which for any kind that is not sphere, capsule or cylinder is the cuboid of its size (src/viewer_world.rs:353). An unknown primitive name takes the spec's default arm with size 1 (src/spec.rs:311), so it shares the unit cuboid with `cube` at size 1. The palette slots, from the same hash: `cube` 1, `boxy` 2, `pad` 3. So:

- Case A, a second material on one shape: `cube` x3 and `boxy` x1 (`testdata/spec-world/places/batch-second-material`). Expected by the formula: one shape, two batch keys. Floor bound 1, so it fails; with one slack, bound 2, so it passes.
- Case B, one shape split across three materials: `cube` x2, `boxy` x2 and `pad` x2 (`testdata/spec-world/places/batch-split-shape`). Expected: one shape, three keys. Floor bound 1 and slack-1 bound 2, so it fails at both.

Slack value: 1. A second material on an existing shape adds exactly one (shape, material) pair, so one extra key is the most that case needs. A larger slack would let case B pass.

Changes, each a local commit by lanmower with no trailers; no build, test, gate or witness ran in Phase A:

| Change | Commit | Files |
| --- | --- | --- |
| `scripts/check-viewer-batching.cjs`: `slack` = `MIRROR_SLACK` (default 1) added to the bound; the bound is `shapesCreated x MIRROR_SLOTS + MIRROR_PROXY + MIRROR_SLACK`; `slack` is reported at the top level and in each result | b68be2f | `scripts/check-viewer-batching.cjs` |
| Two batching case places under the spec-world fixture: `batch-second-material` (case A) and `batch-split-shape` (case B), each with `config.yaml` (mode 3d, gravity) and `cells/x0z0.scene` (static cuboid objects) | 40e2511 | `testdata/spec-world/places/batch-second-material/`, `testdata/spec-world/places/batch-split-shape/` |
| AGENTS.md: the bound line states `MIRROR_SLACK` 1 and the one-extra-pair meaning | 421ade2, the last commit of Round 32 Phase A (it also carries this entry) | `AGENTS.md` |

Phase B plan: run `target/debug/bevy-jayzee viewer` on `testdata/spec-world` for `batch-second-material`, `batch-split-shape`, `test` (static), `main` (neon mirror) and `hud-fixture` `fixture`; each writes `web/status-<place>.json`; run `scripts/check-viewer-batching.cjs` on the five files and quote the bound, the keys and the result of each row. The existing static, mirror and fixture rows must stay green.

## Round 32 Phase B: slack in the mirror batching bound, witnessed

Build: none. The Rust source last changed before the dev binary `target/debug/bevy-jayzee` (built 13:43:15, the last commit touching `src/` is 13:43:30 and carries no later source edit), so no cargo ran. Memory at the start: 4.7 GB available, 14 GB of swap free.

Slack value: MIRROR_SLACK = 1, a global term. Measured reason: each case differs from the floor by whole (shape, material) pairs. The second-material case has 3 mirror keys on 2 shapes, one pair above the floor. The split-shape case has 4 keys on 2 shapes, two pairs above. A slack of 1 admits the one pair and rejects the second. The three existing rows have no pair above the floor, so the slack changes none of them. A slack of 2 would let split-shape pass. The slack is global, not per shape, so two shapes each with a second material would be two pairs and would fail.

Viewer runs (`target/debug/bevy-jayzee viewer <world> <place>`, DISPLAY :1.0, each under `timeout 60`, rc 124 on each, the expected timeout, status written before it):

| Place | World | Status file |
| --- | --- | --- |
| batch-second-material (case A) | testdata/spec-world | web/status-batch-second-material.json |
| batch-split-shape (case B) | testdata/spec-world | web/status-batch-split-shape.json |
| test (static) | testdata/spec-world | web/status-test.json |
| main (mirror) | /workspace/neon-syndicates/world | web/status-main.json |
| fixture (fixture) | testdata/hud-fixture | web/status-fixture.json |

The `local-player` marker is in every place as a second shape with one material (a sphere), so each status counts it. That is why case A has 2 shapes, not 1.

Results, `node scripts/check-viewer-batching.cjs` (default slack 1) and with MIRROR_SLACK=0 (the old floor):

| Row | shapesCreated | objectDraws / objectBatchKeys | mirrorBatchKeys | bound, slack 0 / slack 1 | mirrorsWithinBound, slack 0 / slack 1 | objectsBatched | meshesShared | ok, slack 0 / slack 1 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Case A, batch-second-material | 2 | 5 / 3 | 3 | 2 / 3 | false / true | true | true | false / true |
| Case B, batch-split-shape | 2 | 7 / 4 | 4 | 2 / 3 | false / false | true | true | false / false |
| Static, test | 10 | 8 / 8 | 8 | 10 / 11 | true / true | true | true | true / true |
| Mirror, main | 2 | 42 / 3 | 2 | 2 / 3 | true / true | true | true | true / true |
| Fixture, fixture | 1 | 6 / 1 | 1 | 1 / 2 | true / true | true | true | true / true |

The checker on the three existing rows alone (test, main, fixture) at slack 1: exit 0, `ok` true, `slack` 1. The checker on all five files at slack 1: exit 1, and the only failing row is case B, which is the broken-sharing case that must fail.

Witness outcome: case A passes at the slack and fails at the floor. Case B fails at both. The three existing rows pass at both. The bound is the floor plus one (shapes x 1 + 0 + 1), so it is not raised above what one second material needs.

Broken-sharing scope: case B is a shape split across three palette slots (material sharing broken). The mesh-sharing check (`meshesShared`) is a separate check and was not re-witnessed here, because no case in this round breaks mesh sharing.

Processes: each viewer ended on its `timeout 60`; no viewer remains (checked); no browser or server was started in this round.

## Round 33 Phase A: GPU adapter guard, bench medians and the ranked list

Phase A is code, docs and local commits only. Nothing here was built, run or witnessed. Phase B builds, reproduces the panic and runs the bench.

### 1. The `Unable to find a GPU` panic

Code path that produces it: Bevy 0.20.1, `bevy_render-0.20.1/src/renderer/mod.rs`. `initialize_renderer` calls `instance.request_adapter(...).await.ok()`, and when that is `None` it runs `selected_adapter.expect(GPU_NOT_FOUND_ERROR_MESSAGE)` (line 308). The message is "Unable to find a GPU! Make sure you have installed required drivers! For extra information, see: .../linux_dependencies.md" on Linux. The browser path is the same call: `navigator.gpu.requestAdapter`, which Bevy's wasm build makes through wgpu.

So the panic needs one thing: a null adapter from the request, for any reason (no driver, a transient failure, or a fallback adapter the browser declines). The one-off in the first ASTC gate run was a null adapter on the `?adapter=software` page. Its trigger is not known, because it was not reproduced in two later runs.

Changes:

- `src/main.rs`: `exit_when_no_gpu_adapter()` installs a panic hook. A panic whose message contains "Unable to find a GPU" prints one line that says no GPU adapter was found and exits with status 3. Every other panic goes to the default hook. The hook is installed first in `main`, so it covers every command, and `world`, `bench` and `det` never reach the GPU path. Commit b4fe35b.
- `web/wasm-boot.js`: before it imports the wasm module, the page calls `navigator.gpu.requestAdapter()` itself. A null result (or a rejection) shows the boot-error box with "no WebGPU adapter" and does not import the module, so Bevy never reaches the `expect`. A non-null result loads the module as before. Commit 5e1c322. The copy in `web/wasm/` is refreshed in Phase B by `build-wasm.sh`'s verbatim copy, or by hand when no build runs.

The reproduction and the guard's output are Phase B witnesses.

### 2. Bench: median over several runs, with the load

- `scripts/bench-vs-jayzee.sh`: runs each body count `RUNS` times (default 5, was one run). Before each run the script reads `/proc/loadavg`, and after the run it reads it again. Each row records `loadBefore`, `loadAfter`, `run` and both engines' output.
- `scripts/bench-median.cjs` (new): aggregates the rows. A run is marked noisy when the 1-minute load moves by more than `LOAD_MOVE` (default 0.5) across the run, or when other runnable tasks (the `running` field of `/proc/loadavg`, minus the reader) reach half the core count at either reading. The table reports the median over all runs and the median over the quiet runs only, with the speedup of each, and the noisy-run count. The full rows, with every load reading, go to `target/witness/bench-vs-jayzee.json`.

### 3. Ranked remaining work after Round 33

Each item is either done in a round named here or left undone, with the reason.

1. A real non-BC GPU. Open. This host has no adapter without BC: the NVIDIA adapter and SwiftShader both report it. The device-request path is witnessed instead, so this needs hardware that is not here.
2. ASTC and ETC2 texel quality. Left as it is, by dispatcher decision. It needs a new block-decoder dependency, which is a dispatcher decision for later.
3. The cross-adapter screenshot caveat: BC7 is compared with a hardware RGBA8 render, and ASTC and ETC2 with a SwiftShader RGBA8 render. Open. A SwiftShader BC7 set would remove the difference. Left undone: the BC7 reference already passes, and the queue did not ask for it.
4. The one-off `Unable to find a GPU` in the first ASTC gate run. Guarded in Round 33 (the native hook and the browser pre-check), so the failure now reports a missing adapter and does not panic. The cause of the one-off is still unknown. Open: if it recurs, record the model and the adapter log.
5. The bench on a quiet host. Partly done in Round 33: the bench reports medians over five runs and marks each noisy run with its load readings. Still open: this host is not quiet, so the medians need a quiet host to be a clean regression baseline. Left undone: needs an idle machine.
6. The load gate on the other checks. Open: the device fallback and texture checks still compare single runs at a 1.1 tolerance (the flip in Round 26). Left undone: outside this queue; the bench is the only check changed here.
7. The Round 23 list, unchanged: colour and transform transitions, `skew`, `matrix` and 3D transform functions, non-uniform scale with rotation, Oklab interpolation. Left undone: each is a feature, not a witness, and the queue did not ask for them.

Done since the Round 26 ranked list: the batching floor is now a measured slack of 1 (Round 32); the inspector panel no longer covers the header (Round 29); the strip and tab rows fit the panel (Round 30); the page no longer overflows at 1024x768 (Round 31).

Changes, each a local commit by lanmower with no trailers: b4fe35b (`src/main.rs`), 5e1c322 (`web/wasm-boot.js`), the bench commit (`scripts/bench-median.cjs`, `scripts/bench-vs-jayzee.sh`), and the commit that carries this section.

## Round 33 Phase B: GPU adapter guard and bench medians, witnessed

Builds, one cargo at a time, each with rc and warnings:

| Build | Command | Result |
| --- | --- | --- |
| Debug, with the native guard | `/config/.local/bin/cargo build` | rc 0, 20.22 s, 0 warnings |
| Release, for the bench | `/config/.local/bin/cargo build --release` | rc 0, 17 m 39 s, 0 warnings |

Memory before the release build: 7.6 GB available, swap-in and swap-out at zero in the last samples.

### 1. The GPU panic: reproduced, then guarded

Native reproduction, on the pre-guard debug binary, with the Vulkan loader pointed at no drivers (`VK_ICD_FILENAMES=/nonexistent VK_DRIVER_FILES=/nonexistent WGPU_BACKEND=vulkan`), `DISPLAY=:1.0`, `timeout 60`:

- rc 101. stderr: `thread 'main' (2632731) panicked at .../bevy_render-0.20.1/src/renderer/mod.rs:308:36:` then `Unable to find a GPU! Make sure you have installed required drivers! For extra information, see: https://github.com/bevyengine/bevy/blob/latest/docs/linux_dependencies.md`. Preceded by `vkCreateInstance: Found no drivers!`.

Native guard, on the Round 33 debug binary, same environment:

- rc 3. stderr has no `panicked` line. The guard prints: `bevy-jayzee: no GPU adapter was found, so the renderer cannot start. Check the graphics driver (on Linux, the Vulkan ICD files) and run again. Exit status 3.`

Normal native run, same binary, no driver override: `viewer testdata/spec-world test` runs to the timeout (rc 124, as before). `web/status-test.json` has `objectDraws` 8, `shapesCreated` 10, `mirrorBatchKeys` 8, `objectBatchKeys` 8, `modelsMissing` [], `missingObjects` [], `textureTarget` bc7. `scripts/check-viewer-batching.cjs` on it: `ok` true. These are the Round 32 values.

Browser reproduction, on the pre-guard `web/wasm-boot.js` (db90f1f), in the gate's Chromium at 1024x768, with an init script that makes `navigator.gpu.requestAdapter` return null:

- The console has `panicked at .../bevy_render-0.20.1/src/renderer/mod.rs:308:36: Unable to find a GPU! Make sure you have installed required drivers!`, then `pageerror: RuntimeError: unreachable`. The status reads `WebGPU WASM blocked: headless WebGPU runtime failed`, and the boot box reads `headless WebGPU runtime failed: Uncaught RuntimeError: unreachable`. The status names the wrong cause, because the wasm trap is what the existing error handler sees. Screenshot: `docs/ui-layout/gpu-missing-before.png`.

Browser guard, on the Round 33 `web/wasm-boot.js`, same init script:

- The status reads `WebGPU WASM blocked: no WebGPU adapter`, `ready` is `false`, and the boot box reads `no WebGPU adapter: navigator.gpu.requestAdapter returned no adapter, so Bevy cannot start its renderer (it would panic with "Unable to find a GPU"). Check the browser GPU settings or the adapter selector in the URL, then reload.` The console has no `panicked` line and no `pageerror`. The wasm module is not imported. Screenshot: `docs/ui-layout/gpu-missing-after.png`.

Browser normal run, on the Round 33 boot script, no init script, a fresh page: `Bevy WebGPU WASM initialized`, `ready` true, `window.__BEVY_WASM_READY` true, the boot box hidden, the adapter `NVIDIA GeForce RTX 3060 Laptop GPU`, and the log line `bevy-jayzee WebGPU WASM app-ready`. Screenshot: `docs/ui-layout/normal-after-gpu-guard.png`.

The one-off panic in the first ASTC gate run: the cause is still not known. Both the native and the browser reproduction show the same mechanism: a null adapter reaches Bevy's `expect` at `mod.rs:308`. The guard covers that path in both places. What triggered the null in the gate run is not reproduced.

### 2. Bench: median over five runs, with the load before and after each run

Command: `RUNS=5 scripts/bench-vs-jayzee.sh` (release binary, `TICKS` 300, 12 cores). Each body count ran five times, jayzee and bevy back to back in each run, with `/proc/loadavg` read before and after each run. Two complete passes were run; each row's `loadBefore` and `loadAfter` are in `target/witness/bench-vs-jayzee-run1.json` and `-run2.json`.

The host was busy in both passes: the 1-minute load was 5.7 to 7.1, and most runs read six or more other runnable tasks, so most runs are marked noisy. The load came from other sessions on this host (other `claude` processes, Xvfb, selkies and the gm worker), and from this round's browser, which was stopped before the second pass.

Pass 2, `target/witness/bench-vs-jayzee-run2.json`, medians in ms:

| bodies | runs | noisy runs | jayzee median | bevy median | speedup | quiet runs | jayzee quiet median | bevy quiet median | quiet speedup |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 500 | 5 | 3 | 0.433 | 0.5458 | 0.79 | 2 | 0.4355 | 0.566 | 0.77 |
| 2000 | 5 | 3 | 3.743 | 1.4395 | 2.60 | 2 | 4.1055 | 1.4021 | 2.93 |
| 5000 | 5 | 2 | 13.201 | 2.8786 | 4.59 | 3 | 13.201 | 2.8921 | 4.56 |

Pass 1, `target/witness/bench-vs-jayzee-run1.json`, the first run of the new script, medians in ms: 500 bodies 0.414 against 0.5632 (speedup 0.74, 3 of 5 noisy); 2000 bodies 3.778 against 1.6647 (2.27, 4 of 5 noisy); 5000 bodies 12.926 against 2.7803 (4.65, 3 of 5 noisy).

Per-run load readings, pass 2 (the 1-minute load, then the runnable count): at 500 bodies run 1 went from `5.68 6.58 5.30 10/2410` to `5.68 6.58 5.30 4/2410`, run 4 from `5.68 6.58 5.30 7/2410` to `…14/2419`. At 5000 bodies the 1-minute load ranged 5.80 to 6.27 across the five runs, and the runnable count ranged 2 to 9.

Reading the bench: the speedups at 2000 and 5000 bodies agree between the passes and with the AGENTS.md baseline (1.04 and 2.78 at 2000 bodies, 1.88 and 8.96 at 5000, ratios 2.67 and 4.77), so the engines are measured consistently. The 500-body row is inconclusive: bevy is 0.55 ms against the recorded baseline of 0.40, and the two passes disagree on the ratio (0.74 and 0.79, both below 1). AGENTS.md (line 46) says a later run is compared with the baseline on the same host before it is called a regression; the 500-body baseline ratio is 0.43/0.40 = 1.08 and this host gave 0.79, so this is not called a regression.

### 3. Ranked list

Updated in the Round 33 Phase A section above. This phase changed nothing in it.

### Processes

The wasm server tree (root 2632762) and the playwriter project tree (root 2632800) were stopped by PID as trees, with the gate's browser and renderer processes. The debug and release builds, the viewers and both bench passes exited on their own. No process from this round remains. The `playwriter-ws-server` (PID 2616609) and the other agents' processes predate this round and were left alone.

## Round 34 Phase A: the 500-body bench regression

Phase A is code, docs and local commits only. Nothing here was built, run or measured. The measurements are Phase B.

### What the reading established (no build)

The baseline is commit dcc84d8 ("Measure against jayzee on the same host"), which recorded 0.40 vs 0.43 ms at 500 bodies and 1.04 vs 2.78 at 2000 and 8.96 at 5000 (AGENTS.md line 46). Since dcc84d8, on the bench path:

- `src/bench.rs`: unchanged (`git diff dcc84d8 HEAD -- src/bench.rs` is empty).
- `src/protocol.rs`, the release profile, `.cargo/config.toml`: unchanged. The bench's tick, gravity and ball radius are the same constants.
- Dependency versions: avian3d 0.8.0, parry3d 0.31.1, bevy 0.20.1 (bevy_ecs, bevy_tasks, bevy_app) are the same. The lock file's only removed line is a transitive `convert_case` entry.
- Avian's feature list is unchanged. Bevy's list gained `bevy_render`, `bevy_pbr`, `bevy_ui`, `bevy_winit`, `bevy_gltf`, `reflect_auto_register`, `ktx2`, `basis-universal` and others. `MinimalPlugins` (the bench's only Bevy plugin group) does not include the render or asset plugins, so these add code to the binary but no per-tick work of their own.
- No global allocator and no tracing subscriber in the crate or in the added dependencies (grep).

So the bench code and its physics inputs are identical to the baseline. The growth has to come from the binary's build (feature unification, added crates changing LTO inlining) or from the host.

The measured cost grows with body count. Pass 2 of Round 33 gives bevy minus jayzee of 0.12 ms at 500 bodies, 0.4 ms at 2000 and 1.0 ms at 5000 when the bench was at about load 6 (on 12 cores). That is roughly 0.2 to 0.3 µs per body, so it is per-body cost, not per-tick overhead.

### Ranked candidates

1. Thread contention on a loaded host. The bench runs bevy with `ComputeTaskPool` on all 12 logical cores (`MinimalPlugins` and `avian3d`'s `parallel` feature), while jayzee is single-threaded. The host's 1-minute load was about 6 during the Round 33 runs, from other sessions, so bevy's parallel phases waited on busy cores. The baseline was recorded on a quieter host. This fits per-body growth and the 500-body row. Test in Phase B: pin bevy to one core with `BINARY_PREFIX="taskset -c 0"` and compare the ratio with the unpinned run on the same load.
2. The feature and crate additions changing the release codegen (LTO inlining in avian or bevy_ecs). Not measurable without a build of the dcc84d8 feature set. Test in Phase B only if (1) does not explain the regression.
3. The Round 33 panic hook in `src/main.rs`. It is installed before the command match, but it is only a hook: it does nothing per tick. Ruled out by reading.

### Changes, each a local commit by lanmower with no trailers

- 43f59c8 `Cargo.toml`: adds avian3d's `bevy_diagnostic` feature. The feature only compiles Avian's diagnostics code; `PhysicsPlugins` does not add `PhysicsDiagnosticsPlugin` (Avian's lib.rs says it "must be added separately"), so the normal bench runs the same systems as before.
- a81b593 `src/bench.rs`, `src/main.rs`: the bench JSON gains a `threads` field (the `ComputeTaskPool` thread count). A new `bench-phases` command adds `PhysicsDiagnosticsPlugin` and prints each physics phase's average from `DiagnosticsStore`, with the sample count. The normal `bench` path is unchanged except for the added field and the `diagnostics` flag, which is false there.
- c924f80 `scripts/bench-vs-jayzee.sh`: `BINARY_PREFIX` (empty by default) is put in front of the Bevy command, so a witness can pin bevy with `taskset`. The jayzee command and the default run are unchanged.

### Phase B plan

1. Measure before any change: `RUNS=5 scripts/bench-vs-jayzee.sh` on the existing release binary (built at 20:53, before these commits), with load before and after each run, medians of 500, 2000 and 5000. This is the "before".
2. Build the release binary once through cargo, then run `bench-phases` at 500, 2000 and 5000 to see which physics phase's average grows at 500 bodies.
3. Run the pinned variant (`BINARY_PREFIX="taskset -c 0"`) with the same method, to test candidate 1.
4. Fix the cause that the witnesses show. Do not change the bench to hide it. Re-run the same method for the witness, and check the batching, fixture and device-format checks.

## Round 34 Phase B: the 500-body regression, measured

Result: open. The regression is measured and its sensitivity to host load is clear, but no code change was made, and no phase can be named as the one that grew relative to the baseline, because no baseline phase numbers or baseline binary exist.

### Correction to the Phase A reading

Phase A said the cost grows per body, not as per-tick overhead. That came from the threaded runs alone. The pinned (one-core) runs and the schedule timer show a fixed cost: 0.16 ms per tick with zero bodies. The correction is recorded here, and the Phase A text above is left as written.

### Builds

- Release, with `bench-phases`: `/config/.local/bin/cargo build --release`, rc 0, 17 m 39 s, 0 warnings. A first attempt was stopped by this round's own 1500 s bound at the crate step, before finishing. Nothing was left running. The rebuild ran with a one-hour bound.
- Release, with `bench-schedules`: rc 0, 25 m 33 s, 0 warnings.
- Gate before the first build: MemAvailable 5.3 GB, SwapFree 15 GB, swap-out 0 in the vmstat samples. The free column read 1.8 to 1.9 GB during the second build; the schedule-timer build ran with swap-out at 0.

### 1. Before any change (the pre-change release binary, built 20:53, before Phase A)

Command: `RUNS=5 scripts/bench-vs-jayzee.sh`. Load 1-minute 6.2 to 7.7, runnable 2 to 13 on 12 cores. Medians in ms:

| bodies | jayzee | bevy | ratio | noisy runs |
| --- | --- | --- | --- | --- |
| 500 | 0.571 | 0.6586 | 0.867 | 0 of 5 |
| 2000 | 4.219 | 1.8141 | 2.326 | 2 of 5 |
| 5000 | 13.507 | 3.5436 | 3.812 | 2 of 5 |

Raw rows: `target/witness/bench-r34-pre.json`. The copy taken before the rows were complete was stale and was deleted; this table is the file written by the run that finished at 21:15:56.

Pinned to one core (`BINARY_PREFIX="taskset -c 0"`), same binary, load 7.4 to 8.1:

| bodies | jayzee | bevy | ratio |
| --- | --- | --- | --- |
| 500 | 0.408 | 0.6098 | 0.669 |
| 2000 | 3.309 | 2.2131 | 1.495 |
| 5000 | 12.258 | 6.7199 | 1.824 |

Raw rows: `target/witness/bench-r34-pinned-pre.json`.

### 2. Where the time goes (`bench-phases` and `bench-schedules`, unpinned, load about 3)

`bench-schedules` runs each Bevy `Main` schedule in the order `MainScheduleOrder` gives and times it, with the physics diagnostics off. Medians of three runs, in ms:

| schedule | 0 bodies | 500 | 2000 | 5000 |
| --- | --- | --- | --- | --- |
| RunFixedMainLoop (physics substeps) | 0.0859 | 0.2466 | 0.7307 | 1.3335 |
| PostUpdate | 0.0213 | 0.0194 | 0.0223 | 0.0222 |
| First | 0.0144 | 0.0112 | 0.0122 | 0.0116 |
| Update | 0.0136 | 0.0107 | 0.0121 | 0.0107 |
| PreUpdate | 0.0131 | 0.0105 | 0.0115 | 0.0102 |
| Last | 0.0123 | 0.0116 | 0.0102 | 0.0093 |
| tick | 0.1618 | 0.3111 | 0.8000 | 1.3986 |

Reading it: about 0.08 ms of every tick is Bevy's own cost (six schedules at about 0.013 ms each, even when empty). The rest is `RunFixedMainLoop`, which carries the physics. Its per-body cost is about 0.32 µs at 500 and 2000 bodies and 0.25 µs at 5000.

`bench-phases` splits the physics further using Avian's timers. The timed phases with the largest share, medians of three runs, at 500 and 5000 bodies:

- `avian/solver/integrate_velocities`: 0.0873 ms (17%) and 0.7975 ms (33%). This is the parallel per-body loop that runs once per substep.
- `avian/collision/update_contacts`: 0.0721 ms (14%) and 0.5064 ms (21%).
- The untimed remainder (Bevy and the un-timed Avian systems): 0.28 ms at 500 bodies and 0.69 ms at 5000.

The zero-body timed phases total about 0.007 ms, so the 0.16 ms fixed cost is untimed.

### 3. Thread contention

Pinning bevy to one core makes it slower at 2000 and 5000 bodies (table above) and not faster at 500. So parallelism helps, and the 500-body gap is not caused by the threads competing with each other. Under host load, the parallel loops still lose time to scheduling, which is why the ratios move with load (below).

### 4. After: no code change, same method, low host load

The same binary as the "before" table with the same five-run method, at load 2.6 to 3.0 with 1 to 4 other runnable tasks:

| bodies | jayzee | bevy | ratio | baseline ratio (AGENTS.md line 46) |
| --- | --- | --- | --- | --- |
| 500 | 0.321 | 0.3037 | 1.057 | 1.08 |
| 2000 | 2.061 | 0.8059 | 2.557 | 2.67 |
| 5000 | 6.736 | 1.4436 | 4.666 | 4.77 |

Raw rows: `target/witness/bench-r34-lowload.json`. This binary differs from the "before" binary by the diagnostics feature and the timer commands, which are not on the normal bench path. Its normal `bench` code is unchanged. The before and after figures are not from the same load, so the difference between them is partly host state.

Ratio across the three conditions:

- 500 bodies: 0.867 before (load 6 to 8), 0.669 pinned, 1.057 at low load. Baseline 1.08.
- 2000 bodies: 2.326 before, 2.557 at low load. Baseline 2.67.
- 5000 bodies: 3.812 before, 4.666 at low load. Baseline 4.77.

At low load the three ratios are 2 to 4% under their baselines, which is inside the spread of five runs at this host state. Two runs reached about 1.08 individually at 500 bodies. It is not a clean pass of the ≥1.08 target, and I have not claimed one.

### 5. Why this stays open

- The baseline binary no longer exists, and building the baseline commit needs a copy of the tree or a checkout over the shared one. Invariant 7 forbids both. So no phase can be shown to have grown relative to the baseline, and the feature-set cause (the Bevy features added after dcc84d8: `bevy_render`, `bevy_pbr`, `bevy_ui` and others, which the viewer needs) cannot be isolated by building without them.
- The bench code, Avian and Bevy versions, the release profile and the protocol constants are the same as at the baseline (Phase A reading). A code cause would have to come from the feature set or from codegen, which is the open question above.
- No fix was made. A fix in the bench would hide the cost. A fix in the engine configuration (for example the thread count) changes the viewer too, so that is a product decision for the owner.

### 6. Checks run

- `det 500 300`: pass, hashes equal (0x17ff9626).
- `det 2000 300`: pass, hashes equal (0x3a5f0b51).
- Native viewer on `testdata/spec-world test`: rc 124 by the 60 s timeout (expected), 0 panics; `objectDraws` 8, `shapesCreated` 10, `mirrorBatchKeys` 8, `objectBatchKeys` 8, `textureTarget` bc7; `check-viewer-batching.cjs` ok true.
- Fixture and device-format browser checks: not re-run in this round. No web, wasm or viewer source changed in Round 34, and the wasm crate has its own lock file. Ask for a browser run if you want them repeated.

### Commits this round (all lanmower, no trailers)

43f59c8 (Avian `bevy_diagnostic` feature), a81b593 (thread count and `bench-phases`), c924f80 (`BINARY_PREFIX`), a188691 (Phase A reading), the `bench-schedules` commit, and the commit that carries this section.

### Processes

No process from this round remains. The release builds and the bench runs exited on their own. The viewer exited on its 60 s timeout and no viewer remains.

### Ranked open items after Round 34

1. The 500-body ratio at low host load is 1.057 against the 1.08 baseline, and the other two are below their baselines by 2 to 4%. Open. Needs a baseline binary from the dcc84d8 feature set, which needs a decision on building it outside the shared checkout (invariant 7 forbids a copy).
2. Host load moves the bench ratio by a factor of about 1.3 at 500 bodies (0.87 to 1.06). The bench has no load gate that holds the medians steady. Open. Needs a quiet host or a load gate that the owner accepts.
3. Integrate-velocities and contact update (the largest timed phases) per body: 0.16 µs and 0.10 µs. Recorded for a future engine-side review. Not a witnessed defect.

## Round 35 Phase A: faster builds, measured before the change

Phase A is config and docs only. The builds below were measured in Phase A as the queue asked (the "before" column). The one change is the iterate profile. Phase B re-measures the same builds for the "after" column.

### Commands and the measured before times

All builds through `/config/.local/bin/cargo`, one at a time, each into its own target directory where a cold build was needed. Wall times from `date`, written by `/tmp/timed-build.sh` and the experiment driver.

| Build | Command | Target dir | Wall | Result |
| --- | --- | --- | --- | --- |
| Cold release, current config | `cargo build --release` | `target/timing-cold` | 1015 s (16 m 55 s) | rc 0, 0 warnings |
| Cold dev, current config | `cargo build` | `target/exp-dev-base` | 654 s | rc 0, 0 warnings |
| Incremental dev after a one-line change | edit `src/main.rs` usage line, `cargo build`, revert | `target/debug` | 3 s | rc 0; the binary contains the new string |
| Wasm release, up to date | `WASM_PROFILE=release scripts/build-wasm.sh` | `target/wasm-viewer` | 4 s | rc 0 |
| Wasm release, cold | the same, with the previous `target/wasm-viewer` moved to `target/wasm-viewer-prev` | `target/wasm-viewer` | 252 s (4 m 08 s), 312 crates | rc 0 |

The incremental dev figure is for an edit that recompiles only the `bevy-jayzee` crate and relinks it. The wasm figure is for `wasm-viewer`, which has its own lock file.

### Candidates measured and not adopted

- rust-lld, `-C link-arg=-B<sysroot>/lib/rustlib/x86_64-unknown-linux-gnu/bin/gcc-ld -C link-arg=-fuse-ld=lld` through `--config`. The sysroot's `gcc-ld` and `rust-lld` (LLD 23.1.1) are present. Results: cold dev 658 s against 654 s; incremental dev 4 s against 3 s; cold release 1110 s against 1015 s. The lld link gives no gain on any of the three, so it is not in the config. The incremental link is already short.
- Dependency opt-level 0 in the dev profile (`profile.dev.package."*".opt-level=0`, `debug=0`). Cold dev 204 s against 654 s (3.2 times faster). But the same world check runs 1.91 to 2.00 s against 0.32 to 0.52 s with the opt-3 dependencies (about 4 to 6 times slower). Iterating on physics needs the opt-3 dependencies, so the dev profile keeps `opt-level = 3` for dependencies. Round 6 had already rejected opt 1 for the same reason.
- A memory limit for each compile. `systemd-run` cannot work here (no systemd, no user bus), and `/sys/fs/cgroup` is root-owned and not writable by this user, so no cgroup limit can be set. The compile parallelism stays at cargo's default of 12. Round 6 recorded the same limit.

### Change adopted

- `Cargo.toml`: `[profile.iterate]`, inheriting release, with `lto = false`, `codegen-units = 16`, `incremental = true` and `strip = "none"`. Cold build 507 s with `incremental = false` (the first measurement) against 1015 s for release, so about half. Release is unchanged, and the performance witnesses keep using it. Phase B measures the cold build with the committed profile, and the incremental iterate build after a one-line change.

Commit: the `Cargo.toml` change, as lanmower with no trailers.
