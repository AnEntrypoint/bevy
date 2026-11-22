struct SpeciesMaterial {
    bark_tint_layer: vec4<f32>,
    leaf_tint_layer: vec4<f32>,
    bark_scale_leaf_alpha: vec4<f32>,
}

struct MaterialTable {
    species: array<SpeciesMaterial, 16>,
    look: vec4<f32>,
}

struct Surface {
    texel: vec4<f32>,
    is_bark: bool,
    alpha_cutoff: f32,
}

@group(2) @binding(0) var bark_textures: texture_2d_array<f32>;
@group(2) @binding(1) var leaf_textures: texture_2d_array<f32>;
@group(2) @binding(2) var repeat_sampler: sampler;
@group(2) @binding(3) var clamp_sampler: sampler;
@group(2) @binding(4) var<uniform> materials: MaterialTable;

const BARK_UV_MARK: f32 = -1.0;
const BARK_UV_THRESHOLD: f32 = -0.5;

fn sample_surface(uv: vec2<f32>, species: u32) -> Surface {
    let material = materials.species[species];
    let bark_uv = vec2<f32>(BARK_UV_MARK - uv.x, uv.y) * material.bark_scale_leaf_alpha.xy;
    let bark = textureSample(bark_textures, repeat_sampler, bark_uv, u32(material.bark_tint_layer.w)) * vec4<f32>(material.bark_tint_layer.xyz, 1.0);
    let leaf = textureSample(leaf_textures, clamp_sampler, uv, u32(material.leaf_tint_layer.w)) * vec4<f32>(material.leaf_tint_layer.xyz, 1.0);
    let is_bark = uv.x < BARK_UV_THRESHOLD;
    return Surface(select(leaf, bark, is_bark), is_bark, select(material.bark_scale_leaf_alpha.z * materials.look.x, 0.0, is_bark));
}
