const DISPLAY_CLAMP: f32 = 0.995;
const DISPLAY_GAMMA_INV: f32 = 0.45454545454545453;
const LUMA: vec3<f32> = vec3<f32>(0.2126, 0.7152, 0.0722);
const ACES_FILMIC_A: f32 = 2.51;
const ACES_FILMIC_B: f32 = 0.03;
const ACES_FILMIC_C: f32 = 2.43;
const ACES_FILMIC_D: f32 = 0.59;
const ACES_FILMIC_E: f32 = 0.14;
const ACES_EXPOSURE_DIVISOR: f32 = 0.6;
const ACES_INPUT: mat3x3<f32> = mat3x3<f32>(0.59719, 0.35458, 0.04823, 0.07600, 0.90834, 0.01566, 0.02840, 0.13383, 0.83777);
const ACES_OUTPUT: mat3x3<f32> = mat3x3<f32>(1.60475, -0.53108, -0.07367, -0.10208, 1.10813, -0.00605, -0.00327, -0.07276, 1.07602);
const ACES_INPUT_INVERSE: mat3x3<f32> = mat3x3<f32>(1.76474097, -0.675777678, -0.0889632938, -0.147027852, 1.16025151, -0.0132236597, -0.0363368301, -0.162436437, 1.19877327);
const ACES_OUTPUT_INVERSE: mat3x3<f32> = mat3x3<f32>(0.643038249, 0.311186752, 0.0457754574, 0.0592686904, 0.931436487, 0.00929491568, 0.00596190125, 0.0639290157, 0.930118384);

fn rrt_odt_fit(v: vec3<f32>) -> vec3<f32> {
  let a = v * (v + 0.0245786) - 0.000090537;
  let b = v * ((v + 0.4329510) * 0.983729) + 0.238081;
  return a / b;
}

fn aces_filmic(color: vec3<f32>, exposure: f32) -> vec3<f32> {
  var c = color * exposure / ACES_EXPOSURE_DIVISOR;
  c = c * ACES_INPUT;
  c = rrt_odt_fit(c);
  c = c * ACES_OUTPUT;
  return clamp(c, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn aces_display_curve(c: vec3<f32>) -> vec3<f32> {
  return clamp(c * (c * ACES_FILMIC_A + ACES_FILMIC_B) / (c * (c * ACES_FILMIC_C + ACES_FILMIC_D) + ACES_FILMIC_E), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn aces_display(c: vec3<f32>) -> vec3<f32> {
  return pow(aces_display_curve(c), vec3<f32>(DISPLAY_GAMMA_INV));
}

fn srgb_oetf(c: vec3<f32>) -> vec3<f32> {
  let a = pow(c, vec3<f32>(0.41666)) * 1.055 - 0.055;
  let b = c * 12.92;
  return select(a, b, c <= vec3<f32>(0.0031308));
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
  return select(pow((c + 0.055) / 1.055, vec3<f32>(2.4)), c / 12.92, c <= vec3<f32>(0.04045));
}

fn invert_aces_filmic(t: vec3<f32>, exposure: f32) -> vec3<f32> {
  let z = clamp(t * ACES_OUTPUT_INVERSE, vec3<f32>(0.0), vec3<f32>(DISPLAY_CLAMP));
  let a = z * 0.983729 - 1.0;
  let b = z * (0.983729 * 0.4329510) - 0.0245786;
  let c = z * 0.238081 + 0.000090537;
  let y = (-b - sqrt(max(b * b - a * c * 4.0, vec3<f32>(0.0)))) / (a * 2.0);
  return (y * ACES_INPUT_INVERSE) * (ACES_EXPOSURE_DIVISOR / exposure);
}

fn scene_linear_to_display_referred(linear: vec3<f32>, exposure: f32, aces: bool) -> vec3<f32> {
  var mapped = linear;
  if (aces) {
    mapped = aces_filmic(linear, exposure);
  }
  return srgb_oetf(clamp(mapped, vec3<f32>(0.0), vec3<f32>(1.0)));
}

fn display_referred_to_scene_linear(display: vec3<f32>, exposure: f32, aces: bool) -> vec3<f32> {
  let t = clamp(srgb_to_linear(display), vec3<f32>(0.0), vec3<f32>(DISPLAY_CLAMP));
  if (aces) {
    return invert_aces_filmic(t, exposure);
  }
  return t;
}
