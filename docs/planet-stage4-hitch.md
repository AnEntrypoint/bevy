# Planet stage 4 hitch free: design (Phase A groundwork)

Plan: /workspace/hyperagent/specs/bevy-planet-parity.md, Stage 4 and Architecture 7 ("Nothing is generated, uploaded or built on the main thread beyond a fixed per-frame budget. That rule is what makes it hitch free, and it is witnessed."). Table: docs/planet-parity.md, decision dec:budget (line 11) and the stage 4 rows grb:B01 to grb:B59 (lines 1022 onward). Reference: /workspace/spoint at 5f43c86c (read only, git status clean when read). Port: /workspace/bevy at d4c407c. Bevy 0.20.1 sources cited from /config/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/.

Everything below was read, not run: no cargo command, no build and no game run was made for this design. Every number is either read from a cited line or marked "computed from" the cited constants. Where a cost was not measured it says "unmeasured".

The budget (dec:budget): the reference states no frame budget. While streaming along the scripted paths, main-thread CPU time per frame stays within 1 ms of the same scene's idle p99, and no frame exceeds twice it. This design reads that as one per-frame ceiling:

    ceiling_ms = min(idle_p99_ms + 1.0, 2.0 * idle_p99_ms)

Every streaming frame must be at or under the ceiling. The +1 ms bound binds when idle p99 is 1 ms or more, and the 2x bound binds below that. Open question Q1 asks the owner to confirm this reading.

## 1. Inventory of main-thread work in the planet systems

Plugin wiring in the game: globe::register (src/globe.rs:405-430) adds PlanetLodPlugin, TerrainMaterialPlugin, WaterPlugin, AtmosphereLutPlugin, the sky, light and tone map plugins, then add_vegetation (src/globe.rs:394-397) adds VegGpuPlugin, TreesRenderPlugin, RockRenderPlugin, GrassRenderPlugin and VegStreamPlugin. ColliderStreamPlugin comes in through PlanetLodPlugin (src/planet_lod/stream.rs:222), or on its own when there is no mesh asset store (src/globe.rs:425).

Status key: bounded means a constant in the code limits the work per frame. Unbounded means nothing limits it per frame: it grows with the scene, the number of finished tasks or the number of bodies. Unmeasured means nobody has timed it.

### 1.1 Patch streamer (src/planet_lod/stream.rs)

Systems: one chain in Update: rebase_system, plan_leaves, poll_climate, start_tasks, finish_tasks, retire_patches, publish_stats (stream.rs:223).

| work | where | per-frame bound | status |
|---|---|---|---|
| Leaf selection: select_leaves plus one Planet::height for the ground under the camera | plan_leaves stream.rs:101-116, leaves_at stream.rs:95-99 | Runs on every frame the camera moved (stream.rs:109 skips only an identical position), so it runs on every frame of a flight. Rebuilds the wanted HashSet and filters pending (stream.rs:114-115). | unbounded in time, unmeasured. This is the main per-frame CPU cost of a moving camera. |
| Task start: collect every missing wanted key, compute a distance for each, sort them | start_tasks stream.rs:126-146 (sort stream.rs:135) | The spawn count is bounded by MAX_TASKS_IN_FLIGHT = 64 (stream.rs:18, :127, :141). The collect and sort run over all missing leaves on every frame. | spawns bounded; the scan and sort cost is unmeasured |
| Patch build (build_patch) | spawned on AsyncComputeTaskPool stream.rs:143; build_patch calls note_build (src/planet_lod/patch.rs:97) | off the main thread | bounded (zero main-thread builds; witnessed by stream.rs:371 and :387) |
| Patch finish: Mesh assembly and Assets<Mesh> insert | finish_tasks stream.rs:148-177; PATCH_SPAWNS_PER_FRAME = 2 (was 8; stream.rs:17, :159); patch_mesh stream.rs:85-93 builds a new heights Vec (stream.rs:86); meshes.add stream.rs:173 | at most 8 meshes of 144 vertices and 242 triangles (stream.rs:239-240) per frame | bounded, unmeasured |
| Patch mesh extract | patch_mesh uses RenderAssetUsages::default() (stream.rs:87), which contains MAIN_WORLD, so Bevy clones the whole mesh during extract (bevy_render-0.20.1/src/render_asset.rs:363-364) and keeps the main-world copy | at most 8 new meshes per frame (follows the finish budget) | bounded, unmeasured |
| Retire: for each live key, scan all wanted leaves for overlap | retire_patches stream.rs:179-193 (scan stream.rs:185) | live x wanted on every frame, even when nothing changed | unbounded (quadratic in leaf count), unmeasured |
| Stats: sums over all live patches | publish_stats stream.rs:195-203 | linear in live patches, every frame | unbounded (linear), unmeasured |

### 1.2 Climate bake (src/planet_lod/stream.rs)

| work | where | bound | status |
|---|---|---|---|
| HpfFaces::bake at res 128, max level 7 | started in PlanetLodPlugin::build on AsyncComputeTaskPool (stream.rs:216); bake_climate calls note_build (stream.rs:58-61); constants stream.rs:19-20 | off the main thread, once | bounded |
| Poll | poll_climate stream.rs:118-124, using check_ready (it does not block) | one poll per frame until ready | bounded |
| Gate | start_tasks returns early and counts waiting_on_climate until the bake is ready (stream.rs:136-139) | no patch starts before the bake | bounded (witnessed by stream.rs:413-438) |

### 1.3 Collider stream (src/planet_lod/collider.rs)

Systems: chain in Update: finish_builds, start_builds (needs ColliderGround), drop_orphans (collider.rs:187). ColliderGround is inserted at startup by globe::insert_collider_ground (src/globe.rs:339-342).

| work | where | per-frame bound | status |
|---|---|---|---|
| Start: for each NearBody outside its keep radius, spawn a build task | start_builds collider.rs:136-154 (spawn collider.rs:150); keep radius = 256 x 0.4 = 102.4 m (collider.rs:8, :10, :36-38) | one task per qualifying body per frame; no budget constant; bounded only by the number of NearBody entities | unbounded per frame (scales with bodies), unmeasured |
| Build: sample a 128 x 128 field (Lattice::of(256, 2): round(256/2) = 128, even, so n = 128; collider.rs:30-34; computed) and build Collider::heightfield | build_collider collider.rs:90-98, on the task pool | off the main thread | bounded; counted by its own counter, not by guard.rs (see 1.9) |
| Finish: insert or spawn the heightfield collider for every ready task | finish_builds collider.rs:156-171 (insert collider.rs:164, spawn collider.rs:167) | every finished build in the same frame; no per-frame cap | unbounded per frame (scales with bodies) |
| Avian's work on a new or replaced heightfield collider (AABB, mass properties, broad phase insert) | runs after finish_builds in the physics schedule | none set by the port | unmeasured |
| Orphan sweep | drop_orphans collider.rs:173-179 | linear in fields per frame | bounded by bodies, unmeasured |

### 1.4 Terrain texture layers (src/planet_material/)

| work | where | bound | status |
|---|---|---|---|
| Placeholder images (1 x 1 texels) and material insert | insert_material terrain.rs:252-254, terrain_material terrain.rs:159-177, placeholders terrain.rs:179-189 | Startup, tiny | bounded |
| Decode of 4 layers x 3 JPEGs and the mip build of both 1024 x 1024 x 4 arrays | start_decode spawns decode_all on the task pool (terrain.rs:288-290); decode_all terrain.rs:282-286; decode_layers calls note_build (layers.rs:152-153); array_image builds 11 mips (layers.rs:209-227) | off the main thread | bounded. array_image has no note_build, so a main-thread call of it alone would go uncounted |
| Image insert and material swap | finish_decode terrain.rs:292-313 (images.add terrain.rs:305, material get_mut terrain.rs:306-312) | once | bounded on the main thread (a move, not a copy) |
| GPU upload of the two arrays | Images use RenderAssetUsages::RENDER_WORLD (layers.rs:224), so extract moves the data (render_asset.rs:350-356) and prepare_assets uploads it in one frame. Each array is 4 layers x 4 x (4^11 - 1)/3 bytes = 22,369,616 bytes (computed from SIZE 1024, LAYER_COUNT 4, MIP_LEVELS 11 at layers.rs:15-17 and RGBA8), so 44,739,232 bytes in total | RenderAssetBytesPerFrame is not set anywhere in /workspace/bevy/src (grep found none), and its default max_bytes is None, meaning no limit (render_asset.rs:505-511) | one-frame spike, unmeasured. On the pipelined game it runs on the render thread. In the headless harness it runs on the same thread as the main schedule. |
| Material params every moving frame | update_frame terrain.rs:223-238: tex_cam_frac follows the camera (terrain.rs:218), so get_mut marks the material modified on every moving frame, and the material is extracted and prepared again | one material per frame | bounded, unmeasured |

### 1.5 Atmosphere LUTs (src/planet_sky/lut.rs)

| work | where | bound | status |
|---|---|---|---|
| Transmittance (256 x 128, 500 steps) and scattering (64 x 32 x 24, 64 steps) bake, plus the half-float conversion | start_bake spawns bake().images() on the task pool (lut.rs:99-101); bake calls note_build (lut.rs:33-38); images lut.rs:86-94; constants src/planet_sky/atmosphere.rs:20-26 | off the main thread, once | bounded (witnessed by lut/tests.rs:69-87) |
| Insert and upload | finish_bake lut.rs:103-107 (two images.add calls); RENDER_WORLD images (lut.rs:81). Sizes 256 x 128 x 8 = 262,144 bytes and 64 x 32 x 24 x 8 = 393,216 bytes (computed, Rgba16Float) | once | bounded one-off upload, unmeasured |
| Material bind | bind_atmosphere terrain.rs:240-250, once after the LUTs are ready | once | bounded |

### 1.6 Water (src/planet_water/surface.rs)

Systems: chain in Update after rebase_system: tick_clock, poll_sea_mean, plan_water, start_water, finish_water, update_params (surface.rs:359).

| work | where | bound | status |
|---|---|---|---|
| Sea height mean | task spawned in WaterPlugin::build, calls note_build (surface.rs:346-349); polled surface.rs:238-244 | off the main thread, once | bounded |
| Leaf selection, a second time per moving frame | plan_water surface.rs:246-254 calls its own leaves_at, which is select_leaves plus one height (surface.rs:228-232), on every frame the camera moved | none | unbounded in time, unmeasured. It repeats the patch streamer's selection on the same frame. |
| Whole-ocean mesh build | start_water surface.rs:256-267 spawns build_water (note_build surface.rs:198) when the wanted set changed and nothing is pending | one task at a time, off the main thread | bounded in count |
| Whole-ocean mesh insert | finish_water surface.rs:269-284: meshes.add of the whole mesh (surface.rs:280) and despawn of the old one (surface.rs:282). The mesh uses RenderAssetUsages::default() (surface.rs:190), so extract clones it (render_asset.rs:363-364) | one mesh per rebuild; its size grows with the leaf count | unbounded in size, a one-frame spike on each rebuild, unmeasured |
| Material params every frame | update_params surface.rs:286-313: params.time changes every frame (surface.rs:303), so the material is modified, extracted and prepared on every frame | one material per frame | bounded, unmeasured |

### 1.7 Vegetation stream and GPU upload (src/planet_veg/stream.rs, src/planet_veg/gpu/)

Systems: stream_vegetation in Update (stream.rs:279), not ordered against rebase_system; publish_frame in PostUpdate (gpu/mod.rs:45). Render world: apply_frame, apply_bounds and poll_stats in Prepare; prepare_cull; prepare_draws; run_cull (gpu/mod.rs:52-57).

| work | where | per-frame bound | status |
|---|---|---|---|
| Refocus when the camera crosses into another 32 m chunk: drop beyond the drop ring, then rebuild the three rings with ring_around_dir | refocus stream.rs:200-212 (ring_around_dir stream.rs:209, src/height/placement.rs:149); drop_beyond stream.rs:127-136; ring sizes at the census anchor: 1863 tree, 416 rock and 32 grass chunks (src/planet_veg/stream/tests.rs:25, :28, :30) | once per chunk crossing; the cost scales with the ring sizes | unbounded in time per crossing, unmeasured. A one-frame spike every 32 m of travel. |
| Task spawn | spawn stream.rs:214-230; room = AsyncComputeTaskPool threads x TASKS_PER_THREAD 2 (stream.rs:25, :216); place_chunk on the pool (stream.rs:226) | spawns bounded by tasks in flight | bounded |
| Placement and record encoding (place_tree_chunk, place_rock_chunk, place_grass_chunk, EncodedRecords) | place_chunk stream.rs:96-107; note_build in tree.rs:316, rock.rs:205 and grass.rs:230 | off the main thread | bounded (witnessed by stream/tests.rs:310) |
| Finish: for every ready task, add the chunk to the table and queue its records | KindStream::finish stream.rs:138-157 (store.upload stream.rs:145); VegStore::upload store.rs:251-264 (RangeAllocator::alloc does a linear scan of free ranges, store.rs:104-111) | every ready task in the same frame, so at most tasks in flight; no separate cap | bounded by tasks in flight, unmeasured |
| Upload drain: copy queued bytes up to the kind's budget | UploadQueue::drain store.rs:182-202 (to_vec copy store.rs:191); budgets 64 KiB tree, 64 KiB rock and 512 KiB grass (gpu/buffers.rs:43-48) | 640 KiB per frame across the three kinds | bounded (witnessed by gpu/tests.rs:120 and :307-310) |
| Chunk table encode, every frame | ChunkTable::encode store.rs:86-90: 2048 slots x 32 bytes = 65,536 bytes (store.rs:9-11; computed), called by VegStore::frame store.rs:299-305 from publish_frame store.rs:308-310 | 64 KiB per frame, even when nothing changed | bounded, unmeasured |
| VegFrame extract | publish_frame assigns through ResMut on every frame, so VegFrame is always changed, and ExtractResourcePlugin clones it on every frame (bevy_extract-0.20.1/src/extract_resource.rs:69-70; plugin registered at gpu/mod.rs:38). The clone holds the 64 KiB table plus the drained writes (up to 640 KiB) | at most 704 KiB of copying per frame (computed) | bounded, unmeasured |
| GPU writes | apply_frame gpu/buffers.rs:378-392 writes the table and the drained ranges (render world) | follows the drain budget | bounded |
| Chunk unload | ChunkTable::unload store.rs:63-72 (free.insert(0, ...) at store.rs:70 shifts up to 2048 entries) | per dropped chunk | bounded, unmeasured |
| Instance capacity | instance_capacity [32768, 12288, 540000] (gpu/buffers.rs:74) | an upload that does not fit is refused and counted (stream.rs:145-153) | bounded |

### 1.8 Species load, impostor bake, rock and grass assets (one-offs)

| work | where | bound | status |
|---|---|---|---|
| Species decode, tier merge and texture layer stacks | the TreesRenderPlugin::build task (trees_render.rs:137-138) runs build_tree_assets (trees_render/assets.rs:142-153); load_species calls note_build (species.rs:242-245) | off the main thread, once | bounded. merge_tier and layer_stack are not guarded on their own. |
| Tree tier meshes | finish_load trees_render.rs:117-130 polls without blocking (future::block_on(future::poll_once(..)) at trees_render.rs:119); then spawn_tree_draws trees_render.rs:105-115 clones each tier's geometry (tier_mesh trees_render.rs:97-99) and calls meshes.add for 15 species x 3 tiers (trees_render.rs:106) | once, all in one frame | one-frame spike on the main thread, unmeasured. The geometry is copied, not moved. |
| Bark and leaf array textures, impostor atlas and capture geometry | create_tree_gpu trees_render/gpu.rs:216-295 runs once (gpu.rs:225) in PrepareResources: array textures uploaded (gpu.rs:227-228), capture geometry built and uploaded (gpu.rs:264-293); atlas bake render node (trees_render.rs:148) | once | one-frame spike on the render side, unmeasured |
| Rock hulls | build_rock_meshes on the task pool (rock_render.rs:42-56, :65-67, note_build :43); finish_build calls meshes.add for each rock type (rock_render.rs:69-78) | once | bounded one-off, unmeasured |
| Grass clump texture and the two clump meshes | build_assets on the task pool (grass_render.rs:231-234, :242-244, note_build :169 and :232); finish_bake inserts one image and two meshes (grass_render.rs:258-264) | once | bounded one-off, unmeasured |

### 1.9 Main-thread build guard (src/planet_lod/guard.rs)

- A thread-local flag and counter (guard.rs:3-6). mark_main_thread (guard.rs:8-10) is called in PlanetLodPlugin::build (stream.rs:214), so the thread that builds the app is the marked thread. note_build (guard.rs:12-16) counts only on that thread. builds_on_main_thread is #[cfg(test)] (guard.rs:18-21), so the release binary cannot read the counter.
- note_build is called in: build_patch (patch.rs:97), bake_climate (stream.rs:59), decode_layers (layers.rs:153), the LUT bake (lut.rs:34), build_water (surface.rs:198), the sea mean task (surface.rs:347), place_tree_chunk (tree.rs:316), place_rock_chunk (rock.rs:205), place_grass_chunk (grass.rs:230), load_species (species.rs:243), build_rock_meshes (rock_render.rs:43), bake_clump_mips (grass_render.rs:169) and grass build_assets (grass_render.rs:232).
- note_build is not called in: build_collider, which has its own static counter keyed on the thread name "main" (collider.rs:13, :15, :17-20, :91-93), so it counts nothing on a libtest thread unless the test runs the app on a thread named "main", as collider.rs:300 does; array_image (layers.rs:209); BakedLuts::images (lut.rs:86-94); patch_mesh (stream.rs:85-93), which runs on the main thread by design; water_mesh_asset (surface.rs:190); merge_tier and layer_stack in build_tree_assets; tier_mesh (trees_render.rs:97); capture_geometry (trees_render/gpu.rs:265); select_leaves; ring_around_dir; ChunkTable::encode.
- Tests that read the guard: stream.rs:371, :387, :438; lut/tests.rs:87; water render_tests.rs:275-283; species.rs:393-395; layers.rs:356-360; grass_render/tests.rs:94-97; stream/tests.rs:180 and :310; and, for the collider counter, collider.rs:309 and :330.

### 1.10 Ordering and stalls

- No planet system blocks on a task. Every poll uses check_ready or poll_once: stream.rs:120, :162; collider.rs:158; terrain.rs:299; lut.rs:104; surface.rs:240, :277; veg stream.rs:141; trees_render.rs:119; rock_render.rs:70; grass_render.rs:259.
- The only blocking wait is in the headless harness: pump waits on the device after each update (src/offscreen.rs:93-97). That wait must be kept out of the main-thread measurement.
- The patch chain (stream.rs:223) and the water chain (surface.rs:359, after rebase_system) each run select_leaves on the same moving frame, and the two are not shared.
- stream_vegetation (veg stream.rs:279) is not ordered against rebase_system. That is a correctness point, not a hitch.

### 1.11 One-frame spikes found by reading

1. Terrain layer arrays: 44,739,232 bytes uploaded in one prepare (computed; terrain.rs:305, layers.rs:209-227) with no RenderAssetBytesPerFrame limit (render_asset.rs:505-511, never set in the port).
2. Tree tier meshes: 45 meshes.add calls with cloned geometry in one frame (trees_render.rs:105-115), then array textures and capture buffers created once in the render world (trees_render/gpu.rs:227-228, :264-293).
3. Whole-ocean mesh: meshes.add plus a full clone in extract on every water rebuild (surface.rs:280, :190; render_asset.rs:363-364).
4. Vegetation refocus every 32 m: ring_around_dir over rings of up to 1863 chunks, plus drop_beyond, on one frame (veg stream.rs:200-212).
5. Collider finish: every ready heightfield inserted in the same frame, plus Avian's processing of it (collider.rs:156-171).
6. Patch extract clones: up to 8 patch meshes cloned in extract per frame, because of RenderAssetUsages::default() (stream.rs:87). This is bounded, but it is a copy that a RENDER_WORLD-only mesh would avoid.
7. Leaf selection twice per moving frame (stream.rs:114 and surface.rs:253): runs on every frame, not a single spike, but unmeasured and unbudgeted.

### 1.12 Unbounded or unmeasured main-thread work

Unbounded per frame:
- select_leaves on every moving frame, in both plan_leaves (stream.rs:101-116) and plan_water (surface.rs:246-254)
- retire_patches, live x wanted (stream.rs:185)
- the start_tasks scan and sort over all missing leaves (stream.rs:129-135)
- publish_stats, linear (stream.rs:195-203)
- collider start and finish, all bodies and all ready builds in one frame (collider.rs:143-153, :157-170)
- the whole-ocean mesh insert and extract clone (surface.rs:280)
- vegetation refocus and ring_around_dir (veg stream.rs:200-212)

Unmeasured but bounded:
- the patch finish (8 per frame) and its extract clone
- the chunk table encode and the VegFrame clone (up to 704 KiB per frame)
- the upload drain copy
- the terrain and water material re-prepare on every frame
- Avian's heightfield processing
- every one-off asset insert in 1.11

## 2. Measurement design

### 2.1 What is measured

- The physics benchmarks live in src/bench.rs and are run as subcommands from src/main.rs:146-148 (bench, bench-phases, bench-schedules). bench::simulate times each app.update() with Instant (bench.rs:99-104) after 30 warmup ticks (bench.rs:89, :95-97). The time step is fixed with TimeUpdateStrategy::ManualDuration(TICK) (bench.rs:64-65). run sorts the samples and picks percentiles by rounded nearest rank, samples[round((n - 1) x q)] (bench.rs:114-116), then prints one JSON line (bench.rs:118-126). scripts/bench-vs-jayzee.sh records /proc/loadavg before and after each run (lines 15 and 18). scripts/check-bench-load-gate.cjs checks that loaded runs are refused (LOAD_LIMIT default 4, line 40).
- src/render_stats.rs already splits a frame. sim_begin in First (render_stats.rs:55-59, :162) and sim_end in Last (render_stats.rs:61-66, :163) give sim_ms, the main schedule (render_stats.rs:81). render_begin and render_end bracket RenderSystems::Render (render_stats.rs:85-98, :173-174) and give draw_ms. cpu_ms runs from frame_begin to render_end (render_stats.rs:97). The viewer adds RenderStatsPlugin (src/viewer.rs:175), but no planet test does.
- In a pipelined game the main thread runs the main schedule and then the extract. The Render schedule runs on the render thread. So stage 4 defines main-thread time per frame as:

      main_ms = sim_ms + extract_ms

  sim_ms is First to Last (already measured). extract_ms is the time from the end of Last to the first system of the render schedule. It is measured by one new timestamp in a system in RenderSystems::ExtractCommands, the first set of the render chain (bevy_render-0.20.1/src/lib.rs:188, :351), and one in RenderSystems::PostCleanup (lib.rs:401) to close the render span. render_ms (ExtractCommands to PostCleanup) is recorded as a diagnostic of render-thread time, outside the budget. The timer is wall time inside those spans (Instant, as in bench.rs), because a hitch is wall time and includes lock waits. Thread CPU time is an optional second column (Q4).
- The device wait in offscreen::pump (offscreen.rs:96) is outside update(), so it is not in main_ms.

### 2.2 Idle baseline

- Same app, same process, same pose: the end pose of each path, held still after everything has settled. Settled means: Streamer.pending empty and live == expected (the stream.rs:303-307 test); VegStreamStats.tasks_in_flight == 0, queued_chunks == [0; 3] and pending upload bytes 0 (the Census::streamed test, stream/tests.rs:213-217); water pending None and live keys == wanted; LUTs ready (lut.rs:109-111); layers ready (surf_ready, terrain.rs:311); tree assets loaded and baked (stream/tests.rs:206-211).
- Hold 600 frames at that pose and take p50, p99 and max with the bench.rs:116 pick. Also record the same at the start pose. The budget uses the end-pose idle p99, the pose with the most resident content. Q2 asks whether to use the larger of the two.
- Startup one-offs (1.11 items 1 and 2) happen before the idle window. They are reported as a separate "startup" line with their frame times, and they are outside the path budget (Q3).

### 2.3 Running headless on this host

- offscreen::headless_app (offscreen.rs:72-91) forces lavapipe (VK_ICD_FILENAMES set to /usr/share/vulkan/icd.d/lvp_icd.json, offscreen.rs:17, :64-70), disables WinitPlugin and PipelinedRenderingPlugin (offscreen.rs:85-86), and compiles pipelines synchronously (offscreen.rs:77). It renders to a 320 x 180 target (offscreen.rs:22-23, add_target :112-116, spawn_view :118-133). Because pipelining is off, extract and Render run on the calling thread inside update(). That is why main_ms is taken from the First, Last and ExtractCommands timestamps and not from the update() wall time.
- Time: TimeUpdateStrategy::ManualDuration(1/60 s), as the globe census does (src/globe.rs:486-497). The path is indexed by frame number, never by real time.
- The run goes through the ticket gate (scripts/window-job.sh, via a new scripts/window-planet-stage4.sh shaped like scripts/window-planet-stage3.sh), records loadavg before and after, and refuses the run above the load limit (check-bench-load-gate.cjs). This host had 12 cores and load 9.29 7.70 6.67 when this design was read, so the gate matters.
- What it can tell: main-thread CPU time per frame, the guard counters, patches spawned per frame, upload bytes per frame, collider builds, resident counts, and the render-thread CPU time of the Render schedule.
- What it cannot tell: GPU time. Lavapipe rasterises on the CPU with its own threads, so its "GPU" cost is CPU time that competes with the async task pool. Upload cost on a real driver (PCIe transfer, residency) does not show. render_stats reports which timestamp-query features the device lacks (render_stats.rs:137-152), but GPU frame time on real hardware is the owner's check (spec, Who owns what). Task completion is real time while the camera moves in virtual time. On a slow lavapipe frame the tasks finish "earlier" in frames than they would on a GPU, so the frame on which a patch or chunk lands is not deterministic. The witness asserts on distributions and counters, never on per-frame equality.

## 3. The two scripted paths

Both paths are deterministic: the camera position and orientation are pure functions of the frame index, with a fixed 1/60 s step, on the preset planet (radius 63600, seed 1337; dec:radius, src/planet_veg/context.rs:320-325).

Reference facts:
- bookmarks.js:5: OUT_KM = [5, 20, 100]. bookmarks.js:41: out-<km>km = liveChartSpec([km * 1000, 0, 0], fromBase, false), which is { x, z } with no height (bookmarks.js:28-31).
- The forest bookmark is a search, not a point (bookmarks.js:6, :42).
- relocation.js:105-107 takes x and z from the spec. The default clearance is 2 m (relocation.js:3, :115), with snap 'terrain' (relocation.js:117).
- So in the reference, out-100km, out-20km and out-5km are ground points 100, 20 and 5 km along the anchor chart's x axis, not altitudes. Q5 covers this.

### 3.1 Flight, orbit to ground

- Waypoints:
  - W1: out-100km chart point lifted to 100 km altitude
  - W2: out-20km lifted to 20 km
  - W3: out-5km lifted to 5 km
  - W4: the stage 3 census anchor at the census eye
- The chart-to-direction mapping comes from the reference's own frame: a generator scripts/gen-planet-hitch-path.mjs runs the reference PlanetFrame localToDir read-only for the three chart points on the preset anchor and writes testdata/parity/spoint-planet-hitch-path.json, recording the spoint rev. It does not re-implement the mapping.
- The lift (altitude = bookmark distance) is a port choice and is marked as such in the table.
- Legs: 600 frames per leg for W1 to W2, W2 to W3 and W3 to W4, then a 300-frame hold, 2100 frames in total.
- Within a leg, the direction is slerped and the altitude is interpolated in log space, so the level of detail changes at an even rate.
- The camera looks at the planet centre down to W3. On the last leg it pitches to the census frame.

### 3.2 Ground run through forest

- Start: the census anchor. playerWorld = [-40815.595807093756, 16288.03339696503, 46081.50808984984] (testdata/parity/spoint-planet-veg-placement.json, thinning.playerWorld, fixture rev 5f43c86c; read by stream/tests.rs:68-70; written by scripts/gen-planet-veg-placement.mjs:53). |playerWorld| = 63676.67 m (computed). Its direction (-0.64098, 0.25579, 0.72368) (computed) is the preset anchor (-0.641, 0.2558, 0.7237) to four places (src/globe.rs:706).
- Census frame: Frame::above(player.normalize(), player.length(), 0.0, 6 degrees pitch down) (stream/tests.rs:36, :188).
- Motion: along_north (stream/tests.rs:90-96) at 10 m/s for 4800 frames, 800 m in all. This passes the reference tree drop radius of 744 m (stream/tests.rs:35), so tree chunks drop as well as load. Every 32 m chunk crossing triggers a refocus (veg stream.rs:258-262).
- Eye: the census frame's altitude above the path's ground, clamped to at least the 2 m teleport clearance (relocation.js:3).
- A NearBody rides with the camera, so the collider stream runs (collider.rs:141). ColliderGround is inserted as in globe.rs:339-342.

## 4. Witness and red check

The witness is a test, planet_hitch::the_scripted_paths_stay_inside_the_relative_budget. It runs on a thread named "main" (the collider.rs:300 pattern) with guard::mark_main_thread, and for each path it:

1. runs the startup until settled and reports the startup frames;
2. holds the idle window at the start pose, then runs the path, then holds the idle window at the end pose;
3. prints one line per path: idle p50, p99 and max; path p50, p99 and max; ceiling; over-ceiling frame count; and the load before and after;
4. asserts that no path frame has main_ms above the ceiling;
5. asserts that guard::builds_on_main_thread() did not change (guard.rs:18-21) and that collider::main_thread_collider_builds() did not change (collider.rs:17-20);
6. asserts the run was not vacuous:
   - PlanetLodStats.max_spawned_in_frame == PATCH_SPAWNS_PER_FRAME at least once on the flight (the stream.rs:364-366 pattern);
   - the grass upload drain reached 512 KiB in at least one frame;
   - VegStreamStats changed resident chunk counts during the ground run;
   - ColliderStats.builds_started >= 2 on the ground run;
   - refused_chunks == 0.

Red check, the primary mutant: one synchronous instance-set build put back on the main thread in the vegetation stream (src/planet_veg/stream.rs:226). The guard part catches it through note_build in place_*_chunk, and the timing part should catch it, because a grass chunk holds 182552 / 32 = 5704.75 blades on average at the anchor (computed from stream/tests.rs:30-31). Mutant row, tab separated (name, file, old text, new text, test command):

    veg-place-on-main	src/planet_veg/stream.rs	stream.building.insert(key, pool.spawn(async move { place_chunk(&source, kind, key) }));	let placed = place_chunk(&source, kind, key); stream.building.insert(key, pool.spawn(async move { placed }));	cargo test --bin bevy-jayzee planet_hitch::

Supplementary rows, one per build kind, each with old text that occurs once in its file:

    patch-build-on-main	src/planet_lod/stream.rs	let task = AsyncComputeTaskPool::get().spawn(async move { build_patch(&planet, &baked, &config, key) });	let built = build_patch(&planet, &baked, &config, key); let task = AsyncComputeTaskPool::get().spawn(async move { built });	cargo test --bin bevy-jayzee planet_hitch::
    collider-build-on-main	src/planet_lod/collider.rs	let task = AsyncComputeTaskPool::get().spawn(async move { build_collider(&lattice, (ground.0)(body)) });	let built = build_collider(&lattice, (ground.0)(body)); let task = AsyncComputeTaskPool::get().spawn(async move { built });	cargo test --bin bevy-jayzee planet_hitch::

A 144-vertex patch build may not break the timing ceiling, so the guard counter is what catches patch-build-on-main. collider-build-on-main is caught only because the witness runs on a thread named "main" (collider.rs:13, :91). Each row goes in scripts/mutants/planet-hitch.tsv and is applied, run, confirmed failing and restored with cmp in the stage 4 window.

## 5. Planet render benchmark

- File: src/bench_planet.rs, next to src/bench.rs. Subcommand: bevy-jayzee bench-planet <path> <frames>, dispatched next to src/main.rs:146-148. path is flight or ground; frames defaults to the path length in 3.
- It measures the same thing as the witness, main_ms per frame (2.1), on the same two paths. It prints one JSON line in the bench.rs:118-126 shape: kind "bench-planet", path, frames, threads, idleP50Ms, idleP99Ms, idleMaxMs, p50Ms, p95Ms, p99Ms, maxMs, ceilingMs, overFrames, renderP99Ms (render-thread diagnostic), mainThreadBuilds, colliderMainBuilds, maxPatchesPerFrame, maxVegUploadBytes, loadBefore and loadAfter.
- scripts/bench-planet.sh, shaped like scripts/bench-vs-jayzee.sh, runs it 5 times per path and writes target/witness/bench-planet.json through scripts/bench-median.cjs.
- The binary build needs two stage 4 changes, because both are test-only today: the headless app (mod offscreen is #[cfg(test)] at src/main.rs:3-4) and builds_on_main_thread (guard.rs:18). Either the bench gets its own windowless app with the same lavapipe settings, or both are un-gated (Q6).

## 6. Rows this stage adds to docs/planet-parity.md

Not edited here. These rows would go in the "Grass, rocks and frame budget" section, after grb:B59.

| id | rule or number | reference value | source (path:line in /workspace/spoint) | port today | stage | status |
|---|---|---|---|---|---|---|
| hit:ceiling | per-frame main-thread ceiling while streaming = min(idle p99 + 1 ms, 2 x idle p99) | none stated (dec:budget) | not in the reference; see dec:budget | not measured | 4 | open |
| hit:main-ms | main-thread time per frame = First..Last (sim) + extract | none stated | not in the reference | sim_ms only (src/render_stats.rs:55-66, :81); extract not timed | 4 | open |
| hit:idle.flight | idle p50, p99 and max at the flight end pose, 600 frames | none stated | not in the reference | not measured | 4 | open |
| hit:idle.ground | idle p50, p99 and max at the ground run end pose, 600 frames | none stated | not in the reference | not measured | 4 | open |
| hit:path.flight | flight W1..W4 from out-100km, out-20km and out-5km to the census anchor, 3 x 600 + 300 frames at 1/60 s; p50, p99, max and over-ceiling count | bookmarks are chart points [km*1000, 0, 0] with no height | src/shared/bookmarks.js:5, :28-31, :41; src/shared/relocation.js:3, :105-117 | no path; altitude lift is a port choice | 4 | open |
| hit:path.ground | ground run 800 m north from the census anchor at 10 m/s, 4800 frames; p50, p99, max and over-ceiling count | forest is a search bookmark | src/shared/bookmarks.js:6, :42 | anchor in testdata/parity/spoint-planet-veg-placement.json thinning.playerWorld | 4 | open |
| hit:guard | no patch, instance set, water or asset build on the main thread across both paths | none stated | not in the reference | guard counter, test-only (src/planet_lod/guard.rs:18-21) | 4 | open |
| hit:guard.collider | no collider build on the main thread across both paths | none stated | not in the reference | separate counter keyed on the thread name "main" (src/planet_lod/collider.rs:13-20, :91-93) | 4 | open |
| hit:patch.spawn | patch meshes inserted per frame; patch tasks in flight | streaming scheduler max starts per drain 32, drain budget 2 ms (grb:B20, grb:B21) | client/core/StreamingScheduler.js:74-75 | 8 per frame, 64 in flight (src/planet_lod/stream.rs:17-18) | 4 | open |
| hit:veg.upload | vegetation upload bytes per frame per kind | no byte cap (grb:nf1) | client/core/WebGPUInstancing.js, Grass.js, Rocks.js (searched) | 64 KiB tree, 64 KiB rock, 512 KiB grass (src/planet_veg/gpu/buffers.rs:43-48) | 4 | open |
| hit:veg.tasks | vegetation placement tasks in flight | no numeric budget (grb:nf2) | client/workers (searched) | async pool threads x 2 (src/planet_veg/stream.rs:25, :216) | 4 | open |
| hit:asset.bytes | render asset upload bytes per frame (meshes, images) | none stated | not in the reference | RenderAssetBytesPerFrame unset, so unlimited (bevy_render-0.20.1/src/render_asset.rs:505-511) | 4 | open |
| hit:leaf.select | leaf selections per moving frame | none stated | not in the reference | two: patch and water (src/planet_lod/stream.rs:114; src/planet_water/surface.rs:253) | 4 | open |
| hit:collider.finish | collider builds started and finished per frame | collider byte budget cap x 8 x 512 (geo:bud.colByte) | src/terrain/ColliderStreamer.js:5 | no per-frame cap (src/planet_lod/collider.rs:143-153, :157-170) | 4 | open |
| hit:red | one synchronous instance-set build put back (veg-place-on-main) is caught; patch and collider rows too | none | not in the reference | scripts/mutants/planet-hitch.tsv (to write) | 4 | open |
| hit:bench | planet render benchmark next to the physics benchmarks | frame-time gate x1.10 baseline, 8000 ms capture (grb:B29, grb:B30) | scripts/frame-time-gate.mjs:32, :45 | none; src/bench.rs is physics only (src/main.rs:146-148) | 4 | open |
| hit:gpu | GPU frame time on real hardware | none stated | not in the reference | not measurable on this host (lavapipe); owner's check | 4 | open |

## 7. Open questions

- Q1. Budget reading: is it the per-frame ceiling min(idle p99 + 1 ms, 2 x idle p99) used here, or p99(path) <= idle p99 + 1 ms together with max(path) <= 2 x idle p99? The task text "every frame inside the relative budget" supports the per-frame reading.
- Q2. Idle baseline pose: the end pose only, or the larger of the start and end p99s? The flight's start pose, 100 km up, has far less resident content than its end.
- Q3. Do the startup one-offs count? The 44.7 MB terrain upload, the 45 tree meshes and the atlas bake all land before the paths. dec:budget speaks of streaming along the paths, so this design reports them and leaves them outside the budget. RenderAssetBytesPerFrame (render_asset.rs:505-523) would spread them over several frames if the owner wants them in.
- Q4. Wall time or thread CPU time inside the spans? Wall time is a hitch as the player sees it, but it is load-sensitive on a host that read load 9.29 on 12 cores. Thread CPU time is less load-sensitive but misses lock waits. The design gates on wall time with the load gate and records CPU time as a second column, if a thread-CPU clock is available without a new dependency (unchecked).
- Q5. Bookmarks are ground points, not altitudes. out-100km is 100 km along the chart x axis with a 2 m clearance (bookmarks.js:28-31, :41; relocation.js:3, :115). On the preset radius of 63600 m, 100 km of arc would be about 90 degrees (computed, 100000 / 63600 rad), next to the reference's reach limit of about 87 degrees (bookmarks.js:7; relocation.js:103). Whether localToDir is arc length was not read, which is why the generator in 3.1 takes the directions from the reference itself. The altitude lift (altitude = bookmark km) is a port choice, and earlier stages chose differently: the stage 2 sky orbit is at 70 km (src/planet_sky/sky/tests.rs:312), water at 12,000 km (src/planet_water/render_tests.rs:38), terrain at 9,000 km (src/planet_material/terrain/tests.rs:357), and the patch witness uses radius x 1, x 0.1 and x 0.005 (src/planet_lod/stream.rs:250-252). The owner should fix one pose set for stage 4.
- Q6. Binary-side counters: should builds_on_main_thread (guard.rs:18) and the headless app (main.rs:3-4) be un-gated for the bench, or should the bench carry its own minimal copies? Should build_collider call guard::note_build, so that one counter covers every build kind instead of the separate thread-name counter (collider.rs:13-20)?
- Q7. Frame wall time on lavapipe for a 2100-frame flight and a 4800-frame ground run at 320 x 180 is unmeasured. If one window would pass about 30 minutes (spec, Stages), the run is split per path.
- Q8. Is the async compute pool's thread count on this host large enough that the vegetation finish (all ready tasks in one frame, bounded by threads x 2) and the patch finish (8) are realistic? AsyncComputeTaskPool::thread_num() was not read at run time.
- Q9. Should stage 4 fix the unbounded items in 1.12, or only measure them? Examples: share one leaf selection between patches and water; cap retire_patches and collider finishes per frame; make patch and water meshes RENDER_WORLD only so extract moves them instead of cloning; set RenderAssetBytesPerFrame. The witness decides. Any item that breaks the ceiling is fixed with its own budget row.
