struct GrassInstance {
    local_position: vec3<f32>,
    scale: f32,
    rotation: vec4<f32>,
    ground_rgb: vec3<f32>,
    tint: f32,
    rank: f32,
    wind_phase: f32,
    shadow: f32,
    chunk: u32,
}

struct GrassParams {
    wind_axis_x: vec3<f32>,
    wind_time: f32,
    wind_axis_z: vec3<f32>,
    wind_amplitude: f32,
    wind_origin: vec2<f32>,
    wind_direction: vec2<f32>,
    ring_m: f32,
    full_radius_m: f32,
    fade_m: f32,
    width_min: f32,
    width_span: f32,
    height_scale: f32,
    reserved0: f32,
    reserved1: f32,
}

struct GrassVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct GrassVarying {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world_position: vec3<f32>,
    @location(2) world_up: vec3<f32>,
    @location(3) ground: vec3<f32>,
    @location(4) tint: f32,
    @location(5) shadow: f32,
    @location(6) local_noise: f32,
}

struct ProbeVarying {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) colour: vec4<f32>,
}

@group(1) @binding(0) var<storage, read> blades: array<GrassInstance>;

@group(2) @binding(0) var clump_texture: texture_2d<f32>;
@group(2) @binding(1) var clump_sampler: sampler;
@group(2) @binding(2) var<uniform> grass: GrassParams;

const GRASS_LUMA: vec3<f32> = vec3<f32>(0.299, 0.587, 0.114);
const BASE_COLOUR: vec3<f32> = vec3<f32>(0.19, 0.25, 0.11);
const AMBIENT: vec3<f32> = vec3<f32>(0.32, 0.36, 0.4);
const GROUND_TINT_MIX: f32 = 0.85;
const TIP_GROUND_GAIN: f32 = 1.1;
const BLADE_VARIATION: f32 = 0.8;
const LOCAL_VARIATION: f32 = 0.6;
const GRASS_SATURATION: f32 = 1.0;
const ALPHA_CUTOFF: f32 = 0.6;
const GROW_MAX: f32 = 1.3;
const GROW_K: f32 = 0.005;
const FACE_START_M: f32 = 24.0;
const FACE_END_M: f32 = 44.0;
const SWAY_HEIGHT_GAIN: f32 = 0.45;
const PROBE_GRID: f32 = 8.0;
const PROBE_SCALE: f32 = 1.2;
const PROBE_HEIGHT_RANGE: f32 = 4.0;
const PROBE_WIDTH_RANGE: f32 = 6.0;
const PROBE_ALONG: f32 = 0.75;
const PROBE_SAMPLE: f32 = 0.5;
const PROBE_BACKLIT: f32 = 0.5;

const GUST_NOISE_SCALE: f32 = 0.06;
const GUST_ADVECT: f32 = 0.35;
const GUST_PHASE_GAIN: f32 = 5.5;
const WAVE1_K: f32 = 0.075;
const WAVE1_SPEED: f32 = 1.4;
const WAVE2_KX: f32 = -0.11;
const WAVE2_KZ: f32 = 0.09;
const WAVE2_SPEED: f32 = 2.3;
const WAVE2_GAIN: f32 = 0.5;
const SWAY_BASE: f32 = 0.2;
const SWAY_FLOW: f32 = 0.25;
const FLUTTER_X: f32 = 0.25;
const FLUTTER_Z: f32 = 0.25;
const FLUTTER_SPEED: f32 = 2.2;
const FLUTTER_K: f32 = 0.22;
const FLUTTER_Z_RATE: f32 = 0.7;

fn quat_inverse_rotate(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    return quat_rotate(vec4<f32>(-q.xyz, q.w), v);
}

fn safe_normalize(v: vec2<f32>, fallback: vec2<f32>) -> vec2<f32> {
    let length_v = length(v);
    if (length_v > 1e-6) {
        return v / length_v;
    }
    return fallback;
}

fn grass_hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn grass_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = grass_hash(i);
    let b = grass_hash(i + vec2<f32>(1.0, 0.0));
    let c = grass_hash(i + vec2<f32>(0.0, 1.0));
    let d = grass_hash(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn wind_hash(ix: i32, iz: i32) -> f32 {
    var h = (bitcast<u32>(ix) * 0x1f1f1f1fu) ^ (bitcast<u32>(iz) * 0x85ebca6bu);
    h = (h ^ (h >> 15u)) * 0x2545f491u;
    h = h ^ (h >> 13u);
    return f32(h) / 4294967296.0;
}

fn wind_value_noise(x: f32, z: f32) -> f32 {
    let ix = i32(floor(x));
    let iz = i32(floor(z));
    let fx = x - floor(x);
    let fz = z - floor(z);
    let ux = fx * fx * (3.0 - 2.0 * fx);
    let uz = fz * fz * (3.0 - 2.0 * fz);
    let a = wind_hash(ix, iz);
    let b = wind_hash(ix + 1, iz);
    let c = wind_hash(ix, iz + 1);
    let d = wind_hash(ix + 1, iz + 1);
    let low = a + (b - a) * ux;
    let high = c + (d - c) * ux;
    return low + (high - low) * uz;
}

fn grass_wind_gust(position: vec2<f32>, time: f32, direction: vec2<f32>) -> vec3<f32> {
    let gust = wind_value_noise(
        position.x * GUST_NOISE_SCALE + direction.x * time * GUST_ADVECT,
        position.y * GUST_NOISE_SCALE + direction.y * time * GUST_ADVECT,
    );
    let along = position.x * direction.x + position.y * direction.y;
    let wave = sin(along * WAVE1_K + gust * GUST_PHASE_GAIN + time * WAVE1_SPEED);
    let cross_wave = sin(position.x * WAVE2_KX + position.y * WAVE2_KZ + time * WAVE2_SPEED);
    let flutter = along * FLUTTER_K + time * FLUTTER_SPEED;
    return vec3<f32>(wave + cross_wave * WAVE2_GAIN, sin(flutter), cos(flutter * FLUTTER_Z_RATE));
}

fn gust_displacement(position: vec2<f32>) -> vec2<f32> {
    let direction = safe_normalize(grass.wind_direction, vec2<f32>(1.0, 0.0));
    let gust = grass_wind_gust(position, grass.wind_time, direction);
    let sway = (SWAY_BASE + SWAY_FLOW * gust.x) * grass.wind_amplitude;
    return vec2<f32>(
        direction.x * sway + gust.y * FLUTTER_X * grass.wind_amplitude,
        direction.y * sway + gust.z * FLUTTER_Z * grass.wind_amplitude,
    );
}

fn displace_local(position: vec3<f32>, blade_wind: vec2<f32>) -> vec3<f32> {
    let height = clamp(position.y, 0.0, 1.0);
    let gust = gust_displacement(blade_wind) * height * height * SWAY_HEIGHT_GAIN;
    return position + vec3<f32>(gust.x, 0.0, gust.y);
}

fn blade_wind_position(to_base: vec3<f32>) -> vec2<f32> {
    return grass.wind_origin + vec2<f32>(dot(to_base, grass.wind_axis_x), dot(to_base, grass.wind_axis_z));
}

fn horizontal_distance(to_base: vec3<f32>, world_up: vec3<f32>) -> f32 {
    return length(to_base - world_up * dot(to_base, world_up));
}

fn blade_size(blade: GrassInstance, distance_h: f32) -> vec3<f32> {
    let edge = min(grass.full_radius_m / blade.rank + grass.fade_m, grass.ring_m);
    let fade_in = clamp((edge - distance_h) / grass.fade_m, 0.0, 1.0);
    let grow = min(GROW_MAX, 1.0 + GROW_K * max(distance_h - grass.full_radius_m, 0.0));
    let width = grass.width_min + grass.width_span * blade.tint;
    return vec3<f32>(width, blade.scale * grass.height_scale * fade_in, width) * grow;
}

fn clump_alpha_passes(blade_sample: f32) -> bool {
    return clamp(blade_sample * 1.25, 0.0, 1.0) * 0.92 >= ALPHA_CUTOFF;
}

fn grass_colour(along_v: f32, ground: vec3<f32>, tint: f32, local_noise: f32, blade_sample: f32) -> vec3<f32> {
    let base_root = mix(BASE_COLOUR, ground, GROUND_TINT_MIX);
    let blade_jitter = clamp(1.0 + BLADE_VARIATION * (tint * 2.0 - 1.0) + LOCAL_VARIATION * (local_noise * 2.0 - 1.0), 0.2, 1.25);
    var colour = mix(base_root, ground * TIP_GROUND_GAIN * blade_jitter, smoothstep(0.0, 0.95, along_v));
    colour = colour * mix(0.78, 1.1, smoothstep(0.55, 1.0, blade_sample));
    return colour * (0.62 + smoothstep(0.0, 0.3, along_v) * 0.38);
}

fn grass_shade(colour: vec3<f32>, wrap: f32, backlit: f32, shadow: f32) -> vec3<f32> {
    let lit = colour * (AMBIENT + light.sun_irradiance * wrap * shadow)
        + colour * light.sun_irradiance * backlit * 0.55 * shadow
        + colour * light.ambient_mul
        + light.ambient_add;
    return reference_grade(mix(vec3<f32>(dot(lit, GRASS_LUMA)), lit, GRASS_SATURATION));
}

@vertex
fn vs_grass(vertex: GrassVertex) -> GrassVarying {
    let blade = blades[visible_index(vertex.instance_index)];
    let chunk = chunk_of(blade.chunk);
    let rotation = quat_mul(chunk.rotation, blade.rotation);
    let base = chunk.origin + quat_rotate(chunk.rotation, blade.local_position);
    let to_base = base - view.eye;
    let world_up = quat_rotate(rotation, vec3<f32>(0.0, 1.0, 0.0));
    let distance_h = horizontal_distance(to_base, world_up);
    let blade_wind = blade_wind_position(to_base);

    var local = displace_local(vertex.position, blade_wind);
    let to_camera_local = quat_inverse_rotate(rotation, -to_base);
    let face_amount = smoothstep(FACE_START_M, FACE_END_M, distance_h);
    let to_camera_2d = safe_normalize(to_camera_local.xz, vec2<f32>(0.0, 1.0));
    let perpendicular = vec2<f32>(-to_camera_2d.y, to_camera_2d.x);
    let side = select(-1.0, 1.0, perpendicular.x >= 0.0);
    let axis_dir = safe_normalize(mix(vec2<f32>(1.0, 0.0), perpendicular * side, face_amount), vec2<f32>(1.0, 0.0));
    local = vec3<f32>(local.x * axis_dir.x - local.z * axis_dir.y, local.y, local.x * axis_dir.y + local.z * axis_dir.x);
    let world_position = base + quat_rotate(rotation, local * blade_size(blade, distance_h));

    var out: GrassVarying;
    out.clip_position = view.clip_from_world * vec4<f32>(world_position, 1.0);
    out.uv = vertex.uv;
    out.world_position = world_position;
    out.world_up = world_up;
    out.ground = blade.ground_rgb;
    out.tint = blade.tint;
    out.shadow = blade.shadow;
    out.local_noise = grass_noise(blade_wind);
    return out;
}

fn probe_cell(blade: GrassInstance, vertex: GrassVertex) -> vec4<f32> {
    let cell = u32(blade.wind_phase);
    let inside = vec2<f32>(vertex.position.x + 0.5, vertex.position.y) * 0.8 + 0.1;
    return vec4<f32>((vec2<f32>(f32(cell % 8u), f32(cell / 8u)) + inside) / PROBE_GRID * 2.0 - 1.0, 0.5, 1.0);
}

@vertex
fn vs_grass_probe(vertex: GrassVertex) -> ProbeVarying {
    let blade = blades[visible_index(vertex.instance_index)];
    let chunk = chunk_of(blade.chunk);
    let to_base = chunk.origin + quat_rotate(chunk.rotation, blade.local_position) - view.eye;
    let world_up = quat_rotate(quat_mul(chunk.rotation, blade.rotation), vec3<f32>(0.0, 1.0, 0.0));
    let top = displace_local(vec3<f32>(0.0, 1.0, 0.0), blade_wind_position(to_base));
    let size = blade_size(blade, horizontal_distance(to_base, world_up));
    var out: ProbeVarying;
    out.clip_position = probe_cell(blade, vertex);
    out.colour = vec4<f32>(0.5 + top.x * PROBE_SCALE, 0.5 + top.z * PROBE_SCALE, size.y / PROBE_HEIGHT_RANGE, size.x / PROBE_WIDTH_RANGE);
    return out;
}

@vertex
fn vs_grass_colour_probe(vertex: GrassVertex) -> ProbeVarying {
    let blade = blades[visible_index(vertex.instance_index)];
    var out: ProbeVarying;
    out.clip_position = probe_cell(blade, vertex);
    let colour = grass_shade(grass_colour(PROBE_ALONG, blade.ground_rgb, blade.tint, 0.5, 1.0), 1.0, PROBE_BACKLIT, blade.shadow);
    out.colour = vec4<f32>(colour, select(0.0, 1.0, clump_alpha_passes(PROBE_SAMPLE)));
    return out;
}

@fragment
fn fs_grass_probe(in: ProbeVarying) -> @location(0) vec4<f32> {
    return in.colour;
}

@fragment
fn fs_grass(in: GrassVarying) -> @location(0) vec4<f32> {
    let blade_sample = textureSample(clump_texture, clump_sampler, vec2<f32>(in.uv.x, 1.0 - in.uv.y)).r;
    if (!clump_alpha_passes(blade_sample)) {
        discard;
    }
    let colour = grass_colour(in.uv.y, in.ground, in.tint, in.local_noise, blade_sample);
    let sun = light.sun_direction;
    let wrap = max(dot(normalize(in.world_up), sun), 0.0);
    let backlit_cos = clamp(dot(normalize(view.eye - in.world_position), -sun), 0.0, 1.0);
    return vec4<f32>(grass_shade(colour, wrap, backlit_cos * backlit_cos * backlit_cos * in.uv.y, in.shadow), 1.0);
}
