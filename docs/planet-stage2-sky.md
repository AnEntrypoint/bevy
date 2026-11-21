# Planet parity stage 2: water, atmosphere, sky, light and shadows (Phase A design)

Scope: the water surface, the atmosphere and sky, lighting and shadows of /workspace/bevy/docs/planet-parity.md rows wsl:WSL-001 to WSL-297 and wsl:nf1 to nf6. The terrain material is another design. This document and the shader sources next to it are design only: no Rust file, Cargo.toml or main.rs was touched, and the shaders are not compiled by the crate yet.

Sources read in full: tsl/water-tsl.js, tsl/sea-waves-tsl.js, sea-waves.js, webgpu/water-render.js, shaders/atmosphere.glsl, atmosphere-transmittance-lut.js, atmosphere-scattering-lut.js, atmosphere-lut-job.js, webgpu/atmosphere-lut-compute.js, webgpu/sky-render.js, tsl/sky-tsl.js, tsl/terrain-lighting-tsl.js, tsl/terrain-ambient-shared.js, tsl/display-referred-tsl.js (all under /workspace/spoint/packages/mapspinner/src), and client/core CascadeShadowSelect.js, CascadeShadowSelectTSL.js, ShadowPipeline.js, TimeOfDay.js, UnderwaterTint.js, SceneSetup.js (shadow and light lines), QualityPresets.js, BootFailureOverlay.js, src/shared/water.js, src/terrain/waterLevel.js. Bevy 0.20.1 sources read: bevy_pbr atmosphere (mod.rs, node.rs, transmittance_lut.wesl, render_sky.wesl, functions.wesl), bevy_light atmosphere.rs and cascade.rs, bevy_pbr render/shadows.wesl and mesh_view_bindings.wesl, bevy_core_pipeline tonemapping.wesl, bevy_shader shader.rs.

## 1. Findings that drive the design

1. The shipping reference renderer is three WebGPU with TSL (planet-tsl.js builds createSkyTSL, createWaterTSL, createLegacyTerrainLighting). webgpu/sky-render.js and webgpu/water-render.js are a second, standalone path. Evidence that the TSL path is the authority: water-render.js draws a flat tinted translucent surface (mix 0.55 tint, alpha 0.75, ripple 0.006, rows WSL-172 to 176), nothing like water-tsl.js; and sky-render.js carries ATM_RHO_MAX = 6403.774997918573 (WSL-141) while atmosphere.glsl and sky-tsl.js derive sqrt(6500^2 - 6360^2) = 1341.7898494175606. The first wrong constant I fed my own WGSL bake (I copied 6403.77 from WSL-141) made the GPU bake disagree with the reference LUT by up to 0.99 absolute; with 1341.79 it matches (section 6). So the legacy path cannot even reproduce the reference's own LUT. Its rows (WSL-140 to 151, 172 to 179) close as "reference-only variant, not ported", and WSL-141 is recorded as a defect of that variant.
2. The reference bakes both lookup tables on the CPU in JS (atmosphere-lut-job.js, a worker with a sync fallback). webgpu/atmosphere-lut-compute.js is misnamed: it only creates textures, uploads and reads back. There is no reference GPU compute to port; the port's CPU bake is the literal port, and the WGSL compute I wrote is an optional second path.
3. The reference LUT code runs under node unchanged: atmosphere-transmittance-lut.js and atmosphere-scattering-lut.js are pure ESM, sea-waves.js too. Measured here: transmittance 256x128x500 steps 1092 ms, scattering 64x32x24x64 steps 492 ms. So the witness generator imports the real functions.
4. The TSL node graphs cannot be evaluated in node (the reference's own scripts/tsl-tone-tint-aces-parity-witness.mjs states: no GPU and no node builder). So the water fragment, the sky fragment and the display-referred functions have no node witness; their witnesses are the CPU twins that exist (sea-waves.js octave and mean, the LUT modules) plus offscreen captures of the reference (section 9).
5. three's TSL tone mapping differs from Bevy's AcesFitted in two ways, read from /workspace/spoint/node_modules/three (0.185.1) and bevy_core_pipeline-0.20.1/src/tonemapping.wesl. (a) RRTAndODTFit: three has b = v * ((v + 0.4329510) * 0.983729) + 0.238081; Bevy has b = v * (0.983729 * v + 0.4329510) + 0.238081. (b) three builds the matrices with mat3(9 floats), which WGSL reads column-major, then applies M * c; Bevy applies c * M with the same constructor, which is the transpose. Both also differ in exposure: three multiplies by exposure / 0.6, Bevy by 2^ev. Consequence: Bevy's tonemapper is not the reference tonemapper. The reference feeds three's tonemapper with scene-linear values obtained by inverting that same tonemapper (display-referred trick, display-referred-tsl.js). To keep that trick exact the port ships its own tone map pass (atmosphere_tonemap.wgsl) with three's formulas, and the camera uses Tonemapping::None. Whether three's matrices really act transposed is the one claim I could not run; the port is a literal port of the emitted semantics, and the capture witness of section 9 (swatch card) settles it.
6. Bevy's native atmosphere (bevy_pbr atmosphere, bevy_light atmosphere.rs) compared with the reference:

| quantity | reference | Bevy 0.20.1 native | reproduced |
|---|---|---|---|
| bottom, top radius | 6360, 6500 km units (scaled to the planet) | inner_radius, outer_radius per entity, any units | yes, by setting radii to R and R*6500/6360 |
| Rayleigh coefficients | 0.005802, 0.013558, 0.0331 per km | 5.802e-6, 13.558e-6, 33.1e-6 per m (earth()) | yes, scaled by 6360/R |
| Rayleigh scale height | 18 of 140 | Exponential scale = height / atmosphere height | yes, scale 18/140 |
| Mie | scattering 0.003996, extinction 0.003996/0.9, scale height 4 | earth() has absorption 3.996e-6 and scattering 0.444e-6, scale 1.2/60 | no: the two values are swapped relative to the reference (reference scatters 0.003996 and absorbs 0.000444); fixable with a custom ScatteringMedium |
| Mie phase | Cornette-Shanks, K(1+nu^2)/(1+g^2-2g nu)^1.5, K = 3/(8pi)(1-g^2)/(2+g^2), g 0.8 | Henyey-Greenstein, (1/4pi)(1-g^2)/(1+g^2-2g nu)^1.5 | no without a Curve phase; at g 0.8 the two differ by 14 percent forward and 76 percent at nu 0 |
| ozone | none | earth() adds an ozone absorption term | omit the term |
| multiple scattering | none, single scattering only | multiscattering LUT 32x32, 64 dirs, ground_albedo | no: cannot be switched off |
| transmittance LUT | 256x128, 500 steps, (r, mu) by distance-to-top parametrisation | 256x128, 40 steps | size yes, steps no |
| scattering table | 64x32x24, 64 steps, rgb Rayleigh + scalar Mie, parametrised (r, mu, muS) | sky-view 400x200, aerial view 32x32x32 | no |
| sun | solar irradiance (1.474, 1.8504, 1.91198), disc 0.004675 rad x6, per-pass tint and exposure 14 to 48, saturation 1.3, limb halo, 100 km fade | DirectionalLight illuminance in lux with camera exposure | no |
| aerial perspective | march from the scattering LUT, gate 3 to 120 km, floor tint | aerial-view LUT froxels | no |
| output | display-referred, then inverse ACES | scene-linear HDR, Bevy tonemapper | no |

Decision: the reference wins everywhere they differ, and they differ in the quantities that set the picture (multiple scattering, phase, sun and exposure chain, aerial perspective, output encoding). So the port does not add Bevy's Atmosphere component and does not use AtmospherePlugin's passes. Bevy supplies what is not atmosphere: DirectionalLight and cascades, the shadow maps, the transmission copy used by water refraction, MSAA and the render graph. The medium parameters in the table are kept as the named constants of the shaders so a later switch to the native sky stays possible if its results are ever measured to agree.

## 2. Deliverables and how they are assembled

All shader sources are plain WGSL. Bevy 0.20 loads a .wgsl asset through Shader::from_wgsl with no import or define support (bevy_shader shader.rs: from_wgsl sets imports to empty; shader defs are ignored for non-wesl files), so the Rust side builds each pipeline's source by concatenating the files in the order below with include_str! and replacing the LUT binding tokens, then registers it with Shader::from_wgsl. A shared helper in shaders.rs does the concatenation and is the only place that knows the order.

| file | job | concatenated with |
|---|---|---|
| assets/shaders/planet/atmosphere_core.wgsl | constants named after the reference, phase functions, ray-sphere distances, the (r, mu) and muS parametrisations, densities | none |
| assets/shaders/planet/atmosphere_lut_sample.wgsl | LUT bindings, transmittance and scattering lookups, transmittance to sun, march from the scattering LUT, 8-step sky march | core |
| assets/shaders/planet/atmosphere_display.wgsl | three's ACES, sRGB OETF, display-referred to scene-linear and back | none |
| assets/shaders/planet/atmosphere_sky.wgsl | sky pass: fullscreen triangle, sky-tsl.js fragment | core, lut_sample, display |
| assets/shaders/planet/atmosphere_lighting.wgsl | terrain-lighting-tsl.js as functions: sun and sky irradiance, ambient floor, aerial perspective, terminator, grade | core, lut_sample, display |
| assets/shaders/planet/atmosphere_lut_bake.wgsl | optional GPU bake of both tables and half-float packing | core |
| assets/shaders/planet/atmosphere_tonemap.wgsl | post pass applying three's ACES | display |
| assets/shaders/planet/water.wgsl | water vertex and fragment, sea waves | display |

Tokens in lut_sample: ATM_LUT_GROUP, ATM_LUT_BINDING_TRANSMITTANCE, ATM_LUT_BINDING_SCATTERING, ATM_LUT_BINDING_SAMPLER (the sky pass uses group 0, bindings 1, 2, 3). water.wgsl hard-codes group 3 (bevy_pbr MATERIAL_BIND_GROUP_INDEX = 3, material.rs:75) and, in group 0, the view uniform prefix (clip_from_world is the first field of bevy_render's View) and the transmission texture at bindings 24 and 25 (mesh_view_bindings.wesl:84).

Validation done in Phase A: a scratch crate (/workspace/tmp/bevy/stage2/wgsl-check, naga 30.0.2, the version Bevy 0.20 resolves) parses and validates each concatenation: sky 28 functions 2 entry points, lighting 36 functions, bake 15 functions 4 entry points, tonemap 9 functions 2 entry points, water 15 functions 2 entry points, all ok. The scratch crate is outside the repository.

Constants are named after the reference constants (ATM_BOTTOM, ATM_TOP, ATM_RAYLEIGH_H, ATM_MIE_H, ATM_MIE_G, ATM_RAYLEIGH, ATM_SOLAR_IRRADIANCE, SKY_TINT, SKY_EXPOSURE_HIGH_SUN, SKY_EXPOSURE_LOW_SUN, SKY_SATURATION, SEA_FREQ, SEA_SPEED, SEA_AMP_M, SEA_OCTAVE_ANGLE, SEA_FREQ_LF, MARCH_NEAR_M, LAND_ABOVE_SEA_DISCARD_M and so on). The shaders carry no comments; this document is where the reasons live.

## 3. Water

Water is its own surface (spec architecture 4): grid 4, no skirt, sea level 0 (WSL-001, 002, 155), levels clamped to 11 (WSL-003), key span 4096 (WSL-004).

### Geometry: one mesh, built on the async pool, relative to a camera-near origin
The reference draws an instanced 4x4 grid and places vertices in f32 at the camera-relative position (d0 - camDir) * R. At R = 6,360,000 an f32 unit vector has 6e-8 absolute error, which is 0.38 m of vertex position: visible wobble and cracks between neighbours. The port therefore computes vertex positions in f64 on the CPU and ships them relative to the mesh origin. Per the spec, patches are built on the async compute pool, never on the main thread.

- water_set.rs (pure): takes the quadtree leaves from stage 1 (select_leaves), clamps level above 11 to the level-11 ancestor (ox, oy floored to A = l * 2^(lv-11)), dedupes with the reference key ((face * 4096 + round(oy/l) + 2048) * 4096 + round(ox/l) + 2048) * 12 + lv (WSL-003, 004), then applies the land gate: an instance is dropped when all 25 lattice points are above 1.0 m (WSL-016, 017, 018). Probe order is the reference's (centre, four corners, four edge midpoints, then the rest, landProbeOrder), keys quantised to 1.0 m, memo cleared at 100000 points. Heights come from Planet::height (the stage 1 height field).
- water_mesh.rs (pure): for each kept instance, 25 vertices from buildGridGeometry(4, no skirt): px, py = (x-1)/4 for x in 1..=5, index pattern with the hash-flipped diagonal (the hash h = (x | y << 16), two rounds of imul 0x45d9f3b, bit 17). Vertex dir = normalize(U*fl.x + V*fl.y + C*R) with fl = R * tan((o + g*l)/R * pi/4) (WSL-005), position = dir * R (sea radius exactly R, WSL-156), minus the mesh origin, in f64 then cast to f32. Attributes: position, normal = dir (f32), and a custom Float32 attribute land_height = Planet::height(dir) per vertex (the reference evaluates composeHeight in the vertex shader; the gate already evaluates the same 25 points on the CPU, so the port reads them once and ships them, which removes a GPU height dependency from the water shader; documented deviation).
- The mesh origin is the camera at build time. A rebuild is triggered by the same events that change the leaf set. Positions of near water have f32 error below 1e-4 m while the camera stays within the rebuild distance.

### Material
water_material.rs defines WaterMaterial: Material (custom vertex and fragment shader, AsBindGroup uniform at group 3 binding 0 matching the Water struct in water.wgsl).
- Phase: alpha mode Opaque with reads_view_transmission_texture() returning true. In bevy_pbr 0.20.1 that puts the draw in the Transmissive3d phase (material.rs:1744 to 1750), after the opaque pass, with depth writes on and the colour alpha 1.0. This is the equivalent of three's transparent = true, depthWrite = true: the reference's colour alpha is 1.0 and "transparent" only orders the draw after the opaque scene so viewportSharedTexture reads the scene (WSL-050).
- specialize(): cull_mode None (WSL-051 DoubleSide), vertex layout [POSITION at 0, NORMAL at 1, land_height at 2], no prepass, no shadows (enable_shadows false, enable_prepass false, NotShadowCaster; WSL-052).
- Transmission texture: Bevy copies the main colour before the transmissive phase; screen_space_specular_transmission_steps stays at its default 1 (water refracts the opaque scene only, like the reference). The sky pass must run before the copy (section 4).
- Per frame (water_frame.rs, a main-world system, no allocation): origin_in_render (mesh origin minus render origin, f64 to f32), camera_in_render, time, sun_dir, cam_mod (camera planet position wrapped by 1024 per component, WSL-011), cam_dir, cam_alt, planet_radius, amp, choppy, foam, sea_mean, haze_mul (WSL-053), underwater (camDist < R - 2.0, WSL-055), viewport, display exposure and mode.
- seaMean (WSL-054, 056 to 065): sea_waves.rs ports sea-waves.js (octave, noise, hash with the CPU multipliers 1597334673 and -482951495, mean over 256x256 at step 3.7013 per octave with phase step 0.31, amplitude gain 0.45, choppy pull 0.3, cache limit 16). It is computed on the async pool, not per frame; the default (choppy 0.5, amp 1.0) is 0.8649493292582504 measured from the real sea-waves.js.

### Assumptions that need a stage 1 answer (open questions in section 12)
The fragment works in the planet frame: east/up/north are not used; ux, uy are built from uz exactly as the reference (cross with (0,1,0), or (1,0,0) when |uz.y| >= 0.99), so the wave pattern phase equals the reference only if the render space axes are the planet axes (translation only, no rotation). If stage 1's render space is rotated, the fix is a 3x3 planet_from_render in the uniform and in the sky frame; the sky frame already carries one.

### Wave and shading rows
Every constant and branch of water-tsl.js and sea-waves-tsl.js is in water.wgsl in the same order: march (WSL-008 to 010, 024), fades and octave windows (WSL-025 to 028), HF/LF slopes and their eps (WSL-006, 007, 029), sky reflection and glints (WSL-030 to 032), deep colour, refraction (WSL-033 to 036), absorption and body (WSL-037, 038), fresnel (WSL-039, 040), final mix (WSL-041), foam (WSL-042 to 047), haze (WSL-048, 049), underwater (WSL-019 to 023), ACES display for the water colour (WSL-015). Sampling the transmission texture uses the framebuffer position divided by the viewport (WebGPU screenUV is top-left origin, so the reference's -slope.y is kept literally). The refraction sample goes through scene_linear_to_display_referred as in the reference.

### Not in water.wgsl
- Underwater post tint of host materials (UnderwaterTint.js, WSL-180 to 183): this is a fog-chunk patch of three's StandardMaterial and ShaderMaterial. The Bevy equivalent is a StandardMaterial extension (fragment tint below the sea surface: mix(rgb * (0.30, 0.55, 0.65), (0.04, 0.34, 0.52), clamp((seaY - 2.0 - y) * 0.08, 0, 0.6)), camera within 3.0 m above the sea). It touches vegetation and model materials, so it is scheduled with stage 3 and the rows stay open here.
- src/shared/water.js (WSL-161 to 171, waterFresnel, waterFoamCoverage, waterMedium, waterCausticsAt) belongs to the flat-world water; the planet water does not import it. It is a pure ESM file; if gameplay needs it, a water_medium.rs plus a generator that imports it directly is the witness. Rows stay open.
- waterLevel.js (WSL-157 to 160): liquid state (in water depth > 0, diving when depth > 1.5) is gameplay; a three-line pure function in sea_waves.rs, witnessed by importing createWaterLevel under node.
- window.__waterSurface visibility toggle: not ported; a Rust resource flag does it.

## 4. Atmosphere and sky

### Lookup tables
- Reference tables: transmittance 256x128 RGB, 500 midpoint steps; scattering 64x32x24 RGBA (Rayleigh RGB inscatter without phase, Mie scalar), 64 steps, muS warp tanh with K 1.4 over 24 layers, the transmittance table sampled bilinearly with clamp while baking; both stored RGBA half-float, linear filtering, clamp (WSL-124, 128 to 139).
- lut_cpu.rs: f64 port of both bakes, one function each, same loop order. It runs on the AsyncComputeTaskPool once at start (node: 1.09 s plus 0.49 s; Rust release is comparable or faster), result converted to two Rgba16Float Images by lut_images.rs and inserted as the AtmosphereLuts resource. No main-thread work beyond handing the finished Vec to Assets<Image>.
- atmosphere_lut_bake.wgsl: optional GPU bake for platforms where the CPU bake is too slow (wasm). Entry points bake_transmittance (workgroups 32x16), bake_scattering (8x4x24), pack_transmittance_half, pack_scattering_half. Textures: rgba32float storage for the bake (the scattering bake needs full-precision transmittance, like the reference), then packed to rgba16float for sampling. Each pipeline gets its own bind group layout holding only the bindings its entry point uses, otherwise the same texture is bound as storage and sampled in one dispatch. Measured on llvmpipe (lavapipe, this host): dispatch 79 to 99 ms.

### Sky pass
- atmosphere_sky.wgsl is sky-tsl.js's fragment node. Per pixel: ray from the view-to-planet matrix and the projection diagonal (as the reference's raw WGSL does), camera in atmosphere units (planet position times 6360 / R), if above the top radius move the start onto the top sphere, blend factor w_sky from mu against the tangent with 0.006, one 8-step march to mix(ground distance, top distance, w_sky) using the transmittance table for the sun leg, limb halo (WSL-121, 122), sun disc (cos 0.004675, trans * solar * 6.0), tint (0.82, 0.95, 1.22), exposure mix(48, 14, clamp(dot(sun, up), 0, 1)), ACES curve, saturation 1.3 about luminance, pow 1/2.2, times the altitude fade, then display_referred_to_scene_linear. The scattering table is not used by the sky fragment in the reference (the live sky marches 8 steps), only by aerial perspective on terrain (atm_march_radiance, used in atmosphere_lighting.wgsl).
- Render structure (Bevy 0.20 core 3d is a schedule of systems, see bevy_pbr atmosphere mod.rs: render_sky is added to Core3d .after(main_opaque_pass_3d).before(main_transparent_pass_3d)). planet_sky_pass.rs adds a system render_planet_sky to Core3d ordered .after(main_opaque_pass_3d).before(main_transmissive_pass_3d) so the water refraction copy contains the sky. The pass draws one fullscreen triangle at depth 0.0 (reverse-Z far plane) with a depth attachment loaded and compare GreaterEqual, so geometry in front is untouched. The pipeline is created with PipelineCache and specialised on the view target format and MSAA samples. SkyFrame is a uniform written by a main-world system (planet_sky_frame.rs) and extracted.
- The sky is the scene background in three: drawing first behind everything. Drawing after the opaque pass with the depth test is the equivalent in a deferred-order engine.

### Lighting functions
atmosphere_lighting.wgsl ports terrain-lighting-tsl.js: terrain_sun_irradiance_base (solar times transmittance at ground radius + 0.5), terrain_sky_irradiance and the grey balance, terrain_direct_diffuse (albedo * sunIrr * 1.25 / pi * poolKeep), terrain_indirect_diffuse (sky, day floor 0.14 albedo + (0.020, 0.026, 0.038), night ambient), terrain_grade_output (aerial perspective gate 3 to 120 atmosphere km, apInscat floor, terminator glow, day shade, night fill, exposure 1.05, ACES, saturation 1.25, contrast, display-referred output). The wet-pool specular lobe is left to the terrain material (it needs wetPool inputs). The shadow term: the reference derives shadowK from the shadow-attenuated light colour; in Bevy the terrain extension passes the factor from bevy_pbr::shadows::fetch_directional_shadow into the shadow argument.

## 5. Tone mapping and exposure

- The reference chain: every planet pass computes display colours (ACES-like curve, saturation, gamma 1/2.2) and returns displayReferredToSceneLinear(display); the renderer's ACESFilmic tone mapping and sRGB OETF then reproduce the display colour (WSL-080 to 087, 275 to 277).
- Port: Camera Hdr, Tonemapping::None, DebandDither disabled for witnesses, Exposure set so exposure() = 1 (ev100 = -log2(1.2), bevy_camera camera.rs: exposure() = 2^-ev100 / 1.2), ColorGrading default. atmosphere_tonemap.wgsl runs as a post-process on the view target at the end of Core3d and applies aces_filmic(color, exposure) with the reference exposure (Low tier 0.8, others 1.0, WSL-nf4 closed by reading QualityPresets.js). The sRGB OETF is the swapchain format's (exact piecewise; three uses pow(0.41666): difference below 1e-4).
- The inverse in atmosphere_display.wgsl is a literal port; its matrix constants are the inverses computed with the reference's invertRows3 (ACES_INPUT_INVERSE, ACES_OUTPUT_INVERSE, printed from node and embedded). With the flat mat3x3 constructors used for both forward and inverse the pair is consistent whichever way the matrices act, which is what the reference relies on.
- Planet passes output display_referred_to_scene_linear and the tonemap pass re-applies the forward map, so for every planet pass out(display) = display up to the 0.995 clamp. This is the round-trip witness (section 9).

## 6. Lighting, time of day, shadows

### Time of day (WSL-278 to 289)
time_of_day.rs, pure plus one system. KEYFRAMES at -90, -6, 0, 8, 30, 70, 90 degrees with sun colour, sun intensity, ambient colour and intensity, fill scale exactly as TimeOfDay.js; the three ambient colours with intensity 0.14 pi / luminance(colour) for the first three. Direction: azimuth = frac * 2pi, elevation = sin((frac - 0.25) * 2pi) * (90 - 23 deg), dir = (cos(el) sin(az), sin(el), cos(el) cos(az)); day length 600 s, start fraction 0.3. Interpolation: lerp of intensities, colour lerp in linear RGB between neighbouring keyframes (THREE.Color.lerp on linear components; the hex values are sRGB, so convert with Color::srgb_u8 then lerp linearly, which equals three's linear working space).
- Mapping to Bevy lights for non-terrain objects (vegetation, models, players): DirectionalLight { color, illuminance = keyframe sun intensity } with exposure 1 (three's physically based directional light gives albedo * I * NdotL / pi, Bevy gives albedo / pi * illuminance * NdotL * exposure); AmbientLight / GlobalAmbientLight { color, brightness = keyframe ambient intensity / pi } (three ambient gives albedo * I / pi, Bevy gives albedo * brightness, which is why the reference uses ambIrr = colour * intensity / pi, WSL-229); the fill light is a second DirectionalLight, colour 0x4488ff, 0.4 * fillScale, from (-20, 30, -10), no shadows. These two mappings are the least certain numbers here; Phase B verifies them with a white plane captured at the eight keyframe elevations against the reference.
- Terrain does not use these lights; it uses the sun direction uniform, the transmittance table, the look constants (exposure 1.05, sky fill 0.3, haze 0.4, night floor 0.18, terminator width 0.45, glow 0.6, night lights 0.8, saturation 1.25, contrast 1.0; WSL-190 to 199) and ambIrr from the time-of-day ambient.

### Shadows
- Cascades (WSL-231 to 250): CascadeShadowConfig { bounds: [60.0, 192.0, 614.4][..N], overlap_proportion: 4.0 / 60.0, minimum_distance: 0.1 } built directly, not with the builder (the builder with two cascades would give 60 and 614.4 where the reference gives 60 and 192; with three it gives 60, 192, 614.4 because (614.4/60)^(1/2) = 3.2, the reference's CASCADE_SPLIT). N by tier: Low 1 (and shadows off, QualityPresets), Medium 1, High 2, Ultra 3 (WSL-251 to 253); the parity default is Ultra.
- The 4 m blend is exact for the first boundary only. bevy_pbr shadows.wesl:248 blends over [(1 - overlap) * far, far] of the cascade, one overlap_proportion per light: with 4/60 the three boundaries blend over 4.0, 12.8 and 41.0 m, the reference blends 4 m at each (CASCADE_BLEND_M, WSL-231, 233, 234). A single proportion cannot give 4 m at 60, 192 and 614.4 without forking bevy_pbr's shader, which this port does not do. Recorded as a measured deviation, not closed: rows WSL-231 (partly), 233, 234. Bevy blends linearly in view-space depth, the reference smoothsteps in euclidean camera distance: also recorded.
- DirectionalLightShadowMap { size: 1024 } (WSL-254), shadow_depth_bias and normal bias tuned in Phase B to the reference's -0.0005 and 0.05 (the units differ; the witness is the shadow edge position, not the numbers), ShadowFilteringMethod::Gaussian for the radius-4 PCF (WSL-257, approximate).
- Not ported: far-cascade redraw budget, deferral and lateral snapping (WSL-238 to 242), the caster-only layer, the light offset (40, 80, 30) and shadow camera near/far (WSL-245, 247, 248, 258, 259): Bevy fits and redraws cascades itself every frame. Reason: these exist to save redraw cost in a WebGPU pipeline that cannot cull shadow passes; Bevy's cascades are frustum-fitted per frame with GPU culling. Measured cost on this host is zero GPU, so the check is the frame time on the owner's machine (spec: who owns what).
- Water does not cast or receive shadows (WSL-052).
- Scene defaults for non-planet materials (WSL-262 to 273): GlobalAmbientLight colour 0xfff4d6 brightness 0.5 / pi until time of day takes over; DistanceFog ExponentialSquared density 0.0025 colour 0x87ceeb on StandardMaterial objects only (the planet materials ignore scene fog: nf3); clear colour 0x87ceeb behind the sky pass; the sRGB output (WSL-267) is the swapchain format.

## 7. Rust modules (one job each, directory src/planet_sky/)

| module | job |
|---|---|
| mod.rs | PlanetSkyPlugin: registers the resources, systems and the two render systems |
| constants.rs | named reference numbers with their row ids in the names; the only place numbers are written in Rust |
| shaders.rs | concatenates the include_str! sources, replaces the LUT tokens, registers Shader::from_wgsl assets |
| lut_cpu.rs | f64 CPU mirror of both LUT bakes (section 8) |
| lut_images.rs | f32 to f16 Rgba16Float images for the two tables, sampler linear and clamp |
| lut_task.rs | spawns the bake on the AsyncComputeTaskPool, polls it, inserts AtmosphereLuts |
| lut_gpu.rs | optional compute path over atmosphere_lut_bake.wgsl; off unless a feature flag or the wasm target selects it |
| sky_frame.rs | main-world system: SkyFrame uniform (view_to_planet, proj_diag, fade, exposure, mode, cam_atm, sun_dir) |
| sky_pass.rs | render system and specialised pipeline for atmosphere_sky.wgsl |
| tonemap_pass.rs | render system and pipeline for atmosphere_tonemap.wgsl |
| display.rs | CPU twin of atmosphere_display.wgsl (forward, inverse, OETF), used by the round-trip test and by CPU colour queries |
| sea_waves.rs | CPU port of sea-waves.js plus the GPU functions' f64 twin (sea_octave, sea_height, sea_height_lf), mean cache |
| water_set.rs | leaf set to water instances: level clamp, key dedupe, land gate memo |
| water_mesh.rs | instances to a mesh in f64 relative to an origin, async |
| water_material.rs | WaterMaterial, specialisation, bind group |
| water_frame.rs | per-frame uniform writer (no allocation) |
| time_of_day.rs | keyframes, direction, light application |
| shadows.rs | CascadeShadowConfig by tier, shadow map size, filtering, bias |
| camera_setup.rs | Hdr, Tonemapping::None, Exposure, DebandDither, ColorGrading defaults |
| look.rs | TerrainLook defaults (WSL-190 to 199) and its uniform |

Nothing in water_set, water_mesh, lut_cpu or sea_waves allocates on the main thread: they are pure functions called from task-pool closures. The main-thread systems (sky_frame, water_frame, time_of_day) write fixed-size uniforms only.

## 8. Witnesses from the reference's own code

### Atmosphere tables: scripts/gen-planet-atmosphere-lut.mjs
Imports the real atmosphere-transmittance-lut.js and atmosphere-scattering-lut.js (pure ESM, no imports beyond each other) through scripts/lib/spoint.mjs (spointImport, writeFixture), runs bakeTransmittanceLUT() and bakeScatteringLUT(..., trans), writes testdata/parity/spoint-planet-atmosphere-lut.json: spoint short rev, the constants, per-channel sums over the whole of each table, every 8th texel in x and every 4th in y for the transmittance (1024 texels) and for layers 0, 6, 12, 17, 23 of the scattering table (320 texels), the 24 muSFromLayer values, layerFromMuS samples, and the sizes. Values measured now for the fixture to hold: transmittance texel (0,0) = [0.881333, 0.7628961, 0.5303352], (128,64) = [0.7586164, 0.5244268, 0.2068708], (255,127) = [0.002523662, 5.151489e-06, 8.589479e-13], scattering layer 12 texel (32,16) = [38.79037, 28.6617, 13.51606, 0.009895117], muSFromLayer(0) = -0.9850006834627018. Re-running leaves the fixture byte identical (the generator is deterministic). The Rust test in lut_cpu.rs include_str!s the fixture and compares case by case at relative 1e-9 (both sides f64; V8 and libm exp, tanh and hypot can differ in the last ulp).
### Sea waves: scripts/gen-planet-sea-waves.mjs
sea-waves.js exports only seaHeightMean; octave, noise, hash are module-private. The generator reads the file's text, appends the line export { hash, noise, octave } and imports it from a data: URL, so the reference functions run unchanged (no re-implementation). Fixture: seaHeightMean for choppy in {0.25, 0.5, 0.75, 1.0} and amp in {1.0, 2.0} (measured: (0.5, 1.0) = 0.8649493292582504, (0.25, 1.0) = 0.9277470958529175, (1.0, 2.0) = 1.546618779867089), and 400 octave samples. The Rust test compares sea_waves.rs against both at 1e-12. sea-waves-tsl.js has no CPU twin: seaHeight and seaHeightLF are checked by the WGSL harness below against the f64 twin in sea_waves.rs, which is a line-for-line port reviewed against the TSL text, so the link TSL to port is review, not execution. Said plainly: this is the part of the water the table cannot witness by node.
### GPU shaders against the CPU twins (lavapipe, no X display)
A #[cfg(test)] harness in sea_waves.rs and lut_gpu.rs creates a headless wgpu device, runs a compute shader assembled from the same asset text, reads back, and compares. Measured in Phase A for the bake: with VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json, adapter llvmpipe (LLVM 21.1.8), the WGSL bake against the reference's JS tables: transmittance max relative error 3.6e-4 (absolute 1.5e-5), scattering max relative 4.8e-3 with 84 of 196608 values above 1e-3 (absolute max 0.026 on values up to about 100). The tests use those as tolerances with a margin: 1e-3 and 1e-2. Scratch programs: /workspace/tmp/bevy/stage2/gpu-bake and compare-lut.mjs.

## 9. Offscreen capture plan

- Poses (docs/planet-parity.md, Camera poses): orbit (out-100km), high altitude (out-20km), low altitude (out-5km), ground in forest, shoreline. The sky, water and shadow witnesses add two: sun low (elevation 8 degrees) for long shadows, and underwater (camera 3 m below sea).
- Port: Bevy camera with RenderTarget::Image, 960x540 Rgba8UnormSrgb, headless on lavapipe (VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json), the frame advanced a fixed number of updates with fixed time (Time<Fixed>, water time and time of day set explicitly), read back with the Screenshot component and an observer writing the image; no X display (spec architecture 9). Through the existing ticket gate (scripts/window-job.sh).
- Reference: the same poses, same resolution and time of day, captured through the playwriter setup in gm (invariant: no other browser tooling) from the reference client. If the host browser has no WebGPU, three's WebGPURenderer falls back to its WebGL2 backend, which uses the same TSL graph; the tone map test card below says whether that backend matches.
- Test card: a MeshBasicNodeMaterial plane with a 16x16 grid of known display colours, tone mapped by the reference. It witnesses (a) the transposed-matrix question of section 1 item 5 and (b) the forward and inverse ACES pair. Candidates for the matrix orientation are compared to the card; the port keeps the one with smaller error.
- Measures recorded in docs/planet-parity.md per pose: mean absolute difference of the display-referred image over sky pixels (sky region masked by depth), over water pixels, and over lit terrain pixels; threshold proposal 4/255 mean, 24/255 p99 for sky and water (f32 shader differences, half-float tables), recorded after the first measurement rather than assumed; shadow witness: the luminance ratio of a known shadowed pixel set to the same unshadowed set is within 10 percent of the reference's.
- CPU-only censuses that do not need a render: water instance count and dropped count per pose against the reference (landGateStats: lookups, evaluations, dropped); LUT checksums; time-of-day outputs at the eight keyframe elevations (WSL-282 to 288) against fixture values from a generator that imports TimeOfDay.js with a stub THREE (the file imports only THREE.Color and MathUtils, so a node import with three from the spoint tree runs it unchanged).

## 10. Red checks

Rust-level, each a #[test] that must fail when the feature is removed:
- Atmosphere off: PlanetSkyConfig.enabled = false; the sky census (fraction of sky-masked pixels whose luminance exceeds the clear colour) at the high-altitude pose must be at least the reference's within threshold; with the pass off it is 0 and the test fails.
- Shadows off: DirectionalLight shadows_enabled false (or N cascades 0); the shadow ratio witness at the sun-low pose fails.
- Water off: the water pixel census at the shoreline pose fails.
Shader mutants (each is applied to the named file, the named test must go red, then restored byte-identical):

| name | file | old | new | test |
|---|---|---|---|---|
| top radius | atmosphere_core.wgsl | const ATM_TOP: f32 = 6500.0; | const ATM_TOP: f32 = 6400.0; | GPU bake against fixture |
| Mie g | atmosphere_core.wgsl | const ATM_MIE_G2: f32 = 0.64; | const ATM_MIE_G2: f32 = 0.5; | GPU bake and sky capture |
| rho max | atmosphere_core.wgsl | 1341.7898494175606 | 6403.774997918573 | GPU bake against fixture (the legacy defect) |
| sea freq | water.wgsl | const SEA_FREQ: f32 = 0.2; | const SEA_FREQ: f32 = 0.3; | sea height harness |
| discard | water.wgsl | in.land_height > LAND_ABOVE_SEA_DISCARD_M | in.land_height < LAND_ABOVE_SEA_DISCARD_M | shoreline water census |
| inverse constant | atmosphere_display.wgsl | z * (0.983729 * 0.4329510) | z * (0.983729 * 0.4329500) | display round trip |
| sky exposure | atmosphere_sky.wgsl | const SKY_EXPOSURE_HIGH_SUN: f32 = 14.0; | const SKY_EXPOSURE_HIGH_SUN: f32 = 10.0; | sky capture mean |
| sky irradiance | atmosphere_lighting.wgsl | const TERRAIN_SKY_IRRADIANCE_SCALE: f32 = 0.075; | const TERRAIN_SKY_IRRADIANCE_SCALE: f32 = 0.15; | terrain shadow-side census |
| cascade split | shadows.rs | 614.4 | 300.0 | cascade config test |

## 11. Rows each part closes

| part | rows |
|---|---|
| water_set, water_mesh | WSL-001 to 005, 016 to 018, 155 |
| water.wgsl, water_material, water_frame | WSL-006 to 015, 019 to 053, 055, 066 to 079 |
| sea_waves.rs | WSL-054, 056 to 065 (node witness), 066 to 079 (twin and harness) |
| atmosphere_display.wgsl, tonemap_pass, camera_setup | WSL-080 to 087, 275 to 277, nf4 (values: Low 0.8, others 1.0) |
| atmosphere_core, lut_sample, lut_cpu, lut_images | WSL-088 to 105, 124 to 139, 290 to 297 |
| atmosphere_lut_bake.wgsl, lut_gpu | WSL-297 (sampler policy: linear, clamp; the port samples half-float, which is filterable everywhere) |
| atmosphere_sky, sky_frame, sky_pass | WSL-106, 111 to 123 |
| atmosphere_lighting, look | WSL-107 to 110, 127, 190 to 230 |
| time_of_day | WSL-278 to 289 |
| shadows | WSL-231, 232, 235 to 237, 243, 244, 246, 249 to 257, 260, 261 (partly: see section 6), nf5 |
| scene defaults | WSL-262 to 274 |
| not ported, rows closed as such with the reason | WSL-140 to 151 and 172 to 179 (legacy WGSL path, WSL-141 is its defect), 238 to 242, 245, 247, 248, 258, 259 (WebGPU shadow redraw machinery) |
| open, not in this phase | WSL-156 (legacy), 157 to 160 (gameplay liquid state: small, witnessable under node), 161 to 171 (flat-world water.js), 180 to 189 (UnderwaterTint, terrain ocean colours: stage 3 and terrain material), 233 and 234 (blend widths differ), nf1 to nf3, nf6 |

## 12. What could not be ported, and open questions

Could not port or confirm:
- The exact 4 m cascade blend at the second and third boundary (Bevy exposes one overlap proportion).
- Three's ACES matrix orientation by execution: node cannot evaluate TSL. The port copies three's emitted form; the swatch card decides.
- The sea-waves-tsl.js composition (seaHeight, seaHeightLF) has no node-runnable twin; the f64 twin is reviewed, not witnessed against the reference.
- The refraction texture: Bevy's transmission copy resolution and exposure handling for the HDR main target have not been measured against viewportSharedTexture; Phase B compares a refraction-only crop.
- Far-cascade redraw budgeting, the shadow caster layer and window.__ debug knobs.
- No GPU on this host: only lavapipe. GPU cost is for the owner's machine.

Open questions for the lead:
1. Render space: is the stage 1 render space the planet frame with a translation only (no rotation), with Y as the pole axis? The water fragment and sky use the reference's planet-frame constructions (cross with (0,1,0)); a rotated render space needs the planet_from_render matrix, which the sky frame already has and the water uniform would gain.
2. Who owns Planet::height as a GPU function? The water mesh ships CPU heights as an attribute (deviation); if the terrain material exposes a WGSL compose_height anyway, the water can use it per vertex like the reference. Preferred answer: keep the attribute.
3. Is the transmissive-phase water acceptable to the terrain agent (terrain stays in the opaque phase, so the copy contains terrain and the sky)? If terrain is ever transparent, refraction shows holes.
4. Parity default tier: Ultra (3 cascades) or the reference's boot default by device (Medium, 1 cascade)? The doc assumes Ultra so shadows are visible at every pose.
5. The reference time for the captures: water time and day fraction. The reference reads window.__timeOfDay and its own clock; captures need both pinned on both sides (proposal: day fraction 0.3 and water time 0).
6. Should the optional GPU LUT bake be built at all in this stage? The CPU bake meets the budget (1.6 s on the pool, once) and is the literal port; the GPU path only matters for wasm.
