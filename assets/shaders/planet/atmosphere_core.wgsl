const ATM_PI: f32 = 3.14159265358979;
const ATM_BOTTOM: f32 = 6360.0;
const ATM_TOP: f32 = 6500.0;
const ATM_RAYLEIGH_H: f32 = 18.0;
const ATM_MIE_H: f32 = 4.0;
const ATM_MIE_G: f32 = 0.8;
const ATM_RAYLEIGH: vec3<f32> = vec3<f32>(0.005802, 0.013558, 0.0331);
const ATM_MIE_SCAT: f32 = 0.003996;
const ATM_MIE_EXT: f32 = 0.0044400002;
const ATM_SOLAR_IRRADIANCE: vec3<f32> = vec3<f32>(1.474, 1.8504, 1.91198);
const ATM_SUN_ANGULAR_RADIUS: f32 = 0.004675;
const ATM_BOTTOM2: f32 = 40449600.0;
const ATM_TOP2: f32 = 42250000.0;
const ATM_MIE_G2: f32 = 0.64;
const ATM_MIE_2G: f32 = 1.6;
const ATM_MIE_K: f32 = (3.0 / (8.0 * ATM_PI)) * (1.0 - ATM_MIE_G2) / (2.0 + ATM_MIE_G2);
const ATM_RAYLEIGH_PHASE_K: f32 = 3.0 / (16.0 * ATM_PI);
const ATM_RHO_MAX: f32 = 1341.7898494175606;
const ATM_SCAT_LAYERS: f32 = 24.0;
const ATM_SCAT_K: f32 = 1.4;
const ATM_HORIZON_BLEND_MU: f32 = 0.006;
const ATM_TRANSMITTANCE_WIDTH: u32 = 256u;
const ATM_TRANSMITTANCE_HEIGHT: u32 = 128u;
const ATM_TRANSMITTANCE_STEPS: u32 = 500u;
const ATM_SCATTERING_WIDTH: u32 = 64u;
const ATM_SCATTERING_HEIGHT: u32 = 32u;
const ATM_SCATTERING_LAYERS: u32 = 24u;
const ATM_SCATTERING_STEPS: u32 = 64u;

fn atm_world_to_atmosphere_scale(planet_radius_m: f32) -> f32 {
  return ATM_BOTTOM / planet_radius_m;
}

fn atm_rayleigh_phase(nu: f32) -> f32 {
  return ATM_RAYLEIGH_PHASE_K * (1.0 + nu * nu);
}

fn atm_mie_phase(nu: f32) -> f32 {
  let base = max(1.0 + ATM_MIE_G2 - ATM_MIE_2G * nu, 1e-4);
  return ATM_MIE_K * (1.0 + nu * nu) / (base * sqrt(base));
}

fn atm_dist_to_top(r: f32, mu: f32) -> f32 {
  let disc = r * r * (mu * mu - 1.0) + ATM_TOP2;
  if (disc < 0.0) {
    return -1.0;
  }
  return max(-r * mu + sqrt(disc), 0.0);
}

fn atm_dist_to_ground(r: f32, mu: f32) -> f32 {
  let disc = r * r * (mu * mu - 1.0) + ATM_BOTTOM2;
  if (disc < 0.0) {
    return -1.0;
  }
  let d = -r * mu - sqrt(disc);
  if (d >= 0.0) {
    return d;
  }
  return -1.0;
}

fn atm_dist_to_ground_continuous(r: f32, mu: f32) -> f32 {
  return -r * mu - sqrt(max(r * r * (mu * mu - 1.0) + ATM_BOTTOM2, 0.0));
}

fn atm_lut_uv(r: f32, mu: f32) -> vec2<f32> {
  let rho = sqrt(max(r * r - ATM_BOTTOM2, 0.0));
  let d_min = ATM_TOP - r;
  let d_max = rho + ATM_RHO_MAX;
  var v = 0.0;
  if (d_max > d_min) {
    v = clamp((atm_dist_to_top(r, mu) - d_min) / (d_max - d_min), 0.0, 1.0);
  }
  return vec2<f32>(clamp(rho / ATM_RHO_MAX, 0.0, 1.0), v);
}

fn atm_r_from_u(u: f32) -> f32 {
  let rho = clamp(u, 0.0, 1.0) * ATM_RHO_MAX;
  return sqrt(rho * rho + ATM_BOTTOM2);
}

fn atm_mu_from_v(v: f32, r: f32) -> f32 {
  let rho = sqrt(max(r * r - ATM_BOTTOM2, 0.0));
  let d_min = ATM_TOP - r;
  let d_max = rho + ATM_RHO_MAX;
  let d = d_min + clamp(v, 0.0, 1.0) * (d_max - d_min);
  if (d <= 0.0) {
    return 1.0;
  }
  return clamp((ATM_TOP2 - r * r - d * d) / (2.0 * d * r), -1.0, 1.0);
}

fn atm_mu_s_from_layer(layer: u32) -> f32 {
  let w = (f32(layer) + 0.5) / ATM_SCAT_LAYERS;
  let t = clamp(w, 0.0, 1.0) * 2.0 - 1.0;
  return clamp(tanh(ATM_SCAT_K * t) / tanh(ATM_SCAT_K), -1.0, 1.0);
}

fn atm_layer_from_mu_s(mu_s: f32) -> f32 {
  let t = atanh(clamp(mu_s * tanh(ATM_SCAT_K), -0.999999, 0.999999)) / ATM_SCAT_K;
  let w = clamp((t + 1.0) * 0.5, 0.0, 1.0);
  return clamp(w * ATM_SCAT_LAYERS - 0.5, 0.0, ATM_SCAT_LAYERS - 1.0);
}

fn atm_densities(r: f32) -> vec2<f32> {
  let alt = r - ATM_BOTTOM;
  return vec2<f32>(exp(-alt / ATM_RAYLEIGH_H), exp(-alt / ATM_MIE_H));
}
