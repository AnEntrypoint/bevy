struct SkyFrame {
  view_to_planet: mat3x3<f32>,
  proj_diag: vec2<f32>,
  fade: f32,
  exposure: f32,
  cam_atm: vec3<f32>,
  aces: f32,
  sun_dir: vec3<f32>,
  pad: f32,
}

@group(0) @binding(0) var<uniform> sky: SkyFrame;

const SKY_TINT: vec3<f32> = vec3<f32>(0.82, 0.95, 1.22);
const SKY_EXPOSURE_HIGH_SUN: f32 = 14.0;
const SKY_EXPOSURE_LOW_SUN: f32 = 48.0;
const SKY_SATURATION: f32 = 1.3;
const SKY_HALO_COLOR: vec3<f32> = vec3<f32>(0.32, 0.55, 1.0);
const SKY_MARCH_STEPS: u32 = 8u;
const SKY_SUN_DISC_GAIN: f32 = 6.0;
const SKY_HALO_GAIN: f32 = 0.03;

struct SkyVertexOut {
  @builtin(position) clip: vec4<f32>,
  @location(0) ndc: vec2<f32>,
}

@vertex
fn sky_vertex(@builtin(vertex_index) vid: u32) -> SkyVertexOut {
  var p: vec2<f32>;
  p.x = select(-1.0, 3.0, vid == 1u);
  p.y = select(-1.0, 3.0, vid == 2u);
  var out: SkyVertexOut;
  out.clip = vec4<f32>(p, 0.0, 1.0);
  out.ndc = p;
  return out;
}

fn sky_radiance(ndc: vec2<f32>) -> vec3<f32> {
  let view_dir = normalize(vec3<f32>(ndc.x / sky.proj_diag.x, ndc.y / sky.proj_diag.y, -1.0));
  let ray = normalize(sky.view_to_planet * view_dir);
  let sun = sky.sun_dir;
  let cam_in = sky.cam_atm;
  let r_in = length(cam_in);
  let dt_in = atm_dist_to_top(r_in, dot(cam_in, ray) / r_in);
  var camera = cam_in;
  if (r_in > ATM_TOP && dt_in > 0.0) {
    camera = cam_in + ray * dt_in;
  }
  let r = length(camera);
  let mu = dot(camera, ray) / r;
  let mu_tangent = -sqrt(max(1.0 - ATM_BOTTOM2 / (r * r), 0.0));
  let w_sky = smoothstep(mu_tangent - ATM_HORIZON_BLEND_MU, mu_tangent + ATM_HORIZON_BLEND_MU, mu);
  let d_top = atm_dist_to_top(r, mu);
  let d_end = mix(max(atm_dist_to_ground_continuous(r, mu), 1e-3), d_top, w_sky);
  let marched = atm_march_steps(camera, ray, sun, d_end, SKY_MARCH_STEPS);
  let sky_visible = d_top > 0.0 && (r_in <= ATM_TOP || dt_in >= 0.0);
  var radiance_base = vec3<f32>(0.0);
  var trans = vec3<f32>(1.0);
  if (sky_visible) {
    radiance_base = marched.radiance;
    trans = mix(vec3<f32>(0.0), marched.trans, w_sky);
  }

  let rc = length(cam_in);
  let muc = dot(cam_in, ray) / rc;
  let b = rc * sqrt(max(1.0 - muc * muc, 0.0));
  let t0 = (b - ATM_BOTTOM) / (ATM_TOP - ATM_BOTTOM);
  var halo = 0.0;
  if (muc < 0.0) {
    halo = smoothstep(0.0, 0.06, t0) * (1.0 - smoothstep(0.25, 1.6, t0));
  }
  let limb_dir = normalize(cam_in + ray * (-rc * muc));
  let lit = smoothstep(-0.5, 0.6, dot(limb_dir, sun)) * 0.75 + 0.25;
  var sun_disc = vec3<f32>(0.0);
  if (dot(ray, sun) > cos(ATM_SUN_ANGULAR_RADIUS)) {
    sun_disc = trans * ATM_SOLAR_IRRADIANCE * SKY_SUN_DISC_GAIN;
  }
  return radiance_base + SKY_HALO_COLOR * (halo * lit * SKY_HALO_GAIN) + sun_disc;
}

@fragment
fn sky_fragment(in: SkyVertexOut) -> @location(0) vec4<f32> {
  let radiance = sky_radiance(in.ndc);
  let sun_elev_dot = clamp(dot(sky.sun_dir, normalize(sky.cam_atm)), 0.0, 1.0);
  let c = radiance * SKY_TINT * mix(SKY_EXPOSURE_LOW_SUN, SKY_EXPOSURE_HIGH_SUN, sun_elev_dot);
  let aces = aces_display_curve(c);
  let mapped = clamp(mix(vec3<f32>(dot(aces, LUMA)), aces, SKY_SATURATION), vec3<f32>(0.0), vec3<f32>(1.0));
  let display = pow(mapped, vec3<f32>(DISPLAY_GAMMA_INV)) * sky.fade;
  return vec4<f32>(display_referred_to_scene_linear(display, sky.exposure, sky.aces > 0.5), 1.0);
}

const PROBE_COLUMNS: f32 = 40.0;
const PROBE_ROWS: f32 = 180.0;
const PROBE_MU_S_COUNT: f32 = 8.0;
const PROBE_MU_S_RANGE: vec2<f32> = vec2<f32>(-0.9, 0.9);
const PROBE_AERIAL_DISTANCE_KM: f32 = 20.0;
const PROBE_DISPLAY_SCALE: f32 = 3.0;

fn probe_r(frag: vec2<f32>) -> f32 {
  return ATM_BOTTOM + (ATM_TOP - ATM_BOTTOM) * (frag.x % PROBE_COLUMNS) / PROBE_COLUMNS;
}

fn probe_mu(frag: vec2<f32>) -> f32 {
  return -1.0 + 2.0 * frag.y / PROBE_ROWS;
}

fn probe_mu_s(frag: vec2<f32>) -> f32 {
  return mix(PROBE_MU_S_RANGE.x, PROBE_MU_S_RANGE.y, floor(frag.x / PROBE_COLUMNS) / (PROBE_MU_S_COUNT - 1.0));
}

@fragment
fn probe_transmittance(in: SkyVertexOut) -> @location(0) vec4<f32> {
  return vec4<f32>(atm_transmittance(probe_r(in.clip.xy), probe_mu(in.clip.xy)), 1.0);
}

@fragment
fn probe_scattering(in: SkyVertexOut) -> @location(0) vec4<f32> {
  return atm_scattering(probe_r(in.clip.xy), probe_mu(in.clip.xy), probe_mu_s(in.clip.xy));
}

fn probe_aerial(frag: vec2<f32>) -> AtmRadiance {
  let r = probe_r(frag);
  let mu = probe_mu(frag);
  let mu_s = probe_mu_s(frag);
  let camera = vec3<f32>(0.0, r, 0.0);
  let ray = vec3<f32>(sqrt(max(1.0 - mu * mu, 0.0)), mu, 0.0);
  let sun = vec3<f32>(sqrt(max(1.0 - mu_s * mu_s, 0.0)), mu_s, 0.0);
  return atm_march_radiance(camera, ray, sun, PROBE_AERIAL_DISTANCE_KM);
}

@fragment
fn probe_aerial_radiance(in: SkyVertexOut) -> @location(0) vec4<f32> {
  return vec4<f32>(probe_aerial(in.clip.xy).radiance, 1.0);
}

@fragment
fn probe_aerial_transmittance(in: SkyVertexOut) -> @location(0) vec4<f32> {
  return vec4<f32>(probe_aerial(in.clip.xy).trans, 1.0);
}

@fragment
fn probe_radiance(in: SkyVertexOut) -> @location(0) vec4<f32> {
  return vec4<f32>(sky_radiance(in.ndc), 1.0);
}

@fragment
fn probe_display(in: SkyVertexOut) -> @location(0) vec4<f32> {
  let frag = in.clip.xy;
  let scene = vec3<f32>(frag.x / PROBE_COLUMNS / PROBE_MU_S_COUNT, frag.y / PROBE_ROWS, 1.0 - frag.x / PROBE_COLUMNS / PROBE_MU_S_COUNT) * PROBE_DISPLAY_SCALE;
  return vec4<f32>(scene_linear_to_display_referred(scene, sky.exposure, sky.aces > 0.5), 1.0);
}
