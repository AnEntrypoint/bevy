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

@group(1) @binding(0) var<storage, read> trees: array<TreeInstance>;

const WIND_SWAY_PER_METRE: f32 = 0.06;
const WIND_RATE: f32 = 1.3;
const WIND_Z_RATE: f32 = 0.8;
const WIND_Z_GAIN: f32 = 0.6;
const TINT_RANGE: f32 = 2.0;

fn wind_displacement(local_position: vec3<f32>, wind_phase: f32) -> vec3<f32> {
    let sway = local_position.y * WIND_SWAY_PER_METRE;
    let phase = view.time_s * WIND_RATE + wind_phase;
    let sway_x = sin(phase) * sway * view.wind_strength;
    let sway_z = cos(phase * WIND_Z_RATE) * sway * WIND_Z_GAIN * view.wind_strength;
    return vec3<f32>(local_position.x + sway_x, local_position.y, local_position.z + sway_z);
}

fn unpack_tint(packed: u32) -> vec3<f32> {
    return unpack4x8unorm(packed).rgb * TINT_RANGE;
}
