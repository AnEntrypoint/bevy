struct TonemapInput {
  @builtin(position) position: vec4<f32>,
}

struct ToneMap {
  exposure: f32,
  aces: f32,
  padding: vec2<f32>,
}

@group(0) @binding(0) var hdr_source: texture_2d<f32>;
@group(0) @binding(1) var hdr_sampler: sampler;
@group(0) @binding(2) var<uniform> tonemap: ToneMap;

const ACES_EXPOSURE_DIVISOR: f32 = 0.6;
const ACES_INPUT: mat3x3<f32> = mat3x3<f32>(
  vec3<f32>(0.59719, 0.07600, 0.02840),
  vec3<f32>(0.35458, 0.90834, 0.13383),
  vec3<f32>(0.04823, 0.01566, 0.83777),
);
const ACES_OUTPUT: mat3x3<f32> = mat3x3<f32>(
  vec3<f32>(1.60475, -0.10208, -0.00327),
  vec3<f32>(-0.53108, 1.10813, -0.07276),
  vec3<f32>(-0.07367, -0.00605, 1.07602),
);

fn rrt_odt_fit(v: vec3<f32>) -> vec3<f32> {
  let a = v * (v + 0.0245786) - 0.000090537;
  let b = v * ((v + 0.4329510) * 0.983729) + 0.238081;
  return a / b;
}

fn aces_filmic(color: vec3<f32>, exposure: f32) -> vec3<f32> {
  let fitted = rrt_odt_fit(ACES_INPUT * (color * exposure / ACES_EXPOSURE_DIVISOR));
  return clamp(ACES_OUTPUT * fitted, vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fragment(in: TonemapInput) -> @location(0) vec4<f32> {
  let source = textureLoad(hdr_source, vec2<i32>(in.position.xy), 0);
  var mapped = clamp(source.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
  if (tonemap.aces > 0.5) {
    mapped = aces_filmic(source.rgb, tonemap.exposure);
  }
  return vec4<f32>(mapped, source.a);
}
