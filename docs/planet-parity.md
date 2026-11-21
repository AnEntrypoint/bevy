# Planet parity table

Stage 0 of /workspace/hyperagent/specs/bevy-planet-parity.md. Every row names the reference value, its source path and line in /workspace/spoint (read only), the port's current value, and the stage that closes it. Status is open until the stage's witness passes. Port citations point into /workspace/bevy at the commit that added this file.

## Decisions

| id | decision | source |
|---|---|---|
| dec:radius | The default planet is the game preset: radius 63600, reliefScale 0.001, maxLevel 13, physics extent 256 at resolution 2, seed 1337. The engine default is 6360000 and must hold up, so the floating origin stays. | src/presets/planet.js:6-12; packages/mapspinner/src/planet-radius.js:1 |
| dec:caps | Parity is 30000 trees and 12000 rocks on the planet by the reference's placement rules. The caps come from a flat world (planetRadius 0). | apps/world/tps-game.js:9 and :12; client/core/Vegetation.js:84 |
| dec:budget | The reference states no frame budget. Stage 4: while streaming along the scripted paths, main-thread CPU time per frame stays within 1 ms of the same scene's idle p99 and no frame exceeds twice it. | spec, Reference numbers |
| dec:clouds | Clouds are out of scope: the reference has no cloud module. | spec, Reference numbers |

## Camera poses

Each witness uses these poses on the preset planet. The reference has no named presets for them, so each pose is built from its bookmarks (src/shared/bookmarks.js) and recorded in stage 1 with its eye height.

| pose | reference input | source |
|---|---|---|
| orbit | bookmark out-100km: chart point [100000, 0, 0] | src/shared/bookmarks.js:5 and :41 |
| high altitude | bookmark out-20km: chart point [20000, 0, 0] | src/shared/bookmarks.js:5 and :41 |
| low altitude | bookmark out-5km: chart point [5000, 0, 0] | src/shared/bookmarks.js:5 and :41 |
| ground in forest | search bookmark forest | src/shared/bookmarks.js:6 and :42 |
| shoreline | search bookmark coast | src/shared/bookmarks.js:6 and :42 |

## Measured: triangles per species

Measured by loading each preset through ez-tree's loadPreset, the call at client/core/VegetationBuild.js:496, with the species-to-preset names of VegetationBuild.js:19-23. The counts are deterministic (two runs identical) and are the counts before the tier caps (VegetationBuild.js:556: branch cap 6000, leaf cap 1400) and the meshopt tiers at 0.28 and 0.07 for branches and 0.30 and 0.09 for leaves (VegetationBuild.js:550-551). The triangle count after meshopt simplification is open until stage 3.

| species | preset | branch triangles | leaf triangles | branch vertices | leaf vertices | tier 0 caps |
|---|---|---|---|---|---|---|
| Oak Large | Oak Large | 12006 | 10560 | 8984 | 21120 | branch min(12006, 6000), leaf min(10560, 1400) |
| Pine Medium | Pine Medium | 10032 | 9840 | 6431 | 19680 | branch min(10032, 6000), leaf min(9840, 1400) |
| Aspen Medium | Aspen Medium | 5088 | 2112 | 3376 | 4224 | branch min(5088, 6000), leaf min(2112, 1400) |
| Ash Medium | Ash Medium | 9120 | 10880 | 6639 | 21760 | branch min(9120, 6000), leaf min(10880, 1400) |
| Bush | Bush 1 | 8880 | 4992 | 6331 | 9984 | branch min(8880, 6000), leaf min(4992, 1400) |
| Ash Small | Ash Small | 5616 | 5456 | 3706 | 10912 | branch min(5616, 6000), leaf min(5456, 1400) |
| Ash Large | Ash Large | 11832 | 9680 | 8738 | 19360 | branch min(11832, 6000), leaf min(9680, 1400) |
| Aspen Small | Aspen Small | 2496 | 1120 | 1642 | 2240 | branch min(2496, 6000), leaf min(1120, 1400) |
| Aspen Large | Aspen Large | 7200 | 6468 | 4861 | 12936 | branch min(7200, 6000), leaf min(6468, 1400) |
| Bush 2 | Bush 2 | 3352 | 1408 | 2375 | 2816 | branch min(3352, 6000), leaf min(1408, 1400) |
| Bush 3 | Bush 3 | 13674 | 2496 | 10212 | 4992 | branch min(13674, 6000), leaf min(2496, 1400) |
| Oak Small | Oak Small | 3206 | 3600 | 2344 | 7200 | branch min(3206, 6000), leaf min(3600, 1400) |
| Oak Medium | Oak Medium | 3166 | 10640 | 2780 | 21280 | branch min(3166, 6000), leaf min(10640, 1400) |
| Pine Small | Pine Small | 11112 | 7644 | 7124 | 15288 | branch min(11112, 6000), leaf min(7644, 1400) |
| Pine Large | Pine Large | 12192 | 7200 | 7817 | 14400 | branch min(12192, 6000), leaf min(7200, 1400) |

## Geometry and level of detail

| id | rule or number | reference value | source (path:line in /workspace/spoint) | port today | stage | status |
|---|---|---|---|---|---|---|
| geo:rad.R | planet radius default (m) | 6360000.0 | packages/mapspinner/src/planet-radius.js:1 | 6360.0 as DEFAULT_RADIUS (src/globe.rs:39) | 1 | open |
| geo:rad.resolve | radius override must be finite > 0 else TypeError; opts.radiusM wins over opts.radius | throws TypeError | packages/mapspinner/src/planet-radius.js:5 | self.radius.max(1.0) clamp only; no error (src/globe.rs:169) | 1 | open |
| geo:rad.relief | relief design radius (m) | 63600000.0 | packages/mapspinner/src/terrain-defaults.js:1 | RELIEF_DESIGN_RADIUS_M 63_600_000.0 (src/height/planet.rs:6) | 1 | open |
| geo:rad.reliefScaleDef | reliefScale default in the TSL planet = R / RELIEF_DESIGN_RADIUS_M | R / RELIEF_DESIGN_RADIUS_M (0.1 at default R) | packages/mapspinner/src/tsl/planet-tsl.js:87 | relief_scale None => compose multiplies by 1.0 (src/height/height_spec.rs:280) | 1 | open |
| geo:rad.amp | terrain amplitude as a function of radius (port-only field, biome colour only) | FRACTAL_HEIGHT_M = 750000.0 (height = frac * FRACTAL_HEIGHT_M) | packages/mapspinner/src/tsl/height-spec.js:2 | amplitude = DEFAULT_RADIUS * 3.0 / 64.0 (src/globe.rs:49) | 1 | open |
| geo:frm.faces | cube face count and frame table (c,u,v) for 6 faces | +X u(0,0,-1) v(0,1,0); -X u(0,0,1) v(0,1,0); +Y u(1,0,0) v(0,0,-1); -Y u(1,0,0) v(0,0,1); +Z u(1,0,0) v(0,1,0); -Z u(-1,0,0) v(0,1,0) | packages/mapspinner/src/planet-orchestrator-cull.js:3 | identical FACE_FRAME table (src/height/anchor.rs:3) | 1 | open |
| geo:frm.gpu | face frame mirrored in the WebGPU vertex shader (faceFrame mat3 per face) | fn faceFrame(f: f32) -> mat3x3<f32> (six frames, same as the CPU) | packages/mapspinner/src/webgpu/patch-grid-render.js:297 | no GPU path | 1 | open |
| geo:frm.warp | face-local to face-plane warp R*tan((p/R)*PI/4) in WebGPU shader | k = PI/4 | packages/mapspinner/src/webgpu/patch-grid-render.js:308 | lat/lon sphere mapping; no face warp (src/globe.rs:236) | 1 | open |
| geo:frm.warpTsl | same warp in TSL vertex path | QUARTER_PI = 0.7853981634 | packages/mapspinner/src/tsl/terrain-material-tsl.js:18 | none | 1 | open |
| geo:frm.camLocal | camera to face-local: in front of face when cc>1.0 gives ATAN_INV_K*R*atan(cu/cc); otherwise clamp to +/-R seam | ATAN_INV_K = 4/PI, cc>1.0, SEAM=R | packages/mapspinner/src/planet-orchestrator-cull.js:20 | none (chunk grid keyed by lat/lon) | 1 | open |
| geo:frm.pickFace | face chosen for near-field lattice = argmax over 6 faces of dot(camDir, face.c) | 6 faces | packages/mapspinner/src/planet-orchestrator-cull.js:102 | none | 1 | open |
| geo:frm.rootSize | quadtree root covers face-local [-size,size]^2 (side 2*size) per face at level 0 | origin -size, side 2*size | packages/mapspinner/src/quadtree.js:162 | lat/lon grid: rows ceil(180/span), cols ceil(360/span) (src/globe.rs:264) | 1 | open |
| geo:frm.defaultSize | Quadtree default size = PLANET_RADIUS_M | QUADTREE_DEFAULT_SIZE = PLANET_RADIUS_M (6360000) | packages/mapspinner/src/quadtree.js:4 | no quadtree | 1 | open |
| geo:lvl.levelCeil | hard ceiling on quadtree level count (QUADTREE_LEVEL_CEILING); levelCount accepts 0..32 | 32 | packages/mapspinner/src/quadtree.js:5 | none | 1 | open |
| geo:lvl.maxLevelDef | default maxLevel (TERRAIN_DEFAULTS.maxLevel) | 11 | packages/mapspinner/src/terrain-defaults.js:6 | no levels; single chunk size (src/globe.rs:51) | 1 | open |
| geo:lvl.MAX_MIN | lod-config clamp min for operator maxLevel override | 2 | packages/mapspinner/src/lod-config.js:8 | none | 1 | open |
| geo:lvl.MAX_MAX | MAX_LEVEL_MAX clamp for operator maxLevel override | 22 | packages/mapspinner/src/lod-config.js:9 | none | 1 | open |
| geo:lvl.rcRange | RenderControls maxLevel declared default, min, max | 11 / 2 / 22 | client/core/RenderControls.js:169 | none | 1 | open |
| geo:lvl.nearFloor | near-field leaf floor: node keeps splitting past maxLevel while side > leafM and gap within span | leafM = near.maxLeafM | packages/mapspinner/src/quadtree.js:114 | none | 1 | open |
| geo:lvl.leafSide | leaf side halves per level | 2R / 2^k (R=6360000: level 11 = 6211.3 m) | packages/mapspinner/src/quadtree.js:133 | chunk side = radius*chunk_span = 6360*0.35 = 2226 (src/globe.rs:472) | 1 | open |
| geo:grd.default | gridMeshSize default in TERRAIN_DEFAULTS (patch grid cells per side) | 9 | packages/mapspinner/src/terrain-defaults.js:5 | subdivisions default 8 (cells per chunk side) (src/globe.rs:54) | 1 | open |
| geo:grd.tslUse | TSL planet draws every patch with GRID_SIZE = TD.gridMeshSize | GRID_SIZE = TD.gridMeshSize (9) | packages/mapspinner/src/tsl/planet-tsl.js:22 | subdivisions fixed per chunk | 1 | open |
| geo:grd.glDefault | legacy GL orchestrator fallback gridMeshSize when opts omit it | 11 | packages/mapspinner/src/planet-orchestrator.js:19 | n/a | 1 | open |
| geo:grd.glClient | client passes gridMeshSize 9 to the GL planet | 9 | client/core/TerrainBackdrop.js:89 | n/a | 1 | open |
| geo:grd.webgpuDef | PatchGridRenderer default gridSize (webgpu orchestrator passes none) | 16 | packages/mapspinner/src/webgpu/patch-grid-render.js:752 | n/a | 1 | open |
| geo:grd.minInt | gridMeshSize must be an integer >= 2 | 2 | packages/mapspinner/src/planet-orchestrator.js:15 | subdivisions clamp(1, 64) (src/globe.rs:163) | 1 | open |
| geo:grd.ring | vertex grid has g2 = gridSize+2 cells and n2 = g2+1 vertices per side (one skirt ring each side) | n2 = gridSize + 3 | packages/mapspinner/src/patch-grid-geometry.js:3 | (subdivisions+1) per side, no ring (src/globe.rs:277) | 1 | open |
| geo:grd.vertsPer | vertices per patch with skirt (gridSize 9) | 144 = (9+3)^2; formula (gs+3)*(gs+3) | src/terrain/NearFieldLattice.js:96 | 81 = (8+1)^2 at subdivisions 8 (src/globe.rs:321) | 1 | open |
| geo:grd.interior | ring vertices carry z=1, interior z=0 | interior (gridSize+1)^2 = 100 at 9 | packages/mapspinner/src/patch-grid-geometry.js:13 | 81 all interior | 1 | open |
| geo:grd.indices | every cell of the (gridSize+2)^2 grid emits 2 triangles (6 indices) | 726 indices at 9 | packages/mapspinner/src/patch-grid-geometry.js:23 | 2*n*n triangles = 128 at n=8 (384 indices) (src/globe.rs:297) | 1 | open |
| geo:grd.uv | grid vertex xy = clamp((x-1)*du, 0, 1), du = 1/gridSize | ring clamps to 0 and 1 | packages/mapspinner/src/patch-grid-geometry.js:11 | lat/lon lerp i/n (src/globe.rs:282) | 1 | open |
| geo:grd.diag | triangle diagonal per cell by integer hash of (x,y): bit 17 of hashed value flips diagonal | multiplier 0x45d9f3b, (h>>>17)&1 | packages/mapspinner/src/patch-grid-geometry.js:23 | fixed diagonal [a,b,d],[a,d,c] (src/globe.rs:297) | 1 | open |
| geo:grd.skirtFlag | skirt ring present by default; without skirt ring cells are omitted | skirt = true | packages/mapspinner/src/patch-grid-geometry.js:1 | no skirt | 1 | open |
| geo:grd.winding | GPU path flips the shader normal outward when dot(nrm, dir0) < 0 | nrm = -nrm | packages/mapspinner/src/webgpu/patch-grid-render.js:366 | CPU triangle winding swapped when cross.dot(p0-center)<0 (src/globe.rs:301) | 1 | open |
| geo:grd.instance | instance attribute per patch: iOffset vec4 (ox,oy,size,level) plus iFace f32 | InstancedBufferAttribute(new Float32Array(capacity * 4), 4) as iOffset; InstancedBufferAttribute(new Float32Array(capacity), 1) as iFace (vec4 + f32) | packages/mapspinner/src/tsl/planet-tsl.js:66 | one entity per chunk (src/globe.rs:491) | 1 | open |
| geo:grd.cap | initial instance capacity | 2048 | packages/mapspinner/src/tsl/planet-tsl.js:25 | no instancing | 1 | open |
| geo:grd.capGrow | capacity growth factor when full (growCapacity) | x2 | packages/mapspinner/src/tsl/planet-tsl.js:202 | none | 1 | open |
| geo:spl.tan40 | split distance constant TAN_40DEG = tan(40 deg) | TAN_40DEG = Math.tan(40.0 / 180.0 * Math.PI) (0.8391) | packages/mapspinner/src/quadtree.js:3 | none | 1 | open |
| geo:spl.formula | splitDist = splitFactor * viewportH / 1024 * TAN_40DEG / tan(fovy/2) | floored at 1.1; non-finite -> 1.1 | packages/mapspinner/src/quadtree.js:79 | stream_radius distance test instead (src/globe.rs:483) | 1 | open |
| geo:spl.floor | splitDist floor | 1.1 | packages/mapspinner/src/quadtree.js:80 | none | 1 | open |
| geo:spl.initSplit | Quadtree constructor splitDist before applyLodConfig | 1.1 | packages/mapspinner/src/quadtree.js:21 | none | 1 | open |
| geo:spl.step | LOD_STEP | 7.2 | packages/mapspinner/src/lod-config.js:3 | none | 1 | open |
| geo:spl.stepApply | applyLodConfig: computeSplitDist(splitFactor*LOD_STEP, viewportH, fovy) then setConfig(R, maxLevel, distFactor) | qt.computeSplitDist(cfg.splitFactor * LOD_STEP, viewportH, cfg.fovy) (LOD_STEP = 7.2, so sf*7.2) | packages/mapspinner/src/lod-config.js:61 | none | 1 | open |
| geo:spl.sfDef | splitFactor default | 0.25 | packages/mapspinner/src/terrain-defaults.js:4 | none | 1 | open |
| geo:spl.sfFloor | splitFactor floor on operator override | 0.05 | packages/mapspinner/src/lod-config.js:7 | none | 1 | open |
| geo:spl.rcSf | RenderControls splitFactor default and min | 0.25 / 0.05 | client/core/RenderControls.js:167 | none | 1 | open |
| geo:spl.fovy | fovy must be in (0, PI) else RangeError | (0, PI) | packages/mapspinner/src/lod-config.js:43 | none | 1 | open |
| geo:spl.distFactor | distFactor when not overridden = splitFactor * LOD_POP_ALTITUDE_MUL | splitFactor*8.0 (2.0 at 0.25) | packages/mapspinner/src/lod-config.js:17 | none | 1 | open |
| geo:spl.popMul | LOD_POP_ALTITUDE_MUL | 8.0 | packages/mapspinner/src/lod-config.js:5 | none | 1 | open |
| geo:spl.distFactorInit | Quadtree distFactor constructor default (before setConfig) | 2.0 | packages/mapspinner/src/quadtree.js:22 | none | 1 | open |
| geo:spl.test | node splits when dist < l * effSplit and level < maxLevel (or forced by near floor) | dist < l*effSplit | packages/mapspinner/src/quadtree.js:132 | none | 1 | open |
| geo:spl.dist | node distance = max(altitudeTerm, per-axis nearer-edge distance dx, dy) (Chebyshev) | max(dz, dx, dy) | packages/mapspinner/src/quadtree.js:89 | Euclidean chunk-centre distance to focus (src/globe.rs:483) | 1 | open |
| geo:spl.altTerm | altitude term = max(camAlt / distFactor, 0) | camAlt/distFactor | packages/mapspinner/src/quadtree.js:157 | none | 1 | open |
| geo:spl.nearProtect | nearProtectRadius = max(camAlt*2, horizon*0.2, 10000*(size/6360000)), horizon = sqrt(2*size*camAlt) | 10000 m floor term, 2.0, 0.2 | packages/mapspinner/src/quadtree.js:159 | none | 1 | open |
| geo:spl.fall | lateral falloff: fall = 1/(1+max(0, latC-near)/near); effSplit = splitDist*max(floor, fall) | 1/(1+(latC-near)/near) | packages/mapspinner/src/quadtree.js:106 | none | 1 | open |
| geo:spl.floorFar | farCoarsenFloor = clamp(0.60 - log2(max(camAlt,5000)/5000)*0.05, 0.30, 0.60) | 0.30..0.60, slope 0.05 per octave | packages/mapspinner/src/quadtree.js:161 | none | 1 | open |
| geo:spl.floorKnee | altitude knee in far coarsen floor | 5000 m | packages/mapspinner/src/quadtree.js:160 | none | 1 | open |
| geo:spl.floorInit | farCoarsenFloor constructor default | 0.6 | packages/mapspinner/src/quadtree.js:33 | none | 1 | open |
| geo:spl.aim | lateral distance uses min of nadir distance and aim-point distance when aim given | aim = ray/ground intersection | packages/mapspinner/src/quadtree.js:99 | none | 1 | open |
| geo:spl.aimPoint | aimGroundPoint: ray-sphere intersection at radius R; null when disc<=0 or t<=0 | disc = b * b - (camDist * camDist - R * R) (sphere radius R) | packages/mapspinner/src/tsl/planet-tsl.js:55 | none | 1 | open |
| geo:spl.altRamp | GL only (opts.altSplitRamp): splitFactor multiplied by ramp with PEAK | PEAK 1.4; 150..700 km smoothstep; 700 km+ knot table | packages/mapspinner/src/planet-orchestrator.js:168 | none | 1 | open |
| geo:spl.knots | GL altSplitRamp knot table from 700 km (km, mult) | 700:0.865 ... 40000:0.45 | packages/mapspinner/src/planet-orchestrator.js:171 | none | 1 | open |
| geo:spl.lean | LOD_LEAN multiplies splitFactor only in the GL altSplitRamp branch (not used by TSL or WebGPU paths) | sf = sf * mul * LOD_LEAN | packages/mapspinner/src/planet-orchestrator.js:187 | chunk_span also 0.35 but is an angle in radians, unrelated (src/globe.rs:51) | 1 | open |
| geo:spl.leanDef | LOD_LEAN constant | 0.35 | packages/mapspinner/src/lod-config.js:4 | none | 1 | open |
| geo:spl.deckCap | GL deck cap: below DECK_CAP_ALT_KM maxLevel capped to 12 + round(2*mountainWeight) | 12..14 | packages/mapspinner/src/planet-orchestrator.js:191 | none | 1 | open |
| geo:spl.deckCapAlt | deckCapAltKm default (km) | 1.0 | client/core/RenderControls.js:173 | none | 1 | open |
| geo:spl.altDrop | GL high-altitude level drop: altKm>=30 and mxl>4 lowers by 1, by 2 when altKm>=700 | 30 km / 700 km | packages/mapspinner/src/planet-orchestrator.js:198 | none | 1 | open |
| geo:spl.mount | GL mountain weight ramp over elevAmp | 16.8 .. 18.6 | packages/mapspinner/src/planet-orchestrator.js:12 | none | 1 | open |
| geo:spl.cacheMove | TSL quad cache reuse tolerance moveTol = clamp((camDist-R)*0.00005, 1, 250) m | 1..250 m, 5e-5 | packages/mapspinner/src/tsl/planet-tsl.js:252 | rebuild when focus moved >= 0.5 * radius*chunk_span (src/globe.rs:473) | 1 | open |
| geo:spl.cacheDot | TSL quad cache reuse requires forward dot >= QUAD_CACHE_MIN_FORWARD_DOT | 0.99999 | packages/mapspinner/src/tsl/planet-tsl.js:30 | none | 1 | open |
| geo:spl.nearFloorInit | near leaf floor span defaults | outer 2.0, inner 0.0 | packages/mapspinner/src/quadtree.js:35 | none | 1 | open |
| geo:spl.nearClear | near floor uses ground clearance (camera above ground, not ellipsoid) / distFactor | clearance/distFactor | packages/mapspinner/src/quadtree.js:127 | none | 1 | open |
| geo:spl.groundClear | groundClearanceM = max(camDist - R - surfElev*R, 0) | Math.max(camDist - R - surfElev * R, 0) | packages/mapspinner/src/tsl/planet-tsl.js:165 | none | 1 | open |
| geo:gov.t1 | tier 0 and 1 (tier 0 is the booted base) | maxLevel 11, splitFactor 0.25, vegLeafOff 0, effectScale 1 | client/core/QualityGovernor.js:54 | none | 1 | open |
| geo:gov.t2 | tier 2 | maxLevel 10, splitFactor 0.20, effectScale 0.75 | client/core/QualityGovernor.js:56 | none | 1 | open |
| geo:gov.t3 | tier 3 | maxLevel 9, splitFactor 0.15, vegLeafOff 45, effectScale 0.5 | client/core/QualityGovernor.js:57 | none | 1 | open |
| geo:gov.t4 | tier 4 | maxLevel 8, splitFactor 0.10, vegLeafOff 42, effectScale 0.35 | client/core/QualityGovernor.js:58 | none | 1 | open |
| geo:gov.t5 | tier 5 (MAX_TIER) | maxLevel 7, splitFactor 0.07, vegLeafOff 40, effectScale 0.25 | client/core/QualityGovernor.js:59 | none | 1 | open |
| geo:gov.min | governor never raises: tier values are min(base, tier) | Math.min(base.*, t.*) | client/core/QualityGovernor.js:159 | none | 1 | open |
| geo:gov.maxTier | MAX_TIER | QUALITY_TIERS.length - 1 = 5 | client/core/QualityGovernor.js:61 | none | 1 | open |
| geo:cul.level | frustum cull in the quadtree only for nodes at level >= 2 | 2 | packages/mapspinner/src/quadtree.js:93 | none | 1 | open |
| geo:cul.maxElev | cull bounding elevation; CULL_ELEV_FRAC = this / PLANET_RADIUS_M | 500000 m | packages/mapspinner/src/planet-orchestrator-cull.js:31 | none | 1 | open |
| geo:cul.margin | NDC cull margin | 0.06 | packages/mapspinner/src/planet-orchestrator-cull.js:33 | none | 1 | open |
| geo:cul.sphere | node bound sphere around patch centre at radius R, radius = max corner distance + max elevation | radius = Math.sqrt(maxR2) + ME | packages/mapspinner/src/quadtree.js:224 | none | 1 | open |
| geo:cul.planes | six frustum planes; side planes widened by W = 1 + margin | W = 1.0 + CULL_NDC_MARGIN (1.06) | packages/mapspinner/src/planet-orchestrator-cull.js:85 | none | 1 | open |
| geo:cul.quad | per-leaf second cull quadOutsideFrustum projects a 3x3 sample grid at two radii | 3x3 samples, 2 radii | packages/mapspinner/src/planet-orchestrator-cull.js:48 | none | 1 | open |
| geo:cul.radii | quad cull sample radii R*(1 -/+ CULL_ELEV_FRAC) | radLo = R*(1.0-CULL_ELEV_FRAC), radHi = R*(1.0+CULL_ELEV_FRAC) | packages/mapspinner/src/planet-orchestrator-cull.js:42 | none | 1 | open |
| geo:cul.leafCull | leaves of level >= 2 outside frustum skipped when writing instances | (q.level / 0) >= 2 && quadOutsideFrustum(face, q.ox, q.oy, q.l, R, viewProjNoEye, camWorldPos) | packages/mapspinner/src/tsl/planet-tsl.js:201 | none | 1 | open |
| geo:cul.nearFar | near = 0.5 if altAboveTerrain<2 else max(0.1*alt, 0.5) | 0.5, 0.1, 2.0 | packages/mapspinner/src/tsl/planet-tsl.js:45 | Bevy default camera planes | 1 | open |
| geo:cul.farBlend | far blends from max(horizon, 8*alt) to camDist across altitude 500000..5000000 m | 500000, 4500000 | packages/mapspinner/src/tsl/planet-tsl.js:46 | none | 1 | open |
| geo:cul.farGround | farGround multiplier | 8.0 | packages/mapspinner/src/tsl/planet-tsl.js:47 | none | 1 | open |
| geo:cul.horizon | horizon sphere depth below sea | 150 m | packages/mapspinner/src/tsl/planet-tsl.js:23 | none | 1 | open |
| geo:cul.submerged | SUBMERGED_FAR_REACH | 60000 m | packages/mapspinner/src/tsl/planet-tsl.js:24 | none | 1 | open |
| geo:mor.cond | geomorph runs only when morphSplitDist>0, vertex not ring, level in [1, morphMaxLevel] | level 1..maxLevel | packages/mapspinner/src/tsl/terrain-material-tsl.js:151 | none | 1 | open |
| geo:mor.enable | geomorphLod default (RenderControls) | true | client/core/RenderControls.js:165 | none | 1 | open |
| geo:mor.weight | morph weight m = smoothstep((d-a)/(b-a)), a = size*effSplit(node), b = parentSize*effSplit(parent), t*t*(3-2t) | clamp 0..1, denominator floor size*1e-6 | packages/mapspinner/src/tsl/terrain-material-tsl.js:78 | none | 1 | open |
| geo:mor.parent | vertex snapped to parent grid: odd global vertex indices move to adjacent even index (parentGridPosition) | parentIndex = pairIndex.mul(2.0).add(isOdd.mul(pairIsOdd).mul(2.0)) | packages/mapspinner/src/tsl/geomorph-snap-tsl.js:10 | none | 1 | open |
| geo:mor.uniforms | morphSplitDist = qt.splitDist when geomorph enabled else 0; morphDistFactor, morphMaxLevel, morphNear = nearProtectRadius, morphFloor = farCoarsenFloor per frame | u.morphSplitDist.value = geomorphLod ? qt.splitDist : 0 | packages/mapspinner/src/tsl/planet-tsl.js:278 | none | 1 | open |
| geo:mor.aim | morphAim uniformArray of 6 vec3 per face (aim face-local xy, z=1 valid) | 6 entries | packages/mapspinner/src/tsl/terrain-material-tsl.js:118 | none | 1 | open |
| geo:mor.camLocal | camera face-local for morph uses the same atan mapping | 4/PI | packages/mapspinner/src/tsl/terrain-material-tsl.js:154 | none | 1 | open |
| geo:nrm.fd | normal from finite differences of composed height at world points d*(R+h): n = normalize(cross(wPU-wMU, wPV-wMV)) | normalize(cross(wPU.sub(wMU), wPV.sub(wMV))) | packages/mapspinner/src/tsl/terrain-material-tsl.js:176 | radial normals only: normal = (p - center).normalize() (src/globe.rs:488) | 1 | open |
| geo:nrm.step | finite difference step per patch | off.z * gridInv = patch size / gridSize | packages/mapspinner/src/tsl/terrain-material-tsl.js:150 | none | 1 | open |
| geo:nrm.taps | FD_TAPS loop bound 3 + loopBoundDelta(0): only taps 0,1,2 run (centre, +U, +V) so -U/-V reuse the centre (forward difference); taps 3,4 run only if loopBoundDelta=2 (never set in repo) | 3 | packages/mapspinner/src/tsl/terrain-material-tsl.js:17 | none | 1 | open |
| geo:nrm.loop | loop end expression | fdTaps + loopBoundDelta | packages/mapspinner/src/tsl/terrain-material-tsl.js:163 | none | 1 | open |
| geo:nrm.wgpu | WebGPU vertex shader computes central differences from 4 offset composeHeight calls (5 evaluations per vertex) | var nrm = normalize(cross(wPU - wMU, wPV - wMV)) | packages/mapspinner/src/webgpu/patch-grid-render.js:365 | none | 1 | open |
| geo:nrm.wgpuStep | WebGPU duP injected at pipeline build | 1/gridSize | packages/mapspinner/src/webgpu/patch-grid-render.js:267 | none | 1 | open |
| geo:nrm.up | fragment normal blended toward radial: normalize(mix(vN, dir0, NORMAL_TOWARD_UP)) | 0.05 | packages/mapspinner/src/tsl/terrain-material-tsl.js:20 | none | 1 | open |
| geo:nrm.flat | flatNormal default (when >0.5 normal = radial dir) | 0.0 | packages/mapspinner/src/terrain-defaults.js:34 | always radial (src/globe.rs:488) | 1 | open |
| geo:nrm.relief | reliefShade default (exaggerates normal away from radial when >1) | 6.0 | packages/mapspinner/src/terrain-defaults.js:21 | none | 1 | open |
| geo:nrm.reliefUse | relief shade application | normalize(dir0 + (n - dir0)*reliefShade) | packages/mapspinner/src/tsl/terrain-material-tsl.js:207 | none | 1 | open |
| geo:nrm.nrmStep | nrmStepM default | 0 | packages/mapspinner/src/terrain-defaults.js:14 | none | 1 | open |
| geo:nrm.texDn | texture normal perturbation scale texNrmK | 0.4 | packages/mapspinner/src/terrain-defaults.js:37 | none | 1 | open |
| geo:skt.height | skirt drop for ring vertices = max(patch size*0.06, 30*reliefScale (1 if reliefScale<=0)), subtracted from height; 0 for interior | max(off.z.mul(SKIRT_PATCH_FRAC), reliefOr1.mul(SKIRT_MIN_M)) (0.06*size, 30 m) | packages/mapspinner/src/tsl/terrain-material-tsl.js:179 | no skirt | 1 | open |
| geo:skt.frac | SKIRT_PATCH_FRAC | 0.06 | packages/mapspinner/src/tsl/terrain-material-tsl.js:22 | no skirt | 1 | open |
| geo:skt.min | SKIRT_MIN_M (multiplied by reliefScale) | 30.0 | packages/mapspinner/src/tsl/terrain-material-tsl.js:21 | no skirt | 1 | open |
| geo:skt.wgpu | WebGPU skirt, same rule with reliefScale | max(size*0.06, 30*reliefScale) | packages/mapspinner/src/webgpu/patch-grid-render.js:368 | no skirt | 1 | open |
| geo:skt.wgpuSample | WebGPU sample-mode pipeline skirt without reliefScale | max(size*0.06, 30.0) | packages/mapspinner/src/webgpu/patch-grid-render.js:530 | no skirt | 1 | open |
| geo:skt.noMorph | ring vertices (skirt) are excluded from geomorph | grid.z<0.5 | packages/mapspinner/src/tsl/terrain-material-tsl.js:151 | none | 1 | open |
| geo:skt.crack | crack handling = skirts plus geomorph to parent grid; no stitching or edge-collapse index buffers | vlist.push(px, py, isRing ? 1.0 : 0.0) | packages/mapspinner/src/patch-grid-geometry.js:13 | none (adjacent chunks share exact lat/lon edge vertices at equal size) (src/globe.rs:282) | 1 | open |
| geo:hgt.fraction | fractal fraction = (h0 + layerFbm + layerRidged)/3 | /3.0 | packages/mapspinner/src/tsl/height-spec.js:110 | identical (src/height/height_spec.rs:182) | 1 | open |
| geo:hgt.l0 | layer0 ridged fbm: gain, octaves, offset, exponent | 0.5, 10, 1.064, 1.005 | packages/mapspinner/src/tsl/height-spec.js:103 | identical (src/height/height_spec.rs:180) | 1 | open |
| geo:hgt.l1 | layer1 value fbm gain 0.5, 18 octaves, mapped -2 + 4*(raw*0.5+0.5) | 0.5, 18 | packages/mapspinner/src/tsl/height-spec.js:104 | identical (src/height/height_spec.rs:170) | 1 | open |
| geo:hgt.l2 | layer2 ridged fbm gain, octaves, offset, exponent; mapped -2 + 4*raw | 0.5, 18, 1.064, 1.1 | packages/mapspinner/src/tsl/height-spec.js:105 | identical (src/height/height_spec.rs:175) | 1 | open |
| geo:hgt.warp | domain warp: warped = p + p*1.6*h0 | 1.6 | packages/mapspinner/src/tsl/height-spec.js:109 | identical (src/height/height_spec.rs:181) | 1 | open |
| geo:hgt.scale | direction scaled before fractal | 3.0 | packages/mapspinner/src/tsl/height-spec.js:115 | identical (src/height/height_spec.rs:188) | 1 | open |
| geo:hgt.shape | h = (sample - 0.17) * 0.6 | 0.17, 0.6 | packages/mapspinner/src/tsl/height-spec.js:116 | identical (src/height/height_spec.rs:189) | 1 | open |
| geo:hgt.pmix | power exponent 0.8*mix(0.95, 1.3, pmix), pmix = snoise3(p*0.53 + (123,456,789))*0.5+0.5 | 0.53, 0.95, 1.3, 0.8 | packages/mapspinner/src/tsl/height-spec.js:118 | identical (src/height/height_spec.rs:192) | 1 | open |
| geo:hgt.cratio | cRatio = clamp(snoise3(dir*4)*0.5+0.7, 0.3, 1.0) multiplies shaped height | 4.0, 0.5, 0.7, 0.3, 1.0 | packages/mapspinner/src/tsl/height-spec.js:120 | identical (src/height/height_spec.rs:196) | 1 | open |
| geo:hgt.fracM | FRACTAL_HEIGHT_M | 750000.0 | packages/mapspinner/src/tsl/height-spec.js:2 | 750_000.0 (src/height/height_spec.rs:9) | 1 | open |
| geo:hgt.contBias | CONTINENTAL_BIAS_AMP | 50.0 | packages/mapspinner/src/tsl/height-spec.js:1 | 50.0 (src/height/height_spec.rs:10) | 1 | open |
| geo:hgt.underGain | UNDERWATER_GAIN | 1.25 | packages/mapspinner/src/tsl/height-spec.js:3 | 1.25 (src/height/height_spec.rs:11) | 1 | open |
| geo:hgt.underFloor | UNDERWATER_FLOOR_M | -350000.0 | packages/mapspinner/src/tsl/height-spec.js:4 | -350_000.0 (src/height/height_spec.rs:12) | 1 | open |
| geo:hgt.beach | DEFAULT_BEACH_SHELF_M (used when beachShelfM <= 1) | 150.0 | packages/mapspinner/src/tsl/height-spec.js:5 | 150.0 (src/height/height_spec.rs:13) | 1 | open |
| geo:hgt.landBias | landBias default added to height (m) | -100000.0 | packages/mapspinner/src/terrain-defaults.js:8 | LAND_BIAS_M -100_000.0 (src/height/planet.rs:7) | 1 | open |
| geo:hgt.shelf | beach shelf: quadratic blend below shelf, underwater branch for h<0 | h*h/shelf*(2 - h/shelf) | packages/mapspinner/src/tsl/height-spec.js:158 | identical (src/height/height_spec.rs:269) | 1 | open |
| geo:hgt.relief | reliefScale multiplies composed height when >0 else 1.0 | o.sel(o.gt(reliefParam, 0.0), reliefParam, 1.0) | packages/mapspinner/src/tsl/height-spec.js:165 | identical (src/height/height_spec.rs:280) | 1 | open |
| geo:hgt.order | composeHeight = reliefScale(carve(natural)) plus sculpt delta | natural -> carve -> relief -> sculpt | packages/mapspinner/src/tsl/height-spec.js:182 | compose = relief(carve(natural)); no sculpt (src/height/height_spec.rs:61) | 1 | open |
| geo:hgt.carves | MAX_TERRAIN_CARVES | 4 | packages/mapspinner/src/tsl/height-spec.js:10 | 4 (src/height/height_spec.rs:3) | 1 | open |
| geo:hgt.hpfRes | HPF texture resolution default (opts.hpfTexRes) | 128 | packages/mapspinner/src/tsl/planet-tsl.js:88 | DEFAULT_HPF_TEX_RES 128 (src/globe.rs:40) | 1 | open |
| geo:hgt.hpfWebgpu | HPF resolution in the webgpu orchestrator | 128 | packages/mapspinner/src/webgpu/planet-orchestrator-webgpu.js:97 | 128 (src/globe.rs:40) | 1 | open |
| geo:hgt.hpfInterp | HPF sampling: quintic bilinear over 4 texels per face, denom hpfRes-1, clamped | quintic t^3(6t^2-15t+10) | packages/mapspinner/src/tsl/height-spec.js:137 | hpf_x (src/height/height_spec.rs:239) | 1 | open |
| geo:hgt.hpfSeed | HPF anchor field seed default | 1337 | packages/mapspinner/src/tsl/planet-tsl.js:91 | DEFAULT_SEED 0x5eed1234 (globe seed) (src/globe.rs:38) | 1 | open |
| geo:hgt.rot | octave rotation step (radians) and table size | 0.5236 / 32 | packages/mapspinner/src/tsl/height-spec.js:18 | identical (src/height/height_spec.rs:7) | 1 | open |
| geo:hgt.hashVer | hash versions: 1 float (legacy), 2 integer; TSL default is float(1) | 1 / 2 | packages/mapspinner/src/tsl/planet-tsl.js:89 | default HashVersion::Integer (src/globe.rs:59) | 1 | open |
| geo:hgt.hashMul | integer hash multipliers | 0x8da6b343, 0xd8163841, 0xcb1ab31f | packages/mapspinner/src/tsl/height-spec.js:12 | identical (src/height/height_spec.rs:4) | 1 | open |
| geo:gpu.wgslLayers | standalone WGSL fractalTerrainH: ridged 10 oct, fbm 18 oct, ridged 18 oct; float hash h3 | 10 / 18 / 18 | packages/mapspinner/src/webgpu/terrain-height-wgsl.js:68 | no GPU or WGSL height evaluation; CPU f64 only (src/height/height_spec.rs:60) | 1 | open |
| geo:gpu.wgslWG | height compute workgroup size | 64 | packages/mapspinner/src/webgpu/terrain-height-wgsl.js:112 | none | 1 | open |
| geo:gpu.wgslIO | compute bindings: dirs f32 array (xyz triples), heights f32 array, params vec4<u32>.x = count | 3 bindings | packages/mapspinner/src/webgpu/terrain-height-wgsl.js:108 | none | 1 | open |
| geo:gpu.wgslOut | WGSL output is the fractal fraction only (no landBias, FRACTAL_HEIGHT_M, relief) | h = fractalTerrainH(d) | packages/mapspinner/src/webgpu/terrain-height-wgsl.js:119 | none | 1 | open |
| geo:gpu.thcRes | height bake tile resolution THC_BAKE_RES (also default poolRes) | 130 | packages/mapspinner/src/webgpu/height-bake-compute.js:8 | none | 1 | open |
| geo:gpu.sculpt | SCULPT_RES sculpt texture size in the webgpu orchestrator | 256 | packages/mapspinner/src/webgpu/planet-orchestrator-webgpu.js:19 | none | 1 | open |
| geo:gpu.evals | composeHeight evaluations per vertex: TSL 3 (centre,+U,+V); WebGPU 5 (centre + 4 neighbours) | 3 / 5 | packages/mapspinner/src/webgpu/patch-grid-render.js:350 | one elevation() call per vertex (src/globe.rs:284) | 1 | open |
| geo:gpu.hpfClimate | climate sampled per vertex via hpfSample(d0).zw | vClim.assign(spec.hpfSample(d0).zw) | packages/mapspinner/src/tsl/terrain-material-tsl.js:186 | none | 1 | open |
| geo:gpu.texWarp | vertex texWarp: snoise3 of dir*450, amplitude 1.2 | 450.0 / 1.2 | packages/mapspinner/src/tsl/terrain-material-tsl.js:23 | none | 1 | open |
| geo:nf.steps | near-field tile grid steps default (cells per tile side) | 9 | src/terrain/NearFieldLattice.js:2 | none | 1 | open |
| geo:nf.budget | near-field default tile budget | 64 | src/terrain/NearFieldLattice.js:4 | none | 1 | open |
| geo:nf.budgetUse | plan() instance budget passed by planet-tsl = leaves.length of the near face; perAxis = max(1, min(floor(sqrt(budget)), wanted, coverageCap)) | const budget = Number.isInteger(instanceBudget) && instanceBudget >= 1 ? instanceBudget : leaves.length; const perAxis = Math.max(1, Math.min(Math.floor(Math.sqrt(budget)), wanted, coverageCap)) | src/terrain/NearFieldLattice.js:59-61; packages/mapspinner/src/tsl/planet-tsl.js:193 | none | 1 | open |
| geo:nf.floorTiles | NEAR_FIELD_FLOOR_TILES: leaf floor = tileM * 1 | 1 | src/terrain/NearFieldLattice.js:6 | none | 1 | open |
| geo:nf.minGround | MIN_GROUND_PER_FACE_METRE (outer reach divides by it) | 0.6 | src/terrain/NearFieldLattice.js:8 | none | 1 | open |
| geo:nf.tileM | tileM = gridSteps * spacingM (spacing = collider spacing) | 9 * spacing | src/terrain/NearFieldLattice.js:23 | none | 1 | open |
| geo:nf.reach | floorReachLeaves outerM=(cap*tileM/2+tileM)/0.6, innerM=max(0, cap*tileM/(2*gpf)-maxLeafM) | const outerM = (coverageCap * tileM / 2 + tileM) / MIN_GROUND_PER_FACE_METRE; const innerM = Math.max(0, (coverageCap * tileM) / (2 * gpf) - maxLeafM) | src/terrain/NearFieldLattice.js:29-30 | none | 1 | open |
| geo:nf.verts | near-field tile vertex count (gs+3)^2 per tile (same ring grid) | vertexCount: tiles.length * (gs + 3) * (gs + 3) (144 per tile at gs 9) | src/terrain/NearFieldLattice.js:96 | none | 1 | open |
| geo:nf.gpf | groundPerFaceMetre: 3x3 samples spread by halfSpan, step d = max(1e-6, R*1e-4), larger axis returned | d = R*1e-4 | packages/mapspinner/src/quadtree.js:181 | none | 1 | open |
| geo:nf.gpfDoc | cube-sphere face metre is 0.778 m of ground at the spawn (non-conformal map) | 0.778 (value stated in comment) | packages/mapspinner/src/quadtree.js:170 | none | 1 | open |
| geo:nf.suppress | tile suppressed when fully covered by a straddling leaf; fully covered leaves dropped | if (covered) { suppressed++; continue } | src/terrain/NearFieldLattice.js:88-90 | none | 1 | open |
| geo:nf.faceClamp | near tile origin clamped to face half span (faceHalfSpanM = radius) | x0 = Math.min(Math.max(x0, -faceHalfSpanM), faceHalfSpanM - perAxis * tileMLocal) | src/terrain/NearFieldLattice.js:66 | none | 1 | open |
| geo:nf.level | near tiles drawn with level 0 (excluded from morph, which needs level>=1) | level 0 | src/terrain/NearFieldLattice.js:91 | none | 1 | open |
| geo:col.extent | collider heightfield default extent (m) | 510 | src/shared/colliderLattice.js:2 | 510.0 fallback (src/terrain_stream.rs:97) | 1 | open |
| geo:col.res | collider default resolution (m) | 4 | src/shared/colliderLattice.js:3 | 4.0 fallback (src/terrain_stream.rs:98) | 1 | open |
| geo:col.n | collider N = even(max(2, round(extent/resolution))); spacing = extent/(N-1) | N=128, spacing 4.016 m at defaults | src/shared/colliderLattice.js:21 | n = round_half_up(extent/res).max(2), +1 if odd; spacing extent/(n-1) (src/terrain_stream.rs:100, src/terrain_stream.rs:111) | 1 | open |
| geo:col.follow | collider follows a baked source spacing: sourceN = clamp(round(extent/sourceSpacing)+1, 2, FINE_FIELD_MAX_N); N = max(N, sourceN) | sourceN = Math.min(Math.max(2, Math.round(extent / sourceSpacingM) + 1), FINE_FIELD_MAX_N); if (sourceN > N) N = sourceN | src/shared/colliderLattice.js:27 | none | 1 | open |
| geo:col.fineMax | FINE_FIELD_MAX_N = floor(sqrt(1 MiB / 4 B)) | Math.floor(Math.sqrt(FINE_FIELD_WASM_BUDGET_BYTES / FIELD_WASM_BYTES_PER_SAMPLE)) (512) | src/shared/colliderLattice.js:7 | none | 1 | open |
| geo:col.fineBudget | fine field wasm budget bytes | 1 MiB | src/shared/colliderLattice.js:6 | none | 1 | open |
| geo:col.fieldBudget | coarse field wasm budget bytes | 32 MiB | src/terrain/HeightfieldStreamer.js:6 | 32.0*1024*1024 (src/terrain_stream.rs:3) | 1 | open |
| geo:col.sampleBytes | bytes per height sample in field budgeting | 4 | src/terrain/HeightfieldStreamer.js:7 | 4.0 (src/terrain_stream.rs:4) | 1 | open |
| geo:col.maxFields | max fields = floor(budget / (n*n) / 4), at least 1 | Math.max(1, Math.floor(FIELD_WASM_BUDGET_BYTES / (maxGridN * maxGridN * FIELD_WASM_BYTES_PER_SAMPLE))) | src/terrain/HeightfieldStreamer.js:141 | same formula (src/terrain_stream.rs:107) | 1 | open |
| geo:col.rebuild | field rebuild trigger: coverRadius = extent * rebuildAt | 0.4 | src/terrain/HeightfieldStreamer.js:114 | rebuild_at default 0.4; cover_radius = extent*rebuild_at (src/terrain_stream.rs:99, src/terrain_stream.rs:113) | 1 | open |
| geo:col.fineMs | fine sample time budget (ms) | 600 | src/terrain/HeightfieldStreamer.js:9 | none | 1 | open |
| geo:col.retain | patch retain max | 16 | src/terrain/HeightfieldStreamer.js:16 | none | 1 | open |
| geo:col.snap | height snap per metre | 1000 | src/terrain/HeightfieldStreamer.js:5 | none | 1 | open |
| geo:col.nonFinite | non-finite height fallback (m) | -1000 | src/terrain/HeightfieldStreamer.js:4 | none | 1 | open |
| geo:col.kind | collider kind: N x N heightfield streamed near players; port globe chunks use a trimesh collider per chunk | shape: 'heightfield' | src/physics/World.js:433 | Collider::trimesh per chunk; heightfield streamer exists in terrain_stream.rs (src/globe.rs:493, src/terrain_stream.rs:131) | 1 | open |
| geo:bud.meshQuads | TerrainMeshSurface default max quads | 512 | src/terrain/TerrainMeshSurface.js:17 | none | 1 | open |
| geo:bud.focus | TerrainMeshSurface DEFAULT_FOCUS_RADIUS_M | 240 | src/terrain/TerrainMeshSurface.js:16 | none | 1 | open |
| geo:bud.mshFan | NEAR_GRID_MAX_FAN | 64 | src/terrain/TerrainMeshSurface.js:113 | NEAR_GRID_MAX_FAN 64 (src/terrain_mesh_surface.rs:19) | 1 | open |
| geo:bud.colCompute | ColliderStreamer COMPUTE_BUDGET_MS per pass (veg and rock colliders) | 8 ms | src/terrain/ColliderStreamer.js:233 | none | 1 | open |
| geo:bud.colChunks | MAX_NEW_CHUNKS_PER_PASS | 64 | src/terrain/ColliderStreamer.js:234 | none | 1 | open |
| geo:bud.colAdd | ADD_BUDGET_MS and CLASSIFY_BUDGET_MS per slice | 2 ms / 2 ms | src/terrain/ColliderStreamer.js:575 | none | 1 | open |
| geo:bud.colHyst | keep-radius hysteresis factor | 1.1 | src/terrain/ColliderStreamer.js:4 | restream threshold 0.5*chunk step only (src/globe.rs:473) | 1 | open |
| geo:bud.colQuant | collider ring center quantum (m) | 4 | src/terrain/ColliderStreamer.js:249 | none | 1 | open |
| geo:bud.colRing | ring cache cap | 512 | src/terrain/ColliderStreamer.js:250 | none | 1 | open |
| geo:bud.colChunkCache | chunk cache cap | 4096 | src/terrain/ColliderStreamer.js:222 | none | 1 | open |
| geo:bud.colMaxCenters | default max collider centers | 192 | src/terrain/ColliderStreamer.js:23 | bodies list in config (src/globe.rs:57) | 1 | open |
| geo:bud.colByte | default byte budget = cap * 8 * 512 bytes per body | 8 * 512 | src/terrain/ColliderStreamer.js:5 | none | 1 | open |
| geo:bud.bakeInflight | remote patch baker MAX_IN_FLIGHT | 32 | packages/mapspinner/src/patch-baker-remote.js:3 | none | 1 | open |
| geo:bud.bakeUnclaimed | remote patch baker MAX_UNCLAIMED | 96 | packages/mapspinner/src/patch-baker-remote.js:4 | none | 1 | open |
| geo:bud.draws | terrain render order after opaque occluders | 10 | packages/mapspinner/src/tsl/planet-tsl.js:27 | StandardMaterial default (src/globe.rs:506) | 1 | open |
| geo:bud.bakeLevel | GL bake max level = round(log2(HPF_RES)) | BAKE_MAX_LEVEL = Math.round(Math.log2(HPF_RES)) (7 at 128) | packages/mapspinner/src/planet-orchestrator.js:52 | none | 1 | open |
| geo:tex.tile | texTile base scale (m), scaled by R/PLANET_RADIUS_M | 2400 | packages/mapspinner/src/terrain-defaults.js:57 | none | 1 | open |

## Materials and textures

| id | rule or number | reference value | source (path:line in /workspace/spoint) | port today | stage | status |
|---|---|---|---|---|---|---|
| mat:col.deepsea | bcDeepSea RGB | [0.02,0.06,0.22] | packages/mapspinner/src/terrain-defaults.js:71 | src/globe.rs:340 water (0.10,0.25,0.55) when elevation<=sea_level (src/globe.rs:287) (single water colour, no deep/shallow split) | 2 | open |
| mat:col.sea | bcSea RGB | [0.08,0.18,0.38] | packages/mapspinner/src/terrain-defaults.js:72 | src/globe.rs:340 water (0.10,0.25,0.55) when elevation<=sea_level (src/globe.rs:287) | 2 | open |
| mat:col.shore | bcShore RGB (also sand layer material colour) | [0.82,0.76,0.52] | packages/mapspinner/src/terrain-defaults.js:73 | src/globe.rs:341-342 relative<0.25 -> (0.80,0.74,0.50) | 2 | open |
| mat:col.lowland | bcLowland RGB | [0.18,0.38,0.10] | packages/mapspinner/src/terrain-defaults.js:74 | absent (no lowland band); nearest src/globe.rs:343-344 relative<0.6 -> (0.25,0.55,0.22) | 2 | open |
| mat:col.grass | bcGrass RGB (also grass layer material colour) | [0.30,0.46,0.12] | packages/mapspinner/src/terrain-defaults.js:75 | src/globe.rs:343-344 relative<0.6 -> (0.25,0.55,0.22) | 2 | open |
| mat:col.rock | bcRock RGB default (overwritten at runtime by rock layer mean) | [0.48,0.40,0.30] | packages/mapspinner/src/terrain-defaults.js:76 | src/globe.rs:345-346 relative<0.85 -> (0.45,0.40,0.35) | 2 | open |
| mat:col.snow | bcSnow RGB | [0.94,0.96,1.00] | packages/mapspinner/src/terrain-defaults.js:77 | src/globe.rs:347-348 else -> (0.95,0.95,0.97) | 2 | open |
| mat:band.edges.lo | bandEdgesLo: shore->lowland smoothstep(0,lo0,h), lowland->grass smoothstep(lo0,lo1,h), metres | [8,20] | packages/mapspinner/src/terrain-defaults.js:78; used packages/mapspinner/src/tsl/surface-splat-tsl.js:43 | height band uses relative=elevation/amplitude thresholds 0.25/0.6/0.85 (src/globe.rs:341-347), hard steps no smoothstep | 2 | open |
| mat:band.edges.hi | bandEdgesHi: grass->rock smoothstep(bandWarp+hi0,bandWarp+hi1,h), metres | [60,100] | packages/mapspinner/src/terrain-defaults.js:79; used packages/mapspinner/src/tsl/surface-splat-tsl.js:46 | rock at relative>=0.6 hard step (src/globe.rs:345) | 2 | open |
| mat:band.edges.snow | snowEdges: rock->snow smoothstep(bandWarp+sn0,bandWarp+sn1,h), metres | [120,180] | packages/mapspinner/src/terrain-defaults.js:80; used packages/mapspinner/src/tsl/surface-splat-tsl.js:47 | snow at relative>=0.85 hard step (src/globe.rs:347) | 2 | open |
| mat:band.warp | bandWarp amplitude (m); 3 octave snoise at freq 210/560/1450 (x0.004 pre-warp at 130) weights 1/0.5/0.25 | 20 | packages/mapspinner/src/terrain-defaults.js:69; packages/mapspinner/src/tsl/surface-splat-tsl.js:44-packages/mapspinner/src/tsl/surface-splat-tsl.js:45 | absent: flat vertex colour only (src/globe.rs:338-351); no noise-warped band edges | 2 | open |
| mat:band.warp.splat | splat bandWarp: snoise freq 1100 + 0.5*snoise freq 2580, times bandWarp*0.25 | 1100 / 2580 / 0.5 / 0.25 | packages/mapspinner/src/tsl/surface-splat-tsl.js:90; packages/mapspinner/src/splat-weights.js:12,packages/mapspinner/src/splat-weights.js:25-packages/mapspinner/src/splat-weights.js:26 | absent: flat vertex colour only (src/globe.rs:338-351) | 2 | open |
| mat:slope.rock | slopeRock: rock blend smoothstep(lo, hi+rockWiden, rockSlope), land only (step(0,h)) | [0.25,0.55] | packages/mapspinner/src/terrain-defaults.js:82; used packages/mapspinner/src/tsl/surface-splat-tsl.js:48 | absent: colour ignores slope | 2 | open |
| mat:slope.def | slope = 1 - max(0,dot(n,dir0)); rockSlope = clamp(slope,0,1) | slope = float(1.0).sub(max(0.0, dot(n, dir0))); rockSlope = clamp(slope, 0.0, 1.0) | packages/mapspinner/src/tsl/terrain-material-tsl.js:193 | absent: no slope input to colour | 2 | open |
| mat:slope.rockwiden | rockWiden = smoothstep(20,500,pxWorld)*0.20 | 20 / 500 / 0.20 | packages/mapspinner/src/tsl/surface-splat-tsl.js:34 | absent | 2 | open |
| mat:sea.floor.depth | seabed depthT = clamp(-h/300,0,1) | 300 m | packages/mapspinner/src/tsl/surface-splat-tsl.js:35 | absent (single water colour) | 2 | open |
| mat:sea.floor.silt | seabed silt colour, mix shore->silt smoothstep(0,0.5,depthT) | [0.12,0.11,0.09] | packages/mapspinner/src/tsl/surface-splat-tsl.js:36 | absent | 2 | open |
| mat:sea.floor.basalt | seabed basalt colour, mix smoothstep(0.5,1,depthT) | [0.06,0.06,0.07] | packages/mapspinner/src/tsl/surface-splat-tsl.js:37 | absent | 2 | open |
| mat:sea.floor.rock | seabed to bcRock by slopeRock smoothstep | seaFloor = mix(bed, bcRock, smoothstep(SLOPE_ROCK[0], SLOPE_ROCK[1], rockSlope)) | packages/mapspinner/src/tsl/surface-splat-tsl.js:38 | absent | 2 | open |
| mat:sea.ice.range | seaIce = 1 - smoothstep(0.12,0.22,temp) | 0.12 / 0.22 | packages/mapspinner/src/tsl/surface-splat-tsl.js:39 | absent: no temperature field in colour | 2 | open |
| mat:sea.ice.colour | sea ice colour and mix 0.9 | [0.82,0.88,0.94] x0.9 | packages/mapspinner/src/tsl/surface-splat-tsl.js:40 | absent | 2 | open |
| mat:sea.depthm | seaDepthM | 580 | packages/mapspinner/src/terrain-defaults.js:81 | absent (value unused in tsl/ and splat files read) | 2 | open |
| mat:sea.ocean.deep | uOceanDeep water-shader colour | [0.008,0.025,0.06] | packages/mapspinner/src/terrain-defaults.js:84 | absent | 2 | open |
| mat:sea.ocean.shallow | uOceanShallow | [0.07,0.22,0.26] | packages/mapspinner/src/terrain-defaults.js:85 | absent | 2 | open |
| mat:sea.ocean.k | uOceanK absorption | [0.009,0.004,0.0018] | packages/mapspinner/src/terrain-defaults.js:86 | absent | 2 | open |
| mat:beach.top | beachTop metres; beachM=(1-smoothstep(0.3*top,top,h))*(1-smoothstep(slopeRock)) | 15 | packages/mapspinner/src/terrain-defaults.js:67; packages/mapspinner/src/tsl/surface-splat-tsl.js:50 | absent | 2 | open |
| mat:beach.width | beachWidth multiplier for splat beach edge | 1.0 | packages/mapspinner/src/terrain-defaults.js:68; packages/mapspinner/src/tsl/surface-splat-tsl.js:92 | absent | 2 | open |
| mat:beach.mix | beach colour = bcShore mixed by beachM | beached = mix(mottled, BC_SHORE, beachM) | packages/mapspinner/src/tsl/surface-splat-tsl.js:51 | absent | 2 | open |
| mat:var.amt | variationAmt: albedo mottle snoise freq 120 | 0.05 / freq 120 | packages/mapspinner/src/terrain-defaults.js:23; packages/mapspinner/src/tsl/surface-splat-tsl.js:49 | absent | 2 | open |
| mat:ridge.octaves | ridge albedo octaves [freq,amp]; nyquist smoothstep(0.03wl,0.12wl,pxWorld), wl=reliefScale*40000000/freq, strength 0.02 | [[75,1.0],[375,0.6]], 0.02 | packages/mapspinner/src/tsl/surface-splat-tsl.js:14; packages/mapspinner/src/tsl/surface-splat-tsl.js:54-packages/mapspinner/src/tsl/surface-splat-tsl.js:59 | absent | 2 | open |
| mat:climate.source | temp/humidity from HPF texture zw channels (vClim), temp used for sea ice and sand region | spec.hpfSample(d0).zw | packages/mapspinner/src/tsl/terrain-material-tsl.js:186 | absent: no climate | 2 | open |
| mat:climate.hpfres | HPF texture resolution (opts.hpfTexRes default) | 128 | packages/mapspinner/src/tsl/planet-tsl.js:88 | absent | 2 | open |
| mat:climate.hpfseed | HPF seed default | 1337 | packages/mapspinner/src/tsl/planet-tsl.js:91 | seed 0x5eed1234 at src/globe.rs:38 (different field) | 2 | open |
| mat:climate.params | anchor-field parameter keys | seaBias, elevAmp, temp, humidity, erosion, roughness | packages/mapspinner/src/anchor-field.js:4 | absent | 2 | open |
| mat:splat.layers | layer order index 0..3 and material names | grass, rock, sand, snow | packages/mapspinner/src/surface-texture-decode.js:47 | absent: flat vertex colour only (src/globe.rs:338-351) | 2 | open |
| mat:splat.dryhot.arid | dryHot aridity smoothstep(0.60,0.85,1-humid) | 0.60 / 0.85 | packages/mapspinner/src/splat-weights.js:8; packages/mapspinner/src/tsl/surface-splat-tsl.js:89 | absent | 2 | open |
| mat:splat.dryhot.temp | dryHot temperature smoothstep(0.42,0.62,temp) | 0.42 / 0.62 | packages/mapspinner/src/splat-weights.js:9; packages/mapspinner/src/tsl/surface-splat-tsl.js:89 | absent | 2 | open |
| mat:splat.beach.slope | sand region beach slope fade smoothstep | 0.18 / 0.55 | packages/mapspinner/src/splat-weights.js:6; packages/mapspinner/src/tsl/surface-splat-tsl.js:92 | absent | 2 | open |
| mat:splat.sand.region | sandRegion = clamp(max(dryHot,beach),0,1) | sandRegion = clamp(max(dryHot, beach), 0.0, 1.0) | packages/mapspinner/src/tsl/surface-splat-tsl.js:93 | absent | 2 | open |
| mat:splat.rock.slope | wRockSlope smoothstep(mix(srLo,0.50,sand),mix(srHi,0.70,sand),rockSlope); srLo=max(slopeRock0,0.05), srHi=max(slopeRock1,srLo+0.25) | 0.50 / 0.70 / 0.05 / 0.25 | packages/mapspinner/src/tsl/surface-splat-tsl.js:94-packages/mapspinner/src/tsl/surface-splat-tsl.js:95; packages/mapspinner/src/splat-weights.js:20,packages/mapspinner/src/splat-weights.js:31 | absent | 2 | open |
| mat:splat.rock.band | rockBand smoothstep(0.7*sn0,0.9*sn0 + bandWarp,h)*(1-snowHi) | 0.7 / 0.9 | packages/mapspinner/src/splat-weights.js:10; packages/mapspinner/src/tsl/surface-splat-tsl.js:98 | absent | 2 | open |
| mat:splat.snow.hi | snowHi smoothstep(bandWarp+sn0,bandWarp+sn1,h) | snowHi = smoothstep(bandWarp.add(sn[0]), bandWarp.add(sn[1]), h), sn = TD.snowEdges | packages/mapspinner/src/tsl/surface-splat-tsl.js:97 | absent | 2 | open |
| mat:splat.snow.suppress | wSnow *= 1 - 0.6*wRock | 0.6 | packages/mapspinner/src/splat-weights.js:11; packages/mapspinner/src/tsl/surface-splat-tsl.js:100 | absent | 2 | open |
| mat:splat.sand.weight | wSand = sandRegion*(1-wRock)*(1-wSnow)*(1-smoothstep(0.30,0.70,slope)) | 0.30 / 0.70 | packages/mapspinner/src/splat-weights.js:7; packages/mapspinner/src/tsl/surface-splat-tsl.js:101 | absent | 2 | open |
| mat:splat.grass.weight | wGrass = max(1-wRock-wSnow-wSand,0) | wGrass = max(float(1.0).sub(wRock).sub(wSnow).sub(wSand), 0.0) | packages/mapspinner/src/tsl/surface-splat-tsl.js:102 | absent | 2 | open |
| mat:splat.underwater | uwM = 1-smoothstep(0.3*beachTop,beachTop,h); grass and snow move to sand layer below shore | 0.3 / beachTop | packages/mapspinner/src/tsl/surface-splat-tsl.js:103-packages/mapspinner/src/tsl/surface-splat-tsl.js:105 | absent | 2 | open |
| mat:splat.norm | weights normalised by sum+eps | 1e-4 | packages/mapspinner/src/tsl/surface-splat-tsl.js:106; packages/mapspinner/src/splat-weights.js:13 | absent | 2 | open |
| mat:splat.top2 | top two layers by weight (A,B); crossfade bAB = wA/(wA+wB); B sampled only if wB>0.02 | 0.02 | packages/mapspinner/src/tsl/surface-splat-tsl.js:109-packages/mapspinner/src/tsl/surface-splat-tsl.js:116,packages/mapspinner/src/tsl/surface-splat-tsl.js:122,packages/mapspinner/src/tsl/surface-splat-tsl.js:160 | absent | 2 | open |
| mat:splat.xsoft | xSoft crossfade smoothstep half-width | 0.26 | packages/mapspinner/src/terrain-defaults.js:44; packages/mapspinner/src/tsl/surface-splat-tsl.js:163 | absent | 2 | open |
| mat:splat.xfinger | xFinger displacement fingering | 0.2 | packages/mapspinner/src/terrain-defaults.js:45; packages/mapspinner/src/tsl/surface-splat-tsl.js:162 | absent | 2 | open |
| mat:splat.ordpush | ordPush layer-order push | 0.0 | packages/mapspinner/src/terrain-defaults.js:46; packages/mapspinner/src/tsl/surface-splat-tsl.js:163 | absent | 2 | open |
| mat:splat.ord | layer order values grass/rock/sand/snow | 0.6/0.3/0.0/1.0 | packages/mapspinner/src/tsl/surface-splat-tsl.js:144 | absent | 2 | open |
| mat:splat.xfade | crossFade distance 1-smoothstep(xFade0,xFade1,camDist), m | 100 / 340 | packages/mapspinner/src/terrain-defaults.js:49-packages/mapspinner/src/terrain-defaults.js:50; packages/mapspinner/src/tsl/surface-splat-tsl.js:129 | absent | 2 | open |
| mat:splat.mix | texMix global textured amount | 1.0 | packages/mapspinner/src/terrain-defaults.js:36; packages/mapspinner/src/tsl/surface-splat-tsl.js:28 | absent | 2 | open |
| mat:splat.matcolor | per-layer material colour used far/blend | grass=bcGrass, rock=bcRock(runtime), sand=bcShore, snow=bcSnow | packages/mapspinner/src/tsl/surface-splat-tsl.js:143 | absent | 2 | open |
| mat:tri.sharp | triplanar weight exponent pow(abs(n),triSharp), normalised | 3.0 | packages/mapspinner/src/terrain-defaults.js:39; packages/mapspinner/src/tsl/surface-splat-tsl.js:120-packages/mapspinner/src/tsl/surface-splat-tsl.js:121 | absent: no texture | 2 | open |
| mat:tri.planes | three plane taps yz/xz/xy weighted by bw.x/y/z, explicit gradients dFdx/dFdy | planeTap(tex, g, 'yz'/'xz'/'xy', layer) weighted bw.x, bw.y, bw.z; texture(tex, g.p[uvSw]).grad(g.dx[uvSw], g.dy[uvSw]) (3 taps) | packages/mapspinner/src/tsl/surface-splat-tsl.js:63 | absent | 2 | open |
| mat:tri.fine | fine octave scale on world coords | x4.0 | packages/mapspinner/src/tsl/surface-splat-tsl.js:123 | absent | 2 | open |
| mat:tri.coarse | coarse octave scale | x1.0 | packages/mapspinner/src/tsl/surface-splat-tsl.js:123 | absent | 2 | open |
| mat:tri.nrm | triplanar normal reconstruct from rg*2-1 sign-by-axis | planeTap(...).rg.mul(2.0).sub(1.0) per plane, weighted bw.x.mul(sign(n.x)), bw.y.mul(sign(n.y)), bw.z.mul(sign(n.z)) | packages/mapspinner/src/tsl/surface-splat-tsl.js:77 | absent | 2 | open |
| mat:tile.m | texTile base metres, scaled by R/PLANET_RADIUS_M | 2400 | packages/mapspinner/src/terrain-defaults.js:57; packages/mapspinner/src/tsl/surface-splat-tsl.js:118; packages/mapspinner/src/tsl/planet-tsl.js:242 | absent | 2 | open |
| mat:tile.wrap | texture camera-origin wrap = texTileM*8 | x8 | packages/mapspinner/src/tsl/planet-tsl.js:243-packages/mapspinner/src/tsl/planet-tsl.js:244 | absent | 2 | open |
| mat:tile.warp.freq | texture-coordinate warp noise frequency (vWarp) | 450 | packages/mapspinner/src/tsl/terrain-material-tsl.js:23; packages/mapspinner/src/tsl/terrain-material-tsl.js:181 | absent | 2 | open |
| mat:tile.warp.amp | texture warp amplitude | 1.2 | packages/mapspinner/src/tsl/terrain-material-tsl.js:24; packages/mapspinner/src/tsl/terrain-material-tsl.js:187 | absent | 2 | open |
| mat:tile.warp.mix | texWarp multiplier | 1.0 | packages/mapspinner/src/terrain-defaults.js:43; packages/mapspinner/src/tsl/surface-splat-tsl.js:29,packages/mapspinner/src/tsl/surface-splat-tsl.js:119 | absent | 2 | open |
| mat:tex.far | texFarFade = 1-smoothstep(rs*texFar0, rs*texFar1, pxWorld) | 0 / 10000 | packages/mapspinner/src/terrain-defaults.js:51-packages/mapspinner/src/terrain-defaults.js:52; packages/mapspinner/src/tsl/surface-splat-tsl.js:88 | absent | 2 | open |
| mat:tex.oct.far | octFarFade fine->coarse blend smoothstep(rs*octFar0, rs*octFar1, pxWorld) | 1.5 / 15 | packages/mapspinner/src/terrain-defaults.js:55-packages/mapspinner/src/terrain-defaults.js:56; packages/mapspinner/src/tsl/surface-splat-tsl.js:127 | absent | 2 | open |
| mat:tex.oct.coincide | octaves coincide when pxWorld in [texTileM/texWidth, 0.25*texTileM] and collapse>0.5 | 0.25 | packages/mapspinner/src/tsl/surface-splat-tsl.js:124-packages/mapspinner/src/tsl/surface-splat-tsl.js:126 | absent | 2 | open |
| mat:tex.nrm.fade | texFade normal-map fade 1-smoothstep(nrmFade0,nrmFade1,camDist), m | 100 / 1000 | packages/mapspinner/src/terrain-defaults.js:53-packages/mapspinner/src/terrain-defaults.js:54; packages/mapspinner/src/tsl/surface-splat-tsl.js:128 | absent | 2 | open |
| mat:tex.nrm.k | texNrmK detail normal strength | 0.4 | packages/mapspinner/src/terrain-defaults.js:37; packages/mapspinner/src/tsl/surface-splat-tsl.js:174 | absent | 2 | open |
| mat:tex.nrm.low | nrmLow weight of coarse normal; coarse mul 1.7*nrmLow | 0.4 / 1.7 | packages/mapspinner/src/terrain-defaults.js:38; packages/mapspinner/src/tsl/surface-splat-tsl.js:141 | absent | 2 | open |
| mat:tex.sat | texSat saturation of sampled colour | 1.2 | packages/mapspinner/src/terrain-defaults.js:40; packages/mapspinner/src/tsl/surface-splat-tsl.js:146 | absent | 2 | open |
| mat:tex.bright | texBright final multiplier | 1.12 | packages/mapspinner/src/terrain-defaults.js:41; packages/mapspinner/src/tsl/surface-splat-tsl.js:172 | absent | 2 | open |
| mat:tex.biometint | biomeTint = 0.62*(1-0.85*clamp(w4.z)) | 0.62 / 0.85 | packages/mapspinner/src/terrain-defaults.js:42; packages/mapspinner/src/tsl/surface-splat-tsl.js:171 | absent | 2 | open |
| mat:tex.luma | LUMA weights for detail normalisation | [0.299,0.587,0.114] | packages/mapspinner/src/tsl/surface-splat-tsl.js:11 | absent | 2 | open |
| mat:tex.detail | detail = sat * dot(matColor,LUMA)/max(meanL,0.02) | 0.02 | packages/mapspinner/src/tsl/surface-splat-tsl.js:147 | absent | 2 | open |
| mat:tex.albedo.mix | albedo0 = clamp(mix(matColor,detail,k),0,1); k=texFarFade*texMix*surfReady | k = texFarFade.mul(splatMix).mul(u.surfReady); albedo0 = clamp(mix(matColor, detail, k), vec3(0.0), vec3(1.0)) | packages/mapspinner/src/tsl/surface-splat-tsl.js:170 | absent | 2 | open |
| mat:tex.photo | texPhoto and texPhotoNear (declared, unused in surface-splat) | 0.0 / 0.0 | packages/mapspinner/src/terrain-defaults.js:47-packages/mapspinner/src/terrain-defaults.js:48 | absent | 2 | open |
| mat:tex.files | 12 jpg files: grass/rock/sand/snow-color, -displacement, grass/sand/snow-normals, ground-normals | SPLAT_LAYER_MATS = ['grass', 'rock', 'sand', 'snow'] with '-color.jpg' and '-displacement.jpg'; NRM_JPGS = ['grass-normals.jpg', 'ground-normals.jpg', 'sand-normals.jpg', 'snow-normals.jpg'] (12 files) | packages/mapspinner/src/surface-texture-decode.js:47 | absent: no textures | 2 | open |
| mat:tex.size | every texture decoded to SZ x SZ (source files are 1024x1024) | 1024 | packages/mapspinner/src/surface-texture-decode.js:49 | absent | 2 | open |
| mat:tex.count | layers in arrays (matCount) | 4 | packages/mapspinner/src/surface-texture-decode.js:47,packages/mapspinner/src/surface-texture-decode.js:136 | absent | 2 | open |
| mat:tex.normalfiles | normal map per layer; rock layer uses ground-normals.jpg | grass-,ground-,sand-,snow-normals.jpg | packages/mapspinner/src/surface-texture-decode.js:48 | absent | 2 | open |
| mat:tex.pack.alb | albedo array RGB=color, A=displacement(R of displacement jpg) | albAll[o] = c[i * 4]; albAll[o + 1] = c[i * 4 + 1]; albAll[o + 2] = c[i * 4 + 2]; albAll[o + 3] = d[i * 4] (RGBA8) | packages/mapspinner/src/surface-texture-decode.js:89 | absent | 2 | open |
| mat:tex.pack.nrm | normal array RG=normals, B=displacement, A=255 | nrmAll[o] = nj[i * 4]; nrmAll[o + 1] = nj[i * 4 + 1]; nrmAll[o + 2] = d[i * 4]; nrmAll[o + 3] = 255 (RGBA8) | packages/mapspinner/src/surface-texture-decode.js:90 | absent | 2 | open |
| mat:tex.srgb | albedo sRGB, normal NoColorSpace | sRGB / none | packages/mapspinner/src/tsl/terrain-material-tsl.js:53-packages/mapspinner/src/tsl/terrain-material-tsl.js:55 | absent | 2 | open |
| mat:tex.filter | wrap Repeat, mag Linear, min LinearMipmapLinear, mipmaps on, anisotropy 8 | 8x | packages/mapspinner/src/tsl/terrain-material-tsl.js:33-packages/mapspinner/src/tsl/terrain-material-tsl.js:37 | absent | 2 | open |
| mat:tex.placeholder | placeholder 1x1 mid-grey 128 and flat normal (128,128,255,255), 4 layers, until decode | sz 1 | packages/mapspinner/src/tsl/terrain-material-tsl.js:83 | absent | 2 | open |
| mat:tex.gamma | LIN8 table gamma used for decode | 2.2 | packages/mapspinner/src/surface-texture-decode.js:20 | absent | 2 | open |
| mat:tex.deshade.exp | deshade gain exponent | 0.8 | packages/mapspinner/src/surface-texture-decode.js:39 | absent | 2 | open |
| mat:tex.deshade.min | deshade gain min | 0.4 | packages/mapspinner/src/surface-texture-decode.js:40 | absent | 2 | open |
| mat:tex.deshade.max | deshade gain max | 2.5 | packages/mapspinner/src/surface-texture-decode.js:41 | absent | 2 | open |
| mat:tex.deshade.grid | deshade blur grid G | 32 | packages/mapspinner/src/surface-texture-decode.js:56 | absent | 2 | open |
| mat:tex.contrast | fine contrast stretch | 1.35 | packages/mapspinner/src/surface-texture-decode.js:42,packages/mapspinner/src/surface-texture-decode.js:79 | absent | 2 | open |
| mat:tex.synth.S | normal synthesis from displacement when normals jpg missing: fine gradient scale | 2.2 | packages/mapspinner/src/surface-texture-decode.js:93,packages/mapspinner/src/surface-texture-decode.js:107 | absent (all four have a normals file) | 2 | open |
| mat:tex.synth.mid | synthesised normal mid/low gradient taps and weights | offset 6 w2.5; offset 48 w2.0 | packages/mapspinner/src/surface-texture-decode.js:103-packages/mapspinner/src/surface-texture-decode.js:107 | absent | 2 | open |
| mat:tex.synth.clamp | synthesised normal xy clamp | 0.9 | packages/mapspinner/src/surface-texture-decode.js:109 | absent | 2 | open |
| mat:tex.rockmean | rock layer mean linear colour becomes bcRock | ROCK_LAYER_INDEX 1 | packages/mapspinner/src/surface-texture-decode.js:43,packages/mapspinner/src/surface-texture-decode.js:120-packages/mapspinner/src/surface-texture-decode.js:125; packages/mapspinner/src/tsl/planet-tsl.js:152 | absent | 2 | open |
| mat:tex.meanl | per-layer mean luminance (0.2126/0.7152/0.0722) -> uniform meanL; default (0.2,0.2,0.2,0.5) | 4 values | packages/mapspinner/src/surface-texture-decode.js:127-packages/mapspinner/src/surface-texture-decode.js:133; packages/mapspinner/src/tsl/terrain-material-tsl.js:100 | absent | 2 | open |
| mat:tex.decode.timeout | surface decode worker timeout | 60000 ms | packages/mapspinner/src/tsl/planet-tsl.js:29 | absent | 2 | open |
| mat:tex.load.gate | surface textures loaded only if loadSurfaceTextures!==false and canDecodeImages | opts.loadSurfaceTextures !== false && canDecodeImages() | packages/mapspinner/src/tsl/planet-tsl.js:141 | absent | 2 | open |
| mat:pool.disp.lo | poolDispLo per layer grass/rock/sand/snow | [0.455,0.408,0.275,0.404] | packages/mapspinner/src/terrain-defaults.js:59; packages/mapspinner/src/tsl/surface-splat-tsl.js:149 | absent | 2 | open |
| mat:pool.disp.hi | poolDispHi per layer | [0.498,0.545,0.388,0.486] | packages/mapspinner/src/terrain-defaults.js:60; packages/mapspinner/src/tsl/surface-splat-tsl.js:149 | absent | 2 | open |
| mat:pool.cover | poolCover default | 0.18 | packages/mapspinner/src/terrain-defaults.js:61; packages/mapspinner/src/tsl/surface-splat-tsl.js:169 | absent | 2 | open |
| mat:pool.spec.rough | poolSpecExpRough | 24 | packages/mapspinner/src/terrain-defaults.js:62 | absent | 2 | open |
| mat:pool.spec.sharp | poolSpecExpSharp | 256 | packages/mapspinner/src/terrain-defaults.js:63 | absent | 2 | open |
| mat:pool.slope | poolSlope0/1: pool *= 1-smoothstep(s0,s1,slope) | 0.08 / 0.25 | packages/mapspinner/src/terrain-defaults.js:64-packages/mapspinner/src/terrain-defaults.js:65; packages/mapspinner/src/tsl/terrain-material-tsl.js:203 | absent | 2 | open |
| mat:pool.wet | wetness runtime clamp 0..1 (default 0); pool applies only on land h>0 | 0 | packages/mapspinner/src/pool-params.js:31-packages/mapspinner/src/pool-params.js:33; packages/mapspinner/src/tsl/terrain-material-tsl.js:196 | absent | 2 | open |
| mat:pool.wetdark | WET_DARKEN albedo multiplier mix(1,0.65,wet) | 0.65 | packages/mapspinner/src/tsl/terrain-material-tsl.js:19,packages/mapspinner/src/tsl/terrain-material-tsl.js:205 | absent | 2 | open |
| mat:pool.fresnel | pool Fresnel pow5 *0.98+0.02 | 5 / 0.98 / 0.02 | packages/mapspinner/src/tsl/terrain-lighting-tsl.js:44 | absent | 2 | open |
| mat:pool.speclobe | spec = pow(NdH,specExp)*(specExp+8)/(8pi) | 8 / 8pi | packages/mapspinner/src/tsl/terrain-lighting-tsl.js:56 | absent | 2 | open |
| mat:pbr.metalness | material metalness | 0.0 | packages/mapspinner/src/tsl/terrain-material-tsl.js:224 | StandardMaterial default metallic (not set) src/globe.rs:506 | 2 | open |
| mat:pbr.roughness | roughness: not set; custom Lambert + pool-spec lighting model replaces PBR | material.setupLightingModel = () => new lighting.LightingModel() (no roughness set) | packages/mapspinner/src/tsl/terrain-material-tsl.js:217 | perceptual_roughness 0.95 (src/globe.rs:506) | 2 | open |
| mat:pbr.basecolor | material base colour (vertex colour/colorNode carries albedo) | colorNode | packages/mapspinner/src/tsl/terrain-material-tsl.js:220 | base_color WHITE (src/globe.rs:506), vertex colours (src/globe.rs:501) | 2 | open |
| mat:pbr.side | material side | BackSide | packages/mapspinner/src/tsl/terrain-material-tsl.js:226 | default culling | 2 | open |
| mat:pbr.fog | material fog off | false | packages/mapspinner/src/tsl/terrain-material-tsl.js:225 | scene default | 2 | open |
| mat:norm.up | normal pulled toward up | 0.05 | packages/mapspinner/src/tsl/terrain-material-tsl.js:20,packages/mapspinner/src/tsl/terrain-material-tsl.js:192 | smooth normals src/globe.rs:500 | 2 | open |
| mat:norm.relief | reliefShade: nRelief = normalize(dir0+(n-dir0)*reliefShade) when >1 | 6.0 | packages/mapspinner/src/terrain-defaults.js:21; packages/mapspinner/src/tsl/terrain-material-tsl.js:207 | absent | 2 | open |
| mat:norm.flat | flatNormal | 0.0 | packages/mapspinner/src/terrain-defaults.js:34; packages/mapspinner/src/tsl/terrain-material-tsl.js:192 | absent | 2 | open |
| mat:norm.lit | nLit = normalize(nRelief + texDn) | nLit = normalize(nRelief.add(texDnP)) | packages/mapspinner/src/tsl/terrain-material-tsl.js:208 | absent | 2 | open |
| mat:light.exposure | exposure | 1.05 | packages/mapspinner/src/terrain-defaults.js:20; packages/mapspinner/src/tsl/terrain-lighting-tsl.js:92 | absent | 2 | open |
| mat:light.skyfill | skyFill | 0.3 | packages/mapspinner/src/terrain-defaults.js:22; packages/mapspinner/src/tsl/terrain-lighting-tsl.js:34 | absent | 2 | open |
| mat:light.hazemul | hazeMul | 0.4 | packages/mapspinner/src/terrain-defaults.js:24; packages/mapspinner/src/tsl/terrain-lighting-tsl.js:90 | absent | 2 | open |
| mat:light.vertexao | vertexAO | 0.0 | packages/mapspinner/src/terrain-defaults.js:25 | absent | 2 | open |
| mat:light.aoamt | aoAmt | 0.0 | packages/mapspinner/src/terrain-defaults.js:26 | absent | 2 | open |
| mat:light.biomewarp | biomeWarp (debug biome colours) | 1.1 | packages/mapspinner/src/terrain-defaults.js:27; packages/mapspinner/src/shaders/terrain.glsl:948 | absent | 2 | open |
| mat:light.nightfloor | nightFloor | 0.18 | packages/mapspinner/src/terrain-defaults.js:28; packages/mapspinner/src/tsl/terrain-lighting-tsl.js:91 | absent | 2 | open |
| mat:light.termwidth | termWidth | 0.45 | packages/mapspinner/src/terrain-defaults.js:29; packages/mapspinner/src/tsl/terrain-lighting-tsl.js:42 | absent | 2 | open |
| mat:light.termglow | terminatorGlow | 0.6 | packages/mapspinner/src/terrain-defaults.js:30; packages/mapspinner/src/tsl/terrain-lighting-tsl.js:88 | absent | 2 | open |
| mat:light.nightlights | nightLights | 0.8 | packages/mapspinner/src/terrain-defaults.js:31; packages/mapspinner/src/tsl/terrain-lighting-tsl.js:92 | absent | 2 | open |
| mat:light.looksat | lookSat | 1.25 | packages/mapspinner/src/terrain-defaults.js:32; packages/mapspinner/src/tsl/terrain-lighting-tsl.js:94 | absent | 2 | open |
| mat:light.lookcontrast | lookContrast | 1.0 | packages/mapspinner/src/terrain-defaults.js:33; packages/mapspinner/src/tsl/terrain-lighting-tsl.js:95 | absent | 2 | open |
| mat:light.sungain | SUN_IRRADIANCE_GAIN | 1.25 | packages/mapspinner/src/tsl/terrain-ambient-shared.js:3; packages/mapspinner/src/tsl/terrain-lighting-tsl.js:57 | absent | 2 | open |
| mat:light.skyscale | SKY_IRRADIANCE_SCALE | 0.075 | packages/mapspinner/src/tsl/terrain-ambient-shared.js:4 | absent | 2 | open |
| mat:light.skygrey | SKY_BALANCE_TO_GREY | 0.35 | packages/mapspinner/src/tsl/terrain-ambient-shared.js:5 | absent | 2 | open |
| mat:light.skytint | SKY_FILL_TINT | [0.85,0.92,1.10] | packages/mapspinner/src/tsl/terrain-ambient-shared.js:6 | absent | 2 | open |
| mat:light.termtint | TERMINATOR_TINT | [1.0,0.55,0.34] | packages/mapspinner/src/tsl/terrain-ambient-shared.js:7 | absent | 2 | open |
| mat:light.nightfill | NIGHT_FILL | [0.06,0.075,0.11] | packages/mapspinner/src/tsl/terrain-ambient-shared.js:8 | absent | 2 | open |
| mat:light.floor.albedo | AMBIENT_FLOOR_ALBEDO | 0.14 | packages/mapspinner/src/tsl/terrain-ambient-shared.js:9 | absent | 2 | open |
| mat:light.floor.add | AMBIENT_FLOOR_ADD | [0.020,0.026,0.038] | packages/mapspinner/src/tsl/terrain-ambient-shared.js:10 | absent | 2 | open |
| mat:light.apgate | AP_GATE_KM aerial perspective gate | [3,120] | packages/mapspinner/src/tsl/terrain-ambient-shared.js:11; packages/mapspinner/src/tsl/terrain-lighting-tsl.js:76 | absent | 2 | open |
| mat:light.sundark | SUN_DARK_LUMINANCE | 1e-4 | packages/mapspinner/src/tsl/terrain-ambient-shared.js:12 | absent | 2 | open |
| mat:light.raytint | RAYLEIGH_SKY_TINT | 0.4 | packages/mapspinner/src/tsl/terrain-ambient-shared.js:14 | absent | 2 | open |
| mat:light.skyday | SKY_DAY_MU | [-0.10,0.25] | packages/mapspinner/src/tsl/terrain-ambient-shared.js:15 | absent | 2 | open |
| mat:light.termfade | TERMINATOR_FADE_SIN = sin(10 deg) | sin(10deg) | packages/mapspinner/src/tsl/terrain-ambient-shared.js:16 | absent | 2 | open |
| mat:light.haze | SKY_HAZE_TINT | [0.40,0.55,0.78] | packages/mapspinner/src/tsl/terrain-lighting-tsl.js:12 | absent | 2 | open |
| mat:light.graze | terminator glow graze smoothstep(0.55,1.0,1-/nwSun/), squared; day smoothstep(-0.02,0.18) | 0.55 / 1.0 / -0.02 / 0.18 | packages/mapspinner/src/tsl/terrain-lighting-tsl.js:87-packages/mapspinner/src/tsl/terrain-lighting-tsl.js:88 | absent | 2 | open |
| mat:light.ambirr | ambIrr from timeOfDay ambientColor*intensity/pi | intensity/PI | packages/mapspinner/src/tsl/terrain-lighting-tsl.js:113 | Bevy ambient light default | 2 | open |
| mat:light.aces | ACES filmic coefficients | [2.51,0.03,2.43,0.59,0.14] | packages/mapspinner/src/tsl/terrain-ambient-shared.js:24; packages/mapspinner/src/tsl/terrain-lighting-tsl.js:93 | Bevy tonemapper default | 2 | open |
| mat:light.luma | luma weights for saturation | [0.2126,0.7152,0.0722] | packages/mapspinner/src/tsl/terrain-ambient-shared.js:18; packages/mapspinner/src/tsl/terrain-lighting-tsl.js:94 | absent | 2 | open |
| mat:light.gamma | display gamma pow(1/2.2) then display-referred to scene-linear | 1/2.2 | packages/mapspinner/src/tsl/terrain-ambient-shared.js:25; packages/mapspinner/src/tsl/terrain-lighting-tsl.js:96 | absent | 2 | open |
| mat:light.glsl.skyao | skyAO and ambient-floor AO mix(0.45,1.0,skyAO) (skyAO fixed 1.0 in GLSL) | 1.0 / 0.45 | packages/mapspinner/src/shaders/terrain.glsl:995-packages/mapspinner/src/shaders/terrain.glsl:996 | absent | 2 | open |
| mat:light.glsl.limb | limb factor 0.45+0.55*smoothstep(0,0.45,viewGraze) (computed, not applied to colour in lines read) | 0.45 / 0.55 | packages/mapspinner/src/shaders/terrain.glsl:1055 | absent | 2 | open |
| mat:uw.sun | underwater sun irradiance colour, atten exp(-depth*0.0004), x0.7 | [1.0,0.65,0.35] | packages/mapspinner/src/shaders/terrain.glsl:984-packages/mapspinner/src/shaders/terrain.glsl:986 | absent | 2 | open |
| mat:uw.sky | underwater sky irradiance | [0.02,0.07,0.18] | packages/mapspinner/src/shaders/terrain.glsl:987 | absent | 2 | open |
| mat:uw.absorb | underwater absorb and minTau | [0.035,0.010,0.005] / [0.45,0.30,0.20] | packages/mapspinner/src/shaders/terrain.glsl:1026-packages/mapspinner/src/shaders/terrain.glsl:1027 | absent | 2 | open |
| mat:uw.fog | underwater fog colour | [0.004,0.09,0.18]+[0,0.015,0.03]*depth/1000 | packages/mapspinner/src/shaders/terrain.glsl:1029 | absent | 2 | open |
| mat:uw.fogrange | underwater fog full range | 100000..500000 (dKm*1000) | packages/mapspinner/src/shaders/terrain.glsl:1032 | absent | 2 | open |
| mat:uw.upfill | underwater upFill and gain | mix(0.75,1.35); x1.5 | packages/mapspinner/src/shaders/terrain.glsl:1030-packages/mapspinner/src/shaders/terrain.glsl:1031 | absent | 2 | open |
| mat:glsl.terrainwidthk | legacy GLSL ambient floor and 1.25 sun gain identical to TSL | 0.14 / 1.25 | packages/mapspinner/src/shaders/terrain.glsl:996-packages/mapspinner/src/shaders/terrain.glsl:997 | absent | 2 | open |
| mat:glsl.texlodbias | TEX_LOD_BIAS in legacy GLSL | 0.0 | packages/mapspinner/src/shaders/terrain.glsl:649 | absent | 2 | open |
| mat:glsl.debug.biome | debug biome class colours (display mode only) | e.g. sea (0.10,0.30,0.75) | packages/mapspinner/src/shaders/terrain.glsl:584-packages/mapspinner/src/shaders/terrain.glsl:595 | absent (debug only) | 2 | open |

## Water, atmosphere, sky, lighting and shadows

| id | rule or number | reference value | source (path:line in /workspace/spoint) | port today | stage | status |
|---|---|---|---|---|---|---|
| wsl:WSL-001 | water grid resolution per patch | WATER_GRID = 4 | packages/mapspinner/src/tsl/water-tsl.js:13 | none | 2 | open |
| wsl:WSL-002 | water skirt | false | packages/mapspinner/src/tsl/water-tsl.js:14 | none | 2 | open |
| wsl:WSL-003 | water max quadtree level | 11 | packages/mapspinner/src/tsl/water-tsl.js:15 | none | 2 | open |
| wsl:WSL-004 | water instance key span | 4096 | packages/mapspinner/src/tsl/water-tsl.js:16 | none | 2 | open |
| wsl:WSL-005 | cube-face tan warp constant (pi/4) | 0.7853981634 | packages/mapspinner/src/tsl/water-tsl.js:17 | none | 2 | open |
| wsl:WSL-006 | wave slope finite-difference eps (HF) | 0.25 m | packages/mapspinner/src/tsl/water-tsl.js:18 | none | 2 | open |
| wsl:WSL-007 | wave slope finite-difference eps (LF) | 1.25 m | packages/mapspinner/src/tsl/water-tsl.js:19 | none | 2 | open |
| wsl:WSL-008 | raymarch near distance | 140 m | packages/mapspinner/src/tsl/water-tsl.js:20 | none | 2 | open |
| wsl:WSL-009 | raymarch linear steps | 5 | packages/mapspinner/src/tsl/water-tsl.js:21 | none | 2 | open |
| wsl:WSL-010 | raymarch bisection max iterations | 5 | packages/mapspinner/src/tsl/water-tsl.js:22 | none | 2 | open |
| wsl:WSL-011 | wave tile wrap period | 1024 m | packages/mapspinner/src/tsl/water-tsl.js:23 | none | 2 | open |
| wsl:WSL-012 | underwater switch: camera below sea by | 2 m | packages/mapspinner/src/tsl/water-tsl.js:24 | none | 2 | open |
| wsl:WSL-013 | land-above-sea discard threshold | 1.0 m | packages/mapspinner/src/tsl/water-tsl.js:25 | none | 2 | open |
| wsl:WSL-014 | default ocean amplitude / choppiness / foam | 1.0 / 0.5 / 0.5 | packages/mapspinner/src/tsl/water-tsl.js:26 | none | 2 | open |
| wsl:WSL-015 | water display ACES coefficients and gamma | 2.51, 0.03, 2.43, 0.59, 0.14; pow 1/2.2 | packages/mapspinner/src/tsl/water-tsl.js:28 | none | 2 | open |
| wsl:WSL-016 | land probe key quantisation | 1.0 m | packages/mapspinner/src/tsl/water-tsl.js:30 | none | 2 | open |
| wsl:WSL-017 | land memo cap | 100000 points | packages/mapspinner/src/tsl/water-tsl.js:31 | none | 2 | open |
| wsl:WSL-018 | fragment discard rule | discard when composeHeight > LAND_ABOVE_SEA_DISCARD_M | packages/mapspinner/src/tsl/water-tsl.js:75 | none | 2 | open |
| wsl:WSL-019 | underwater depth attenuation | exp(max(0,-camAlt) * -0.0005) | packages/mapspinner/src/tsl/water-tsl.js:97 | none | 2 | open |
| wsl:WSL-020 | underwater body colour | base (0.005,0.06,0.18) + (1,0.6,0.3)*atten*ndl*0.4 + (0,0.02,0.06)*\|slope\| | packages/mapspinner/src/tsl/water-tsl.js:98 | none | 2 | open |
| wsl:WSL-021 | underwater snell window | smoothstep(0.50,0.82,\|dot(view,n)\|) | packages/mapspinner/src/tsl/water-tsl.js:99 | none | 2 | open |
| wsl:WSL-022 | underwater window colour | (0.40,0.60,0.85)*(ndl*0.9+0.5), mix weight snell*0.85 | packages/mapspinner/src/tsl/water-tsl.js:100 | none | 2 | open |
| wsl:WSL-023 | underwater sun glint | colour (1,0.92,0.70), pow 180, weight snell*0.6+0.4 | packages/mapspinner/src/tsl/water-tsl.js:101 | none | 2 | open |
| wsl:WSL-024 | bisection count falloff | int(clamp(5 - (d-30)/36, 2, 5)) | packages/mapspinner/src/tsl/water-tsl.js:119 | none | 2 | open |
| wsl:WSL-025 | near wave fade | clamp(1 - tHit/120, 0, 1) | packages/mapspinner/src/tsl/water-tsl.js:129 | none | 2 | open |
| wsl:WSL-026 | far LF wave fade | clamp((tHit-30)/60,0,1) * clamp(1 - tHit/300,0,1) | packages/mapspinner/src/tsl/water-tsl.js:130 | none | 2 | open |
| wsl:WSL-027 | near octave start | int(clamp(2 - tHit/40, 0, 2)) | packages/mapspinner/src/tsl/water-tsl.js:131 | none | 2 | open |
| wsl:WSL-028 | near octave end | int(clamp(8 - tHit/60, 6, 8)) | packages/mapspinner/src/tsl/water-tsl.js:132 | none | 2 | open |
| wsl:WSL-029 | HF slope weight by depth | clamp(-composeHeight/6, 0, 1) | packages/mapspinner/src/tsl/water-tsl.js:135 | none | 2 | open |
| wsl:WSL-030 | sky reflection zenith / horizon colours | (0.15,0.45,0.95) / (0.55,0.75,1.00) | packages/mapspinner/src/tsl/water-tsl.js:137 | none | 2 | open |
| wsl:WSL-031 | sky reflection horizon exponent | pow(1 - max(dot(refl,up),0), 3) | packages/mapspinner/src/tsl/water-tsl.js:140 | none | 2 | open |
| wsl:WSL-032 | sun glint on sky reflection | (1,0.9,0.7) pow(reflSun,8)*0.8 + (1,0.85,0.55) pow(reflSun,16)*0.4 | packages/mapspinner/src/tsl/water-tsl.js:141 | none | 2 | open |
| wsl:WSL-033 | deep water colour | (0.04,0.34,0.52) | packages/mapspinner/src/tsl/water-tsl.js:144 | none | 2 | open |
| wsl:WSL-034 | refraction fade range | 1 - smoothstep(250,350,tHit) | packages/mapspinner/src/tsl/water-tsl.js:145 | none | 2 | open |
| wsl:WSL-035 | refraction UV distortion | slope * refrW * NoV * 0.36; UV clamp 0.001..0.999 | packages/mapspinner/src/tsl/water-tsl.js:146 | none | 2 | open |
| wsl:WSL-036 | refraction black fallback threshold | dot(scene,1) <= 0.01 -> deepColor | packages/mapspinner/src/tsl/water-tsl.js:148 | none | 2 | open |
| wsl:WSL-037 | absorption coefficients | (0.09,0.028,0.012) per m, depth/max(NoV,0.10) | packages/mapspinner/src/tsl/water-tsl.js:149 | none | 2 | open |
| wsl:WSL-038 | water body mix | refr*absorb*0.40 + deep*(1-absorb) | packages/mapspinner/src/tsl/water-tsl.js:150 | none | 2 | open |
| wsl:WSL-039 | fresnel | pow(1-NoV,5)*0.98+0.02 | packages/mapspinner/src/tsl/water-tsl.js:151 | none | 2 | open |
| wsl:WSL-040 | reflection weight | fresnel * (1 - mean(absorb)) | packages/mapspinner/src/tsl/water-tsl.js:152 | none | 2 | open |
| wsl:WSL-041 | water final mix and sun spec | mix(deep,skyRefl,0.7); + pow(reflSun,64)*0.5 | packages/mapspinner/src/tsl/water-tsl.js:153 | none | 2 | open |
| wsl:WSL-042 | foam noise scale/drift | scale 0.7, drift (0.02,-0.03)*t | packages/mapspinner/src/tsl/water-tsl.js:154 | none | 2 | open |
| wsl:WSL-043 | foam UV scale/drift | scale 4.0, drift (1.4,0.9)*t | packages/mapspinner/src/tsl/water-tsl.js:155 | none | 2 | open |
| wsl:WSL-044 | foam patch mask | mix(noise, mix(oct(uv,choppy), oct(uv*0.5+(1.3,2.7), choppy*0.7), 0.4), 0.6) | packages/mapspinner/src/tsl/water-tsl.js:156 | none | 2 | open |
| wsl:WSL-045 | foam crest threshold | clamp((\|slope\|-0.45)*2,0,1) | packages/mapspinner/src/tsl/water-tsl.js:157 | none | 2 | open |
| wsl:WSL-046 | foam amount | crest*(mask*0.6+0.4)*foam | packages/mapspinner/src/tsl/water-tsl.js:158 | none | 2 | open |
| wsl:WSL-047 | foam colour and lighting | (0.95,0.98,1.0), weight foam*(NoL*0.35+0.15) | packages/mapspinner/src/tsl/water-tsl.js:159 | none | 2 | open |
| wsl:WSL-048 | water distance haze | mix(wcol, mix(horizon,zenith,..)*0.5, tHit/(tHit+R)*hazeMul) | packages/mapspinner/src/tsl/water-tsl.js:161 | none | 2 | open |
| wsl:WSL-049 | water haze weight | tHit/(tHit+max(R,1)) | packages/mapspinner/src/tsl/water-tsl.js:160 | none | 2 | open |
| wsl:WSL-050 | water material flags | transparent, depthWrite, DoubleSide, forceSinglePass | packages/mapspinner/src/tsl/water-tsl.js:171 | none | 2 | open |
| wsl:WSL-051 | water side | DoubleSide | packages/mapspinner/src/tsl/water-tsl.js:174 | none | 2 | open |
| wsl:WSL-052 | water castShadow/receiveShadow | false / false | packages/mapspinner/src/tsl/water-tsl.js:182 | none | 2 | open |
| wsl:WSL-053 | hazeMul runtime default | TD.hazeMul (window.__hazeMul override) | packages/mapspinner/src/tsl/water-tsl.js:284 | none | 2 | open |
| wsl:WSL-054 | seaMean subtracted from wave height | seaHeightMean(choppy, amp) | packages/mapspinner/src/tsl/water-tsl.js:281 | none | 2 | open |
| wsl:WSL-055 | underwater test | camDist < R - UNDERWATER_BELOW_M | packages/mapspinner/src/tsl/water-tsl.js:282 | none | 2 | open |
| wsl:WSL-056 | sea octave count (CPU) | 8 | packages/mapspinner/src/sea-waves.js:1 | none | 2 | open |
| wsl:WSL-057 | sea base amplitude (CPU mean) | 0.6 m | packages/mapspinner/src/sea-waves.js:2 | none | 2 | open |
| wsl:WSL-058 | sea mean phase step | 0.31 | packages/mapspinner/src/sea-waves.js:3 | none | 2 | open |
| wsl:WSL-059 | sea amplitude gain per octave | 0.45 | packages/mapspinner/src/sea-waves.js:4 | none | 2 | open |
| wsl:WSL-060 | sea choppy pull to one | 0.3 | packages/mapspinner/src/sea-waves.js:5 | none | 2 | open |
| wsl:WSL-061 | sea mean sample grid | 256 | packages/mapspinner/src/sea-waves.js:6 | none | 2 | open |
| wsl:WSL-062 | sea mean sample step | 3.7013 | packages/mapspinner/src/sea-waves.js:7 | none | 2 | open |
| wsl:WSL-063 | sea mean cache limit | 16 | packages/mapspinner/src/sea-waves.js:8 | none | 2 | open |
| wsl:WSL-064 | sea hash multipliers (CPU) | 1597334673, -482951495 | packages/mapspinner/src/sea-waves.js:13 | none | 2 | open |
| wsl:WSL-065 | sea octave shape exponent | pow(1 - pow(wx*wy, 0.65), choppy) | packages/mapspinner/src/sea-waves.js:36 | none | 2 | open |
| wsl:WSL-066 | sea octaves (GPU) | 8 | packages/mapspinner/src/tsl/sea-waves-tsl.js:3 | none | 2 | open |
| wsl:WSL-067 | sea base frequency | 0.2 | packages/mapspinner/src/tsl/sea-waves-tsl.js:4 | none | 2 | open |
| wsl:WSL-068 | sea speed | 2.4 | packages/mapspinner/src/tsl/sea-waves-tsl.js:5 | none | 2 | open |
| wsl:WSL-069 | sea amplitude | 0.6 m | packages/mapspinner/src/tsl/sea-waves-tsl.js:6 | none | 2 | open |
| wsl:WSL-070 | sea octave rotation angle | 1.57079633 rad per octave | packages/mapspinner/src/tsl/sea-waves-tsl.js:7 | none | 2 | open |
| wsl:WSL-071 | sea LF frequency | 0.16 | packages/mapspinner/src/tsl/sea-waves-tsl.js:8 | none | 2 | open |
| wsl:WSL-072 | sea hash multipliers (GPU) | 1597334673, 3812015801 | packages/mapspinner/src/tsl/sea-waves-tsl.js:13 | none | 2 | open |
| wsl:WSL-073 | sea octave per-step time scale | ts = 1 - i*0.08 | packages/mapspinner/src/tsl/sea-waves-tsl.js:39 | none | 2 | open |
| wsl:WSL-074 | sea time vector | (t*SPEED*0.6, t*SPEED*0.4) | packages/mapspinner/src/tsl/sea-waves-tsl.js:33 | none | 2 | open |
| wsl:WSL-075 | sea frequency multiplier per octave | 1.9 | packages/mapspinner/src/tsl/sea-waves-tsl.js:42 | none | 2 | open |
| wsl:WSL-076 | sea amplitude multiplier per octave | 0.45 | packages/mapspinner/src/tsl/sea-waves-tsl.js:43 | none | 2 | open |
| wsl:WSL-077 | sea choppy pull per octave | mix(choppy,1,0.3) | packages/mapspinner/src/tsl/sea-waves-tsl.js:44 | none | 2 | open |
| wsl:WSL-078 | sea LF directions | (0.866,0.5) and (-0.5,0.866) | packages/mapspinner/src/tsl/sea-waves-tsl.js:51 | none | 2 | open |
| wsl:WSL-079 | sea LF amplitude | amp * SEA_AMP_M * 0.5 | packages/mapspinner/src/tsl/sea-waves-tsl.js:53 | none | 2 | open |
| wsl:WSL-080 | tone map modes | NONE 0, ACES 4, AGX 6, NEUTRAL 7 | packages/mapspinner/src/tsl/display-referred-tsl.js:3 | none | 2 | open |
| wsl:WSL-081 | display clamp | 0.995 | packages/mapspinner/src/tsl/display-referred-tsl.js:8 | none | 2 | open |
| wsl:WSL-082 | ACES input matrix | [0.59719,0.35458,0.04823],[0.07600,0.90834,0.01566],[0.02840,0.13383,0.83777] | packages/mapspinner/src/tsl/display-referred-tsl.js:24 | none | 2 | open |
| wsl:WSL-083 | ACES output matrix | [1.60475,-0.53108,-0.07367],[-0.10208,1.10813,-0.00605],[-0.00327,-0.07276,1.07602] | packages/mapspinner/src/tsl/display-referred-tsl.js:25 | none | 2 | open |
| wsl:WSL-084 | default tone mapping mode | ACES (renderer.toneMapping) | packages/mapspinner/src/tsl/display-referred-tsl.js:41 | none | 2 | open |
| wsl:WSL-085 | tone mapping exposure | renderer.toneMappingExposure \|\| 1.0 | packages/mapspinner/src/tsl/display-referred-tsl.js:42 | none | 2 | open |
| wsl:WSL-086 | ACES inverse constants | 0.983729, 0.4329510, 0.0245786, 0.238081, 0.000090537; x 0.6/exposure | packages/mapspinner/src/tsl/display-referred-tsl.js:49 | none | 2 | open |
| wsl:WSL-087 | display to scene linear ACES filmic | sRGB->linear, clamp 0.995, invert ACES | packages/mapspinner/src/tsl/display-referred-tsl.js:62 | none | 2 | open |
| wsl:WSL-088 | atmosphere bottom radius | 6360 | packages/mapspinner/src/shaders/atmosphere.glsl:3 | none | 2 | open |
| wsl:WSL-089 | atmosphere top radius | 6500 | packages/mapspinner/src/shaders/atmosphere.glsl:4 | none | 2 | open |
| wsl:WSL-090 | Rayleigh scale height | 18 | packages/mapspinner/src/shaders/atmosphere.glsl:5 | none | 2 | open |
| wsl:WSL-091 | Mie scale height | 4 | packages/mapspinner/src/shaders/atmosphere.glsl:6 | none | 2 | open |
| wsl:WSL-092 | Mie g | 0.8 | packages/mapspinner/src/shaders/atmosphere.glsl:7 | none | 2 | open |
| wsl:WSL-093 | Rayleigh coefficients | (0.005802, 0.013558, 0.0331) | packages/mapspinner/src/shaders/atmosphere.glsl:9 | none | 2 | open |
| wsl:WSL-094 | Mie scattering | 0.003996 | packages/mapspinner/src/shaders/atmosphere.glsl:10 | none | 2 | open |
| wsl:WSL-095 | Mie extinction | 0.003996/0.9 | packages/mapspinner/src/shaders/atmosphere.glsl:11 | none | 2 | open |
| wsl:WSL-096 | solar irradiance | (1.474, 1.8504, 1.91198) | packages/mapspinner/src/shaders/atmosphere.glsl:12 | none | 2 | open |
| wsl:WSL-097 | sun angular radius | 0.004675 rad | packages/mapspinner/src/shaders/atmosphere.glsl:13 | none | 2 | open |
| wsl:WSL-098 | Rayleigh phase | 3/(16 pi) * (1 + nu^2) | packages/mapspinner/src/shaders/atmosphere.glsl:19 | none | 2 | open |
| wsl:WSL-099 | Mie phase | K=3/(8pi)*(1-g2)/(2+g2) | packages/mapspinner/src/shaders/atmosphere.glsl:18 | none | 2 | open |
| wsl:WSL-100 | world to atmosphere scale | pos * (6360 / R_m) | packages/mapspinner/src/shaders/atmosphere.glsl:22 | none | 2 | open |
| wsl:WSL-101 | optical-depth march steps | 4 midpoint | packages/mapspinner/src/shaders/atmosphere.glsl:56 | none | 2 | open |
| wsl:WSL-102 | transmittance to sun horizon softness | smoothstep(muHoriz-0.035, muHoriz+0.005, mu) | packages/mapspinner/src/shaders/atmosphere.glsl:92 | none | 2 | open |
| wsl:WSL-103 | scattering LUT layers (shader) | 24 | packages/mapspinner/src/shaders/atmosphere.glsl:99 | none | 2 | open |
| wsl:WSL-104 | scattering muS warp K | 1.4 | packages/mapspinner/src/shaders/atmosphere.glsl:100 | none | 2 | open |
| wsl:WSL-105 | horizon blend mu | 0.006 | packages/mapspinner/src/shaders/atmosphere.glsl:172 | none | 2 | open |
| wsl:WSL-106 | sky march steps (non-LUT) | 8 sky, 8 ground | packages/mapspinner/src/shaders/atmosphere.glsl:193 | none | 2 | open |
| wsl:WSL-107 | sky irradiance day ramp | smoothstep(-0.10, 0.25, muS) | packages/mapspinner/src/shaders/atmosphere.glsl:211 | none | 2 | open |
| wsl:WSL-108 | sky irradiance scale | solar * 0.075 * day * skyTint * 0.5(1+dot(n,up)) | packages/mapspinner/src/shaders/atmosphere.glsl:214 | none | 2 | open |
| wsl:WSL-109 | sky tint Rayleigh mix | mix(1, rayTint, 0.4) | packages/mapspinner/src/shaders/atmosphere.glsl:213 | none | 2 | open |
| wsl:WSL-110 | sun direct irradiance at ground | solar * T(up*(bottom+0.5)) * clamp(n.sun) | packages/mapspinner/src/shaders/atmosphere.glsl:209 | none | 2 | open |
| wsl:WSL-111 | sky tint | (0.82, 0.95, 1.22) | packages/mapspinner/src/tsl/sky-tsl.js:20 | none | 2 | open |
| wsl:WSL-112 | sky exposure high sun | 14.0 | packages/mapspinner/src/tsl/sky-tsl.js:21 | none | 2 | open |
| wsl:WSL-113 | sky exposure low sun | 48.0 | packages/mapspinner/src/tsl/sky-tsl.js:22 | none | 2 | open |
| wsl:WSL-114 | sky saturation | 1.3 | packages/mapspinner/src/tsl/sky-tsl.js:23 | none | 2 | open |
| wsl:WSL-115 | limb halo colour | (0.32, 0.55, 1.0) | packages/mapspinner/src/tsl/sky-tsl.js:24 | none | 2 | open |
| wsl:WSL-116 | sky fade altitude | 100000 m | packages/mapspinner/src/tsl/sky-tsl.js:25 | none | 2 | open |
| wsl:WSL-117 | sky march steps | 8 | packages/mapspinner/src/tsl/sky-tsl.js:26 | none | 2 | open |
| wsl:WSL-118 | sky exposure blend | mix(LOW, HIGH, clamp(dot(sun, normalize(cam)),0,1)) | packages/mapspinner/src/tsl/sky-tsl.js:170 | none | 2 | open |
| wsl:WSL-119 | sky tonemap | ACES then luminance (0.2126,0.7152,0.0722) saturation mix, pow 1/2.2, x fade | packages/mapspinner/src/tsl/sky-tsl.js:171 | none | 2 | open |
| wsl:WSL-120 | sun disc | dot(ray,sun) > cos(0.004675) adds trans*solar*6.0 | packages/mapspinner/src/tsl/sky-tsl.js:166 | none | 2 | open |
| wsl:WSL-121 | limb halo | smoothstep(0,0.06,t0)*(1-smoothstep(0.25,1.6,t0)), x lit x 0.03 | packages/mapspinner/src/tsl/sky-tsl.js:163 | none | 2 | open |
| wsl:WSL-122 | halo lit term | smoothstep(-0.5,0.6,dot(limb,sun))*0.75+0.25 | packages/mapspinner/src/tsl/sky-tsl.js:165 | none | 2 | open |
| wsl:WSL-123 | sky fade | max(0, 1 - (\|cam\| - radius)/100000) | packages/mapspinner/src/tsl/sky-tsl.js:183 | none | 2 | open |
| wsl:WSL-124 | sky LUT texture type | RGBA HalfFloatType, linear, clamp | packages/mapspinner/src/tsl/sky-tsl.js:38 | none | 2 | open |
| wsl:WSL-125 | sky mie scattering | 0.003996 | packages/mapspinner/src/tsl/sky-tsl.js:11 | none | 2 | open |
| wsl:WSL-126 | sky Mie g | 0.8 | packages/mapspinner/src/tsl/sky-tsl.js:12 | none | 2 | open |
| wsl:WSL-127 | ground transmittance radius | ATM_BOTTOM + 0.5 | packages/mapspinner/src/tsl/sky-tsl.js:186 | none | 2 | open |
| wsl:WSL-128 | transmittance LUT width | 256 | packages/mapspinner/src/atmosphere-transmittance-lut.js:9 | none | 2 | open |
| wsl:WSL-129 | transmittance LUT height | 128 | packages/mapspinner/src/atmosphere-transmittance-lut.js:10 | none | 2 | open |
| wsl:WSL-130 | transmittance march steps | 500 | packages/mapspinner/src/atmosphere-transmittance-lut.js:11 | none | 2 | open |
| wsl:WSL-131 | transmittance LUT verify zenith steps | 200 steps, 32 altitude samples | packages/mapspinner/src/atmosphere-transmittance-lut.js:115 | none | 2 | open |
| wsl:WSL-132 | scattering LUT width | 64 | packages/mapspinner/src/atmosphere-scattering-lut.js:10 | none | 2 | open |
| wsl:WSL-133 | scattering LUT height | 32 | packages/mapspinner/src/atmosphere-scattering-lut.js:11 | none | 2 | open |
| wsl:WSL-134 | scattering LUT layers | 24 | packages/mapspinner/src/atmosphere-scattering-lut.js:12 | none | 2 | open |
| wsl:WSL-135 | scattering march steps | 64 | packages/mapspinner/src/atmosphere-scattering-lut.js:13 | none | 2 | open |
| wsl:WSL-136 | scattering muS warp K | 1.4 | packages/mapspinner/src/atmosphere-scattering-lut.js:15 | none | 2 | open |
| wsl:WSL-137 | scattering smoothness verify jump | 0.6 | packages/mapspinner/src/atmosphere-scattering-lut.js:16 | none | 2 | open |
| wsl:WSL-138 | scattering Mie channel mean | (tR+tG+tB)/3 | packages/mapspinner/src/atmosphere-scattering-lut.js:115 | none | 2 | open |
| wsl:WSL-139 | LUT bake order | transmittance first, then scattering using it | packages/mapspinner/src/atmosphere-lut-job.js:11 | none | 2 | open |
| wsl:WSL-140 | WGSL atmosphere top | 6500 | packages/mapspinner/src/webgpu/sky-render.js:7 | none | 2 | open |
| wsl:WSL-141 | WGSL rho max | 6403.774997918573 | packages/mapspinner/src/webgpu/sky-render.js:24 | none | 2 | open |
| wsl:WSL-142 | WGSL Mie extinction | 0.0044400002 | packages/mapspinner/src/webgpu/sky-render.js:13 | none | 2 | open |
| wsl:WSL-143 | WGSL sky exposure | mix(48, 14, sunElevDot) | packages/mapspinner/src/webgpu/sky-render.js:242 | none | 2 | open |
| wsl:WSL-144 | WGSL sky tint | (0.82,0.95,1.22) | packages/mapspinner/src/webgpu/sky-render.js:243 | none | 2 | open |
| wsl:WSL-145 | WGSL sky sun disc | t * solar * 6.0 | packages/mapspinner/src/webgpu/sky-render.js:239 | none | 2 | open |
| wsl:WSL-146 | WGSL sky saturation | 1.3 | packages/mapspinner/src/webgpu/sky-render.js:246 | none | 2 | open |
| wsl:WSL-147 | WGSL sky gamma and fade | pow(mapped, 1/2.2) * fade | packages/mapspinner/src/webgpu/sky-render.js:255 | none | 2 | open |
| wsl:WSL-148 | WGSL sky fade from altitude | max(0, 1 - alt/100000) | packages/mapspinner/src/webgpu/sky-render.js:271 | none | 2 | open |
| wsl:WSL-149 | WGSL halo colour | (0.32,0.55,1.0), x 0.03 | packages/mapspinner/src/webgpu/sky-render.js:234 | none | 2 | open |
| wsl:WSL-150 | sky uniform buffer size | 112 bytes | packages/mapspinner/src/webgpu/sky-render.js:260 | none | 2 | open |
| wsl:WSL-151 | sky render formats | colour bgra8unorm, depth24plus | packages/mapspinner/src/webgpu/sky-render.js:300 | none | 2 | open |
| wsl:WSL-152 | planet preset radius (spec context) | 63600 | src/presets/planet.js:6 | DEFAULT_RADIUS 6360.0 (differs from 63600) (src/globe.rs:39) | 2 | open |
| wsl:WSL-153 | planet preset maxLevel | 13 | src/presets/planet.js:8 | none | 2 | open |
| wsl:WSL-154 | relief design radius | 63600000.0 m | packages/mapspinner/src/terrain-defaults.js:1 | none | 2 | open |
| wsl:WSL-155 | sea level default (water level) | 0 | src/terrain/waterLevel.js:14 | GlobeWorld default sea_level 0.0 (src/globe.rs:48) | 2 | open |
| wsl:WSL-156 | sea surface radial offset in water WGSL | hR = 0.0 | packages/mapspinner/src/webgpu/water-render.js:86 | GlobeWorld default sea_level 0.0 (src/globe.rs:48) | 2 | open |
| wsl:WSL-157 | sea setter finite check | setSeaLevel(value) requires finite | src/terrain/waterLevel.js:24 | none | 2 | open |
| wsl:WSL-158 | RenderControls seaLevelY default | null | client/core/RenderControls.js:66 | none | 2 | open |
| wsl:WSL-159 | liquid state floating/diving depth split | depth > 1.5 diving else floating | src/terrain/waterLevel.js:51 | none | 2 | open |
| wsl:WSL-160 | liquid state in-water test | depth > 0 | src/terrain/waterLevel.js:48 | none | 2 | open |
| wsl:WSL-161 | water medium max depth | 64 m | src/shared/water.js:2 | none | 2 | open |
| wsl:WSL-162 | water medium extinction | 0.09 | src/shared/water.js:3 | none | 2 | open |
| wsl:WSL-163 | water medium scatter albedo | 0.35 | src/shared/water.js:4 | none | 2 | open |
| wsl:WSL-164 | water clear tint | (0.85, 0.95, 1) | src/shared/water.js:5 | none | 2 | open |
| wsl:WSL-165 | water deep tint | (0.04, 0.34, 0.52) | src/shared/water.js:6 | none | 2 | open |
| wsl:WSL-166 | foam slope onset | 0.45 | src/shared/water.js:7 | none | 2 | open |
| wsl:WSL-167 | foam slope gain | 2 | src/shared/water.js:8 | none | 2 | open |
| wsl:WSL-168 | foam shore base | 0.25 | src/shared/water.js:9 | none | 2 | open |
| wsl:WSL-169 | caustics sharpness | 6 | src/shared/water.js:10 | none | 2 | open |
| wsl:WSL-170 | water fresnel F0 default | 0.02, exponent 5 | src/shared/water.js:73 | none | 2 | open |
| wsl:WSL-171 | caustics time skew | v phase - 0.75 t | src/shared/water.js:118 | none | 2 | open |
| wsl:WSL-172 | water WGSL swell scale | uvSwell = swellP * 0.016 | packages/mapspinner/src/webgpu/water-render.js:91 | none | 2 | open |
| wsl:WSL-173 | water WGSL ripple amplitude | 0.006 | packages/mapspinner/src/webgpu/water-render.js:98 | none | 2 | open |
| wsl:WSL-174 | water WGSL tint | (0.05, 0.22, 0.38) | packages/mapspinner/src/webgpu/water-render.js:101 | none | 2 | open |
| wsl:WSL-175 | water WGSL tint mix and alpha | mix 0.55, alpha 0.75 | packages/mapspinner/src/webgpu/water-render.js:102 | none | 2 | open |
| wsl:WSL-176 | water WGSL alpha | 0.75 | packages/mapspinner/src/webgpu/water-render.js:103 | none | 2 | open |
| wsl:WSL-177 | water WGSL default grid size | 16 | packages/mapspinner/src/webgpu/water-render.js:166 | none | 2 | open |
| wsl:WSL-178 | water WGSL default ocean amp / choppy | 1.0 / 0.5 | packages/mapspinner/src/webgpu/water-render.js:179 | none | 2 | open |
| wsl:WSL-179 | water WGSL pipeline state key | water-blended-cull-front | packages/mapspinner/src/webgpu/water-render.js:170 | none | 2 | open |
| wsl:WSL-180 | underwater submerge margin | 2.0 m | client/core/UnderwaterTint.js:3 | none | 2 | open |
| wsl:WSL-181 | underwater camera tint limit | 3.0 m above sea | client/core/UnderwaterTint.js:4 | none | 2 | open |
| wsl:WSL-182 | underwater depth tint rate | clamp(depth*0.08, 0, 0.6) | client/core/UnderwaterTint.js:75 | none | 2 | open |
| wsl:WSL-183 | underwater fog colours | scene*(0.30,0.55,0.65) mixed to (0.04,0.34,0.52) | client/core/UnderwaterTint.js:76 | none | 2 | open |
| wsl:WSL-184 | ocean deep colour (terrain defaults) | (0.008, 0.025, 0.06) | packages/mapspinner/src/terrain-defaults.js:84 | none | 2 | open |
| wsl:WSL-185 | ocean shallow colour | (0.07, 0.22, 0.26) | packages/mapspinner/src/terrain-defaults.js:85 | none | 2 | open |
| wsl:WSL-186 | ocean extinction K | (0.009, 0.004, 0.0018) | packages/mapspinner/src/terrain-defaults.js:86 | none | 2 | open |
| wsl:WSL-187 | deep sea / sea band colours | (0.02,0.06,0.22) / (0.08,0.18,0.38) | packages/mapspinner/src/terrain-defaults.js:71 | globe biome_color water (0.10,0.25,0.55) (src/globe.rs:340) | 2 | open |
| wsl:WSL-188 | sea depth span | 580.0 m | packages/mapspinner/src/terrain-defaults.js:81 | none | 2 | open |
| wsl:WSL-189 | pool specular exponent rough/sharp | 24.0 / 256.0 | packages/mapspinner/src/terrain-defaults.js:62 | none | 2 | open |
| wsl:WSL-190 | terrain exposure | 1.05 | packages/mapspinner/src/terrain-defaults.js:20 | none | 2 | open |
| wsl:WSL-191 | relief shade | 6.0 | packages/mapspinner/src/terrain-defaults.js:21 | none | 2 | open |
| wsl:WSL-192 | sky fill | 0.3 | packages/mapspinner/src/terrain-defaults.js:22 | none | 2 | open |
| wsl:WSL-193 | haze multiplier | 0.4 | packages/mapspinner/src/terrain-defaults.js:24 | none | 2 | open |
| wsl:WSL-194 | night floor | 0.18 | packages/mapspinner/src/terrain-defaults.js:28 | none | 2 | open |
| wsl:WSL-195 | terminator width | 0.45 | packages/mapspinner/src/terrain-defaults.js:29 | none | 2 | open |
| wsl:WSL-196 | terminator glow | 0.6 | packages/mapspinner/src/terrain-defaults.js:30 | none | 2 | open |
| wsl:WSL-197 | night lights | 0.8 | packages/mapspinner/src/terrain-defaults.js:31 | none | 2 | open |
| wsl:WSL-198 | look saturation | 1.25 | packages/mapspinner/src/terrain-defaults.js:32 | none | 2 | open |
| wsl:WSL-199 | look contrast | 1.0 | packages/mapspinner/src/terrain-defaults.js:33 | none | 2 | open |
| wsl:WSL-200 | sun irradiance gain | 1.25 | packages/mapspinner/src/tsl/terrain-ambient-shared.js:3 | none | 2 | open |
| wsl:WSL-201 | sky irradiance scale | 0.075 | packages/mapspinner/src/tsl/terrain-ambient-shared.js:4 | none | 2 | open |
| wsl:WSL-202 | sky balance to grey | 0.35 | packages/mapspinner/src/tsl/terrain-ambient-shared.js:5 | none | 2 | open |
| wsl:WSL-203 | sky fill tint | (0.85, 0.92, 1.10) | packages/mapspinner/src/tsl/terrain-ambient-shared.js:6 | none | 2 | open |
| wsl:WSL-204 | terminator tint | (1.0, 0.55, 0.34) | packages/mapspinner/src/tsl/terrain-ambient-shared.js:7 | none | 2 | open |
| wsl:WSL-205 | night fill | (0.06, 0.075, 0.11) | packages/mapspinner/src/tsl/terrain-ambient-shared.js:8 | none | 2 | open |
| wsl:WSL-206 | ambient floor albedo | 0.14 | packages/mapspinner/src/tsl/terrain-ambient-shared.js:9 | none | 2 | open |
| wsl:WSL-207 | ambient floor add | (0.020, 0.026, 0.038) | packages/mapspinner/src/tsl/terrain-ambient-shared.js:10 | none | 2 | open |
| wsl:WSL-208 | aerial perspective gate | 3.0 to 120.0 km | packages/mapspinner/src/tsl/terrain-ambient-shared.js:11 | none | 2 | open |
| wsl:WSL-209 | sun dark luminance | 1e-4 | packages/mapspinner/src/tsl/terrain-ambient-shared.js:12 | none | 2 | open |
| wsl:WSL-210 | shadow cascade light name prefix | shadowCascade | packages/mapspinner/src/tsl/terrain-ambient-shared.js:13 | none | 2 | open |
| wsl:WSL-211 | Rayleigh sky tint weight | 0.4 | packages/mapspinner/src/tsl/terrain-ambient-shared.js:14 | none | 2 | open |
| wsl:WSL-212 | sky day mu ramp | -0.10 .. 0.25 | packages/mapspinner/src/tsl/terrain-ambient-shared.js:15 | none | 2 | open |
| wsl:WSL-213 | terminator fade angle | sin(10 deg) | packages/mapspinner/src/tsl/terrain-ambient-shared.js:16 | none | 2 | open |
| wsl:WSL-214 | luma weights | (0.2126, 0.7152, 0.0722) | packages/mapspinner/src/tsl/terrain-ambient-shared.js:18 | none | 2 | open |
| wsl:WSL-215 | terrain ACES coefficients | 2.51, 0.03, 2.43, 0.59, 0.14 | packages/mapspinner/src/tsl/terrain-ambient-shared.js:24 | none | 2 | open |
| wsl:WSL-216 | display gamma | 1/2.2 | packages/mapspinner/src/tsl/terrain-ambient-shared.js:25 | none | 2 | open |
| wsl:WSL-217 | terrain sky haze tint | (0.40, 0.55, 0.78) | packages/mapspinner/src/tsl/terrain-lighting-tsl.js:12 | none | 2 | open |
| wsl:WSL-218 | sun luminance uniform default | 1.0 | packages/mapspinner/src/tsl/terrain-lighting-tsl.js:17 | none | 2 | open |
| wsl:WSL-219 | wet pool fresnel | pow(1-N.V,5)*0.98+0.02 | packages/mapspinner/src/tsl/terrain-lighting-tsl.js:44 | none | 2 | open |
| wsl:WSL-220 | wet pool specular normalisation | (exp+8)/(8 pi) | packages/mapspinner/src/tsl/terrain-lighting-tsl.js:56 | none | 2 | open |
| wsl:WSL-221 | direct sun diffuse | albedo * sunIrr * 1.25/pi | packages/mapspinner/src/tsl/terrain-lighting-tsl.js:57 | none | 2 | open |
| wsl:WSL-222 | aerial perspective smoothstep | smoothstep(AP_GATE_KM) | packages/mapspinner/src/tsl/terrain-lighting-tsl.js:76 | none | 2 | open |
| wsl:WSL-223 | terminator graze ramp | smoothstep(0.55,1.0,1-\|up.sun\|) | packages/mapspinner/src/tsl/terrain-lighting-tsl.js:87 | none | 2 | open |
| wsl:WSL-224 | terminator glow ramps | smoothstep(-0.02,0.18,nwSun) and smoothstep(0,sin10,nwSun) | packages/mapspinner/src/tsl/terrain-lighting-tsl.js:88 | none | 2 | open |
| wsl:WSL-225 | final haze mix | mix(lit, hazed, apGate*hazeMul) | packages/mapspinner/src/tsl/terrain-lighting-tsl.js:90 | none | 2 | open |
| wsl:WSL-226 | day shade | mix(nightFloor,1,smoothstep(-termWidth,termWidth,nwSun)) | packages/mapspinner/src/tsl/terrain-lighting-tsl.js:91 | none | 2 | open |
| wsl:WSL-227 | terrain output exposure | c * exposure before ACES | packages/mapspinner/src/tsl/terrain-lighting-tsl.js:92 | none | 2 | open |
| wsl:WSL-228 | terrain output grade | ACES, then saturation mix, contrast (x-0.5)*c+0.5, pow 1/2.2 | packages/mapspinner/src/tsl/terrain-lighting-tsl.js:95 | none | 2 | open |
| wsl:WSL-229 | ambient colour from time of day | color * intensity / PI | packages/mapspinner/src/tsl/terrain-lighting-tsl.js:113 | none | 2 | open |
| wsl:WSL-230 | sun luminance from light | intensity * (0.2126 r + 0.7152 g + 0.0722 b) | packages/mapspinner/src/tsl/terrain-lighting-tsl.js:117 | none | 2 | open |
| wsl:WSL-231 | cascade blend distance | 4.0 m | client/core/CascadeShadowSelect.js:5 | none | 2 | open |
| wsl:WSL-232 | cascade count clamp (GL path) | 1..3 | client/core/CascadeShadowSelect.js:12 | none | 2 | open |
| wsl:WSL-233 | cascade far-edge fade | 1 - smoothstep(farEdge - blend, farEdge, camDist) | client/core/CascadeShadowSelect.js:31 | none | 2 | open |
| wsl:WSL-234 | cascade near-edge fade | smoothstep(nearEdge - blend, nearEdge, camDist) | client/core/CascadeShadowSelect.js:34 | none | 2 | open |
| wsl:WSL-235 | cascade split literal precision | toFixed(2) metres | client/core/CascadeShadowSelect.js:68 | none | 2 | open |
| wsl:WSL-236 | cascade extent ratio | CASCADE_SPLIT 3.2 | client/core/ShadowPipeline.js:6 | none | 2 | open |
| wsl:WSL-237 | max cascades | 3 | client/core/ShadowPipeline.js:7 | none | 2 | open |
| wsl:WSL-238 | shadow redraw caster height | 20 m | client/core/ShadowPipeline.js:8 | none | 2 | open |
| wsl:WSL-239 | far cascade lateral step | 1 m | client/core/ShadowPipeline.js:9 | none | 2 | open |
| wsl:WSL-240 | depth step divisor | 16 | client/core/ShadowPipeline.js:10 | none | 2 | open |
| wsl:WSL-241 | far cascade redraws per frame | 1 | client/core/ShadowPipeline.js:11 | none | 2 | open |
| wsl:WSL-242 | far cascade max defer frames | 2 | client/core/ShadowPipeline.js:12 | none | 2 | open |
| wsl:WSL-243 | base cascade extent default | 60 m half-extent | client/core/ShadowPipeline.js:17 | none | 2 | open |
| wsl:WSL-244 | default cascade request | 1 | client/core/ShadowPipeline.js:18 | none | 2 | open |
| wsl:WSL-245 | light offset default | (40, 80, 30) | client/core/ShadowPipeline.js:21 | none | 2 | open |
| wsl:WSL-246 | cascade i extent | baseExtent * 3.2^i (60, 192, 614.4) | client/core/ShadowPipeline.js:51 | none | 2 | open |
| wsl:WSL-247 | cascade shadow camera near | 0.5 | client/core/ShadowPipeline.js:117 | none | 2 | open |
| wsl:WSL-248 | cascade shadow camera far | offset length + extent*1.5 | client/core/ShadowPipeline.js:117 | none | 2 | open |
| wsl:WSL-249 | cascade secondary lights share | mapSize, bias, normalBias, radius copied from sun | client/core/ShadowPipeline.js:39 | none | 2 | open |
| wsl:WSL-250 | cascade secondary light colour/intensity | white, intensity 0, castShadow true | client/core/ShadowPipeline.js:36 | none | 2 | open |
| wsl:WSL-251 | cascade count by quality tier | Low 1, Medium 1, High 2, Ultra 3 | client/BootFailureOverlay.js:7 | none | 2 | open |
| wsl:WSL-252 | cascade count clamp (boot) | 1..3 | client/BootFailureOverlay.js:5 | none | 2 | open |
| wsl:WSL-253 | shadowCascades control default | 1 | client/core/RenderControls.js:105 | none | 2 | open |
| wsl:WSL-254 | sun shadow map size | 1024 x 1024 | client/core/SceneSetup.js:278 | viewer DirectionalLight illuminance 2600.0, shadow_maps_enabled false (src/viewer.rs:322) | 2 | open |
| wsl:WSL-255 | sun shadow bias | -0.0005 | client/core/SceneSetup.js:279 | viewer DirectionalLight illuminance 2600.0, shadow_maps_enabled false (src/viewer.rs:322) | 2 | open |
| wsl:WSL-256 | sun shadow normalBias | 0.05 | client/core/SceneSetup.js:280 | viewer DirectionalLight illuminance 2600.0, shadow_maps_enabled false (src/viewer.rs:322) | 2 | open |
| wsl:WSL-257 | sun shadow radius | 4 | client/core/SceneSetup.js:281 | viewer DirectionalLight illuminance 2600.0, shadow_maps_enabled false (src/viewer.rs:322) | 2 | open |
| wsl:WSL-258 | sun shadow camera near | 0.5 | client/core/SceneSetup.js:282 | none | 2 | open |
| wsl:WSL-259 | sun shadow camera far | 200 | client/core/SceneSetup.js:283 | none | 2 | open |
| wsl:WSL-260 | sun default colour/intensity | 0xffffff / 1.5 | client/core/SceneSetup.js:276 | viewer DirectionalLight illuminance 2600.0, shadow_maps_enabled false (src/viewer.rs:322) | 2 | open |
| wsl:WSL-261 | sun default position | (21, 50, 20), castShadow true | client/core/SceneSetup.js:277 | none | 2 | open |
| wsl:WSL-262 | ambient light default | 0xfff4d6, intensity 0.5 | client/core/SceneSetup.js:271 | no AmbientLight spawned in viewer.rs (grep of src/*.rs) (src/viewer.rs:322) | 2 | open |
| wsl:WSL-263 | fill (studio) light default | 0x4488ff, intensity 0.4 | client/core/SceneSetup.js:274 | none | 2 | open |
| wsl:WSL-264 | scene background | 0x87ceeb | client/core/SceneSetup.js:16 | ClearColor srgb(0.035,0.045,0.065) (src/viewer.rs:176) | 2 | open |
| wsl:WSL-265 | scene fog default | FogExp2 0x87ceeb density 0.0025 | client/core/SceneSetup.js:17 | none | 2 | open |
| wsl:WSL-266 | fog far default (window.__fogFar) | 200 | client/core/SceneSetup.js:18 | none | 2 | open |
| wsl:WSL-267 | renderer output colour space | SRGBColorSpace | client/core/SceneSetup.js:159 | none | 2 | open |
| wsl:WSL-268 | config fog far default | 200 | client/core/SceneSetup.js:323 | none | 2 | open |
| wsl:WSL-269 | config exp2 fog density default | 0.0025 | client/core/SceneSetup.js:325 | none | 2 | open |
| wsl:WSL-270 | config linear fog near default | 80 | client/core/SceneSetup.js:327 | none | 2 | open |
| wsl:WSL-271 | config ambient intensity default | 0.3 | client/core/SceneSetup.js:331 | none | 2 | open |
| wsl:WSL-272 | config sun intensity default | 1.5 | client/core/SceneSetup.js:332 | none | 2 | open |
| wsl:WSL-273 | config fill intensity default | 0.4 | client/core/SceneSetup.js:334 | none | 2 | open |
| wsl:WSL-274 | fogFar control default | 200 | client/core/RenderControls.js:38 | none | 2 | open |
| wsl:WSL-275 | tone mapping mode default | ACESFilmic | client/core/RenderControls.js:89 | Camera3d::default(); no Tonemapping or Exposure component set (src/viewer.rs:327) | 2 | open |
| wsl:WSL-276 | tone mapping exposure default | 1.0 | client/core/RenderControls.js:91 | none | 2 | open |
| wsl:WSL-277 | tone mapping applied to renderer | r.toneMapping / r.toneMappingExposure | client/core/RenderControls.js:280 | none | 2 | open |
| wsl:WSL-278 | terrain floor irradiance match | 0.14 * PI | client/core/TimeOfDay.js:6 | none | 2 | open |
| wsl:WSL-279 | deep night ambient colour | 0xb4c3e6 | client/core/TimeOfDay.js:17 | none | 2 | open |
| wsl:WSL-280 | twilight ambient colour | 0xb9c1df | client/core/TimeOfDay.js:18 | none | 2 | open |
| wsl:WSL-281 | horizon ambient colour | 0xc8b6c8 | client/core/TimeOfDay.js:19 | none | 2 | open |
| wsl:WSL-282 | time of day keyframe -90 deg | sun 0x0a1030 i0.0; ambient 0xb4c3e6, i=0.14pi/luma; fill 0.44 | client/core/TimeOfDay.js:22 | none | 2 | open |
| wsl:WSL-283 | time of day keyframe -6 deg | sun 0x1a2550 i0.0; ambient 0xb9c1df, i=0.14pi/luma; fill 0.52 | client/core/TimeOfDay.js:23 | none | 2 | open |
| wsl:WSL-284 | time of day keyframe 0 deg | sun 0xff7a3c i0.55; ambient 0xc8b6c8, i=0.14pi/luma; fill 0.8 | client/core/TimeOfDay.js:24 | none | 2 | open |
| wsl:WSL-285 | time of day keyframe 8 deg | sun 0xffa552 i1.8; ambient 0xffcf9e i0.65; fill 1.3 | client/core/TimeOfDay.js:25 | none | 2 | open |
| wsl:WSL-286 | time of day keyframe 30 deg | sun 0xfff0dc i1.45; ambient 0xc9d4e8 i0.48; fill 0.96 | client/core/TimeOfDay.js:26 | none | 2 | open |
| wsl:WSL-287 | time of day keyframe 70 deg | sun 0xffffff i1.6; ambient 0xfff4d6 i0.5; fill 1.0 | client/core/TimeOfDay.js:27 | none | 2 | open |
| wsl:WSL-288 | time of day keyframe 90 deg | sun 0xffffff i1.55; ambient 0xfff4d6 i0.5; fill 1.0 | client/core/TimeOfDay.js:28 | none | 2 | open |
| wsl:WSL-289 | time of day interpolation | lerp intensities, colour lerp between keyframes | client/core/TimeOfDay.js:44 | none | 2 | open |
| wsl:WSL-290 | atmosphere bottom (transmittance module) | 6360.0 | packages/mapspinner/src/atmosphere-transmittance-lut.js:1 | none | 2 | open |
| wsl:WSL-291 | atmosphere top (transmittance module) | 6500.0 | packages/mapspinner/src/atmosphere-transmittance-lut.js:2 | none | 2 | open |
| wsl:WSL-292 | Rayleigh height (transmittance module) | 18.0 | packages/mapspinner/src/atmosphere-transmittance-lut.js:3 | none | 2 | open |
| wsl:WSL-293 | Mie height (transmittance module) | 4.0 | packages/mapspinner/src/atmosphere-transmittance-lut.js:4 | none | 2 | open |
| wsl:WSL-294 | Rayleigh coefficients (transmittance module) | [0.005802, 0.013558, 0.0331] | packages/mapspinner/src/atmosphere-transmittance-lut.js:5 | none | 2 | open |
| wsl:WSL-295 | Mie scattering (transmittance module) | 0.003996 | packages/mapspinner/src/atmosphere-transmittance-lut.js:6 | none | 2 | open |
| wsl:WSL-296 | Mie extinction (transmittance module) | 0.003996 * (1/0.9) | packages/mapspinner/src/atmosphere-transmittance-lut.js:7 | none | 2 | open |
| wsl:WSL-297 | GPU LUT sampler | linear if float32-filterable else nearest, clamp-to-edge | packages/mapspinner/src/webgpu/atmosphere-lut-compute.js:6 | none | 2 | open |

## Vegetation

| id | rule or number | reference value | source (path:line in /workspace/spoint) | port today | stage | status |
|---|---|---|---|---|---|---|
| veg:place.chunk | chunk size m | 32 | src/terrain/VegPlacement.js:7 | absent; port cells come from globe.cell_bounds, vegetation.rs:68 | 3 | open |
| veg:place.cell | cell size m | 4 | src/terrain/VegPlacement.js:8 | absent, no equivalent in vegetation.rs (98 lines) | 3 | open |
| veg:place.grid | cells per chunk side | 8 (64 cells per chunk) | src/terrain/VegPlacement.js:9 | vegetation.rs:4 fixed 6 slots per chunk | 3 | open |
| veg:place.jitter | jitter m | 1.8, applied as JITTER/CELL = 0.45 of a cell | src/terrain/VegPlacement.js:10 and src/terrain/VegPlacement.js:270 | vegetation.rs:42-43 uniform hash over whole cell bounds | 3 | open |
| veg:place.slope_d | slope sample distance m | 1.5 | src/terrain/VegPlacement.js:11 | absent, no equivalent in vegetation.rs (98 lines) | 3 | open |
| veg:place.slope_max | slope gate, hypot of slope components | 0.6, reject above | src/terrain/VegPlacement.js:12 and src/terrain/VegPlacement.js:223 | absent, no equivalent in vegetation.rs (98 lines) | 3 | open |
| veg:place.sea_reject | seaBias reject threshold | -2 | src/terrain/VegPlacement.js:13 and src/terrain/VegPlacement.js:162 | vegetation.rs:44 elevation > sea_level only | 3 | open |
| veg:place.treeline | treeline elevation m | 4000, scaled by relief margin | src/terrain/VegPlacement.js:14 and src/terrain/VegPlacement.js:216 | absent, no equivalent in vegetation.rs (98 lines) | 3 | open |
| veg:place.treeline_fade | treeline fade m | 120, scaled by relief margin | src/terrain/VegPlacement.js:15 and src/terrain/VegPlacement.js:217 | absent, no equivalent in vegetation.rs (98 lines) | 3 | open |
| veg:place.hash.cell | per-cell hash | hash3(seed, row, j) from lattice row; seed = (worldSeed/0) ^ 0x7eed | src/terrain/VegPlacement.js:179 and src/terrain/VegPlacement.js:266 | vegetation.rs:42 globe.hash(key.0,key.1,2*slot) | 3 | open |
| veg:place.hash.fn | hash3 mixing constants | 0x27d4eb2d, 0x165667b1, 0x2c1b3c6d, 0x297a2d39, shifts 15,12,15 | src/terrain/VegPlacement.js:96-104 | globe.hash, different function | 3 | open |
| veg:place.rand.fn | rand(h,k) mixing | h ^ imul(k+1, 0x9e3779b1), 0x85ebca6b, >>>13 | src/terrain/VegPlacement.js:106-111 | absent | 3 | open |
| veg:place.cellhash | classify cell hash seed | hash3(0x5eed, row, j) | src/terrain/VegPlacement.js:202 | absent | 3 | open |
| veg:place.coin | coin key | K_COIN = 2; reject if coin >= BASE_DENSITY_CEILING*groveMul | src/terrain/VegPlacement.js:27 and src/terrain/VegPlacement.js:204 | absent; every slot kept if on land | 3 | open |
| veg:key.table | per-cell random keys | K_JITX = 0, K_JITZ = 1, K_COIN = 2, K_VARIANT = 3, K_YAW = 4, K_SCALE = 5, K_WIND = 6, K_SPECIESH = 7, K_SIZE = 8, K_SHAPE = 9, K_TINT_HUE = 10, K_TINT_SAT = 11, K_TINT_VAL = 12, K_LEAN = 13, K_LEAN_DIR = 14, K_UNDERSTORY = 15, K_COMPANION = 16 | src/terrain/VegPlacement.js:26 | absent | 3 | open |
| veg:density.base | baseDensity(temp,humidity,erosion) | (0.12 + 0.73*wet*(0.4+0.6*warm)) * (1 - 0.45*ero), inputs clamped 0..1 | src/terrain/VegPlacement.js:142-147 | TREES_PER_CHUNK = 6, vegetation.rs:4 | 3 | open |
| veg:density.erosion_default | erosion default when climate lacks it | 0.3 | src/terrain/VegPlacement.js:161 | absent | 3 | open |
| veg:density.temp_hum_default | temp and humidity defaults | 0.5, 0.5 | src/terrain/VegPlacement.js:159-160 | absent | 3 | open |
| veg:density.grove.seed | grove noise seed | 0x67a0e5 | src/terrain/VegPlacement.js:32 | absent | 3 | open |
| veg:density.grove.cells | grove cell m / detail cell m | 96 / 37 | src/terrain/VegPlacement.js:33 | absent | 3 | open |
| veg:density.grove.mix | groveAt blend | 0.7 * coarse + 0.3 * detail (detail seed GROVE_SEED ^ 0x9e37) | src/terrain/VegPlacement.js:73 | absent | 3 | open |
| veg:density.grove.smooth | grove weight | smoothstep(0.30, 0.70, grove) | src/terrain/VegPlacement.js:200 | absent | 3 | open |
| veg:density.grove.mul | groveMul | (0.15 + 1.45*groveWeight) * cell.area | src/terrain/VegPlacement.js:34 and src/terrain/VegPlacement.js:201 | absent | 3 | open |
| veg:density.soil | renderedSoilWeight | beach top 15 m, sand-only below 4.5 m (15*0.3), smoothstep between | src/terrain/VegPlacement.js:121-129 | absent | 3 | open |
| veg:density.painted | painted grass weight and snow veto | densityMul = soilMul*painted.grass; skip when snow beats grass, rock and sand | src/terrain/VegPlacement.js:224-228 | absent | 3 | open |
| veg:density.aridity | aridity line | 0.28; unjittered humidity below it gives Bush | src/terrain/VegPlacement.js:113 and src/terrain/VegPlacement.js:134-135 | absent | 3 | open |
| veg:density.relief | relief calibration baseline | 0.01 | src/terrain/VegPlacement.js:115 | absent | 3 | open |
| veg:patch.t | patch temperature noise seed / cell | 0x51a7c1 / 64 m; vT = (rand-0.5)*0.30 + (noise-0.5)*0.60 | src/terrain/VegPlacement.js:32-33 and src/terrain/VegPlacement.js:231 | absent | 3 | open |
| veg:patch.h | patch humidity noise seed | 0x2b3c4d; vH = (rand-0.5)*0.30 + (noise-0.5)*0.60 | src/terrain/VegPlacement.js:32 and src/terrain/VegPlacement.js:232 | absent | 3 | open |
| veg:species.count | species list length | SPECIES = Object.freeze(['Oak Large', 'Pine Medium', 'Aspen Medium', 'Ash Medium', 'Bush', 'Ash Small', 'Ash Large', 'Aspen Small', 'Aspen Large', 'Bush 2', 'Bush 3', 'Oak Small', 'Oak Medium', 'Pine Small', 'Pine Large']) (15 names) | src/terrain/VegPlacement.js:18 | 1 trunk plus 1 canopy mesh pair, vegetation.rs:31-32 | 3 | open |
| veg:species.list | names in order | Oak Large, Pine Medium, Aspen Medium, Ash Medium, Bush, Ash Small, Ash Large, Aspen Small, Aspen Large, Bush 2, Bush 3, Oak Small, Oak Medium, Pine Small, Pine Large | src/terrain/VegPlacement.js:18-22 | absent | 3 | open |
| veg:species.genus | genus choice rule (replaces weights) | arid -> Bush; elevNorm>0.62 or t<0.30 -> Pine; t>0.62 and h>0.55 -> Oak; h>0.50 -> Ash; else Aspen; t,h rounded to 0.1 | src/terrain/VegPlacement.js:131-140 | absent | 3 | open |
| veg:species.companion.chance | companion swap chance | 0.22 | src/terrain/VegPlacement.js:44 and src/terrain/VegPlacement.js:236 | absent | 3 | open |
| veg:species.companion.map | companion genus | Oak->Ash, Ash->Aspen, Aspen->Oak, Pine->Aspen | src/terrain/VegPlacement.js:45 | absent | 3 | open |
| veg:species.understory | understory Bush chance | 0.08 + 0.35*(1-groveWeight) | src/terrain/VegPlacement.js:35 and src/terrain/VegPlacement.js:234 | absent | 3 | open |
| veg:species.sizeclass | size class cut | p = sizeR*0.7 + (1-elevNorm)*0.3; p<0.40 small, p<0.75 medium, else large | src/terrain/VegPlacement.js:54-57 | absent | 3 | open |
| veg:species.siblings | size sibling table | Oak S/M/L = Oak Small, Oak Medium, Oak Large; Pine = Small, Medium, Large; Aspen and Ash likewise; Bush = Bush, Bush 2, Bush 3 | src/terrain/VegPlacement.js:47-53 | absent | 3 | open |
| veg:scale.low | scale base | 0.62 | src/terrain/VegPlacement.js:36 | trees spawn at unit scale, vegetation.rs:54 | 3 | open |
| veg:scale.span | scale age span / skew | 0.72 / 1.4; age = rand^1.4 | src/terrain/VegPlacement.js:36 and src/terrain/VegPlacement.js:240 | absent | 3 | open |
| veg:scale.sizeclass | size class scale multipliers | 0.90, 1.0, 1.10 | src/terrain/VegPlacement.js:37 and src/terrain/VegPlacement.js:241 | absent | 3 | open |
| veg:scale.yaw | yaw | rand(K_YAW) * 2PI | src/terrain/VegPlacement.js:242 | rotation from globe.orientation only, vegetation.rs:53 | 3 | open |
| veg:lean.max | lean max rad tree / bush | 0.09 / 0.16; magnitude = leanR^2 * max | src/terrain/VegPlacement.js:38 and src/terrain/VegPlacement.js:246-247 | absent | 3 | open |
| veg:shape.variants | shape variants per species | 3, picked by floor(rand(K_SHAPE)*3) | src/terrain/VegPlacement.js:30 and src/terrain/VegPlacement.js:244 | absent | 3 | open |
| veg:tint.bright | tint brightness | 0.80 + 0.36*rand, times (1 - 0.06*(scale-1)) | src/terrain/VegPlacement.js:39 and src/terrain/VegPlacement.js:251 | flat base_color, vegetation.rs:33-34 | 3 | open |
| veg:tint.sat | tint saturation | 0.35 + 0.65*rand, times genus strength (Pine 0.4, Bush 0.8) | src/terrain/VegPlacement.js:40 and src/terrain/VegPlacement.js:43 and src/terrain/VegPlacement.js:250 | absent | 3 | open |
| veg:tint.hue | tint hue weights | 0.55 random + 0.45 season; season noise cell 420 m seed 0x5ea50a | src/terrain/VegPlacement.js:32-33 and src/terrain/VegPlacement.js:41 and src/terrain/VegPlacement.js:249 | absent | 3 | open |
| veg:tint.colors | warm / cool targets | [1.22,0.76,0.52] / [0.64,1.07,0.58] | src/terrain/VegPlacement.js:42 | absent | 3 | open |
| veg:build.height | target height m per species | Oak L 9, Pine M 12, Aspen M 9.5, Ash M 10, Bush 2.8, Ash S 6.5, Ash L 13, Aspen S 6, Aspen L 13, Bush2 3.0, Bush3 3.2, Oak S 5.5, Oak M 7, Pine S 8, Pine L 16 | client/core/VegetationBuild.js:28-32 | trunk 1.2 + canopy 2.0 cone, about 3 m, vegetation.rs:31-32 | 3 | open |
| veg:build.height.fallback | unknown species height | 9 | client/core/VegetationBuild.js:516 | absent | 3 | open |
| veg:build.tricap | branch / leaf triangle cap | 6000 / 1400 (defaults) | client/core/VegetationBuild.js:556 and client/core/Vegetation.js:156 | absent | 3 | open |
| veg:build.lod.simplify | LOD simplify ratios | branch b1 0.28 strict, b2 0.07 sloppy; leaf l1 0.30, l2 0.09; shadow 0.07 sloppy | client/core/VegetationBuild.js:549-551 | absent | 3 | open |
| veg:build.variant.tint | variant leaf tint bias | [1,1,1], [1.06,1.0,0.90], [0.92,1.04,1.0] | client/core/VegetationBuild.js:426 | absent | 3 | open |
| veg:build.leaf.alphatest | leaf alphaTest floor | 0.5 | client/core/VegetationBuild.js:503 | absent | 3 | open |
| veg:tier.render_distance | render distance m | 640 default | client/core/Vegetation.js:79; apps pass 640 at apps/world/tps-game.js:9 | absent; no distance streaming | 3 | open |
| veg:tier.ring | ring radius / drop radius | renderDistance+40 = 680; drop = ring+64 = 744 | client/core/Vegetation.js:80-81 and :30 | absent | 3 | open |
| veg:tier.d1 | LOD1 distance m | min(14, renderDistance*0.045) = 14 at 640 | client/core/Vegetation.js:107 | absent | 3 | open |
| veg:tier.d2 | LOD2 distance m | min(35, renderDistance*0.11) = 35 at 640 | client/core/Vegetation.js:108 | absent | 3 | open |
| veg:tier.d3 | impostor distance m | min(renderDistance, max(D2+12, renderDistance*0.13)) = 83.2 at 640 | client/core/Vegetation.js:109 | absent | 3 | open |
| veg:tier.near_cutoff | impostor near cutoff | D3*0.92 | client/core/Vegetation.js:110 | absent | 3 | open |
| veg:tier.shadow | shadow cast distance m | 35 | client/core/Vegetation.js:111 | absent | 3 | open |
| veg:tier.hysteresis | LOD hysteresis | 0.12 | client/core/Vegetation.js:112 | absent | 3 | open |
| veg:tier.handoff | impostor dissolve fade band m | 3.0; impostorStart = cutoff-3, meshEnd = cutoff+3 | client/core/VegImpostorTier.js:10-14 | absent | 3 | open |
| veg:tier.far_swap | far LOD swap | max(D2*(1+hyst), meshEnd) | client/core/Vegetation.js:337 | absent | 3 | open |
| veg:tier.reentry | mesh re-entry distance | nearCutoff + LOD_REEVAL_MOVE_M | client/core/Vegetation.js:338 | absent | 3 | open |
| veg:tier.reeval | LOD re-evaluate move m | 0.5 | client/core/WebGPULodInstancer.js:7 | absent | 3 | open |
| veg:tier.leaf_floor | leaf off floor | D2*(1+hyst); QualityGovernor mid band edge 39.2 m | client/core/Vegetation.js:339 and client/core/QualityGovernor.js:52 | absent | 3 | open |
| veg:tier.falloff.near | keep-probability near distance m | 96 | client/core/Vegetation.js:85 | absent | 3 | open |
| veg:tier.falloff.floor | keep-probability floor | 0.15, smoothstep from near to renderDistance | client/core/Vegetation.js:86 and :88-94 | absent | 3 | open |
| veg:tier.falloff.coin | keep coin hash | trunkId ^ 0x9e3779b1, 0x85ebca6b | client/core/Vegetation.js:95-100 and :398 | absent | 3 | open |
| veg:tier.bvh.margin | BVH margin m | 24 | client/core/Vegetation.js:113 | absent | 3 | open |
| veg:imp.sprites | impostor sprites per side | 8 | client/core/Vegetation.js:194 and :247 | absent | 3 | open |
| veg:imp.texture | per-species bake texture px | 512 | client/core/Vegetation.js:194 and :248 | absent | 3 | open |
| veg:imp.alpha | alphaClamp | 0.4 | client/core/Vegetation.js:247 and :306 | absent | 3 | open |
| veg:imp.hemi | useHemiOctahedron | false | client/core/Vegetation.js:194 | absent | 3 | open |
| veg:imp.atlas.grid | shared atlas grid side | ceil(sqrt(speciesCount)) = 4 for 15 species | client/core/VegImpostorTier.js:29 | absent | 3 | open |
| veg:imp.atlas.size | shared atlas mega size px | atlasSize*gridSide; tile width from albedo (fallback 1024), 512*4 = 2048 | client/core/VegImpostorTier.js:28-30 | absent | 3 | open |
| veg:imp.atlas.aniso | atlas anisotropy | 4 | client/core/VegImpostorTier.js:38 | absent | 3 | open |
| veg:imp.atlas.bytes | atlas bytes formula | mega*mega*4*(2 with normal) | client/core/VegImpostorTier.js:82 | absent | 3 | open |
| veg:imp.capacity | shared impostor capacity | min(maxInstances or 20000, initCapacity or 4096) | client/core/VegImpostorTier.js:91 | absent | 3 | open |
| veg:imp.polyoffset | impostor polygon offset | factor -4, units -8 | client/core/VegImpostorTier.js:103-104 | absent | 3 | open |
| veg:imp.parallax | parallax default scale | 0.3 (off by default) | client/core/VegImpostorTier.js:100 | absent | 3 | open |
| veg:imp.bakevariant | variants that bake | variant 0 only; others reuse the species bake | client/core/Vegetation.js:190 and :241 | absent | 3 | open |
| veg:budget.max_instances | max instances | 30000 (cfg.maxInstances) | client/core/Vegetation.js:84; apps/world/tps-game.js:9; apps/world/_shared/sillos-terrain.js:5 | absent; no cap, count only reported, vegetation.rs:24 | 3 | open |
| veg:budget.30k_figure | the 30,000-tree figure | it is the maxInstances cap, not a measured count; also the second test scale in the culling A/B | client/core/Vegetation.js:84 and client/core/WebGPUCullingHubIntegration.js:34 | absent | 3 | open |
| veg:budget.init_cap | initial instancer capacity | min(maxInstances, 2048) | client/core/Vegetation.js:101 | absent | 3 | open |
| veg:budget.load_ms | chunk load budget ms per frame | 2 | client/core/Vegetation.js:470 | absent | 3 | open |
| veg:budget.retries | deferred chunk retries per frame | 2 | client/core/Vegetation.js:471 | absent | 3 | open |
| veg:budget.idle | idle stream stride / epsilon | 16 frames / 0.05 m | client/core/Vegetation.js:475 | absent | 3 | open |
| veg:budget.bvh_growth | BVH rebuild growth fraction | 0.5 | client/core/Vegetation.js:628 | absent | 3 | open |
| veg:budget.prewarm | prewarm defaults | 64 chunks minimum, 4000 ms | client/core/Vegetation.js:532 | absent | 3 | open |
| veg:budget.tex.leaf | leaf texture max px | 128 (cfg or ?leaftex) | client/core/TreeTextureBudget.js:5 | absent | 3 | open |
| veg:budget.tex.bark | bark texture max px | 64 (cfg or ?barktex) | client/core/TreeTextureBudget.js:6 | absent | 3 | open |
| veg:budget.tex.source | tree texture source px | 1024 | client/core/TreeTextureBudget.js:2 | absent | 3 | open |
| veg:budget.tex.range | texture min / max px | 64 / 1024, power of two only | client/core/TreeTextureBudget.js:3-4 and :40-42 | absent | 3 | open |
| veg:budget.tex.halving | texture halving limit | 16 | client/core/TreeTextureBudget.js:7 | absent | 3 | open |
| veg:cull.ab.counts | culling A/B scales | 12000 and 30000 instances, 5 runs each | client/core/WebGPUCullingHubIntegration.js:34 | absent | 3 | open |
| veg:cull.ab.spread | synthetic instance spread | 1280 m xz, 40 m y, radius 2 to 8, seed 42+n | client/core/WebGPUCullingHubIntegration.js:6 and :39 | absent | 3 | open |
| veg:cull.planes | frustum test | 6 planes, sphere inside when dist >= -radius | client/core/WebGPUCullingHubIntegration.js:23-26 | absent | 3 | open |
| veg:cull.cell | LOD grid cell size m / max cells | 24 / 4096 | client/core/WebGPULodInstancer.js:19 and :14 | absent | 3 | open |
| veg:cull.cell_occ | target cell occupancy | 4 | client/core/WebGPULodInstancer.js:20 | absent | 3 | open |
| veg:cull.occ_margin | occlusion proxy margin m | 2, lift max(2, height*0.5) | client/core/Vegetation.js:884 | absent | 3 | open |
| veg:wind.sway | sway amplitude | position.y * 0.06 * uVegWind | client/core/VegetationTSL.js:15 and :17 | absent | 3 | open |
| veg:wind.speed | phase | uVegTime*1.3 + windPhase; windPhase = rand(K_WIND)*2PI | client/core/VegetationTSL.js:16 and src/terrain/VegPlacement.js:243 | absent | 3 | open |
| veg:wind.z | z sway | cos(phase*0.8) * wsway * 0.6 * uVegWind | client/core/VegetationTSL.js:18 | absent | 3 | open |
| veg:wind.default | wind strength default | 1 (window.__vegWind override) | client/core/Vegetation.js:103 | absent | 3 | open |

## Grass, rocks and frame budget

| id | rule or number | reference value | source (path:line in /workspace/spoint) | port today | stage | status |
|---|---|---|---|---|---|---|
| grb:G01 | grass render distance (ring radius m) | 88 | client/core/Grass.js:57 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G02 | grass drop margin (m) | 16 | client/core/Grass.js:34 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G03 | grass max instances | 540000 | client/core/Grass.js:61 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G04 | grass initial instanced capacity cap | 4096 near | client/core/Grass.js:62 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G05 | mid-LOD instancer initial capacity | min(INIT_CAP,2048) | client/core/Grass.js:100 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G06 | near LOD distance (m) | 8 | client/core/Grass.js:67 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G07 | near clump geometry segments,cards | makeClumpGeo(2,1) | client/core/Grass.js:87 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G08 | mid clump geometry segments,cards | makeClumpGeo(1,1) | client/core/Grass.js:88 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G09 | grass draw calls | 2 (near+mid) | client/core/Grass.js:129 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G10 | grass render order | 2 | client/core/Grass.js:114 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G11 | clump width min | 2.268 | client/core/Grass.js:35 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G12 | clump width span (scaled by tint) | 1.176 | client/core/Grass.js:36 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G13 | clump height scale | 0.75 | client/core/Grass.js:37 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G14 | blade instance scale law | width=MIN+SPAN*tint; y=scale*HEIGHT_SCALE | client/core/Grass.js:149 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G15 | grass wind animation speed | 0.25 | client/core/Grass.js:38 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G16 | grass chunk size (m) | 32 | src/terrain/GrassPlacement.js:13 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G17 | grass placement cell size (m) | 2 | src/terrain/GrassPlacement.js:14 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G18 | grass cells per chunk edge | 16 | src/terrain/GrassPlacement.js:15 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G19 | grass cell jitter (m) | 0.9 | src/terrain/GrassPlacement.js:16 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G20 | grass slope sample distance | 1.0 | src/terrain/GrassPlacement.js:17 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G21 | grass max slope gradient (reject above) | 0.9 | src/terrain/GrassPlacement.js:18 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G22 | grass sea-bias reject threshold | -2 | src/terrain/GrassPlacement.js:19 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G23 | blade scale min | 0.462 | src/terrain/GrassPlacement.js:20 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G24 | blade scale span | 3.213 | src/terrain/GrassPlacement.js:21 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G25 | blades per cell | 79 | src/terrain/GrassPlacement.js:22 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G26 | clump radius (m) for blade offset | 1.15 | src/terrain/GrassPlacement.js:23 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G27 | foot-read iterations on mesh height | 2 | src/terrain/GrassPlacement.js:48 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G28 | density full radius (m) | 16 | src/terrain/GrassPlacement.js:27 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G29 | density falloff exponent P | 1.25 | src/terrain/GrassPlacement.js:28 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G30 | density fade (m) | 5 | src/terrain/GrassPlacement.js:29 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G31 | density lead (m) | 6 | src/terrain/GrassPlacement.js:30 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G32 | density edge margin (m) | 8 | src/terrain/GrassPlacement.js:31 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G33 | growth max scale of far blades | 1.3 | src/terrain/GrassPlacement.js:32 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G34 | growth per metre beyond full radius | 0.005 | src/terrain/GrassPlacement.js:33 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G35 | face-camera start distance (m) | 24 | src/terrain/GrassPlacement.js:34 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G36 | face-camera end distance (m) | 44 | src/terrain/GrassPlacement.js:35 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G37 | edge cap m = max(FULL_R+FADE+1, ring-EDGE) | const EDGE_CAP_M = Math.max(FULL_R + GRASS_DENSITY.FADE_M + 1, ringRadius - GRASS_DENSITY.EDGE_M) | client/core/Grass.js:71 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G38 | keep fraction law beyond fullR | (fullR/d)^P | src/terrain/GrassPlacement.js:45 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G39 | per-blade edge distance law | fullR/q^(1/P)+FADE | src/terrain/GrassPlacement.js:39 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G40 | blade accept: coin<density | if (coin >= p.density) return null | src/terrain/GrassPlacement.js:201 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G41 | cell density = grassDensity*soil*splat.grass*area | _clamp01(grassDensity(temp, humidity, slopeRatio) * soil * splat.grass * cell.area) | src/terrain/GrassPlacement.js:179 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G42 | grassDensity base: wet/warm/flat mix | (0.5+0.5*wet*(0.45+0.55*warm))*(0.35+0.65*flat) | src/terrain/GrassPlacement.js:111 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G43 | grassDensity contrast curve | 0.5+4*(base*biome-0.5) | src/terrain/GrassPlacement.js:114 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G44 | dry-biome smoothstep range | (wet-0.05)/0.15 | src/terrain/GrassPlacement.js:112 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G45 | reject when slope gradient > SLOPE_MAX | if (grad > GRASS.SLOPE_MAX) return null | src/terrain/GrassPlacement.js:174 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G46 | reject when sea bias < SEA_REJECT | clim.seaBias < GRASS.SEA_REJECT) return null | src/terrain/GrassPlacement.js:145 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G47 | reject when soil weight <= 0 | if (soil <= 0) return null | src/terrain/GrassPlacement.js:149 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G48 | cell shadow term | clamp01(0.55+0.45*ndl) | src/terrain/GrassPlacement.js:186 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G49 | fixed approx sun dir for cell shadow | (0.4,0.8,0.3) normalised | src/terrain/GrassPlacement.js:52 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G50 | LOD spread (m) | 3 | src/terrain/GrassPlacementLod.js:7 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G51 | LOD slack (m) | 4 | src/terrain/GrassPlacementLod.js:8 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G52 | LOD release (m) | 8 | src/terrain/GrassPlacementLod.js:9 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G53 | release flush fraction divisor | 16 | src/terrain/GrassPlacementLod.js:10 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G54 | release flush min | 64 | src/terrain/GrassPlacementLod.js:11 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G55 | chunk load budget per frame (ms) | 4 | client/core/Grass.js:555 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G56 | refine budget per frame (ms) | 2 | client/core/Grass.js:80 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G57 | cull entry budget per frame | 8192 | client/core/Grass.js:40 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G58 | LOD revisit step (m) | 4 | client/core/Grass.js:43 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G59 | LOD revisit budget | 2048 | client/core/Grass.js:44 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G60 | idle camera epsilon | 0.05 | client/core/Grass.js:636 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G61 | decal refresh interval (ms) | 2000 | client/core/Grass.js:706 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G62 | max benders (grass bending actors) | 8 | client/core/GrassMaterial.js:14 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G63 | max scorch decals | 8 | client/core/GrassMaterial.js:16 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G64 | bend radius (m) | 1.1 | client/core/GrassMaterial.js:59 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G65 | bend strength | 0.35 | client/core/GrassMaterial.js:60 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G66 | scorch shrink factor | 0.15 | client/core/GrassMaterial.js:63 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G67 | scorch colour | (0.22,0.15,0.06) | client/core/GrassMaterial.js:64 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G68 | alpha cutoff | 0.6 | client/core/GrassMaterial.js:20 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G69 | ground tint mix into root | 0.85 | client/core/GrassMaterial.js:22 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G70 | tip ground mix | 1.0 | client/core/GrassMaterial.js:24 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G71 | tip ground gain | 1.1 | client/core/GrassMaterial.js:26 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G72 | blade variation | 0.8 | client/core/GrassMaterial.js:30 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G73 | local variation | 0.6 | client/core/GrassMaterial.js:32 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G74 | base colour | (0.19,0.25,0.11) | client/core/GrassMaterial.js:50 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G75 | tip1 colour | (0.61,0.83,0.55) | client/core/GrassMaterial.js:51 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G76 | tip2 colour | (0.12,0.21,0.16) | client/core/GrassMaterial.js:52 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G77 | ambient colour default | (0.32,0.36,0.4) | client/core/GrassMaterial.js:56 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G78 | sun colour default | (1,1,0.96) | client/core/GrassMaterial.js:55 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G79 | wind direction default | (0.8,0.6) | client/core/GrassMaterial.js:53 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G80 | default wind strength uniform | 0.25 | client/core/GrassMaterial.js:53 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G81 | night floor | 0.18 | client/core/GrassMaterial.js:90 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G82 | terminator width | 0.45 | client/core/GrassMaterial.js:91 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G83 | exposure | 1.05 | client/core/GrassMaterial.js:92 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G84 | light gain night (dusk 1.08, day 0.67 on following lines) | 0.45 | client/core/GrassMaterial.js:93 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G85 | sun reference intensity | 1.5 | client/core/GrassMaterial.js:89 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G86 | alpha output | clamp(blade*1.25)*0.92 | client/core/GrassMaterial.js:317 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G87 | fragment discard on clump texture | blade<cutoff | client/core/GrassMaterial.js:290 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G88 | backlit (translucency) factor | 0.55 | client/core/GrassMaterial.js:309 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G89 | blade vertical gradient shading | 0.62+0.38*smoothstep(0,0.3,v) | client/core/GrassMaterial.js:300 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G90 | bend sag | 0.12*|bend|*gw (gw=gv^2*0.45) | client/core/GrassMaterial.js:237 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G91 | clump texture size | 256 | client/core/GrassClump.js:3 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G92 | blades per clump texture | 22 | client/core/GrassClump.js:4 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G93 | blade curve steps | 14 | client/core/GrassClump.js:6 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G94 | clump random seed | 0x2f6e2b1 | client/core/GrassClump.js:5 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G95 | blade height range in texture | 0.5+0.48*(1-|out|*0.55)*(0.65+0.35r) | client/core/GrassClump.js:52 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G96 | blade half width in texture | 0.03+0.022r | client/core/GrassClump.js:55 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G97 | clump texture anisotropy | 4 | client/core/GrassClump.js:67 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G98 | wind field cells | 64 | client/core/GrassWindField.js:3 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G99 | wind field extent (m) | 192 | client/core/GrassWindField.js:4 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G100 | wind field bands per update | 1 | client/core/GrassWindField.js:5 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G101 | wind field snap (m) | 24 | client/core/GrassWindField.js:6 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G102 | wind field reach (m) | 77.6 | client/core/GrassWindField.js:7 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G103 | gust noise scale | 0.06 | client/core/GrassWindField.js:10 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G104 | gust advect | 0.35 | client/core/GrassWindField.js:11 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G105 | gust phase gain | 5.5 | client/core/GrassWindField.js:12 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G106 | wave1 k (speed 1.4 next line) | 0.075 | client/core/GrassWindField.js:13 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G107 | wave2 kx (kz 0.09, speed 2.3, gain 0.5 follow) | -0.11 | client/core/GrassWindField.js:15 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G108 | sway base (flow 0.25 next line) | 0.2 | client/core/GrassWindField.js:19 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G109 | flutter x (z 0.25, speed 2.2, k 0.22, z-rate 0.7 follow) | 0.25 | client/core/GrassWindField.js:21 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G110 | wind field texel storage | half float RGBA (Uint16Array) | client/core/GrassWindField.js:63 | none (no grass in /workspace/bevy) | 3 | open |
| grb:G111 | grass disabled by cfg.grass===false | if (cfg.grass === false) return null | client/core/Grass.js:55 | none (no grass in /workspace/bevy) | 3 | open |
| grb:R01 | rock render distance (m) | 320 default | client/core/Rocks.js:92 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R02 | rock ring radius | renderDistance+40 | client/core/Rocks.js:93 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R03 | rock drop margin (m) | 64 | client/core/Rocks.js:21 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R04 | rock max instances | 12000 | client/core/Rocks.js:97 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R05 | tps-game rock config | rocks true, dist 320, max 12000 | apps/world/tps-game.js:12 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R06 | rock collider radius/cap (tps-game) | 32 / 128 | apps/world/tps-game.js:13 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R07 | rock load budget per frame (ms) | 2 | client/core/Rocks.js:185 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R08 | deferred chunk retries per frame | 4 | client/core/Rocks.js:186 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R09 | rock base seed | 1337 | client/core/Rocks.js:22 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R10 | rock shape SDF seed per type | base+type*7919 | client/core/Rocks.js:29 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R11 | rock shape types | 6 | src/terrain/RockPlacement.js:15 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R12 | rock chunk size (m) | 32 | src/terrain/RockPlacement.js:8 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R13 | rock cell size (m) | 8 | src/terrain/RockPlacement.js:9 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R14 | rock cells per chunk edge | 4 | src/terrain/RockPlacement.js:10 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R15 | rock jitter (m) | 3.2 | src/terrain/RockPlacement.js:11 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R16 | rock slope sample distance | 1.5 | src/terrain/RockPlacement.js:12 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R17 | rock water margin (m) | 0.3 | src/terrain/RockPlacement.js:13 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R18 | rock sea reject | -2 | src/terrain/RockPlacement.js:14 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R19 | rock density floor | 0.025 | src/terrain/RockPlacement.js:16 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R20 | rock scale min | 2.0 | src/terrain/RockPlacement.js:17 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R21 | rock scale span | 8.0 | src/terrain/RockPlacement.js:18 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R22 | rock squash min | 0.55 | src/terrain/RockPlacement.js:19 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R23 | rock squash span | 0.55 | src/terrain/RockPlacement.js:20 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R24 | rock scale quantisation levels | 10 | src/terrain/RockPlacement.js:23 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R25 | rock squash quantisation levels | 3 | src/terrain/RockPlacement.js:24 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R26 | rock scale law | MIN+SPAN*sc^3 biased by cluster | src/terrain/RockPlacement.js:152 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R27 | rock scale cluster bias | (cluster-0.5)*0.6 | src/terrain/RockPlacement.js:151 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R28 | rock tilt wobble | (rand(cellHash, K_TILTX) - 0.5) * 0.12 (+-0.06 on normal) | src/terrain/RockPlacement.js:154 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R29 | rock density law | (0.15+0.85*ero)*(0.4+1.6*slope^2)*arid*band | src/terrain/RockPlacement.js:63 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R30 | rock elevation band | 0.6+0.4*elevNorm | src/terrain/RockPlacement.js:62 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R31 | rock aridity boost | 1+(1-hum/ARIDITY_LINE) below line | src/terrain/RockPlacement.js:61 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R32 | rock patch size (cluster scale 224 next lines) | 112 | src/terrain/RockPlacement.js:80 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R33 | rock max cluster strength | 0.6 | src/terrain/RockPlacement.js:82 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R34 | rock patch factor | 0.25+1.5*seaNoise | src/terrain/RockPlacement.js:112 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R35 | rock cluster strength drops with slope | 0.6*(1-slopeRatio) | src/terrain/RockPlacement.js:145 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R36 | rock rejected under water margin | elev > WATER_MARGIN*reliefMargin | src/terrain/RockPlacement.js:125 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R37 | rock default climate erosion/humidity/temp | 0.3/0.5/0.5 | src/terrain/RockPlacement.js:84 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R38 | rock accept rule | coin<accept | src/terrain/RockPlacement.js:147 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R39 | rock treeline elevation normaliser | VEG.TREELINE | src/terrain/RockPlacement.js:129 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R40 | rock yaw | uniform 0..2pi | src/terrain/RockPlacement.js:153 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R41 | rock instance scale (x,y*squash,z) | scale,scale*squash,scale | client/core/Rocks.js:154 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R42 | rock tint law | 0.78+variant*0.4 capped 1 | client/core/Rocks.js:159 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R43 | rock rendering | single BatchedMesh, 1 draw call | client/core/Rocks.js:111 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R44 | rock mesh SDF cut count | 14+floor(r*8) | src/terrain/RockShapes.js:29 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R45 | rock cut radius | 1.4+1.1r | src/terrain/RockShapes.js:35 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R46 | rock base radius | 1.05 | src/terrain/RockShapes.js:41 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R47 | rock axis scale x | 0.75+0.5r (y 0.6+0.4r, z 0.75+0.5r) | src/terrain/RockShapes.js:38 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R48 | rock SDF clamp | 0.3 | src/terrain/RockShapes.js:48 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R49 | rock marching-cube bounds | +-1.5 | src/terrain/RockShapes.js:55 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R50 | rock mesh resolution | 16 | src/terrain/RockShapes.js:129 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R51 | rock hull shapes default | 6, seed 1337 | src/terrain/RockShapes.js:131 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R52 | rock material (WebGL) | MeshStandard white rough 0.95 metal 0 | client/core/Rocks.js:110 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R53 | rock tint noise octave 1.7 weight 0.6 (6.5/0.3, 23/0.1 same line) | 1.7/6.5/23 | client/core/Rocks.js:52 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R54 | rock colour ramp lo/hi | (0.30,0.28,0.25)/(0.66,0.62,0.56) | client/core/Rocks.js:53 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R55 | rock tint multiplier | 2.05 | client/core/Rocks.js:54 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R56 | rock TSL colour multiplier | 2.05 | client/core/RocksTSL.js:30 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:R57 | rock tile extent margin/lift | 2 / max(2,0.5*height) | client/core/Rocks.js:313 | none (no rocks in /workspace/bevy) | 3 | open |
| grb:B01 | target frame budget (ms, 60 fps) | 16.67 | client/core/QualityGovernor.js:3 | /workspace/bevy/wasm-viewer/src/lib.rs:433 frame interval bar 16.7 ms (display only, no governor) | 4 | open |
| grb:B02 | lower-quality dead band up | +1.0 ms | client/core/QualityGovernor.js:4 | none | 4 | open |
| grb:B03 | raise-quality dead band down | -3.0 ms | client/core/QualityGovernor.js:5 | none | 4 | open |
| grb:B04 | frames per window | 30 | client/core/QualityGovernor.js:8 | none | 4 | open |
| grb:B05 | windows before step | 2 | client/core/QualityGovernor.js:9 | none | 4 | open |
| grb:B06 | min step interval (ms) | 1000 | client/core/QualityGovernor.js:10 | none | 4 | open |
| grb:B07 | recover step interval (ms) | 400 | client/core/QualityGovernor.js:11 | none | 4 | open |
| grb:B08 | hitch threshold (ms) | 2*16.67=33.34 | client/core/QualityGovernor.js:12 | none | 4 | open |
| grb:B09 | hitch step interval (ms) | 500 | client/core/QualityGovernor.js:13 | none | 4 | open |
| grb:B10 | hitch response bound (frames) | 1 | client/core/QualityGovernor.js:14 | none | 4 | open |
| grb:B11 | window percentile | 0.95 | client/core/QualityGovernor.js:15 | none | 4 | open |
| grb:B12 | excursion window frames | 12 | client/core/QualityGovernor.js:18 | none | 4 | open |
| grb:B13 | excursion trip count | 8 | client/core/QualityGovernor.js:19 | none | 4 | open |
| grb:B14 | DPR controller budget (144 Hz, ms) | 6.94 | client/core/FrameMetrics.js:129 | none | 4 | open |
| grb:B15 | DPR controller frames per window | 45 | client/core/FrameMetrics.js:130 | none | 4 | open |
| grb:B16 | DPR lower/raise factors | 1.15 / 0.80 | client/core/FrameMetrics.js:131 | none | 4 | open |
| grb:B17 | DPR scale min/max/step | 0.40/1.0/0.08 | client/core/FrameMetrics.js:132 | none | 4 | open |
| grb:B18 | terrain VDRS scale min/max/step | 0.5/1.0/0.1 | client/core/FrameMetrics.js:152 | none | 4 | open |
| grb:B19 | profiler 60 fps reference (ms) | 16.67 | client/core/PerformanceProfiler.js:2 | /workspace/bevy/wasm-viewer/src/lib.rs:433 16.7 ms display reference | 4 | open |
| grb:B20 | streaming scheduler drain budget (ms) | 2 | client/core/StreamingScheduler.js:74 | none | 4 | open |
| grb:B21 | streaming max starts per drain | 32 | client/core/StreamingScheduler.js:75 | none | 4 | open |
| grb:B22 | occlusion query fast-frame skip (ms) | 6 | client/core/OcclusionQueryBudget.js:2 | none | 4 | open |
| grb:B23 | occlusion query slow frame (ms) | 20 | client/core/OcclusionQueryBudget.js:3 | none | 4 | open |
| grb:B24 | occlusion query max total budget | 96 | client/core/OcclusionQueryBudget.js:4 | none | 4 | open |
| grb:B25 | occlusion query min total budget | 24 | client/core/OcclusionQueryBudget.js:5 | none | 4 | open |
| grb:B26 | occlusion query per consumer floor | 4 | client/core/OcclusionQueryBudget.js:6 | none | 4 | open |
| grb:B27 | orbit hitch threshold (ms) | 33.4 | scripts/planet-orbit-hitch-probe.mjs:3 | none | 4 | open |
| grb:B28 | orbit severe hitch threshold (ms) | 50 | scripts/planet-orbit-hitch-probe.mjs:4 | none | 4 | open |
| grb:B29 | frame-time gate regression threshold | x1.10 baseline | scripts/frame-time-gate.mjs:32 | none | 4 | open |
| grb:B30 | frame-time gate capture length (ms) | 8000 | scripts/frame-time-gate.mjs:45 | none | 4 | open |
| grb:B31 | frame-time gate vegetation floor | 1000 instances | scripts/frame-time-gate.mjs:47 | none | 4 | open |
| grb:B32 | frame-time gate settle budget (ms) | argvNumber(process.argv.slice(2), 'settle-budget-ms', 240_000) (240000) | scripts/frame-time-gate.mjs:58 | none | 4 | open |
| grb:B33 | frame-time gate settle worst-frame factor | 4 | scripts/frame-time-gate.mjs:52 | none | 4 | open |
| grb:B34 | frame-time gate settle min probe samples | 40 | scripts/frame-time-gate.mjs:53 | none | 4 | open |
| grb:B35 | frame-time gate orbit pose radius (m) | 8 | scripts/frame-time-gate.mjs:434 | none | 4 | open |
| grb:B36 | frame-time gate static pose position (x,z) | -15,-12.5 | scripts/frame-time-gate.mjs:39 | none | 4 | open |
| grb:B37 | orbit-hitch probe altitude default (m) | 40 | scripts/planet-orbit-hitch-probe.mjs:12 | none | 4 | open |
| grb:B38 | orbit-hitch probe duration (s) | 60 | scripts/planet-orbit-hitch-probe.mjs:11 | none | 4 | open |
| grb:B39 | orbit-hitch probe orbit period (s) | 45 | scripts/planet-orbit-hitch-probe.mjs:14 | none | 4 | open |
| grb:B40 | orbit-hitch probe orbit radius | R + altM | scripts/planet-orbit-hitch-probe.mjs:23 | none | 4 | open |
| grb:B41 | default planet radius (m) | 63600 | src/presets/planet.js:6 | none | 4 | open |
| grb:B42 | default planet anchor direction | (-0.641,0.2558,0.7237) | src/presets/planet.js:5 | none | 4 | open |
| grb:B43 | default planet max level | 13 | src/presets/planet.js:8 | none | 4 | open |
| grb:B44 | default planet relief scale | 0.001 | src/presets/planet.js:7 | none | 4 | open |
| grb:B45 | default planet seed | 1337 | src/presets/planet.js:12 | none | 4 | open |
| grb:B46 | terrain physics extent/resolution | 256 / 2 | src/presets/planet.js:11 | none | 4 | open |
| grb:B47 | tps-game spawn point | [-25,8.39,-45] | apps/world/tps-game.js:50 | none | 4 | open |
| grb:B48 | tps-game spawn-sillos-1 | [-25,8.39,-45] | apps/world/tps-game.js:44 | none | 4 | open |
| grb:B49 | tps-game spawn-sillos-2 | [15,8.39,-35] | apps/world/tps-game.js:45 | none | 4 | open |
| grb:B50 | tps-game spawn-sillos-3 | [-17.5,8.39,-45] | apps/world/tps-game.js:46 | none | 4 | open |
| grb:B51 | tps-game spawn-sillos-4 | [-12.5,8.39,-7.5] | apps/world/tps-game.js:47 | none | 4 | open |
| grb:B52 | tps-game vegetation render distance | 640 | apps/world/tps-game.js:9 | none | 4 | open |
| grb:B53 | tps-game physics body budget | 512 | apps/world/tps-game.js:21 | none | 4 | open |
| grb:B54 | tps-game weather particle count | 3000 | apps/world/tps-game.js:7 | none | 4 | open |
| grb:B55 | bookmark far poses (km out) | 5,20,100 | src/shared/bookmarks.js:5 | none | 4 | open |
| grb:B56 | search bookmarks (shoreline/forest/rocks equivalents) | coast, hills, forest, rocks | src/shared/bookmarks.js:6 | none | 4 | open |
| grb:B57 | bookmark pole pose | pole-north [0,1,0] | src/shared/bookmarks.js:44 | none | 4 | open |
| grb:B58 | bookmark reachable angle below anchor (deg) | 87 | src/shared/bookmarks.js:7 | none | 4 | open |
| grb:B59 | bookmark origin pose | [0,0,0] | src/shared/bookmarks.js:40 | none | 4 | open |

## Open rows

Values the reference source does not state, or that the table cannot yet confirm. Each closes by the witness of its stage or stays recorded as absent from the reference.

| id | what | finding | status |
|---|---|---|---|
| geo:nf1 | Quadtree split hysteresis (a different threshold for merge than for split): searched quadtree.js, lod-config.js, tsl/planet-tsl.js, planet-orchestrator*.js for hysteresis, merge, hyst. The only hysteresis in the reference is the collider keep radius factor (bud.colHyst) and the TSL quad cache reuse tolerance (spl.cacheMove, spl.cacheDot). | not found in the reference source | open |
| geo:nf2 | Per-frame cap on the number of quadtree leaves or instances: searched for maxQuads, MAX_QUADS, budget, capacity in planet-tsl.js, webgpu/patch-grid-render.js, webgpu/planet-orchestrator-webgpu.js. Only a growable instance buffer exists (grd.cap, grd.capGrow); the near-field tile budget is the leaf count of the near face (nf.budgetUse). | not found in the reference source | open |
| geo:nf3 | Per-frame cap on patch bakes or height tile uploads in the live TSL path: searched patch-baker*.js and tsl/. Only the remote baker in-flight and unclaimed caps exist (bud.bakeInflight, bud.bakeUnclaimed). | not found in the reference source | open |
| geo:nf4 | Collider level-of-detail that varies with distance or level: searched src/terrain/TerrainPhysics.js, HeightfieldStreamer.js, ColliderStreamer.js, src/shared/colliderLattice.js. The collider is one fixed N x N lattice (col.n) with no levels. | not found in the reference source | open |
| geo:nf5 | A vertex-count figure per patch fixed in code: only derived from the grid formula (grd.vertsPer, grd.indices), not a stored constant. | not found in the reference source | open |
| geo:nf6 | Normal derivation with nrmStepM: the default is 0 (nrm.nrmStep) and no code path reading it was found in tsl/ or webgpu/ (grep of nrmStepM only hit terrain-defaults.js). | not found in the reference source | open |
| geo:nf7 | GPU patch grid cell count in the webgpu orchestrator: it does not pass gridSize, so the PatchGridRenderer default of 16 applies (grd.webgpuDef); no explicit setting found. | not found in the reference source | open |
| geo:nf8 | Port equivalents for the quadtree, skirts, geomorph, culling, near-field lattice, face warp, WGSL height evaluation and finite-difference normals: searched src/globe.rs, src/height/*.rs, src/*.rs for quadtree, skirt, geomorph, split_factor, max_level, near_field, wgsl; none exist, and the port column states none for those rows. | not found in the reference source | open |
| mat:nf1 | Terrain roughness value: the terrain material sets only metalness (packages/mapspinner/src/tsl/terrain-material-tsl.js:224) and a custom LightingModel (packages/mapspinner/src/tsl/terrain-material-tsl.js:218); searched roughness/metalness in tsl/*.js, terrain-material-tsl.js, terrain.glsl and defaults, found no roughness number. | not found in the reference source | open |
| mat:nf2 | Terrain texture budget (memory/count cap): searched 'budget' in packages/mapspinner/src (only planet-tsl.js hits, near-lattice quad budget, not textures) and client/core/TreeTextureBudget.js is for trees; no terrain texture budget constant exists beyond SZ=1024 x 4 layers. | not found in the reference source | open |
| mat:nf3 | Per-layer tiling scale differing by layer: only one texTile (2400) and fixed x4/x1 octaves; searched surface-splat-tsl.js, terrain-defaults.js, glsl. | not found in the reference source | open |
| mat:nf4 | Rock normals file: no rock-normals.jpg; rock layer uses ground-normals.jpg (found, packages/mapspinner/src/surface-texture-decode.js:48). | not found in the reference source | open |
| mat:nf5 | Per-biome (temperature/humidity) ground colour in the live TSL: only temp for sea ice and dryHot sand; humid/temp biome class colours exist only in debug GLSL (terrain.glsl:584-595). | not found in the reference source | open |
| wsl:nf1 | Planet-specific overrides of fog, sun, ambient, exposure or sea level in src/presets/planet.js: searched the whole file (15 lines; only radius 63600, reliefScale, maxLevel 13, physics, seed). Defaults therefore come from client/core/SceneSetup.js, TimeOfDay.js, terrain-defaults.js, RenderControls.js. | not found in the reference source | open |
| wsl:nf2 | A numeric planet sea level other than 0: searched seaLevel/SEA_LEVEL/sea_level in apps, packages/mapspinner/src, src, client; only the waterLevel.js default 0, water-render.js hR 0.0 and the water-tsl.js 1.0 m discard threshold exist. | not found in the reference source | open |
| wsl:nf3 | Fog tied to the planet terrain TSL path: no fog constants in terrain-lighting-tsl.js; planet haze is hazeMul 0.4 plus AP_GATE_KM; scene fog (FogExp2 0.0025) is the only fog constant found. | not found in the reference source | open |
| wsl:nf4 | Per-quality-preset toneMappingExposure values: RenderControls.js:92 says device-tier presets lower it, but the preset values were not read (QualityPresets.js not opened). | not found in the reference source | open |
| wsl:nf5 | Bevy-side tonemapping, exposure, fog, ambient and cascade settings: absent (Camera3d::default() only, viewer.rs), so the port column states none. | not found in the reference source | open |
| wsl:nf6 | Bevy atmosphere, water and sky modules: grep of /workspace/bevy/src found no render code for them. | not found in the reference source | open |
| veg:nf1 | Per-species weights: none exist. Species come from climate rules (species.genus), companion and understory chances, and size classes; searched VegPlacement.js, Vegetation.js, VegetationBuild.js. | not found in the reference source | open |
| veg:nf2 | A literal "more than 30,000 trees" statement: searched the repo for 30,000, 30000, 30k with tree/veg/instan context. Only the maxInstances cap of 30000 exists. | not found in the reference source | open |
| veg:nf3 | Height gating other than treeline and soil weight: no separate height band per species. | not found in the reference source | open |
| veg:nf4 | Biome gating by name: none; gating is climate temp/humidity/erosion, painted grass weight and snow veto. | not found in the reference source | open |
| veg:nf5 | Culling budgets for the live vegetation path: only the synthetic A/B counts exist in WebGPUCullingHubIntegration.js; no per-frame cull budget found there. | not found in the reference source | open |
| grb:nf1 | Per-frame GPU upload byte limits: searched client/core/WebGPUInstancing.js, Grass.js, Rocks.js for upload/budget/perFrame; only capacity doubling exists, no byte cap. | not found in the reference source | open |
| grb:nf2 | Worker/async budgets: client/workers holds only OffscreenRenderWorker.js with no numeric budget found; macrotaskYield has no numeric budget found. | not found in the reference source | open |
| grb:nf3 | Named camera presets (orbit, high altitude, low altitude, forest ground, shoreline): none exist as presets; only bookmarks (B55-B59), hitch probe altitude (B37), gate poses (B35, B36) and tps-game spawns (B47-B51). Numeric high/low altitude poses: not found. | not found in the reference source | open |
| grb:nf4 | Render-stat caps: grep of client/core/RuntimeStats.js, FrameMetrics.js, PerformanceProfiler.js for budget/limit found no fixed render-stat limits beyond those tabulated. | not found in the reference source | open |
| grb:nf5 | Bevy planet camera/spawn poses: grep of /workspace/bevy .rs for orbit/altitude/spawn_point found only the demo camera (wasm-viewer/src/lib.rs:154 from_xyz(0,5.2,11), :317 orbit radius 11); no planet poses. | not found in the reference source | open |
| mea:tris-tier | triangle counts after meshopt simplification per species and tier | measured in stage 3 | open |
| mea:eye | eye height of the poses | read in stage 1 | open |

## Removed rows

Rows dropped because the cited line does not state the value.

| id | reason |
|---|---|
| geo:col.stream | removed: port-only row with no reference source line (reference value n/a) |
| geo:col.span | removed: port-only row with no reference source line (reference value n/a) |
