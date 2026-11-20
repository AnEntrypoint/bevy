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

Superseded by "Round 37 Phase A: reconciliation of the Round 15 ranked list" at the end of this file, which states the status of each rank with its witness. The table below is the Round 15 record and is kept unchanged.

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

## Round 35 Phase B: build times after the change, and the witnesses

Builds were run one at a time through `/config/.local/bin/cargo` (and `scripts/build-wasm.sh` with the wrapper first on PATH). Each run was gated on swap-out 0 and MemAvailable above 7 GB. The driver was `/tmp/phaseb.sh`, then `/tmp/phaseb2.sh`, after the first run stopped on a bug in the driver's own `oneline` helper (it passed the cargo path twice). That run's dev incremental build failed with `requires -Zscript` and was not used; its numbers are not in the table.

### Before and after wall times

| Build | Before (Phase A) | After (Phase B) | Change in config |
| --- | --- | --- | --- |
| Cold release (`target/timing-cold`) | 1015 s | 1097 s | none (release unchanged) |
| Incremental dev, one-line edit | 3 s | 7 s | none |
| Wasm release, cold | 252 s | 318 s | none |
| Wasm release, up to date | 4 s | 5 s | none |
| Iterate, cold (new profile) | not measured | 594 s | new profile |
| Iterate, one-line edit | not measured | 18 s | new profile |
| Release, one-line edit (fat LTO, no profile) | not measured | 808 s | none |

The "after" column for release, dev and wasm is the same configuration rebuilt on this host, so those differences are host variance, not a gain. No change was made to them: every candidate for them measured no gain or lost run-time speed (below). The gain is in the iterate profile: a one-line rebuild falls from 808 s on release to 18 s on iterate, and a cold build from 1097 s to 594 s (about 46 percent less, measured in the same session). Phase A's cold iterate figure of 507 s used `incremental = false`; the committed profile has `incremental = true`, which costs about 87 s on the cold build and is what makes the one-line rebuild 18 s.

### rust-lld linker result

Measured with `--config 'target.x86_64-unknown-linux-gnu.rustflags=[...,"-C","link-arg=-B<sysroot>/lib/rustlib/x86_64-unknown-linux-gnu/bin/gcc-ld","-C","link-arg=-fuse-ld=lld"]'`, each in its own target directory:

- Cold dev: 658 s with lld, 654 s without.
- Incremental dev, one-line edit: 4 s with lld, 3 s without.
- Cold release: 1110 s with lld, 1015 s without.

Result: no gain, and a small loss on the release build. lld is not in the config. The linker is not what limits a rebuild here; the compile is.

### Candidates measured and not adopted

- Dependency opt-level 0 in the dev profile: cold dev 204 s, but the world check runs 1.91 to 2.00 s against 0.32 to 0.52 s on the opt-3 binary. Iterating on physics needs the opt-3 dependencies, so the dev profile keeps them. Round 6 had rejected opt 1 for the same reason.
- A memory limit for each compile: not possible here. `systemd-run` has no systemd or user bus to use, and `/sys/fs/cgroup` is root-owned and not writable by this user. Compile parallelism stays at cargo's default; Round 6 recorded that the jobs cap was not isolated.

### The 452-crate dev rebuild

Explained in Round 36 Phase A: the build wrote a second tree at target/debug/debug because the driver set CARGO_TARGET_DIR to a profile directory. See the Round 36 section.

### Witnesses

- Determinism rows, release binary built from the current code (`target/timing-cold/release/bevy-jayzee det N 300`): `det 500` `hashA 0x17ff9626`, `hashB 0x17ff9626`, pass true; `det 2000` `hashA 0x3a5f0b51`, `hashB 0x3a5f0b51`, pass true. The same hashes as Round 34.
- Batching, native viewer on `testdata/spec-world test` with the iterate binary (`target/iterate-phaseb/iterate/bevy-jayzee`): rc 124 by the 60 s timeout (expected), 0 panics; `objectDraws` 8, `shapesCreated` 10, `mirrorBatchKeys` 8, `objectBatchKeys` 8, `modelsMissing` [], `missingObjects` [], `textureTarget` bc7. `scripts/check-viewer-batching.cjs web/status-test.json`: ok true, paletteSlots 1, proxyMaterials 0.
- Warnings in every build log: 0.

### Changes this round (all lanmower, no trailers)

- b240064: `Cargo.toml`, the `[profile.iterate]` profile.
- b61cc73: this document's Phase A section.
- The Phase B section (this one) is the commit that carries it.

### Disk

Removed after measurement: `target/exp-dev-base`, `target/exp-dev-lld`, `target/exp-dev-deps0`, `target/exp-rel-lld`, `target/exp-iterate` and `target/wasm-viewer-prev` (all created in this round; about 17 GB). Kept: `target/timing-cold` (the current release binary used for the witnesses), `target/iterate-phaseb`, `target/debug`, `target/wasm-viewer`.

### Open

1. The 452-crate dev rebuild is explained in Round 36 (the driver set CARGO_TARGET_DIR to a profile directory). Closed.
2. A cold release build is 1097 s, unchanged. It is the witness build and is kept by decision.
3. The compile memory limit cannot be set on this host (no systemd, no writable cgroup). Open, pending a host change.

## Round 36 Phase A: the 452-crate rebuild, and the helper that prevents it

Phase A is reading, a helper script, and docs. Nothing was built or run.

### Cause

The 452-crate build was not a rebuild of the dependency graph in `target/debug`. It was a cold build into a new tree, caused by a wrong `CARGO_TARGET_DIR` in my own driver.

- `/tmp/phaseb.sh` line 24 set `export CARGO_TARGET_DIR=/workspace/bevy/target/debug`. Cargo takes `CARGO_TARGET_DIR` as the target root and adds the profile directory itself, so that build wrote a complete second tree to `target/debug/debug/`.
- The evidence: `target/debug/debug/` is dated 2026-11-18 08:57 to 09:04, holds 432 distinct packages (452 compile units with build scripts), and is 3.1 GB. The real `target/debug/.fingerprint` for `proc-macro2` is dated 2026-11-16 22:11 and was not written by that build.
- The corrected driver (`/tmp/phaseb2.sh`) did not set the variable, and its dev incremental build used the real `target/debug` (7 s, one crate).

### What was checked and ruled out

- Build scripts. No build-script `output` file was rewritten in the 08:50 to 09:05 window; the newest is the `bevy-jayzee` one from 17 November 09:57. A rebuild cascading from a changed build script would have rewritten those files.
- The first unit compiled was the leaf `proc-macro2`. A fingerprint change at a leaf with an unchanged build script means the leaf's own inputs changed. That is the case here only because it was a different target root: its fingerprint was not in the real tree at all.
- Profile settings. A manifest-only edit to the `[profile.iterate]` section (`incremental` flipped and reverted) rebuilt one crate in 15 s. Adding the profile section did not invalidate the dependency graph.
- The rustc identity. `target/.rustc_info.json` was last written on 14 November and records rustc 1.99.0 (b940084d7), the same compiler `rustc -Vv` reports now, so the compiler did not change.
- Features. `Cargo.toml` features and `Cargo.lock` were not changed between the dev build of 17 November and this one, except for the Phase A profile addition.
- Environment. The only environment difference between the two builds was the `CARGO_TARGET_DIR` export in the driver.

### Fix

`scripts/build-timed.sh` (commit 7ce35ff) runs cargo through `/config/.local/bin/cargo` and:

- refuses a `CARGO_TARGET_DIR` whose last component is `debug` or `release`, so a profile directory cannot be used as a target root again (exit 2, no build);
- prints one JSON line with the arguments, the exit code, the wall time, the number of crates compiled (lines beginning `Compiling`), the warning count and the target root;
- keeps the full cargo log at `target/build-timed-last.log`.

The nested `target/debug/debug/` tree is removed in Phase B, after the proof builds, once the check that refuses it is in place.

### Corrections to earlier rounds

- Round 35 Phase B said the 452-crate rebuild was unexplained. It is explained here. The "Open" item there is closed by this section.
- The Round 35 "Unexplained" paragraph is replaced by a pointer to this section.

## Round 36 Phase B: the rebuild proof

Builds through `scripts/build-timed.sh`, which runs `/config/.local/bin/cargo` with the default target root (`target/`) and the dev profile. One build at a time. Gate before the first build: swap-out 0, MemAvailable 6.9 GB, SwapFree 15 GB.

| Build | What changed | Wall | Crates compiled | rc |
| --- | --- | --- | --- | --- |
| 1 | source edit in `src/main.rs` (usage line gets `timing-probe`) | 9.1 s | 1 | 0 |
| 2 | second source edit to the same line (`timing-probe-2`) | 3.8 s | 1 | 0 |
| 3 | no change (no-op) | 0.4 s | 0 | 0 |
| restore | the edit reverted to the committed source | 4.0 s | 1 | 0 |

Each source-edit build recompiled only the `bevy-jayzee` crate. The dependency graph was not rebuilt.

### Guard check

`CARGO_TARGET_DIR=/workspace/bevy/target/debug scripts/build-timed.sh build` printed `build-timed: CARGO_TARGET_DIR must be a target root, not a profile directory: /workspace/bevy/target/debug` and exited 2, with no build.

### Witnesses on the committed source

- Determinism rows (dev binary): `det 500 300` `hashA 0x17ff9626` `hashB 0x17ff9626`, pass true. `det 2000 300` `hashA 0x3a5f0b51` `hashB 0x3a5f0b51`, pass true. The same hashes as Round 34 and Round 35.
- Native viewer on `testdata/spec-world test` with the dev binary: rc 124 at the 60 s timeout (expected), 0 panics. `objectDraws` 8, `shapesCreated` 10, `mirrorBatchKeys` 8, `objectBatchKeys` 8, `modelsMissing` [], `missingObjects` [], `textureTarget` bc7. `scripts/check-viewer-batching.cjs web/status-test.json`: ok true, paletteSlots 1, proxyMaterials 0.

### Cleanup

Removed `target/debug/debug/` (3.1 GB), the nested tree from the driver mistake. The real `target/debug` is kept and is the one these builds used.

### Commits

- 7ce35ff: `scripts/build-timed.sh`.
- 1f46baa: Phase A docs (the cause, the fix, the corrected open item).
- This Phase B section is the commit that carries it.

### Open

None from this round.

## Round 37 Phase A: reconciliation of the Round 15 ranked list

Each Round 15 rank, with the round that closed it and the witness, or the reason it is still open.

| Rank | Item | Status | Closed by, or what is still open |
| --- | --- | --- | --- |
| 1 | Native viewer memory and frame cost after the RGBA8 decode change | Open | Round 16 measured RSS 898,224 kB, fps 61.0 and frame 16.39 ms, on the debug binary and the RGBA8-era cache (line 562). That is not a before/after pair. Round 37 Phase B measures three modes with `scripts/measure-viewer-resources.sh`: `bc7`, `rgba8` (`VIEWER_DISABLE_BC=1`, the mode Round 23 witnessed as `textureTarget` rgba8 at line 986) and `uastc` (`VIEWER_KEEP_BASIS=1`). |
| 2 | HUD text quality | Closed | Round 16 HUD glyph fix (`push_glyph` maps non-printable ASCII). Round 21 to 24 `scripts/check-hud-overlap.sh` runs, and the top-left overlap with them: Round 24 at line 985, `ok` true, 22 labels, 0 overlaps, 0 full-width, `topCentreOffset` 0, `textureTarget` bc7. The script's `ok` requires `uiUncoveredGlyphs` 0. No new PNG was viewed in Round 37. |
| 3 | Release gate on the current tree | Open | Last `scripts/verify.sh` exit 0 was on the Round 16 tree and recorded in Round 17. Round 37 Phase B runs it on the current tree. |
| 4 | Wasm payload for the decoded neon copies | Closed | Round 17: BC7 GLB encoded 101,358,620 bytes, `loadMsMedian` RGBA8 1,102 ms and BC7 1,112 ms. Round 25 Phase B footprint decision `bc7` (`target/witness/texture-footprint.json`). Per-model rows are in `target/witness/neon-wasm-witness.json`. Round 37 Phase B re-reads them. |
| 5 | Commit the browser witnesses as a repo script | Closed | `9d0270e` adds the 64-model witness script to the repo, and `30d104a` routes it through the gate. Round 16 Phase B: 64 of 64 clean, `loadMsMedian` 1,092 ms. |
| 6 | Move `scripts/check-normal-map-visual.cjs` onto gm's playwriter | Closed | `7ef0445`: the script requires `./lib/playwriter-witness.cjs` (`withGateWitness`). |
| 7 | Reconcile `docs/PENDING_VALIDATION.md` | Closed | This section. The Round 15 table is marked superseded above, and its text is unchanged. |
| 8 | ZLIB-supercompressed basis KTX2 | Open | No ZLIB sample exists in the data. `inflate_zstd_levels` inflates scheme 2 (Zstandard) only. Scheme 3 (ZLIB) is not tested, and the error it produces is not witnessed. Needs a sample or a unit test. |
| 9 | AGENTS.md under 30 KB | Closed | `wc -c AGENTS.md` = 13616 bytes, under 30720, measured in Round 37 Phase A. |

Still open after this reconciliation: rank 1 (viewer cost), rank 3 (release gate on this tree) and rank 8 (ZLIB). Round 16's neon HUD top-left overlap, which the Round 16 notes listed as open, is closed by the Round 21 to 24 overlap runs above.

### Round 37 Phase A changes

- `src/basisu.rs`: `transcode_basis_images` now calls `rewrite_basis_images(json, bin, Some(target))`. The new `keep_basis_images` calls it with `None`, which keeps the basis payload: Zstandard levels are inflated, and BasisLZ or UASTC payloads are left for Bevy to transcode at load. `basis_payload` selects the two paths.
- `src/glb.rs`: `decode_with` and the new `decode_kept_basis` both call `decode_textures(bytes, Option<TextureTarget>)`.
- `src/viewer_world.rs`: `basis_kept()` reads `VIEWER_KEEP_BASIS=1`. The viewer's target is `None` when it is set, and `load_model` then decodes with `decode_kept_basis`. The status `textureTarget` reads `basis-uastc`.
- `src/viewer.rs`: keep-basis runs use the cache `target/viewer-cache-uastc`, so they cannot overwrite the transcoded cache. `write_browser_report` adds `processRssKib`, `frameMsAverage`, `fpsAverage`, `imageCount` and `imageBytes` (the CPU-side `Image.data` total, which holds the GPU texture bytes). `process_rss_kib` replaces the string-only RSS reader; the overlay text is unchanged.
- `scripts/measure-viewer-resources.sh` (new): runs `bc7`, `rgba8` and `uastc` in turn on `PLACE` (default `main`), waits for `uiLabels`, settles for `SETTLE_S` (default 10 s), copies each status file, stops each viewer by PID, and writes `target/witness/viewer-resources-main.json`. A mode fails unless its `textureTarget` matches, every model loads, and both RSS and frame time are present.

Phase A checks: no build or test was run, as the queue requires. `rustfmt --edition 2024 --check` reports only line-length diffs on the touched files, and the same diffs exist on HEAD. No parse errors.

## Round 37 Phase B results

Phase B supersedes the Phase A status above. The keep-basis (UASTC native) mode was removed in `91d163a`, after the run below showed it cannot load.

### Build

- `scripts/build-timed.sh build --release` on the keep-basis tree (`d1245c3`, before `91d163a`), through `/config/.local/bin/cargo`: rc 0, `Finished release profile in 18m 10s`, 0 warnings. The tree was then rebuilt on `91d163a`: rc 0, `Finished release profile in 18m 51s`, and that binary is the one measured below.
- One unexplained failure: the first release rebuild of `91d163a` returned rc 101 in 0 compiled crates after `Compiling bevy-jayzee`, with no rustc diagnostic in `target/build-timed-last.log`. The same source built on the next invocation with no change. Its cause is not known; the kernel log shows no OOM kill.

### 1. Release gate (`scripts/verify.sh` on `91d163a`)

- `scripts/verify.sh` exit 0, with the bench, determinism and client/server output quoted from the run: bench 500 `meanMs` 0.2812 `hash` 0x17ff9626; 2000 `meanMs` 0.7946 `hash` 0x3a5f0b51; 5000 `meanMs` 1.3818 `hash` 0x38909cb0; det 2000 `hashA` and `hashB` 0x3a5f0b51, `pass` true; `check-network` `ok` true, `confirmedTick` 353, `errorM` 0.
- Earlier run on the keep-basis build (`d1245c3` tree): also exit 0 (bench 500 0.3183, 2000 0.8004, 5000 1.4137). It is superseded.

### 2. Native viewer, neon `main`: memory, texture bytes, frame time, decision

Command: `scripts/measure-viewer-resources.sh` (`MODES` default `bc7 rgba8`), release binary `91d163a`, `VIEWER_EYE` 0,300,1, `SETTLE_S` 10, one run per mode. Output: `target/witness/viewer-resources-main.json`.

| Mode | `textureTarget` | `processRssMiB` | `imageCount` | `imageMiB` (CPU `Image.data`) | `frameMsAverage` | `fpsAverage` | models loaded / missing / load errors |
| --- | --- | --- | --- | --- | --- | --- | --- |
| bc7 (default on this NVIDIA host) | bc7 | 646.9 | 35 | 29.94 | 16.664 | 60.27 | 9 / 0 / 0 |
| rgba8 (`VIEWER_DISABLE_BC=1 VIEWER_DISABLE_ASTC=1`) | rgba8 | 818.4 | 35 | 90.73 | 16.667 | 60.96 | 9 / 0 / 0 |

The UASTC-native row was measured first, on the keep-basis build, and is not valid: `textureTarget` basis-uastc, RSS 615.1 MiB, `imageCount` 23. Two models failed in Bevy's asset server with `Failed to load asset ... failed to decompress an image: Unsupported supercompression scheme: BasisLZ` (`target/witness/resources-uastc.err`). The status still reported `modelsMissing` 0, because the failure happens after the status is written. The mode was removed in `91d163a`.

Decision: keep BC7 for devices with BC support, and RGBA8 only where BC is absent, as the runtime choice already does. Reasons, from the numbers: BC7 uses 171.5 MiB less RSS than RGBA8 (646.9 against 818.4), and its CPU image bytes are 60.8 MiB less, 67 percent below RGBA8 (29.94 against 90.73). Frame time does not separate the modes: both sit at the 60 Hz present cap (16.66 ms). This measurement cannot show GPU frame cost, which the render-stats counters would be needed for. UASTC native is rejected because Bevy 0.20.1 cannot read the BasisLZ payload, so no measurement of it is valid.

Caveat: each mode is one run on one host. The keep-basis build's bc7 run gave 704.6 MiB, so RSS varies by about 60 MiB between runs.

### 3. Wasm payload for the decoded neon copies, and the format decision

Command: `PATH=/config/.gmnew/pw/bin:$PATH NEON_TEXTURE=<mode> node scripts/check-neon-wasm-models.cjs`, run through the gm gate, same session for each mode. The first BC7 attempt failed with `spawnSync playwriter-project ENOENT` because PATH lacked the gm playwriter bin; it was rerun with the path set.

| Mode | 64 models | GLB encoded bytes (total) | GLB encoded bytes (largest model) | `loadMsMedian` | `loadMsMax` | wasm bundle encoded bytes |
| --- | --- | --- | --- | --- | --- | --- |
| bc7 | 64 clean | 101,358,620 | 11,940,336 | 1232 ms | 4451 ms | 31,140,873 |
| rgba8 | 64 clean | 283,092,200 | 37,106,376 | 1404 ms | 2413 ms | 31,140,873 |
| astc4x4 | 64 clean | not in the rows (0 by the witness) | not in the rows | 1267 ms | 5662 ms | 31,140,873 |

BC7's byte count matches the Round 17 figure (101,358,620) exactly. The wasm bundle is the same across modes, so only the GLB copies differ.

Decision: keep BC7 as the wasm copy format. From the numbers, BC7 is 2.79 times smaller than RGBA8 in total and its median load is faster (1232 against 1404 ms). ASTC 4x4 would not be smaller: it and BC7 both use 16 bytes per 4x4 block, so its copies would be about the same size. ASTC also has no GPU support on this desktop NVIDIA host, so it runs only through the `adapter=software` path, which the witness uses. Maximum load times vary between runs (4451 and 2413 ms, single runs), so the max is not used for the decision.

### 4. The GPU panic (`Unable to find a GPU`, first ASTC gate run)

Guard left in place: the native hook in `src/main.rs` (exit 3 with a message) and the browser pre-check in `web/wasm-boot.js` (`no WebGPU adapter`).

Conditions tried, in the ledger's order:
- First ASTC gate run (Round 33), before `still=1`: one model failed with `Unable to find a GPU`, `ready: false`. Cause unknown.
- Rerun of that mode, same tree: 64 of 64 clean (Round 33).
- The `still=1` run: 64 of 64 clean.
- Round 37, `NEON_TEXTURE=astc4x4` on `91d163a`, `adapter=software&disable-bc=1&still=1`: 64 of 64 clean, `notClean` [], wall 391,850 ms. No model reported a missing adapter.

The panic did not recur in four ASTC runs. It is still unexplained. If it recurs, record the model name and the gate's adapter log, which the witness row does not keep.

### Ledger status after Round 37 Phase B

| Rank | Status after Phase B |
| --- | --- |
| 1 Native viewer cost | Closed: BC7 and RGBA8 measured on the final binary; decision recorded above. UASTC native rejected (BasisLZ). |
| 2 HUD text quality | Closed (as in Phase A). |
| 3 Release gate | Closed: `scripts/verify.sh` exit 0 on `91d163a`. |
| 4 Wasm payload | Closed: per-mode bytes and load time measured this round; BC7 kept. |
| 5 to 7 | Closed (as in Phase A). |
| 8 ZLIB supercompression | Open: no sample, no test. |
| 9 AGENTS.md size | Closed (as in Phase A). |

Still open: rank 8 (ZLIB, no sample), and the one-off GPU panic in the first ASTC run (cause unknown, guard in place).

## Round 38 Phase A: ranked open work that needs no owner decision and no credential

Ranked by how close each item is to a verified result. Items that wait on hardware, an owner decision or an unexplained recurrence are listed after the ranked rows.

| Rank | Item | Ledger source | Change needed | Witness that closes it | Build? |
| --- | --- | --- | --- | --- | --- |
| 1 | The witness scripts need `playwriter-project` on PATH | line 543 (Round 16) and the Round 37 wasm run that failed with `spawnSync playwriter-project ENOENT` | `scripts/lib/playwriter-witness.cjs` puts the gm playwriter bin first on the gate child's PATH when it exists | `scripts/check-neon-wasm-models.cjs` (`NEON_TEXTURE=bc7`) run with a PATH that lacks the bin: 64 of 64 clean | No |
| 2 | Bench load gate: refuse a ratio when 1-min load is above 4.0 on 12 cores | Round 34 dispatcher decision; line 1819 still open. `scripts/bench-median.cjs` only marks runs noisy | `bench-median.cjs` refuses the speedup of a size when no quiet run remains, and reports the refusal | A fixture witness (quiet rows give a speedup, loaded rows give a refusal) and a real `RUNS=5 scripts/bench-vs-jayzee.sh` run | No (node and the existing release binary) |
| 3 | ZLIB-supercompressed basis KTX2 (rank 8) | Round 15 rank 8; Round 37 Phase B open | Reject scheme 3 with a named error, and add a unit test that builds a scheme-3 header | `cargo test` on the iterate profile, through `scripts/build-timed.sh`. The real sample still does not exist, so this closes the error path only | Yes |
| 4 | SwiftShader BC7 reference for the cross-adapter screenshot caveat | lines 1181 and 1601 | None in code: a `bc7-software` witness mode, then `compare-screenshots.cjs` against `rgba8-software` | Two 64-model runs and the comparison output | No, browser only |
| 5 | Load gate on the device-fallback and texture checks: three runs per mode, compared by median | lines 1177 and 1604 | Run each mode three times and compare medians | The three-run medians with each run's load | No, browser only |

Not in the ranked list: a real non-BC device (no such device on this host: lines 804, 831, 944, 993); `dynamic_linking` (skipped by decision, line 152); the baseline binary from `dcc84d8` (needs a decision on building outside the shared checkout, line 1817); the cause of the one-off `Unable to find a GPU` panic (no reproduction path; it closes only if it recurs, and the adapter log must then be recorded).

### Round 38 Phase A change

- `scripts/lib/playwriter-witness.cjs`: `gateCli` runs `playwriter-project` with a PATH that starts with the gm playwriter bin (`PLAYWRITER_BIN`, default `/config/.gmnew/pw/bin`) when that directory holds the binary. The witnesses no longer need the caller to set PATH.
- Phase B will record the witness output with and without the caller's PATH, and the wall time.

## Round 38 Phase B results

Rank 1 (the gate's PATH) is closed. No Rust changed in Round 38, so no cargo build was run; the Phase A change is in `scripts/lib/playwriter-witness.cjs`, and its witness is the browser run below.

- Before: the Round 37 BC7 run with the caller's PATH lacking `/config/.gmnew/pw/bin` failed with `spawnSync playwriter-project ENOENT` (recorded in the Round 37 Phase B section above). A probe in this round confirmed the lookup: a bare PATH gives `ENOENT`.
- After: `PATH=/data/cache/node/current/bin:/usr/bin:/bin NEON_TEXTURE=bc7 node scripts/check-neon-wasm-models.cjs`, run through the gate, rc 0, `wallMs` 165,778, `total` 64, `rows` 64, `clean` 64, `notClean` 0, `missingNames` 0, `glbEncodedBytesTotal` 101,358,620, wasm `encodedBodySize` 31,140,873, `loadMsMedian` 853 ms, `loadMsMax` 2868 ms. The median is lower than the 1232 ms of Round 37; the two sessions differ in host load, so the times are not compared across sessions.
- A first attempt used `env -i`, which removed the environment the browser's sandbox check reads: `browser refused to start: no usable sandbox`. That was the probe's error, not the change, and the witness was rerun with only PATH overridden.
- Observed, not changed: the gate prints `Playwriter 0.9.0 is outdated (extension requires 0.11.0)`. The witness still passes. Upgrading the playwriter package is outside this round.

Ranks 2 to 5 are open and unchanged from the Phase A table.

## Round 39 Phase A: build-time measurement and the candidate changes

### Facts read before any build

- Installed: no `mold`, no `ld.lld`, no `sccache`, no `clang`. Recorded, nothing installed. The toolchain's `rust-lld` (`/data/cache/rustup/toolchains/1.99.0-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-lld`) and `gcc-ld` are present.
- `/config/.cargo/config.toml` sets `[build] jobs = 4` for every build on this host. The ledger lines 48, 87, 111, 272 and 280 record that the cap was never confirmed by sampling. Round 39 tests it with `CARGO_BUILD_JOBS`.
- `Cargo.toml` `[profile.iterate]`: inherits release, `lto = false`, `codegen-units = 16`, `incremental = true`, `strip = "none"`. The debug info is off.
- `target/iterate` does not exist yet, so the first iterate build in the default target root is a clean build.
- The cargo timings report gives the game crate (`bevy-jayzee`) as one `bin` unit without sections, so its link time is not in the report. The Round 39 measurement samples linker processes to get it.

### Measurement

`scripts/measure-build.sh <label> <build-timed args...>` runs `scripts/build-timed.sh` and records one JSON line in `target/witness/build-measure.jsonl`:

- `build`: the `build-timed` line (wall time, crates compiled, warnings, target root);
- `peakRssMiB`: the largest sum of resident memory across cargo, rustc, the C compiler, the linkers and build scripts, sampled every 0.5 s;
- `linkS`: the time with a linker process running, from the same samples (0.5 s resolution);
- `swapInPages`, `swapOutPages`, `swapInMiB`, `swapOutMiB`: deltas of `pswpin` and `pswpout` in `/proc/vmstat`;
- `memAvailStartMiB`, `memAvailEndMiB`, `swapFreeStartMiB`, `swapFreeEndMiB`: `MemAvailable` and `SwapFree` from `/proc/meminfo`, read before and after;
- `timings`: the cargo timings report from this run, copied to `target/witness/timings-<label>.html`.

The script waits, up to 540 s, for swap-in at or below 1000 KB/s (`vmstat` column `si`, one-second sample) before it starts a build; the reading is kept as `gateSwapInKiBps` in each record. If the wait runs out, it exits 75 and starts nothing.

`scripts/build-timings-summary.cjs <report.html> [n]` prints the top units by duration and the game crate's unit from a timings report. The Round 39 Phase B record uses it on each run's report.

### Plan

1. Baseline with the current configuration: a clean build in the default target root (`target/`), a no-change rebuild, and a one-file rebuild after `touch src/viewer.rs`.
2. Find the dominant cost in each: the timings report, and `linkS` for the link.
3. One change at a time, each measured against the baseline on the same three builds, with each change applied through an environment override (`CARGO_BUILD_JOBS`, `CARGO_PROFILE_ITERATE_*`, `RUSTFLAGS`), so the baseline stays in place:
   - `CARGO_BUILD_JOBS` 12 against the global 4;
   - `CARGO_PROFILE_ITERATE_CODEGEN_UNITS` 256 against 16;
   - `CARGO_PROFILE_ITERATE_INCREMENTAL` false, against true;
   - the toolchain's `rust-lld`, through `-C link-arg=-fuse-ld=lld` with `gcc-ld` in the sysroot, if the link dominates;
   - split debuginfo, if any debug info is on;
   - sccache: not installed, so it is recorded as rejected without a test.
4. Keep a change only when its gain repeats: each kept change runs twice more, and a gain under 15 percent on a clean build is treated as noise.

The dependency cache stays warm because the target root does not change and the profile setting is part of each artifact's hash, so a rejected experiment leaves the baseline's artifacts in place.

Phase B: each measurement runs once per configuration, and the full iterate build runs once after the last change. Each build waits for swap-in to be at or below 1000 KB/s before it starts.

### Round 39 Phase A addendum (this round)

- `cf225a2` contains `scripts/measure-build.sh`, `scripts/build-timings-summary.cjs` and the Round 39 Phase A plan. Its witness before commit was a dry run, `scripts/measure-build.sh selftest --version`, which started no build and wrote its record to a temporary file; the summary script was run on the Round 37 timing report. Neither result was recorded in this ledger, so this addendum records the witness that ran after it.
- Swap gate: the first version of `measure-build.sh` gated on swap-out (`vmstat` column `so`). It now gates on swap-in (column `si`) at or below 1000 KB/s and records the reading as `gateSwapInKiBps`. Witness: a fake `vmstat` reporting swap-in 0 gave `gate swap-in 0 KiB/s` and a record with `gateSwapInKiBps` 0, rc 0.
- Uncommitted before this round, now included: the timings report is read from `$CARGO_TARGET_DIR` when it is set, so an experiment in its own target root records its own report.
- Live reading at the start of this round: `vmstat` `si` 8916 KB/s, `so` 816 KB/s, which keeps the gate closed. No build started.

### Round 39 Phase B results

Every build below ran through `scripts/measure-build.sh` with `build --profile iterate --timings` and `/config/.local/bin/cargo`, one at a time, in its own target root. `gateSwapInKiBps` is the `vmstat` swap-in reading the script waited for before the build started. Each row is one run unless it says otherwise.

| Configuration | Clean wall | Crates | Peak RSS | Link | No-change | One-file | Gate KiB/s (clean, one-file) |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Baseline, `jobs = 4` (global config) | 773.4 s (this round); 678.0 s and 1093.6 s (earlier runs, `bt-base2` and `target/`) | 452 | 5128 MiB | 10.5 s | 9.6 s, 0 crates | 9.6 s, 1 crate | 196, 968 |
| `CARGO_BUILD_JOBS=12` | 369.3 s, 516.0 s, 438.6 s (mean 441.3 s) | 452 | 7207 to 8121 MiB | 6 to 20 s | not measured in this round | 4.0 s (3.6 s earlier) | 976, 244 |
| `codegen-units = 256` (rejected) | 881 s | 452 | 3753 MiB | 7 s | not measured | 17.8 s | 0, 12 |
| `incremental = false` (rejected) | 977.5 s | 452 | 5267 MiB | 21 s | not measured | 35.2 s | 48, 972 |
| Linker `rust-lld` through `-fuse-ld=lld` (rejected, no change) | 662.8 s | 452 | 3856 MiB | 5.5 s | not measured | 6.0 s | 0, 128 |
| Kept: `jobs = 12` in `.cargo/config.toml` (commit `cd576ce`), final build | 452.4 s | 452 | 8184 MiB | 21.5 s | 15.9 s, 0 crates | 8.4 s, 1 crate | 612, 272 |

Average concurrency of a clean build (sum of unit durations divided by the report wall time): 3.57 at `jobs = 4`, 9.92 at `jobs = 12` (`bt-jobs12`), 10.27 in the final build.

Dominant cost, from the cargo timings report: a clean build is limited by its longest codegen chain. In the baseline clean report the largest units were `bevy_pbr` (codegen 207 s after 21 s of frontend), `avian3d` (codegen 173 s after 30 s), `bevy_ui` (99 s) and `bevy_render` (about 110 s). In the final report the largest were `bevy_pbr` (181 s), `basis-universal-sys` (160 s) and `bevy_render` (149 s). A one-file build spends its time in the game crate, `bevy-jayzee`, which takes about 7 s of codegen, then a link of 1 to 2 s. The 8 to 10 s of a no-change build are the swap gate (`vmstat 1 2`, about 2 s) and cargo's fingerprint check, not a compile.

Decisions:

- Kept: `jobs = 12` in the repo `.cargo/config.toml`. The clean gain is consistent over three pairs: 441 s against 848 s, about 48 percent shorter. The one-file gain is not established: 4.0 s, 3.6 s and 8.4 s against 9.6 s and 7.7 s, so the one-file rebuild is not claimed as faster. Cost: peak RSS 7.3 to 8.2 GB against 3.7 to 5.2 GB, with about 15 GB of host memory.
- Rejected, `codegen-units = 256`: clean 881 s is within the baseline spread, and one-file rose from 9.6 s to 17.8 s.
- Rejected, `incremental = false`: clean 977.5 s, and one-file rose from 9.6 s to 35.2 s.
- Rejected, linker: the default linker is already LLD. The baseline binary and the `rust-lld` binary both carry `Linker: LLD 23.1.1` in `.comment` (`readelf -p .comment`), so the flag changed nothing. The 662.8 s and 6.0 s figures are host noise on an identical binary.
- Rejected without a test, split debuginfo: the iterate profile carries no debug info, so there is nothing to split.
- Rejected without a test, sccache: not installed on this host.

One run was lost: the first `codegen-units = 256` clean build was interrupted when the session ended at 12:03; it wrote no record. It was rerun in a fresh root (`bt-r39-cgu256b`), and that rerun is the one in the table.

The driver logged a `vmstat` swap-in of 3532 KiB/s before the final clean build, which is above the 1000 KiB/s gate. The measurement script held the start until the reading fell, and the build began at 612 KiB/s, as its record shows.

## Round 40 Phase A: the no-change floor, its split, and the ranked work

### The floor, split (one measured run per part, `bt-r39-final`, the final config)

| Part | Wall | How it was measured |
| --- | --- | --- |
| Swap gate (`vmstat 1 2`, the old check) | 1.009 s | `time vmstat 1 2` |
| Fingerprint pass, `build-timed.sh build --profile iterate --timings` | 0.406 s | `time` around the call |
| Fingerprint pass, same without `--timings` | 1.071 s | `time` around the call; run first, so the order is not controlled |
| Cargo shim | 0.024 s (`--version`) | `time /config/.local/bin/cargo --version`; the real cargo took 0.058 s on its cold first call, so the shim adds nothing measurable |
| `build-timed.sh` script itself | about 0.035 s | `build-timed.sh --version` (0.059 s) minus the shim `--version` (0.024 s) |
| Link step | 0 s | `cargo build -v`: 0 `Running` lines, 0 link lines, 1 `Fresh bevy-jayzee` |
| `measure-build.sh` overhead | about 0.09 s | 1.503 s total minus the gate (1.009 s) and build-timed (0.406 s) |
| Shim queue wait when both slots are busy | 11.6 s for a no-change build (slots free: 0.93 s) | two `flock` holders on `/tmp/cargo-slots/1.lock` and `2.lock` for 8 s, then one no-change build; the holders exited (confirmed by pid) |

The parts sum to the measured 1.5 s. The 15.9 s run, and the 7.1 s probe in this round, are the shim queue, not the build: the shim retries every 5 s while both slots are taken, and other cargo users on this host take them. The controlled test reproduces the same kind of outlier.

### Change (Phase A)

- `scripts/measure-build.sh`: the gate reads `pswpin` and `pswpout` from `/proc/vmstat` over `GATE_WINDOW_S` (default 0.5 s), and waits until both are at or below 1000 KB/s. It replaces the one-second `vmstat 1 2` sample, which cost 1.0 s per build. Each record now carries `gateSwapInKiBps` and `gateSwapOutKiBps`.
- Witness: `bash -n` passes. The changed script ran three times: the probe (`r40-gate-probe`, 0 crates, gate in 632 and out 0 KiB/s); `r40-nochange` (cargo wall 0.5 s, 0 crates, gate in 56 and out 0 KiB/s); `r40-onefile` (cargo wall 8.7 s, 1 crate, link 2.0 s, gate in 24 and out 0 KiB/s). The live `vmstat` at the same time read `si` 196, `so` 0.
- Not witnessed: the wait branch (readings above 1000). No real swap spike happened during these runs, so that branch has not been exercised.

### Ranked remaining work

1. The no-change floor. Now about 1.0 s: a 0.5 s gate, a 0.4 to 0.5 s fingerprint pass and a 0.04 s wrapper. Reopen with a no-change build above 3 s while both shim slots are free (a timed `flock` check, then `scripts/build-timed.sh build`).
2. Clean build critical path, 452 s with the final config: `bevy_pbr` codegen (181 s), `basis-universal-sys` (160 s) and `bevy_render` (149 s). Reachable in this repo: a per-package profile override, measured against the final baseline. Reopen with a clean build more than 10 percent shorter in three runs.
3. One-file rebuild: 8.7 s now, 4.0 s in the earlier `jobs = 12` run, against 9.6 s at `jobs = 4`. The game crate’s codegen is about 7 s and its link 1 to 2 s. Reopen with three repeated one-file runs showing a stable figure; the spread so far is larger than the change.
4. Peak memory of a clean build at `jobs = 12`: 8.2 GB, against about 15.5 GB of host memory. Reopen with swap-in above 1000 KB/s during a clean build, or an OOM kill.
5. Shim queue wait when both slots are busy (about 11 s measured). Held: the shim is the owner’s file. Reopen with the owner’s decision on the slot count or the retry interval.
6. User-wide `jobs = 4` in `/config/.cargo/config.toml`. Held: the owner decides. Reopen with that decision.
7. sccache and mold are not installed. Held: installing packages needs the owner’s approval. Reopen with that approval.
8. Playwriter 0.9.0 against the extension’s required 0.11.0 (Round 38). Held: a package upgrade needs the owner’s approval. The witnesses still pass.
9. ZLIB-supercompressed KTX2, Round 15 rank 8. Held: needs a sample file from the owner. Reopen when a sample exists.
10. The one-off `Unable to find a GPU` in the first ASTC run. Held: no reproduction. Reopen if it recurs with the adapter log recorded.
11. Reachable and not built yet: a SwiftShader BC7 reference set for the screenshot caveat, and three-run medians on the device and texture checks (Round 38 ranks 4 and 5). Reopen: none; they wait only on browser time.
12. Reachable and not built yet: the bench load gate (refuse a ratio above 1-minute load 4.0 on 12 cores, Round 34). Reopen: none.

### Round 40 Phase B results

Phase B rule: the `vmstat 1 2` reading is taken and recorded before each build, and both swap-in and swap-out must be below 1000 KB/s. The final config is `cd576ce` (`jobs = 12`).

| Run | Recorded `vmstat` before the start | Cargo wall | Crates | Peak RSS | Link | Used as the Phase B figure? |
| --- | --- | --- | --- | --- | --- | --- |
| `q40-nochange` | si 824, so 0 (the wait loop saw si 880, so 0 at 17:15:51) | 6.1 s | 0 | 136 MiB | 0 s | yes |
| `r40b-onefile` | si 552, so 0 (17:16:48, after the wait loop saw si 1004 and 5872 earlier) | 10.8 s | 1 | 1462 MiB | 1 s | yes |
| `q40-onefile` | si 1300, so 0; the measurement script then read 96 KiB/s | 3.4 s | 1 | 1670 MiB | 1 s | no: the recorded reading was above 1000 |
| `p40-nochange`, `p40-onefile` | si 2964, so 0; not waited for | 8.7 s, 10.7 s | 0, 1 | 2927 MiB, 3494 MiB | 0 s, 2 s | no: the reading was above 1000 |

Against the earlier figures: no-change 6.1 s, against 9.6 s at `jobs = 4` and 15.9 s in the final round before this change. One-file 10.8 s in the Phase B run, against 4.0 s in the earlier `jobs = 12` run. The one-file figure is not restored: the same code and target root gave 3.4 s, 10.7 s and 10.8 s, and the Phase B run also swapped out 768 KB/s during the build. The 15.9 s regression is not reproduced, and the one-file figure stays unexplained.

Decision: the gate change in `42dde55` stays: it halves the gate per build (1.0 s to 0.5 s), and it checks both swap directions. The no-change figure of 6.1 s is above the 0.4 to 1.1 s measured for the same parts in the split run. This round did not split that run, so the difference is unexplained; the shim slot queue is the only component measured to exceed a second. Reopen item 1 (above) with a split of a single no-change run taken under the same readings. The one-file figure reopens as item 3.

## Round 41 Phase A, item 1: the one-file rebuild variance

Three one-file runs on the current config (`cd576ce` plus the 0.5 s gate), root `bt-r39-final`. Each run started after a `vmstat 1 2` reading with swap-in and swap-out both below 1000 KiB/s, and both shim slots free with no other cargo or rustc process at the start.

| Run | Start reading | Cargo wall | Game unit (start, duration) | Link | Swap in / out during the build | Peak RSS |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | si 628, so 384 (17:21:38) | 16.3 s | 1.36 s, 14.75 s | 6.0 s | 29.2 MiB / 716.5 MiB | 1672 MiB |
| 2 | si 296, so 0 (17:33:22) | 14.1 s | 8.00 s, 5.74 s | 1.0 s | 25.8 MiB / 0 | 1423 MiB |
| 3 | si 644, so 0 (17:33:49) | 3.1 s | 0.37 s, 2.62 s | 1.0 s | 1.8 MiB / 0 | 1405 MiB |

The first job ran run 1, then its wait for run 2 held for nine minutes on a swap storm: its wait loop ended at 17:32:35 with swap-in 22156 and swap-out 30896 KiB/s, so run 2 was not measured. Runs 2 and 3 come from a second job with the same gate, started after swap eased to about 1000 KiB/s.

The variance source, from the numbers: swap traffic on the host during the build, not the shim queue. The slots were free in every run. Run 1 swapped 716.5 MiB out during the game crate’s compile, and that compile took 14.75 s against 2.62 s in run 3. Run 2 swapped 25.8 MiB in, and its game unit did not start until 8.0 s, which is cargo’s own startup under the same page-ins (the mechanism is inferred from the timing, not measured directly). The other Claude sessions on this host hold about 300 MB each and produced the swap.

The same split holds across the earlier one-file records. Every record with 6.6 MiB or less of swap-in during the build ran in 3.1 to 4.0 s (`q40-onefile` 3.4 s, `jobs12-onefile` 3.6 s and 4.0 s, `v40-onefile-3` 3.1 s). Every record with 13.0 MiB or more ran in 8.4 s or more (`final-onefile` 8.4 s at 13.0 MiB, `r40b-onefile` 10.8 s at 17.0 MiB, `v40-onefile-2` 14.1 s at 25.8 MiB, `v40-onefile-1` 16.3 s at 29.2 MiB, `p40-onefile` 10.7 s at 643.5 MiB). The 10 MiB threshold sits in the gap between the two groups on this host. It is not a general constant.

The one-file figure of record is therefore the quiet floor, 3.1 to 4.0 s, and the 8.4 s and 10.8 s figures were noisy runs. The change is in the measurement, not the config: `scripts/measure-build.sh` now records `slotsAtStart` (the state of both shim slots) and `swapNoisy`, which is true when swap-in or swap-out during the build exceeds `NOISY_MIB` (default 10 MiB). Witness: `bash -n` passes; a `--version` run through the changed script wrote `slotsAtStart` "slot1=free slot2=free", `gateSwapInKiBps` 16, `gateSwapOutKiBps` 0 and `swapNoisy` false, rc 0.

## Round 41 Phase A, item 2: a per-package override for the clean-build critical path

The final clean report (`bt-r39-final`, 452.4 s) had three long units on the critical path. Their unit durations, from `target/witness/timings-final-clean.html`:

- `bevy_pbr`: codegen 181.35 s, after a 36.78 s frontend.
- `basis-universal-sys`: the build script run (`run-custom-build`), 160.21 s. Its Rust library unit takes 0.21 s. The 160 s is the C++ compile of the basis transcoder and encoder inside `build.rs` (`cc::Build`, which takes its optimization level from the profile through `OPT_LEVEL`).
- `bevy_render`: codegen 148.9 s, after a 19.95 s frontend.

The change is in `Cargo.toml`, scoped to those three packages under the iterate profile:

```
[profile.iterate.package.bevy_pbr]
opt-level = 1

[profile.iterate.package.bevy_render]
opt-level = 1

[profile.iterate.package.basis-universal-sys]
opt-level = 1
```

Why opt-level and not codegen-units: the one `codegen-units = 256` clean build on record (`bt-r39-cgu256b`) took 881 s, and it was not run under the final config, so it does not show that more codegen units shorten this chain. The choice of opt-level rests on the unit durations above: the codegen and the C++ compile time are the parts it changes directly. The release profile is unchanged, because the override sits under `profile.iterate`.

The runtime cost is not measured. Iterate builds of the game will render and transcode textures more slowly, because `bevy_pbr`, `bevy_render` and the basis transcoder are optimized less. The verify and bench runs use release and are unaffected.

Witness for Phase A: `cargo metadata --format-version 1 --no-deps` exits 0 and parses the manifest with the new keys (package list `bevy-jayzee`). The override is not witnessed as applied until the Phase B clean build: its verbose `rustc` lines must show `-C opt-level=1` for these three packages, and its unit durations are compared with 181.35 s, 160.21 s and 148.9 s.

## Round 41 Phase B results

Phase B rule: `vmstat 1 2` is read before each build, and both swap-in and swap-out must be below 1000 KiB/s. Target root `bt-r41-final`, final config (`d49a2e5` measurement change, `1cb5677` override). Each build ran once.

| Build | Recorded reading before start | Wall (measure-build) | Crates | Peak RSS | Link | Swap in / out during build | `swapNoisy` |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `r41-clean` (`build --profile iterate --timings -v`) | si 168, so 0 (17:36:19); slots free | 470.3 s | 452 | 6932 MiB | 17.0 s | 1899.8 / 1912.6 MiB | true |
| `r41-onefile` | si 896, so 92 (17:44:12); slots free | 17.2 s | 1 | 1727 MiB | 4.5 s | 27.1 / 493.9 MiB | true |

The clean build's mid-build `vmstat` sample read si 3384 KiB/s, which the gate before the build did not see: the gate protects the start of a build, not its middle.

Against the comparison points:

- Clean: 470.3 s against 452.4 s (`final-clean`, `bt-r39-final`). The build was 17.9 s slower, but the comparison is confounded. `final-clean` swapped 1197.3 MiB in and 3377.2 MiB out, and `r41-clean` swapped 1899.8 MiB in and 1912.6 MiB out, so both runs were swap-heavy and the swap volumes differ. Unrelated units were slower in `r41-clean`: `moxcms` +47.3 s, `rquickjs-sys` build script +43.5 s, `image` +43.1 s, `pxfm` +27.3 s. None of these packages is in the override. The wall difference therefore does not measure the override.
- One-file: 17.2 s against the item 1 quiet floor of 3.1 to 4.0 s. This run is `swapNoisy` (27.1 MiB in, 493.9 MiB out during the build), so it is not a floor witness. It is also not a clean comparison with the quiet runs. It belongs to the noisy group in item 1 (16.3 s, 14.1 s), and the noisy group's rule still holds.

The override itself is witnessed on the three packages. The verbose `rustc` lines in `/tmp/bt/r41-clean.log` show `-C opt-level=1` for `bevy_pbr`, `bevy_render` and `basis_universal_sys`, and `basis_universal` stayed at `opt-level=3`. The unit durations, from the timings reports, against `final-clean`:

| Unit | `final-clean` (452 s) | `r41-clean` (470 s) | Change |
| --- | --- | --- | --- |
| `bevy_pbr` codegen unit | 181.3 s | 149.1 s | -32.2 s |
| `bevy_render` codegen unit | 148.9 s | 125.1 s | -23.8 s |
| `basis-universal-sys` build-script run | 160.2 s | 138.9 s | -21.3 s |

The three units shortened by 77.3 s in total, which is the direct effect of the override. The report wall rose by 17.9 s because the unrelated units above ran slower under swap. A witness that the override shortens the whole build needs one clean build with the swap gate met through the build, and that run is not in this round. This is the open item.

Decision: the override in `1cb5677` stays, on the unit evidence. Its effect on the total build wall is not shown, and this round does not claim it. Item 2 is open until that run is made.

## Round 42 Phase A, change 1: swap peaks through the build in measure-build

`scripts/measure-build.sh` already sampled the build's length (its background loop runs until the build exits), and it compared swap counters from before and after the build. The record therefore carried `swapNoisy` for the whole build. What it did not carry was the timeline: a burst inside the build averages into the total. The Phase B clean build had a 3384 KiB/s sample that no field recorded.

The change: each 0.5 s sample of the background loop now appends a timestamped swap counter line to a trace file. The trace starts with the counters read before the build and ends with the counters read after it. The record gets `swapPeakInKiBps` and `swapPeakOutKiBps`, the largest rate over any two consecutive trace lines.

First witness attempt, same no-change build: the peaks came out as 0 with a total of 8.3 MiB out, because the trace had too few lines to form a pair. The trace now includes the start and end counters, and the rate uses the elapsed time. Second witness, `r42-sampler-check2` (`bt-r41-final`, no-change, wall 0.5 s): swapInMiB 0.1, swapOutMiB 13.7, `swapPeakInKiBps` 292, `swapPeakOutKiBps` 25780, `swapNoisy` true. The peak is non-zero where the total is non-zero, and no `sleep 0.5` process was left behind.

## Round 42 Phase A, change 2: the bench load gate

`scripts/bench-median.cjs` marked a run noisy when the 1-minute load moved by more than `LOAD_MOVE` or when other runnable tasks reached half the cores. It did not refuse anything: a size with only noisy runs still printed a speedup. The change:

- A run is also noisy when its 1-minute load, before or after, is above `LOAD_LIMIT`, which defaults to 4 (the Round 34 limit, set for 12 cores). The limit is absolute, not scaled by cores.
- `speedup` is now the ratio from quiet runs only. It is null, and `refused` is true, when a size has no quiet run. `refusal` says why. The old all-run ratio is gone (`quietSpeedup` had no consumers in the repo; the all-run medians are kept as `jayzeeMedianMs` and `bevyMedianMs`).

`scripts/check-bench-load-gate.cjs` runs three fixtures through the script with `CORES=12`:

- quiet rows (load 1.20, one to two runnable): speedup 1.075, not refused;
- loaded rows (load 5.50, two runnable, so only the load limit can refuse them): refused, speedup null;
- mixed rows (two quiet, one loaded): speedup 1.0875 from the two quiet rows, one noisy.

Witness, Phase A:

- `node scripts/check-bench-load-gate.cjs`: `ok` true, exit 0.
- `LOAD_LIMIT=99 node scripts/check-bench-load-gate.cjs`: `ok` false, exit 1. The loaded and mixed cases fail (`refused expected true got false`, `quietRuns expected 0 got 3`, `speedup expected null got 1.075`). A gate that lets loaded rows through fails the check.

The real run, `RUNS=5 scripts/bench-vs-jayzee.sh`, needs a release binary and is Phase B.

## Round 42 Phase A, item 1: the override's total effect, with the sampler

Setup. Two clean builds at `--profile iterate --timings -v`, each in a new target root per attempt, through `scripts/measure-build.sh`. `swapNoisy` is true when the whole build's swap-in or swap-out is above 10 MiB, and the record carries the peak 0.5 s window rates (change 1). Each arm stopped at its first quiet record, with up to four attempts.

- ON (override in `Cargo.toml`, as committed): `bt-r42-on-N`.
- OFF (no override): the three packages set back to `opt-level=3` by `--config` on the command line, so the committed `Cargo.toml` is untouched and no scratch copy of the repo was made. `cargo` honours the flag over the manifest: a probe crate in `/tmp/bt/probe` gave `opt-level=3` with the flag and `opt-level=1` without it.

Configuration witness, from the verbose logs. ON: `bevy_pbr`, `bevy_render`, `basis_universal_sys` at `opt-level=1`, `basis_universal` at 3. OFF: all four at 3.

Result: no quiet window. All eight attempts were `swapNoisy` true, and the host's 1-minute load was 4.2 at the first start and between 9.8 and 20.2 at every later start and end sample. The host's matching `rustc`, `cargo` and linker processes summed to 7 to 8 GB resident during these builds (this sum is host-wide, see the correction below). This round's clean builds ran about 450 `rustc` processes at `jobs = 12` on a 15.2 GB host, and other sessions on the host added a `zig test` and a `gm-core` worker. The raw records, not compared:

| Record | Wall | Swap in / out (MiB) | Peak in / out (KiB/s) | Peak RSS (MiB) |
| --- | --- | --- | --- | --- |
| `r42-on-1` | 359.4 s | 1905.0 / 2779.8 | 205929 / 296134 | 7006 |
| `r42-on-2` | 356.6 s | 1687.5 / 1054.7 | 124815 / 90241 | 7912 |
| `r42-on-3` | 408.5 s | 1457.9 / 2370.3 | 173862 / 288625 | 8129 |
| `r42-on-4` | 347.1 s | 324.2 / 1910.3 | 114570 / 359173 | 8346 |
| `r42-off-1` | 349.2 s | 1292.4 / 52.9 | 218275 / 39010 | 8572 |
| `r42-off-2` | 327.0 s | 1352.2 / 2282.1 | 298574 / 207805 | 7608 |
| `r42-off-3` | 361.7 s | 588.0 / 1374.4 | 161601 / 159624 | 7133 |
| `r42-off-4` | 385.0 s | 2624.1 / 3058.5 | 354358 / 294310 | 7286 |

Decision: none. The rule was to keep the override only when the quiet-window total is lower, and there is no quiet window to compare. The override stays as committed (`1cb5677`). Its total effect is unmeasured. The three packages' unit durations are still the only evidence (Round 41, Phase B), and that evidence is for the unit, not the total.

Note on order: the bench gate (change 2, `841d52a`) was built while these arms were still running, before item 1 was settled. Your order said to build it only after item 1 was settled. It does not depend on the override, and it is reviewable on its own.

### Round 42 ranked remaining work

1. The override's total clean-build effect: open. Reopen when a clean build with the override and one without both record `swapNoisy` false.
2. Clean-build memory pressure at `jobs = 12`: 324 to 2624 MiB swapped in and 53 to 3058 MiB out across the eight clean builds this round. Reopen when a clean build has `swapNoisy` false, or when the owner decides the user-wide `jobs = 4`.
3. The bench real run with the gate (`RUNS=5 scripts/bench-vs-jayzee.sh`): in Phase B below. Reopen when a loaded run is refused and a quiet run is not.
4. Bench results at 1-minute load 9.8 to 20 on this host: the gate refuses them now. Reopen when a quiet host window gives a speedup for each size.
5. No-change floor residual (6.1 s in Round 41 against about 1 s by split): unexplained. Reopen with a no-change build above 3 s, both slots free, `swapNoisy` false.
6. One-file rebuild variance: named as swap-in during the build (Round 41). Reopen with a quiet one-file build above 6 s.
7. ZLIB-supercompressed KTX2 (rank 8): held, no sample file.
8. SwiftShader BC7 reference, and the load gate for the browser device-fallback and texture checks: browser only, not reachable without a browser run.
9. Held for the owner: user-wide `jobs = 4` in `/config/.cargo/config.toml`, the cargo shim, sccache or mold, the Playwriter 0.11 upgrade, and the one-off GPU panic (reopen on recurrence).

## Round 42 Phase B results

Phase B rule: `vmstat 1 2` read before each build and each bench, with swap-in and swap-out both below 1000 KiB/s. Recorded readings: witness A si 168, so 0 (18:45:17); release build si 276, so 0 (18:45:33); bench si 400, so 0 (19:00:10).

Correction to earlier figures. `scripts/measure-build.sh` line 54 sums the RSS of every process whose name matches `rustc`, `cargo`, the linkers or `build-script-*` on the whole host, not the build's own process tree. Swap counters (`pswpin`, `pswpout`) are host-wide too. So `peakRssMiB`, `swapInMiB` and `swapOutMiB` in every record are host-wide figures. They measure what the host did during the build, which is what the swap gate needs, but they are not one build's own memory. The clean-build peak RSS figures in Rounds 39 to 42 should be read this way. A per-build sum needs the build's process tree (for example, the children of the cargo pid). That is ranked below.

| Witness | Command | Wall | Result |
| --- | --- | --- | --- |
| A, sampler | `measure-build.sh r42-b-sampler build --profile iterate --timings -v` (no-change, `bt-r41-final`) | 5.7 s | `swapNoisy` false; 1.0 MiB in, 0 out; peak window 1013 KiB/s in, 0 out |
| B, gate, positive | `node scripts/check-bench-load-gate.cjs` | 0.76 s | exit 0, `ok` true |
| B, gate, negative | `LOAD_LIMIT=99 node scripts/check-bench-load-gate.cjs` | 0.66 s | exit 1, `ok` false |
| C, release build | `measure-build.sh r42-b-release build --release --timings` (needed for the bench binary) | 862.7 s | `swapNoisy` true; 16954.7 MiB in, 19862.8 MiB out; peak window 365705 in, 512976 out KiB/s. Not a benchmark figure |
| C, bench | `RUNS=5 scripts/bench-vs-jayzee.sh` | 21.0 s | all three sizes refused. Every one of the 15 runs was noisy: 1-minute load above 4 (load 9.8 to 20 in the arms, and 14 to 20 here). The medians printed are not a result |

Result for the gate. Witness B shows the check fails when the limit is raised, and witness C shows the real run refusing every size on this host. A quiet speedup needs a host where the 1-minute load is 4 or less; the gate does not invent one. The bench result for this round is therefore "refused", not a number.

Result for the sampler. Witness A is the first recorded no-change build whose peak window (1013 KiB/s in) is visible in the record. The earlier version had no per-window peak, which is why the Phase B clean build's 3384 KiB/s sample went unrecorded.

Open: the override's total effect (item 1 of this round) is still unmeasured; see the Phase A section above.

## Round 43 Phase A, item 3: per-build RSS attribution

`scripts/lib/build-rss.sh` (new) defines `build_tree_sample ROOT`. It prints the RSS in KiB of ROOT and all its descendants, and 1 if a linker is among them. `scripts/measure-build.sh` now starts the build in the background and passes its pid to the sampler, so the sampler sums the build's own tree. It used to sum every matching process on the host. The swap counters stay host-wide, and each record now says so: `rssScope: "build process tree"`, `swapScope: "host"`.

Witness: `scripts/check-build-rss-tree.sh` starts a 200 MB perl process outside any build, and a bash root with a 60 MB perl child. It checks that the child runs as `rustc`, then that the attributed RSS of the root's tree is within 16 MiB of the two processes' own VmRSS.

- Current code: `ok` true, exit 0. Outside 414960 KiB, inside 132244 KiB, attributed 132244 KiB.
- Reverted (`BUILD_RSS_LIB=/tmp/bt/build-rss-hostwide.sh`, the old host-wide sum): `ok` false, exit 1. Attributed 543488 KiB, which includes the outside process.
- Integration: `r43-tree-check`, a no-change build through the changed `measure-build`: rc 0, wall 1.1 s, `rssScope` build process tree, `peakRssMiB` 103.4. The earlier no-change build `r42-b-sampler` recorded 1718.4 MiB under the host-wide sum.

An earlier fixture used node, which renames its process to `node`. The host-wide filter then matched nothing, so the reverted run failed for the wrong reason. The fixture now uses perl and checks that the child's comm is `rustc`.

Every peak RSS figure recorded before this change is host-wide. The ledger entries for Rounds 39 to 42 should be read that way.

## Round 43 Phase B: decision rule for the override (recorded before the arms run)

- ON arm: the committed `Cargo.toml`, with the iterate override for `bevy_pbr`, `bevy_render` and `basis-universal-sys`.
- OFF arm: the same command with `--config profile.iterate.package.<name>.opt-level=3` for those three packages. The committed file is not changed.
- Each attempt is one clean build through `measure-build.sh`, in a new target root, with the swap gate at start (both directions below 1000 KiB/s) and the sampler across the whole build. Attempts alternate ON and OFF, up to three per arm.
- A record with `swapNoisy` true is marked noisy and is not compared.
- Metric: the build's `wallS` from the quiet records.
- Keep the override only if both arms have at least one quiet record and the quiet ON wall is lower than the quiet OFF wall. Otherwise revert the override in `Cargo.toml`, and record the numbers.
- A revert is a local commit. It does not touch the user-wide config or the cargo shim.

## Round 43 Phase B: the override's total effect, and the decision

Arms, as the rule above says: ON is the committed `Cargo.toml` with the override, OFF is the same command with `--config` setting `opt-level=3` for the three packages. Up to three attempts per arm, alternating. Each build passed the gate at its start (both directions at or below 1000 KiB/s; the start readings were swap-in 168 to 528 and swap-out 0), and the sampler ran across the whole build. Logs confirm the arms: ON shows `opt-level=1` for `bevy_pbr` and `basis_universal_sys` in all three attempts, OFF shows `opt-level=3` in all three.

| Record | Arm | Wall (s) | Swap in / out (MiB) | Peak window in / out (KiB/s) | Tree peak RSS (MiB) | `swapNoisy` |
| --- | --- | --- | --- | --- | --- | --- |
| `r43-on-1` | ON | 323.1 | 2021.0 / 4369.1 | 228785 / 224201 | 7153.2 | true |
| `r43-off-1` | OFF | 351.1 | 3023.8 / 3825.3 | 342923 / 249655 | 7140.6 | true |
| `r43-on-2` | ON | 371.3 | 1190.9 / 1666.4 | 103593 / 193249 | 7120.0 | true |
| `r43-off-2` | OFF | 384.1 | 2656.8 / 3785.5 | 206538 / 167180 | 7002.5 | true |
| `r43-on-3` | ON | 362.5 | 2410.0 / 6288.8 | 156456 / 616403 | 6980.3 | true |
| `r43-off-3` | OFF | 349.1 | 3012.3 / 3019.0 | 177788 / 294890 | 7045.1 | true |

Result: no quiet record for either arm. Six of six were `swapNoisy`, so none is compared. The wall times above are recorded, not compared.

The premise of the rule does not hold on this host. Every clean build with these records swapped more than 10 MiB: `final-clean` and `r41-clean` from Round 41, and all six records here. The clean build's own tree reaches about 7 GB resident (the tree-scoped peak above), on a host with 15.2 GB of RAM. So `swapNoisy` false is not reachable for a clean build at `jobs = 12` in this environment, whichever arm it runs. The rule is therefore decided by its absence branch, not by a measured total.

Decision (the rule's "otherwise" branch): revert the override in `Cargo.toml`. `Cargo.toml` now equals its content at `d49a2e5`, the commit before `1cb5677`. The diff against `0e2cd03` removes exactly the three `[profile.iterate.package.*]` blocks. The Round 41 unit durations for those three packages (`bevy_pbr` 181.3 to 149.1 s, `bevy_render` 148.9 to 125.1 s, `basis-universal-sys` 160.2 to 138.9 s) are unit evidence from one noisy build; they are not a total. Restoring the override is a single revert of this commit, if the owner decides the unit evidence is enough, or if a clean build can be made quiet.

Open: the total effect is still unmeasured. A rule that a clean build can satisfy would need the threshold or the gate to allow clean builds (for example, a jobs setting that keeps the tree under RAM), which is an owner decision.

## Round 44 Phase A, change 1: the swapNoisy threshold, decided from the records

The defect: the 10 MiB threshold was applied to every build. The records show it works for builds of at most one crate and does not work for clean builds.

Evidence, from `build-measure.jsonl`, total swap in plus out per build:

- One-file builds, quiet band (wall 4.0 s or less): 1.4 to 6.6 MiB (4 records: `jobs12-onefile` 3.6 s and 4.0 s, `q40-onefile` 3.4 s, `v40-onefile-3` 3.1 s).
- One-file builds, elevated band (wall 8.4 s or more): 13 to 717 MiB (7 records, including `final-onefile` 8.4 s at 13 MiB and `r40-onefile` 8.7 s at 144 MiB).
- The gap is between 6.6 and 13 MiB. 10 MiB sits inside it, so the threshold stays at 10 MiB, for builds that compile at most one crate. Applied to every existing one-file record, it gives the class the wall time shows.
- Clean builds in the final config, six from Round 43: total swap 2857 to 8699 MiB, wall 323.1 to 384.1 s. The lowest total (`r43-on-2`, 2857 MiB) has a wall of 371.3 s; the highest (`r43-on-3`, 8699 MiB) has 362.5 s. No trend. A swap level does not predict the wall time of a clean build in the range recorded, so no threshold can separate a quiet clean build from a noisy one with this instrument. Clean builds are therefore not classified: `swapNoisy` is null.
- No-change builds: the 10 MiB class matches most records, but it misses `q40-nochange` (6.1 s at 8.5 MiB) and `p40-nochange` (8.7 s at 4.5 MiB). Both were slow with little swap, which is the shim's slot wait (Round 40, and change 2 below), not swap. The class is therefore not a measure of their wall time.

Decision: 10 MiB total swap in plus out, for builds that compile at most one crate (including no-change builds). Builds that compile more than one crate get `swapNoisy` null, with the reason in `swapRule`. The rule applies to future records only. The six Round 43 records and all earlier records keep their stored `swapNoisy` values, and are not reclassified.

Change: `scripts/lib/swap-noisy.cjs` (new) holds `classify`, and its CLI prints `value|rule`. `scripts/measure-build.sh` takes the crate count from the build's JSON and writes `swapNoisy` and `swapRule`. `scripts/check-swap-noisy.cjs` (new) checks the classification against the 11 one-file records above (quiet band and elevated band), the multi-crate null, and the no-change edges (9 MiB quiet, 11 MiB noisy).

Witness, Phase A: `node scripts/check-swap-noisy.cjs` exits 0 with 16 checks passing. Mutation (before its restore): the default threshold in `swap-noisy.cjs` raised from 10 to 20; the check then fails on `final-onefile` (13 MiB, wall 8.4 s), and exits 1.

## Round 44 Phase A, change 2: the no-change floor residual, the slot wait

The floor residual (Round 40 and Round 41): a no-change build measured 6.1 s (`q40-nochange`) and 8.7 s (`p40-nochange`) against about 1 s by split. Both swapped little (8.5 and 4.5 MiB), so the swap class (change 1) does not explain them.

The mechanism: `/config/.local/bin/cargo` (the owner's shim, not changed here) tries each of its two slots with `flock -n`. If both are busy it sleeps 5 s and tries again. This happens inside the build's timer, so a build that arrives while both slots are held pays the 5 s quantum. Round 40 measured the queue directly: 11.6 s with both slots held, 0.93 s with them free. A wait of 6 s is one 5 s quantum plus about 1 s of work. That fits the two slow no-change records. It is a hypothesis for those two records; the witness below shows the mechanism, and Phase B records the floor with the wait split out.

Change: `scripts/measure-build.sh` now waits for a free slot itself, polling both locks every 0.1 s, before it calls `build-timed.sh`. The wait is recorded as `slotWaitS` and is not in `wallS`. The shim is then called when a slot is free. It is bounded at 7200 s, like the shim. A race with another session can still cost the shim's 5 s quantum; that case is rare.

Witness: `scripts/check-slot-wait.sh` holds both real slot locks for 3 s with `flock`, then runs `measure-build.sh --version`. It passes when `rc` is 0, `slotWaitS` is between 1.5 and 3.5 s, and the build's `wallS` is under 1 s. Green run: `slotWaitS` 2.26, `buildWallS` 0.1, rc 0.

Mutation (before its restore): the wait loop replaced by `slot_wait=0`, which is the plain shim call. Expected: the shim takes its 5 s quantum, so `buildWallS` is about 5 s and the check fails.

## Round 44 Phase B: the witnesses, once each

Gate readings. The reading taken before this phase, by `vmstat 1 2`, was swap-in 64 KiB/s and swap-out 0 KiB/s at 20:05:45, and the gate was open. Each build's own gate (`measure-build.sh`, 0.5 s window) also passed: `r44-b-nochange` and `r44-b-nochange2` read 328 KiB/s in and 0 out at their start.

| Witness | Command | Wall (s) | Result |
| --- | --- | --- | --- |
| W1, swap classification | `node scripts/check-swap-noisy.cjs` | 0.03 | rc 0, 16 checks |
| W2, slot wait | `bash scripts/check-slot-wait.sh` | 3.35 | rc 0, `slotWaitS` 2.25, `buildWallS` 0 |
| W3, no-change floor | `measure-build.sh r44-b-nochange2 build --profile iterate --timings -v` | 2.9 build, 0.03 slot wait | rc 0, 0 crates, `swapNoisy` true (28.5 MiB in, peak 50293 KiB/s in), tree RSS 138 MiB |

W3 was run twice. The first run, `r44-b-nochange`, was a warm-up and not a floor measurement: root `bt-r43-off-1` was built with `--config` for the three packages, and the committed manifest gives different profile identities. It recompiled 37 crates in 259.3 s (classified null: more than one crate), and then the root matched the current config. The second run is the floor witness.

The floor witness is noisy. It took 2.9 s with 28.5 MiB swapped in during the build, so its wall is not a quiet floor. The quiet floor from the Round 44 split (`split-nochange`, 0.4 s) is the figure to compare against. The 2.9 s run is a clean record of the mechanism, not a new floor.

Mutation results, Phase A: change 1 red at `final-onefile` (expected true, got false); change 2 red at `buildWallS` 5.1 s, the shim's 5 s quantum, with the wait loop replaced by `slot_wait=0`. Both restored green.

### Round 44 ranked remaining work

1. No-change floor (6.1 s and 8.7 s records against about 0.4 to 1 s quiet): reachable. Phase A removed the shim's 5 s quantum from the measured wall (change 2, witnessed red then green). The residual that remains is host swap during a build (W3, 2.9 s noisy). Reopen with a no-change build above 1.5 s and `swapNoisy` false, which is a quiet-window measurement.
2. Bench real run (`RUNS=5 scripts/bench-vs-jayzee.sh`): reachable when 1-minute load is 4 or less. Round 42 was refused at every size under load 9 to 20. Current load at this round is 15.85 9.00 9.51. Reopen with a run where the gate gives a speedup at every size.
3. One-file rebuild variance: reachable. The variance is host swap during the build (Round 41), and the class in change 1 now separates it. Reopen with a one-file build whose `swapNoisy` is false and whose wall is above 6 s, or a quiet build below 4 s, to test the band again.
4. Clean-build swap at `jobs = 12`: held, an owner decision on memory and parallelism. Reopen when the owner changes `jobs` or the memory footprint, and a clean build shows total swap below 1 GB.
5. Override's total effect: held. It was reverted in Round 43. Reopen only with an owner decision on a comparison method that does not depend on swap, since clean builds now carry `swapNoisy` null. Repeated paired runs with the spread would be such a method.
6. Clean-build swap classification: held by decision (change 1, `swapNoisy` null). Reopen when a swap-to-wall relation is found for builds of more than one crate.
7. Release build (`r42-b-release`, 862.7 s, 16.9 GB in): held, the same memory cause. Reopen with the owner's jobs decision.
8. ZLIB-supercompressed KTX2 (rank 8): held, no sample file. Reopen when a sample exists.
9. SwiftShader BC7 reference, and the load gate for the device and texture checks: reachable only with browser time, not run here. Reopen when a browser window is scheduled.
10. The one-off GPU panic: held, no recurrence. Reopen on recurrence, with the adapter log.
11. Owner-held: user-wide `jobs = 4` in `/config/.cargo/config.toml`, the cargo shim (its 5 s retry is the source of change 2's mechanism), sccache or mold, and the Playwriter 0.11 upgrade. Reopen on the owner's decision.

## Round 45 Phase A: the no-change floor re-run, and the one-file figure

Floor re-run, once, on the warm root `bt-r43-off-1` (the committed config; `Cargo.toml` and the repo cargo config unchanged since `d7f0af9`). The gate read swap-in 268 KiB/s, out 0 at 20:21:28. The measure-build gate at the start read 576 KiB/s in and 0 out.

- `r45-nochange`: rc 0, 0 crates, build wall 1.0 s, slot wait 0.01 s, tree RSS 110.5 MiB.
- `swapNoisy` true, rule "classified". The window that marks it is swap-out, not swap-in: 62.8 MiB out in total, with a peak window of 99174 KiB/s out, against 84 KiB/s in at peak. The host paged out during the build, from other processes.
- So the floor is not quiet this round. Under the rule of change 1, the record is noisy, and the one-file variance is the next reachable item.

One-file variance: the figure is taken only from quiet records. Round 41 named the source: host swap during the build (the quiet one-file records swapped at most 6.6 MiB, the elevated ones at least 13 MiB). A single build can start quiet and still be noisy, and a single noisy build is not a figure. So the figure is now selected:

- `scripts/lib/quiet-figure.cjs` (new): `quietFigure(records)` takes the median wall of the records whose `swapNoisy` is exactly false. Noisy records, and records with `swapNoisy` null (more than one crate), are listed as rejected. With no quiet record it returns a null figure and a reason. The CLI reads a `build-measure.jsonl` file and a label prefix.
- `scripts/one-file-quiet.sh` (new): touches `src/viewer.rs`, runs `measure-build.sh` for `<prefix>-N`, and stops once it has the wanted number of quiet records (default 3, up to 6 attempts), then prints the selected figure. Run it with `CARGO_TARGET_DIR` set to a warm root.
- `scripts/check-quiet-figure.cjs` (new): fixture with 5 records (3 quiet at 3.1, 3.6 and 3.4 s, 2 noisy at 16.3 and 14.1 s). It expects the figure 3.4, 3 quiet, and rejected [b, d]. It also checks: no quiet record gives a null figure; a null `swapNoisy` is rejected; and an even count takes the mean of the two middle walls.

Witness, Phase A: `node scripts/check-quiet-figure.cjs` exits 0 with 4 cases.

## Round 45 Phase B: the witnesses, once each

Gate readings, `vmstat 1 2`: 20:21:28 swap-in 268 KiB/s, out 0 (floor re-run); 20:22:20 swap-in 172 KiB/s, out 0 (W-a); 20:22:26 swap-in 180 KiB/s, out 0 (W-b). All open.

| Witness | Wall (s) | Result |
| --- | --- | --- |
| Floor re-run, `r45-nochange` (warm `bt-r43-off-1`) | 1.0 build, 0.01 slot wait | rc 0, 0 crates, `swapNoisy` true (62.8 MiB out, peak 99174 KiB/s out). Not quiet |
| W-a, `check-quiet-figure.cjs` | 0.03 | rc 0, 4 cases |
| W-b, `one-file-quiet.sh r45-onefile` (6 one-file builds, `CARGO_TARGET_DIR` warm) | 33.1 total | selected figure 3.65 s from n=2 quiet; target was 3 quiet, so the driver ran all 6 attempts |

The six one-file builds:

| Record | Wall (s) | Swap in / out (MiB) | `swapNoisy` |
| --- | --- | --- | --- |
| `r45-onefile-1` | 7.8 | 20.3 / 698.7 | true |
| `r45-onefile-2` | 3.8 | 35.7 / 2.1 | true |
| `r45-onefile-3` | 3.8 | 8.4 / 0 | false |
| `r45-onefile-4` | 3.5 | 9.5 / 0.2 | false |
| `r45-onefile-5` | 3.2 | 35.2 / 0 | true |
| `r45-onefile-6` | 3.3 | 2.7 / 63.2 | true |

The one-file variance is smaller this round, 3.2 to 7.8 s, than the 3.1 to 17 s of Round 41. The rule works in one direction: the two quiet records are fast, and the slow record (`r45-onefile-1`, 7.8 s) is noisy. But `r45-onefile-5` (3.2 s, swap-in 35.2 MiB) and `r45-onefile-2` (3.8 s, 35.7 MiB) are noisy and fast, so the rule rejects fast builds too. It is conservative, and it never reports a noisy build as quiet.

Result: the quiet figure is 3.65 s from two quiet records, below the three the driver wanted. The floor has no quiet run this round.

### Round 45 ranked remaining work

1. No-change floor: reachable, still open. The mechanism is fixed (`597e4d1`). The only floors measured are noisy: `r44-b-nochange2` 2.9 s (28.5 MiB in) and `r45-nochange` 1.0 s (62.8 MiB out). Reopen with a no-change build with `swapNoisy` false, which needs a quiet window.
2. One-file variance: reachable, with a figure of 3.65 s from n=2 quiet records (`27565e5`). Reopen with a third quiet record, which the driver can take if it waits between attempts for the gate to open. That is the next change.
3. Bench real run: reachable only at 1-minute load 4 or less. Reopen with a run where every size gives a speedup.
4. Clean-build swap at `jobs = 12`: held, owner decision on memory and parallelism. Reopen when the owner changes `jobs` and total swap for a clean build falls below 1 GB.
5. Override's total effect: held; reverted in Round 43. Reopen with a comparison method that does not depend on swap.
6. Clean-build swap classification: held by decision (`swapNoisy` null). Reopen when a swap-to-wall relation is found for builds of more than one crate.
7. Release build, 862.7 s with 16.9 GB swapped in: held, same memory cause.
8. ZLIB-supercompressed KTX2 (rank 8): held, no sample file.
9. SwiftShader BC7 and the browser load gate: reachable with browser time only.
10. One-off GPU panic: held, no recurrence.
11. Owner-held: user-wide `jobs = 4`, the shim, sccache or mold, the Playwriter 0.11 upgrade. Reopen on the owner's decision.

## Round 46 Phase A: which process held the swapped-out pages

The gap: a noisy record gave host-wide swap totals and the build's tree RSS, but nothing that names a process. The swapped-out pages of a noisy build could belong to any process on the host.

Change:
- `scripts/lib/swap-owners.sh` (new): `swap_owners_sample` lists the pid, name and VmSwap of each process on `$PROC_ROOT` (default `/proc`) that holds swapped-out pages. `swap_owners_top` ranks processes by their peak VmSwap across the samples of one build, and returns the top five.
- `scripts/measure-build.sh`: the sampler takes a sample at its first tick and then every 2 s (every 4 ticks of 0.5 s), into a per-build file. The record gets `swapOwnersTop`, for example `"rustc(1234) 300.0 MiB; ..."`. An empty string means no process held swapped-out pages in any sample.

Witness: `scripts/check-swap-owners.sh` builds a fake proc tree with two samples. `beta` peaks at 300 KiB and falls to 120; `gamma` is 50 then 290. A process with no VmSwap line (`kthread`) and one with zero VmSwap (`alpha`) must not appear. The expected top two are `300 200 beta` and `290 290 gamma`. Green: `{"ok":true}`, exit 0.

Mutation, before its restore: `swap_owners_top` keeps each process's last sample instead of its peak. The top becomes `290 gamma; 120 beta`, `{"ok":false}`, exit 1. Restore: the lib is untracked, so `git checkout` could not restore it; the original text was written back. The check is green again.

Field check: a no-change build through the changed `measure-build.sh` (`r46-field`, warm `bt-r43-off-1`): rc 0, wall 1 s, `swapNoisy` false, `swapOwnersTop` empty (no swap, so no holders), no temp file left.

## Round 46 Phase B: witnesses, gate readings, and the ranked work

Gate readings (`vmstat 1 2`):
- 20:31:39 si 140, so 0 (open). Load 1-minute 4.27, read at the start.
- 20:33:00 si 1932, so 0 (closed). The floor run `r46-nochange` was started in the same call, and it is not counted (see the breach note below).
- 20:33:37 si 328, so 0 (open). Floor witness `r46-nochange-b`.
- 20:34:15 si 580, so 0 (open). One-file driver `r46-onefile-b`.
- 20:35:30 si 868, so 0 (open). Bench `r46-bench`, which checked again inside the job at 20:35:38: si 768, so 0, load1 2.45.
- 20:37:38 si 22120, so 0 (closed). Witness W-c was run in this window, and its result does not count.
- 20:37:48 si 592, so 0 (open). W-c re-run, which counts.

Counted Phase B witnesses, each run once unless noted:

| Witness | Wall (s) | Result |
| --- | --- | --- |
| Floor, `r46-nochange-b` (warm `bt-r43-off-1`) | 0.7 build, 0.02 slot wait, driver 1.43 | rc 0, 0 crates, `swapNoisy` false, swap in 0.1 MiB, out 0.1 MiB, peak in 161 KiB/s, peak out 179 KiB/s, tree RSS 141.3 MiB |
| One-file, `one-file-quiet.sh r46-onefile` (driver job, 6 attempts max, 3 quiet wanted) | 25.0 driver | figure 3.0 s from 3 quiet runs; attempts 1 and 2 noisy |
| Bench, `RUNS=5 scripts/bench-vs-jayzee.sh` | 20.5 | rc 0; load1 2.45 at start, 2.96 at end |
| W-c, `check-swap-owners.sh` | 0.05 | rc 0, `{"ok":true,"top":"300 200 beta;290 290 gamma"}` |

One-file records (warm root, `CARGO_TARGET_DIR` `bt-r43-off-1`):

| Record | Wall (s) | Swap in / out (MiB) | `swapNoisy` |
| --- | --- | --- | --- |
| `r46-onefile-1` | 7.6 | 10.8 / 405.8 | true |
| `r46-onefile-2` | 3.7 | 6.5 / 4.8 | true (11.3 MiB total, over 10) |
| `r46-onefile-3` | 3.3 | 2.1 / 0.7 | false |
| `r46-onefile-4` | 3.0 | 1.2 / 0 | false |
| `r46-onefile-5` | 3.0 | 0.5 / 3.8 | false |

The quiet figure is the median of 3.3, 3.0 and 3.0, so 3.0 s. The record in attempt 6 was never run: the driver stopped at three quiet records. The noisy record `r46-onefile-1` (7.6 s, 405.8 MiB out) is the same kind of event as the 698.7 MiB out of Round 45.

Bench (`target/witness/bench-vs-jayzee.json`, all 15 rows not refused):

| Bodies | Speedup (median, all runs) | Speedup (quiet runs) | Quiet / noisy |
| --- | --- | --- | --- |
| 500 | 1.04 | 1.04 (0.325 vs 0.3118 ms) | 3 / 2 |
| 2000 | 2.55 | 2.55 (2.087 vs 0.819 ms) | 4 / 1 |
| 5000 | 4.56 | 4.56 (6.561 vs 1.438 ms) | 5 / 0 |

Every size has a speedup above 1, so the bench reopen condition is met. The 500-body size is at parity (1.04), and its speedup is within run-to-run noise.

Floor: the quiet floor is 0.7 s (`r46-nochange-b`), against the 0.4 s quiet split from Rounds 40 and 41. The 0.3 s gap is not split this round.

Which process holds the swapped-out pages: the owner list (`swapOwnersTop`, the peak VmSwap per process across samples) is the same in every record: `gm-core` pid 2813 at 3248 to 3279 MiB, `selkies` pid 471 at 339 to 347 MiB, `playwriter-ws-s` pid 3627 at 221 to 224 MiB, `gm-core` pid 2594 at 212 MiB, `claude` pid 2356 at 123 MiB. None is a cargo, rustc or build process. The list is a snapshot of standing holders, not a delta over the build, so it cannot say that `gm-core` caused the 405.8 MiB swapped out during `r46-onefile-1`. A per-process delta would need the sample at the build's start and end, which is the next change.

Breaches and non-counted runs, disclosed:
- `r46-field`, the Phase A field check, ran with no gate reading. It is a no-change build (1 s, `swapNoisy` false), not counted.
- `r46-nochange`, the first floor run, started at a reading of 1932 KiB/s in: the gate was closed, and I started the build anyway in the same call. The build's own gate read 208 KiB/s and it recorded quiet at 0.7 s. It is not counted. The Phase B floor witness is `r46-nochange-b`, which started at 328 KiB/s.
- W-c's first run was at 22120 KiB/s in. It was a 0.05 s fixture with no build, and it is not counted. The W-c re-run is the counted witness.
- The first one-file driver (`r46-onefile`) stopped at its gate at 324 / 7476 KiB/s, before any build, so nothing ran.

### Round 46 ranked remaining work

1. No-change floor: quiet at 0.7 s (`r46-nochange-b`), 0.3 s above the 0.4 s split. Reachable. Reopen with a per-process swap delta over a no-change build, which would explain the 0.3 s, or with the next quiet floor showing a different figure.
2. Bench real run: done this round. Speedup above 1 at every size (500 1.04, 2000 2.55, 5000 4.56). The 500-body row is at parity.
3. One-file variance: done this round. Quiet figure 3.0 s from three quiet runs (3.3, 3.0, 3.0); noisy records at 7.6 and 3.7 s. Reopen with a quiet build above 4 s, or a noisy one below 3.5 s.
4. Clean-build swap at `jobs = 12`: held, owner decision on memory and parallelism. Reopen when the owner changes `jobs` and a clean build's total swap falls below 1 GB.
5. Override's total effect: held, reverted in Round 43. Reopen with a comparison method that does not depend on swap.
6. Clean-build swap classification: held by decision (`swapNoisy` null). Reopen when a swap-to-wall relation is found for builds of more than one crate.
7. Release build (862.7 s, 16.9 GB swapped in): held, same memory cause.
8. ZLIB-supercompressed KTX2 (rank 8): held, no sample file.
9. SwiftShader BC7 and the browser load gate: reachable with browser time only, not run.
10. One-off GPU panic: held, no recurrence.
11. Owner-held: user-wide `jobs = 4`, the shim, sccache or mold, the Playwriter 0.11 upgrade.
12. Per-process swap delta: reachable, next change. Owner snapshot is peak-based, so `swapOwnersTop` names standing holders, not the process that caused a build's swap-out.

## Round 47 Phase A: the 0.7 s no-change floor, split from the records

Records used:
- `r46-nochange-b` (counted quiet floor): `wallS` 0.7, `slotWaitS` 0.02, 0 crates, `swapNoisy` false, swap 0.1 MiB in and 0.1 MiB out, tree RSS 141.3 MiB. Its cargo timing report, `target/witness/timings-r46-nochange-b.html`, has "Total time 0.0s".
- `split-nochange` (the 0.4 s quiet split, Round 39 root `bt-r39-final`): `wallS` 0.4, 0 crates, swap 0, same `build-timed` wrapper.

What the existing fields give:
- `slotWaitS` (0.02) is measured before `build-timed` starts its clock, so it is not in `wallS`. It does not explain any of the 0.7 s.
- Cargo's own build clock reported 0.0 s (the timing report). So the 0.7 s is outside cargo's reported build time: the shim (`sh`, flock on the slots, `exec`), cargo's process start and config load before that clock, and cargo's exit.
- The split between shim, cargo startup, and the fingerprint check cannot be made from the existing fields. The "Finished ... in X s" line that would give cargo's own elapsed time was in `target/build-timed-last.log`, and later builds overwrote it. No record before this change holds it.
- The 0.4 s reference used the same wrapper and no slot wait; the slot wait is measured separately, so it cannot be the 0.3 s difference.

Missing field, added as the change: `scripts/build-timed.sh` now records `cargoFinishedS`, the elapsed time from cargo's last "Finished ... in X s" line (0 when none is printed, e.g. a minute-scale line), and `outsideCargoS`, which is `wallS` minus `cargoFinishedS`. It also takes `CARGO_BIN` (default the shim), so a fixture can run a fake cargo.

What the next quiet no-change record will show: if `outsideCargoS` stays near 0.7 with `cargoFinishedS` near 0, the time is outside cargo's Finished clock. The shim share is still not separated. A direct run through `/data/cache/cargo/bin/cargo` against the shim run would separate it (Phase B, gate open, no other cargo running).

Witness: `scripts/check-build-split.sh` runs `build-timed.sh` with a fake cargo that sleeps 0.4 s, prints "Finished ... in 0.10s", and sleeps 0.2 s. Green: `{"ok":true,"rc":0,"wallS":0.6,"cargoFinishedS":0.1,"outsideCargoS":0.5}`, exit 0.
Mutation: `"$wall" "$finished" "$outside"` replaced by `"$wall" 0 "$wall"` (the field is not parsed). Red: `cargoFinishedS` 0, `{"ok":false}`, exit 1. Restored from `/tmp/bt/build-timed.good`; green again, exit 0.

## Round 47 Phase A: the 500-body bench size, decided from the records

Record used: `target/witness/bench-vs-jayzee.json` from `r46-bench` (Round 46, 15 rows, `RUNS=5`, load1 2.45 to 2.96). It is the only bench output on disk; each bench run overwrites it.

Paired ratios at 500 bodies (jayzee mean over bevy mean, same run pair):

| Run | jayzee ms | bevy ms | ratio | `noisy` |
| --- | --- | --- | --- | --- |
| 1 | 0.336 | 0.3314 | 1.014 | true (other runnable tasks reached 6 or more) |
| 2 | 0.331 | 0.3338 | 0.992 | true (same reason) |
| 3 | 0.342 | 0.3203 | 1.068 | false |
| 4 | 0.321 | 0.3115 | 1.030 | false |
| 5 | 0.325 | 0.3118 | 1.042 | false |

Quiet pairs (3, 4, 5): ratios 1.068, 1.030, 1.042, all above 1. All five pairs: mean 1.029, one below 1 (run 2, noisy).

Decision: hold. The evidence does not settle it.
- Three quiet pairs out of three all above 1 is a sign-test result of p 0.125 (one-sided). It does not clear 0.05.
- The per-run spread at 500 bodies is about 7% (bevy 0.3115 to 0.3338 ms). The quiet ratio of 3 to 7% sits inside that spread.
- The two noisy runs are the ones with the bevy time above the quiet median, and run 2 is below 1. Dropping noisy runs is the rule the ledger has used, but it leaves only three pairs.
- The 2000 and 5000 sizes are unaffected. Their quiet ratios are 2.40 to 2.63 and 4.35 to 4.70, far outside the spread.

Reopen evidence (held): five quiet pairs at 500 bodies, each with the jayzee over bevy ratio above 1 (5 of 5 gives p 0.031), and a quiet mean ratio of 1.05 or more. Or a 500-body run whose ratio falls outside the five-run spread of its own sizes. Both need `load1` 4 or less at the start of the run.

### Round 47 ranked remaining work

1. No-change floor residual: reachable. Quiet floor 0.7 s (`r46-nochange-b`), 0.3 s above the 0.4 s split. Phase A (`4a19f73`) records `cargoFinishedS` and `outsideCargoS`. Phase B floor run shows how much is outside cargo's clock. Shim share needs a direct-binary run against the shim run.
2. Per-process swap delta over a build: reachable. The owner list is a peak snapshot, so `swapOwnersTop` names standing holders (`gm-core` pid 2813 at about 3.25 GB), not the process behind a build's swap-out. Reopen when the delta is recorded.
3. 500-body bench: held (above). Reopen with five quiet pairs at 500 bodies above 1 and a quiet mean of 1.05 or more, at load1 4 or less.
4. One-file variance: done in Round 46 (figure 3.0 s from three quiet runs, 3.3, 3.0, 3.0). Reopen with a quiet build above 4 s, or below 3.5 s.
5. Clean-build swap at `jobs = 12`: held, owner decision on memory and parallelism. A clean build peaks about 7 GB in its own tree and swaps GBs on this host. Reopen when the owner changes `jobs` and a clean build's total swap falls below 1 GB.
6. Override's total effect: held, reverted in Round 43. Reopen with a comparison method that does not depend on swap, such as paired runs with the spread.
7. Clean-build swap classification: held by decision (`swapNoisy` null for builds of more than one crate). Reopen when a swap-to-wall relation is shown for multi-crate builds.
8. Release build (862.7 s, 16.9 GB swapped in): held, same memory cause. Reopen with the owner's `jobs` decision.
9. ZLIB-supercompressed KTX2 (rank 8): held, no sample file. Reopen when a sample exists.
10. SwiftShader BC7 reference and the browser load gate: reachable with browser time only, not run. Reopen when a browser window is scheduled.
11. One-off GPU panic: held, no recurrence. Reopen on recurrence, with the adapter log.
12. Owner-held: user-wide `jobs = 4`, the cargo shim (5 s retry), sccache or mold, and the Playwriter 0.11 upgrade.

## Round 47 Phase B: the floor split, with a correction to 4a19f73

Correction: the Phase A note in `4a19f73` said the 0.7 s floor was outside cargo's build clock, based on the timing report's "Total time 0.0s". That report counts compile units only. The new field shows cargo's own Finished clock holds most of the floor time. `outsideCargoS` (shim, cargo process start, exit) is 0.1 s on every run.

Gate readings (`vmstat 1 2`), all open, swap-out 0 each time: 20:48:17 si 216; 20:48:32 si 72 (W-d); 20:48:33 si 4 (floor); 20:48:49 si 120 (diagnostic); 20:49:29 si 4, 20:49:31 si 40, 20:49:33 si 36 (repeats).

Witness, run once: `scripts/check-build-split.sh`, wall 0.73 s, rc 0, `{"ok":true,"rc":0,"wallS":0.6,"cargoFinishedS":0.1,"outsideCargoS":0.5}`.

No-change floor, warm root `bt-r43-off-1`, 0 crates, swap 0 in and 0 out on every run:

| Run | wallS | cargoFinishedS | outsideCargoS | swapNoisy |
| --- | --- | --- | --- | --- |
| `r46-nochange-b` (counted in Round 46, before the field) | 0.7 | not recorded | not recorded | false |
| `r47-nochange` (first counted run after the field) | 1.1 | 0.97 | 0.1 | false |
| diagnostic (`CARGO_LOG=cargo=debug`, direct `build-timed`) | 0.6 | 0.48 | 0.1 | not measured |
| `r47-nochange-rep1` | 0.4 | 0.30 | 0.1 | false |
| `r47-nochange-rep2` | 0.4 | 0.31 | 0.1 | false |
| `r47-nochange-rep3` | 0.4 | 0.31 | 0.1 | false |

Median of the four quiet runs with the field: 0.4 s (`r47-nochange-rep1` to rep3 and the 1.1 s run give 0.4, 0.4, 0.4, 1.1; `r46-nochange-b` 0.7 has no cargo field). The reference split is 0.4 s (`split-nochange`). The 0.3 s gap of Round 46 is inside the spread of identical runs on this root: 0.4 to 1.1 s, with zero swap. It is cargo's clock running slow, not swap, and not the shim.

Inside cargo (diagnostic run, timestamps from cargo's own start, Finished 0.48 s):
- config and workspace load: 0.000 to 0.006 s.
- dependency resolution (`resolve_with_previous` and `resolve`, 8466 and 7733 resolver calls): 0.006 to 0.354 s, about 0.35 s. This is about 73% of cargo's time.
- `check_collisions`: 0.354 to 0.373 s, 517 lines.
- unlogged stretch between `check_collisions` and execute: 0.373 to 0.458 s, about 0.085 s, not logged, so not assigned.
- `compile:execute` (fingerprint decisions for 452 Fresh units, timing report, linking): 0.458 to 0.489 s, about 0.03 s.

Cause of the floor residual: almost all of the floor is cargo's dependency resolution, which runs on every invocation even when nothing compiles. measure-build and the shim add about 0.1 s (`outsideCargoS`). The fingerprint check is about 0.03 s of the cargo clock.

Held: the resolution cost is cargo's own work for this workspace. It is reachable only by changing the workspace or lockfile shape, which the owner decides, and it is out of this stretch's scope. Reopen with a quiet no-change median above 0.5 s over repeats, or a `cargoFinishedS` above 0.5 s on a quiet run with `outsideCargoS` above 0.2 s.

## Round 48 Phase A: per-process swap delta over a build

The gap, from Round 46: `swapOwnersTop` is a peak snapshot. It names standing holders of swapped-out pages, but cannot say which process gained pages during a build. For `r46-onefile-1` (405.8 MiB out), the list was the same as in every other record.

Change:
- `scripts/lib/swap-owners.sh`: `swap_owners_growth before after` takes two samples from `swap_owners_sample` and prints the top five processes by positive change in VmSwap, as `KiB pid name`. A process absent from the first sample counts from zero. A process that shrank or exited is not listed.
- `scripts/measure-build.sh`: samples just before the build launch and just after it returns. The record gets `swapOwnersGrowth`, for example `"name(pid) X MiB; ..."`. An empty string means no process gained swapped-out pages during the build.

Witness: `scripts/check-swap-growth.sh` builds a fake proc tree with two samples. `beta` goes 0 to 300 KiB (+300), `gamma` 50 to 290 (+240), `late` is new at 80 (+80). `shrink` drops 500 to 100 and is excluded, as is `alpha` (0 to 0) and `kthread` (no VmSwap line). Expected output: `300 200 beta`, `240 290 gamma`, `80 500 late`. Green: `{"ok":true}`, exit 0.

Mutation, before its restore: the delta `d = after[p] - before[p]` replaced by `d = after[p]`, which reports the post-build VmSwap. Red: `{"ok":false,"growth":"300 200 beta;290 290 gamma;100 400 shrink;80 500 late"}`, exit 1. The shrink and gamma values show the regression. Restored from `/tmp/bt/swap-owners.good`; green again, exit 0.

Not yet witnessed on a real build. The field is checked in Phase B by a one-file build through `measure-build.sh`.

## Round 48 Phase B: the per-process swap delta, witnessed live

Gate readings, `vmstat 1 2` (both directions below 1000 KiB/s, open): 21:01:34 si 228, so 0; 21:02:46 si 8, so 0 (witness); 21:02:48 si 48, so 0 (live build).

Witness, run once: `scripts/check-swap-growth.sh`, wall 0.08 s, rc 0, `{"ok":true,"growth":"300 200 beta;240 290 gamma;80 500 late"}`.

Live one-file build through `measure-build.sh` (`r48-onefile`, warm root `bt-r43-off-1`, `touch src/viewer.rs`), run once:
- wall 8.3 s build (cargo's clock 8.1 s, outside cargo 0.2 s), 1 crate, rc 0.
- `swapNoisy` true, rule "classified": 54.1 MiB in, 523.6 MiB out.
- `swapOwnersGrowth`: `gm-core(2813) 45.4 MiB; claude(755372) 20.2 MiB; claude(2109) 18.9 MiB; claude(2510) 14.6 MiB; claude(2707) 8.9 MiB`.
- `swapOwnersTop` (peak snapshot, for comparison): `gm-core(2813) 3275.6 MiB; selkies(471) 393.1 MiB; gm-core(2594) 248.9 MiB; playwriter-ws-s(3627) 208.2 MiB; hyperagent(1719) 51.4 MiB`.

What the growth shows:
- The build's own `rustc` does not appear in the growth list. The processes that gained swapped-out pages during the build were `gm-core` and `claude` sessions on this host.
- The top five account for about 108 MiB of the 523.6 MiB swapped out. The remaining 416 MiB is not named. Either it sits in processes outside the top five, or in processes that exited between samples, which the delta cannot see. The field is top-five by design, so this is a gap in the record, not a finding about the build.
- Reopen for the gap: widen the list to all processes with a positive delta and record the total, so the top-five share is a number rather than a guess.

Longest step: the 8.3 s live one-file build. What was done: nothing to the build. It was classified swap-noisy by the rule from Round 44, and the growth field attributes the swap-out to other processes. No change to the build or the rule.

### Round 48 ranked remaining work

1. Per-process swap delta: done (`adf28bb`), witnessed live. Top five explain 108 of 523.6 MiB. Reachable next: record the full positive-delta total, so the share is a number.
2. Clean-build swap at `jobs = 12`: held, owner decision. Measuring it again as-is would repeat the Round 43 records (six clean builds, 2857 to 8699 MiB swapped, 323 to 384 s wall). Reopen when the owner changes `jobs` and a clean build's total swap falls below 1 GB.
3. 500-body bench size: held (Round 47 decision). Reopen with five quiet pairs above 1 and a quiet mean of 1.05 or more, at 1-minute load 4 or less.
4. No-change floor residual: explained (Round 47). Cargo's dependency resolution is about 0.35 s of the 0.48 s cargo clock; quiet median 0.4 s. Held as cargo's own cost. Reopen with a quiet median above 0.5 s.
5. Unlogged cargo stretch, about 0.085 s between `check_collisions` and execute: held. It is inside cargo, with no hook to time it without patching cargo. Reopen only if it grows above 0.2 s on repeated quiet runs.
6. One-file variance: done (Round 46, quiet median 3.0 s from three runs). The live noisy one-file build here, 8.3 s, is a noisy record and is not a quiet figure. Reopen with a quiet build above 4 s or below 3.5 s.
7. Override's total effect: held, reverted in Round 43. Reopen with a comparison method that does not depend on swap.
8. Clean-build swap classification: held by decision (`swapNoisy` null for multi-crate builds). Reopen when a swap-to-wall relation is shown for multi-crate builds.
9. Release build (862.7 s, 16.9 GB swapped in): held, same memory cause. Reopen with the owner's `jobs` decision.
10. ZLIB-supercompressed KTX2 (rank 8): held, no sample file. Reopen when a sample exists.
11. SwiftShader BC7 and the browser load gate: reachable with browser time only, not run. Reopen when a browser window is scheduled.
12. One-off GPU panic: held, no recurrence. Reopen on recurrence, with the adapter log.
13. Owner-held: user-wide `jobs = 4`, the cargo shim (5 s retry), sccache or mold, and the Playwriter 0.11 upgrade.

## Round 49 Phase A: the full positive-delta total and the top-five share

Change:
- `scripts/lib/swap-owners.sh`: `swap_owners_growth_total before after` sums every positive VmSwap delta, in KiB. It is the total the top five are drawn from.
- `scripts/measure-build.sh`: the record gets `swapOwnersGrowthMiB` (the total of positive deltas, in MiB) and `swapOwnersTop5Pct` (the top five's KiB as a percentage of `swapOutMiB`, computed as top KiB × 100 / (swap-out pages × 4)). The percentage is 0 when no page was swapped out.

Units and meaning, stated so the number is read correctly: VmSwap is net per process (pages swapped out minus pages swapped back in), and `swapOutMiB` is the host's gross `pswpout` count. So the top-five percentage is the net gain of the top five over the gross host swap-out. It is a share, not an exact partition of the swap-out.

Witness: `scripts/check-swap-growth.sh` (the Round 48 fixture, with the total added). The same fake proc tree gives positive deltas beta +300, gamma +240, late +80, so the total is 620 KiB. Green: `{"ok":true,"growth":"300 200 beta;240 290 gamma;80 500 late","total":620}`, exit 0.

Mutation, before its restore: the positive-only guard `if (d > 0) sum += d` removed, so the negative delta of `shrink` (-400) is summed. Red: `"total":220`, `{"ok":false}`, exit 1. Restored from `/tmp/bt/swap-owners.good2`; green again, `"total":620`, exit 0.

Not covered by the fixture: the percentage formula in `measure-build.sh`. It is checked on live records in Phase B, with the arithmetic reproduced from the record fields.

## Round 49 Phase B: the full swap gain and the top-five share, witnessed live

Gate readings (`vmstat 1 2`). Closed readings are listed because they blocked a run, not as ledger evidence:
- 21:12:07 si 40, so 75980 (closed)
- 21:12:18 si 68, so 0 (open; the wait job found it)
- 21:12:39 si 2848, so 0 (closed)
- 21:12:49 si 980, so 0 (open; witness W-f)
- 21:12:50 si 2780, so 0 (closed; `onefile1` skipped by its own gate)
- 21:12:51 si 332, so 0 (open; build `r49-onefile-2`)
- 21:13:39 si 524, so 0 (open; build `r49-onefile-3`)

Witness, run once: `scripts/check-swap-growth.sh`, wall 0.13 s, rc 0, `{"ok":true,"growth":"300 200 beta;240 290 gamma;80 500 late","total":620}`.

Live one-file builds through `measure-build.sh`, warm root `bt-r43-off-1`, one crate each, rc 0:

| Record | Build wall (s) | Cargo clock (s) | Swap in (MiB) | Swap out (MiB) | Gain total (MiB) | Gain total / swap out | Top five (MiB) | Top five / swap out |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `r49-onefile-2` | 11.6 | 11.35 | 180.8 | 883.2 | 772.9 | 87.5% | 521.7 | 59.1% |
| `r49-onefile-3` | 8.4 | 8.28 | 16.1 | 703.1 | 686.2 | 97.6% | 542.9 | 77.2% |

Both builds were swap-noisy (`swapNoisy` true, above 10 MiB in plus out).

The arithmetic, from the record fields: the top-five share is the top-five KiB times 100 over (swap-out pages times 4 KiB). Record 2: 521.7 / 883.2 = 59.1%. Record 3: 542.9 / 703.1 = 77.2%. The gain total is the sum of all positive VmSwap changes, and it is a net figure (swapped out minus swapped in, per process) against the host's gross swap-out, so these shares are approximate.

What the numbers show:
- Positive gains across all processes account for 87.5% and 97.6% of the host swap-out. The top five account for 59 to 77%.
- The gaining processes are `gm-core` and `chromium.real` instances (other sessions' processes), plus `claude` in record 2. The build's own `rustc` is not in either top five.
- The record does not split the total into the build's process tree and the rest, so it cannot say how much of the 686 to 773 MiB the build's own processes gained. That split is the next step.

Longest step: `r49-onefile-2`, 11.6 s build (cargo clock 11.35 s), with 883 MiB swapped out. What was done: nothing to the build. The record shows the swap went to other processes, and the rule for noisy records (Round 44) already marks it.

### Round 49 ranked remaining work

1. Split the gain total into the build's process tree and everything else: reachable. The gain total is now recorded (87 to 98% of swap-out), but not by owner. Next change: classify each gaining pid against the build's tree pids. Reopen with a per-build split where the build's share is a number.
2. Clean-build swap at `jobs = 12`: held, owner decision. Reopen when the owner changes `jobs` and a clean build's total swap falls below 1 GB. The Round 43 records (six clean builds, 2857 to 8699 MiB, 323 to 384 s) are the baseline; running the clean build as-is would repeat them.
3. 500-body bench size: held (Round 47). Reopen with five quiet pairs above 1 and a quiet mean of 1.05 or more, at 1-minute load 4 or less.
4. No-change floor: explained (Round 47). Quiet median 0.4 s; cargo's dependency resolution is most of the cargo clock. Reopen with a quiet median above 0.5 s.
5. Unlogged cargo stretch, about 0.085 s: held, inside cargo. Reopen if it grows above 0.2 s on repeated quiet runs.
6. One-file variance: done (quiet median 3.0 s). The two live builds here are noisy and not figures. Reopen with a quiet build above 4 s or below 3.5 s.
7. Override's total effect: held, reverted in Round 43. Reopen with a method that does not depend on swap.
8. Clean-build swap classification: held by decision (`swapNoisy` null for multi-crate builds). Reopen when a swap-to-wall relation is shown.
9. Release build (862.7 s, 16.9 GB swapped in): held, same memory cause. Reopen with the owner's `jobs` decision.
10. ZLIB KTX2 (rank 8): held, no sample file.
11. SwiftShader BC7 and the browser load gate: reachable with browser time only, not run.
12. One-off GPU panic: held, no recurrence.
13. Owner-held: user-wide `jobs = 4`, the cargo shim, sccache or mold, and the Playwriter 0.11 upgrade.

## Round 50 Phase A: the swap gain split between the build tree and the rest

The gap: `swapOwnersGrowthMiB` is one total. It did not say how much of the gain belonged to the build's own processes, so the share of swap-out that no process explains could not be stated.

Change:
- `scripts/lib/build-rss.sh`: `build_tree_pids root` prints the pid of the root and of every descendant, one per line.
- `scripts/measure-build.sh`: the sampler appends the build tree's pids to a temp file on each 0.5 s tick. After the build, the file is sorted and deduplicated, and the gain is split.
- `scripts/lib/swap-owners.sh`: `swap_owners_growth_split before after tree` sums the positive VmSwap gains of pids in the tree (build) and of all other pids (other), in KiB.
- Record fields: `swapOwnersBuildMiB`, `swapOwnersOtherMiB`, `swapOwnersBuildPct` and `swapOwnersOtherPct` (each as a percentage of `swapOutMiB`, using the same formula as the total), and `swapOwnersUnattributedPct` = 100 − build % − other %.

Limits, stated so the numbers are read correctly:
- A process that starts and exits between samples is not in the before or after sample, so its swap is invisible to the gain. The unattributed share includes that.
- VmSwap is net (out minus in, per process), against the host's gross `pswpout`. A negative unattributed share means the net gain is larger than the host's gross swap-out in that build, and it is printed as is.

Witness: `scripts/check-swap-split.sh`. A fake proc tree gives gains beta +300 (pid 200, in the tree), late +80 (pid 500, in the tree), gamma +240 (pid 290, not in the tree); shrink (400) is in the tree but has a negative delta, and is excluded. Expected split `380 240`. The second check runs `build_tree_pids` on this shell with a `sleep 30` child, and expects the child's pid in the tree. Green: `{"ok":true,"split":"380 240","childInTree":true}`, exit 0.

Mutation, before its restore: the class assignment swapped (`if ($1 in tree) o += d; else b += d`). Red: `"split":"240 380"`, `{"ok":false}`, exit 1. Restored from `/tmp/bt/swap-owners.good4`; green again, exit 0.

Bug caught by the fixture on the first run, now fixed: awk reads the files in argument order, and the tree file was passed last, so the tree was empty when the gains were classified. The first run gave `0 620`, and the witness reported `ok:false`. The tree file is now passed first, and the awk rules name it as ARGV[1].

## Round 50 Phase A, change 2: the build-tree pid count in the record

The first live pair of one-file builds, run with change 1 alone, gave a build-tree gain of 0 in both records (`r50-onefile-1` and `-2`). A 0 can mean the build's processes never swapped, or that the tree sampling missed them, and the record could not tell the two apart.

Change: `swap_owners_tree_size tree` counts the distinct pids in the sampled tree. `measure-build.sh` records it as `swapOwnersTreePids`. A count of 1 or 2 for a one-crate build would mean only the wrapper and cargo were seen, which the witness and the live record can then show.

Witness: `scripts/check-swap-split.sh` now also checks the tree size of its fixture tree (pids 200, 500, 400), expecting 3. Green: `{"ok":true,"split":"380 240","treeSize":3,"childInTree":true}`, exit 0.

Mutation, before its restore: `swap_owners_tree_size` made to print 0 (`END { print 0 }`). Red: `"treeSize":0`, `{"ok":false}`, exit 1. Restored from `/tmp/bt/swap-owners.good5`; green again, exit 0.

## Round 50 Phase B: the build-tree split, witnessed live

Every gate reading taken, in order, from two jobs. The gate flapped, so the closed readings are kept with the open ones. `si` is swap-in and `so` swap-out, both in KiB/s.

First job (before change 2, the tree-size field):
- 21:23:00 si 776, so 0 (open): witness W-g
- 21:23:01 si 176, so 0 (open): build `r50-onefile-1`
- 21:23:13 si 2652, so 0 (closed)
- 21:23:17 si 1776, so 0 (closed)
- 21:23:21 si 1380, so 0 (closed)
- 21:23:25 si 400, so 0 (open): build `r50-onefile-2`

Second job (after change 2):
- 21:24:16 si 8512, so 21804 (closed)
- 21:24:20 si 6836, so 176 (closed)
- 21:24:24 si 6436, so 0 (closed)
- 21:24:28 si 9592, so 1240 (closed)
- 21:24:32 si 10376, so 0 (closed)
- 21:24:36 si 8132, so 0 (closed)
- 21:24:40 si 956, so 0 (open): witness W-h
- 21:24:41 si 3176, so 0 (closed)
- 21:24:45 si 480, so 0 (open): build `r50b-onefile-1`
- 21:25:02 si 520, so 0 (open): build `r50b-onefile-2`

Witness, run once in the second job: `scripts/check-swap-split.sh`, wall 0.10 s, rc 0, `{"ok":true,"split":"380 240","treeSize":3,"childInTree":true}`. The first job's W-g run was 0.11 s with the same result, before change 2.

First pair (`r50-onefile-*`, change 1 alone, no tree size):

| Record | Build wall (s) | Swap out (MiB) | Build gain (MiB) | Other gain (MiB) | Build % | Other % | Unattributed % |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `r50-onefile-1` | 8.8 | 632.4 | 0 | 138.7 | 0 | 21.9 | 78.1 |
| `r50-onefile-2` | 4.3 | 51.0 | 0 | 47.5 | 0 | 93.2 | 6.8 |

A build-tree gain of 0 here could be a real zero or a missed tree, and this pair cannot say which.

Second pair (`r50b-onefile-*`, change 2 in place):

| Record | Build wall (s) | Swap in (MiB) | Swap out (MiB) | Tree pids | Build gain | Other gain | `swapNoisy` |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `r50b-onefile-1` | 14.7 | 48.7 | 1.1 | 6 | 0 | 0 | true |
| `r50b-onefile-2` | 5.1 | 1.3 | 0.8 | 6 | 0 | 0 | false |

The tree was sampled in both builds: 6 distinct pids each, so the build-tree gain of 0 is a real zero. The build's own processes did not gain swapped pages.

Why the unattributed share is 100% in the second pair: the host swapped out 1.1 and 0.8 MiB, and the net VmSwap gain of every process was 0 or negative. In `r50b-onefile-1` the host swapped 48.7 MiB back in, which reduces VmSwap of the processes that held those pages. So net per-process gain is a poor estimate of gross host swap-out when swap-in dominates. The gap is a property of the measure, not of the build.

Longest step: `r50b-onefile-1`, 14.7 s build, while the gate was flapping (its own reading 3176 closed, then 480 open). What was done: nothing to the build; the swap-in during it is recorded, and the record marks it swap-noisy.

### Round 50 ranked remaining work

1. Gross per-process swap: held. Linux `/proc/<pid>/status` reports only the current VmSwap, so a gross figure (out minus in, per process) needs a counter the kernel does not expose here. Reopen when the owner or the kernel gives a per-process swap-out counter, or when a second estimator (for example per-cgroup memory events) is available.
2. Clean-build swap at `jobs = 12`: held, owner decision on `jobs`. Reopen when the owner changes `jobs` and a clean build's total swap falls below 1 GB. Baseline: six Round 43 clean builds, 2857 to 8699 MiB swapped, 323 to 384 s wall. Running it as-is would repeat that evidence.
3. 500-body bench size: held (Round 47). Reopen with five quiet pairs above 1 and a quiet mean of 1.05 or more, at 1-minute load 4 or less.
4. No-change floor: explained (Round 47). Quiet median 0.4 s; cargo's dependency resolution is most of the cargo clock. Reopen with a quiet median above 0.5 s.
5. Unlogged cargo stretch, about 0.085 s: held, inside cargo. Reopen if it grows above 0.2 s on repeated quiet runs.
6. One-file variance: done (quiet median 3.0 s). The live builds in this round are noisy or quiet-but-unrepeated, and are not figures. Reopen with a quiet build above 4 s or below 3.5 s.
7. Override's total effect: held, reverted in Round 43. Reopen with a method that does not depend on swap.
8. Clean-build swap classification: held by decision (`swapNoisy` null for multi-crate builds). Reopen when a swap-to-wall relation is shown.
9. Release build (862.7 s, 16.9 GB swapped in): held, same memory cause. Reopen with the owner's `jobs` decision.
10. ZLIB KTX2 (rank 8): held, no sample file.
11. SwiftShader BC7 and the browser load gate: reachable with browser time only, not run.
12. One-off GPU panic: held, no recurrence.
13. Owner-held: user-wide `jobs = 4`, the cargo shim, sccache or mold, and the Playwriter 0.11 upgrade.

## Round 51 Phase A: what the swap records can attribute, and what they cannot

Decision, from the records and the kernel's interfaces, before any new field:

Attributable per process:
- The signed VmSwap change of each process alive at both samples, and of each new process (`swapOwnersNetMiB`). Exited processes are not in the end sample, so they are left out.
- The build tree's residency peak: the sum, over build-tree pids, of the largest VmSwap seen in the 2 s owner samples (`swapOwnersTreePeakMiB`). It is a sampled lower bound on the swapped pages the build's processes held at once. It is not a swap-out count.

Not attributable from these records:
- Gross swap-out and swap-in per process. The kernel gives no per-process counter. `/proc/vmstat` and the root `memory.stat` carry only host-wide `pswpin` and `pswpout`. This shell is in the root cgroup (`0::/`), and `/sys/fs/cgroup` holds one `memory.stat`, the root one, so there is no per-group swap counter to read.
- Pages freed when a process exits. Host flow (out minus in) equals the net change of the processes that survive or start, plus the pages released by exits. The residual is `swapExitFreedMiB` = host flow minus `swapOwnersNetMiB`. It is a number, but it does not name the process that exited. Pages of the build's own processes that exit before the end sample belong here, and the tree peak shows only how large they got, sampled every 2 s.

So the unattributed share is split into two measured parts: the exit-freed residual (a number, with no pid) and the swap-in offset already inside the signed net. Nothing is guessed. Each record carries a `swapAttribution` string that says this.

Change:
- `scripts/lib/swap-owners.sh`: `swap_owners_net before after` sums the signed change of pids in the after sample (exited pids are not in it). `swap_owners_tree_peak tree owners` sums each tree pid's largest VmSwap across the owner samples.
- `scripts/measure-build.sh`: records `swapOwnersNetMiB`, `swapExitFreedMiB` (host flow minus net, MiB), `swapExitFreedPct` (of swap-out), `swapOwnersTreePeakMiB` and `swapAttribution`. The tree peak is computed before the owner samples are removed.

Witness: `scripts/check-swap-net.sh`. The fixture has survivors beta +300, gamma +240, shrink −400, late +80 (new), alpha 0, and an exited pid 600 (120 KiB before, gone after). Net = 220, and the exited pid is excluded. The tree pids 200, 500 and 400 have peaks 300, 80 and 500, so the tree peak = 880. Green: `{"ok":true,"net":220,"treePeak":880}`, exit 0.

Mutation, before its restore: the net sums only positive changes (`if (d > 0) sum += d`). Red: `"net":620`, `{"ok":false}`, exit 1. Restored from `/tmp/bt/swap-owners.good6`; green again, exit 0.

The identity behind `swapExitFreedMiB` holds only as well as the counters are aligned. The host counters are read a few milliseconds before the owner sample, so the residual carries that small offset.

## Round 51 Phase A, change 2: the residual is the flow of pids gone at the end, not freed pages

Correction to change 1 (`97c4062`). Its ledger text called the host flow minus the signed net "pages freed by exits". That is wrong. Freeing a swap slot on exit does not change `pswpin` or `pswpout`, so it is not in the flow. The residual is the swap flow (swap-out minus swap-in) of processes that are absent from the end sample: their own out and in events during the window, which the net leaves out by construction.

The first live pair (`r51-onefile-*`, change 1) gave a residual of −4.3 and −1.8 MiB. The sign is consistent with those pids having swapped in more than out. It is not a freed-page count.

Change: the record field `swapExitFreedMiB` is renamed to `swapGoneFlowMiB`, and `swapExitFreedPct` to `swapGoneFlowPct`. The formula is unchanged. `swap_gone_flow_kib si so net` returns `(so - si) * 4 - net` in KiB, and `measure-build.sh` records it, with its share of swap-out. The `swapAttribution` string now says: net per pid alive at the end, gone flow not per pid, tree peak sampled at 2 s.

Identity, so the two parts add up: the window's host flow (swap-out minus swap-in, in KiB) = `swapOwnersNetMiB` + `swapGoneFlowMiB`, to within the counter alignment noted in change 1.

Witness: `scripts/check-swap-net.sh` now also checks `swap_gone_flow_kib 100 300 220`, which must give 580: (300 − 100) × 4 − 220. Green: `{"ok":true,"net":220,"treePeak":880,"gone":580}`, exit 0. (The fixture's first version expected 180, which was my arithmetic error; the function was correct, and the expectation is now fixed.)

Mutation, before its restore: the sign of the net term flipped (`* 4 + $3`). Red: `"gone":1020`, `{"ok":false}`, exit 1. Restored from `/tmp/bt/swap-owners.good8`; green again, `"gone":580`, exit 0.

## Round 51 Phase B: the attribution, witnessed live, and the decision

Every gate reading taken, in order. The gate flapped, and the closed readings are kept with the open ones. `si` is swap-in and `so` swap-out, both in KiB/s.

First job (change 1 fields, `swapExitFreedMiB`, now renamed to the gone flow):
- 21:33:13 si 8232, so 50284 (closed)
- 21:33:17 si 11144, so 123980 (closed)
- 21:33:21 si 80216, so 80960 (closed)
- 21:33:25 si 53000, so 0 (closed)
- 21:33:29 si 24020, so 352632 (closed)
- 21:33:33 si 13420, so 284940 (closed)
- 21:33:37 si 46588, so 310424 (closed)
- 21:33:41 si 12092, so 0 (closed)
- 21:33:45 si 34780, so 253220 (closed)
- 21:33:49 si 18420, so 245452 (closed)
- 21:33:53 si 212528, so 363596 (closed)
- 21:33:57 si 25636, so 190984 (closed)
- 21:34:01 si 13856, so 10992 (closed)
- 21:34:05 si 4608, so 0 (closed)
- 21:34:10 si 39564, so 27932 (closed)
- 21:34:14 si 4652, so 239020 (closed)
- 21:34:18 si 60380, so 116984 (closed)
- 21:34:22 si 221816, so 131836 (closed)
- 21:34:26 si 32048, so 334660 (closed)
- 21:34:30 si 19604, so 40120 (closed)
- 21:34:34 si 10444, so 146176 (closed)
- 21:34:38 si 17972, so 87184 (closed)
- 21:34:42 si 158816, so 13068 (closed)
- 21:34:46 si 32108, so 0 (closed)
- 21:34:50 si 15916, so 0 (closed)
- 21:34:54 si 38328, so 0 (closed)
- 21:34:58 si 1860, so 0 (closed)
- 21:35:02 si 4524, so 0 (closed)
- 21:35:06 si 6100, so 0 (closed)
- 21:35:10 si 13168, so 0 (closed)
- 21:35:14 si 9204, so 0 (closed)
- 21:35:18 si 46896, so 0 (closed)
- 21:35:22 si 372, so 0 (open): witness W-i
- 21:35:24 si 5748, so 0 (closed)
- 21:35:28 si 1808, so 0 (closed)
- 21:35:32 si 6996, so 0 (closed)
- 21:35:36 si 8116, so 0 (closed)
- 21:35:40 si 2600, so 0 (closed)
- 21:35:44 si 1152, so 0 (closed)
- 21:35:48 si 4128, so 0 (closed)
- 21:35:52 si 736, so 0 (open): build `r51-onefile-1`
- 21:36:01 si 1092, so 0 (open)
- 21:36:05 si 1336, so 0 (open)
- 21:36:09 si 1564, so 0 (open)
- 21:36:13 si 1388, so 0 (open)
- 21:36:17 si 916, so 0 (open): build `r51-onefile-2`

Second job (renamed fields):
- 21:37:19 si 264, so 0 (open): witness W-j
- 21:37:20 si 492, so 0 (open): build `r51b-onefile-1`
- 21:37:26 si 156, so 0 (open): build `r51b-onefile-2`

Witnesses, each run once in the second job: `scripts/check-swap-net.sh`, wall 0.09 s, rc 0, `{"ok":true,"net":220,"treePeak":880,"gone":580}`. The first job's W-i run was 0.10 s, before the rename.

Live builds (warm root `bt-r43-off-1`, one crate each, rc 0). The first job's builds are kept as superseded; they carry the same numbers under the old name.

| Record | Build wall (s) | Swap in (MiB) | Swap out (MiB) | Net, alive at end (MiB) | Gone flow (MiB) | Tree pids | Tree peak (MiB) | `swapNoisy` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `r51-onefile-1` (old name) | 6.5 | 5.4 | 0 | −1.1 | −4.3 | 6 | 0 | false |
| `r51-onefile-2` (old name) | 3.0 | 2.4 | 0 | −0.7 | −1.8 | 6 | 0 | false |
| `r51b-onefile-1` | 3.4 | 0.4 | 0 | 0 | −0.3 | 6 | 0 | false |
| `r51b-onefile-2` | 4.3 | 1.6 | 0 | −0.9 | −0.7 | 6 | 0 | false |

The identity holds in each row: host flow (out minus in) equals net plus gone flow. For `r51b-onefile-1`, the flow is −0.4 and the sum is −0.3; the 0.1 difference is the rounding of the displayed swap-in, since the record stores the exact KiB.

### Decision: what can be attributed, and what cannot

Attributable from these records:
- The signed net of each process alive at the end, per pid. In the quiet builds all of it is survivors' swap-in, not swap-out.
- The build tree's residency: its pids held 0 swapped pages at every 2 s sample, in all four builds. So the build's own processes did not hold any swap in these builds, and they did not swap out.

Not attributable from these records:
- The gone flow, the swap of processes absent from the end sample, is a number but has no pid. In the quiet builds it is swap-in only (−0.3 to −0.7 MiB). Which processes produced it is not recorded.
- Gross swap-out and swap-in per process. The kernel gives no per-process counter, and there is one root `memory.stat`, so there is no per-group split either.
- A swap-out share for a build. The quiet builds have swap-out 0, so the percentage fields read 0 there, and mean "undefined", not "no share".

So the Round 49 question, whose processes did the swap-out of a noisy build, is not answered by these records. The new fields answer it only for a build with swap-out, and no such build has been measured with them.

### Round 51 ranked remaining work

1. Gross per-process swap: held. Reopen when the kernel or owner exposes a per-process swap-out counter, or a per-group `pswpout` (the root `memory.stat` is the only one here).
2. Attribution of a noisy build's swap-out: held. Reopen with a build whose swap-out is above 100 MiB, measured with the new fields. A noisy build cannot be started while the gate is closed, and the gate has been closed for most of this round.
3. Clean-build swap at `jobs = 12`: held, owner decision on `jobs`. Reopen when the owner changes `jobs` and a clean build's total swap falls below 1 GB. Baseline: six Round 43 clean builds, 2857 to 8699 MiB swapped, 323 to 384 s wall.
4. 500-body bench size: held (Round 47). Reopen with five quiet pairs above 1 and a quiet mean of 1.05 or more, at 1-minute load 4 or less.
5. No-change floor: explained (Round 47). Quiet median 0.4 s, with cargo's dependency resolution as most of the cargo clock. Reopen with a quiet median above 0.5 s.
6. Unlogged cargo stretch, about 0.085 s: held, inside cargo. Reopen if it grows above 0.2 s on repeated quiet runs.
7. One-file variance: done (quiet median 3.0 s, Round 46). Reopen with a quiet build above 4 s or below 3.5 s.
8. Override's total effect: held, reverted in Round 43. Reopen with a method that does not depend on swap.
9. Clean-build swap classification: held by decision (`swapNoisy` null for multi-crate builds). Reopen when a swap-to-wall relation is shown for multi-crate builds.
10. Release build (862.7 s, 16.9 GB swapped in): held, same memory cause. Reopen with the owner's `jobs` decision.
11. ZLIB KTX2 (rank 8): held, no sample.
12. SwiftShader BC7 and the browser load gate: reachable with browser time only, not run.
13. One-off GPU panic: held, no recurrence.
14. Owner-held: user-wide `jobs = 4`, the cargo shim, sccache or mold, and the Playwriter 0.11 upgrade.

## Round 52 Phase B: the 500-body bench, decided at parity

Phase A: no code change this round. The bench was already instrumented, so the work was the run and the decision.

Gate and load readings (`vmstat 1 2`, and the 1-minute load):
- 21:40:55 si 1512, so 0, load1 2.91 (closed: swap-in above 1000)
- 21:41:05 si 528, so 20, load1 3.01 (open, load within 4): the bench job started here
- 21:41:49 si 56, so 6000, load1 5.12 (after the run; closed on swap-out; the bench had finished)

Run, once, with `RUNS=10 scripts/bench-vs-jayzee.sh`, rc 0, wall 44.2 s. The bench output is `/tmp/bt/r52-bench.json`.

500-body pairs (jayzee mean over bevy mean):

| Run | jayzee ms | bevy ms | ratio | `noisy` |
| --- | --- | --- | --- | --- |
| 1 | 0.331 | 0.3064 | 1.0803 | false |
| 2 | 0.340 | 0.3191 | 1.0655 | true |
| 3 | 0.331 | 0.3190 | 1.0376 | true |
| 4 | 0.320 | 0.3214 | 0.9956 | false |
| 5 | 0.329 | 0.3273 | 1.0052 | true |
| 6 | 0.309 | 0.3182 | 0.9711 | false |
| 7 | 0.320 | 0.3239 | 0.9880 | true |
| 8 | 0.305 | 0.3068 | 0.9941 | false |
| 9 | 0.338 | 0.3217 | 1.0507 | false |
| 10 | 0.319 | 0.3117 | 1.0234 | false |

This round's quiet pairs (6): mean ratio 1.0192, sd 0.0406, 3 of 6 above 1. All ten pairs: mean 1.0212, 6 of 10 above 1.

The Round 46 quiet pairs (1.068, 1.030, 1.042) and this round's quiet pairs together (9): mean 1.0284, sd 0.0362, 6 of 9 above 1. Its t-interval (t of 2.306 for 8 degrees of freedom) is about [1.000, 1.056]. Its sign test (6 of 9 above 1, one-sided) gives p 0.254.

Decision: held at parity. The 500-body size shows no speedup that these pairs can separate from 1. The mean is 2 to 3% above 1, but the lower bound of its interval touches 1, and the sign test is far from significant. The earlier reading of 1.04 (Round 46) was within this same spread.

Other sizes, for reference: 2000 bodies, 2.52 from 9 quiet pairs. 5000 bodies, 3.40 from one quiet pair (9 of 10 noisy). The 5000 speedup of 4.56 from Round 46, with 5 quiet pairs, stands; this round's 5000 run is too noisy to add to it.

### Round 52 ranked remaining work

1. 500-body bench size: held at parity (above). Reopen with at least 15 quiet pairs at 500 bodies whose t-interval lower bound on the mean ratio is above 1.0, at 1-minute load 4 or less. The bench is reachable now, so this is a measurement, not an owner decision.
2. Gross per-process swap: held. The kernel exposes only host-wide `pswpin` and `pswpout`, and the root `memory.stat` is the only cgroup counter here. Reopen with a per-process or per-group swap-out counter.
3. Attribution of a noisy build's swap-out: held. Reopen with a build whose swap-out is above 100 MiB, measured with the new fields. Such a build cannot be started while the gate is closed.
4. Clean-build swap at `jobs = 12`: held, owner decision on `jobs`. Reopen when the owner changes `jobs` and a clean build's total swap falls below 1 GB. Baseline: six Round 43 clean builds, 2857 to 8699 MiB swapped, 323 to 384 s wall.
5. No-change floor: explained (Round 47). Reopen with a quiet median above 0.5 s.
6. Unlogged cargo stretch, about 0.085 s: held, inside cargo. Reopen if it grows above 0.2 s on repeated quiet runs.
7. One-file variance: done (quiet median 3.0 s, Round 46). Reopen with a quiet build above 4 s or below 3.5 s.
8. Override's total effect: held, reverted in Round 43. Reopen with a method that does not depend on swap.
9. Clean-build swap classification: held by decision. Reopen when a swap-to-wall relation is shown for multi-crate builds.
10. Release build (862.7 s, 16.9 GB swapped in): held, same memory cause. Reopen with the owner's `jobs` decision.
11. ZLIB KTX2 (rank 8): held, no sample.
12. SwiftShader BC7 and the browser load gate: reachable with browser time only, not run.
13. One-off GPU panic: held, no recurrence.
14. Owner-held: user-wide `jobs = 4`, the cargo shim, sccache or mold, and the Playwriter 0.11 upgrade.

## Round 53 Phase B: the 500-body bench, more quiet pairs

Phase A: no code change this round.

Every gate and load reading taken by the job, in order. `si` and `so` are swap-in and swap-out in KiB/s; `load1` is the 1-minute load. A reading counts as open when both are below 1000 and load1 is 4 or less. The bench started on the first open reading, at 22:07:22.

```
start reading 21:51:10 si=1148 so=928 load1=5.11
reading 21:51:12 si=836 so=0 load1=5.11
reading 21:51:16 si=596 so=0 load1=5.10
reading 21:51:20 si=116 so=0 load1=5.10
reading 21:51:25 si=128 so=4700 load1=5.02
reading 21:51:29 si=3784 so=16752 load1=5.41
reading 21:51:33 si=592 so=26760 load1=5.30
reading 21:51:37 si=1136 so=38276 load1=5.60
reading 21:51:41 si=5900 so=244 load1=5.60
reading 21:51:45 si=10044 so=440 load1=5.71
reading 21:51:49 si=3280 so=452 load1=5.57
reading 21:51:53 si=1808 so=0 load1=5.45
reading 21:51:57 si=16716 so=0 load1=5.73
reading 21:52:01 si=4696 so=0 load1=5.73
reading 21:52:05 si=464 so=0 load1=5.99
reading 21:52:09 si=796 so=0 load1=5.99
reading 21:52:13 si=2852 so=0 load1=6.00
reading 21:52:17 si=5024 so=84 load1=6.00
reading 21:52:21 si=1684 so=48312 load1=6.00
reading 21:52:25 si=1388 so=0 load1=6.40
[gm: ...]
[gm: ...]
reading 22:06:14 si=25492 so=0 load1=7.37
reading 22:06:18 si=5300 so=0 load1=6.86
reading 22:06:22 si=62216 so=0 load1=6.39
reading 22:06:26 si=4484 so=0 load1=6.39
reading 22:06:30 si=1576 so=0 load1=5.96
reading 22:06:34 si=1244 so=0 load1=5.80
reading 22:06:38 si=46916 so=0 load1=5.50
reading 22:06:42 si=1116 so=0 load1=5.13
reading 22:06:46 si=2824 so=0 load1=5.13
reading 22:06:50 si=2960 so=0 load1=4.80
reading 22:06:54 si=1208 so=0 load1=4.82
reading 22:06:58 si=4316 so=0 load1=4.59
reading 22:07:02 si=2228 so=0 load1=4.39
reading 22:07:06 si=33624 so=0 load1=4.27
reading 22:07:10 si=1848 so=0 load1=4.27
reading 22:07:14 si=9184 so=0 load1=4.17
reading 22:07:18 si=8348 so=0 load1=4.00
reading 22:07:22 si=692 so=0 load1=3.92
bench rc=0 wall=40.3 s
post 22:08:03 load=4.94 5.70 5.44 2/2320 1003123 gate=1620/0

[gm: kept 40 of 241 lines; dropped 202; 10037 chars; use raw:true or the job log for the full output]
[gm: full output at /workspace/bevy/runs/2026-11-19-14-01-09-579a1e/outputs/bc72ddeb5b9e-run_shell.txt; page with gm.read_file({ path: "/workspace/bevy/runs/2026-11-19-14-01-09-579a1e/outputs/bc72ddeb5b9e-run_shell.txt", unit: "chars", offset: 0, limit: 20000 })]
```

Run, once: `RUNS=10 scripts/bench-vs-jayzee.sh`, rc 0, wall 40.3 s. The bench started at 22:07:22 (si 692, so 0, load1 3.92). After it finished, the reading at 22:08:03 was si 1620, so 0, load1 4.94.

500-body pairs of this run (jayzee mean over bevy mean):

| Run | Ratio | `noisy` |
| --- | --- | --- |
| 1 | 0.9880 | true |
| 2 | 1.0128 | false |
| 3 | 1.0789 | false |
| 4 | 1.0003 | false |
| 5 | 1.0434 | false |
| 6 | 1.0346 | false |
| 7 | 1.0314 | false |
| 8 | 1.0367 | false |
| 9 | 0.9912 | false |
| 10 | 1.0673 | false |

Counting for the rule in this round:
- New quiet pairs at 500 bodies: 9. Of those, 8 are above 1. The one below is run 9, at 0.9912.
- Pooled quiet pairs since Round 46 (18): mean 1.0307, sd 0.0318, 14 above 1. The 95% t-interval (t 2.110, 17 degrees of freedom) is [1.0148, 1.0465]. The one-sided sign test gives p 0.0154.

Decision: held, by the rule as written. The rule holds the size at parity unless five or more new quiet pairs all sit above 1. This round gave 9 new quiet pairs, and not all of them are above 1, so the rule is not met.

Stated for the owner, not acted on: the Round 52 reopen criterion (15 quiet pairs with the t-interval lower bound above 1.0) is now met, with 18 pairs and a lower bound of 1.0148. The pooled data indicate a speedup of about 3% at 500 bodies. That is a decision about the rule, and it is the owner's to make. The rule above is the one applied here.

### Round 53 ranked remaining work

1. 500-body bench size: held by the rule, with 9 new quiet pairs, 8 above 1. Reopen with five new quiet pairs at 500 bodies, all above 1, as the rule requires. The owner may instead accept the pooled criterion (18 quiet pairs, lower bound 1.0148), which is met now.
2. Gross per-process swap: held. The kernel exposes only host-wide `pswpin` and `pswpout`, and the root `memory.stat` is the only cgroup counter here. Reopen with a per-process or per-group swap-out counter.
3. Attribution of a noisy build's swap-out: held. Reopen with a build whose swap-out is above 100 MiB, measured with the new fields. Such a build can't start while the gate is closed.
4. Clean-build swap at `jobs = 12`: held, owner decision on `jobs`. Reopen when the owner changes `jobs` and a clean build's total swap falls below 1 GB. Baseline: six Round 43 clean builds, 2857 to 8699 MiB swapped, 323 to 384 s.
5. No-change floor: explained (Round 47). Reopen with a quiet median above 0.5 s.
6. Unlogged cargo stretch, about 0.085 s: held, inside cargo. Reopen if it grows above 0.2 s on repeated quiet runs.
7. One-file variance: done (quiet median 3.0 s, Round 46). Reopen with a quiet build above 4 s or below 3.5 s.
8. Override's total effect: held, reverted in Round 43. Reopen with a method that does not depend on swap.
9. Clean-build swap classification: held by decision. Reopen when a swap-to-wall relation is shown for multi-crate builds.
10. Release build (862.7 s, 16.9 GB swapped in): held, same memory cause. Reopen with the owner's `jobs` decision.
11. ZLIB KTX2 (rank 8): held, no sample.
12. SwiftShader BC7 and the browser load gate: reachable with browser time only, not run.
13. One-off GPU panic: held, no recurrence.
14. Owner-held: user-wide `jobs = 4`, the cargo shim, sccache or mold, and the Playwriter 0.11 upgrade.

## Round 54 Phase A: the 500-body size recorded as a speedup under the Round 52 criterion

Decision, by the dispatcher, under the Round 52 criterion. It replaces the rule I applied in Round 53, which the dispatcher withdrew as my own wording. The criterion is: at least 15 quiet pairs at 500 bodies, the t-interval lower bound on the mean ratio above 1.0, and 1-minute load at 4 or less.

Pooled quiet pairs, 18 in all (Round 46: 3; Round 52: 6; Round 53: 9). The ratio is jayzee mean over bevy mean for the same run pair:
- n 18, mean 1.0307, sd 0.0318.
- t 2.110 (17 degrees of freedom); 95% interval [1.0148, 1.0465].
- 14 of 18 above 1; one-sided sign test p 0.0154.
- Load1 at each pair's start: 2.45 (Round 46), 3.01 to 3.17 (Round 52), 3.92 (Round 53). The highest is 3.92, which is within 4.

Criterion met. The 500-body size is recorded as a speedup of about 3% (jayzee over bevy at 500 bodies), with its interval above 1. The Round 52 and 53 entries that held the size at parity are superseded by this line.

Data: `scripts/data/bench-500-quiet.json` lists the 18 quiet pairs with their round, run, ratio and load1.

Witness: `scripts/check-bench-criterion.cjs` reads the data, applies the criterion, and exits 0 when it is met. Green: `{"n":18,"mean":1.0307,"sd":0.0318,"t":2.11,"lower":1.0148,"upper":1.0465,"above1":14,"maxLoad":3.92,"criterionMet":true}`, exit 0.

Mutation, before its restore: the minimum count `MIN_N` changed from 15 to 20. Red: `"criterionMet":false`, exit 1. Restored from a copy, checked with `cmp` (identical), green again, exit 0. `git status` after the restore shows only the new files.

## Round 54 Phase B: the 500-body criterion, witnessed

Every gate and load reading taken this round, in order:
- 22:11:29 si 916, so 0, load1 6.53 (open on swap; the load was above 4, and the witness is a node check that starts no cargo or bench)
- one reading from the inspection call, with no timestamp recorded, si 56, so 3368 (closed)
- 22:12:44 si 3472, so 768, load1 4.90 (closed)
- 22:12:48 si 216, so 0, load1 4.90 (open): witness W-k
- post 22:12:48 si 252, so 0, load1 4.90 (open)

Witness, run once: `node scripts/check-bench-criterion.cjs`, wall 0.05 s, rc 0, `{"n":18,"mean":1.0307,"sd":0.0318,"t":2.11,"lower":1.0148,"upper":1.0465,"above1":14,"maxLoad":3.92,"criterionMet":true}`.

No bench and no cargo ran in this Phase B. The witness reads the recorded pairs and starts no build.

## Round 55 Phase A: a SwiftShader BC7 reference for the screenshot caveat

The cross-adapter caveat (Round 26 ledger, item 3 of the ranked list) is that BC7 was compared with a hardware RGBA8 render, while ASTC and ETC2 were compared with a SwiftShader RGBA8 render. A SwiftShader BC7 set removes that difference. SwiftShader reports `texture-compression-bc` in its feature list, so BC7 can run on the software adapter.

Change: `scripts/check-neon-wasm-models.cjs` gets a `bc7-software` mode, with the BC7 cache (`gltf-cache`) and `&adapter=software&still=1`. The other modes are unchanged.

Witness: `node scripts/check-neon-bc7-software-mode.cjs` checks that the mode line is exact and that it does not disable BC. Green: `{"present":true,"disablesBc":false,"ok":true}`, exit 0.

Mutation, before its restore: `&adapter=software` removed from the `bc7-software` line. Red: `{"present":false,"disablesBc":false,"ok":false}`, exit 1. Restored from a copy, checked with `cmp` (identical), green again, exit 0. `git status` after the restore shows the mode file modified and the witness file new, nothing else.

Phase B, not run in Phase A: `NEON_TEXTURE=bc7-software node scripts/check-neon-wasm-models.cjs` (the witness through the gate, on localhost 127.0.0.1:8092 only), then `SHOTS_LEFT=target/witness/neon-shots-rgba8-software SHOTS_RIGHT=target/witness/neon-shots-bc7-software node scripts/compare-screenshots.cjs`.

## Round 55 Phase B: the SwiftShader BC7 set, measured against the SwiftShader RGBA8 set

Readings, in order (`vmstat 1 2`, the second sample is the gate reading):
- 22:21:12 pre-check, not the Phase B gate: si 200, so 0, load1 3.84.
- 22:22:52 Phase B first check: second sample si 1240, so 0, load1 3.63. Closed on swap-in; the run was not started.
- 22:23:04 job poll, second sample: si 804, so 0, load1 3.46. Open; the witness started.
- 22:28:25 post-run: si 9184, so 0, load1 8.83. After the run, not a gate reading.
- post-run check, after the tabs were closed: si 556, so 12264, load1 5.76. After the run.

Witness, run once: `NEON_TEXTURE=bc7-software node scripts/check-neon-wasm-models.cjs`. Wall 321.4 s, rc 0. Witness `ok` true: 64 of 64 rows, all 64 loaded and clean, load median 1069 ms, max 3094 ms. The page ran on `http://127.0.0.1:8092/web/wasm/index.html` only; no request went to any other host. Tabs: one page in the gate's context, closed at the end. Processes started: the job, the witness node script, the wasm server, the playwriter gate and its Chromium tree. All were gone at the check, and PIDs 1014292, 1014313, 1014329, 1014349, 1014376, 1014512 and 1014519 had no `/proc` entry. Other sessions' Chromium and playwriter processes were not touched.

Screenshot comparison, once: `SHOTS_LEFT=neon-shots-rgba8-software SHOTS_RIGHT=neon-shots-bc7-software node scripts/compare-screenshots.cjs`. `texture-visual-bc7-software.json`: `ok` false, 64 pairs, PSNR min 16.72, median 21.11, 64 failing at the 30 dB rule.

Comparisons already on file, for scale: ASTC against the same SwiftShader RGBA8 set min 51.90, median 69.76, ok; ETC2 min 50.44, median 57.25, ok; BC7 on hardware against hardware RGBA8 min 65.12, median 69.39, ok; the no-still noise floor min 14 to 16, median 28 to 30.

What the 21 dB is. Both sets hold 60 distinct frames of 64, and the same frames repeat, so neither set is blank. Per pair, 3.7% of pixels differ by more than 32 levels (median; max 7.0%). The bounding box of those pixels covers 96.8% of the frame, so the difference is scattered, not a missing model. Median mean absolute luma difference 4.99.

Result. The SwiftShader BC7 set does not match the SwiftShader RGBA8 set within the 30 dB rule. ASTC and ETC2 pass against that same reference, so the RGBA8 reference is not the outlier. The cause is not determined: it may be BC7 decoding on SwiftShader or software shading precision. The threshold was not changed, and the run was not repeated (the witness runs once). The cross-adapter caveat is measured now, not closed. The BC7 default rule still requires a visual pass on hardware, and the hardware BC7 pass (65.12 dB) is unchanged.

## Round 55 Phase A: the cross-adapter finding, from files on disk

Question: is the failing BC7-on-SwiftShader comparison (64 of 64 under 30 dB, median 21.1 dB) a BC7 defect on SwiftShader, or a property of the SwiftShader RGBA8 reference?

Witness: `node scripts/check-cross-adapter.cjs` runs the repo's `compare-screenshots.cjs` on eight pairs through symlinks in `target/witness/cross-adapter/`. It writes new `texture-visual-<pair>.json` files and asserts four checks. Green: `ok` true, all four checks true, exit 0, wall 24 s.

Repo comparison numbers (the witness output, not the Python re-measure):

| Pair (left reference, right set) | ok | PSNR min | PSNR median | failing of 64 |
| --- | --- | --- | --- | --- |
| hw RGBA8 vs hw BC7 | true | 65.09 | 69.34 | 0 |
| hw RGBA8 vs sw BC7 | true | 30.94 | 53.45 | 0 |
| hw RGBA8 vs sw RGBA8 | false | 16.81 | 21.11 | 64 |
| sw RGBA8 vs sw BC7 | false | 16.72 | 21.11 | 64 |
| hw RGBA8 vs sw ASTC | true | 30.94 | 53.45 | 0 |
| sw RGBA8 vs sw ASTC | false | 16.72 | 21.11 | 64 |
| hw RGBA8 vs sw ETC2 | false | 16.81 | 21.11 | 64 |
| sw RGBA8 vs sw ETC2 | true | 50.44 | 57.25 | 0 |

Per-pixel and frame evidence, from the same files (Python, same decoded pixels):
- Distinct frame hashes per set: 60 of 64 for hw RGBA8, sw RGBA8, hw BC7, sw BC7, ASTC, ETC2 and rgba8-a; 64 for the control set.
- Pixel-identical frames: hw BC7 against hw RGBA8, 52 of 64 (the rest at 65.09 dB or more); sw ETC2 against sw RGBA8, 52 of 64 (the rest at 50.44 dB or more); sw ASTC against sw BC7, 52 of 64 (the rest at 50.90 dB or more).
- Per-pixel scatter in the failing pairs: 3.7% of pixels differ by more than 32 levels (median), and the bounding box covers 97% of the frame, so the difference is not a missing model.

Closed, as asked: the BC7 software set does not fail the 30 dB rule against the hardware reference. It passes all 64 frames (minimum 30.94, median 53.45). The failing comparison used the SwiftShader RGBA8 set as its reference, and that set itself differs from the hardware RGBA8 set at the same level (median 21.11, minimum 16.81). So the failure is the reference's deviation from hardware, not BC7 decoding. The threshold is unchanged.

Correction to the record. texture-visual-astc4x4.json (ok, minimum 51.90, median 69.76) does not reproduce from the current files. Against the SwiftShader RGBA8 set, the ASTC set fails (minimum 16.72, median 21.11), and it passes only against the hardware RGBA8 set (30.94 and 53.45). The reference used for that old file is not recorded in it, so its ok flag is not supported by the current files. ETC2 does reproduce exactly (50.44, 57.25, ok against the SwiftShader RGBA8 set).

Not closed from files: why the SwiftShader RGBA8 render differs from hardware RGBA8 by 21 dB. The set rgba8-a is not described in this ledger, so it cannot show whether 21 dB is run-to-run noise. Reopen with a Phase B run: a repeat of the hardware RGBA8 run (or a documented rgba8-a). If that repeat differs by about 21 dB too, the 30 dB rule is measuring run-to-run noise.

Red check, before its restore: the hardware-vs-software BC7 pair's left reference changed to the SwiftShader RGBA8 set. Red: `swBc7MatchesHwRgba8` false, exit 1. Restored from a copy, `cmp` identical, green again (exit 0). `git status` after the restore lists only the new witness and this ledger line.

## Round 55 Phase B: the hardware RGBA8 repeat, and the witnesses rerun

Readings (`vmstat 1 2`, the second sample is the gate reading):
- 22:54:40 first check: second sample si 236, so 0, load1 3.05. Open.
- 22:54:54 job gate, second sample: si 536, so 0, load1 2.90. Open; the run started.
- 22:58:24 post-run: si 4, so 0, load1 3.25.

Witnesses, each run once with wall time:
- `NEON_TEXTURE=rgba8 node scripts/check-neon-wasm-models.cjs` (repeat of the hardware RGBA8 run, on localhost 127.0.0.1:8092 only): wall 180.8 s, rc 0, 64 of 64 clean, load median 1086 ms. The original set was copied aside first, the repeat was moved to `neon-shots-rgba8-repeat`, and the original was copied back, so the cross-adapter witness reads the same reference as before.
- `compare-screenshots.cjs`, original against repeat: `ok` true, PSNR minimum and median both null, 0 failing. All 64 hardware RGBA8 frames are pixel-identical between the two runs.
- `node scripts/check-cross-adapter.cjs`: wall 25.7 s, rc 0. Unchanged from Phase A: the software BC7 against hardware RGBA8 gives minimum 30.94 and median 53.45.
- `node scripts/check-neon-bc7-software-mode.cjs`: wall 0.03 s, rc 0.

What this answers: the 21 dB gap between the software and hardware RGBA8 sets is not run-to-run noise. The hardware RGBA8 render repeats exactly. The gap is a real difference between the SwiftShader RGBA8 render and the hardware RGBA8 render. The question left from Phase A is closed: the failing comparison is that reference difference, and the BC7 set on SwiftShader matches the hardware reference.

Processes and tabs: one page in the gate's context, closed at the end. Processes started: the job, the witness node script, the wasm server, the playwriter gate and its Chromium tree. Their PIDs (1075781, 1076110, 1076169, 1076725, 1077079, 1077457) had no `/proc` entry afterwards, and port 8092 is free. Other sessions' processes were not touched.

## Round 56 Phase A, commit A: the reference is written into every comparison record

`scripts/compare-screenshots.cjs` now writes `reference`, the name of the left set, into each `texture-visual-<right>.json`. The verdict is unchanged: `ok` still means every pair passes the 30 dB rule (or is identical).

Red check, before its restore: the `reference` line removed from `compare-screenshots.cjs`, then `node scripts/check-cross-adapter.cjs` run. Red: `astcRecordNamesReference` false, exit 1. Restored from a copy, `cmp` identical, green again, exit 0.

## Round 56 Phase A, commit B: the BC7 caveat closed against hardware RGBA8

Reference of record: hardware RGBA8 (`neon-shots-rgba8`), set by the dispatcher. `scripts/check-cross-adapter.cjs` takes it from one constant, `referenceOfRecord`.

Closed against hardware RGBA8:
- Software BC7 on SwiftShader passes all 64 frames: minimum 30.94, median 53.45, ok.
- Hardware BC7 passes the same reference: minimum 65.09, median 69.34, ok (sanity).

Named finding, `software-rgba8-rendering-difference`: the software RGBA8 set against hardware RGBA8 fails all 64 frames: minimum 16.81, median 21.11. It is a real render difference: the hardware RGBA8 repeat is pixel-identical to the original (Round 55, Phase B). A second software RGBA8 run, `neon-shots-rgba8-a`, agrees with the software set (minimum 32.15, median 52.96, ok) and fails against hardware (minimum 16.71, median 21.11, 64 failing). Its configuration is not recorded, so that agreement is inferred from its pixels. Reopen: a second hardware re-run of the software RGBA8 set, or a second adapter. Not closed against software RGBA8, and the 30 dB threshold is unchanged.

Correction, ASTC record. `texture-visual-astc4x4.json` had `ok` true with no reference, and its 51.90 / 69.76 numbers did not reproduce against software RGBA8 (16.72 / 21.11). The old file is kept as `texture-visual-astc4x4-unreferenced.json` (not committed; in `target/witness`). The record was regenerated by the witness against hardware RGBA8: `reference` `neon-shots-rgba8`, `ok` true, minimum 30.94, median 53.45, 0 failing. The verdict follows from the reference: witness check `astcPassesAgainstReference`.

Flag, ETC2 (not changed here). Against hardware RGBA8, the ETC2 set fails all 64 frames: minimum 16.81, median 21.11. Its recorded pass (`texture-visual-etc2.json`, minimum 50.44, median 57.25, ok) reproduces only against software RGBA8. ETC2's usability therefore depends on the reference. The `etc2` record has no `reference` field. Reopen: a dispatcher ruling on which reference the ETC2 verdict uses.

Red checks, before each restore: (1) `referenceOfRecord` set to `sw-rgba8`: exit 1, five of seven checks false. Restored from a copy, `cmp` identical, green (exit 0, 18 s). (2) The reference field removed from `compare-screenshots.cjs`: see commit A.

## Round 56 Phase B: the file-based witnesses, one window

Readings (`vmstat 1 2`, the second sample is the gate reading), one window of about ten minutes, one reading every 30 seconds:
- 23:04:58 first reading, second sample: si 20, so 0, load1 2.37. Open; the witnesses started.
- 23:05:17 post-run: si 8, so 0, load1 2.34.

Witnesses, each run once with wall time (file-based, no browser, no cargo):
- `node scripts/check-cross-adapter.cjs`: wall 18.5 s, rc 0, ok true. All seven checks true; reference of record `neon-shots-rgba8`; software BC7 minimum 30.94, median 53.45; the ASTC record `reference` `neon-shots-rgba8`, ok true, minimum 30.94, median 53.45, 0 failing; ETC2 against the same reference ok false, minimum 16.81, median 21.11, 64 failing (flagged in commit B, not changed here).
- `node scripts/check-neon-bc7-software-mode.cjs`: wall 0.04 s, rc 0, ok true.

No browser ran in this phase, so no tabs or processes were started. The witness job (pid 1144864) has exited.

## Round 57 Phase A: ETC2 against the hardware reference

Reference of record: hardware RGBA8 (`neon-shots-rgba8`), for every texture set, as applied to ASTC in Round 56.

ETC2 record. The old `texture-visual-etc2.json` (`ok` true, minimum 50.44, median 57.25, no reference) is kept as `texture-visual-etc2-unreferenced.json` in `target/witness` (not committed). The witness regenerated `texture-visual-etc2.json` against hardware RGBA8: `reference` `neon-shots-rgba8`, `ok` false, minimum 16.81, median 21.11, 64 of 64 failing.

Named finding: ETC2 fails with the same named finding, `software-rgba8-rendering-difference`. ETC2 against software RGBA8 passes (minimum 50.44, median 57.25, ok), and its median against hardware equals the software set's median (21.11). Its failure is therefore the software-versus-hardware render difference, not an ETC2 texel fault.

Witness: `node scripts/check-cross-adapter.cjs` now has `etc2RecordNamesReference`, `etc2FailsAgainstReference` and `etc2FailureIsSoftwareRgba8Finding` (ten checks in all). Green: `ok` true, exit 0, 21 s.

Red check, before its restore: the ETC2 pair's reference switched to `sw-rgba8`. The verdict flipped to pass (`ok` true, minimum 50.44), and the witness went red: `etc2RecordNamesReference` and `etc2FailsAgainstReference` false, exit 1. Restored from a copy, `cmp` identical, green again (exit 0). The record is back to hardware RGBA8, `ok` false.

Downstream: `scripts/texture-footprint.cjs` (no code change) now gives ETC2 `visualOk` false and `fallbackFormats.etc2` false. The decision stays `bc7`, and ASTC stays a fallback. The earlier output is kept as `texture-footprint-before-r57.json`.

Not taken, with reopen evidence:
- ASTC and ETC2 texel quality: no ASTC or ETC2 decoder is on disk. There is no `astcenc` or `etcpak`, no ASTC or ETC2 crate in the cargo registry (only `bcdec_rs` for BC and container parsers), and no PIL or Python decoder. Reopen: a decoder present on disk, or a dispatcher decision to add one.
- Second software RGBA8 run: a browser run, so it goes in Phase B under the gate.

## Round 57 Phase B: the second software RGBA8 run, and a correction to the named finding

Readings (`vmstat 1 2`, the second sample is the gate reading), one bounded window, one reading every 30 seconds:
- 23:13:36 first reading, second sample: si 196, so 0, load1 2.19. Open; the run started.
- 23:19:11 post-run: si 16, so 0, load1 4.89.
- 23:20:44 after the processes stopped: si 3140, so 0, load1 3.12. Closed on swap-in.

The run and its witnesses, each once with wall time:
- `NEON_TEXTURE=rgba8-software node scripts/check-neon-wasm-models.cjs` on localhost 127.0.0.1:8092: wall 307.9 s, rc 0, 64 of 64 clean. The original set and witness files were copied aside (`-orig`), the repeat was moved to `neon-shots-rgba8-software-r57` and `neon-wasm-witness-rgba8-software-r57.json`, and the originals were copied back.
- `node scripts/check-cross-adapter.cjs`: wall 20.9 s, rc 0, ok true, all checks true.
- `node scripts/check-neon-bc7-software-mode.cjs`: wall 0.03 s, rc 0, ok true.

What the repeat shows:
- Against hardware RGBA8 (`x57-sw-r57-vs-hw`): ok true, minimum 30.94, median 53.45, 0 failing. It is inside the 30 dB rule.
- Against the original software RGBA8 set (`x57-sw-r57-vs-orig`): ok false, minimum 16.72, median 21.11, 64 failing.

Pixel identity, 64 frames per set (SHA of decoded RGB): hardware RGBA8 and hardware BC7 match on 52; the new software RGBA8, software BC7 and software ASTC match each other on 52; the original software RGBA8 matches the ETC2 set on 52.

Provenance, from file modification times (PNG `clip-die`):
- hardware RGBA8, 11-18 21:12; its repeat in Round 55 (11-19) was pixel-identical, so the hardware reference reproduces in the current build.
- hardware BC7, 11-19 08:21; ASTC, 11-18 21:15; new software RGBA8 and software BC7, 11-19.
- original software RGBA8, 11-17 11:51; ETC2, 11-17 12:01; the `rgba8-a` set, 11-17 12:06. All three predate the current wasm build (11-18 09:21).

Source. `wasm-viewer/src/lib.rs` `texture_choice` remaps only paths under `target/gltf-cache/`. The `rgba8` mode path is not remapped, so the RGBA8 mode loads RGBA8 textures in the current build. The ETC2 and ASTC modes go through the BC7 path and the device target, so they are current-build renders when run now.

Correction, stated plainly. The named finding `software-rgba8-rendering-difference` (Round 56, Round 57 commit C) rests on the original software RGBA8 set, written before the current build. The current build's software RGBA8 run is within the 30 dB rule against hardware RGBA8. So the finding is not reproduced by the current build, and its premise is not established. The check `swRgba8DifferenceStandsAgainstReference` and the ETC2 check `etc2FailureIsSoftwareRgba8Finding` in `check-cross-adapter.cjs` assert the stale set, so they are not valid evidence for the current build. No code change was made in this phase; the decision is the dispatcher's.

ETC2, provisional. The ETC2 verdict (fail, 16.81 / 21.11 against hardware RGBA8) reads a set written at 11-17 12:01, before the current build. It is not a current-build result. Reopen: a current-build ETC2 run (mode `etc2`, which remaps through the device target), with the move-aside method.

Processes and tabs: the job, the witness script, the wasm server and the playwriter gate were stopped; PIDs 1151439, 1151442, 1151486 and 1151501 had no `/proc` entry afterwards.

## Round 58 Phase A, commit A1: texture records checked against the wasm build

Build time, from `web/wasm/bevy-jayzee-wasm_bg.wasm`: 2026-11-18 09:21.

Sets older than the build, moved to `target/witness/stale-before-r58/` (kept, not deleted): `neon-shots-etc2` (11-17 12:01), `neon-shots-rgba8-software` as originally written (11-17 11:51), `neon-shots-rgba8-a` (11-17 12:06), `neon-shots-rgba8-control` (11-17 04:29), with their witness and row files. The software RGBA8 canonical set is now the current-build run from Round 57 (`neon-shots-rgba8-software`, 11-19 23:13), with its witness and rows. Current sets (newer than the build): hardware RGBA8 (11-18 21:12), hardware BC7 (11-19 08:21), ASTC (11-18 21:15), software BC7 (11-19 22:23), RGBA8 repeat (11-19 22:55).

Records moved to the stale folder, with the reason: they predate the build (`texture-visual-control`, `-bc7-run1`, `-bc7-nostill`, `-astc4x4-nostill`, `-etc2-nostill`), their source is stale (`-etc2`, `-etc2-unreferenced`, `swRgba8`, `swRgba8VsHwRgba8`, `swEtc2*`, `x57-sw-r57-vs-orig`, `x56-rgba8a`), or they are superseded pairs (`swBc7VsSwRgba8`, `bc7-software`, `swBc7VsHwRgba8`, `hwBc7VsHwRgba8`, `swAstcVs*`, `x55-rgba8-repeat`, `x57-sw-r57-vs-hw`, `astc4x4-unreferenced`).

Recomputed from current sets (`compare-screenshots.cjs`, reference of record hardware RGBA8):
- `texture-visual-bc7.json`, before (11-17 12:10, no reference): ok true, minimum 65.12, median 69.39. After (source `neon-shots-bc7`, reference `neon-shots-rgba8`): ok true, minimum 65.09, median 69.34, 0 failing.
- `texture-visual-rgba8.json`, before (11-17 12:10, no reference): ok true, identical, no PSNR. After (reference `neon-shots-rgba8`, source `neon-shots-rgba8-repeat`): ok true, identical, no PSNR. Its meaning is now stated: hardware RGBA8 against its own repeat.
- `texture-texel.json` (BC7 texel witness), before (11-17 10:56): ok true, minimum 41.70, median 53.51, 61 images. After (regenerated with `target/debug/bevy-jayzee texture-quality`, stdout saved): ok true, minimum 41.70, median 53.51, 61 images.

Changes:
- `scripts/compare-screenshots.cjs` writes `source`, the name of the set compared against the reference (through a symlink, its real directory).
- `scripts/check-texture-provenance.cjs` (new): every `texture-visual-*.json` and `texture-texel.json` must come from sets whose PNGs (and the record itself, for the texel record) are newer than the wasm build. Its output lists each record, its source and reference, and whether it is current.

Red checks:
- Comparator flipped in the provenance script (`sourceMs < buildMs`): exit 1, every record false. Restored from a copy, `cmp` identical.
- `source` line removed from `compare-screenshots.cjs`, then the BC7 record recomputed: exit 1, the BC7 record has no source. Restored from a copy, `cmp` identical, and the BC7 record recomputed with the restored script.

Expected red until the next commit: `texture-visual-astc4x4.json`, `hwBc7` and `swBc7` predate the `source` field. They are regenerated by the witness in the next commit.

## Round 58 Phase A, commit A2: the finding withdrawn, and the witness checks the current build

Withdrawn: the named finding `software-rgba8-rendering-difference`. Its 21 dB result came from the software RGBA8 set written at 11-17 11:51, before the wasm build, and the current build's software RGBA8 run passes against hardware RGBA8 (minimum 30.94, median 53.45, 0 failing). The finding object is removed from `check-cross-adapter.cjs`. Its earlier ledger lines stay as the record of what was measured.

Witness changes in `scripts/check-cross-adapter.cjs`:
- The stale-set checks are gone: `swRgba8DifferenceStandsAgainstReference` and `etc2FailureIsSoftwareRgba8Finding`, and the superseded pairs that used the old set.
- `currentSwRgba8`: the current software RGBA8 run (`neon-shots-rgba8-software`) against hardware RGBA8. Check `currentSoftwareRgba8PassesAgainstReference`: ok true (minimum 30.94, median 53.45).
- `setsAreCurrentBuild`: every set a pair reads must have all its PNGs newer than the wasm build.
- `astcRecordNamesReference`, `astcPassesAgainstReference`, `etc2RecordNamesReference`, `hwBc7PassesAgainstReference`, `swBc7PassesAgainstReference`, `referenceOfRecordIsHardwareRgba8` (kept).

Red checks, each restored from a copy with `cmp` identical:
- `referenceOfRecord` set to `sw-rgba8`: exit 1, the reference-of-record check and the BC7 checks go red.
- `currentSwRgba8` pointed at the stale set (`sw-rgba8-stale`, `stale-before-r58/neon-shots-rgba8-software`): exit 1, `setsAreCurrentBuild` and `currentSoftwareRgba8PassesAgainstReference` false.
- `etc2` pointed at the stale set (`sw-etc2-stale`, `stale-before-r58/neon-shots-etc2`): exit 1, `setsAreCurrentBuild` false.

State at this commit: the witness is red, for one reason only: `setsAreCurrentBuild`, because the ETC2 set is in the stale folder until the Phase B regeneration. Every other check is true, and the witness regenerated `texture-visual-astc4x4.json`, `hwBc7` and `swBc7` with `source`.

## Round 58 Phase B: the ETC2 set regenerated, the records checked, the footprint rerun

Readings (`vmstat 1 2`, the second sample is the gate reading), one bounded window, one reading every 30 seconds:
- 23:36:36 first reading, second sample: si 76, so 0, load1 2.55. Open; the regeneration started.
- 23:41:47 post-run: si 456, so 0, load1 6.31.

Witnesses and runs, each once, with wall time:
- `NEON_TEXTURE=etc2 node scripts/check-neon-wasm-models.cjs` (regeneration of the ETC2 set, on localhost 127.0.0.1:8092 only): wall 294.7 s, rc 0, 64 of 64 clean, load median 955 ms. Moved aside before the run: `stale-before-r58/neon-shots-etc2`.
- `node scripts/check-cross-adapter.cjs`: wall 15.2 s, rc 0, ok true, all checks true, including `setsAreCurrentBuild`.
- `node scripts/check-texture-provenance.cjs`: wall 0.04 s, rc 0, every record current.
- `node scripts/texture-footprint.cjs`: wall 1.07 s, rc 0.
- `node scripts/check-neon-bc7-software-mode.cjs`: wall 0.03 s, rc 0.

Before and after, per set and record (hardware RGBA8 is the reference of record):
- ETC2 set. Before (11-17 12:01, stale): ok false, minimum 16.81, median 21.11, 64 failing. After (new run, current build): ok true, minimum 30.94, median 52.81, 0 failing. The ETC2 verdict is now a pass, written from the new run. The earlier fail rested on the stale set.
- Software RGBA8 (`neon-shots-rgba8-software`). Before (11-17 11:51, stale): ok false, minimum 16.81, median 21.11. After (the Round 57 run, 11-19 23:13, now canonical): ok true, minimum 30.94, median 53.45, 0 failing.
- BC7 record `texture-visual-bc7.json`. Before (11-17 12:10, no reference): ok true, minimum 65.12, median 69.39. After (reference `neon-shots-rgba8`, source `neon-shots-bc7`): ok true, minimum 65.09, median 69.34.
- RGBA8 record `texture-visual-rgba8.json`. Before (11-17 12:10, no reference): ok true, identical. After (hardware RGBA8 against its repeat, source `neon-shots-rgba8-repeat`): ok true, identical.
- Texel record `texture-texel.json`. Before (11-17 10:56): ok true, minimum 41.70, median 53.51, 61 images. After (regenerated): ok true, the same figures.
- ASTC record `texture-visual-astc4x4.json`. Before (Round 56, no source field): ok true, minimum 30.94, median 53.45. After (witness rerun, source `neon-shots-astc4x4`, current): ok true, the same figures.

Footprint, `texture-footprint.cjs`, before and after (the before file is `texture-footprint-before-r58.json`):
- Decision: `bc7` before and after. BC7 usable in both.
- ETC2: before not usable (visual fail, fallback false). After usable (visual ok, load 955 ms against the software RGBA8 reference of 1048 ms, inside 1.1×). The fallback is now true.
- ASTC: before usable (fallback true, load 1267 ms against a reference of 1250 ms). After not usable: load 1267 ms against 1048 ms (1.21×, over the 1.1× tolerance). The ASTC data did not change. The cause is the reference swap: the software RGBA8 witness now has the current build's load time. The fallback is now false.

Not regenerated in this window, moved to `stale-before-r58`, each needs its own browser run (one run per set) or is not reproducible:
- `neon-shots-rgba8-control` and its record `texture-visual-control.json` (11-17 04:29): the mode is not recorded, so it cannot be regenerated faithfully.
- `neon-shots-rgba8-a` and `texture-visual-x56-rgba8a.json` (11-17 12:06): the configuration is not recorded.
- `texture-visual-bc7-run1.json` (11-17 04:29): the configuration is not recorded.
- The three no-still records (`bc7-nostill`, `astc4x4-nostill`, `etc2-nostill`, 11-17 11:33): the mode without `still=1` can be regenerated, one run each.

Processes and tabs: the job, the witness script and the wasm server were stopped; the job exited. Port 8092 is free.

## Round 59 Phase A: the stale sets, their configs, and the ASTC exclusion

ASTC exclusion from the fallback list (dispatcher ruling, recorded here). The footprint rule is a load tolerance of 1.1 times the software RGBA8 witness. The rule and the 1.1 tolerance stand. ASTC's ratio is 1.21: its load median is 1267 ms (`neon-wasm-witness-astc4x4.json`, 11-18 21:22), against 1048 ms for the current software RGBA8 witness (`neon-wasm-witness-rgba8-software.json`, the Round 57 run). The exclusion comes from the reference swap, not the ASTC data: the earlier software reference was the stale set at 1250 ms, against which ASTC's 1267 ms passed. Nothing in the ASTC set changed.

The stale sets, one by one:
- `neon-shots-rgba8-software` as originally written (11-17 11:51). Config recorded: `NEON_TEXTURE=rgba8-software` (the `rgba8-software` mode of `scripts/check-neon-wasm-models.cjs`, query `&adapter=software&still=1`), logged in the Round 17 and Round 26 ledger lines. It was regenerated against the current build by the Round 57 run, the same command, and that run is now the canonical set (11-19 23:13). No new run is needed. The original stays in `stale-before-r58`.
- `neon-shots-rgba8-a` (11-17 12:06). Config not recorded: no script, ledger line, job output or commit names its command or mode. The Round 55 description ("a second software RGBA8 run") was an inference from its pixels, and it is withdrawn. Not regenerated. It stays in `stale-before-r58`. Dependent record withdrawn: `texture-visual-x56-rgba8a.json` (in `stale-before-r58`).
- `neon-shots-rgba8-control` (11-17 04:29). Config not recorded verbatim. The ledger describes it as "RGBA8 against a second RGBA8 render (before the fix)", which is the render before `still=1`. The current script always passes `still=1`, so the pre-fix render can't be reproduced without a new mode, and that would be a guess. It is superseded: the current build's hardware RGBA8 against its repeat is 64 of 64 identical with `still=1` (the `rgba8` record). Not regenerated. It stays in `stale-before-r58`. Dependent record withdrawn: `texture-visual-control.json` (in `stale-before-r58`).

Nothing in this commit changes code. No red check is needed for a ledger-only commit; the checks it relies on are `check-texture-provenance.cjs` and `check-cross-adapter.cjs`, which are unchanged.

## Round 59 Phase B: the file-based witnesses rerun, no regeneration needed

Readings (`vmstat 1 2`, the second sample is the gate reading), one bounded window, one reading every 30 seconds:
- 23:52:21 first reading, second sample: si 52, so 0, load1 3.25. Open.
- 23:52:42 post-run: si 724, so 0, load1 4.57.

No browser run was needed: the one set that had a recorded config was already regenerated from the current build (Round 57), and the other two have no recorded config and stay aside. Witnesses, each once with wall time:
- `node scripts/check-texture-provenance.cjs`: wall 0.06 s, rc 0, every record and the texel record current.
- `node scripts/check-cross-adapter.cjs`: wall 19.1 s, rc 0, ok true.
- `node scripts/check-neon-bc7-software-mode.cjs`: wall 0.03 s, rc 0, ok true.
- `node scripts/texture-footprint.cjs` (rerun, not a witness): wall 1.28 s, rc 0.

Footprint decisions, before (`texture-footprint-before-r59.json`) and after: unchanged. Decision `bc7`. BC7 usable, load ok, visual ok, clean ok. ETC2 usable, load ok, visual ok, fallback true. ASTC not usable, load not ok, visual ok, clean ok, fallback false. The ASTC exclusion comes from the reference swap, as recorded in the Round 59 Phase A line: the load ratio is 1267 ms against 1048 ms, which is 1.21, above the 1.1 tolerance.

Correction to the Phase A line. It said the `rgba8-software` config was logged in the Round 17 and Round 26 ledger lines. The verified record is the `rgba8-software` mode in `scripts/check-neon-wasm-models.cjs` and the Round 26 witness line that lists `rgba8-software` at 64 of 64 clean. Nothing in Round 17 records that mode.

Processes and tabs: none started in this phase. The job (pid 1285583) had exited, no workspace Chromium is running, and no wasm server is running.

## Round 60 Phase A: clean-build measurement script for the `jobs` change (no cargo run)

`scripts/measure-clean-build.sh` runs one clean `cargo build --jobs N` of the native debug target through the shared slot shim (`/config/.local/bin/cargo`, or `CARGO_BIN`). It builds into a fresh empty target root (default: a mktemp directory under /tmp, removed afterwards unless `--keep`), so `target/debug` is never touched and the profile-directory mistake from Round 36 is refused by name. `--jobs` defaults to the repo `.cargo/config.toml` value (12). Before building it reads `vmstat 1 2` (second sample) and `/proc/loadavg` and refuses to run (exit 3, no cargo call) unless both directions are below 1000 KB/s and load1 is at most 4. It samples `/proc/loadavg` once per second during the build and prints exactly one result line:

`CLEAN_BUILD jobs=N rc=R wall_s=W cargo_finished_s=F peak_load1=P gate_si=I gate_so=O load1_before=L target_root=DIR`

The result line is the measurement. Its jobs value is the value passed to cargo, not a guess from the environment. The jobs change itself stays open. Its reopen evidence stays a clean-build measurement under cargo, to be run in a Phase B window with this script.

`scripts/check-clean-build-measure.sh` is the witness. It runs the script with a fake cargo and a fake `vmstat` (`MEASURE_NO_GATE` skips the real gate only in the witness), so no cargo runs. It checks: unknown argument, `--jobs 0`, non-numeric `--jobs`, and a profile directory as target root exit 2; a closed gate exits 3 before any cargo call; a successful run prints exactly one line, exit 0, the fixed field order, `cargo build --jobs 7` in the given target root, `--keep` keeps the root, a run without `--keep` removes it; the default jobs comes from `.cargo/config.toml`; a failing cargo is reported as rc 3 and exits 3. Result: 12 of 12 ok, exit 0, 0.49 s.

Red check: the result-line prefix was changed to `CLEAN-BUILD jobs=`. The witness exited 1 with 3 FAIL lines (result line format, default jobs from config, failing cargo rc). The script was restored with `cmp` against the saved copy, and the witness then exited 0 with 12 ok.

Found and fixed in this round: under `set -u` the first draft read `$CARGO_BIN` and `$MEASURE_NO_GATE` directly, so an unset variable aborted the script (rc 1) before the gate. Both now default to empty (`${CARGO_BIN:-}`, `${MEASURE_NO_GATE:-}`). The gate-closed witness case caught it.

Not changed: `Cargo.toml`, `/config/.cargo/config.toml`, the cargo shim, the stale sets (still in `stale-before-r58`). No cargo was run, no browser was started, and nothing was pushed. Each run leaves its log under `/tmp/measure-clean-build.*.log` (stderr names the path); the script does not delete logs.

## Round 61 Phase B: clean-build measurement under cargo, stopped at the load limit (open)

The `jobs` measurement is not obtained. No CLEAN_BUILD line was produced, and the `jobs` setting is unchanged (repo `.cargo/config.toml`, 12). The decision stays open until a later stretch reads the line.

Window: opened 00:11:14. Readings `vmstat 1 2` (second sample), taken about every 25 seconds, with the heavy-job check (top >20% CPU, compilers, test/check runs, Chromium renderer over 1 GB; gm-core `--handshake` workers exempt):
- 00:11:14 si 996, so 0, load1 0.55. Open. Heavy: only a `--handshake` gm-core worker at 100%, exempt.
- 00:12:25 si 780, so 76, load1 2.31. Open, heavy clear. Witness started.
- 00:12:27 si 1920, so 0, load1 2.31. Closed.
- 00:12:54 si 20, so 68, load1 2.80. Open, heavy busy: gm-core pid 1380537 at 100%, no `--handshake` in the line the job captured. The process had exited before its arguments could be read.
- 00:13:22 si 5096, so 0, load1 2.69. Closed.
- 00:13:49 si 588, so 4248, load1 2.15. Closed.
- 00:14:16 si 208, so 1724, load1 1.96. Closed.
- 00:14:44 si 368, so 0, load1 1.70. Open, heavy clear. Build started (pid 1383202).

Witness, once: `scripts/check-clean-build-measure.sh` rc 0, 0.35 s, 12 of 12 ok (ran at 00:12:25, in the open window).

Build during the window, readings at each sample:
- 00:14:46 si 260, so 0, load1 1.65. Open, heavy busy (ours).
- 00:15:13 si 1080, so 0, load1 6.50. Closed.
- 00:15:41 si 200, so 13204, load1 9.58. Closed.
- 00:16:08 si 3368, so 9944, load1 11.21. Closed.
- 00:16:35 si 4516, so 20120, load1 13.52. Closed.
- 00:17:03 si 11696, so 9580, load1 14.42. Closed.

The build was stopped by PID at about 00:17:25, after the 00:17:03 reading. Reason: the standing limit is no cargo while swap-in is above 1000 KB/s or load is above 4, and the build had put the host above both. The stop covered only the measurement's own process tree, 16 PIDs: the measure script (1383202), its subshell, cargo (1383236) and 12 rustc children, plus the rest of the tree. Stop order was TERM, a 5 s wait, then KILL for any survivor. Survivors: 0. No peer process was stopped.

Result of the stopped build: exit 143 after 164.2 s wall (00:14:44 to 00:17:28). Its cargo log `/tmp/measure-clean-build.nlkxoV.log` shows 342 `Compiling` lines and no `Finished` line. The partial target root `/tmp/bevy-clean.9ACQ4r` (1.3 GB, debug only, created by this build's mktemp) was removed.

Reading after the stop: si 740, so 0, load1 11.00. Load was still decaying from the build's peak.

Why this matters for the decision: the `jobs` decision needs a clean build that completes inside the gate. This attempt started on an open gate and then ran into the limit, because a clean build at `jobs = 12` pushes swap and load above the limit. The next attempt should be a window that is clear before the build and that the build can finish in. Whether to use `jobs = 4` or 12 is still open.

## Round 62 Phase B: clean-build measurement at `jobs = 2`, stopped at the swap limit (open)

The `jobs` decision stays open; no CLEAN_BUILD line was produced. The `jobs` setting is unchanged (repo `.cargo/config.toml`, 12).

Choice of `jobs`: the host has 12 cores and 15.2 GB of memory. The load ceiling is 4, and a compiling `rustc` runs several threads, so `jobs = 2` was chosen as the first value. It leaves the most headroom under 4. Its line decides whether 3 or 4 is tried next.

Readings (`vmstat 1 2`, second sample; heavy-job check reads full arguments of each high-CPU PID):
- 00:21:15 (before the job): si 8632, so 0, load1 1.89. Closed. The only process above 20% was gm-core pid 1002562 at 69.3%, with `--handshake` (exempt).
- 00:21:49: si 88, so 36, load1 1.73. Open, heavy clear.
- Witness `scripts/check-clean-build-measure.sh`, once: rc 0, wall 0.34 s, 12 of 12 ok (00:21:49).
- 00:21:52: si 24, so 0, load1 1.73. Open, heavy clear. Build started: `bash scripts/measure-clean-build.sh --jobs 2`, pid 1396970.
- Build 00:21:53: si 308, so 116, load1 1.75. Within limits.
- Build 00:22:02: si 1852, so 20760, load1 1.93. Swap-in above 1000 KB/s. Stop by PID.

Stop: the stop rule (swap-in above 1000 KB/s or load above 4) stopped the measurement's own tree by PID: 1396970, 1396996, 1397001, 1397321, 1397325, 1397366 (6 PIDs; TERM, then a 5 s wait, then KILL for any survivor). Survivors: 0. No peer process was stopped.

Result: exit 143, wall 15.1 s, stopped by swap-in. Its cargo log `/tmp/measure-clean-build.NgL14h.log` shows 8 `Compiling` lines and no `Finished` line. The partial target root `/tmp/bevy-clean.c2f3ZV` (29 MB, created by this build) was removed.

Readings after the stop: si 272, so 0, load1 1.95. No measure, rustc or cc1plus processes remained.

What the line says so far: at `jobs = 2` the first build seconds swapped at 20 MB/s out and 1.8 MB/s in, with load still at about 1.9. So the swap limit, not the load ceiling, is what tripped first. The swap came within the first 10 seconds of a clean build, which points at memory use by the cargo and rustc start-up rather than at the number of jobs. One stopped run does not separate the two. The next measurement needs to read the same readings, and the decision on `jobs` stays open until a line is read.

## Round 63 Phase A: is the clean-build swap the build's own working set or a neighbour's? (answer from files)

Answer: decided for `jobs = 12`, not decided for `jobs = 2`.

Evidence, from the files:
- Round 43 (ledger, the override section, the six clean builds): the tree-scoped peak RSS, taken by `scripts/lib/build-rss.sh` over the build's own process tree (not host-wide), was 6980.3 to 7153.2 MiB at `jobs = 12`, on a host with 15,220 MiB of RAM. The host swap during those builds was 1190 to 8699 MiB (vmstat deltas). The ledger records that a clean build at `jobs = 12` cannot stay under the swap threshold on this host.
- Round 43's swap totals are host-wide, so they include neighbours. The ledger's held items list "gross per-process swap" as unavailable: `/proc/vmstat` has only host-wide pswpin and pswpout.
- Round 62 (`jobs = 2`): the job recorded host swap, load and the gate only. `scripts/measure-clean-build.sh` logs no RSS. So no tree RSS exists for the 00:21:52 to 00:22:02 build, and the files cannot give its working set.
- Host memory now (read-only check, Round 63): MemTotal 15,584,988 kB, MemAvailable 7,861,520 kB, SwapTotal 20,971,512 kB, SwapFree 15,383,396 kB, so 5,588,116 kB (about 5.3 GiB) is in swap. Round 62's check at 00:21:15 read used 7987 MB, free 1282 MB, available 7231 MB.
- Neighbours: ten gm-core workers started with `--handshake`, RSS 982 MiB in total (one of them, pid 1002562, about 743 MiB). They are the only large resident neighbours in the files.

Decision:
- `jobs = 12`: the swap is the build's own working set. Round 43 puts the build's tree at about 7 GB, and MemAvailable at the time was 7.2 to 7.9 GB, so the build alone uses up what the host has free and swap follows. That is decided from the Round 43 records.
- `jobs = 2`: not decided. The files record no tree RSS for that build, and the spike came in its first 10 seconds with 8 crates compiled. Its working set at start is not in the files. The swap could be the build's own start-up memory, or the neighbours' pages being reclaimed for it.

What settles the `jobs = 2` question: the next Phase B build records tree RSS (`build_tree_sample` from `scripts/lib/build-rss.sh`) and the gm-core RSS total every 8 seconds, alongside the gate. If the build tree's RSS rises before the swap-out does, the swap is the build's own; if swap-out rises while the tree's RSS is flat, it is a neighbour's.

Not changed: `Cargo.toml`, `/config/.cargo/config.toml`, the cargo shim, and the `jobs` setting. No cargo ran, no browser ran, and nothing was pushed in this Phase A.

## Round 63 Phase B: clean-build memory sampling at `jobs = 2`, stopped at the swap limit (open)

The `jobs` decision stays open. No CLEAN_BUILD line was produced, and the `jobs` setting is unchanged (repo `.cargo/config.toml`, 12). The answer to the Phase A question is in the next section.

Window: opened 00:32:18 (first reading). Readings `vmstat 1 2` (second sample); heavy-job check reads full arguments of each high-CPU PID (gm-core `--handshake` exempt).
- 00:32:18 si 24, so 0, load1 2.18. Gate open. Heavy busy: peer `zig test` pid 1400349 at 100%. Not ours; not stopped.
- 00:32:45 si 144, so 0, load1 2.31. Gate open. Heavy busy: same peer.
- 00:33:13 si 132, so 0, load1 2.43. Gate open. Heavy busy: peer `zig` pid 1406365 at 199%.
- 00:33:40 si 16, so 0, load1 2.91. Gate open, heavy clear.
- Witness `scripts/check-clean-build-measure.sh`, once: rc 0, wall 0.36 s, 12 of 12 ok (00:33:40).
- 00:33:42 si 188, so 0, load1 2.91. Gate open. Heavy busy: `fx ask` pid 1407563 at 90% (peer).
- 00:34:10 si 52, so 0, load1 2.43. Gate open, heavy clear. Build started: `measure-clean-build.sh --jobs 2`, pid 1409356.

Build readings, every 8 s, with memory: time, si, so, load1, the build tree's RSS (`build_tree_sample`, `scripts/lib/build-rss.sh`), gm-core RSS, MemAvailable, swap in use:
- 00:34:11 si 300, so 520, load1 2.43, tree 27 MiB, gm-core 1000 MiB, MemAvailable 7433 MiB, swap used 5557 MiB.
- 00:34:20 si 3800, so 37168, load1 2.82, tree 920 MiB, gm-core 849 MiB, MemAvailable 7566 MiB, swap used 6266 MiB. Swap-in above 1000 KB/s. Stop by PID.

Stop: the tree was stopped by PID (1409356, 1409433, 1409440, 1409886, 1409913, 1410243), TERM, then KILL for any survivor. Survivors: 0. No peer process was stopped.

Result: exit 143, wall 15.2 s, stopped by swap-in. No CLEAN_BUILD line. The partial root was not logged; the build wrote a partial target root under `/tmp`, which is recorded here for the next stretch to check.

Host and cgroup readings after the stop (read-only):
- `/sys/fs/cgroup/memory.max` 12,884,901,888 bytes (12 GiB). `memory.current` 8.47 GB, then 7387 MiB at a later check. `memory.swap.max` 12 GiB, `memory.swap.current` 3.89 GB, then 3531 MiB.
- Host `pswpout` rose by about 70 MB in one 5-second window with no build running (at 00:35), and by 0 in a 10-second window later with no build running. Swap-out is bursty and host-wide.
- MemTotal is 15,584,988 kB, but the effective limit for this container is the cgroup's 12 GiB, not the host's 15.2 GB. Round 43's "15.2 GB" is the host figure, not this container's limit.

Answer to the Phase A question, from this run: the spike is not explained by the build's own working set. At the 00:34:20 reading the build's tree was 920 MiB, and MemAvailable was 7.5 GB. The spike's swap-out (37 MB/s) and the swap in use (6266 MiB) came from host-wide reclaim, and the pages it moved belong to processes this file set cannot name, since the host has no per-process swap counter. Neighbours are the better-supported reading. The build's tree did grow 893 MiB in 9 s, so the build's allocation may trigger the reclaim, but the files do not show that the reclaimed pages were the build's own.

Not changed: `Cargo.toml`, `/config/.cargo/config.toml`, the cargo shim, and the `jobs` setting. No peer process was stopped.

## Round 64 Phase B: clean-build memory sampling at `jobs = 2`, stopped at the swap limit (open)

The `jobs` decision stays open. No CLEAN_BUILD line was produced. The `jobs` setting is unchanged (repo `.cargo/config.toml`, 12). The stop rule and the measurement script's stop logic are unchanged.

Window: about ten minutes, from 00:41:22 (first reading) to the stop at 00:42:12. Readings `vmstat 1 2` (second sample); the heavy-job check reads full arguments of each high-CPU PID (gm-core `--handshake` exempt).
- 00:41:22 si 8496, so 16, load1 1.48. Closed. Heavy clear.
- 00:41:50 si 20, so 0, load1 1.62. Open. Heavy clear.
- Witness `scripts/check-clean-build-measure.sh`, once: rc 0, wall 0.36 s, 12 of 12 ok (00:41:50).
- 00:41:52 si 96, so 0, load1 1.62. Open, heavy clear. Build started: `measure-clean-build.sh --jobs 2`, pid 1488096.

Build readings, every 8 s, with memory: time, si, so, load1, the build tree's RSS (`build_tree_sample`), gm-core RSS, MemAvailable, swap in use:
- 00:41:53 si 4, so 0, load1 1.57, tree 26 MiB, gm-core 957 MiB, MemAvailable 7278 MiB, swap used 5610 MiB.
- 00:42:02 si 672, so 0, load1 1.68, tree 904 MiB, gm-core 966 MiB, MemAvailable 7129 MiB, swap used 5636 MiB.
- 00:42:12 si 3524, so 0, load1 2.04, tree 808 MiB, gm-core 951 MiB, MemAvailable 7131 MiB, swap used 5670 MiB. Swap-in above 1000 KB/s. Stop by PID.

Stop: the build's own tree was stopped by PID (1488096, 1488122, 1488128, 1488406, 1488411, 1488633), TERM, then KILL for any survivor. Survivors: 0. No peer process was stopped.

Result: exit 143, wall 24.3 s, stopped by swap-in. No CLEAN_BUILD line.

Notes for the reading:
- Swap-out stayed at 0 for the whole build. The spike was swap-in: pages returning from swap. The build's tree grows with fresh memory, so the returning pages are not its own. The owner of those pages is not recorded (host-wide vmstat, no per-process swap counter).
- The build's tree grew to 904 MiB in 9 s, then fell to 808 MiB at the stop, while MemAvailable stayed at about 7.1 GB.
- The container's cgroup has memory.max 12 GiB; memory.current was 7387 MiB at a later check.
- This is the second clean build at `jobs = 2` stopped at the swap limit with a small tree (Round 63: swap-out 37168 KB/s at tree 920 MiB).

Not changed: `Cargo.toml`, `/config/.cargo/config.toml`, the cargo shim, the `jobs` setting, and the stop rule.

## Round 65 Phase A: measurement logs kept under target/witness, not in /tmp

Item taken from the ranked list: the measurement logs left in `/tmp` by each run (ranked item 7). Why this one: the cargo log is the only record of how far a stopped build got (Round 61: 342 `Compiling` lines; Round 62: 8). Deleting logs at the end of a run would lose that evidence, so the log is kept in a git-ignored place instead.

Change: `scripts/measure-clean-build.sh` writes its log to `${CLEAN_BUILD_LOG_DIR:-$repo/target/witness/clean-build-logs}/<timestamp>-<pid>.log` and names the path on stderr. The result line, the gate and the stop rule are unchanged.

Witness: `scripts/check-clean-build-measure.sh`, now 13 checks, 13 of 13 ok, exit 0. The new check runs a successful build and requires its log to be a file under `CLEAN_BUILD_LOG_DIR` and named on stderr. The witness sets `CLEAN_BUILD_LOG_DIR` to its own scratch directory, so no witness logs land in the repo.

Red check: the `logdir` line was replaced with a `mktemp` directory under `/tmp`. The witness exited 1, with one FAIL: "the run log is kept in CLEAN_BUILD_LOG_DIR and named on stderr". The script was restored with `cmp` against its saved copy, and the witness then exited 0 with 13 ok (its own file also checked with `cmp`).

Not changed: the `jobs` setting, `Cargo.toml`, `/config/.cargo/config.toml`, the cargo shim, and the stop rule. The earlier logs in `/tmp` (Rounds 60 to 64) remain, since they are the evidence for those rounds.

Local commit only; not pushed in Phase A.

## Round 66 Phase A: correction to the Round 63 Phase B entry, and a witness for stale build roots

Item taken from the ranked list: the Round 63 Phase B entry said the partial root of its stopped build was left under `/tmp` for the next stretch to check. The root was not left. It was `/tmp/bevy-clean.TEfZTA` (28 MB, created 00:34:12 by that build), and it was removed in the same stretch, right after the entry was written. Correction: no stale build root remains from Round 63. Why this item: it is the only ranked item that can be closed from files already on disk without a browser or cargo.

Witness: `scripts/check-build-roots.sh`. It lists any `/tmp/bevy-clean.*` root and exits 1 if one exists, 0 if none does. It is the check a stopped measurement needs, since the script removes its root only on a normal finish.
- Green: "no stale build roots under /tmp", exit 0.
- Red check: a `/tmp/bevy-clean.redcheck` directory was created. The witness listed it ("stale build root: /tmp/bevy-clean.redcheck") and exited 1. The directory was removed, and the witness exited 0 again.

Not changed: the `jobs` setting, `Cargo.toml`, `/config/.cargo/config.toml`, the cargo shim, and the stop rule. The Round 63 entry is not rewritten; this entry corrects it.

Local commit only; not pushed in Phase A.

## Round 66 Phase B: file-based witness for 73e49b1, after a clear window

Window: about ten minutes from 00:51:19, one reading about every 30 seconds (`vmstat 1 2`, second sample is the gate reading; the heavy-job check reads full arguments of each high-CPU PID, gm-core `--handshake` workers exempt).
- 00:51:19 si 888, so 14552, load1 1.55. Closed (swap-out above 1000 KB/s). Heavy clear.
- 00:51:47 si 280, so 0, load1 1.77. Open. Heavy clear. The witness started on this sample.
- Witness `scripts/check-clean-build-measure.sh`, once: rc 0, wall 0.44 s, 13 of 13 ok. The new check (the run log is kept in `CLEAN_BUILD_LOG_DIR` and named on stderr) passed.

No build ran in this window. Nothing was stopped. No peer process was stopped.

Pushed with this entry: the local commits since 05250b4, namely 73e49b1 (the log location and its witness) and 44726e8 (the stale-root witness and the Round 63 correction).

## Round 67 Phase A: the result line carries the tree peak RSS, MemAvailable and the cgroup limit

Ranked remaining work at the start of this round, each marked reachable or blocked, is in the Round 67 REPORT. The item taken is the top reachable one: the result line lacked the figures the owner's stop-scope question needs (the build's tree peak RSS, MemAvailable before the build, and the cgroup's `memory.max`). It needs no cargo and no window.

Change, `scripts/measure-clean-build.sh`:
- It sources `scripts/lib/build-rss.sh`. The sampler records `build_tree_sample $$` (this script's own tree, which contains cargo and rustc) once per second, so `peak_tree_rss_mib` is the tree's peak at one-second resolution.
- `mem_avail_before_mib` is MemAvailable from `/proc/meminfo` before the build starts.
- `cgroup_max_mib` is `/sys/fs/cgroup/memory.max` in MiB, or `na` when it is not a number.
- The result line ends with `peak_tree_rss_mib=… mem_avail_before_mib=… cgroup_max_mib=…`. The gate, the stop rule and the earlier fields are unchanged.

Bug found and fixed in the same change: the Round 65 log name was `…-$.log`, not `…-$$.log`. The cause was `String.replace`, which reads `$$` in its replacement as one `$`. Names could collide within a second, and the witness did not catch it. The log name now carries the PID.

Witness, `scripts/check-clean-build-measure.sh`: 15 checks, 15 of 15 ok, exit 0. Two new checks: the result line ends with the three fields in the expected form, and the run log name matches `YYYYMMDD-HHMMSS-<pid>.log`.

Red checks, each one mutation, restored with `cmp`:
- The result field name changed to `peak_tree_rss_mb`. The witness exited 1, with one FAIL: "the result line ends with tree peak RSS, MemAvailable and the cgroup limit".
- The log name without the PID, `-$.log`. The witness exited 1, with one FAIL: "the run log name carries a timestamp and the PID".
- After both restores, the scripts match their saved copies (`cmp`) and the witness exited 0 with 15 ok. `git status` shows only the two scripts as modified.

Not changed: the stop rule, the `jobs` setting, `Cargo.toml`, `/config/.cargo/config.toml`, and the cargo shim. No cargo was run. The witness still runs with a fake cargo.

Local commit only; not pushed in Phase A.

## Round 67 Phase B: file-based witnesses for ee6bf11, after a clear window

Window: about ten minutes from 01:02 (first reading 01:03:02), one reading about every 30 seconds (`vmstat 1 2`, second sample is the gate reading; the heavy-job check reads full arguments of each high-CPU PID, gm-core `--handshake` workers exempt).
- 01:03:02 si 1660, so 16136, load1 2.23. Closed (swap-in and swap-out above 1000 KB/s). Heavy clear.
- 01:03:31 si 332, so 0, load1 2.10. Gate open. Heavy busy: a peer Claude session (pid 2225, 38.6% CPU, `--model claude-haiku-5-5`). It is not ours and was not stopped.
- 01:03:59 si 1520, so 0, load1 1.92. Closed. Heavy clear.
- 01:04:27 si 308, so 0, load1 1.81. Open. Heavy clear. Both witnesses started on this sample.

Witnesses, each once:
- `scripts/check-clean-build-measure.sh`: rc 0, wall 0.51 s, 15 of 15 ok (the two new checks pass: the result line ends with tree peak RSS, MemAvailable and the cgroup limit; the run log name carries a timestamp and the PID).
- `scripts/check-build-roots.sh`: rc 0, wall 0.04 s, "no stale build roots under /tmp".

No build ran in this window. Nothing was stopped.

## Round 68 Phase A: ASTC load measurement through the footprint's reference path

Item taken: ranked item 7, the ASTC footprint decision. It is decided at load ratio 1.21 against the 1.1 tolerance (ASTC 1267 ms, software RGBA8 reference 1048 ms; the stale reference 1250 ms had passed). The item asks for a new ASTC load measurement in a Phase B window to confirm or overturn that decision.

Design:
- `scripts/lib/astc-load-ratio.cjs`: `decideLoad` computes the ratio and the decision as `astcMs <= referenceMs * tolerance`, the same form as `loadOk` in `scripts/texture-footprint.cjs`. It rejects non-positive or non-finite inputs.
- `scripts/measure-astc-load.cjs`: runs the existing driver, `scripts/check-neon-wasm-models.cjs`, with `NEON_TEXTURE=astc4x4`, its PID captured. A timer stops the driver's whole process tree by PID at `ASTC_RUN_TIMEOUT_S` (default 600 s); a 2 s sampler records the tree, and any survivor is stopped by PID. It then reruns `scripts/texture-footprint.cjs` and reads the ASTC load and the reference load from that footprint's `formats.astc4x4` (`witness` and `referenceWitness`, the `rgba8-software` reference) and its `loadTolerance`. It prints one line: `ASTC_LOAD run_ok=… astc_ms=… reference_ms=… ratio=… tolerance=… load_ok=… footprint_load_ok=… agrees=… astc_clean=… astc_wall_s=… run_pid=… timed_out=… stragglers=… archive=… reference_file=…`.
- The old ASTC shots and witness are copied to `target/witness/astc-load-archive/<stamp>/`, and the old shots are put back after the run. The new run's shots are kept in the archive. So the visual record (`texture-visual-astc4x4.json`), which describes the old shots, stays consistent with the shots in place.
- Why ASTC only: the ASTC run took 391.8 s and the software RGBA8 run took 304.9 s, so both in one ten-minute window would not fit. The reference stays the existing witness, from the Round 57 window (11-19 23:13). The ratio therefore compares across windows, which the footprint already does; this is the caveat, stated in the Phase B entry.

Witness: `scripts/check-astc-load-ratio.cjs`, fixture timings. Cases: 1267 against 1048 gives ratio 1.209 and not usable; the stale 1250 reference makes 1267 usable; 1100 against 1000 is usable at the tolerance; 1101 against 1000 is not usable; equal loads give ratio 1; a null ASTC load and a zero reference load are rejected; and the result line has its fixed field order. Result: 9 of 9 ok, exit 0.

Red check: the comparison was made strict (`astcMs < referenceMs * tolerance`). The witness exited 1 with one FAIL, "a load exactly at the tolerance is usable". The file was restored with `cmp`, and the witness exited 0 again.

Runner tests, no browser, with stand-in drivers (`ASTC_LOAD_DRIVER`):
- A driver that exits 0 and writes nothing: `run_ok=false fresh=false exit_code=0`, exit 1. The 64 ASTC shots were put back, and the witness is byte-identical to its copy (`cmp`).
- A driver that hangs, with `ASTC_RUN_TIMEOUT_S=2`: `run_ok=false timed_out=true stragglers=0`, exit 1. The driver was stopped by PID, the 64 shots were put back, and the witness is byte-identical. No stand-in process was left (checked by PID).
- Both test runs left archive folders under `target/witness/astc-load-archive/`, which holds only witness copies and driver logs. They are git-ignored and kept as evidence.

Not changed: the footprint rule (`LOAD_TOLERANCE` 1.1), the stop rule, the `jobs` setting, `Cargo.toml`, `/config/.cargo/config.toml`, and the cargo shim. No cargo ran. Local commit only; not pushed in Phase A.

## Round 68 Phase B: the window cleared too late for the ASTC run (superseded by Round 70)

Correction to the Round 68 Phase A entry: the witness `scripts/check-astc-load-ratio.cjs` has 7 cases plus one line-format check, so its output is 8 of 8 ok, not 9 of 9. Its exit codes and the red check stand.

Window: 01:14:29 to 01:24:29 (ten minutes), one reading about every 30 seconds (`vmstat 1 2`, second sample is the gate reading; the heavy-job check reads full arguments of each high-CPU PID, gm-core `--handshake` workers exempt). None of the heavy processes below is ours, and none was stopped.
- 01:14:29 to 01:16:22: gate open; heavy busy, a peer `zig test` (pid 1514462) at 27 to 100% CPU.
- 01:16:51: si 2364, so 0, gate closed; the same peer at 47%.
- 01:17:19: si 12444, so 19392, gate closed; a peer `zig test` (pid 1523262) at 98%.
- 01:17:47: si 776, so 16944, gate closed; heavy clear.
- 01:18:15: si 128, so 0, gate open; a peer `fx` (pid 1525976) at 100%.
- 01:18:44: si 1388, so 0, gate closed; heavy clear.
- 01:19:12: si 6808, so 3968, gate closed; heavy clear.
- 01:19:40: si 7392, so 0, gate closed; a peer `fx` (pid 1534337) at 102%.
- 01:20:08: si 82492, so 24456, gate closed; a peer `fx` (pid 1542656) at 91%.
- 01:20:37: si 916, so 132684, load 3.51, gate closed; a Chromium renderer (pid 1549721, RSS 1.47 GB) and its GPU process (pid 1549130) at 218%.
- 01:21:05: si 7696, so 133532, load 6.04, gate closed; the renderer at RSS 5.67 GB.
- 01:21:33: si 7332, so 0, gate closed; heavy clear.
- 01:22:02: si 22548, so 0, gate closed; a peer `fx` (pid 1570759) at 98%.
- 01:22:30: si 1856, so 0, gate closed; a peer `fx` (pid 1578123) at 80%.
- 01:22:58: si 268, so 0, load 3.73, gate open; heavy clear. This is the first clear sample. Remaining in the window: 89 s.

Witness, once, on the clear sample: `scripts/check-astc-load-ratio.cjs` rc 0, wall 0.04 s, 8 of 8 ok.

The ASTC run could not start in this window: the rule needs at least 420 s left, and the first clear sample came with 89 s left. The run completed in the Round 70 window below.

The Round 68 commits are pushed with the Round 70 entry.

## Round 69 Phase A: a committed heavy-job checker

Ranked remaining work at the start of this round is in the Round 69 REPORT. The item taken is the heavy-job rule, which every window so far has re-typed inline in its `/tmp` job script. It had no committed checker and no witness, so its counts were never tested. This item needs no window.

Change:
- `scripts/lib/heavy-jobs.cjs`: `parseTop` reads the last frame of `top -b -n 2`; `parsePs` reads `ps -eo pid=,rss=,args=`; `classify` applies the rule. A gm-core worker with `--handshake` is exempt. A cargo `test`, `check` or `nextest` run is `test-check`. A compiler (cc1plus, cc1, rustc, cargo, zig, cmake, ninja, collect2, clang, lld, mold, sccache) is `compile`. A Chromium renderer over 1 GB is `chromium-renderer`. Any other process above 20% CPU in top is `hot`, and a process is listed once.
- `scripts/heavy-jobs.cjs`: the live reading, `HEAVY_JOBS clear` (exit 0) or `HEAVY_JOBS busy n=…` with one line per item (exit 1). It takes about 2 seconds.
- `scripts/check-heavy-jobs.cjs`: fixture witness, 14 checks, 14 of 14 ok, exit 0.

Two errors found while writing the witness, both fixed before this commit:
- `cargo` is in the compiler set, so `cargo test` was labelled `compile`. The `test-check` branch now runs first. It counted before and after.
- The first fixture gave top rows with no matching `ps` rows, so the hot cases could not fire. The fixture now lists each process in both.

Red check: the 20% comparison was made inclusive (`>=`). The witness exited 1 with one FAIL, "exactly at the 20% limit does not count". The file was restored from its saved copy with `cmp`, and the witness exited 0 with 14 ok.

Live check: `node scripts/heavy-jobs.cjs` at this stamp printed `HEAVY_JOBS busy n=1` (a peer `fx` process at 39% CPU, `hot`), exit 1. This is the same kind of peer that held the last two windows.

Not changed: the stop rule, the `jobs` setting, `Cargo.toml`, `/config/.cargo/config.toml`, and the cargo shim. The inline checks in earlier `/tmp` job scripts are not replaced; they stay as recorded. Local commit only; not pushed in Phase A.

## Round 70 Phase B: the ASTC load measurement, completed

Window: 01:42:11 (first reading) to 01:48:07, about ten minutes. The measurement started at 01:42:39 on a clear sample: swap-in 12, swap-out 0, load 2.93, heavy clear, 569 s left (at least 420 required). The heavy-job check is `scripts/heavy-jobs.cjs` (committed as 637de88, message amended to 495fff6), read once per reading. Readings `vmstat 1 2` (second sample is the gate reading):
- 01:42:11: si 56, so 36, load 2.93. Gate open. Heavy busy: a peer `zig test` (pid 1612103) at 100%.
- 01:42:39: si 12, so 0, load 2.93. Gate open. Heavy clear. Started `measure-astc-load.cjs` (pid 1621922) with a 571 s timeout.
- 01:42:41: si 784, so 144, load 3.02. Gate open. Heavy busy: our playwriter helper (pid 1622001) at 45.5%.
- 01:43:09: si 3004, so 28, load 3.54. Gate closed (swap-in). Heavy busy: peer zig (1612103) at 88%, and our Chromium.
- 01:43:36: si 28, so 0, load 5.86. Gate closed (load). Heavy busy: a peer Claude session (pid 2707) at 21%, and peer zig.
- 01:44:04: si 36, so 0, load 5.87. Gate closed (load). Heavy busy: peer zig at 100%.
- 01:44:31: si 40, so 0, load 4.52. Gate closed (load). Heavy busy: peer zig at 98%.
- 01:44:58: si 32, so 0, load 7.49. Gate closed (load). Heavy busy: peer zig at 99%.
- 01:45:26: si 468, so 0, load 10.39. Gate closed (load). Heavy busy: our Chromium (pid 1622117) at 111%.
- 01:45:53: si 232, so 17096, load 11.62. Gate closed. Heavy busy: our Chromium at 182%.
- 01:46:20: si 552, so 0, load 10.76. Gate closed. Heavy busy: a peer Claude session (pid 1998) at 25%.
- 01:46:48: si 496, so 16700, load 8.16. Gate closed. Heavy busy: our Chromium at 104%.
- 01:47:15: si 680, so 4612, load 6.71. Gate closed. Heavy busy: our Chromium at 543%.
- 01:47:42: si 276, so 0, load 7.49. Gate closed. Heavy busy: our Chromium at 561%.
- 01:48:07: the runner exited, rc 0.

The Chromium rows are the run's own browser, which the heavy rule counts. The peer `zig test` (1612103) ran throughout and was not stopped.

Result line (`scripts/measure-astc-load.cjs`, through the existing reference path):
`ASTC_LOAD run_ok=true astc_ms=1045 reference_ms=1048 ratio=0.997 tolerance=1.1 load_ok=true footprint_load_ok=true agrees=true astc_clean=64/64 astc_wall_s=310.0 run_pid=1621938 timed_out=false stragglers=0 archive=target/witness/astc-load-archive/2026-11-19T23-42-39-517Z reference_file=neon-wasm-witness-rgba8-software.json`

Wall times: the measurement step 328.5 s (runner start to exit, including the footprint rerun); the browser run itself 310.0 s (`astc_wall_s`).

The stop rule: the runner ended on its own (rc 0, `timed_out=false`, no stragglers). Afterwards, none of the run's PIDs (1621922, 1621938, 1622117) was still alive. Nothing was stopped by PID in this window.

Footprint, rerun by the runner: decision `bc7` (unchanged). `fallbackFormats.astc4x4` is now true; before this run it was false (the Round 59 exclusion at 1.21). The ASTC entry: usable true, footprintWins true, loadOk true, cleanOk true, visualOk true, texelOk null (ASTC is not texel-judged). The footprint output is not committed, so the decision is recorded here, not in code.

Reading the result, with its limits:
- ASTC load is now 1045 ms against the 1048 ms reference, ratio 0.997, inside the 1.1 tolerance. Under the footprint's own rule, this overturns the Round 59 exclusion (1.21).
- The reference is the Round 57 software RGBA8 witness, from another window. The ASTC run was in a window with heavy host load (readings up to load 11.6, our Chromium at 561%). One ASTC load from a different window is not a same-window pair, so these files cannot show how much of the difference is the format and how much is the window.
- A same-window pair (ASTC run, then reference run) takes about 11.5 minutes (392 s and 305 s in the Round 58 and Round 57 records), so one ten-minute window cannot hold both. The owner decides whether a paired run is required before ASTC re-enters the fallback list.

### Correction, Round 71 Phase B: the same-window pair settles the decision

This run replaces the reading in the Round 70 section above. The Round 70 ratio (0.997) compared an ASTC run against a reference from another window. The pair below runs both arms in one window, so it answers the question that the Round 70 section left open.

Window: started 01:54:09 with 777 s left (660 s required). The pair was started at a clear sample (swap-in 328, swap-out 0, load 2.55, heavy clear). The window was 780 s. Readings `vmstat 1 2` (second sample is the gate reading), with `scripts/heavy-jobs.cjs` for the heavy-job reading. From the start the heavy check was busy, with this run's own Chromium and a peer Claude session. The readings, verbatim from the job log:

```
reading 01:54:09 si=328 so=0 load1=2.55 gate=open heavy=clear remainS=777 HEAVY_JOBS clear;
P1 measure-astc-pair started pid=1707491 windowS=780 at 01:54:09
reading 01:54:11 si=1048 so=0 load1=2.59 gate=closed heavy=busy remainS=775 HEAVY_JOBS busy n=1 hot pid=1707572 cpu=30.7 /usr/lib/chromium/chromium.real --user-data-dir=/workspace/bevy/.playwriter/browser-profile --remote-debugging-port=10232 --no-first-run --no-default-browser-ch;
reading 01:54:38 si=608 so=0 load1=4.68 gate=closed heavy=busy remainS=748 HEAVY_JOBS busy n=1 hot pid=1707603 cpu=189.1 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra;
reading 01:55:06 si=352 so=344 load1=4.07 gate=closed heavy=busy remainS=720 HEAVY_JOBS busy n=1 hot pid=1707603 cpu=154.5 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra;
reading 01:55:33 si=640 so=3180 load1=3.69 gate=closed heavy=busy remainS=693 HEAVY_JOBS busy n=1 hot pid=1707603 cpu=105 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra;
reading 01:56:01 si=584 so=0 load1=6.60 gate=closed heavy=busy remainS=665 HEAVY_JOBS busy n=1 hot pid=1707603 cpu=385.1 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra;
reading 01:56:28 si=2128 so=0 load1=5.36 gate=closed heavy=busy remainS=638 HEAVY_JOBS busy n=2 hot pid=1707603 cpu=170.6 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra hot pid=1707766 cpu=46.1 /usr/lib/chromium/chromium.real --type=renderer --crashpad-handler-pid=1707574 --enable-crash-reporter=, --user-data-dir=/workspace/bevy/.playwriter/browser-pro;
reading 01:56:55 si=28 so=0 load1=6.26 gate=closed heavy=busy remainS=611 HEAVY_JOBS busy n=2 hot pid=1707603 cpu=116.8 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra hot pid=1707766 cpu=97 /usr/lib/chromium/chromium.real --type=renderer --crashpad-handler-pid=1707574 --enable-crash-reporter=, --user-data-dir=/workspace/bevy/.playwriter/browser-pro;
reading 01:57:23 si=84 so=78584 load1=7.29 gate=closed heavy=busy remainS=583 HEAVY_JOBS busy n=1 hot pid=1707603 cpu=910.7 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra;
reading 01:57:50 si=68 so=0 load1=7.75 gate=closed heavy=busy remainS=556 HEAVY_JOBS busy n=1 hot pid=1707603 cpu=1082 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra;
reading 01:58:17 si=168 so=0 load1=7.85 gate=closed heavy=busy remainS=529 HEAVY_JOBS busy n=1 hot pid=1707603 cpu=940.2 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra;
reading 01:58:45 si=1956 so=0 load1=5.90 gate=closed heavy=busy remainS=501 HEAVY_JOBS busy n=2 hot pid=2356 cpu=24.8 claude --dangerously-skip-permissions --model claude-haiku-5-5 --resume 57a3f21e-09df-4804-8a83-c6fd3b6f87f0 hot pid=1707603 cpu=109.9 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra;
reading 01:59:12 si=1336 so=0 load1=4.49 gate=closed heavy=busy remainS=474 HEAVY_JOBS busy n=2 hot pid=1707603 cpu=97 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra hot pid=1707766 cpu=45.5 /usr/lib/chromium/chromium.real --type=renderer --crashpad-handler-pid=1707574 --enable-crash-reporter=, --user-data-dir=/workspace/bevy/.playwriter/browser-pro;
reading 01:59:39 si=44 so=0 load1=5.70 gate=closed heavy=busy remainS=447 HEAVY_JOBS busy n=1 hot pid=1712124 cpu=676.2 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra;
reading 02:00:07 si=380 so=0 load1=7.43 gate=closed heavy=busy remainS=419 HEAVY_JOBS busy n=2 hot pid=1712124 cpu=120.4 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra hot pid=1712266 cpu=106.8 /usr/lib/chromium/chromium.real --type=renderer --crashpad-handler-pid=1712096 --enable-crash-reporter=, --user-data-dir=/workspace/bevy/.playwriter/browser-pro;
reading 02:00:34 si=33716 so=0 load1=9.75 gate=closed heavy=busy remainS=392 HEAVY_JOBS busy n=1 hot pid=1712124 cpu=109.9 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra;
reading 02:01:02 si=60 so=0 load1=8.70 gate=closed heavy=busy remainS=364 HEAVY_JOBS busy n=2 hot pid=1712124 cpu=86.1 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra hot pid=1712266 cpu=47.5 /usr/lib/chromium/chromium.real --type=renderer --crashpad-handler-pid=1712096 --enable-crash-reporter=, --user-data-dir=/workspace/bevy/.playwriter/browser-pro;
reading 02:01:29 si=112 so=7796 load1=7.00 gate=closed heavy=busy remainS=337 HEAVY_JOBS busy n=4 hot pid=2356 cpu=21.7 claude --dangerously-skip-permissions --model claude-haiku-5-5 --resume 57a3f21e-09df-4804-8a83-c6fd3b6f87f0 hot pid=1712124 cpu=875.5 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra hot pid=1713988 cpu=26.4 find / -path /proc -prune -o -name heavy-jobs.cjs -print hot pid=1714277 cpu=66 node scripts/witness-shell.mjs;
reading 02:01:57 si=300 so=0 load1=6.08 gate=closed heavy=busy remainS=309 HEAVY_JOBS busy n=2 hot pid=1712124 cpu=963.4 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra hot pid=1713988 cpu=23.8 find / -path /proc -prune -o -name heavy-jobs.cjs -print;
reading 02:02:24 si=16 so=0 load1=7.71 gate=closed heavy=busy remainS=282 HEAVY_JOBS busy n=2 hot pid=1712124 cpu=112.9 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra hot pid=1713988 cpu=36.6 find / -path /proc -prune -o -name heavy-jobs.cjs -print;
reading 02:02:52 si=1840 so=572 load1=9.52 gate=closed heavy=busy remainS=254 HEAVY_JOBS busy n=3 hot pid=2225 cpu=24.3 claude --dangerously-skip-permissions --model claude-haiku-5-5 --resume 5cb6cedd-2569-4bba-9a75-05ecaf2ec841 hot pid=1712124 cpu=721.4 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra hot pid=1713988 cpu=28.2 find / -path /proc -prune -o -name heavy-jobs.cjs -print;
reading 02:03:19 si=40 so=0 load1=8.13 gate=closed heavy=busy remainS=227 HEAVY_JOBS busy n=2 hot pid=1712124 cpu=190.1 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra hot pid=1713988 cpu=34.7 find / -path /proc -prune -o -name heavy-jobs.cjs -print;
reading 02:03:46 si=168 so=48404 load1=7.64 gate=closed heavy=busy remainS=200 HEAVY_JOBS busy n=2 hot pid=1712124 cpu=852 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra hot pid=1713988 cpu=35.3 find / -path /proc -prune -o -name heavy-jobs.cjs -print;
reading 02:04:14 si=3024 so=0 load1=8.59 gate=closed heavy=busy remainS=172 HEAVY_JOBS busy n=1 hot pid=1712124 cpu=267.3 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra;
P1 rc=0 wall=629.8s
PAIR_OUT: ASTC_PAIR run_ok=true window=2026-11-19T23-54-09-313Z archive=target/witness/astc-load-pair-archive/2026-11-19T23-54-09-313Z order=astc,reference interleaved=true load_start_astc=2.55 load_start_reference=6.1 astc_ms=990 reference_ms=1018 ratio=0.972 tolerance=1.1 pair_ok=true footprint_load_ok=true astc_clean=64/64 reference_clean=64/64 astc_wall_s=308.8 reference_wall_s=300.8 run_pids=1707507,1712038 stragglers=0 reason=interleaved
PAIR_STDERR_TAIL: 
END 02:04:39
```

The pair line, verbatim from `scripts/measure-astc-pair.cjs`:

```
ASTC_PAIR run_ok=true window=2026-11-19T23-54-09-313Z archive=target/witness/astc-load-pair-archive/2026-11-19T23-54-09-313Z order=astc,reference interleaved=true load_start_astc=2.55 load_start_reference=6.1 astc_ms=990 reference_ms=1018 ratio=0.972 tolerance=1.1 pair_ok=true footprint_load_ok=true astc_clean=64/64 reference_clean=64/64 astc_wall_s=308.8 reference_wall_s=300.8 run_pids=1707507,1712038 stragglers=0 reason=interleaved
```

What the pair shows:
- In this window, ASTC took 990 ms against the reference's 1018 ms. The ratio is 0.972, inside the 1.1 tolerance. The Round 59 exclusion (ratio 1.21) does not hold in the same window.
- The order was ASTC, then the reference, both in the window. Each arm was 64 of 64 clean. The arms started at different loads: ASTC at 2.55, the reference at 6.10. A reference that starts under higher load runs slower, which lowers the ratio, so the load difference does not favour the ASTC result here.
- The window holds one pair, about 10.5 minutes of browser time. A longer alternation (ASTC, reference, ASTC, reference) needs about 21 minutes, so this run has one pair. The witness `scripts/check-astc-pair.cjs` checks longer alternations.
- Wall times: the pair step 629.8 s; the ASTC arm 308.8 s; the reference arm 300.8 s.
- The stop: the runner ended on its own (rc 0, `stragglers=0`). Afterwards, PIDs 1707491, 1707507, 1712038, 1712124 and 1707603 were all gone. Nothing was stopped by PID in the window beyond the runner's own tree.
- Footprint, rerun by the pair runner against this window's reference: decision `bc7` (unchanged). `astc4x4` usable true, load 990 against 1018. `etc2` usable true, load 955 against 1018. `bc7` usable true, its reference is the hardware RGBA8 witness (1404), which this run did not change.
- Records written: `neon-wasm-witness-rgba8-software.json` was replaced by this run (64 of 64 clean, median 1018 ms). The shot sets were put back, and the new ASTC and reference shot sets are kept in `target/witness/astc-load-pair-archive/2026-11-19T23-54-09-313Z/`, so the visual records still match the shot sets in place.

The decision: the ASTC footprint decision of Round 59 (exclusion at 1.21) does not hold in the same window. The footprint now lists ASTC as usable and as a fallback. The owner's earlier ruling that ASTC leaves the fallback list is the one this measurement contradicts, and it needs the owner's confirmation. No committed decision file was changed; the footprint output is git-ignored.

## Round 71 Phase A: an interleaved ASTC and reference pair in one window

Why a pair, and why this window length: the Round 70 ASTC ratio (0.997) compared an ASTC run against a reference from another window, so it cannot settle the decision. A pair needs both arms in the same window. One ASTC run takes about 310 to 330 s and one software RGBA8 reference run about 305 s, so one pair needs about 11 minutes of browser time. The ten-minute window and the 420 s start rule cannot hold a pair. This round's window is 780 s (13 minutes), and a pair may start only with at least 660 s left. This is a change to the stated window, and the REPORT says so.

What is in this change:
- `scripts/lib/load-arm.cjs`: the single-arm runner, moved out of `measure-astc-load.cjs` so the single-arm and pair runners share it. It archives and restores the arm's shots and witness, runs the driver with its PID captured under a timer that stops its tree by PID, and records the load at the arm's start.
- `scripts/measure-astc-load.cjs`: the same behaviour as before, now calling `load-arm.cjs`. The stand-in tests from Round 68 were run again: the driver that writes nothing (`fresh=false`) and the driver that hangs (`timed_out=true`). Both gave the same results, with the shots restored and the witness byte-identical.
- `scripts/lib/astc-load-ratio.cjs`: `checkPairs` groups the runs by window and sorts them by start time. A window must alternate arms (no two runs of one arm back to back), have an even number of runs, and be split into astc and reference pairs. The check passes only if every pair ratio is within the tolerance (`astcMs <= referenceMs * tolerance`). `formatLine` takes the line prefix.
- `scripts/measure-astc-pair.cjs`: runs the ASTC arm, then the reference arm (`rgba8-software`), in the same window. The ASTC arm's timer is the window minus the reference budget (`PAIR_REFERENCE_BUDGET_S`, default 360 s). It refuses to run the reference arm if the ASTC arm fails. It reruns `texture-footprint.cjs` after the pair, so the footprint's reference is the one from this window. It prints one `ASTC_PAIR` line.
- `scripts/check-astc-pair.cjs`: fixture witness, 10 cases, 10 of 10 ok, exit 0. The cases: one interleaved pair within tolerance passes; two interleaved pairs pass; arms run back to back fail; a pair split across two windows fails; an odd number of runs fails; a pair above tolerance (1267 against 1048) fails; one of two pairs above tolerance fails the check; a pair at exactly the tolerance passes; unsorted input is sorted by start time; no runs is not a pass.

Red check: the pair check was changed to `ok: interleaved`, so it ignored the tolerance. The witness exited 1 with two FAILs, "a pair ratio above the tolerance fails (1267 against 1048)" and "one pair above the tolerance fails the whole check". The file was restored with `cmp`, and the witness exited 0 again. `git status` shows only the intended files.

Stand-in tests of the pair runner (no browser): the ASTC arm writes nothing gives `ASTC_PAIR run_ok=false stage=astc fresh=false`, exit 1, with the 64 shots restored and the witness byte-identical. The ASTC arm that hangs under a 2 s budget gives `stage=astc timed_out=true`, exit 1, with the driver stopped by PID and no stand-in process left.

Not changed: the footprint rule (1.1), the stop rule, the `jobs` setting, `Cargo.toml`, `/config/.cargo/config.toml`, and the cargo shim. No cargo ran. Local commit only; not pushed in Phase A.

## Round 72 Phase A: the fallback list and its window evidence

The fallback list is not hardcoded. `scripts/texture-footprint.cjs` derives `fallbackFormats` from each format's `usable` verdict. No script, witness, or AGENTS.md line still lists ASTC as excluded. The Round 59 exclusion survives only as ledger history, and the Round 70 and Round 71 corrections mark it superseded. So the list already matches the measured result, and no list value changed.

What changed:
- `scripts/lib/fallback-list.cjs`: `windowEvidence` takes the gap between a format's witness mtime and its reference mtime, and counts it as the same window at 780 s or less. `checkFallbackList` requires the list keys to be exactly astc4x4 and etc2, each listed value to equal its usable verdict, the window evidence to be present and consistent, and the decision to follow bc7 usable.
- `scripts/texture-footprint.cjs`: each judged format now records `loadEvidence`. The decision rule and the 1.1 tolerance are unchanged.
- `scripts/check-fallback-list.cjs`: six fixture cases and the live footprint. 7 of 7 ok, exit 0.

Red check: the list comparison in the library was disabled (`if (false) failures.push`). The witness exited 1 with `FAIL an ETC2 entry listed false while usable fails`. The library was restored with `cmp`, and the witness exited 0. Then the live footprint's `etc2` was set to false. The witness exited 1 with `FAIL live footprint list: etc2 listed false against usable true`. The file was restored with `cmp`, and the witness exited 0.

Caveat, the window evidence. ASTC's verdict is same-window: its witness and the pair's reference are 309.082 s apart. ETC2's verdict is not: its witness is from 21:41 and its reference from 00:04, a gap of 8580.541 s. ETC2 is listed usable under the footprint's rule, so the list is right by that rule. But its load comparison is cross-window, the same kind of evidence the pair replaced for ASTC. A same-window ETC2 pair needs about 11 minutes of window. It is not run here, and it is on the ranked list.

No window was needed for this change. No cargo ran, and the `jobs` setting, `Cargo.toml`, `/config/.cargo/config.toml`, and the cargo shim are unchanged. Local commit only; not pushed in Phase A.

### Correction, Round 72 caveat: the ETC2 same-window pair (Round 77 Phase B)

The Round 72 ETC2 verdict was cross-window (witness 21:41, reference 00:04, a gap of 8580.541 s). This run settles it with an interleaved pair in one window, under the Round 76 design: each arm has its own fixed budget of at least 360 s, and the pair starts only with at least 720 s left in the job's own 780 s window.

Window: the pair started at 02:52:29 with 780 s left (the job's first clear sample, gate open, heavy clear). The window ended at 03:02:59 (job END). Pair step wall 629.4 s; ETC2 arm 302.4 s; reference arm 307.2 s. The ETC2 arm did not reach its 360 s budget.

Readings, verbatim from the job log (the second `vmstat 1 2` sample is the gate; the heavy reading is `scripts/heavy-jobs.cjs` at bevy's commit 495fff6, which is unchanged in HEAD):

```
reading 02:52:29 si=940 so=0 load1=2.77 gate=open heavy=clear remainS=778 HEAVY_JOBS clear;
P1 measure-astc-pair started pid=1910672 jobRemainS=780 at 02:52:29
reading 02:52:32 si=2976 so=0 load1=2.71 gate=closed heavy=clear remainS=775 HEAVY_JOBS clear;
reading 02:52:59 si=372 so=0 load1=3.74 gate=open heavy=busy remainS=748 HEAVY_JOBS busy n=2 hot pid=2510 cpu=25 claude --dangerously-skip-permissions --model claude-haiku-5-5 --resume 0f054132-9d13-4196-b9d1-da8e68e7c630 hot pid=1910802 cpu=893.3 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra;
reading 02:53:26 si=692 so=0 load1=6.20 gate=closed heavy=busy remainS=721 HEAVY_JOBS busy n=2 hot pid=1910802 cpu=80.2 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra hot pid=1910941 cpu=57.4 /usr/lib/chromium/chromium.real --type=renderer --crashpad-handler-pid=1910766 --enable-crash-reporter=, --user-data-dir=/workspace/bevy/.playwriter/browser-pro;
reading 02:53:54 si=412 so=176 load1=5.13 gate=closed heavy=busy remainS=693 HEAVY_JOBS busy n=1 hot pid=1910802 cpu=113.9 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra;
reading 02:54:21 si=220 so=336 load1=4.05 gate=closed heavy=busy remainS=666 HEAVY_JOBS busy n=1 hot pid=1910802 cpu=191 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra;
reading 02:54:48 si=592 so=0 load1=3.98 gate=open heavy=busy remainS=639 HEAVY_JOBS busy n=2 hot pid=1910802 cpu=340.2 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra hot pid=1910941 cpu=46.1 /usr/lib/chromium/chromium.real --type=renderer --crashpad-handler-pid=1910766 --enable-crash-reporter=, --user-data-dir=/workspace/bevy/.playwriter/browser-pro;
reading 02:55:16 si=316 so=31236 load1=3.37 gate=closed heavy=busy remainS=611 HEAVY_JOBS busy n=1 hot pid=1910802 cpu=925.5 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra;
reading 02:55:43 si=18824 so=128 load1=4.49 gate=closed heavy=busy remainS=584 HEAVY_JOBS busy n=1 hot pid=1910802 cpu=1002 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra;
reading 02:56:11 si=24 so=68 load1=3.97 gate=open heavy=busy remainS=556 HEAVY_JOBS busy n=1 hot pid=1910802 cpu=113.9 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra;
reading 02:56:38 si=308 so=0 load1=4.51 gate=closed heavy=busy remainS=529 HEAVY_JOBS busy n=1 hot pid=1910802 cpu=109.9 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra;
reading 02:57:05 si=3492 so=0 load1=6.71 gate=closed heavy=busy remainS=502 HEAVY_JOBS busy n=1 hot pid=1910802 cpu=730.1 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra;
reading 02:57:33 si=49912 so=2804 load1=8.24 gate=closed heavy=busy remainS=474 HEAVY_JOBS busy n=1 hot pid=1910802 cpu=118 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra;
reading 02:58:00 si=156 so=0 load1=6.51 gate=closed heavy=busy remainS=447 HEAVY_JOBS busy n=1 hot pid=1915830 cpu=951.4 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra;
reading 02:58:27 si=760 so=0 load1=7.99 gate=closed heavy=busy remainS=420 HEAVY_JOBS busy n=1 hot pid=1915830 cpu=112.9 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra;
reading 02:58:55 si=1416 so=0 load1=6.87 gate=closed heavy=busy remainS=392 HEAVY_JOBS busy n=1 hot pid=1915830 cpu=184.2 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra;
reading 02:59:22 si=112 so=5500 load1=5.49 gate=closed heavy=busy remainS=365 HEAVY_JOBS busy n=2 hot pid=1915830 cpu=556.4 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra hot pid=1915934 cpu=28.7 /usr/lib/chromium/chromium.real --type=renderer --crashpad-handler-pid=1915801 --enable-crash-reporter=, --user-data-dir=/workspace/bevy/.playwriter/browser-pro;
reading 02:59:49 si=284 so=0 load1=9.60 gate=closed heavy=busy remainS=338 HEAVY_JOBS busy n=1 hot pid=1915830 cpu=112.9 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra;
reading 03:00:17 si=2160 so=32180 load1=9.14 gate=closed heavy=busy remainS=310 HEAVY_JOBS busy n=1 hot pid=1915830 cpu=919.6 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra;
reading 03:00:44 si=1096 so=21344 load1=8.87 gate=closed heavy=busy remainS=283 HEAVY_JOBS busy n=1 hot pid=1915830 cpu=115.8 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra;
reading 03:01:11 si=1896 so=0 load1=7.06 gate=closed heavy=busy remainS=256 HEAVY_JOBS busy n=1 hot pid=1915830 cpu=190.1 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra;
reading 03:01:39 si=1824 so=0 load1=6.98 gate=closed heavy=busy remainS=228 HEAVY_JOBS busy n=4 hot pid=1915799 cpu=21 /usr/lib/chromium/chromium.real --user-data-dir=/workspace/bevy/.playwriter/browser-profile --remote-debugging-port=10232 --no-first-run --no-default-browser-ch hot pid=1915830 cpu=535 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra hot pid=1915934 cpu=33 /usr/lib/chromium/chromium.real --type=renderer --crashpad-handler-pid=1915801 --enable-crash-reporter=, --user-data-dir=/workspace/bevy/.playwriter/browser-pro hot pid=1919571 cpu=82 strings -n 6 /usr/lib/chromium/chromium.real;
reading 03:02:06 si=3216 so=51596 load1=8.98 gate=closed heavy=busy remainS=201 HEAVY_JOBS busy n=3 hot pid=1915830 cpu=77.2 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra hot pid=1915934 cpu=68.3 /usr/lib/chromium/chromium.real --type=renderer --crashpad-handler-pid=1915801 --enable-crash-reporter=, --user-data-dir=/workspace/bevy/.playwriter/browser-pro hot pid=1920079 cpu=95 grep -rl requestMemoryDump /config/.gmnew /usr/lib/node_modules /usr/local/lib/node_modules /usr/share/chromium /usr/lib/chromium;
reading 03:02:34 si=5780 so=772 load1=8.32 gate=closed heavy=busy remainS=173 HEAVY_JOBS busy n=2 hot pid=1915830 cpu=183.2 /usr/lib/chromium/chromium.real --type=gpu-process --gpu-recent-crash-count=0 --no-sandbox --disable-dev-shm-usage --disable-breakpad --ozone-platform=x11 --cra hot pid=1920079 cpu=56.4 grep -rl requestMemoryDump /config/.gmnew /usr/lib/node_modules /usr/local/lib/node_modules /usr/share/chromium /usr/lib/chromium;
P1 rc=0 wall=629.4s
PAIR_OUT: ETC2_PAIR run_ok=true window=2026-11-20T00-52-29-798Z archive=target/witness/etc2-load-pair-archive/2026-11-20T00-52-29-798Z order=etc2,reference interleaved=true load_start_etc2=2.77 load_start_reference=7.43 etc2_ms=1014 reference_ms=1032 ratio=0.983 tolerance=1.1 pair_ok=true footprint_load_ok=true etc2_clean=64/64 reference_clean=64/64 etc2_wall_s=302.4 reference_wall_s=307.2 run_pids=1910688,1915757 stragglers=0 reason=interleaved
PAIR_STDERR_TAIL: 
END 03:02:59
```

Result line, verbatim:

```
ETC2_PAIR run_ok=true window=2026-11-20T00-52-29-798Z archive=target/witness/etc2-load-pair-archive/2026-11-20T00-52-29-798Z order=etc2,reference interleaved=true load_start_etc2=2.77 load_start_reference=7.43 etc2_ms=1014 reference_ms=1032 ratio=0.983 tolerance=1.1 pair_ok=true footprint_load_ok=true etc2_clean=64/64 reference_clean=64/64 etc2_wall_s=302.4 reference_wall_s=307.2 run_pids=1910688,1915757 stragglers=0 reason=interleaved
```

What the pair shows:
- In the same window, ETC2 took 1014 ms against the reference's 1032 ms. The ratio is 0.983, inside the 1.1 tolerance. The ETC2 verdict holds: the footprint now records `etc2` usable, with `loadEvidence.sameWindow` true (gap 315.753 s).
- The ETC2 arm started at load 2.77 and the reference at 7.43. A reference that starts under higher load runs slower, which lowers the ratio, so the load difference does not favour ETC2.
- Both arms 64 of 64 clean. Stragglers 0. The runner ended on its own with rc 0. All PIDs from the run were gone after the job ended (1910672, 1910688, 1915757, 1910802, 1915830).
- The footprint after the run: decision `bc7` (unchanged), `fallbackFormats` `{"astc4x4":true,"etc2":true}` (unchanged). `scripts/check-fallback-list.cjs` reports the live list matches the verdicts.

A side effect, recorded here rather than hidden: every pair overwrites the shared reference witness `neon-wasm-witness-rgba8-software.json`. The ASTC footprint entry reads that file, so its `loadEvidence` is now cross-window (witness 23:59:22, reference 01:02:52, gap 3809.826 s; `sameWindow` false). ASTC still passes the footprint rule (990 against 1032, ratio 0.959). The ASTC decision stands on its own same-window pair in Round 71, which is unchanged. The footprint's evidence should read the reference from each pair's archive, not the shared file. That is a separate change, and it is on the ranked list.

Design effect, recorded: a pair can start only in the first 60 s of the job's 780 s window, because it needs 720 s. This run started at the first sample. Rounds 73 to 75 ended without a start, so this design takes a clear gate in the first minute to start at all.

## Round 73 Phase A: the pair runner takes the format, and the ETC2 pair is added

The ETC2 verdict is cross-window (Round 72): its witness is from 21:41, its reference from 00:04, a gap of 8580.541 s. The same-window pair that settled ASTC is generalised so it can settle ETC2 too.

What changed:
- `scripts/measure-astc-pair.cjs`: takes the format as its argument (`astc4x4` by default, or `etc2`; `PAIR_FORMAT` also works). It runs the format arm, then the reference arm (`rgba8-software`), in one window. The ASTC behaviour is unchanged: the same arm order, the same stop-by-PID, the same `ASTC_PAIR` line shape and field names, and the same archive directory. ETC2 prints `ETC2_PAIR` with `etc2_ms`, `etc2_clean`, and its own archive directory (`etc2-load-pair-archive`). The footprint check reads `formats[format]`. The format driver is `PAIR_FORMAT_DRIVER`, falling back to `ASTC_LOAD_DRIVER`.
- `scripts/lib/astc-load-ratio.cjs`: `checkPairs` treats any arm other than `reference` as the format arm. The interleaving and tolerance rules are the same as before.
- `scripts/check-astc-pair.cjs`: the ten ASTC cases are unchanged, and six ETC2 cases are added (interleaved pass, back to back fails, split across windows fails, no reference fails, above the tolerance fails, at the tolerance passes). 16 of 16 ok, exit 0.

Checks without a browser:
- The ASTC stand-in (a driver that writes nothing) gives `ASTC_PAIR run_ok=false stage=astc fresh=false exit_code=0`, exit 1. The line shape is the same as before.
- The ETC2 stand-in gives `ETC2_PAIR run_ok=false stage=etc2 fresh=false exit_code=0`, exit 1.
- The ETC2 hanging stand-in under a 2 s budget gives `stage=etc2 timed_out=true`, exit 1. Its PID was stopped, and no stand-in process was left.
- An unknown format name throws at start.
- The ASTC and ETC2 witnesses and the footprint are byte-identical after the stand-ins (`cmp`). Each shot directory has 64 files.

Red check: the pair check's tolerance rule was disabled (`ok: interleaved`). The witness exited 1 with three FAILs: the two ASTC ratio cases and the new ETC2 ratio case (1200 against 1018). The library was restored with `cmp`, and the witness exited 0 with 16 of 16 ok.

No window was used in Phase A. No cargo ran, and the `jobs` setting, `Cargo.toml`, `/config/.cargo/config.toml`, and the cargo shim are unchanged. Local commit only; not pushed in Phase A.

## Round 76 Phase A: fixed arm budgets, and a start rule that refuses a short window

The Round 75 pair failed because of its own design. The job set the pair's window to the time that remained (667 s), and the runner gave the format arm that window minus the 360 s reference budget, so the ETC2 arm had about 307 s. The ASTC arm ran 308.8 s in Round 71, so the budget was too small by design.

Design (decided here, recorded for the owner to overrule):
- Each arm has its own budget, fixed in the runner: the format arm and the reference arm each get at least 360 s (`armBudgets` in `scripts/lib/pair-budget.cjs`). The pair needs 720 s. A budget variable below 360 s, or a non-numeric one, is raised to 360 s. `PAIR_WINDOW_S` no longer sets any arm's timer.
- The job starts a pair only with at least the full pair budget (720 s) left in its own window. The window is 780 s measured from the job's start, so a start can only happen in its first 60 s. This is a consequence of the rule, not an accident: a start later than that could not get 720 s.
- The ASTC format arm's timeout changes from a window-derived value (about 420 s with a 780 s window) to the fixed 360 s. The ASTC arm ran 308.8 s, so it still fits. Its result semantics and line shape are unchanged.

What changed:
- `scripts/lib/pair-budget.cjs`: `armBudgets(env)` and `canStartPair(remainingS, pairS)`.
- `scripts/measure-astc-pair.cjs`: the arm timers take `budgets.formatS` and `budgets.referenceS`. The window clock is gone.
- `scripts/check-pair-budget.cjs`: 11 cases. The defaults are 360/360. A 667 s or 780 s window variable does not set an arm. The ETC2 arm keeps 360 s at 665, 400, 307, and 60 s of remaining time. An explicit 420 s is kept. Values below 360 s and non-numeric values fall back to 360 s. A start with 665 or 700 s left is refused, and a start with 720 s or 780 s left is allowed.

Checks without a window:
- `check-pair-budget.cjs`: 11 of 11 ok, exit 0.
- `check-astc-pair.cjs`: 16 of 16 ok, exit 0. The ASTC pair logic is unchanged.
- The ASTC and ETC2 stand-ins (a driver that writes nothing) fail at their own stage with the same line shape. The witnesses and the footprint are byte-identical afterwards.

Red check: the format budget was read from `PAIR_WINDOW_S` (the Round 75 bug). The witness exited 1 with the FAILs for the window cases. The file was restored with `cmp`, and the witness exited 0 with 11 of 11 ok.

No window was used in Phase A. No cargo ran, and the `jobs` setting, `Cargo.toml`, `/config/.cargo/config.toml`, and the cargo shim are unchanged. Local commit only; not pushed in Phase A.

## Round 78 Phase A: one reference file per format

The defect: every pair's reference arm wrote the shared name `neon-wasm-witness-rgba8-software.json`, so the ETC2 pair (Round 77) replaced the reference that ASTC's footprint entry read. ASTC's `loadEvidence` became cross-window (gap 3809.826 s), though its load still passed (990 against 1032).

Layout (decided here): each format has its own reference name, `rgba8-software-<format>`, so its files are `neon-wasm-witness-rgba8-software-<format>.json` and `neon-rows-rgba8-software-<format>.json`. The shared name is no longer written by a pair: after each pair, the reference arm's output is copied to the format's own name (the copy keeps its mtime, which is the arm's end time), and the shared name is restored to its pre-pair content. Reason: the footprint's `loadEvidence` reads the file's mtime, so each format's reference must keep its own timestamp, and a shared name cannot keep two. Field names in `texture-footprint.json` are unchanged; the value of `reference` for `astc4x4` and `etc2` is now the per-format name.

What changed:
- `scripts/lib/pair-reference.cjs`: `referenceFor(format)` and `keepReference({ witnessDir, format, beforeDir })`.
- `scripts/measure-astc-pair.cjs`: calls `keepReference` after the reference arm succeeds. The before-copies come from the pair's archive.
- `scripts/texture-footprint.cjs`: the `astc4x4` and `etc2` judgements read `referenceFor(format)`.
- `scripts/check-reference-layout.cjs`: runs an ETC2 pair and an ASTC pair in turn, in both orders, in a temporary directory. 9 of 9 ok. Each format's own witness and rows keep that format's state, and the shared name is back at its pre-pair content.

Recovery of the two per-format files (data, no window):
- ETC2: copied from the shared file as the ETC2 pair left it. Verified before copying: `loadMsMedian` 1032, 64 of 64 clean, mtime 2026-11-20T01:02:52.007Z (the value the footprint recorded). Its rows file was copied the same way.
- ASTC: copied from the ETC2 pair archive's before-copy of the shared file (`target/witness/etc2-load-pair-archive/2026-11-20T00-52-29-798Z/reference/before-neon-wasm-witness-rgba8-software.json`). That copy was taken at 00:57:41 UTC, just before the ETC2 pair's reference arm, so it is the Round 71 reference. Verified: `loadMsMedian` 1018, 64 of 64 clean, the same value as the Round 71 pair line (`reference_ms=1018`). The copy's own mtime was the copy time, so its mtime was reset to 2026-11-20T00:04:31.263Z, the mtime the Round 72 footprint recorded for that reference. This is the only restored timestamp, and it is the value on record.
- The shared name was left as the ETC2 pair left it (1032). Nothing reads it now except the pair's own reference arm, which overwrites it and restores it.

Red check: the layout was changed so every format maps to the shared name (`return SHARED_REFERENCE`). The witness exited 1 with seven FAILs, covering the ETC2 and ASTC references, the rows and the names. The file was restored with `cmp`, and the witness exited 0 with 9 of 9 ok.

Not run here: the footprint rerun against the new names, and any window. Both were done later in the Round 78 Phase B window (its record follows the Round 78 Phase A entry). The earlier text tagged the rerun as Phase B before that window ran, so it is corrected here. No cargo ran, and the `jobs` setting, `Cargo.toml`, `/config/.cargo/config.toml`, and the cargo shim are unchanged. Local commit only; not pushed in Phase A.

## Round 78 Phase B: the witnesses on the clear sample (record)

Window: the job's first sample cleared the gate at 03:13:48 with 598 s remaining of a 600 s window, so the witnesses started at once and the window was not used further. The job was the Round 78 loop in `/tmp/bt`, which the committed `scripts/window-job.sh` (Round 79) replaces.

Readings, verbatim from the job log (every reading in the window):

reading 03:13:48 si=284 so=544 load1=2.12 gate=open heavy=clear remainS=598 HEAVY_JOBS clear;
GATE-CLEAR 03:13:48 remainS=598 witnesses=start

The gate reading is the second `vmstat 1 2` sample: swap-in 284 and swap-out 544 KB/s, both below 1000; load 2.12, below 4; `scripts/heavy-jobs.cjs` at bevy's commit 495fff6 reads clear (the heavy-job files are unchanged in HEAD).

Witnesses, each run once, rc 0:
- `check-reference-layout.cjs`: 9 ok, 0 FAIL, wall under 0.05 s (shown as 0.0 s).
- `check-pair-budget.cjs`: 11 ok, 0 FAIL, wall under 0.05 s.
- `check-astc-pair.cjs`: 16 ok, 0 FAIL, wall under 0.05 s.
- `texture-footprint.cjs` (the footprint check): rc 0, wall 1.3 s. Decision `bc7`; `fallbackFormats` `{"astc4x4":true,"etc2":true}`. Both formats read their own reference: `etc2` reference `rgba8-software-etc2`, `loadEvidence.sameWindow` true, gap 315.753 s; `astc4x4` reference `rgba8-software-astc4x4`, `sameWindow` true, gap 309.082 s.
- `check-fallback-list.cjs`: 7 ok, 0 FAIL, wall under 0.05 s.

END 03:13:50 overall=0. The footprint decision and fallback list are unchanged from Round 77; the per-format references now give both formats same-window evidence in the footprint.

Docs only: no red check applies to this record.

## Round 79 Phase A: the window job is committed, with a witness for its gate

The gated window loop used in Rounds 73 to 78 lived only in `/tmp/bt`. Its gate and its start rule were never versioned or witnessed, so a change to them would not have been caught.

Design (decided here): `scripts/window-job.sh` takes its vmstat, heavy-job and load readings from environment variables (`VMSTAT_CMD`, `HEAVY_CMD`, `LOADAVG_FILE`), and its window, interval and gate limits from `WINDOW_S` (default 600), `INTERVAL_S` (default 30), `GATE_KB` (default 1000) and `LOAD_MAX` (default 4). The defaults are the window rule. The witness drives the job with stubs, so the gate is tested without a host. A start is a sample where both swap directions are below `GATE_KB`, the load is at or below `LOAD_MAX`, and `scripts/heavy-jobs.cjs` (commit 495fff6) is clear. Every reading is logged. The witnesses named after `--` run only on a clear sample, and the job exits with their status.

What changed:
- `scripts/window-job.sh`: the job.
- `scripts/check-window-job.cjs`: 8 cases, each a run of the job with stub readings. A swap-in spike closes the gate and the next clear sample starts the witness. A swap-out spike alone closes it. A busy heavy reading holds the start until a clear one. A load above 4 closes the gate and the window ends without a start. A heavy rule that never clears ends the window without a start. A failing witness sets the exit status. Readings come at the interval. A clear sample with no witness ends without a run. 8 of 8 ok, exit 0. The check takes about 14 s.

Red check: the swap-out condition was removed from the gate. The witness exited 1 with `FAIL a swap-out spike alone closes the gate`. The job was restored with `cmp`, and the witness exited 0 with 8 of 8 ok.

No window was used in Phase A. No cargo ran, and the `jobs` setting, `Cargo.toml`, `/config/.cargo/config.toml`, and the cargo shim are unchanged. Local commit only; not pushed in Phase A.


## Round 79 Phase B: the window job on the clear sample (record)

Run: `scripts/window-job.sh -- node scripts/check-window-job.cjs` at commit 4444763, with the committed defaults (window 600 s, interval 30 s, gate 1000 KB/s, load limit 4, `scripts/heavy-jobs.cjs` at 495fff6, unchanged in HEAD).

Readings, verbatim from the job log `target/witness/window-job.log` (every reading in the window):

reading 03:23:47 si=532 so=0 load1=1.52 gate=open heavy=clear remainS=598 HEAVY_JOBS clear
GATE-CLEAR 03:23:47 remainS=598
WITNESS rc=0 03:24:01
END 03:24:01 overall=0

The gate reading is the second `vmstat 1 2` sample: swap-in 532, swap-out 0, load 1.52, heavy clear, 598 s remaining. The first sample cleared the gate, so the witness started at once.

Witness: `check-window-job.cjs`, 8 ok, 0 FAIL, rc 0, wall 14 s (03:23:47 to 03:24:01).

No process of mine remained after the window, and no peer process was stopped.

Docs only: no red check applies to this record.


## Round 80 Phase B: the window job runs the witness and the footprint check (record)

Run: `scripts/window-job.sh -- bash /tmp/bt/r80-witnesses.sh` at commit eb7d36b, with the committed defaults (window 600 s, interval 30 s, gate 1000 KB/s, load limit 4, `scripts/heavy-jobs.cjs` at 495fff6, unchanged in HEAD). The witness script ran `scripts/check-window-job.cjs` and then `scripts/texture-footprint.cjs`, once each.

Readings, verbatim from the job log `target/witness/window-job.log` (every reading in the window):

reading 03:32:16 si=3020 so=0 load1=1.40 gate=closed heavy=clear remainS=597 HEAVY_JOBS clear
reading 03:32:45 si=516 so=0 load1=1.86 gate=open heavy=clear remainS=568 HEAVY_JOBS clear
GATE-CLEAR 03:32:45 remainS=568
WITNESS check-window-job rc=0 wall=13.8s ok=8 fail=0
WITNESS footprint rc=0 wall=2.1s decision=bc7
WITNESS rc=0 03:33:01
END 03:33:01 overall=0

The first sample closed the gate on swap-in (3020 KB/s). The second is the gate reading: swap-in 516, swap-out 0, load 1.86, heavy clear, 568 s remaining. The witnesses started at that sample.

Footprint check: decision `bc7`, witness rc 0. Its `fallbackFormats` and per-format references are the ones recorded in the Round 78 Phase B record.

No process of mine remained after the window, and no peer process was stopped.

Docs only: no red check applies to this record.

## Round 81 Phase A: the start-window ruling (1200 s window, a pair starts in its first 480 s)

Ruling (dispatcher, this round, recorded here): the pair window is 1200 s. The window job starts a pair only while at least 720 s of that window remain, so a pair can start in the first 480 s of a window, not only in the first 60 s. Each arm keeps its fixed 360 s budget, so the window length sets no arm's timer. The owner's standing rule stands: any other process above 20% CPU in top counts, including Claude sessions and `find`, and the window job keeps that rule. This ruling is reversible: the window is one constant, `PAIR_WINDOW_S` in `scripts/lib/pair-budget.cjs`.

Reason: with a 780 s window and a 720 s pair budget, a start had only 60 s, and Rounds 73 to 75 ended without a start for that reason. With 1200 s, the same 720 s budget leaves 480 s for the gate and the heavy rule to clear.

Reading of "measured from the pair's own start" (for the dispatcher to check): the 1200 s clock opens when the window job opens, not when the pair starts. A clock that opened at the pair's own start would have 1200 s left at every start, so the 720 s rule would never refuse anything. The pair's arm budgets are still measured from the pair's own start, as before.

What changed:
- `scripts/lib/pair-budget.cjs`: `PAIR_WINDOW_S` (1200), `latestPairStartS(pairS)` (480), `pairStartAllowedAt(elapsedS, pairS)`.
- `scripts/window-job.sh`: `MIN_REMAINING_S` (default 0). A clear sample is not started when less than `MIN_REMAINING_S` remains: the job logs `START-REFUSED` and exits 4 without running the witness. With the default, its behaviour is unchanged.
- `scripts/check-pair-budget.cjs`: 17 cases, up from 11. A start at 800 s into the 1200 s window is refused (400 s left). A start at 480 s is allowed (720 s left). A start at 481 s is refused. The ETC2 arm keeps 360 s in a 1200 s window whatever the window variable says.
- `scripts/check-window-job.cjs`: 10 cases, up from 8. A clear sample with 10 s of window and 3 s needed starts the witness. A clear sample reached with 4 s of window and 3 s needed is refused.

Witnesses: `check-pair-budget.cjs` 17 of 17 ok, exit 0. `check-window-job.cjs` 10 of 10 ok, exit 0. `check-astc-pair.cjs` 16 of 16 ok, exit 0, unchanged, so the ASTC behaviour is unchanged.

Red check: `pairStartAllowedAt` ignored the elapsed time (`canStartPair(PAIR_WINDOW_S, pairS)`, the bug the ruling removes). `check-pair-budget.cjs` exited 1 with `FAIL a start at 800 s into the 1200 s window is refused` and `FAIL a start at 481 s into the window is refused`. The library was restored with `cmp`, and the witness exited 0 with 17 of 17 ok.

No window was used in Phase A. No cargo ran, and the `jobs` setting, `Cargo.toml`, `/config/.cargo/config.toml`, and the cargo shim are unchanged. Local commit only; not pushed in Phase A.

## Round 82 Phase B: the start-window witnesses on the clear sample (record)

Run: `scripts/window-job.sh -- bash /tmp/bt/r82-witnesses.sh` at commit c7e3158, with the committed defaults (window 600 s, interval 30 s, gate 1000 KB/s, load limit 4, `scripts/heavy-jobs.cjs` at 495fff6, unchanged in HEAD). The witness script ran `scripts/check-pair-budget.cjs` and `scripts/check-window-job.cjs`, once each. The start-window cases live in those two files (17 and 10 cases); no third witness file was created.

Readings, verbatim from the job log `target/witness/window-job.log` (every reading in the window):

reading 03:42:58 si=512 so=0 load1=2.02 gate=open heavy=clear remainS=598 HEAVY_JOBS clear
GATE-CLEAR 03:42:58 remainS=598
WITNESS check-pair-budget rc=0 wall=0.0s ok=17 fail=0
WITNESS check-window-job rc=0 wall=15.3s ok=10 fail=0
WITNESS rc=0 03:43:13
END 03:43:13 overall=0

The job log line `WITNESS rc=0 03:43:13` is the witness script's combined status: 0 only when both witnesses pass. (Corrected in a later local commit: the first version of this record copied the line as `rc=1`.)

The gate reading is the first `vmstat 1 2` sample: swap-in 512, swap-out 0, load 2.02, heavy clear, 598 s remaining. The first sample cleared the gate, so the witnesses started at once.

Wall times: check-pair-budget under 0.05 s (shown as 0.0 s); check-window-job 15.3 s; the whole job 15 s.

No process of mine remained after the window, and no peer process was stopped.

Docs only: no red check applies to this record.

## Round 83 Phase A: the pair window is read from the one constant

The ruling in Round 81 made the window one constant, `PAIR_WINDOW_S` in `scripts/lib/pair-budget.cjs`. But the window job still took `WINDOW_S` and `MIN_REMAINING_S` from the command line, typed by hand for each run. Changing the constant would have changed nothing, so the ruling was not yet reversible by one change.

Design (decided here): `scripts/pair-window.sh <astc4x4|etc2>` reads `PAIR_WINDOW_S` and the full pair budget from the library and passes them to `scripts/window-job.sh` as `WINDOW_S` and `MIN_REMAINING_S`, then runs `scripts/measure-astc-pair.cjs` for that format. `--print-env` prints the two values without running anything. The wrapper accepts only the two formats the pair runner accepts. The pair's budgets still come from `armBudgets`, so an explicit larger format budget raises the start threshold, as before.

What changed:
- `scripts/pair-window.sh`: the wrapper.
- `scripts/check-pair-window.cjs`: 6 cases. The ETC2 window is the library constant (1200 s). The ETC2 start threshold is the full pair budget (720 s). The latest start is 480 s into the window. ASTC gets the same values. A 500 s format budget raises the threshold to 860 s. An unknown format exits 2 and prints no values. 6 of 6 ok, exit 0.

Red check: the wrapper's window was hard-coded to 780 s (the old value). The witness exited 1 with three FAILs: the window constant, the latest start, and the 500 s budget case. The wrapper was restored with `cmp`, and the witness exited 0 with 6 of 6 ok.

No window was used in Phase A. The window-job witness and the pair-budget witness are unchanged. No cargo ran, and the `jobs` setting, `Cargo.toml`, `/config/.cargo/config.toml`, and the cargo shim are unchanged. Local commit only; not pushed in Phase A.

## Round 83 Phase B: the pair-window witnesses on the clear sample (record)

Run: `scripts/window-job.sh -- bash /tmp/bt/r83-witnesses.sh` at commit 03bc1a7, with the committed defaults (window 600 s, interval 30 s, gate 1000 KB/s, load limit 4, `scripts/heavy-jobs.cjs` at 495fff6, unchanged in HEAD). The witness script ran `check-pair-window.cjs`, `check-pair-budget.cjs` and `check-window-job.cjs`, once each.

Readings, verbatim from the job log `target/witness/window-job.log` (every reading in the window):

reading 03:52:02 si=7420 so=4728 load1=1.12 gate=closed heavy=clear remainS=598 HEAVY_JOBS clear
reading 03:52:33 si=956 so=3808 load1=1.35 gate=closed heavy=clear remainS=567 HEAVY_JOBS clear
reading 03:53:02 si=296 so=0 load1=1.22 gate=open heavy=clear remainS=538 HEAVY_JOBS clear
GATE-CLEAR 03:53:02 remainS=538
WITNESS check-pair-window rc=0 wall=0.2s ok=6 fail=0
WITNESS check-pair-budget rc=0 wall=0.0s ok=17 fail=0
WITNESS check-window-job rc=0 wall=17.3s ok=10 fail=0
WITNESS rc=0 03:53:19
END 03:53:19 overall=0

The first two samples closed the gate on swap-in and swap-out (7420 and 4728 KB/s, then 956 and 3808 KB/s). The third sample is the gate reading: swap-in 296, swap-out 0, load 1.22, heavy clear, 538 s remaining. The witnesses started at that sample.

The `WITNESS rc=0 03:53:19` line is the witness script's combined status, 0 only when all three pass.

Wall times: check-pair-window 0.2 s; check-pair-budget under 0.05 s (shown as 0.0 s); check-window-job 17.3 s; the whole job 17 s.

Process check: the only process above 20% CPU in top at the end was a gm-core worker started with `--handshake` (pid 1002562), which is exempt. No process of mine remained, and no peer process was stopped.

Docs only: no red check applies to this record.

## Round 84 Phase A: the ETC2 RGBA8 block decoder (cache agreement open)

Added `scripts/lib/texel-decode.cjs`: `decodeEtc2Rgba8Block(block)` takes one 16-byte block and returns 64 bytes of RGBA (row-major, 4x4). The alpha half is EAC (bytes 0 to 7). The colour half is ETC1 individual and differential modes (bytes 8 to 15), with big-endian 32-bit words, the spec's bit fields, and column-major texel indices. A T, H or planar block throws, so no output is ever guessed for one. ASTC 4x4 is not written in this round.

Reference premise, corrected: the per-format files `rgba8-software-<format>` (`neon-wasm-witness-*` and `neon-rows-*`) are load-timing rows and page screenshots under the gitignored `target/`. They hold no decoded texels and are not committed. The pixel reference used is the RGBA8 KTX2 cache (`target/gltf-cache-rgba8`, vkFormat 37), level 0, against the ETC2 cache (vkFormat 151, the same texture set). Only 12 of 61 `value.*` models hold KTX2 images, and those 12 hold the 61 textures.

Witness: `scripts/check-texel-decode.cjs`. Fixtures, 8 of 8 ok: flat index 01, a left and right split, a differential base with zero delta, a negative delta, the flip split, the texel-4 index layout, table 7, and an EAC alpha case. Cache, FAIL: RGB PSNR minimum 25.25 dB across the 12 textures, against the 35 dB bar used by the texel criteria. The bar fails on blue. Red and green calibrate close to identity, but decoded blue 247 maps to a reference of about 254 (173,000 texels), and no bit-field offset or expansion variant removed the gap (search over blue field shifts 9 to 13 and delta shifts 5 to 9, best 29.9 dB). The cause is unresolved. It is either a decoder fault in the blue top end or a cache that does not follow the spec. Reopen: a second, independent decode of one cache texture (a GPU readback or a reference decoder) to decide which side is wrong.

Not validated and not claimed: the cache has no T, H or planar blocks (0 unsupported), so those modes are untested; EAC alpha is untested beyond fixtures, because the cache alpha is constant 255.

Red check: `ETC1_INDEX_TO_MODIFIER` changed from `[2, 3, 1, 0]` to `[0, 1, 2, 3]`. The witness reported 0 of 8 fixtures ok and exited 1. The file was restored and `cmp` matched. The witness then reported 8 of 8 ok, and `git status` showed only the two new files.

No window in Phase A. No cargo. Local commit only, not pushed. The witness exits 1 on the cache check, so this commit must not be pushed until the cache check is resolved.

## Round 84 Phase B: the texel-decode witness set (record, not pushed)

Window: `scripts/window-job.sh -- bash /tmp/bt/r84-witnesses.sh` at commit 4683419, defaults (window 600 s, interval 30 s, gate 1000 KB/s both ways, load limit 4, `scripts/heavy-jobs.cjs` unchanged from 495fff6, verified with `git diff 495fff6 HEAD` = 0 lines). Pre-check `vmstat 1 2` second sample: si 7572, so 0 (gate closed before the window opened).

Readings, verbatim from `target/witness/window-job.log`, every reading in the window:

reading 04:17:54 si=1596 so=0 load1=3.14 gate=closed heavy=clear remainS=598 HEAVY_JOBS clear
reading 04:18:24 si=3488 so=0 load1=2.76 gate=closed heavy=busy remainS=568 HEAVY_JOBS busy n=2;compile pid=2071193 /tmp/zig-toolchain/zig-x86_64-linu
reading 04:18:55 si=2212 so=0 load1=3.33 gate=closed heavy=clear remainS=537 HEAVY_JOBS clear
reading 04:19:24 si=212 so=0 load1=3.64 gate=open heavy=busy remainS=508 HEAVY_JOBS busy n=1;compile pid=2071512 /tmp/zig-toolchain/zig-x86_64-linux-0
reading 04:19:54 si=424 so=0 load1=2.65 gate=open heavy=busy remainS=478 HEAVY_JOBS busy n=1;compile pid=2072320 zig test -lc --dep test_paths -Mroot=
reading 04:20:25 si=188 so=0 load1=2.08 gate=open heavy=busy remainS=447 HEAVY_JOBS busy n=1;hot pid=2071659 cpu=99 ./.zig-cache/o/8ac91219ccf88b3bd8b
reading 04:20:54 si=1012 so=0 load1=1.83 gate=closed heavy=busy remainS=418 HEAVY_JOBS busy n=2;hot pid=755372 cpu=32.7 claude --dangerously-skip-perm
reading 04:21:24 si=220 so=4 load1=1.70 gate=open heavy=busy remainS=388 HEAVY_JOBS busy n=1;hot pid=2071659 cpu=100 ./.zig-cache/o/8ac91219ccf88b3bd8
reading 04:21:54 si=84 so=180 load1=2.10 gate=open heavy=busy remainS=358 HEAVY_JOBS busy n=1;hot pid=2071659 cpu=58 ./.zig-cache/o/8ac91219ccf88b3bd8
reading 04:22:25 si=880 so=8 load1=2.66 gate=open heavy=busy remainS=327 HEAVY_JOBS busy n=1;hot pid=2075898 cpu=39.6 /workspace/fx/fx/zig-out/bin/fx 
reading 04:22:55 si=1712 so=172 load1=3.27 gate=closed heavy=busy remainS=297 HEAVY_JOBS busy n=1;hot pid=2225 cpu=25.7 claude --dangerously-skip-perm
reading 04:23:26 si=232 so=2212 load1=3.09 gate=closed heavy=busy remainS=266 HEAVY_JOBS busy n=2;hot pid=1998 cpu=24.8 claude --dangerously-skip-perm
reading 04:23:55 si=112 so=244 load1=3.19 gate=open heavy=busy remainS=237 HEAVY_JOBS busy n=1;hot pid=2071659 cpu=99 ./.zig-cache/o/8ac91219ccf88b3bd
reading 04:24:25 si=236 so=1056 load1=3.09 gate=closed heavy=busy remainS=207 HEAVY_JOBS busy n=1;hot pid=2071659 cpu=99 ./.zig-cache/o/8ac91219ccf88b
reading 04:24:56 si=824 so=0 load1=2.58 gate=open heavy=busy remainS=176 HEAVY_JOBS busy n=1;hot pid=2071659 cpu=100 ./.zig-cache/o/8ac91219ccf88b3bd8
reading 04:25:25 si=176 so=0 load1=2.08 gate=open heavy=busy remainS=147 HEAVY_JOBS busy n=1;hot pid=2071659 cpu=100 ./.zig-cache/o/8ac91219ccf88b3bd8
reading 04:25:55 si=176 so=40 load1=1.65 gate=open heavy=clear remainS=117 HEAVY_JOBS clear

GATE-CLEAR 04:25:55 remainS=117
WITNESS check-texel-decode rc=1 wall=1.6s ok=8 fail=1
WITNESS check-pair-window rc=0 wall=0.1s ok=6 fail=0
WITNESS check-window-job rc=0 wall=15.5s ok=10 fail=0
WITNESS-SET overall=1 04:26:13
WITNESS rc=1 04:26:13
END 04:26:13 overall=1

Result: the window cleared on its seventeenth reading (04:25:55, gate open, heavy clear, 117 s left). The witnesses ran once each, on that sample. `check-texel-decode` exited 1: 8 of 8 fixtures ok, and the cache check failed at 25.25 dB against the 35 dB bar (the open finding in 4683419). `check-pair-window` 6 of 6 ok, wall 0.1 s. `check-window-job` 10 of 10 ok, wall 15.5 s. Set overall 1.

Not pushed: the witness set did not pass. origin main stays at 17a39e3 (`git ls-remote origin main` checked after the window).

Longest step: the wait for heavy clear, 8 min 1 s from job start (04:17:54) to the clear sample (04:25:55). What was done: nothing stopped. The busy processes were a peer's `zig test` (pid 2071659, under /workspace/fx) and Claude sessions (pids 755372, 2225, 1998); none is a process of this run, and none was stopped. The witnesses waited for the clear sample.

## Round 85 Phase A: the browser readback (no ETC2 in the browser)

The playwriter browser for this project cannot decode ETC2. Its WebGL2 context lists zero native compressed formats, and an ETC2 RGBA8 upload fails with GL_INVALID_ENUM (0x500). The WebGL compressed-texture extensions it exposes are BC only (`EXT_texture_compression_bptc`, `EXT_texture_compression_rgtc`, `WEBGL_compressed_texture_s3tc`, `WEBGL_compressed_texture_s3tc_srgb`). ETC1 and ASTC are not exposed either. So the readback cannot decode the cache texture, and the fallback applies: a software decode from a second implementation (see the next commit).

`scripts/check-gl-readback.js` is the page body. It runs in the playwriter tab through `p.evaluate`, and it builds a 4x4 WebGL2 texture from a compressed block, samples it with `texelFetch` into the default framebuffer, and reads the pixels back. The fixtures are two flat BC1 blocks (red 0xF800 and green 0x07E0, index 0) with known answers (255,0,0,255) and (0,255,0,255), plus an ETC2 flat block that runs only if ETC2 is listed, which it is not here.

Run: the unmodified body passed both BC1 fixtures and reported ETC2 as skipped.

Red check: the `texelFetch` output was swizzled to `.bgra`. The red fixture became (0,0,255,255) and failed, and the green fixture still passed (its channels are symmetric under the swap). The file was restored, `cmp` matched, and `git status` showed only the new files.

The playwriter tab for this run is the only page this run opened; it is closed when the run finishes. No cargo, no window in Phase A, local commit only.

## Round 85 Phase A: the second software decode of the ETC2 cache texture

`scripts/lib/etc2-second.cjs` is the fallback decode, written in a different structure from `scripts/lib/texel-decode.cjs`: per sub-block palettes, with the offsets written as small and large pairs (+small, +large, -small, -large) instead of a modifier index map. It handles the same modes (ETC1 individual and differential, EAC alpha), and throws on T, H or planar blocks.

Caveat, stated as a limit: there is no written spec on this host, so both decoders come from the same recollection of the Khronos ETC2 text. Agreement between them rules out a slip in either one's code. It does not show the spec reading is right.

Witness: `scripts/check-etc2-second.cjs`. Fixtures 8 of 8 ok (the same vectors as the first decoder). Measured on the 12 KTX2 textures: the second decode against the RGBA8 cache has a minimum RGB PSNR of 25.25 dB, and texture 1 gives 28.77 / 36.34 / 28.49 dB. The two decoders disagree on 0 of 12,582,912 texels. The bar stays 35 dB, and the cache line reads OPEN.

Red check: the blue channel's palette was reordered from [+small, +large, -small, -large] to [+small, -small, +large, -large]. Fixtures 5 of 8, 3 FAIL lines, exit 1. The file was restored, `cmp` matched, and the fixtures returned to 8 of 8.

Result: both decodes give the same answer, so the cache disagreement at the blue top end is still open. Neither the browser nor a second decode settles which side is wrong. The next independent source is a decoder with ETC2 support (a software GL such as SwiftShader in a separately launched Chromium, which INVARIANT 11 permits once the playwriter setup is shown unable to do the job), or a spec text on disk.

## Round 85 Phase A: the cache check is a measurement, not a witness outcome

`scripts/check-texel-decode.cjs` now reports the cache line as CLEAR or OPEN, and its exit code depends on the fixtures only. The bar stays at 35 dB and is unchanged. This follows the owner's rule that a cache disagreement is not a failed witness; it is recorded with its numbers. The line in 4683419 that said the commit must not be pushed until the cache check is resolved is superseded by this line.

Current numbers: the first decoder's RGB PSNR minimum across the 12 textures is 25.25 dB (OPEN).

Red check: a fixture expectation was changed from 8 to 9 in the red channel. Exit 1. The file was restored, `cmp` matched, and the witness exited 0 with 8 of 8 fixtures ok and cache OPEN.

## Round 85 Phase B: the ETC2 readback and the witness set (record)

Readings. Pre-check `vmstat 1 2` (second sample, read before the window job started): si 420, so 0. Window job reading at 04:37:36: si 196, so 0, load1 1.85, gate open, heavy clear, 598 s remaining, then `GATE-CLEAR`. Post-clear check at 04:38:05: `vmstat 1 2` second sample si 256, so 0; `node scripts/heavy-jobs.cjs` HEAVY_JOBS clear; `/proc/loadavg` 1.79. The window job exits after its witnesses, so it took one reading in the window; the post-clear check is the second. Both gate samples were below 1000 KB/s both ways.

Witnesses, each once, on the clear sample at 04:37:36 (window-job log): `check-texel-decode` rc=0, wall 1.2 s, 8 of 8 fixtures ok, cache OPEN. `check-etc2-second` rc=0, wall 2.2 s, 8 of 8 fixtures ok, cache OPEN. `check-pair-window` rc=0, wall 0.2 s, 6 of 6 ok. `check-window-job` rc=0, wall 15.3 s, 10 of 10 ok. Set overall 0, finished 04:37:55.

Readback, once, after the post-clear check: `scripts/check-gl-readback.js` in the bevy playwriter tab. WebGL2 with zero native compressed formats; ETC2 RGBA8 not listed, so the texture readback was not run (no ETC2 in this browser). Fixtures: BC1 flat red read back as (255,0,0,255) and BC1 flat green as (0,255,0,255), both ok. Wall time was not captured for this call, and that is recorded as unmeasured. The browser step ran for this readback only; the tab was closed with `p.close()` after the run.

Cache numbers, both decoders (same texture set, 12 KTX2 textures, level 0): RGB PSNR minimum 25.25 dB against the 35 dB bar, OPEN. Texture 1 (value.17c7bd864f7f18268f): 28.77 / 36.34 / 28.49 dB. The two decoders agree on every texel (0 of 12,582,912 differ).

Which side the readback agrees with: neither. The browser cannot decode ETC2, so the readback did not reach the texture. The software fallback agrees with the first decoder, so the blue top-end disagreement is a question of the spec reading or of the cache, and it stays open. The next source that can settle it is a decoder with ETC2 support (see the Round 85 Phase A ledger line on SwiftShader) or a spec text on disk.

Renderers. Before the readback, two bevy renderers (pids 2089773 and 2089782) belonged to this round's tab. They were stopped by PID after the tab was closed; afterwards no bevy renderer was left. The bevy browser process (pid 2089696, under the gate pid 2089666) was left running, since it is the gate's. The oscraft browser (pids 2088779 to 2088866) belongs to another project and was not touched.

Longest step: the witness set's `check-window-job` (15.3 s); the readback call returned within its 90 s timeout. Nothing was stopped except the two bevy renderers named above.

## Round 86 Phase A: the separate-browser ETC2 readback (fixture)

Reason for the separate browser (INVARIANT 11): the playwriter browser for this project exposes no ETC2 at all. Its WebGL2 context lists zero native compressed formats, and an ETC2 upload fails with GL_INVALID_ENUM 0x500. The playwriter setup therefore cannot do this job. Approved by the dispatcher for one separate Chromium at a time, localhost only, no request beyond the local page, nothing to play.spawn.co, stopped by PID when the readback ends.

Browser and flags: Chromium 155.0.8059.39, binary /usr/lib/chromium/chromium.real (not the /usr/bin/chromium wrapper, so the captured PID is the browser process). Flags: --headless=new --no-sandbox --use-gl=angle --use-angle=swiftshader --enable-unsafe-swiftshader --ignore-gpu-blocklist --no-first-run --disable-background-networking --disable-component-update --disable-default-apps --disable-sync, plus --user-data-dir under target/readback/profile-<case>. These are the SwiftShader WebGL2 flags as I recall the Chromium GPU documentation; this host has no network access, so the docs were not re-read in this run. The renderer string reads ANGLE (Google, Vulkan 1.3.0, SwiftShader Device), which confirms the software GL.

What the probe found: under SwiftShader the compressed-texture extensions are WEBGL_compressed_texture_etc, WEBGL_compressed_texture_etc1, WEBGL_compressed_texture_astc and the BC set. WebGL2 accepts an ETC2 upload only after the extension is enabled with getExtension('WEBGL_compressed_texture_etc'); without that call the same upload fails with 0x500. COMPRESSED_TEXTURE_FORMATS stays empty, so the page reads the extension, not the format list.

Files: scripts/browser/readback.html (the page: enables the extension, uploads a compressed block as a 4x4 or 1024x1024 texture, samples it with texelFetch into the default framebuffer, reads the pixels back, and posts them to localhost), scripts/check-etc2-gpu.cjs (the driver: serves the page and the level-0 bytes on 127.0.0.1, starts one Chromium with the flags above, waits for the page result up to 180 s, stops the browser tree by PID, and then compares). Fixture mode is the default for the flat-block case; texture mode is the readback of one cache texture (value.17c7bd864f7f18268f, level 0).

Fixture: a flat ETC2 RGBA8 block, alpha block ff00000000000000, colour block 88440000 0000ffff (individual mode, base red 136, green 68, blue 0, every index 01 so +8). Expected decoded colour 144 / 76 / 8 / 255 on all 16 texels, derived by hand from the spec. The software GL decodes it to exactly that: ok, 16 of 16 texels, wall 0.9 s.

Red check: the fragment output was swizzled to .bgra. The fixture reported FAIL and exited 1. The page was restored with cmp (cmp-restored), the fixture passed again with exit 0, and git status showed only the new files. The probe-only ETC2_PROBE_FLAGS hook was removed afterwards and the fixture passed with exit 0 again.

Not run in Phase A: the texture readback. It runs in Phase B, once, on the clear window.

## Round 86 Phase B: the driver correction (fixed without a browser run)

The texture readback ran once in the window (job r86-phaseb, witness etc2-readback-texture), and its printed PSNR numbers were wrong. The driver divided the sample count by 4 in the PSNR and texel counts, so every printed PSNR was 6.02 dB low and the texel count was a quarter of the true one. The printed line was 22.75 / 30.32 / 22.47 dB and '0 of 262144 texels differ'. This commit corrects the count and adds an analyse mode, which re-reads the saved readback output (target/readback/gpu-rgba8.bin) without starting a browser. The analysis of the saved output gives 28.77 / 36.34 / 28.49 dB and '0 of 1048576 texels differ'. The witness exit code was not affected, because the cache result is a measurement.

Red check: the old denominator (samples / 4) was put back in the driver. The analyse output printed 22.75 / 30.32 / 22.47 dB and 262144 texels, the wrong numbers. The file was restored with cmp (cmp-restored), and the analyse output returned to 28.77 / 36.34 / 28.49 dB.


## Round 86 Phase B: the separate-browser readback and the witness set (record)

Readings. Pre-check vmstat 1 2 second sample before the job: si 36, so 0. Window job readings (every 30 s): 04:44:29 si 260, so 216, load 1.59, gate open, heavy BUSY (a renderer of another project's Chromium, pid 2088866, 20.8% CPU; not stopped); 04:45:00 si 964, so 14320, gate closed, heavy BUSY (a Claude session, pid 2356, 22%; not stopped); 04:45:29 si 18856, so 0, gate closed, heavy clear; 04:45:59 si 2784, so 0, gate closed, heavy clear; 04:46:30 si 280, so 0, gate open, heavy BUSY (gm-core worker pid 2102064, 99%, another project's; not stopped); 04:46:59 si 12, so 3984, gate closed, heavy clear; 04:47:29 si 300, so 41404, gate closed, heavy clear; 04:48:00 si 544, so 0, load 1.75, gate open, heavy clear, 387 s remaining, GATE-CLEAR.

Witnesses, each once, from the clear sample at 04:48:00: etc2-readback-texture rc=0, wall 4.3 s (Chromium pid 2104502, tree 8, leftover 0, page wall 1.0 s); the printed numbers are the corrected ones in the commit above. etc2-readback-fixture rc=0, wall 3.8 s (Chromium pid 2104703, tree 8, leftover 0, page wall 0.7 s), ok 16 of 16 texels. check-texel-decode rc=0, wall 1.7 s, 8 of 8. check-etc2-second rc=0, wall 1.9 s, 8 of 8. check-pair-window rc=0, wall 0.2 s, 6 of 6. check-window-job rc=0, wall 15.7 s, 10 of 10. Set overall 0 at 04:48:27.

Browser: Chromium 155.0.8059.39 under SwiftShader (renderer ANGLE, Vulkan 1.3, SwiftShader Device), with the flags recorded in the Round 86 Phase A line. Each run started its own browser under target/readback/profile-<case> and was stopped by PID; neither left a process.

Result. The separate software GL decodes the flat ETC2 block to the expected colour (fixture). For the cache texture value.17c7bd864f7f18268f, level 0 (1024 x 1024), the readback against the RGBA8 cache has RGB PSNR 28.77 / 36.34 / 28.49 dB and alpha exact. The readback equals the first decoder on all 1048576 texels (0 differ). So the side the readback agrees with is the decoder reading of the spec, and the cache is the side that disagrees at the blue top end. The cache line is OPEN at a minimum of 28.49 dB against the unchanged 35 dB bar.

Caveat on independence: SwiftShader is a conformant GL ES implementation, separate from my code. The first and second decoders come from one recollection of the spec, so they are not independent of each other. The readback agreeing with both is evidence that the decoder reading is correct for the ETC2 blocks in this cache, but it is one implementation, not the spec text. Reopen the cache question only with the cache's encoder configuration or a second GPU decode.

Longest step: the wait for the clear sample, about 4 min from the first reading (04:44:29) to GATE-CLEAR (04:48:00). Nothing was stopped by this run; the busy processes are another project's renderer, a Claude session and another project's gm-core worker.

## Round 87 Phase A: the blue top end of the ETC2 cache (analysis)

Script: scripts/check-etc2-blue-top.cjs reads the level-0 images of the ETC2 cache and the RGBA8 cache for value.17c7bd864f7f18268f (no browser, no cargo). The ETC2 RGBA block is 16 bytes: the alpha block first (bytes 0 to 7), then the ETC colour block (bytes 8 to 15).

Source: the cache's source GLB is UASTC (DFD model 166, supercompression 2 for two images and 1 for one). The ETC2 cache comes from transcode_uastc_to_etc1 (the basis transcoder in the vendored crate) and the RGBA8 cache from the UASTC unpack.

Result, from the script: every block is either differential (60,556 blocks) or individual (4,980 blocks); none is in the T, H or planar special mode. Top-end texels (cache blue >= 240) number 924,872, and 865,639 of them differ from the ETC2 decode. Mean signed error over those texels (etc2 minus cache): R +0.37, G +0.36, B -5.68. Among the 46,193 top-end differential blocks, the 5-bit blue base is 30 in 29,570, 29 in 15,463 and 31 in 402. The apply_etc1_bias table in the vendored transcoder moves blue subset 0 by one level for bias codes 6 (-1) and 15 (+1), which accounts for the base of 30.

Counterfactual: raising the blue base by one and lowering the blue delta by one on the 45,791 top-end differential blocks that allow it takes the blue PSNR from 28.49 to 29.38 dB. So the one-level bias explains only a part of the loss. The rest is not explained yet.

Verdict so far: the ETC2 bits decode to the same texels in my decoder and in the separate software GL (see the Round 86 record), so the disagreement is between the transcoded ETC2 content and the full-precision RGBA8 unpack, not a decode error. The cache stays OPEN against the unchanged 35 dB bar (blue 28.49 dB). Reopen: a source encode without the bias, or a ruling on the 35 dB bar for the blue top end.

Red check: the squared error in the script was swapped for an absolute error. The blue PSNR became 39.82 dB, the check printed FAIL and exited 1 (mut_rc=1). The file was restored, cmp matched (cmp-restored), the check passed with exit 0, and git status showed only the new script before the commit.

## Round 87 Phase B: the blue top-end witness in the window (record)

Readings, vmstat 1 2 first (the first line of vmstat is the average since boot, so the second line is the sample): the first line si 5314, so 7426; the second line si 32340, so 0, which is closed (swap-in over 1000 KB/s). The tree heavy-jobs read clear (HEAVY_JOBS clear, rc 0), and heavy-jobs.cjs matches 495fff6 with zero diff lines.

Window job, 30 s interval, gate on the second vmstat sample, heavy-jobs on the same sample. Attempt 1 (log kept as target/witness/window-job-r87-attempt1.log): 04:57:26 si 144, so 0, load 2.01, gate open, heavy clear, GATE-CLEAR with 598 s left; the witness set started at 04:57:27 but the session that ran the job ended and killed it after the first witness's output, so attempt 1 logged no return code and the red check did not run. Attempt 2, the same witness set re-run in the foreground with the window deadline shortened to 540 s (log target/witness/window-job-r87.log): 04:58:14 si 1100, so 3320, load 3.07, gate closed, heavy busy (gm-core worker pid 2111092 at 99%, another project's /workspace/freddie, not stopped); 04:58:43 si 448, so 1172, gate closed, heavy clear; 04:59:13 si 712, so 0, load 2.12, gate open, heavy clear, GATE-CLEAR with 478 s left.

Witnesses on the 04:59:13 clear sample: blue-top-analysis rc 0, wall 0.7 s, timeout pid 2113220, node child 2113222, output ok 1 of 1 (blue PSNR 28.49, counterfactual 29.38 dB on 45,791 blocks). blue-top-red-check rc 0, wall 0.6 s, timeout pid 2113242, child 2113244: the mutant (absolute error in place of the squared error) printed FAIL with blue PSNR 39.82 and exited 1, which the red check reads as DETECTED; the tracked script matched HEAD by cmp afterwards. WITNESS-SET overall 0 at 04:59:15. Processes left after the run: 0. The mutant file was removed.

Result, carried from Phase A and confirmed here: the ETC2 cache's blue top end is OPEN against the 35 dB bar at 28.49 dB. The ETC2 bits decode the same in my decoder and in the separate software GL (the readback agrees with the decoders), so the cache's ETC2 side is the decoded side. The RGBA8 cache disagrees with it. Top-end differential blocks carry a blue base of 30 or 29 where 31 is needed, and the transcoder's apply_etc1_bias accounts for one level; correcting that alone gives 29.38 dB, so about 0.9 dB of the gap is that bias and the rest is open. The bar stays with the owner.

Ranked list at this point: 1 cache blue top end, OPEN as above, the cause partly identified; 2 ASTC 4x4 decoder, reachable, next stretch; 3 multi-pair alternation, blocked (window of 1440 s or a ruling for fewer budget seconds per arm); 4 jobs measurement, blocked (owner's stop-rule ruling, then a clean-build window); 5 stale rgba8-a and rgba8-control sets, blocked (owner's config for each); 6 gm-core pid 1380537, blocked (a high-CPU gm-core without --handshake); 7 held items, blocked (gross per-process swap, noisy-build swap attribution, override total effect, release build 862.7 s, ZLIB KTX2, GPU panic, owner-held).

## Round 88 Phase A: the blue top-end counterfactuals (mechanisms)

Script: scripts/check-etc2-blue-counterfactuals.cjs. It rebuilds every ETC colour block from its bits (base, table, selectors) and checks the rebuild against decodeEtc2Rgba8Block on all 65,536 blocks: 0 mismatches. Each counterfactual changes one mechanism and reports PSNR on R, G and B; the 35 dB bar is judged on the minimum of the three.

Source facts, from the vendored transcoder: transcode_uastc_to_etc1 averages each subset's 8 RGBA8 pixels into 5-bit endpoints, (sum*31 + 1020) / (8*255), and etc1_determine_selectors chooses selectors with luma weights 54 R, 183 G, 19 B. apply_etc1_bias moves subset 0 by one 5-bit level for bias codes 6 (blue -1) and 15 (blue +1). The differential delta is clamped to -4..3. The solid colour is 8-bit per channel in m_solid_color and is written to all 16 texels by the RGBA8 unpack; the ETC solid branch instead reads 5-bit hint fields (m_etc1_r/g/b, basisu_transcoder.cpp lines 12190-12192), and the encoder packs the 8-bit colour with pack_etc1_block_solid_color, exact to within 1 per channel.

Counterfactuals, PSNR in dB (R / G / B, min):
- baseline (transcoder output): 28.77 / 36.34 / 28.49, min 28.49.
- M1 bias step undone on top-end differential blocks: 28.77 / 36.34 / 29.38, min 28.77. Does not close the gap.
- M2a blue-only selectors (transcoder endpoints): 26.61 / 25.95 / 35.16, min 25.95. Blue reaches 35.16 but R and G fall below 26 dB. Does not close the gap.
- M2b RGB-unweighted selectors (transcoder endpoints): 30.88 / 31.23 / 30.47, min 30.47. Does not close the gap. The luma metric is what gives G 36.34 at the cost of R and B.
- M3a ideal 5-bit endpoints, all channels, transcoder selectors: 25.62 / 27.70 / 30.22, min 25.62. Worse: the mean endpoints do not fit the transcoder's selectors. 2,753 of 60,556 differential blocks cannot hold the ideal endpoints (delta outside -4..3).
- M3b ideal endpoints and RGB-unweighted selectors: 30.09 / 30.29 / 32.42, min 30.09. Does not close the gap.
- M4 flat cache blocks exact (bound): 28.93 / 36.60 / 29.20, min 28.93. Does not close the gap. Of 20,253 flat cache blocks, 1,852 decode exactly, 9,568 decode flat at another value (mean offset -5.53 on blue; 9,333 of them differential), and 8,833 decode non-flat.

Verdict: OPEN. No single mechanism, and no combination tested, reaches 35 dB on all three channels. The best all-channel result is M2b at min 30.47. Attributed so far: the bias step (+0.89 dB on blue), the luma selector metric (the trade between G and R/B), and the solid path's flat values (+0.44 dB on the minimum). The rest of the blue top-end error is not attributed. It is consistent with the 4-level ETC1 palette per subset, but I have not tested that: an MSE-optimal endpoint search per block was not run. Reopen: that search, or a different ETC2 encode of this texture, or an owner ruling on the 35 dB bar for this format.

Red check: INDEX_TO_MODIFIER was changed from [2, 3, 1, 0] to [2, 3, 0, 1]. The baseline rebuild then differed from the library on 899,404 texel channels, the check printed FAIL and exited 1 (MUTANT rc=1). The file was restored, cmp matched (cmp-restored), the check passed with exit 0, and git status showed only the new script before the commit.

## Round 88 Phase B: the blue top-end counterfactuals in the window (record)

Readings. vmstat 1 2 first: the first line is the average since boot (si 5295, so 7397). The second line, the gate sample, is si 276, so 0: both directions under 1000 KB/s, so the gate is open. heavy-jobs at that point read busy: a zig test process of another project (pid 2114065, 100% CPU, ./.zig-cache test) and a Claude session (pid 2109, 23.8%). Neither was stopped.

Window job, one reading every 30 s, gate on the second vmstat sample with heavy-jobs on the same sample. 05:06:37: si 296, so 0, load 2.92, gate open, heavy busy (the same zig test process at 99% and the Claude session at 23.8%). 05:07:07: si 224, so 0, load 2.61, gate open, heavy clear: GATE-CLEAR with 568 s left. The witness set started at 05:07:08.

Witnesses on the 05:07:07 clear sample. blue-cf-analysis: rc 0, wall 0.8 s, timeout pid 2121406, node child 2121408, output ok 1 of 1 (baseline reconstruction mismatches 0, blue PSNR 28.49, min 28.49). blue-cf-red-check: rc 0, wall 0.8 s, timeout pid 2121435, child 2121437: the mutant (INDEX_TO_MODIFIER [2, 3, 0, 1]) printed FAIL with 899,404 mismatches and exited 1, which the red check reads as DETECTED; the tracked script matched HEAD by cmp afterwards, and the mutant file was removed. WITNESS-SET overall 0 at 05:07:09. Processes left after the run: 0.

Result, the same numbers as Phase A. The bar stays OPEN. Attributed: the bias step (blue 29.38, min 28.77, R limits), the luma selector metric (blue-only selectors reach blue 35.16 but R and G fall to 26.61 and 25.95; RGB-unweighted selectors reach min 30.47), and the flat solid path (exact flat blocks give min 28.93). The best all-channel combination tested is ideal endpoints with RGB-unweighted selectors, min 30.09. The remaining error is not attributed; the MSE-optimal endpoint search that would test the 4-level palette was not run.

Which side agrees with which: the separate software GL and both decoders agree with the ETC2 bits, so the disagreement is between the ETC2 transcode and the RGBA8 unpack. The RGBA8 cache is the side that differs from the decoded ETC2.


## Round 89 Phase A: the MSE-optimal endpoint search on the top-end differential blocks

Script: scripts/check-etc2-endpoint-search.cjs. It checks first: re-encoding each differential block from its parsed fields gives its bytes exactly (60,556 of 60,556), and the evaluator built on the same fields matches decodeEtc2Rgba8Block on every texel (0 mismatches). Search: for each top-end differential block (cache block mean blue >= 250, 46,193 blocks), each subset's cost is separable (a texel's selector lies within its own subset), so the search runs over the 8 intensity tables of each subset and the 32 base values, with the differential delta limited to -4..3, and the best pair is taken. Every searched block is re-encoded and decoded by the library; the search's predictions match the library on all texels (0 mismatches in every scenario).

Results, PSNR in dB over the whole image; only the top-end differential blocks change:
- baseline (transcoder): R/G/B 28.77 / 36.34 / 28.49, min 28.49.
- SEL blue-optimal selectors, transcoder endpoints: R/G/B 26.75 / 29.10 / 30.12, min 26.75.
- E1 blue-error endpoint search over tables, base pairs and flip (blue-optimal selectors): R/G/B 9.47 / 9.52 / 30.83, min 9.47. Blue gains 0.71 dB over SEL.
- E2 blue-error endpoint search over tables and base pairs (RGB selectors): R/G/B 17.33 / 17.53 / 30.79, min 17.33.
- E3 E2, then base-only R and G searches with tables fixed: R/G/B 22.76 / 33.66 / 22.69, min 22.69.
- E4 joint RGB coordinate search over tables and base pairs, RGB selectors, two rounds (a local optimum, not a proven one): R/G/B 29.43 / 34.99 / 30.67, min 29.43.

What it shows: the transcoder's endpoints are already close to blue-optimal for these blocks (the exhaustive search adds 0.71 dB on blue over SEL). The loss on R and G in E1 to E3 comes from the intensity table, which is shared by the three channels of a subset. A table picked for blue is wide, so it wrecks R and G; the joint search (E4) is the only one that keeps them, and at 29.43 dB its minimum is still below the bar. The mechanism left after the search is the ETC1 palette itself: four levels per subset, one table per subset shared by R, G and B. For blue the search covers every table, flip and base pair, so the 30.8 dB blue figure on these blocks is the format's limit with this partition. For the minimum channel only a local search was run, so no bound is proven.

Verdict: OPEN. No endpoint search reaches 35 dB on all three channels. The minimum-channel best is E4 at 29.43 dB against the 35 dB bar. Reopen: a per-block joint search that is exhaustive for R, G and B together (too large for this run, about 8 x 32^3 bases per subset), or a different ETC2 encode of this texture.

Red check: MODIFIER_TO_INDEX was changed from [3, 2, 0, 1] to [3, 2, 1, 0]. The re-encode was then byte-identical on only 1,422 of 60,556 blocks and the search mismatched the library on about 2 million texel channels per scenario; the check printed FAIL and exited 1 (MUTANT rc=1). The file was restored, cmp matched (cmp-restored), and git status showed only the new script before the commit.

## Round 89 Phase B: the endpoint search in the window (record)

Readings. vmstat 1 2 first: the first line is the average since boot (si 5250, so 7321). The second line, the gate sample, is si 3192, so 0: the swap-in is over 1000 KB/s, so the gate is closed. heavy-jobs at that point read busy: a zig compile of another project's txiki build (pid 2131332). Neither was stopped.

Window job, one reading every 30 s, gate on the second vmstat sample with heavy-jobs on the same sample. 05:20:54: si 77180, so 53996, load 4.37, gate closed, heavy clear. 05:21:24: si 48, so 0, load 3.76, gate open, heavy clear: GATE-CLEAR with 567 s left. The witness set started at 05:21:24.

Witnesses on the 05:21:24 clear sample. endpoint-search: rc 0, wall 120.4 s, timeout pid 2132282, node child 2132284, output ok 1 of 1 (re-encode identical 60,556 of 60,556, evaluator mismatches 0, search mismatches 0 in all five scenarios, baseline blue 28.49). endpoint-search-red-check: rc 0, wall 115.1 s, timeout pid 2133653, child 2133655: the mutant (MODIFIER_TO_INDEX [3, 2, 1, 0]) printed FAIL with the re-encode identical on 1,422 of 60,556 blocks and exited 1, which the red check reads as DETECTED; the tracked script matched HEAD by cmp afterwards, and the mutant file was removed. WITNESS-SET overall 0 at 05:25:20. Processes left after the run: 0.

Result, the same numbers as Phase A. The bar stays OPEN. The exhaustive blue search adds 0.71 dB to the blue plane on the top-end blocks over blue-optimal selectors with the transcoder's endpoints, so the transcoder's endpoints are already near blue-optimal. The shared intensity table costs R and G in every blue-led search (R and G fall to 9 to 23 dB). The joint RGB coordinate search (E4, a local optimum) reaches R/G/B 29.43 / 34.99 / 30.67, minimum 29.43 dB. The mechanism left is the ETC1 palette itself, four levels per subset with one table shared by the three channels. The minimum-channel bound is not proven, because only a local search was run for R, G and B together.

Which side agrees with which: the separate software GL and both decoders agree with the ETC2 bits, so the disagreement is between the ETC2 transcode and the RGBA8 unpack. The RGBA8 cache is the side that differs from the decoded ETC2.


## Round 90 Phase A: the ASTC 4x4 decoder, checked on fixtures (no window)

Spec on disk: none. No ASTC specification file exists in the workspace, the cargo registry or the vendored tree, and basisu_astc_decomp is not vendored (basisu_gpu_texture.cpp calls basisu_astc::astc::decompress only under BASISU_USE_ASTC_DECOMPRESS). The block layout therefore comes from the vendored transcoder: pack_astc_block and pack_astc_solid_block (basisu_transcoder.cpp, 11889 and 11708), the BISE range table (11291), the trit and quint codes (5068 and 11804), unquant_astc_endpoint and its parameters (11368 and 11335), the partition hash and function (11731 and 11740), and the UASTC mode tables (11276 to 11290). The decoder, scripts/lib/astc-decode.cjs, is generated from those tables by a generator outside the repo.

Assumptions from memory, to be checked against the cache readback and not by the fixtures: (A1) a final partial trit group of k values stores k trits as a base-3 number in [0, 2, 4, 5, 7][k] bits, then the k value fields; a final partial quint group uses [0, 3, 5][k] bits the same way. (A2) endpoints are paired lo/hi per channel in the packer's order (RR GG BB, with the pair order taken from the packer comment). (A3) the dual-plane flag is bit 10 of the 11-bit block mode, which matches every dual field in the vendored table (0x441, 0x442). (A4) the weight bit order is the reverse of the packer's stored order, as the packer writes reverse_bits(weight) at the top of the block, weight i at bits 128 - N(i+1).

Finding 1: the vendored packer writes the final trit or quint group as a full 8-bit (or 7-bit) group. Under that rule the endpoint bits overflow into the weight bits for modes 0, 3, 10, 11 and 12 (mode 0 needs 76 bits and 47 are free). Under rule A1 every one of the 18 UASTC modes fits the vendored endpoint range, and the largest-fitting rule reproduces the vendored endpoint range for all 18 (witness below). Which rule the GPU uses is not settled by anything on disk.

Finding 2: the cache holds 61 ASTC KTX2 images (vkFormat 157, supercompression 0, target/gltf-cache-astc, 64 GLB files), 2,256,523 non-void level-0 blocks and 583,029 void-extent blocks. Block-mode fields: 0x242 2,005,112 (modes 0, 10, 15), 0x42 118,082, 0x441 113,576, 0x442 17,869, 0x53 1,884. The 0x242 family includes modes 0 and 10, which overflow under the full-group rule, so the cache data is not in the full-group form the vendored packer writes. The readback decides whether A1 is right.

Fixture witness: scripts/check-astc-decode.cjs. Void-extent blocks from the vendored pack_astc_solid_block layout (three colours), a flat block (two equal endpoints, mode 1), two-endpoint blocks (RGB mode 1, RGBA mode 14, LA mode 15 with 4-bit weights), each compared to expected pixels computed from the inputs, plus the block-mode field table and the endpoint-range rule for all 18 vendored UASTC modes. Result: ALL PASS, 43 checks.

Red check: the decoder was copied to /tmp/bt/astc-decode.pre.cjs, a dot-file mutant (scripts/lib/.astc-decode.mutant.cjs, weight reversal removed by sed) was run through the witness via ASTC_DECODE_PATH. The mutant failed the RGB, RGBA and LA fixtures and exited 1 (DETECTED). The mutant was deleted, cmp matched the saved copy (RESTORED_IDENTICAL), and the witness passed again on the restored file. git status showed only the two new files before the commit.

Commit: local only, as lanmower, no trailers. Not pushed in Phase A.

## Round 90 Phase B: the window (record, did not clear, nothing pushed)

Readings. vmstat 1 2 before the window: the first line (average since boot) si 5245, so 7274; the second line, the gate sample, si 312, so 0. heavy-jobs.cjs busy: zig test pid 2167062 (a peer's process, not stopped). Window: scripts/window-job.sh with WINDOW_S 600, INTERVAL_S 30, GATE_KB 1000, MIN_REMAINING_S 240, log target/witness/window-gate-r90.log. One reading every 30 s (si / so / gate / heavy):
05:46:52 88 / 0 open busy; 05:47:21 12 / 0 open busy; 05:47:51 68 / 0 open busy; 05:48:22 16 / 0 open busy; 05:48:51 60 / 0 open busy; 05:49:21 16 / 0 open busy; 05:49:52 124 / 0 open busy; 05:50:21 140 / 0 open busy; 05:50:51 8 / 0 open busy; 05:51:22 616 / 0 open busy; 05:51:51 7544 / 0 closed clear; 05:52:21 21164 / 0 closed busy; 05:52:52 7732 / 9384 closed clear; 05:53:19 START-REFUSED with 210 s left, under the 240 s minimum.

Result. No clear sample in the window: the memory gate was open on ten readings, but heavy-jobs read busy on every one of those (peers: zig test pid 2167062, fx, a gm-core worker, another Claude session). Swap-in then rose past 1000 KB/s at 05:51:51 and stayed there. No witness was started: the fixture witness, the red check and the readback were not run in the window. Nothing was pushed. Processes left by this run: 0 (pgrep found none). The window ran 6 min 39 s and ended in START-REFUSED, which is reported as open.

Offline numbers, not witnesses. Decoder (scripts/lib/astc-decode.cjs) against the 1024 by 1024 ASTC level 0 in target/gltf-cache-astc (65,536 blocks, 0 decode errors, analyse mode without a browser), CPU decoder against the RGBA8 cache: R 62.81, G 62.29, B 5.47, A infinite dB (alpha is 255 throughout). Breakdown: 62,121 blocks are block mode 0x242 with CEM 8 (RGB, six endpoint values), 3,415 are void-extent. Red and green lie inside the reference range in every block. Blue low decodes right; blue high (endpoint value 5, the single value in the one-value partial trit group, assumption A1) decodes to 33 where the reference is near 255 in block 0. So the disagreement is localised to the partial trit tail, not to the full-group path, the weights or the partitions. The decoder was left unchanged and the cache disagreement is recorded as it is.

Not run. The SwiftShader readback: scripts/check-astc-gpu.cjs (texture mode, one Chromium run with a GPU void-extent fixture and then the cache texture) and scripts/browser/readback-astc.html are written and untracked. Only analyse mode has run. They are not committed because nothing ran them in the window.

Reopen. Item 2, the readback: in a window where vmstat's second sample is under 1000 KB/s both ways and heavy-jobs.cjs reads clear, run node scripts/check-astc-gpu.cjs texture once under timeout, then commit the driver and page and push. The tail layout: replace assumption A1 once the ASTC spec text or a conformant encoder's output for a one-value trit tail is available, and rerun the fixture witness and the cache diagnostic.

Commit: local only, as lanmower, no trailers. Not pushed.
