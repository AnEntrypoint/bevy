import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    mesh_view_bindings::view,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
};

struct TerrainParams {
    planet_center: vec3<f32>,
    def_radius: f32,
    tex_cam_frac: vec3<f32>,
    relief_scale: f32,
    sun_dir: vec3<f32>,
    wetness: f32,
    bc_rock: vec3<f32>,
    pool_cover: f32,
    mean_l: vec4<f32>,
    pool_lo: vec4<f32>,
    pool_hi: vec4<f32>,
    pool_spec: vec4<f32>,
    albedo_override: vec4<f32>,
    surf_ready: f32,
    tex_nrm_k: f32,
    relief_shade: f32,
    flat_normal: f32,
    atm_ready: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> terrain: TerrainParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var surf_alb: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var surf_alb_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var surf_nrm: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(104) var surf_nrm_sampler: sampler;

const PLANET_RADIUS_M = 6360000.0;
const ATM_BOTTOM_KM = 6360.0;

const BC_SHORE = vec3<f32>(0.82, 0.76, 0.52);
const BC_LOWLAND = vec3<f32>(0.18, 0.38, 0.10);
const BC_GRASS = vec3<f32>(0.30, 0.46, 0.12);
const BC_SNOW = vec3<f32>(0.94, 0.96, 1.00);
const BAND_EDGES_LO = vec2<f32>(8.0, 20.0);
const BAND_EDGES_HI = vec2<f32>(60.0, 100.0);
const SNOW_EDGES = vec2<f32>(120.0, 180.0);
const SLOPE_ROCK = vec2<f32>(0.25, 0.55);
const BAND_WARP = 20.0;
const BEACH_TOP = 15.0;
const BEACH_WIDTH = 1.0;
const VARIATION_AMT = 0.05;
const SEA_FLOOR_SILT = vec3<f32>(0.12, 0.11, 0.09);
const SEA_FLOOR_BASALT = vec3<f32>(0.06, 0.06, 0.07);
const SEA_ICE_COLOUR = vec3<f32>(0.82, 0.88, 0.94);
const LUMA = vec3<f32>(0.299, 0.587, 0.114);
const RIDGE_ALBEDO_FREQ = vec2<f32>(75.0, 375.0);
const RIDGE_ALBEDO_AMP = vec2<f32>(1.0, 0.6);

const SAND_REGION_SLOPE = vec2<f32>(0.18, 0.55);
const SAND_SLOPE_FADE = vec2<f32>(0.30, 0.70);
const DRY_HOT_ARIDITY = vec2<f32>(0.60, 0.85);
const DRY_HOT_TEMP = vec2<f32>(0.42, 0.62);
const ROCK_BAND_SNOW_FRAC = vec2<f32>(0.7, 0.9);
const SNOW_SUPPRESSED_BY_ROCK = 0.6;
const BAND_WARP_FREQ = vec2<f32>(1100.0, 2580.0);
const WEIGHT_EPS = 1e-4;
const BAND_WARP_SPLAT_SCALE = 0.25;
const BAND_WARP_SPLAT_RANGE = BAND_WARP * BAND_WARP_SPLAT_SCALE * 1.5;

const TEX_MIX = 1.0;
const TEX_NRM_LOW = 0.4;
const TRI_SHARP = 3.0;
const TEX_SAT = 1.2;
const TEX_BRIGHT = 1.12;
const BIOME_TINT = 0.62;
const TEX_WARP = 1.0;
const TEX_WARP_FREQ = 450.0;
const TEX_WARP_AMP = 1.2;
const X_SOFT = 0.26;
const X_FINGER = 0.2;
const ORD_PUSH = 0.0;
const X_FADE = vec2<f32>(100.0, 340.0);
const TEX_FAR = vec2<f32>(0.0, 10000.0);
const NRM_FADE = vec2<f32>(100.0, 1000.0);
const OCT_FAR = vec2<f32>(1.5, 15.0);
const TEX_TILE_M = 2400.0;
const OCTAVE_COLLAPSE = 1.0;
const FINE_OCTAVE_SCALE = 4.0;
const COARSE_OCTAVE_SCALE = 1.0;
const NORMAL_TOWARD_UP = 0.05;
const WET_DARKEN = 0.65;

const LATTICE_MUL_X = 0x8da6b343u;
const LATTICE_MUL_Y = 0xd8163841u;
const LATTICE_MUL_Z = 0xcb1ab31fu;
const AVALANCHE_MUL_A = 0x7feb352du;
const AVALANCHE_MUL_B = 0x846ca68bu;
const UNIT_24BIT = 1.0 / 16777216.0;

const EXPOSURE = 1.05;
const SKY_FILL = 0.3;
const HAZE_MUL = 0.4;
const TERMINATOR_GLOW = 0.6;
const NIGHT_LIGHTS = 0.8;
const NIGHT_FLOOR = 0.18;
const TERM_WIDTH = 0.45;
const LOOK_SAT = 1.25;
const LOOK_CONTRAST = 1.0;
const SKY_HAZE_TINT = vec3<f32>(0.40, 0.55, 0.78);
const TERMINATOR_TINT = vec3<f32>(1.0, 0.55, 0.34);
const NIGHT_FILL = vec3<f32>(0.06, 0.075, 0.11);
const AP_GATE_KM = vec2<f32>(3.0, 120.0);
const TERMINATOR_FADE_SIN = 0.17364817766693033;
const ACES_FILMIC = vec3<f32>(2.51, 0.03, 2.43);
const ACES_FILMIC_TAIL = vec2<f32>(0.59, 0.14);
const LUMA_DISPLAY = vec3<f32>(0.2126, 0.7152, 0.0722);
const DISPLAY_GAMMA_INV = 0.45454545454545453;

@if(HASH_FLOAT)
fn lattice_hash(p: vec3<f32>) -> f32 {
    let q0 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    let q = q0 + dot(q0, q0.yxz + 33.33);
    return fract((q.x + q.y) * q.z) * 2.0 - 1.0;
}
@else
fn lattice_hash(p: vec3<f32>) -> f32 {
    let mixed = (bitcast<u32>(i32(p.x)) * LATTICE_MUL_X) ^ (bitcast<u32>(i32(p.y)) * LATTICE_MUL_Y) ^ (bitcast<u32>(i32(p.z)) * LATTICE_MUL_Z);
    let a = (mixed ^ (mixed >> 16u)) * AVALANCHE_MUL_A;
    let b = (a ^ (a >> 15u)) * AVALANCHE_MUL_B;
    let h = b ^ (b >> 16u);
    return f32(h >> 8u) * UNIT_24BIT * 2.0 - 1.0;
}

fn snoise3(P: vec3<f32>) -> f32 {
    let fl = floor(P);
    let f = P - fl;
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let i1 = fl + 1.0;
    let n000 = lattice_hash(fl);
    let n100 = lattice_hash(vec3<f32>(i1.x, fl.y, fl.z));
    let n010 = lattice_hash(vec3<f32>(fl.x, i1.y, fl.z));
    let n110 = lattice_hash(vec3<f32>(i1.x, i1.y, fl.z));
    let n001 = lattice_hash(vec3<f32>(fl.x, fl.y, i1.z));
    let n101 = lattice_hash(vec3<f32>(i1.x, fl.y, i1.z));
    let n011 = lattice_hash(vec3<f32>(fl.x, i1.y, i1.z));
    let n111 = lattice_hash(i1);
    let x00 = mix(n000, n100, u.x);
    let x10 = mix(n010, n110, u.x);
    let x01 = mix(n001, n101, u.x);
    let x11 = mix(n011, n111, u.x);
    return mix(mix(x00, x10, u.y), mix(x01, x11, u.y), u.z);
}

@if(CENSUS_REMOVE_LAYER)
fn remove_layer(w: vec4<f32>) -> vec4<f32> {
    var r = w;
    r[constants::CENSUS_REMOVE_LAYER] = -1.0;
    return r;
}
@else
fn remove_layer(w: vec4<f32>) -> vec4<f32> {
    return w;
}

fn by_layer4(lay: f32, v0: vec3<f32>, v1: vec3<f32>, v2: vec3<f32>, v3: vec3<f32>) -> vec3<f32> {
    return select(select(select(v3, v2, lay < 2.5), v1, lay < 1.5), v0, lay < 0.5);
}

fn by_layer1(lay: f32, v: vec4<f32>) -> f32 {
    return select(select(select(v.w, v.z, lay < 2.5), v.y, lay < 1.5), v.x, lay < 0.5);
}

fn terrain_albedo_climate(h: f32, rock_slope: f32, temp: f32, nwp: vec3<f32>, px_world: f32, bc_rock: vec3<f32>) -> vec3<f32> {
    let rock_widen = smoothstep(20.0, 500.0, px_world) * 0.20;
    let depth_t = clamp(-h / 300.0, 0.0, 1.0);
    let bed0 = mix(BC_SHORE, SEA_FLOOR_SILT, smoothstep(0.0, 0.5, depth_t));
    let bed = mix(bed0, SEA_FLOOR_BASALT, smoothstep(0.5, 1.0, depth_t));
    let sea_floor = mix(bed, bc_rock, smoothstep(SLOPE_ROCK.x, SLOPE_ROCK.y, rock_slope));
    let sea_ice = 1.0 - smoothstep(0.12, 0.22, temp);
    let sea_out = mix(sea_floor, SEA_ICE_COLOUR, sea_ice * 0.9);

    let land0 = mix(mix(BC_SHORE, BC_LOWLAND, smoothstep(0.0, BAND_EDGES_LO.x, h)), BC_GRASS, smoothstep(BAND_EDGES_LO.x, BAND_EDGES_LO.y, h));
    let bww = nwp + vec3<f32>(snoise3(nwp * 130.0)) * 0.004;
    let band_warp = (snoise3(bww * 210.0) + snoise3(bww * 560.0) * 0.5 + snoise3(bww * 1450.0) * 0.25) * BAND_WARP;
    let land1 = mix(land0, bc_rock, smoothstep(band_warp + BAND_EDGES_HI.x, band_warp + BAND_EDGES_HI.y, h));
    let land2 = mix(land1, BC_SNOW, smoothstep(band_warp + SNOW_EDGES.x, band_warp + SNOW_EDGES.y, h));
    let land3 = mix(land2, bc_rock, smoothstep(SLOPE_ROCK.x, rock_widen + SLOPE_ROCK.y, rock_slope) * step(0.0, h));
    let mottled = land3 * (snoise3(nwp * 120.0) * VARIATION_AMT + 1.0);
    let beach_m = (1.0 - smoothstep(BEACH_TOP * 0.3, BEACH_TOP, h)) * (1.0 - smoothstep(SLOPE_ROCK.x, SLOPE_ROCK.y, rock_slope));
    let beached = mix(mottled, BC_SHORE, beach_m);
    var ridge_sum = 0.0;
    var ridge_amp = 0.0;
    for (var o = 0u; o < 2u; o = o + 1u) {
        let fq = RIDGE_ALBEDO_FREQ[o];
        let am = RIDGE_ALBEDO_AMP[o];
        let wl = terrain.relief_scale * (40000000.0 / fq);
        let nyq = 1.0 - smoothstep(wl * 0.03, wl * 0.12, px_world);
        ridge_sum = ridge_sum + nyq * am * snoise3(nwp * fq + vec3<f32>(f32(o) * 7.3));
        ridge_amp = ridge_amp + am;
    }
    let land_out = beached * (ridge_sum / ridge_amp * 0.02 + 1.0);
    return select(land_out, sea_out, h < 0.0);
}

fn splat_band_warp(dir0: vec3<f32>) -> f32 {
    return (snoise3(dir0 * BAND_WARP_FREQ.x) + snoise3(dir0 * BAND_WARP_FREQ.y) * 0.5) * (BAND_WARP * BAND_WARP_SPLAT_SCALE);
}

fn splat_weights(h: f32, slope: f32, rock_slope: f32, humid: f32, temp: f32, dir0: vec3<f32>) -> vec4<f32> {
    let sr_lo = max(SLOPE_ROCK.x, 0.05);
    let sr_hi = max(SLOPE_ROCK.y, sr_lo + 0.25);
    let dry_hot = smoothstep(DRY_HOT_ARIDITY.x, DRY_HOT_ARIDITY.y, 1.0 - humid) * smoothstep(DRY_HOT_TEMP.x, DRY_HOT_TEMP.y, temp);
    let band_warp = splat_band_warp(dir0);
    let bw_pos = max(band_warp, 0.0);
    let beach = (1.0 - smoothstep(bw_pos, bw_pos + BEACH_TOP * BEACH_WIDTH, h)) * (1.0 - smoothstep(SAND_REGION_SLOPE.x, SAND_REGION_SLOPE.y, slope));
    let sand_region = clamp(max(dry_hot, beach), 0.0, 1.0);
    let w_rock_slope = smoothstep(mix(sr_lo, 0.50, sand_region), mix(sr_hi, 0.70, sand_region), rock_slope);
    let snow_hi = smoothstep(band_warp + SNOW_EDGES.x, band_warp + SNOW_EDGES.y, h);
    let rock_band = smoothstep(band_warp + SNOW_EDGES.x * ROCK_BAND_SNOW_FRAC.x, band_warp + SNOW_EDGES.x * ROCK_BAND_SNOW_FRAC.y, h) * (1.0 - snow_hi);
    let w_rock = max(w_rock_slope, rock_band);
    let w_snow = clamp(snow_hi, 0.0, 1.0) * (1.0 - SNOW_SUPPRESSED_BY_ROCK * w_rock);
    let w_sand = sand_region * (1.0 - w_rock) * (1.0 - w_snow) * (1.0 - smoothstep(SAND_SLOPE_FADE.x, SAND_SLOPE_FADE.y, slope));
    let w_grass = max(1.0 - w_rock - w_snow - w_sand, 0.0);
    let uw_m = 1.0 - smoothstep(BEACH_TOP * 0.3, BEACH_TOP, h);
    let wz = w_sand + (w_grass + w_snow) * uw_m;
    let wx = w_grass * (1.0 - uw_m);
    let ww = w_snow * (1.0 - uw_m);
    let sum = wx + w_rock + wz + ww + WEIGHT_EPS;
    return vec4<f32>(wx, w_rock, wz, ww) / sum;
}

struct PlaneGrads {
    p_yz: vec2<f32>, p_xz: vec2<f32>, p_xy: vec2<f32>,
    dx_yz: vec2<f32>, dx_xz: vec2<f32>, dx_xy: vec2<f32>,
    dy_yz: vec2<f32>, dy_xz: vec2<f32>, dy_xy: vec2<f32>,
}

fn triplanar_grads(wt: vec3<f32>, wt_dx: vec3<f32>, wt_dy: vec3<f32>, scale: f32) -> PlaneGrads {
    let p = wt * scale;
    let dx = wt_dx * scale;
    let dy = wt_dy * scale;
    return PlaneGrads(p.yz, p.xz, p.xy, dx.yz, dx.xz, dx.xy, dy.yz, dy.xz, dy.xy);
}

fn tri_tap(tex: texture_2d_array<f32>, samp: sampler, g: PlaneGrads, bw: vec3<f32>, layer: i32) -> vec4<f32> {
    return textureSampleGrad(tex, samp, g.p_yz, layer, g.dx_yz, g.dy_yz) * bw.x
        + textureSampleGrad(tex, samp, g.p_xz, layer, g.dx_xz, g.dy_xz) * bw.y
        + textureSampleGrad(tex, samp, g.p_xy, layer, g.dx_xy, g.dy_xy) * bw.z;
}

fn tri_nrm(g: PlaneGrads, bw: vec3<f32>, layer: i32, n: vec3<f32>) -> vec3<f32> {
    let px = textureSampleGrad(surf_nrm, surf_nrm_sampler, g.p_yz, layer, g.dx_yz, g.dy_yz).rg * 2.0 - 1.0;
    let py = textureSampleGrad(surf_nrm, surf_nrm_sampler, g.p_xz, layer, g.dx_xz, g.dy_xz).rg * 2.0 - 1.0;
    let pz = textureSampleGrad(surf_nrm, surf_nrm_sampler, g.p_xy, layer, g.dx_xy, g.dy_xy).rg * 2.0 - 1.0;
    return vec3<f32>(0.0, px.x, px.y) * (bw.x * sign(n.x))
        + vec3<f32>(py.x, 0.0, py.y) * (bw.y * sign(n.y))
        + vec3<f32>(pz.x, pz.y, 0.0) * (bw.z * sign(n.z));
}

struct LayerSample {
    nrm: vec3<f32>,
    mat_color: vec3<f32>,
    ord: f32,
    detail: vec3<f32>,
    disp: f32,
    pool: f32,
}

struct SplatResult {
    top_layer: f32,
    albedo: vec3<f32>,
    tex_dn: vec3<f32>,
    pool: f32,
}

struct SplatInputs {
    n: vec3<f32>,
    h: f32,
    slope: f32,
    rock_slope: f32,
    humid: f32,
    temp: f32,
    dir0: vec3<f32>,
    biome_c: vec3<f32>,
    px_world: f32,
    cam_dist: f32,
    wt: vec3<f32>,
    wt_dx: vec3<f32>,
    wt_dy: vec3<f32>,
}

fn layer_sample(l: f32, g_fine: PlaneGrads, g_coarse: PlaneGrads, tw: vec3<f32>, n: vec3<f32>, oct_far_fade: f32, octaves_coincide: bool, tex_fade: f32) -> LayerSample {
    let li = i32(l + 0.5);
    let alb_fine = tri_tap(surf_alb, surf_alb_sampler, g_fine, tw, li);
    var col = alb_fine.rgb;
    if (oct_far_fade != 0.0 && !octaves_coincide) {
        col = mix(alb_fine.rgb, tri_tap(surf_alb, surf_alb_sampler, g_coarse, tw, li).rgb, oct_far_fade);
    }
    var nrm = vec3<f32>(0.0);
    if (tex_fade != 0.0) {
        let n_fine = tri_nrm(g_fine, tw, li, n);
        var n_second = n_fine;
        if (!octaves_coincide) {
            n_second = tri_nrm(g_coarse, tw, li, n);
        }
        nrm = n_fine + n_second * (1.7 * TEX_NRM_LOW);
    }
    let mat_color = by_layer4(l, BC_GRASS, terrain.bc_rock, BC_SHORE, BC_SNOW);
    let ord = select(select(select(1.0, 0.0, l < 2.5), 0.3, l < 1.5), 0.6, l < 0.5);
    let mean_l = by_layer1(l, terrain.mean_l);
    let sat = max(mix(vec3<f32>(dot(col, LUMA)), col, TEX_SAT), vec3<f32>(0.0));
    let detail = sat * (dot(mat_color, LUMA) / max(mean_l, 0.02));
    let disp = alb_fine.a;
    let pool = 1.0 - smoothstep(by_layer1(l, terrain.pool_lo), by_layer1(l, terrain.pool_hi), disp);
    return LayerSample(nrm, mat_color, ord, detail, disp, pool);
}

fn surface_splat(s: SplatInputs) -> SplatResult {
    let rs = terrain.relief_scale;
    let tex_far_fade = 1.0 - smoothstep(rs * TEX_FAR.x, rs * TEX_FAR.y, s.px_world);
    let w4 = remove_layer(splat_weights(s.h, s.slope, s.rock_slope, s.humid, s.temp, s.dir0));

    var l_a = 0.0;
    var w_a = w4.x;
    var l_b = 0.0;
    var w_b = -1.0;
    let candidates_l = vec3<f32>(1.0, 2.0, 3.0);
    let candidates_w = w4.yzw;
    for (var i = 0u; i < 3u; i = i + 1u) {
        let lay = candidates_l[i];
        let wl = candidates_w[i];
        let beats_a = wl > w_a;
        let beats_b = wl > w_b;
        let new_l_b = select(select(l_b, lay, beats_b), l_a, beats_a);
        let new_w_b = select(select(w_b, wl, beats_b), w_a, beats_a);
        l_a = select(l_a, lay, beats_a);
        w_a = select(w_a, wl, beats_a);
        l_b = new_l_b;
        w_b = new_w_b;
    }

    let tex_tile_m = terrain.def_radius / PLANET_RADIUS_M * TEX_TILE_M;
    let tw0 = pow(abs(s.n), vec3<f32>(TRI_SHARP));
    let tw = tw0 / (tw0.x + tw0.y + tw0.z + 1e-4);
    let b_ab = clamp(w_a / max(w_a + w_b, 1e-4), 0.0, 1.0);
    let g_fine = triplanar_grads(s.wt, s.wt_dx, s.wt_dy, FINE_OCTAVE_SCALE);
    let g_coarse = triplanar_grads(s.wt, s.wt_dx, s.wt_dy, COARSE_OCTAVE_SCALE);
    let octaves_coincide = s.px_world >= tex_tile_m / f32(textureDimensions(surf_alb).x) && s.px_world <= tex_tile_m * 0.25 && OCTAVE_COLLAPSE > 0.5;
    let oct_far_fade = smoothstep(rs * OCT_FAR.x, rs * OCT_FAR.y, s.px_world);
    let tex_fade = 1.0 - smoothstep(NRM_FADE.x, NRM_FADE.y, s.cam_dist);
    let cross_fade = 1.0 - smoothstep(X_FADE.x, X_FADE.y, s.cam_dist);

    var albedo = s.biome_c;
    var tex_dn = vec3<f32>(0.0);
    var pool = terrain.pool_cover;
    let k = tex_far_fade * TEX_MIX * terrain.surf_ready;
    if (terrain.surf_ready > 0.5 && tex_far_fade > 0.001) {
        let a = layer_sample(l_a, g_fine, g_coarse, tw, s.n, oct_far_fade, octaves_coincide, tex_fade);
        var detail = a.detail;
        var mat_color = a.mat_color;
        var tex_nrm = a.nrm;
        var pool_tex = a.pool;
        if (w_b > 0.02) {
            let b = layer_sample(l_b, g_fine, g_coarse, tw, s.n, oct_far_fade, octaves_coincide, tex_fade);
            let finger = (a.disp - b.disp) * X_FINGER * cross_fade;
            let b_sharp = smoothstep(-X_SOFT, X_SOFT, (b_ab - 0.5) * 2.0 + (a.ord - b.ord) * ORD_PUSH + finger);
            detail = mix(b.detail, a.detail, b_sharp);
            mat_color = mix(b.mat_color, a.mat_color, b_sharp);
            tex_nrm = mix(b.nrm, a.nrm, b_sharp);
            pool_tex = mix(b.pool, a.pool, b_sharp);
        }
        pool = mix(terrain.pool_cover, pool_tex, k);
        let albedo0 = clamp(mix(mat_color, detail, k), vec3<f32>(0.0), vec3<f32>(1.0));
        let tinted = mix(albedo0, s.biome_c, BIOME_TINT * (clamp(w4.z, 0.0, 1.0) * -0.85 + 1.0));
        albedo = mix(s.biome_c, tinted * TEX_BRIGHT, tex_far_fade);
        let safe_nrm = select(vec3<f32>(0.0), tex_nrm / max(length(tex_nrm), 1e-6), dot(tex_nrm, tex_nrm) > 1e-12);
        tex_dn = safe_nrm * (k * terrain.tex_nrm_k) * tex_fade;
    }
    return SplatResult(l_a, albedo, tex_dn, pool);
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    return select(pow((c + 0.055) / 1.055, vec3<f32>(2.4)), c / 12.92, c <= vec3<f32>(0.04045));
}

struct AerialPerspective {
    trans: vec3<f32>,
    radiance: vec3<f32>,
}

fn aerial_perspective(rel: vec3<f32>, d_km: f32) -> AerialPerspective {
    if (terrain.atm_ready < 0.5 || d_km <= AP_GATE_KM.x) {
        return AerialPerspective(vec3<f32>(1.0), vec3<f32>(0.0));
    }
    let camera = (view.world_position - terrain.planet_center) * (ATM_BOTTOM_KM / terrain.def_radius);
    let marched = atm_march_radiance(camera, rel / max(length(rel), 1e-4), terrain.sun_dir, max(d_km, 1e-3));
    return AerialPerspective(marched.trans, marched.radiance);
}

fn grade_output(lit: vec3<f32>, up: vec3<f32>, rel: vec3<f32>) -> vec3<f32> {
    let d_km = length(rel * (ATM_BOTTOM_KM / terrain.def_radius));
    let ap_gate = smoothstep(AP_GATE_KM.x, AP_GATE_KM.y, d_km);
    let ap = aerial_perspective(rel, d_km);
    let ap_inscat = max(ap.radiance, SKY_FILL * SKY_HAZE_TINT * (vec3<f32>(1.0) - ap.trans));
    let nw_sun = dot(up, terrain.sun_dir);
    let graze = smoothstep(0.55, 1.0, 1.0 - abs(nw_sun));
    let glow = TERMINATOR_GLOW * graze * graze * smoothstep(-0.02, 0.18, nw_sun) * smoothstep(0.0, TERMINATOR_FADE_SIN, nw_sun) * ap_gate;
    let hazed = lit * ap.trans + ap_inscat + TERMINATOR_TINT * glow;
    let color = mix(lit, hazed, ap_gate * HAZE_MUL);
    let day_shade = mix(NIGHT_FLOOR, 1.0, smoothstep(-TERM_WIDTH, TERM_WIDTH, nw_sun));
    let night_amt = 1.0 - smoothstep(-TERM_WIDTH, 0.0, nw_sun);
    let c = (color * day_shade + NIGHT_FILL * NIGHT_LIGHTS * (1.0 - day_shade) * (1.0 - night_amt)) * EXPOSURE;
    let aces = clamp(c * (c * ACES_FILMIC.x + ACES_FILMIC.y) / (c * (c * ACES_FILMIC.z + ACES_FILMIC_TAIL.x) + ACES_FILMIC_TAIL.y), vec3<f32>(0.0), vec3<f32>(1.0));
    let sat = mix(vec3<f32>(dot(aces, LUMA_DISPLAY)), aces, LOOK_SAT);
    let graded = clamp((sat - 0.5) * LOOK_CONTRAST + 0.5, vec3<f32>(0.0), vec3<f32>(1.0));
    return srgb_to_linear(pow(graded, vec3<f32>(DISPLAY_GAMMA_INV)));
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    let rel = in.world_position.xyz - view.world_position;
    let px_world = max(length(fwidth(rel)), 0.001);
    let cam_dist = length(rel);
    let dir0 = normalize(in.world_position.xyz - terrain.planet_center);
    let h = in.uv_b.x;
    let temp = in.uv.x;
    let humid = in.uv.y;
    let n_geo = normalize(in.world_normal);
    let n = select(normalize(mix(n_geo, dir0, NORMAL_TOWARD_UP)), dir0, terrain.flat_normal > 0.5);
    let slope = 1.0 - max(0.0, dot(n, dir0));
    let rock_slope = clamp(slope, 0.0, 1.0);
    let wet = select(0.0, terrain.wetness, h > 0.0);

    let warp_base = dir0 * TEX_WARP_FREQ;
    let warp = vec3<f32>(snoise3(warp_base), snoise3(warp_base + vec3<f32>(7.3)), snoise3(warp_base + vec3<f32>(23.9))) * TEX_WARP_AMP;
    let tex_tile_m = terrain.def_radius / PLANET_RADIUS_M * TEX_TILE_M;
    let wt = (rel + terrain.tex_cam_frac) / tex_tile_m + warp * TEX_WARP;
    let wt_dx = dpdx(wt);
    let wt_dy = dpdy(wt);

    let biome_c = terrain_albedo_climate(h, rock_slope, temp, dir0, px_world, terrain.bc_rock);
    let splat = surface_splat(SplatInputs(n, h, slope, rock_slope, humid, temp, dir0, biome_c, px_world, cam_dist, wt, wt_dx, wt_dy));

    let pool_p = splat.pool * (1.0 - smoothstep(terrain.pool_spec.z, terrain.pool_spec.w, slope)) * wet;
    let albedo = mix(splat.albedo, terrain.albedo_override.rgb, terrain.albedo_override.a) * mix(1.0, WET_DARKEN, wet);
    let n_relief = select(n, normalize(dir0 + (n - dir0) * terrain.relief_shade), terrain.relief_shade > 1.0);
    let n_lit = normalize(n_relief + splat.tex_dn);

    let pool_fresnel = pow(1.0 - max(dot(n_lit, pbr_input.V), 0.0), 5.0) * 0.98 + 0.02;
    let pool_weight = pool_fresnel * pool_p;
    let spec_exp = mix(terrain.pool_spec.x, terrain.pool_spec.y, pool_p);

    pbr_input.material.base_color = vec4<f32>(albedo, 1.0);
    pbr_input.material.metallic = 0.0;
    pbr_input.material.reflectance = vec3<f32>(0.5 * pool_weight);
    pbr_input.material.perceptual_roughness = mix(1.0, pow(2.0 / (spec_exp + 2.0), 0.25), pool_weight);
    pbr_input.N = n_lit;

    var out: FragmentOutput;
    let lit = apply_pbr_lighting(pbr_input);
    out.color = vec4<f32>(grade_output(lit.rgb, dir0, rel), 1.0);
@if(LAYER_CENSUS)
    out.color = vec4<f32>(splat.top_layer / 255.0, splat_band_warp(dir0) / BAND_WARP_SPLAT_RANGE * 0.5 + 0.5, 0.0, 1.0);
@if(!LAYER_CENSUS)
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
