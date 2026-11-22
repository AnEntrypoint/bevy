struct VegView {
    clip_from_world: mat4x4<f32>,
    world_from_view: mat4x4<f32>,
    eye: vec3<f32>,
    time_s: f32,
    wind_strength: f32,
    reserved0: f32,
    reserved1: f32,
    reserved2: f32,
}

struct ChunkRecord {
    origin: vec3<f32>,
    generation: u32,
    rotation: vec4<f32>,
}

struct TreeInstance {
    local_position: vec3<f32>,
    scale: f32,
    tilt: vec4<f32>,
    yaw: f32,
    wind_phase: f32,
    tint_rgb8: u32,
    trunk_id: u32,
    species_and_shape: u32,
    chunk: u32,
    reserved0: u32,
    reserved1: u32,
}

struct DrawSlot {
    list_offset: u32,
    reserved0: u32,
    reserved1: u32,
    reserved2: u32,
}

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) colour: vec3<f32>,
}

@group(0) @binding(0) var<uniform> view: VegView;
@group(1) @binding(0) var<storage, read> trees: array<TreeInstance>;
@group(1) @binding(1) var<storage, read> chunks: array<ChunkRecord>;
@group(1) @binding(2) var<storage, read> visible: array<u32>;
@group(3) @binding(0) var<uniform> draw_slot: DrawSlot;

fn quat_mul(a: vec4<f32>, b: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(
        a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y,
        a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x,
        a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w,
        a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z,
    );
}

fn quat_rotate(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    let t = 2.0 * cross(q.xyz, v);
    return v + q.w * t + cross(q.xyz, t);
}

@vertex
fn vs_test(input: VertexInput) -> VertexOutput {
    let tree = trees[visible[draw_slot.list_offset + input.instance_index]];
    let chunk = chunks[tree.chunk & 65535u];
    let rotation = quat_mul(chunk.rotation, tree.tilt);
    let world = chunk.origin + quat_rotate(chunk.rotation, tree.local_position) + quat_rotate(rotation, input.position * tree.scale);
    var out: VertexOutput;
    out.clip = view.clip_from_world * vec4<f32>(world, 1.0);
    out.normal = quat_rotate(rotation, input.normal);
    out.colour = vec3<f32>(0.15 + 0.2 * f32(tree.species_and_shape & 255u), 0.7, 0.2);
    return out;
}

@fragment
fn fs_test(input: VertexOutput) -> @location(0) vec4<f32> {
    let shade = 0.55 + 0.45 * dot(normalize(input.normal), normalize(vec3<f32>(0.3, 0.9, 0.3)));
    return vec4<f32>(input.colour * shade, 1.0);
}
