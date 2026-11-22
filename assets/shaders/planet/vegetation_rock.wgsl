struct RockInstance {
    local_position: vec3<f32>,
    scale: f32,
    rotation: vec4<f32>,
    squash: f32,
    variant: f32,
    kind: u32,
    chunk: u32,
}

struct RockVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct RockVarying {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) variant_tint: f32,
}

@group(1) @binding(0) var<storage, read> rocks: array<RockInstance>;

const ROCK_LOW: vec3<f32> = vec3<f32>(0.30, 0.28, 0.25);
const ROCK_HIGH: vec3<f32> = vec3<f32>(0.66, 0.62, 0.56);
const ROCK_TINT_GAIN: f32 = 2.05;
const VARIANT_TINT_BASE: f32 = 0.78;
const VARIANT_TINT_SPAN: f32 = 0.4;

fn rock_hash(point: vec3<f32>) -> f32 {
    var p = fract(point * 0.3183099 + 0.1);
    p = p * 17.0;
    return fract(p.x * p.y * p.z * (p.x + p.y + p.z));
}

fn rock_noise(x: vec3<f32>) -> f32 {
    let i = floor(x);
    var f = fract(x);
    f = f * f * (3.0 - 2.0 * f);
    let n00 = mix(rock_hash(i), rock_hash(i + vec3<f32>(1.0, 0.0, 0.0)), f.x);
    let n01 = mix(rock_hash(i + vec3<f32>(0.0, 1.0, 0.0)), rock_hash(i + vec3<f32>(1.0, 1.0, 0.0)), f.x);
    let n10 = mix(rock_hash(i + vec3<f32>(0.0, 0.0, 1.0)), rock_hash(i + vec3<f32>(1.0, 0.0, 1.0)), f.x);
    let n11 = mix(rock_hash(i + vec3<f32>(0.0, 1.0, 1.0)), rock_hash(i + vec3<f32>(1.0, 1.0, 1.0)), f.x);
    return mix(mix(n00, n01, f.y), mix(n10, n11, f.y), f.z);
}

fn rock_albedo(local_position: vec3<f32>) -> vec3<f32> {
    let n = rock_noise(local_position * 1.7) * 0.6
        + rock_noise(local_position * 6.5) * 0.3
        + rock_noise(local_position * 23.0) * 0.1;
    return mix(ROCK_LOW, ROCK_HIGH, clamp(n, 0.0, 1.0)) * ROCK_TINT_GAIN;
}

@vertex
fn vs_rock(vertex: RockVertex) -> RockVarying {
    let rock = rocks[visible_index(vertex.instance_index)];
    let chunk = chunk_of(rock.chunk);
    let rotation = quat_mul(chunk.rotation, rock.rotation);
    let scale = vec3<f32>(rock.scale, rock.scale * rock.squash, rock.scale);
    let world_position = chunk.origin
        + quat_rotate(chunk.rotation, rock.local_position)
        + quat_rotate(rotation, vertex.position * scale);

    var out: RockVarying;
    out.clip_position = view.clip_from_world * vec4<f32>(world_position, 1.0);
    out.local_position = vertex.position;
    out.world_normal = normalize(quat_rotate(rotation, vertex.normal / scale));
    out.variant_tint = min(1.0, VARIANT_TINT_BASE + rock.variant * VARIANT_TINT_SPAN);
    return out;
}

fn rock_surface(in: RockVarying) -> vec3<f32> {
    return vec3<f32>(in.variant_tint) * rock_albedo(in.local_position);
}

@fragment
fn fs_rock(in: RockVarying) -> @location(0) vec4<f32> {
    return vec4<f32>(light_surface(rock_surface(in), normalize(in.world_normal)), 1.0);
}

@fragment
fn fs_rock_probe(in: RockVarying) -> @location(0) vec4<f32> {
    return vec4<f32>(rock_surface(in), 1.0);
}
