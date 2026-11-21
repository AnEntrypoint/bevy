@group(ATM_LUT_GROUP) @binding(ATM_LUT_BINDING_TRANSMITTANCE) var atm_transmittance_lut: texture_2d<f32>;
@group(ATM_LUT_GROUP) @binding(ATM_LUT_BINDING_SCATTERING) var atm_scattering_lut: texture_2d_array<f32>;
@group(ATM_LUT_GROUP) @binding(ATM_LUT_BINDING_SAMPLER) var atm_lut_sampler: sampler;

struct AtmRadiance {
  radiance: vec3<f32>,
  trans: vec3<f32>,
}

fn atm_transmittance(r: f32, mu: f32) -> vec3<f32> {
  return textureSampleLevel(atm_transmittance_lut, atm_lut_sampler, atm_lut_uv(r, mu), 0.0).rgb;
}

fn atm_scattering(r: f32, mu: f32, mu_s: f32) -> vec4<f32> {
  let lf = atm_layer_from_mu_s(mu_s);
  let l0 = floor(lf);
  let l1 = min(l0 + 1.0, ATM_SCAT_LAYERS - 1.0);
  let uv = atm_lut_uv(r, mu);
  let s0 = textureSampleLevel(atm_scattering_lut, atm_lut_sampler, uv, i32(l0), 0.0);
  let s1 = textureSampleLevel(atm_scattering_lut, atm_lut_sampler, uv, i32(l1), 0.0);
  return mix(s0, s1, lf - l0);
}

fn atm_transmittance_to_sun(p: vec3<f32>, sun: vec3<f32>) -> vec3<f32> {
  let r = length(p);
  let mu = dot(p, sun) / r;
  let mu_horizon = -sqrt(max(1.0 - ATM_BOTTOM2 / (r * r), 0.0));
  return atm_transmittance(r, mu) * smoothstep(mu_horizon - 0.035, mu_horizon + 0.005, mu);
}

fn atm_transmittance_to_sun_at_ground(up: vec3<f32>, sun: vec3<f32>) -> vec3<f32> {
  let ground_r = ATM_BOTTOM + 0.5;
  let mu = dot(up, sun);
  let mu_horizon = -sqrt(max(1.0 - ATM_BOTTOM2 / (ground_r * ground_r), 0.0));
  return atm_transmittance(ground_r, mu) * smoothstep(mu_horizon - 0.035, mu_horizon + 0.005, mu);
}

fn atm_scattered_radiance(sun_irradiance: vec3<f32>, inscat_r: vec3<f32>, inscat_m: vec3<f32>, nu: f32) -> vec3<f32> {
  return sun_irradiance * (inscat_r * ATM_RAYLEIGH * atm_rayleigh_phase(nu) + inscat_m * ATM_MIE_SCAT * atm_mie_phase(nu));
}

fn atm_march_radiance(camera: vec3<f32>, ray: vec3<f32>, sun: vec3<f32>, d_end: f32) -> AtmRadiance {
  let r = length(camera);
  let mu = dot(camera, ray) / r;
  let mu_s = dot(camera, sun) / r;
  let nu = dot(ray, sun);
  let d_top = atm_dist_to_top(r, mu);
  let full = atm_scattering(r, mu, mu_s);
  let t_full = atm_transmittance(r, mu);
  let p_end = camera + ray * d_end;
  let r_end = length(p_end);
  let mu_end = dot(p_end, ray) / r_end;
  let tail = atm_scattering(r_end, mu_end, dot(p_end, sun) / r_end);
  let t_to_end = t_full / max(atm_transmittance(r_end, mu_end), vec3<f32>(1e-6));
  let has_tail = d_end < d_top - 1e-4;
  var inscat_r = full.rgb;
  var inscat_m = full.a;
  var trans = t_full;
  if (has_tail) {
    inscat_r = max(full.rgb - t_to_end * tail.rgb, vec3<f32>(0.0));
    inscat_m = max(full.a - dot(t_to_end, vec3<f32>(1.0 / 3.0)) * tail.a, 0.0);
    trans = t_to_end;
  }
  var out: AtmRadiance;
  if (d_top > 0.0) {
    out.radiance = atm_scattered_radiance(ATM_SOLAR_IRRADIANCE, inscat_r, vec3<f32>(inscat_m), nu);
    out.trans = trans;
  } else {
    out.radiance = vec3<f32>(0.0);
    out.trans = vec3<f32>(1.0);
  }
  return out;
}

fn atm_march_steps(camera: vec3<f32>, ray: vec3<f32>, sun: vec3<f32>, d_end: f32, steps: u32) -> AtmRadiance {
  let nu = dot(ray, sun);
  let dt = d_end / f32(steps);
  var od_r = 0.0;
  var od_m = 0.0;
  var inscat_r = vec3<f32>(0.0);
  var inscat_m = vec3<f32>(0.0);
  var t_view = vec3<f32>(1.0);
  for (var i = 0u; i < steps; i++) {
    let p = camera + ray * (dt * (f32(i) + 0.5));
    let altitude = length(p) - ATM_BOTTOM;
    let d_r = exp(altitude / -ATM_RAYLEIGH_H) * dt;
    let d_m = exp(altitude / -ATM_MIE_H) * dt;
    od_r += d_r;
    od_m += d_m;
    t_view = exp(-(ATM_RAYLEIGH * od_r + od_m * ATM_MIE_EXT));
    let t = t_view * atm_transmittance_to_sun(p, sun);
    inscat_r += t * d_r;
    inscat_m += t * d_m;
  }
  var out: AtmRadiance;
  out.radiance = atm_scattered_radiance(ATM_SOLAR_IRRADIANCE, inscat_r, inscat_m, nu);
  out.trans = t_view;
  return out;
}
