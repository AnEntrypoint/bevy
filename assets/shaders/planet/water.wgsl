struct View {
  clip_from_world: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(24) var view_transmission_texture: texture_2d<f32>;
@group(0) @binding(25) var view_transmission_sampler: sampler;

struct Water {
  origin_in_render: vec3<f32>,
  time: f32,
  camera_in_render: vec3<f32>,
  sea_mean: f32,
  sun_dir: vec3<f32>,
  planet_radius: f32,
  cam_mod: vec3<f32>,
  amp: f32,
  cam_dir: vec3<f32>,
  choppy: f32,
  cam_alt: f32,
  foam: f32,
  haze_mul: f32,
  underwater: f32,
  viewport: vec2<f32>,
  exposure: f32,
  aces: f32,
  census_mode: f32,
}

@group(3) @binding(0) var<uniform> water: Water;

const SLOPE_EPS_M: f32 = 0.25;
const SLOPE_EPS_LF_M: f32 = 1.25;
const MARCH_NEAR_M: f32 = 140.0;
const MARCH_STEPS: i32 = 5;
const BISECT_MAX: i32 = 5;
const LAND_ABOVE_SEA_DISCARD_M: f32 = 1.0;
const SEA_OCTAVES: i32 = 8;
const SEA_FREQ: f32 = 0.2;
const SEA_SPEED: f32 = 2.4;
const SEA_AMP_M: f32 = 0.6;
const SEA_OCTAVE_ANGLE: f32 = 1.57079633;
const SEA_FREQ_LF: f32 = 0.16;
const SKY_ZENITH: vec3<f32> = vec3<f32>(0.15, 0.45, 0.95);
const SKY_HORIZON: vec3<f32> = vec3<f32>(0.55, 0.75, 1.00);
const DEEP_COLOR: vec3<f32> = vec3<f32>(0.04, 0.34, 0.52);
const ABSORPTION: vec3<f32> = vec3<f32>(0.09, 0.028, 0.012);
const FOAM_COLOR: vec3<f32> = vec3<f32>(0.95, 0.98, 1.0);

struct WaterVertexIn {
  @location(0) position: vec3<f32>,
  @location(1) dir: vec3<f32>,
  @location(2) land: vec2<f32>,
}

struct WaterVertexOut {
  @builtin(position) clip: vec4<f32>,
  @location(0) land_height: f32,
  @location(1) dir: vec3<f32>,
  @location(2) rel: vec3<f32>,
}

@vertex
fn water_vertex(in: WaterVertexIn) -> WaterVertexOut {
  let render_position = in.position + water.origin_in_render;
  var out: WaterVertexOut;
  out.clip = view.clip_from_world * vec4<f32>(render_position, 1.0);
  out.land_height = in.land.x;
  out.dir = in.dir;
  out.rel = render_position - water.camera_in_render;
  return out;
}

fn sea_hash(p: vec2<f32>) -> f32 {
  let q = vec2<u32>(vec2<i32>(p)) * vec2<u32>(1597334673u, 3812015801u);
  let n = (q.x ^ q.y) * 1597334673u;
  return f32(n) * (1.0 / 4294967296.0);
}

fn sea_noise(p: vec2<f32>) -> f32 {
  let i = floor(p);
  let f0 = p - i;
  let f = f0 * f0 * (f0 * -2.0 + 3.0);
  return mix(
    mix(sea_hash(i), sea_hash(i + vec2<f32>(1.0, 0.0)), f.x),
    mix(sea_hash(i + vec2<f32>(0.0, 1.0)), sea_hash(i + vec2<f32>(1.0, 1.0)), f.x),
    f.y
  );
}

fn sea_octave(uv_in: vec2<f32>, choppy: f32) -> f32 {
  let uv = uv_in + sea_noise(uv_in);
  let wv0 = 1.0 - abs(sin(uv));
  let wv = mix(wv0, abs(cos(uv)), wv0);
  return pow(1.0 - pow(wv.x * wv.y, 0.65), choppy);
}

fn sea_height(p: vec2<f32>, t: f32, o_start: i32, o_end: i32, amp0: f32, choppy0: f32) -> f32 {
  var freq = SEA_FREQ;
  var amp = amp0 * SEA_AMP_M;
  var choppy = choppy0;
  var h = 0.0;
  let sea_time = vec2<f32>(t * (SEA_SPEED * 0.6), t * (SEA_SPEED * 0.4));
  for (var i = 0; i < SEA_OCTAVES; i++) {
    if (i >= o_start && i < o_end) {
      let a = f32(i) * SEA_OCTAVE_ANGLE;
      let sa = sin(a);
      let ca = cos(a);
      let duv = vec2<f32>(ca * p.x + sa * p.y, ca * p.y - sa * p.x);
      let ts = 1.0 - f32(i) * 0.08;
      h += sea_octave((duv + sea_time * ts) * freq, choppy) * amp;
    }
    freq *= 1.9;
    amp *= 0.45;
    choppy = mix(choppy, 1.0, 0.3);
  }
  return h;
}

fn sea_height_lf(p: vec2<f32>, t: f32, amp: f32, choppy: f32) -> f32 {
  let q = p * SEA_FREQ_LF;
  let a = sea_octave(q + vec2<f32>(0.866, 0.5) * (t * (SEA_SPEED * 0.6)), choppy);
  let b = sea_octave(q + vec2<f32>(-0.5, 0.866) * (t * (SEA_SPEED * 0.4)), choppy);
  return (a + b) * (amp * (SEA_AMP_M * 0.5));
}

fn sea_slope_lf(p: vec2<f32>) -> vec2<f32> {
  let lf = sea_height_lf(p, water.time, water.amp, water.choppy);
  return vec2<f32>(
    sea_height_lf(p + vec2<f32>(SLOPE_EPS_LF_M, 0.0), water.time, water.amp, water.choppy) - lf,
    sea_height_lf(p + vec2<f32>(0.0, SLOPE_EPS_LF_M), water.time, water.amp, water.choppy) - lf
  ) / SLOPE_EPS_LF_M;
}

fn sea_slope(p: vec2<f32>, o0: i32, o1: i32) -> vec2<f32> {
  let h = sea_height(p, water.time, o0, o1, water.amp, water.choppy);
  let hx = sea_height(p + vec2<f32>(SLOPE_EPS_M, 0.0), water.time, o0, o1, water.amp, water.choppy);
  let hy = sea_height(p + vec2<f32>(0.0, SLOPE_EPS_M), water.time, o0, o1, water.amp, water.choppy);
  return vec2<f32>(hx - h, hy - h) / SLOPE_EPS_M;
}

struct TangentBasis {
  ux: vec3<f32>,
  uy: vec3<f32>,
  uz: vec3<f32>,
}

fn tangent_basis(dir: vec3<f32>) -> TangentBasis {
  let uz = normalize(dir);
  let ux = normalize(cross(select(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0), abs(uz.y) < 0.99), uz));
  return TangentBasis(ux, cross(uz, ux), uz);
}

fn water_normal(basis: TangentBasis, slope: vec2<f32>) -> vec3<f32> {
  return normalize(basis.uz - basis.ux * slope.x - basis.uy * slope.y);
}

const CENSUS_STEP_PX_M: f32 = 1.9;
const CENSUS_RANGE: f32 = 8.0;
const CENSUS_FIXED_POINT: f32 = 16777215.0;

fn census_value(mode: u32, p: vec2<f32>, basis: TangentBasis) -> f32 {
  let t = water.time;
  switch mode {
    case 0u: { return sea_height(p, t, 0, SEA_OCTAVES, water.amp, water.choppy); }
    case 1u: { return sea_height(p, t, 2, 6, water.amp, water.choppy); }
    case 2u: { return sea_height_lf(p, t, water.amp, water.choppy); }
    case 3u: { return sea_slope(p, 0, SEA_OCTAVES).x; }
    case 4u: { return sea_slope(p, 0, SEA_OCTAVES).y; }
    case 5u: { return sea_slope_lf(p).x; }
    case 6u: { return sea_slope_lf(p).y; }
    case 7u: { return dot(water_normal(basis, sea_slope(p, 0, SEA_OCTAVES)), basis.ux); }
    case 8u: { return dot(water_normal(basis, sea_slope(p, 0, SEA_OCTAVES)), basis.uy); }
    case 9u: { return dot(water_normal(basis, sea_slope(p, 0, SEA_OCTAVES)), basis.uz); }
    case 10u: { return basis.ux.x; }
    case 11u: { return basis.ux.y; }
    case 12u: { return basis.ux.z; }
    case 13u: { return basis.uy.x; }
    case 14u: { return basis.uy.y; }
    default: { return basis.uy.z; }
  }
}

fn census_encode(value: f32) -> vec3<f32> {
  let fixed = floor(clamp((value + CENSUS_RANGE) / (2.0 * CENSUS_RANGE), 0.0, 1.0) * CENSUS_FIXED_POINT);
  let high = floor(fixed / 65536.0);
  let middle = floor((fixed - high * 65536.0) / 256.0);
  return vec3<f32>(high, middle, fixed - high * 65536.0 - middle * 256.0) / 255.0;
}

@fragment
fn water_census_fragment(in: WaterVertexOut) -> @location(0) vec4<f32> {
  let value = census_value(u32(water.census_mode), in.clip.xy * CENSUS_STEP_PX_M, tangent_basis(in.dir));
  return vec4<f32>(census_encode(value), 1.0);
}

@fragment
fn water_fragment(in: WaterVertexOut) -> @location(0) vec4<f32> {
  if (in.land_height > LAND_ABOVE_SEA_DISCARD_M) {
    discard;
  }
  let planet_r = water.planet_radius;
  let t = water.time;
  let sun = water.sun_dir;
  let basis = tangent_basis(in.dir);
  let uz = basis.uz;
  let ux = basis.ux;
  let uy = basis.uy;
  let flat_dist = length(in.rel);
  let ray_dir = in.rel / max(flat_dist, 1e-4);
  let view_w = -ray_dir;
  let wp_flat = vec2<f32>(dot(in.rel + water.cam_mod, ux), dot(in.rel + water.cam_mod, uy));
  var color = vec3<f32>(0.0);

  if (water.underwater > 0.5) {
    let slope_w = sea_slope(wp_flat, 0, SEA_OCTAVES);
    let wn = water_normal(basis, slope_w);
    let ndl = max(dot(wn, sun), 0.0);
    let depth_atten = exp(max(0.0, -water.cam_alt) * -0.0005);
    var wcol = vec3<f32>(0.005, 0.06, 0.18) + vec3<f32>(1.0, 0.6, 0.3) * (depth_atten * ndl * 0.4) + vec3<f32>(0.0, 0.02, 0.06) * length(slope_w);
    let snell = smoothstep(0.50, 0.82, abs(dot(view_w, wn)));
    wcol = mix(wcol, vec3<f32>(0.40, 0.60, 0.85) * (ndl * 0.9 + 0.5), snell * 0.85);
    wcol += vec3<f32>(1.0, 0.92, 0.70) * pow(max(dot(view_w, sun), 0.0), 180.0) * (snell * 0.6 + 0.4);
    color = aces_display(wcol);
  } else {
    let cam_uz = dot(water.cam_dir, uz);
    let dc = water.cam_dir - uz;
    let ray_h0 = water.cam_alt * cam_uz - planet_r * 0.5 * dot(dc, dc);
    let ray_dir_uz = dot(ray_dir, uz);
    var t_hit = flat_dist;
    var wp_w = wp_flat;
    if (flat_dist <= MARCH_NEAR_M) {
      var t_min = 0.0;
      var t_max = flat_dist * 1.5;
      let step_t = t_max / f32(MARCH_STEPS);
      var found = 0.0;
      for (var i = 1; i < MARCH_STEPS + 1; i++) {
        let ti = step_t * f32(i);
        let q = water.cam_mod + ray_dir * ti;
        let gap = ray_h0 + ti * ray_dir_uz - (sea_height(vec2<f32>(dot(q, ux), dot(q, uy)), t, 0, SEA_OCTAVES, water.amp, water.choppy) - water.sea_mean);
        if (found < 0.5 && gap < 0.0) {
          t_min = ti - step_t;
          t_max = ti;
          found = 1.0;
        }
      }
      let bisect_n = i32(clamp(5.0 - (flat_dist - 30.0) / 36.0, 2.0, 5.0));
      for (var i = 0; i < BISECT_MAX; i++) {
        if (i < bisect_n) {
          let t_mid = (t_min + t_max) * 0.5;
          let q = water.cam_mod + ray_dir * t_mid;
          let gap = ray_h0 + t_mid * ray_dir_uz - (sea_height(vec2<f32>(dot(q, ux), dot(q, uy)), t, 0, SEA_OCTAVES, water.amp, water.choppy) - water.sea_mean);
          if (gap < 0.0) {
            t_max = t_mid;
          } else {
            t_min = t_mid;
          }
        }
      }
      t_hit = (t_min + t_max) * 0.5;
      let q = water.cam_mod + ray_dir * t_hit;
      wp_w = vec2<f32>(dot(q, ux), dot(q, uy));
    }
    let near_fade = clamp(1.0 - t_hit / 120.0, 0.0, 1.0);
    let far_fade = clamp((t_hit - 30.0) / 60.0, 0.0, 1.0) * clamp(1.0 - t_hit / 300.0, 0.0, 1.0);
    let near_start = i32(clamp(2.0 - t_hit / 40.0, 0.0, 2.0));
    let near_end = i32(clamp(8.0 - t_hit / 60.0, 6.0, 8.0));
    let slope_far = sea_slope_lf(wp_flat);
    let slope_w = sea_slope(wp_w, near_start, near_end) * near_fade * clamp(-in.land_height / 6.0, 0.0, 1.0) + slope_far * far_fade;
    let wn = water_normal(basis, slope_w);
    let refl_dir = reflect(ray_dir, wn);
    let refl_sun = max(dot(refl_dir, sun), 0.0);
    let sky_refl = mix(SKY_ZENITH, SKY_HORIZON, pow(1.0 - max(dot(refl_dir, uz), 0.0), 3.0))
      + vec3<f32>(1.0, 0.9, 0.7) * (pow(refl_sun, 8.0) * 0.8)
      + vec3<f32>(1.0, 0.85, 0.55) * (pow(refl_sun, 16.0) * 0.4);
    let n_o_v = max(dot(wn, view_w), 0.0);
    let n_o_l = max(dot(wn, sun), 0.0);
    let refr_w = 1.0 - smoothstep(250.0, 350.0, t_hit);
    let screen_uv = in.clip.xy / water.viewport;
    let refr_uv = clamp(screen_uv + vec2<f32>(slope_w.x, -slope_w.y) * (refr_w * n_o_v * 0.36), vec2<f32>(0.001), vec2<f32>(0.999));
    let refr_scene = scene_linear_to_display_referred(textureSampleLevel(view_transmission_texture, view_transmission_sampler, refr_uv, 0.0).rgb, water.exposure, water.aces > 0.5);
    let refr_col = mix(refr_scene, DEEP_COLOR, step(dot(refr_scene, vec3<f32>(1.0)), 0.01));
    let absorb = exp(-ABSORPTION * (max(-in.land_height, 0.0) / max(n_o_v, 0.10)));
    let water_body = refr_col * absorb * 0.40 + DEEP_COLOR * (vec3<f32>(1.0) - absorb);
    let fresnel = pow(1.0 - n_o_v, 5.0) * 0.98 + 0.02;
    let refl_w = fresnel * (1.0 - dot(absorb, vec3<f32>(1.0 / 3.0)));
    var wcol = mix(water_body, mix(DEEP_COLOR, sky_refl, 0.7), refl_w) + pow(refl_sun, 64.0) * 0.5;
    let foam_noise = sea_noise(wp_w * 0.7 + vec2<f32>(t * 0.02, t * -0.03));
    let foam_uv = wp_w * 4.0 + vec2<f32>(t * 1.4, t * 0.9);
    let patch_mask = mix(
      foam_noise,
      mix(sea_octave(foam_uv, water.choppy), sea_octave(foam_uv * 0.5 + vec2<f32>(1.3, 2.7), water.choppy * 0.7), 0.4),
      0.6
    );
    let crest = clamp((length(slope_w) - 0.45) * 2.0, 0.0, 1.0);
    let foam_amt = clamp(crest * (patch_mask * 0.6 + 0.4) * water.foam, 0.0, 1.0);
    wcol = mix(wcol, FOAM_COLOR, foam_amt * (n_o_l * 0.35 + 0.15));
    let fog_w = t_hit / (t_hit + max(planet_r, 1.0));
    wcol = mix(wcol, mix(SKY_HORIZON, SKY_ZENITH, max(dot(view_w, uz), 0.0)) * 0.5, fog_w * water.haze_mul);
    color = aces_display(wcol);
  }
  return vec4<f32>(display_referred_to_scene_linear(color, water.exposure, water.aces > 0.5), 1.0);
}
