# Planet stage 2 terrain material: design

Plan: /workspace/hyperagent/specs/bevy-planet-parity.md, Stage 2 and Architecture 3-4. Table: docs/planet-parity.md rows mat:*. Reference: /workspace/spoint/packages/mapspinner (read only).
Shader: assets/shaders/planet/terrain_material.wesl. It is .wesl, not .wgsl, for a reason found in the vendored source (see Bevy 0.20 facts).

## Bevy 0.20.1 facts that fix the design

Read in /config/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bevy_pbr-0.20.1, bevy_shader-0.20.1 and bevy-0.20.1/_release-content/migration-guides/shaders_use_wesl.md.

- The naga_oil preprocessor is gone. bevy_shader/src/shader.rs sends a .wgsl asset to the compiler unchanged (Source::Wgsl, shader_cache.rs:338), with no #import and no #ifdef. Only a .wesl asset can import bevy_pbr functions. So the material shader is terrain_material.wesl (WESL is a WGSL superset). A .wgsl copy would be unable to call pbr_input_from_standard_material, apply_pbr_lighting or read view, which is the whole point of extending StandardMaterial.
- Module paths in 0.20: bevy_pbr::render::forward_io::{VertexOutput, FragmentOutput}, bevy_pbr::render::pbr_fragment::pbr_input_from_standard_material, bevy_pbr::render::pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing}, bevy_pbr::render::mesh_view_bindings::view (View.world_position is the camera in render-origin space). The entry point shape is bevy_pbr-0.20.1/src/render/pbr.wesl.
- Shader defs are @if(NAME) on declarations and statements. UInt defs are readable as constants::NAME and also enable the flag NAME. The material bind group is @group(constants::MATERIAL_BIND_GROUP). Extension bindings start at 100 (the example extended_material.rs uses #[uniform(100)]).
- An AsBindGroup extension with no #[bindless] attribute makes the whole ExtendedMaterial non-bindless (extended_material.rs: bindless_supported = base AND extension). Then StandardMaterial binds at group bindings 0 to 12 and up in the plain layout and our bindings 100 to 104 sit beside them. No bindless code is needed.
- 2D array textures: #[texture(N, dimension = "2d_array")] with #[sampler(M)] (example array_texture.rs). Every sample uses textureSampleGrad, because the layer is chosen inside non-uniform branches.
- Material pieces used from StandardMaterial: PbrInput.world_normal (geometric, used by shadow normal offset), PbrInput.N (the lit normal, we overwrite it), PbrInput.V, material.base_color, metallic, perceptual_roughness, reflectance. Directional light, cascaded shadow maps and clustering come from apply_pbr_lighting untouched.
- The host has Vulkan lavapipe (/usr/share/vulkan/icd.d/lvp_icd.json) and no X display: offscreen capture runs on it.

## What the shader does (one pass, one material)

Inputs by name, no vertex shader of our own (the standard mesh vertex shader runs):
- in.world_position, in.world_normal: patch mesh position and normal, render-origin space, planet frame equals world frame.
- in.uv = (temp, humid): the HPF zw channels at the vertex direction (reference vClim).
- in.uv_b = (height_m, 0): the vertex height above the sphere, kept as a vertex value because |p| - R in f32 loses 0.5 m at R 6.36e6 and the beach and band edges are metres wide.
- terrain uniform (binding 100): planet_center (render-origin space, moves with the floating origin), def_radius, tex_cam_frac, relief_scale, sun_dir (planet frame), wetness, bc_rock, pool_cover, mean_l, pool_lo, pool_hi, pool_spec, albedo_override, surf_ready, tex_nrm_k, relief_shade, flat_normal.
- surf_alb (101) and surf_nrm (103): 4-layer RGBA8 array textures, samplers 102 and 104.

Per fragment, in the reference order: dir0, n = normalize(mix(n_geo, dir0, 0.05)), slope = 1 - max(0, dot(n, dir0)), rockSlope = clamp(slope,0,1), pxWorld = max(length(fwidth(rel)), 0.001), wet = wetness when h > 0, warp = 3 snoise at dir0*450 (+7.3, +23.9) times 1.2; wt = (rel + tex_cam_frac)/texTileM + warp; wt derivatives taken before any branch; terrain_albedo_climate (biome colour: seabed, sea ice, shore/lowland/grass/rock/snow bands with the 3-octave band warp, slope rock, mottle, beach, 2 ridge octaves); splat_weights (dry-hot, beach, sand region, rock slope, rock band, snow, sand, grass, underwater move, normalise); top-2 layer pick; triplanar sharpness 3.0, fine x4 and coarse x1 octaves, the octave coincide and far fades, cross-fade by camera distance 100 to 340, normal fade 100 to 1000, finger and soft blend; albedo tint by biome 0.62, texBright 1.12; tex_dn normal perturbation scaled by texNrmK 0.4; pool wetness term; then reliefShade 6 (nRelief) and nLit = normalize(nRelief + texDn).

Lighting handover to Bevy PBR: base_color = albedo, metallic 0, N = nLit, perceptual_roughness 1 and reflectance 0 on dry ground (a Lambert surface, as the reference model is Lambert plus a pool lobe), the pool weight raises reflectance and lowers roughness through roughness = (2/(specExp+2))^0.25. Sun, shadow cascades, ambient and clustering are Bevy's.

Output grade (grade_output, ported from gradeOutput in terrain-lighting-tsl.js): terminator glow, night floor and night fill, exposure 1.05, ACES filmic, lookSat 1.25, lookContrast 1.0, gamma 1/2.2 then sRGB decode so the sRGB target reproduces the display-referred value. The camera must use Tonemapping::None and no DistanceFog, or the grade is applied twice.

Shader defs: HASH_FLOAT (planet hash version Float; default Integer), LAYER_CENSUS (writes top layer id / 255 in the red channel, skips post processing; target is Rgba8Unorm), CENSUS_REMOVE_LAYER = n (UInt, forces layer n's weight to -1 before the top-2 pick, the red check).

Hook for the atmosphere piece: aerial_perspective(rel, d_km) returns trans = 1, radiance = 0 now. The atmosphere module replaces its body from its LUTs; ap_gate, haze mix and the terminator glow already wrap it.

## Rust module list (new, one job each)

Registration lines in src/main.rs and src/planet_lod/mod.rs are the lead's; nothing below exists yet.

- src/planet_material/mod.rs: TerrainMaterialPlugin (MaterialPlugin for TerrainMaterial, load the .wesl, add systems) and type TerrainMaterial = ExtendedMaterial<StandardMaterial, TerrainExtension>.
- src/planet_material/extension.rs: TerrainExtension, TerrainParams, MaterialExtension impl (fragment_shader = "shaders/planet/terrain_material.wesl", specialize pushes shader defs from TerrainKey).
- src/planet_material/layers.rs: decode the 12 JPEGs, the deshade pass, pack to the two array images, mip chains, mean_l and rock_mean; returns TerrainTextureSet.
- src/planet_material/splat.rs: pure Rust CPU mirror of the splat rule (below). No Bevy types.
- src/planet_material/vertex_inputs.rs: the two functions the patch builder calls per vertex: climate_uv(dir) and height_uv(h). Patch code inserts Mesh::ATTRIBUTE_UV_0 and ATTRIBUTE_UV_1 with them.
- src/planet_material/frame_params.rs: one system that writes planet_center, tex_cam_frac, sun_dir, bc_rock, mean_l, surf_ready into the material each frame.
- src/height/climate.rs: Rust port of the anchor-field regional climate band so the HPF has its zw channels (see Climate).
- src/planet_material/census.rs: CPU layer census at a pose and its comparison with the GPU census image.
- src/planet_material/capture.rs: offscreen render to an image, read back, the image measure.
- scripts/gen-splat-weights.mjs: reference fixture (uses lib/spoint.mjs writeFixture) from splat-weights.js.
- scripts/gen-climate.mjs: reference fixture from createAnchorField(...).sampleUV for temp and humidity.
- scripts/copy-planet-textures.mjs: copies the 12 reference JPEGs into assets/textures/planet/ (reads /workspace/spoint, never writes there).
- scripts/capture-reference-poses.mjs: reference captures per pose through playwriter on the display that has a GPU.

## The material extension (AsBindGroup layout)

    #[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
    #[bind_group_data(TerrainKey)]
    pub struct TerrainExtension {
        #[uniform(100)] pub params: TerrainParams,
        #[texture(101, dimension = "2d_array")] #[sampler(102)] pub albedo: Handle<Image>,
        #[texture(103, dimension = "2d_array")] #[sampler(104)] pub normals: Handle<Image>,
    }

TerrainParams derives ShaderType and mirrors the WGSL struct field for field, in the same order (vec3 followed by f32 so std140 packs without padding fields: planet_center + def_radius, tex_cam_frac + relief_scale, sun_dir + wetness, bc_rock + pool_cover, then the vec4s, then four f32). TerrainKey { hash_float: bool, census: Option<CensusMode> } is the bind group data and drives shader defs in MaterialExtension::specialize. The base StandardMaterial is: base_color WHITE, perceptual_roughness 1.0, metallic 0.0, reflectance 0.0, opaque_render_method Forward, unlit false. One material asset serves all patches; per-frame uniform writes touch that one asset.

Winding: the reference uses BackSide with its own winding. The patch builder owns winding, so the material sets cull_mode from the patch winding (open question 3).

## How patches supply the inputs

- Per vertex (patch worker thread, never the main thread): height h from the same compose height that places the vertex, minus nothing (skirts move only the position, as in the reference where vH stays unskirted); climate (temp, humid) from the HPF zw channels at the vertex direction. Stored as UV_0 = (temp, humid) and UV_1 = (h, 0).
- Per fragment: slope from the interpolated mesh normal (the reference uses the finite-difference vertex normal, the same normal the patch already carries), dir0 from planet_center, rel from view.world_position, everything else from the noise in the shader.
- Per frame uniform: planet_center = FloatingOrigin::to_local(DVec3::ZERO) (f32 after the f64 subtraction), tex_cam_frac = camera planet position wrapped per axis to a period of texTileM * 8 computed in f64 (reference texCamFracOf, tile.wrap), sun_dir from the sun light, def_radius = Planet radius, relief_scale = radius / 63_600_000 (planet.rs RELIEF_DESIGN_RADIUS_M).
- Planet frame equals Bevy world frame: the triplanar axes and the sign(n) normal assembly are in planet axes; the planet entity must have identity rotation. FACE_FRAME stays inside the patch builder.

## Climate (a gap the reference needs and the port lacks)

The reference HPF texture has four channels (seaBias, elevAmp, temp, humidity; bakeHpfTexels in tsl/ops-js.js:1-15) and the fragment reads zw. src/height/anchor.rs only builds the seaBias channel (sea_bias_faces). The temp and humidity formulas are the 'regional' band params in anchor-field-bands.js:105-135 (latitude cosine, two 2D fractals tNoise and hNoise, inland from the band fractal, equatorial wet bulge, subtropical dry trough, temperate recovery, rain shadow from belt), combined through anchor-field.js sampleUV. Stage 2 ports that into src/height/climate.rs with a fixture from createAnchorField(seed).sampleUV for the HPF grid. Until it lands the material takes a constant temp 0.5, humid 0.6 and the climate census rows stay open.

## Texture array packing and loading

Copy plan: scripts/copy-planet-textures.mjs copies from /workspace/spoint/packages/mapspinner/textures to /workspace/bevy/assets/textures/planet/: grass-color, grass-displacement, grass-normals, ground-normals, rock-color, rock-displacement, sand-color, sand-displacement, sand-normals, snow-color, snow-displacement, snow-normals (all .jpg, 1.6 MB total). There is no rock-normals file: layer 1 (rock) takes ground-normals.jpg (surface-texture-decode.js:48, mat:nf4).

Packing (layers.rs, mirrors decodeSurfaceTextureSet): layer order grass, rock, sand, snow. For each layer: decode colour, displacement and normals at 1024x1024; deshade the colour (32x32 grid mean, bilinear blur, gain = clamp((mean/blur)^0.8, 0.4, 2.5) in linear light using the 2.2 table, contrast stretch 1.35 around the mean, re-encode with delin8); albedo array RGB = colour, A = displacement R; normal array R,G = normal map R,G, B = displacement R, A = 255. Then rock_mean = linear mean of the rock layer RGB (becomes bc_rock) and mean_l = per-layer mean luminance with the 0.2126/0.7152/0.0722 weights.

Image construction: Image::new with extent 1024 x 1024 x 4, TextureDimension::D2, albedo Rgba8UnormSrgb, normals Rgba8Unorm, texture_descriptor.mip_level_count = 11, data layer-major with each layer's full mip chain (box filter; albedo averaged in linear light), texture_view_descriptor dimension D2Array, sampler Descriptor with address Repeat, mag Linear, min Linear, mipmap Linear, anisotropy_clamp 8. Until decode finishes (background task, not the main thread) the extension holds 1x1 placeholders (grey 128 / flat normal 128,128,255,255, 4 layers) and surf_ready = 0, which makes the shader return the biome colour alone; decode finishing swaps the handles and sets surf_ready = 1.

## CPU mirror of the splat rule (pure Rust)

src/planet_material/splat.rs:

    pub struct LayerWeights { pub grass: f32, pub rock: f32, pub sand: f32, pub snow: f32 }
    pub fn splat_weights(snoise3: &dyn Fn(V3) -> f64, dir: V3, height_m: f64, slope: f64, humid: f64, temp: f64) -> LayerWeights
    pub fn top_layer(w: LayerWeights) -> Layer

It is the literal port of createSplatWeights in splat-weights.js (same constants, same order, bandWarp from snoise3 at dir * 1100 and * 2580 times 20*0.25, then dryHot, beach, sandRegion, wRockSlope, snowHi, rockBand, wRock, wSnow, wSand, wGrass, uwM, the underwater move, normalise by sum + 1e-4). snoise3 is HeightSpec::snoise3 in f32 precision for the same hash as the planet, so no new noise code exists. top_layer is the reference's top-2 first element (grass default, strict greater wins, order rock, sand, snow). The splat-weights.js file is itself the reference's CPU mirror of the GPU rule, so the fixture compares to the reference's own code, not to our reading of it. Where the GPU rule differs from that file the Rust mirror follows the GPU rule: rockSlope in place of slope, which are equal because slope is already in [0,1].

Fixture: scripts/gen-splat-weights.mjs writes rows (dir, h, slope, humid, temp, grass, rock, sand, snow) for a grid that covers every branch (below sea, beach top, band edges, snow edges, steep and flat, dry-hot, humid) for hash versions 1 and 2. Rust test uses testkit close with tolerance 1e-5 (f32) and a label per row.

## Layer census witness at a pose

For each pose in docs/planet-parity.md (orbit, high altitude, low altitude, forest, shoreline):
1. Camera ray grid 64 x 36 over the capture field of view; each ray is marched to the Planet::height surface (sphere trace on the f64 compose height, same function the patches use).
2. At each hit: h, normal from the finite difference the reference uses (planet frame), slope = 1 - max(0, dot(n, dir)), (temp, humid) from climate.rs, then splat_weights and top_layer. Misses are counted as sky.
3. Census = counts per layer [grass, rock, sand, snow] and sky. The same hit points go to the reference fixture (scripts/gen-splat-weights.mjs run in the pose mode) and the witness asserts exact count equality: that proves the rule at the pose.
4. GPU check: the capture is repeated with LAYER_CENSUS on an Rgba8Unorm target; red channel x 255 is the layer id of each pixel. The share of pixels whose id equals the CPU id of the same ray must be at least 0.97 at the pose, and each layer share must be within 0.03 of the CPU share. The differences come from patch LOD against the analytic surface; the threshold is recorded as mea:cap.census in the table after the first measured run.

## Offscreen capture plan

- Headless app: DefaultPlugins with WindowPlugin { primary_window: None, ..default() }, WGPU_BACKEND=vulkan and VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json (software Vulkan, no display), ScheduleRunnerPlugin.
- Camera3d with RenderTarget::Image(handle) on a 640 x 360 Image with TextureUsages RENDER_ATTACHMENT | COPY_SRC | TEXTURE_BINDING, Rgba8UnormSrgb for beauty and Rgba8Unorm for census, Tonemapping::None, the pose from the stage 1 pose record.
- Wait gate: frames run until the stream reports no pending patches and TerrainExtension.params.surf_ready is 1, then spawn Screenshot::image(handle), observe the captured event to take the pixels, write target/witness/planet-<pose>.png, send AppExit.
- Reference captures: scripts/capture-reference-poses.mjs drives the reference with playwriter on the display that has the GPU (the host here has none), same pose, same 640 x 360, same sun, saved under testdata/planet-parity/ref-<pose>.png.
- Measure mea:cap.rms: both images box-downsampled to 64 x 36, mean absolute RGB difference in [0,1]. Threshold set from the noise floor: two reference captures of the same pose a frame apart give the floor and the threshold is floor + 0.05, recorded in the table with the floor. Second measure mea:cap.hist: L1 distance between 16-bin per-channel histograms, threshold recorded the same way. These are the proposal; the numbers come from the first measured run.

## Red checks (each must be seen failing, then the fix restored)

1. One layer removed: CENSUS_REMOVE_LAYER = n for each n at the pose where layer n holds more than 2% of the CPU census; the GPU census agreement must drop under 0.97. Also the CPU mirror with that weight forced to zero must fail the fixture.
2. Wrong hash: HASH_FLOAT set on an Integer planet; fixture and census agreement must fail.
3. Placeholder textures (surf_ready 0): beauty capture at the low altitude pose must exceed mea:cap.rms.
4. Shadows off: DirectionalLight.shadows_enabled = false with the sun low at the shoreline pose; the capture luminance difference in the shadowed region must exceed its recorded threshold (the check proves the Bevy cascades act on the terrain).
5. Atmosphere off: belongs to the atmosphere module (it replaces aerial_perspective); listed here because the same capture harness runs it.
6. Mutated constant in splat.rs (for example snow edge 120 to 140) must fail the fixture.

## Rows each part closes

- terrain_material.wesl albedo and band code: mat:col.deepsea (not used by the terrain pass, see not ported), col.sea (not used), col.shore, col.lowland, col.grass, col.rock, col.snow, band.edges.lo, band.edges.hi, band.edges.snow, band.warp, band.warp.splat, slope.rock, slope.def, slope.rockwiden, sea.floor.depth, sea.floor.silt, sea.floor.basalt, sea.floor.rock, sea.ice.range, sea.ice.colour, beach.top, beach.width, beach.mix, var.amt, ridge.octaves.
- splat_weights (shader and splat.rs): splat.layers, splat.dryhot.arid, splat.dryhot.temp, splat.beach.slope, splat.sand.region, splat.rock.slope, splat.rock.band, splat.snow.hi, splat.snow.suppress, splat.sand.weight, splat.grass.weight, splat.underwater, splat.norm, splat.top2, splat.xsoft, splat.xfinger, splat.ordpush, splat.ord, splat.xfade, splat.mix, splat.matcolor.
- triplanar and tiling: tri.sharp, tri.planes, tri.fine, tri.coarse, tri.nrm, tile.m, tile.wrap, tile.warp.freq, tile.warp.amp, tile.warp.mix, tex.far, tex.oct.far, tex.oct.coincide, tex.nrm.fade, tex.nrm.k, tex.nrm.low, tex.sat, tex.bright, tex.biometint, tex.luma, tex.detail, tex.albedo.mix, tex.photo (declared, unused), tex.meanl, tex.rockmean.
- layers.rs and the arrays: tex.files, tex.size, tex.count, tex.normalfiles, tex.pack.alb, tex.pack.nrm, tex.srgb, tex.filter, tex.placeholder, tex.gamma, tex.deshade.exp, tex.deshade.min, tex.deshade.max, tex.deshade.grid, tex.contrast, tex.rockmean, tex.meanl, tex.load.gate (surf_ready).
- pool: pool.disp.lo, pool.disp.hi, pool.cover, pool.spec.rough, pool.spec.sharp, pool.slope, pool.wet, pool.wetdark, pool.fresnel, pool.speclobe (the last as the roughness mapping).
- normals and pbr: norm.up, norm.relief, norm.flat, norm.lit, pbr.metalness, pbr.roughness, pbr.basecolor, pbr.side (cull_mode), pbr.fog (no DistanceFog on the camera).
- climate: climate.source, climate.hpfres, climate.hpfseed (the Rust planet seed is 0x5eed1234 in globe.rs and a fixture on the reference default 1337 is the comparison, the seed row stays as a recorded difference), climate.params (via climate.rs).
- grade in the shader: light.exposure, light.skyfill (SKY_FILL in the haze term), light.hazemul, light.nightfloor, light.termwidth, light.termglow, light.nightlights, light.looksat, light.lookcontrast, light.nightfill, light.termtint, light.apgate, light.termfade, light.haze, light.graze, light.aces, light.luma, light.gamma, light.vertexao and light.aoamt (0, nothing to port), light.biomewarp (debug only).
- Rust side lights (frame_params and the sun setup, by the light and atmosphere pieces): light.sungain, light.skyscale, light.skygrey, light.skytint, light.floor.albedo, light.floor.add, light.ambirr, light.raytint, light.skyday, light.sundark.
- census and capture: the witness rows of Stage 2 (the mea:* measure rows added for the census and capture thresholds).

## Not ported, and why

- The reference TSL LightingModel (sky irradiance from the atmosphere transmittance LUT, SUN_IRRADIANCE_GAIN, ambient floor, cascade-light special case SHADOW_CASCADE_NAME_PREFIX and the sunFlag per-light uniform): three.js specific. Bevy PBR replaces it, with the constants mapped to the DirectionalLight and AmbientLight values on the Rust side; the cascade-name rule has no Bevy meaning. Exact radiance match is therefore measured by the capture comparison, not by identity of formula.
- Aerial perspective march (sky.marchRadiance), and the sky node: the atmosphere module. The shader keeps the exact gate and mix and a neutral hook.
- mat:sea.depthm, sea.ocean.deep, sea.ocean.shallow, sea.ocean.k, uw.sun, uw.sky, uw.absorb, uw.fog, uw.fogrange, uw.upfill: water surface and underwater pass, not the terrain pass (the reference notes sea.depthm is unused in the files read). col.deepsea and col.sea likewise belong to the water shader.
- mat:glsl.terrainwidthk, glsl.texlodbias, glsl.debug.biome, light.glsl.skyao, light.glsl.limb: the legacy GLSL path and debug display, which the TSL path does not use.
- fsCheap debug output and every window.__ lever (texMix, texWarp, wetness, exposure and so on): browser tuning hooks. Their defaults are compiled in as consts; wetness stays a uniform at its default 0.
- mat:tex.synth, tex.synth.mid, tex.synth.clamp: the normal synthesised from displacement when a normals file is missing. All four normals files exist, so the branch never runs; not ported until a layer lacks its file.
- mat:tex.decode.timeout: the browser async decode timeout; the Rust decode is a background task with surf_ready as the gate.
- displayReferredToSceneLinear for ACES, AgX and Neutral modes: only the tone mapping none case is reproduced (sRGB decode of the display value); the camera must run Tonemapping::None.
- Per-vertex warp vWarp: computed per fragment here (3 extra noise evaluations) instead of interpolated, because there is no custom vertex shader. Cost is measured in stage 4; the reference is per vertex.
- mat:nf1 to nf5 are not-found rows and stay recorded as such.

## Open questions

1. Is the pool and wetness path needed at all at default wetness 0? It is ported but dead at default; keeping it costs shader time.
2. The Cargo.toml bevy features have png but no jpeg, so decoding the 12 JPEGs needs the jpeg feature or a decoder crate (jpeg-decoder or zune-jpeg) added by the lead.
3. Patch winding and cull mode: which side the Stage 1 patch builder winds, to choose the material cull_mode.
4. Hash version at the planet: the preset uses Integer; HASH_FLOAT exists only for the Float-hash world option in globe.rs.
5. Sun colour and illuminance: the reference sun irradiance comes from solarIrradiance times the atmosphere transmittance at the surface, and Bevy exposure scales DirectionalLight illuminance. The light piece must set illuminance times exposure equal to 1.25 times the reference sun luminance for the capture comparison to converge; this needs the atmosphere module's numbers.
6. The climate port (src/height/climate.rs) is a new ported field with its own fixture; confirm it belongs to stage 2 and not to a separate height stage.
7. Exact MaterialExtension::specialize and bind_group_data signatures in 0.20.1 were not compiled (no compile allowed during this design); the implementer must check them against bevy_pbr-0.20.1/src/extended_material.rs and examples/shader/extended_material.rs.
8. Whether the reference can be captured at a pose from this host: the capture script needs the GPU display; thresholds are proposals until two reference runs give the noise floor.
