struct CaptureCell {
    centre: vec3<f32>,
    radius: f32,
    sprite: vec2<f32>,
    species: u32,
    sprites_minus_one: f32,
}

struct CaptureVertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct CaptureVarying {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) view_normal: vec3<f32>,
    @location(2) depth01: f32,
}

struct CaptureOutput {
    @location(0) albedo: vec4<f32>,
    @location(1) normal_depth: vec4<f32>,
}

@group(0) @binding(0) var<uniform> cell: CaptureCell;
@group(0) @binding(1) var mip_albedo: texture_2d<f32>;
@group(0) @binding(2) var mip_normal_depth: texture_2d<f32>;

const MINIMUM_ALPHA_CUT: f32 = 0.2;
const CAPTURE_NEAR: f32 = 0.001;

@vertex
fn vs_capture(vertex: CaptureVertex) -> CaptureVarying {
    let direction = decode_direction(cell.sprite, cell.sprites_minus_one);
    let basis = plane_basis(direction);
    let offset = vertex.position - cell.centre;
    let depth01 = (cell.radius - dot(direction, offset) - CAPTURE_NEAR) / (cell.radius * 2.0);
    var out: CaptureVarying;
    out.clip_position = vec4<f32>(dot(basis.tangent, offset) / cell.radius, dot(basis.bitangent, offset) / cell.radius, depth01, 1.0);
    out.uv = vertex.uv;
    out.view_normal = vec3<f32>(dot(basis.tangent, vertex.normal), dot(basis.bitangent, vertex.normal), dot(direction, vertex.normal));
    out.depth01 = depth01;
    return out;
}

@fragment
fn fs_capture(in: CaptureVarying, @builtin(front_facing) front_facing: bool) -> CaptureOutput {
    let surface = sample_surface(in.uv, cell.species);
    if (surface.texel.a < max(MINIMUM_ALPHA_CUT, surface.alpha_cutoff)) {
        discard;
    }
    let normal = normalize(in.view_normal) * select(-1.0, 1.0, front_facing);
    return CaptureOutput(surface.texel, vec4<f32>(normal * 0.5 + 0.5, 1.0 - in.depth01));
}

@vertex
fn vs_mip(@builtin(vertex_index) corner: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((corner << 1u) & 2u), f32(corner & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

fn mean_of_four(source: texture_2d<f32>, texel: vec2<i32>) -> vec4<f32> {
    let base = texel * 2;
    return (textureLoad(source, base, 0) + textureLoad(source, base + vec2<i32>(1, 0), 0) + textureLoad(source, base + vec2<i32>(0, 1), 0) + textureLoad(source, base + vec2<i32>(1, 1), 0)) * 0.25;
}

@fragment
fn fs_mip(@builtin(position) position: vec4<f32>) -> CaptureOutput {
    let texel = vec2<i32>(position.xy);
    return CaptureOutput(mean_of_four(mip_albedo, texel), mean_of_four(mip_normal_depth, texel));
}
