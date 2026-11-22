# Planet stage 3 vegetation: design

Plan: /workspace/hyperagent/specs/bevy-planet-parity.md, Stage 3 and Architecture 5. Table: docs/planet-parity.md rows veg:* and grb:*, plus the measured species triangle table. Reference: /workspace/spoint (read only). Port: Bevy 0.20.1 (vendored under /config/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f), Rust, avian3d 0.8.
Shaders delivered with this design (all six validate with naga 30.0.2, the version Bevy 0.20.1 builds, all capabilities on): assets/shaders/planet/cull_vegetation.wgsl, vegetation_tree.wgsl, vegetation_impostor.wgsl, vegetation_capture.wgsl, vegetation_grass.wgsl, vegetation_rock.wgsl. They are plain .wgsl on purpose (see Why plain WGSL). Nothing here was compiled into the crate and no .rs file was touched. The validator is a scratch crate, /workspace/tmp/bevy/stage3/naga-check (own target dir, offline); run: naga-target/release/naga-check <files>.

## Decisions in one place

1. Vegetation is data in GPU storage buffers. Entities do not grow with trees: one draw-set entity per (kind, geometry set), at most 54 for the whole planet (45 tree geometry sets, 6 rock types, 1 shared impostor, 2 grass LODs).
2. Placement is the reference's pure function ported to Rust, run per 32 m chunk on the async compute task pool, witnessed against a fixture made by running the reference's real VegPlacement.js, RockPlacement.js and GrassPlacement.js under node.
3. Culling is a compute pass (cull_vegetation.wgsl) that writes per (geometry, tier) visible index lists and DrawIndexedIndirect arguments. Draws are single draw_indexed_indirect calls with first_instance 0, so neither MULTI_DRAW_INDIRECT nor INDIRECT_FIRST_INSTANCE is needed (both are missing on WebGPU baseline).
4. Impostors: one shared octahedral atlas, baked once at startup by a render node that renders each species from the 8 by 8 octahedral directions into one 2048 by 2048 atlas pair, no camera entities.
5. Positions are stored in a per-chunk chart (anchored at the chunk centre) and re-based each frame against the floating origin in a small chunk table, so f32 holds at radius 6,360,000.
6. Placement parity is witnessed in world space (planet centred), so the port may choose its own chart anchor.

## What the reference does (read in full, with the numbers that matter)

Placement, src/terrain/VegPlacement.js (303 lines):
- VEG: CHUNK 32, CELL 4, GRID 8, JITTER 1.8 (jitterOverCell 0.45), SLOPE_D 1.5, SLOPE_MAX 0.6, SEA_REJECT -2, TREELINE 4000 with 120 m fade, both scaled by reliefMarginScaleOf(frame) = reliefScale / 0.01. At the preset (reliefScale 0.001) that is 0.1: the treeline is 400 m with a 12 m fade. This is easy to miss; the port must carry relief_scale into the placement.
- Per cell: cell direction from the lattice with jitter drawn from rand(hash3(seed ^ 0x7eed, row, j), K_JITX/K_JITZ); a second hash hash3(0x5eed, row, j) gives coin and every later draw; keys K_JITX 0 through K_COMPANION 16 (row veg:key.table).
- Density: baseDensity(temp, humidity, erosion) times groveMul = (0.15 + 1.45 smoothstep(0.30, 0.70, grove)) times the lattice cell-area correction; coin must be below BASE_DENSITY_CEILING * groveMul before the (expensive) surface solve and below base * soilMul * painted.grass after it. soilMul = renderedSoilWeight(elev) (smoothstep 4.5 m to 15 m) times the treeline fade. Snow dominance rejects. Climate defaults 0.5, 0.5, 0.3; seaBias below -2 rejects.
- Species: speciesFor on 0.1-rounded jittered temperature and humidity (jitter = 0.30 per-cell plus 0.60 patch noise at 64 m cells), arid (unjittered humidity below 0.28) gives Bush, else Pine, Oak, Ash, Aspen by the cuts in veg:species.genus; 22% companion genus; understory Bush chance 0.08 + 0.35 (1 - groveWeight); size class from p = 0.7 sizeR + 0.3 (1 - elevNorm) with cuts 0.40 and 0.75; sibling table maps genus and class to one of the 15 names.
- Instance record: x, y, z (f32 chart-local), species, shape (0 to 2), scale = (0.62 + 0.72 age) times (0.90, 1.0, 1.10) with age = rand^1.4, yaw, windPhase, tint (3 f32 from hue, saturation, value, warm and cool targets, season noise at 420 m cells), tiltQuat = tangentFrameQuat * lean quaternion, normal, trunkId = lattice cell id (a double up to 6 * cellsPerFace^2; the reference hashes id|0, so only the low 32 bits matter).
- Hash functions hash3 and rand use Math.imul with the constants in rows veg:place.hash.fn and veg:place.rand.fn. valueNoise3 (PlacementChart.js:245) uses smoothstep fades and hash(hash(seed, ix, iy), iz, 0).

Rendering, client/core/Vegetation.js (926 lines), VegetationBuild.js, VegImpostorTier.js, WebGPULodInstancer.js, VegetationTSL.js, VegetationAmbientTSL.js:
- renderDistance 640, ring = 680, drop = 744; MAX_INSTANCES 30000 caps resident instances; chunks load nearest-first in a spiral under a 2 ms per-frame budget (rows veg:budget.load_ms, veg:tier.ring).
- Thinning is applied when a chunk loads, with the player position of that moment: a tree further than 96 m survives only if coin(trunkId) is below keepProb(d) = 1 - 0.85 smoothstep((d - 96) / (640 - 96)), floor 0.15 at 640 m (Vegetation.js:85-100 and :398). The coin is the integer hash ((id ^ 0x9e3779b1), imul 0x85ebca6b, ^ >>> 13) / 2^32.
- Tiers (rows veg:tier.*): D1 = min(14, 0.045 R), D2 = min(35, 0.11 R), D3 = min(R, max(D2 + 12, 0.13 R)), with R the render distance. At 640: 14, 35, 83.2. Hysteresis 0.12: a tree moves to the farther tier when d >= D (1 + 0.12) and stays in it until d < D (1 - 0.12). NEAR_CUTOFF = 0.92 D3 = 76.544; impostor start = 73.544; mesh end = far swap = max(D2 (1 + h), cutoff + 3) = 79.544; mesh re-entry = max(nominal (1 - h), cutoff + 0.5) = 77.044 where nominal = far swap / (1 + h). The impostor fades in by an ordered dither over 3 m (73.544 to 76.544).
- Shadows: branch geometry only (b2shadow, simplified 0.07), cast distance 35 with the same hysteresis (on at 30.8, off at 39.2). Leaves cast none.
- Per tree: branch instancer and leaf instancer each with 3 mesh tiers (geometry simplified by meshopt: branch 0.28 strict and 0.07 sloppy, leaf 0.30 and 0.09; tier-0 caps 6000 and 1400 triangles). There are 15 species times 3 shape variants = 45 geometry sets. Only variant 0 is baked to an impostor; variants reuse the species tile.
- Wind (VegetationTSL.js:12-19): displacement in local space before the instance matrix, sway = position.y * 0.06; phase = t * 1.3 + windPhase; x += sin(phase) sway w; z += cos(phase 0.8) sway 0.6 w; w default 1.
- Impostors (octahedral-impostor-display-tsl.js, createTextureAtlas in octahedral-impostor-ez.js): 8 by 8 sprites per species, 512 px per species bake (64 px per sprite), alphaClamp 0.4, hemi off, far-single-sprite off (three-sprite blend), shared atlas grid ceil(sqrt(15)) = 4 so the atlas is 2048 by 2048 (the "1024" in the plan is the fallback width only; the baked width is 512, VegImpostorTier.js:28). Capture camera is orthographic with half-extent equal to the bounding-sphere radius, placed at centre + octaGridToDir(col/7, row/7) * radius, near 0.001, far 2 radius + 0.001; two targets: albedo (alpha cut 0.2) and view-space normal in rgb with 1 - depth01 in alpha.
- The reference's WebGPU "culling" is not the compute probe in WebGPUCullingHubIntegration.js (that file is a CPU versus GPU frustum A/B with synthetic data, veg:cull.ab.*). The production cull is CPU: WebGPULodInstancer.js groups instances into 24 m cells (at most 4096, target occupancy 4) and tests cells then instances (rows veg:cull.cell, veg:cull.cell_occ). Architecture 5 asks for GPU culling, so cull_vegetation.wgsl is the GPU form of the same per-instance rules (tier with hysteresis, far swap, shadow list, frustum sphere) and not a port of the cell grid.

Grass, src/terrain/GrassPlacement.js, client/core/Grass.js, GrassTSL.js, GrassClump.js, GrassWindField.js, GrassMaterial.js:
- CHUNK 32, CELL 2, GRID 16, JITTER 0.9, SLOPE_D 1.0, SLOPE_MAX 0.9, 79 blades per cell, CLUMP_R 1.15; keys 20 to 26 (K_JITX to K_WIND); cell hash hash3(0x6a55, row, j) and seed worldSeed ^ 0x6a55; density = clamp01(grassDensity * soil * painted.grass * area); blade accepted when coin < density, q = coin / density; ground colour (groundRgb) from the biome band code of terrain.
- Render distance 88, drop margin 16, MAX_INSTANCES 540000; two geometries (near makeClumpGeo(2,1) within 8 m, mid makeClumpGeo(1,1)); instance scale (2.268 + 1.176 tint, 0.75 scale, same width); a blade stays while d < min(16 / q^(1/1.25) + 5, ring cap) with ring cap = max(22, 88 - 8) = 80; vertical fade over 5 m, growth to 1.3 at 0.005 per metre beyond 16 m, camera-facing blend 24 to 44 m, benders (8, radius 1.1, strength 0.35), scorch decals (8), gust from the wind field.
- The wind field is a 64 by 64 half-float texture (192 m extent, 3 m cells) that the CPU refills from grassWindGust; the shader samples it bilinearly. The port evaluates the same function at the blade (exact, no texture).
- Clump alpha texture (GrassClump.js): 256 px, 22 blades from mulberry32(0x2f6e2b1), 14 steps per blade, drawn as grey polygons on black in ascending height order. Needs a canvas. Ported as an analytic bake in compute (bake_clump), fed by the 22 blade parameter rows.

Rocks, src/terrain/RockPlacement.js, client/core/Rocks.js, RocksTSL.js, RockShapes.js:
- CHUNK 32, CELL 8, GRID 4, JITTER 3.2, 6 types from marching-cube SDF meshes (seed 1337 + type * 7919, res 16; src/rock_shapes.rs and src/rock_tri_table.rs already port the hull and the table), scale table 10 log levels from 2.0 span 8.0, squash 3 log levels from 0.55 span 0.55; instance matrix = tiltQuat * yawQuat, scale (s, s squash, s); colour = variant tint min(1, 0.78 + 0.4 variant) times a three-octave value-noise ramp (0.30,0.28,0.25) to (0.66,0.62,0.56) times 2.05; ring = 320 + 40, drop +64, 12000 max, one BatchedMesh draw.

## Reading Bevy 0.20.1 (evidence for each decision)

- Shaders. bevy_shader-0.20.1/src/shader.rs:281-340: a .wgsl asset is handed to the compiler unchanged (Shader::from_wgsl, imports empty); only .wesl resolves imports of bevy_pbr. The stage 2 design reached the same fact. So the six files delivered here are self-contained WGSL and cannot import bevy_pbr functions or the View struct.
- Storage buffers. bevy_render-0.20.1/src/storage.rs:60 ShaderBuffer asset (GPU side GpuShaderBuffer, :293); AsBindGroup accepts #[storage(N, read_only)] Handle<ShaderBuffer> (render_resource/bind_group.rs:125-151 docs). For the custom draw path raw wgpu buffers in a render-world resource are simpler (RawBufferVec, BufferVec in render_resource/buffer_vec.rs:40 and :491, StorageBuffer in storage_buffer.rs:37).
- Instancing by hand. examples/shader_advanced/custom_shader_instancing.rs defines a RenderCommand that binds the mesh's vertex and index slices from MeshAllocator (lines 320-345: mesh_vertex_slice, mesh_index_slice) and calls draw_indexed(range, base_vertex, 0..instance_count); its camera carries NoIndirectDrawing (view/mod.rs:903) because the command does direct draws. custom_phase_item.rs and specialized_mesh_pipeline.rs show the Opaque3d phase item and SpecializedMeshPipeline route.
- Indirect draw. render_phase/draw_state.rs:451 draw_indexed_indirect (single, baseline WebGPU), :563 multi_draw_indexed_indirect and :604 the count variant (native features only). Bevy's own indirect path (batching/gpu_preprocessing.rs:977 IndirectParametersBuffers, pbr render/build_indirect_params.wesl, mesh_preprocess.wesl) builds arguments for mesh entities from CPU-known batches; it cannot hold millions of instances without one entity each, so it is the wrong tool. Our commands write their own arguments.
- Compute and passes. Bevy 0.20 renders by schedules: RenderGraph is a Schedule (renderer/mod.rs:41-48) and an app system added as game_of_life.before(camera_driver) runs a compute pass before every camera (examples/shader/compute_shader_game_of_life.rs:104-110). Core3d has sets Prepass, MainPass, EarlyPostProcess, PostProcess (bevy_core_pipeline-0.20.1/src/schedule.rs:50-69), so the cull system runs in Core3dSystems::Prepass ahead of the first draw, per main view.
- Shadows. pbr render/light.rs:2800 pub struct Shadow with impl PhaseItem at :2853. The tree shadow casters are queued as Shadow phase items with vs_tree_depth and the light view uniform; this was not compiled and is spike 3 below.
- Async. bevy_tasks AsyncComputeTaskPool, examples/async_tasks/async_compute.rs.
- Offscreen. RenderTarget::Image (examples/3d/render_to_texture.rs:78), Screenshot, lavapipe on this host (stage 2 doc). The impostor bake does not need cameras: a render-graph style system builds one command encoder and issues begin_render_pass with set_viewport per cell (same pattern as the game_of_life node, RenderContext from renderer).
- Limits. wgpu default max_storage_buffers_per_shader_stage is 8: cull_trees uses exactly 8 (trees, chunks, geo bounds, state, slot layout, draws, visible, stats); do not add a ninth. Vertex shaders here read 3 storage buffers (4 for the impostor, which adds impostor_dims).

## Data in GPU buffers

All instance structs are plain repr(C) Pod with the field order of the WGSL struct. Sizes are the maxima (capacities are constants, defaults below).

- ChunkRecord (32 B, one per resident chunk, 2048 slots, shared by trees, rocks and grass): origin (vec3, the chart origin relative to the camera, f64 subtracted on the CPU then rounded), generation (u32), rotation (quat, chart to planet axes: columns east, up, north; the chart is right-handed because north = east cross up in tangentFrameInto). Re-based and uploaded every frame (64 KB).
- TreeInstance (64 B): local_position, scale, tilt (frame * lean), yaw, wind_phase, tint_rgb8 (tint / 2 as unorm8, the tint maximum is about 1.43), trunk_id (cell id mod 2^32, only its hash matters), species_and_shape (species | shape << 8), chunk (slot | generation << 16), two reserved words. Capacity 32768 slots (64 B * 32768 = 2 MB); the resident cap is the reference 30000.
- RockInstance (48 B): local_position, scale, rotation (tilt * yaw, as the reference composes), squash, variant, kind, chunk. Capacity 12288.
- GrassInstance (64 B): local_position, scale, rotation (tilt of the surface normal * yaw), ground_rgb, tint, rank (q^(1/1.25)), wind_phase, shadow, chunk. Capacity 540000 (34.6 MB).
- Per geometry state: tree_state (u32 per tree: tier, no-mesh flag, shadow flag, in-ring flag, ready flag). Rock state likewise.
- Visible lists and arguments: tree slots = geo * 4 + tier (0 to 2 mesh tiers, 3 shadow list) plus one impostor slot = 181 slots; per-slot capacities 256, 1024, 2048 and 512 for a geometry (bounded by area: one tree per 16 m2 at most and the tier radii including hysteresis, so tier 2 inside 89 m holds at most about 1555) and 32768 for the impostor slot; 205,568 u32 entries (822 KB). Rocks: 6 slots of 12288. Grass: 2 slots of 540000. Arguments: DrawArgs (20 B: index_count, instance_count, first_index, base_vertex, first_instance); the CPU writes the static fields once from the mesh slab slices and rewrites them only if the slab moves; the compute pass resets and fills instance_count and clamps it to the capacity (finish_*).
- Stats (CullStats, 12 atomics): overflow (must stay 0), tree_mesh, tree_impostor, tree_shadow, tree_resident, tree_thinned, tree_frustum_culled, rock_visible, grass_near, grass_mid. Read back asynchronously for the witnesses only.
- Impostor atlas: two 2048 by 2048 Rgba8Unorm textures (albedo, normal-depth) with mips, 22 MB together, plus impostor_dims (centre, radius per species).

Per-frame upload budget (the reference has none, rows grb:nf1 and grb:nf2, so these are port decisions recorded in the table): chunk table 64 KB fixed; new chunk instances at most 64 KB for trees and rocks and 512 KB for grass per frame, drained from a queue nearest chunk first; one chunk of grass is up to 20224 blades (1.3 MB) so large chunks are split across frames by sub-ranges.

## How placement leaves the main thread

1. chunk_stream (system, main thread, O(ring) integer work only): from the camera planet position and the placement lattice, list the chunk keys of each kind inside its ring (trees 680 m, rocks 360 m, grass 88 m; drop radii 744, 424, 104) with the reference's hysteresis (load at ring, keep to drop). New keys are sorted nearest first and spawned as tasks on AsyncComputeTaskPool, at most 2 times the pool thread count in flight.
2. A task is the pure function place_*_chunk(context, key): it owns Arc handles to the Planet, the climate field and the splat function (all immutable after startup) and returns a Vec of packed instances plus the chunk anchor. No Bevy resources, no locks.
3. Results arrive on a channel; the main-thread system pops up to the per-frame byte budget, allocates a range in the kind's RangeAllocator, assigns a chunk slot and generation, and queues the byte upload; the render world writes the buffers in prepare. Unload only bumps the generation of the chunk slot (a few bytes); the shader rejects instances whose generation differs, so freed ranges need no clearing.
4. Total thread cost at the preset anchor measured in node (JS, one thread): 0.5 to 0.8 ms per tree chunk, 1863 chunks in the ring = 0.78 s; the Rust port should be at or below that. The main thread does a fixed amount of work per frame regardless of chunk count.

## Rust modules (one job each, none exist yet)

Directory src/planet_veg/ (registration lines are the lead's). Existing src/vegetation.rs (the 6-trees-per-chunk demo) is replaced and deleted when this lands.

- veg_hash.rs: hash3, rand, value_noise3 and the sea_noise wrapper; pure, integers exact (imul, rotations as in the reference).
- veg_species.rs: SPECIES, shape variants, species_for, size_class_of, size_sibling, base_density, rendered_soil_weight, relief_margin_scale, grove_at, tint_multiplier, lean_quat; constants only from the reference.
- lattice.rs: the placement lattice. src/height/placement.rs already holds PlacementLattice behind cfg(test) with its fixture (gen-placement-lattice.mjs); un-gate it and move it here (it is used by the binary now).
- chart.rs: per-chunk chart: anchor basis (east, up, north), surface_along_dir via Planet::height (the reference cpu-solve path, PlacementChart.js:117-122), tangent_frame_into, tangent_frame_quat, quat_mul_f32, radial_slope (4 probes at SLOPE_D). Reuse src/height/planet_frame.rs ChartFrame where it already has local_to_world and anchor_basis; add what is missing there rather than copying.
- tree_placement.rs: place_tree_chunk(&PlacementContext, key) -> Vec<TreeInstance>; test-only TreeRecord with the unpacked f64 and f32 fields for the fixture.
- rock_placement.rs, grass_placement.rs: same shape (grass also needs groundRgb, which depends on the stage 2 splat and biome colour code).
- clump.rs: mulberry32, the 22 blade parameter rows (sorted by height as drawn), the ClumpBlades uniform.
- chunk_stream.rs: ring policy and task spawning above.
- instance_pool.rs: RangeAllocator (first fit, coalescing), ChunkTable (slot, generation, f64 origin, basis), the pending-upload queue with byte budgets.
- species_assets.rs: loads the baked species set (below) and builds the 45 geometry sets with tiers.
- cull_pass.rs: pipelines for the nine compute entry points, bind groups, per-frame dispatch (reset, cull, finish), stats readback.
- draw.rs: the RenderCommands (set view, set instance buffers, set material, set draw slot with a dynamic offset, draw_indexed_indirect) for main, depth and shadow, and the queue system that adds phase items per draw-set entity and view.
- impostor_atlas.rs: the capture render node, the layout contract, the mip chain.
- veg_uniforms.rs: VegView, VegLight, GrassParams from the camera, the sun and the shared look values (stage 2 frame_params).

Dependencies on other stages (found by reading, not by guessing): temperature, humidity and erosion at a direction (the stage 2 design plans src/height/climate.rs for temp and humidity from the anchor-field band; placement also needs erosion and seaBias, so that module must expose all four channels; src/climate_cache.rs already ports the 8 m sector cache and the Climate struct with all four fields); splat weights (stage 2 plans src/planet_material/splat.rs, a literal createSplatWeights port); the biome colour used by grass groundRgb (terrain constants, stage 2); the floating origin (src/planet_lod/origin.rs).

## The pure placement function and its witness

Does the reference import under node and what must be stubbed? Measured with scripts in /workspace/tmp/bevy/stage3 (run with node, nothing stubbed):
- /workspace/spoint/src/terrain/VegPlacement.js imports cleanly under node: dynamic imports of mapspinner/splat-weights resolve through spoint's own node_modules. Importing it and PlacementChart.js, PlacementLattice.js, PaintedWeights.js, ClimateCache.js, PlanetFrame.js needs no stub. RockPlacement.js and GrassPlacement.js also import cleanly (they import mapspinner/terrain-defaults.js and tsl/ops-jsgen.js, which resolve).
- What the call needs are real inputs, not stubs: a height sampler (createHeightSampler from packages/mapspinner/src/height-cpu.js), an anchor field (createAnchorField from anchor-field.js) wrapped by createCachedAnchorField (ClimateCache.js), and a planet frame per chunk (createPlanetFrame from PlanetFrame.js, anchorDir at the chunk centre from lattice.chunkCentreDir, reliefScale 0.001). hashVersion comes from src/shared/terrainConfig.js DEFAULT_TERRAIN_HASH_VERSION.
- GrassClump.js needs only a stand-in for document.createElement('canvas'): a recording proxy for the 2D context. It then imports (three resolves) and makeClumpAlphaTexture runs: 750 recorded calls (22 blades). The recorded moveTo, lineTo, fillStyle calls are the reference's own blade polygons, the fixture for clump.rs without re-implementing anything.
- GrassWindField.js imports three but grassWindGust is a pure function; no stub.
- The reference runs its placement against ExactPatchFrame.placementFrame (heights read from the rendered patch mesh); the fixture uses the plain sampler frame (cpu-solve), which is Planet::height.

Measured outputs of those scripts (seed 1337, radius 63600, reliefScale 0.001, preset anchor direction (-0.641, 0.2558, 0.7237)):
- proto.mjs: placementsForChunk returns records with all fields; a chunk with 3 trees took 5 ms cold.
- scan.mjs, ring 400 m: 639 chunks, 564 with trees, 4652 trees, 0.80 ms per chunk, densest chunk 30. Species present 0, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12.
- scan3.mjs, ring 680 m at 8 poses: preset anchor 1863 chunks, 9888 placed, 4982 resident after the reference thinning, species 0 and 2 to 12; pose (-0.5, 0.2, 0.8) 10962 placed, 4700 resident, densest chunk 39; pose (0.2, 0.9, -0.4) is cold and has all 15 species including the pines (1, 13, 14) with 854 placed; five poses have no trees at all (ocean or bare). No pose gets near 30000 (see open question 1).
- scan2.mjs: rocks 91 in a 300 m ring (362 chunks); grass 88058 blades in 16 chunks (64 m ring), so the 540000 cap is reached at about 100 chunks, near the 88 m ring.

Generator (Phase B): scripts/gen-veg-placement.mjs using writeFixture from scripts/lib/spoint.mjs, building the sampler, anchor field and a frame per case exactly as scan3.mjs. Cases: seeds 1337 and 0x5eed1234; radii 63600 (with relief 0.001), 6360, and 6360000 (the trunkId above 2^32 case); chunks chosen by a deterministic scan (smallest keys) so that every species 0 to 14 appears, plus an empty chunk, a coast chunk (beach soil weight), a steep chunk (slope reject), a treeline chunk (elevation above 400 m at relief 0.001), a pole chunk (pole frame branch POLE_REF_SWITCH), and two adjacent chunks for the seam. Per placement it stores the unpacked fields and the world position (frame.localToWorld of the rounded local values, f64). The test include_str!s the fixture and compares per chunk: count, then per placement exactly species, shape and trunk_id mod 2^32, close() with explicit tolerances for scale, yaw, wind_phase, tint (1e-6), tiltQuat (1e-6), normal, and world position (2e-3 m, because the reference rounds the chart-local values to f32). Re-running the generator must leave the file byte-identical.
Similar generators: gen-veg-hash.mjs (hash3, rand, valueNoise3, groveAt, speciesFor, tintMultiplier, leanQuat exact integers), gen-rock-placement.mjs, gen-grass-placement.mjs (a few chunks only; each is 5500 blades), gen-grass-clump.mjs (recorded canvas calls and the blade parameters; the bake is compared by coverage against a polygon fill done in the test from the same calls, tolerance recorded), gen-grass-wind.mjs (grassWindGust at a grid of positions and times).

## Culling, tiers, thinning (cull_vegetation.wgsl)

Entry points: reset_tree_draws, cull_trees, finish_tree_draws, reset_rock_draws, cull_rocks, finish_rock_draws, reset_grass_draws, cull_grass, finish_grass_draws (one thread per instance or per slot, workgroup 64; grass at 540000 is 8438 workgroups).

Per tree, in this order:
1. Reject instances whose chunk generation differs (unloaded chunk).
2. Distance in render space; in-ring with hysteresis (enter 680, stay to 744); otherwise clear state.
3. Thinning: distance above 96 m and tree_coin(trunk_id) >= keep_probability(d) removes the tree (same coin function and smoothstep law as the reference). Deviation recorded: the reference decides at chunk load time with the player position then; the port evaluates every frame, so a tree reappears as the camera approaches. The reference behaviour (frozen at load) leaves thinned chunks near the player when they loaded while far away; the port deliberately does not copy that. Checked: nothing refreshes such chunks; discardedPlacementKeys is filled only by rebuildPlacement (Vegetation.js:917-918, called after a biome paint), so a chunk keeps the thinning of its load moment until it leaves the 744 m drop radius. See open question 5.
4. Mesh tier via next_mesh_tier: raw tier by d >= D (1 + 0.12), keep the farther tier until d < D (1 - 0.12), no-mesh when d >= 79.544, re-enter the mesh at 77.044. The state byte holds the previous result, as the reference's per-record tier does.
5. Shadow flag: on at 30.8, off at 39.2; the shadow list is not view-culled.
6. Frustum sphere test (centre rotated bounds centre, radius times scale); then append to the mesh-tier list and, from 73.544 m, to the impostor list (the shader dissolves it in over 3 m).
Counters feed CullStats. Lists are per (geometry, tier), so branch and leaf draws of the same tier share one list.

Rocks: ring 360 enter, 424 drop; frustum; list per rock kind. Grass: keep while d < min(16 / rank + 5, 80); frustum sphere; near list within 8 m, mid list beyond; no state.

Expected transition distances for the cull witness (render distance 640): mesh tier 0 to 1 at 15.68 (back at 12.32), 1 to 2 at 39.2 (back at 30.8), mesh to impostor at 79.544, impostor to mesh at 77.044, impostor fade 73.544 to 76.544, shadow on 30.8 and off 39.2, thinning knee 96 and floor 640, ring 680 and 744.

## Draw calls and entities

Main pass draw calls are bounded by species times variants times tiers times parts plus the fixed ones:
- trees: 15 species * 3 shape variants * 3 tiers * 2 parts (branch, leaf) = 270
- shared impostor: 1 (all 15 species in one atlas, one list)
- rocks: 6 (the reference batches them into one multi-draw; WebGPU baseline has no multi-draw, so one indirect call per kind)
- grass: 2 (near, mid)
Total at most 279. Shadow views add 45 branch shadow draws per shadow view (45 * cascade count). A depth prepass, if enabled, repeats the 279. A draw whose list is empty still issues one indirect call with instance_count 0 (cheap) so the bound holds by construction and the witness is an equality: counted vegetation draws per frame is at most 279 at any tree count.

Entity rule: the Bevy world contains one draw-set entity per (kind, geometry set): 45 tree sets, 6 rock kinds, 1 impostor, 2 grass LODs = 54 entities, spawned once at startup. Placing 1,000 or 30,000 trees changes buffer contents and a chunk table, not entity count. The queue system adds the per-tier, per-part phase items for each set (several phase items per entity differ by bin key).

## Impostor capture plan

Atlas layout contract (the sampling code in vegetation_impostor.wgsl is the reference WebGPU path, atlasFlipY = true): sprite (sc, sr) of tile (tc, tr), tile = species index (tc = species mod 4, tr = species div 4), grid 4, 8 sprites per side, 64 px per sprite, so a tile is 8 * 64 = 512 px and the atlas is 4 * 512 = 2048 px. Pre-flip uv of a point (u, v) in the sprite is ((tc + (sc + u) / 8) / 4, (tr + (sr + v) / 8) / 4) with v up, and the sampled uv is (x, 1 - y). So the sprite rect on the image is x from ((tc * 8 + sc) / 32) * 2048, y_top = 2048 - ((tr * 8 + sr + 1) / 32) * 2048, size 64. Sprite (sc, sr) is rendered with the camera direction octaGridToDir(sc / 7, sr / 7) (the same encode and decode functions as the shader).
Capture node (impostor_atlas.rs), a system that runs once at startup after the species meshes load, in budgeted slices (64 cells per frame, one species per frame, 15 frames):
- Render pass over the whole atlas pair (clear both to 0 once), per cell set_viewport and set_scissor to the rect, uniform CaptureView with the orthographic clip from the bounding sphere of the unscaled species tree (branch plus leaf), view = rotation built from the plane basis (tangent, bitangent, normal) of the cell direction computed with the shader's own plane_basis, not a look_at, so capture and sampling share one basis at the poles. near 0.001, far 2 radius + 0.001, half extent radius, camera at centre + direction * radius.
- Pipeline from vegetation_capture.wgsl: two colour targets Rgba8Unorm, depth 2048 transient, double sided, discard below max(0.2, alpha_test).
- After all cells: mip chain by successive box downsample passes (the reference generates mipmaps on the mega atlas, anisotropy 4); albedo linear filtered, normal nearest.
- Impostor dimensions per species (bounding sphere centre and radius) go to impostor_dims. Variant 0 bakes; variants 1 and 2 reuse the species tile (VegImpostorTier.js and Vegetation.js:190, :241).
Round-trip witness: render a species tree from direction d (a cell centre) with the normal pipeline into an offscreen target, render a single impostor of the same species at the matching distance from the same direction, compare silhouettes: intersection over union at least 0.95 (threshold proposed, set from the measured floor).

## Species geometry

The 15 species and their 3 shape variants are generated procedurally by @dgreenheck/ez-tree through VegetationBuild.js (stage 0 already ran loadPreset under node to measure the triangle table above, which shows the library runs under node). There is no Rust port and none is proposed. Plan: a script (scripts/bake-species.mjs, Phase B) runs the reference's own build (speciesLodChain: presets, the 6000 and 1400 triangle caps, meshopt 0.28 strict and 0.07 sloppy for branches, 0.30 and 0.09 for leaves, the shadow geometry at 0.07, variant tint biases) and writes 45 geometry sets as glTF assets (branch and leaf, 3 tiers each, shadow, bounding sphere, leaf alpha texture) under assets/vegetation/. Rust loads them through bevy_gltf and treats them as data. The measured species table in docs/planet-parity.md gives the tier-0 input triangles; the post-meshopt counts are measured by the same script and recorded then (row text: "open until stage 3").

## Look, lighting, wind

Per frame: VegView (clip_from_world, world_from_view, eye, time, wind strength), VegLight (sun direction, sun irradiance, exposure, ambient albedo multiplier and add, night fill, look saturation and contrast, grade_mode), GrassParams (colours, wind frame, rings, benders, decals). grade_mode 0 emits scene-linear and lets the camera tonemap; 1 applies the reference grade (day shade, night fill, exposure, ACES filmic with constants 2.51, 0.03, 2.43, 0.59, 0.14, saturation, contrast, gamma 1/2.2) for a Tonemapping::None camera. The stage 2 design also assumes a None tonemapper; both pieces must agree on which side applies the grade.
Wind: trees as in the reference (formula above, in local space, scaled by the instance scale). Grass: grass_wind_gust is the reference function (hash2i with imul constants, value noise, two waves, flutter) evaluated at the blade with wind coordinates continuous across chunks: wind_origin plus the blade's render-space offset projected on wind_east and wind_north (set per frame from the camera's tangent chart), never the chunk-local coordinates.

## Why plain WGSL (and what it costs)

Bevy 0.20 only resolves bevy_pbr imports in .wesl. The task fixed the file names as .wgsl, and the draw path is a custom RenderCommand with its own bind groups anyway. The shaders therefore light surfaces themselves: Lambert sun term with irradiance / pi, the reference's vegetation ambient delta (albedo * ambient_mul + ambient_add) and the grade. They do not receive Bevy cascade shadows or clustered lights. Shadow casting from vegetation works through the Shadow phase (vs_tree_depth, vs_rock_depth). If receiving cascade shadows on trees is required for the capture comparison, Phase B adds a thin .wesl wrapper that imports bevy_pbr::render::{pbr_functions, shadows} and calls light_surface; that file is outside what this design owned and is listed in open question 2.

## Red checks (each must be seen failing, then restored byte-identical)

Planned, not run (no Rust compiled in this phase). Shader mutants are in /workspace/tmp/bevy/stage3/mutants-vegetation-shaders.tsv (19 lines; every old text occurs exactly once, checked by check-mutants.mjs):
1. Placement parity: mutate each rule once in the Rust modules and expect veg_placement to fail: JITTER 1.8 to 1.7, SLOPE_MAX 0.6 to 0.7, TREELINE 4000 to 4100, TREELINE_FADE, GROVE_CELL_M 96 to 97, GROVE_MUL_FLOOR, hash3 constant 0x27d4eb2d, rand constant 0x9e3779b1, COMPANION_CHANCE 0.22 to 0.23, speciesFor threshold 0.62 to 0.63, size class cut 0.40 to 0.41, SCALE_AGE_SKEW 1.4 to 1.5, tint weight 0.55 to 0.56, K_YAW key 4 to 5, relief margin scale dropped (treeline 4000 unscaled). The same for rocks (CELL, JITTER, density law, scale table) and grass (79 blades, CLUMP_R, density law).
2. Cull parity (veg_cull_parity): a Rust CPU mirror of cull_trees (test-only) and the GPU readback of draw arguments, visible lists and stats agree for a synthetic instance set (the 12000 and 30000 sizes the reference A/B uses, spread 1280 m, rows veg:cull.ab.*) at a camera path that crosses every transition in the table above, in both directions. The WGSL mutants cull.* must each make it fail.
3. Draw-call bound (veg_draw_bound): counted draws at 1000, 10000 and 30000 resident trees are equal and at most 279. Red: spawn one draw entity per tree (what the old design does) and the count and entity witnesses must both fail.
4. Entity rule (veg_entity_count): entity count after streaming 1000 trees equals the count after 30000 (difference 0) and is at most 64. Red: the same mutation as 3.
5. Forest pose (veg_forest_visible): at the dense-forest pose from scan3.mjs, GPU stats tree_resident equals the reference-rule resident count from the fixture within 0.1 percent (coin float rounding) and tree_mesh + tree_impostor equals the CPU mirror after the frustum test; overflow 0. Red: change the thinning floor 0.15 to 0.30 or the ring to 600.
6. Impostor round trip (veg_impostor_roundtrip): mutants imp.flip, imp.fade, imp.dominant, cap.depth.
7. Wind (veg_wind_witness, veg_grass_wind): a tree at fixed time: the image-space tip displacement against the formula; grass: grass_wind_gust CPU function against the fixture and the GPU against the CPU at sample points. Mutants tree.wind.*, grass.*.
8. Unload (veg_stream_unload): chunk unload then reuse of the same slot never shows old trees (mutant cull.generation).
9. Main-thread budget (veg_stream_budget): over a scripted walk the per-frame upload bytes never exceed the budget and no placement runs on the main thread (a thread-id assertion inside place_*_chunk). Red: call place_tree_chunk from the system.

## Rows each part closes

- veg_hash, veg_species, tree_placement: veg:place.* (chunk, cell, grid, jitter, slope_d, slope_max, sea_reject, treeline, treeline_fade, hash.cell, hash.fn, rand.fn, cellhash, coin), veg:key.table, veg:density.* (base, erosion_default, temp_hum_default, grove.seed, grove.cells, grove.mix, grove.smooth, grove.mul, soil, painted, aridity, relief), veg:patch.t, veg:patch.h, veg:species.* (count, list, genus, companion.chance, companion.map, understory, sizeclass, siblings), veg:scale.*, veg:lean.max, veg:shape.variants, veg:tint.*.
- species_assets and bake script: veg:build.height, veg:build.height.fallback, veg:build.tricap, veg:build.lod.simplify, veg:build.variant.tint, veg:build.leaf.alphatest, and the measured table's post-meshopt column.
- cull_vegetation.wgsl and cull_pass: veg:tier.render_distance, ring, d1, d2, d3, near_cutoff, shadow, hysteresis, handoff, far_swap, reentry, falloff.near, falloff.floor, falloff.coin, veg:budget.max_instances, veg:budget.30k_figure (recorded as a cap, not a count), veg:cull.planes; veg:cull.cell and veg:cull.cell_occ become "replaced by the per-instance GPU test" (recorded as a deviation with the reason above).
- impostor_atlas and vegetation_impostor.wgsl, vegetation_capture.wgsl: veg:imp.sprites, texture, alpha, hemi, atlas.grid, atlas.size, bakevariant, capacity, polyoffset (pipeline state), parallax (off by default, not ported).
- vegetation_tree.wgsl: veg:wind.sway, speed, z, default.
- chunk_stream, instance_pool: veg:budget.load_ms, retries, idle, init_cap (replaced by fixed capacities), grb:nf1 and grb:nf2 (port decisions: byte budgets above).
- grass: grb:G01 to G111 except the ones listed under not ported (placement G16 to G27, density G28 to G49, ring G01 to G10, instance law G11 to G15, wind G98 to G110 by the in-shader function, bend and scorch G62 to G67, material G68 to G90, clump G91 to G97).
- rocks: grb:R01 to R57 (R06 and R57 are physics and contact extents, see not ported).
- grb:B31: the frame-time gate's vegetation floor of 1000 instances is met by the cull stats witness; the frame budget rows (B01 to B30) belong to stage 4.

## Not ported, and why

- ez-tree itself (the generator): baked to assets by the reference's own code, not re-implemented.
- The 24 m cell grid and CPU BVH/occlusion cells (veg:cull.cell, cell_occ, bvh.margin, occ_margin): replaced by the GPU per-instance cull. Occlusion culling and the contact occlusion field (ContactOcclusionField.js, treeContactStamps): not ported; no stage row asks for them.
- QualityGovernor leaf gate (leafGateM, setLeafDrawDistance) and CullFreeze: not ported; the leaf gate is off (unlimited) by default in the reference.
- WebGL fallback paths (InstancedMesh2, createOctahedralImpostorMaterial, hemi-octahedron, GLSL grass): the port has one path.
- Normal-mapped capture: the reference capture uses the source material's normal map when present. This design captures mesh normals only; whether the ez-tree materials carry normal maps was not verified; if they do, the capture shader gets a tangent-space branch.
- Tree and rock collision (VegPhysics.js, RockPhysics.js, grb:R06): outside Stage 3's list (collision is Architecture 6).
- Receiving Bevy cascade shadows and clustered lights on vegetation (see Why plain WGSL).
- Grass blending: the reference draws blades with alpha up to 0.92 blended; the shader returns that alpha and the pipeline decides (Mask versus Blend); depth sorting of 540000 blades is not attempted. Recorded as an open choice.
- Wind field texture: exact function per blade instead of a 3 m bilinear texture (differs by the bilinear approximation, bounded by the field's spatial frequency).
- renderMeshHeightAt foot reads (GRASS_FOOT_READS) fall back to the analytic bowl (grassJetAt) until the terrain mesh provides a CPU height query; blade feet may differ from the rendered mesh by sub-decimetre amounts.
- Per-frame cell budgets CULL_ENTRY_BUDGET 8192 and LOD_REVISIT_BUDGET 2048 (rows grb:G57, G59): they exist because the reference culls on the CPU; the GPU pass evaluates every blade every frame.

## Spikes for the first hour of Phase B (each decides a risk before more code)

1. One tree draw: custom RenderCommand with indirect arguments written by hand, shader vs_tree and fs_tree, no culling; confirm slab offsets (first_index, base_vertex) reach the draw and that default GPU preprocessing on the camera does not interfere (the custom_shader_instancing example sets NoIndirectDrawing; check whether it is required when the command touches no mesh uniforms).
2. cull_trees on 30000 synthetic instances, read back stats, compare with a CPU mirror.
3. Shadow phase item for vs_tree_depth (Shadow, light.rs:2800) and the cascade view uniform layout.
4. Impostor bake of one species and the round trip.

## Open questions

1. The Stage 3 witness text says the visible tree count at the forest pose is at least the reference's (more than 30,000). Under the reference's own rules the count cannot reach that: 30000 is the cap (row veg:budget.30k_figure), and the measured resident count in a 680 m ring is 4982 at the preset anchor and at most about 4700 to 5000 at the eight poses scanned (10962 placed at best). Proposed witness: resident equals the reference-rule count from the fixture at the same pose (about 5000 at the preset anchor), and the buffer capacity is 30000 with overflow 0. Raising the ring to reach 30000 is a decision for the lead.
2. Vegetation lighting: accept the self-contained Lambert plus ambient grade (no cascade shadow receive), or add a .wesl wrapper that imports bevy_pbr lighting and shadows. The wrapper is small but it is a new file type next to these.
3. Ownership of climate (erosion and seaBias channels, not only temp and humidity) and of splat weights: stage 2 plans both; placement can only start after they exist with fixtures. Confirm the exported names (Climate with temp, humidity, erosion, sea_bias; splat_weights returning grass, rock, sand, snow).
4. Species assets: confirm that baking the 45 geometry sets with the reference's own code (and copying its leaf and bark textures) is acceptable, including the licence and size of the textures (the reference halves them to 128 px leaf and 64 px bark, rows veg:budget.tex.*).
5. Thinning: per-frame distance (this design) versus decided at load (reference, verified: no refresh except rebuildPlacement). The reference's frozen thinning leaves far-thinned chunks thin when the player arrives; the design treats that as a defect and does not copy it. If exact parity of that behaviour is wanted, cull_trees can use a stored load-time distance instead (one extra u32 per tree).
6. Tree base height: the reference snaps trees to the rendered patch mesh (ExactPatchFrame); this design uses Planet::height. Both agree to the mesh interpolation error; a measure row (tree base versus patch surface, threshold from the level-13 patch size) should be added.
7. MSAA and leaf alpha: the reference turns on alphaToCoverage for leaves when MSAA is present; confirm the Bevy camera's Msaa setting before choosing the leaf pipeline.
8. WebGL2 is not reachable with storage buffers; Stage 5 must say whether the web build is WebGPU only.
