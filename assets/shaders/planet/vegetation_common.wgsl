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

struct VegLight {
    sun_direction: vec3<f32>,
    day_shade: f32,
    sun_irradiance: vec3<f32>,
    exposure: f32,
    ambient_mul: vec3<f32>,
    look_saturation: f32,
    ambient_add: vec3<f32>,
    look_contrast: f32,
    night_fill: vec3<f32>,
    grade_mode: u32,
}

struct ChunkRecord {
    origin: vec3<f32>,
    generation: u32,
    rotation: vec4<f32>,
}

struct DrawSlot {
    list_offset: u32,
    reserved0: u32,
    reserved1: u32,
    reserved2: u32,
}

@group(0) @binding(0) var<uniform> view: VegView;
@group(0) @binding(1) var<uniform> light: VegLight;
@group(1) @binding(1) var<storage, read> chunks: array<ChunkRecord>;
@group(1) @binding(2) var<storage, read> visible: array<u32>;
@group(3) @binding(0) var<uniform> draw_slot: DrawSlot;

const CHUNK_SLOT_MASK: u32 = 65535u;
const LUMA: vec3<f32> = vec3<f32>(0.2126, 0.7152, 0.0722);
const INV_PI: f32 = 0.3183098861837907;

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

fn visible_index(instance_index: u32) -> u32 {
    return visible[draw_slot.list_offset + instance_index];
}

fn chunk_of(packed: u32) -> ChunkRecord {
    return chunks[packed & CHUNK_SLOT_MASK];
}

fn reference_grade(lit: vec3<f32>) -> vec3<f32> {
    if (light.grade_mode == 0u) {
        return lit;
    }
    let exposed = (lit * light.day_shade + light.night_fill) * light.exposure;
    let aces = clamp(
        exposed * (exposed * 2.51 + 0.03) / (exposed * (exposed * 2.43 + 0.59) + 0.14),
        vec3<f32>(0.0),
        vec3<f32>(1.0),
    );
    let saturated = mix(vec3<f32>(dot(aces, LUMA)), aces, light.look_saturation);
    let contrasted = clamp((saturated - 0.5) * light.look_contrast + 0.5, vec3<f32>(0.0), vec3<f32>(1.0));
    return pow(contrasted, vec3<f32>(1.0 / 2.2));
}

fn light_surface(albedo: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    let direct = albedo * light.sun_irradiance * INV_PI * max(dot(normal, light.sun_direction), 0.0);
    let ambient = albedo * light.ambient_mul + light.ambient_add;
    return reference_grade(direct + ambient);
}
