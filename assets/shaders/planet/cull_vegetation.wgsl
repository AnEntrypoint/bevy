const CHUNK_SLOT_MASK: u32 = 65535u;
const CHUNK_GENERATION_SHIFT: u32 = 16u;
const TIER_NONE: u32 = 3u;
const SHADOW_TIER: u32 = 3u;
const SLOTS_PER_GEO: u32 = 4u;

const STATE_READY: u32 = 256u;
const STATE_NO_MESH: u32 = 512u;
const STATE_SHADOW: u32 = 1024u;
const STATE_IN_RING: u32 = 2048u;
const STATE_TIER_MASK: u32 = 3u;

struct DrawArgs {
    index_count: u32,
    instance_count: atomic<u32>,
    first_index: u32,
    base_vertex: i32,
    first_instance: u32,
}

struct ChunkRecord {
    origin: vec3<f32>,
    generation: u32,
    rotation: vec4<f32>,
}

struct CullStats {
    overflow: atomic<u32>,
    tree_mesh: atomic<u32>,
    tree_impostor: atomic<u32>,
    tree_shadow: atomic<u32>,
    tree_resident: atomic<u32>,
    tree_thinned: atomic<u32>,
    tree_frustum_culled: atomic<u32>,
    rock_visible: atomic<u32>,
    grass_near: atomic<u32>,
    grass_mid: atomic<u32>,
    reserved0: atomic<u32>,
    reserved1: atomic<u32>,
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

struct TreeCullParams {
    planes: array<vec4<f32>, 6>,
    eye: vec3<f32>,
    instance_count: u32,
    species_count: u32,
    geo_count: u32,
    slot_count: u32,
    impostor_slot: u32,
    ring_enter_m: f32,
    ring_drop_m: f32,
    render_distance_m: f32,
    falloff_near_m: f32,
    falloff_floor: f32,
    hysteresis: f32,
    d1_m: f32,
    d2_m: f32,
    mesh_far_m: f32,
    mesh_reentry_m: f32,
    impostor_start_m: f32,
    shadow_distance_m: f32,
}

struct RockInstance {
    local_position: vec3<f32>,
    scale: f32,
    rotation: vec4<f32>,
    squash: f32,
    variant: f32,
    kind: u32,
    chunk: u32,
}

struct RockCullParams {
    planes: array<vec4<f32>, 6>,
    eye: vec3<f32>,
    instance_count: u32,
    kind_count: u32,
    slot_capacity: u32,
    ring_enter_m: f32,
    ring_drop_m: f32,
}

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

struct GrassCullParams {
    planes: array<vec4<f32>, 6>,
    eye: vec3<f32>,
    instance_count: u32,
    full_radius_m: f32,
    fade_m: f32,
    edge_cap_m: f32,
    near_m: f32,
    slot_capacity: u32,
    width_min: f32,
    width_span: f32,
    height_scale: f32,
}

@group(0) @binding(0) var<uniform> tree_params: TreeCullParams;
@group(0) @binding(1) var<storage, read> trees: array<TreeInstance>;
@group(0) @binding(2) var<storage, read> chunks: array<ChunkRecord>;
@group(0) @binding(3) var<storage, read> tree_geo_bounds: array<vec4<f32>>;
@group(0) @binding(4) var<storage, read_write> tree_state: array<u32>;
@group(0) @binding(5) var<storage, read> tree_slot_layout: array<vec2<u32>>;
@group(0) @binding(6) var<storage, read_write> tree_draws: array<DrawArgs>;
@group(0) @binding(7) var<storage, read_write> tree_visible: array<u32>;
@group(0) @binding(8) var<storage, read_write> stats: CullStats;

@group(0) @binding(10) var<uniform> rock_params: RockCullParams;
@group(0) @binding(11) var<storage, read> rocks: array<RockInstance>;
@group(0) @binding(12) var<storage, read_write> rock_state: array<u32>;
@group(0) @binding(13) var<storage, read> rock_kind_bounds: array<vec4<f32>>;
@group(0) @binding(14) var<storage, read_write> rock_draws: array<DrawArgs>;
@group(0) @binding(15) var<storage, read_write> rock_visible: array<u32>;

@group(0) @binding(20) var<uniform> grass_params: GrassCullParams;
@group(0) @binding(21) var<storage, read> blades: array<GrassInstance>;
@group(0) @binding(22) var<storage, read_write> grass_draws: array<DrawArgs>;
@group(0) @binding(23) var<storage, read_write> grass_visible: array<u32>;

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

fn sphere_in_frustum(planes: array<vec4<f32>, 6>, centre: vec3<f32>, radius: f32) -> bool {
    for (var p = 0u; p < 6u; p = p + 1u) {
        if (dot(planes[p].xyz, centre) + planes[p].w < -radius) {
            return false;
        }
    }
    return true;
}

fn tree_coin(trunk_id: u32) -> f32 {
    var x = trunk_id ^ 0x9e3779b1u;
    x = (x ^ (x >> 16u)) * 0x85ebca6bu;
    x = x ^ (x >> 13u);
    return f32(x) / 4294967296.0;
}

fn keep_probability(distance: f32) -> f32 {
    let near = tree_params.falloff_near_m;
    let far = tree_params.render_distance_m;
    let floor_keep = tree_params.falloff_floor;
    if (distance <= near) {
        return 1.0;
    }
    if (distance >= far) {
        return floor_keep;
    }
    let t = (distance - near) / max(1.0, far - near);
    return 1.0 - (1.0 - floor_keep) * (t * t * (3.0 - 2.0 * t));
}

fn raw_mesh_tier(distance_sq: f32) -> u32 {
    let h = tree_params.hysteresis;
    let enter2 = tree_params.d2_m * (1.0 + h);
    let enter1 = tree_params.d1_m * (1.0 + h);
    if (distance_sq >= enter2 * enter2) {
        return 2u;
    }
    if (distance_sq >= enter1 * enter1) {
        return 1u;
    }
    return 0u;
}

fn leave_distance(tier: u32) -> f32 {
    let h = tree_params.hysteresis;
    if (tier == 2u) {
        return tree_params.d2_m * (1.0 - h);
    }
    return tree_params.d1_m * (1.0 - h);
}

fn mesh_reentry_distance() -> f32 {
    let h = tree_params.hysteresis;
    let nominal = tree_params.mesh_far_m / (1.0 + h);
    return max(nominal * (1.0 - h), tree_params.mesh_reentry_m);
}

fn next_mesh_tier(distance: f32, ready: bool, was_no_mesh: bool, was_tier: u32) -> u32 {
    let distance_sq = distance * distance;
    if (distance >= tree_params.mesh_far_m) {
        return TIER_NONE;
    }
    let raw = raw_mesh_tier(distance_sq);
    if (!ready) {
        return raw;
    }
    if (was_no_mesh) {
        let reentry = mesh_reentry_distance();
        if (distance_sq >= reentry * reentry) {
            return TIER_NONE;
        }
        return raw;
    }
    if (was_tier > raw) {
        let leave = leave_distance(was_tier);
        if (distance_sq >= leave * leave) {
            return was_tier;
        }
    }
    return raw;
}

fn append_tree(slot: u32, tree_index: u32) {
    let slot_layout = tree_slot_layout[slot];
    let n = atomicAdd(&tree_draws[slot].instance_count, 1u);
    if (n < slot_layout.y) {
        tree_visible[slot_layout.x + n] = tree_index;
    } else {
        atomicAdd(&stats.overflow, 1u);
    }
}

@compute @workgroup_size(64)
fn reset_tree_draws(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < tree_params.slot_count) {
        atomicStore(&tree_draws[id.x].instance_count, 0u);
    }
}

@compute @workgroup_size(64)
fn cull_trees(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= tree_params.instance_count) {
        return;
    }
    let tree = trees[i];
    let chunk = chunks[tree.chunk & CHUNK_SLOT_MASK];
    if (chunk.generation != (tree.chunk >> CHUNK_GENERATION_SHIFT)) {
        tree_state[i] = 0u;
        return;
    }
    let base = chunk.origin + quat_rotate(chunk.rotation, tree.local_position);
    let offset = base - tree_params.eye;
    let distance = length(offset);

    let flags = tree_state[i];
    let was_in_ring = (flags & STATE_IN_RING) != 0u;
    let in_ring = distance <= tree_params.ring_enter_m || (was_in_ring && distance <= tree_params.ring_drop_m);
    if (!in_ring) {
        tree_state[i] = 0u;
        return;
    }
    if (distance > tree_params.falloff_near_m && tree_coin(tree.trunk_id) >= keep_probability(distance)) {
        tree_state[i] = 0u;
        atomicAdd(&stats.tree_thinned, 1u);
        return;
    }
    atomicAdd(&stats.tree_resident, 1u);

    let ready = (flags & STATE_READY) != 0u;
    let was_no_mesh = (flags & STATE_NO_MESH) != 0u;
    let was_tier = flags & STATE_TIER_MASK;
    let tier = next_mesh_tier(distance, ready, was_no_mesh, was_tier);

    let shadow_on = tree_params.shadow_distance_m * (1.0 - tree_params.hysteresis);
    let shadow_off = tree_params.shadow_distance_m * (1.0 + tree_params.hysteresis);
    let was_shadow = (flags & STATE_SHADOW) != 0u;
    let shadow_wanted = distance <= shadow_on || (was_shadow && distance <= shadow_off);

    var next_flags = STATE_READY | STATE_IN_RING;
    if (tier == TIER_NONE) {
        next_flags = next_flags | STATE_NO_MESH;
    } else {
        next_flags = next_flags | tier;
    }
    if (shadow_wanted) {
        next_flags = next_flags | STATE_SHADOW;
    }
    tree_state[i] = next_flags;

    let species = tree.species_and_shape & 255u;
    let shape = (tree.species_and_shape >> 8u) & 255u;
    let geo = shape * tree_params.species_count + species;

    if (shadow_wanted) {
        append_tree(geo * SLOTS_PER_GEO + SHADOW_TIER, i);
        atomicAdd(&stats.tree_shadow, 1u);
    }

    let rotation = quat_mul(chunk.rotation, tree.tilt);
    let bounds = tree_geo_bounds[geo];
    let sphere_centre = base + quat_rotate(rotation, bounds.xyz * tree.scale);
    if (!sphere_in_frustum(tree_params.planes, sphere_centre - tree_params.eye, bounds.w * tree.scale)) {
        atomicAdd(&stats.tree_frustum_culled, 1u);
        return;
    }
    if (tier != TIER_NONE) {
        append_tree(geo * SLOTS_PER_GEO + tier, i);
        atomicAdd(&stats.tree_mesh, 1u);
    }
    if (distance >= tree_params.impostor_start_m) {
        append_tree(tree_params.impostor_slot, i);
        atomicAdd(&stats.tree_impostor, 1u);
    }
}

@compute @workgroup_size(64)
fn finish_tree_draws(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < tree_params.slot_count) {
        let capacity = tree_slot_layout[id.x].y;
        let drawn = atomicLoad(&tree_draws[id.x].instance_count);
        atomicStore(&tree_draws[id.x].instance_count, min(drawn, capacity));
    }
}

@compute @workgroup_size(64)
fn reset_rock_draws(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < rock_params.kind_count) {
        atomicStore(&rock_draws[id.x].instance_count, 0u);
    }
}

@compute @workgroup_size(64)
fn cull_rocks(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= rock_params.instance_count) {
        return;
    }
    let rock = rocks[i];
    let chunk = chunks[rock.chunk & CHUNK_SLOT_MASK];
    if (chunk.generation != (rock.chunk >> CHUNK_GENERATION_SHIFT)) {
        rock_state[i] = 0u;
        return;
    }
    let base = chunk.origin + quat_rotate(chunk.rotation, rock.local_position);
    let distance = length(base - rock_params.eye);

    let was_in_ring = (rock_state[i] & STATE_IN_RING) != 0u;
    let in_ring = distance <= rock_params.ring_enter_m || (was_in_ring && distance <= rock_params.ring_drop_m);
    if (!in_ring) {
        rock_state[i] = 0u;
        return;
    }
    rock_state[i] = STATE_IN_RING;

    let rotation = quat_mul(chunk.rotation, rock.rotation);
    let bounds = rock_kind_bounds[rock.kind];
    let radius = bounds.w * rock.scale * max(1.0, rock.squash);
    let centre = base + quat_rotate(rotation, bounds.xyz * vec3<f32>(rock.scale, rock.scale * rock.squash, rock.scale));
    if (!sphere_in_frustum(rock_params.planes, centre - rock_params.eye, radius)) {
        return;
    }
    let n = atomicAdd(&rock_draws[rock.kind].instance_count, 1u);
    if (n < rock_params.slot_capacity) {
        rock_visible[rock.kind * rock_params.slot_capacity + n] = i;
        atomicAdd(&stats.rock_visible, 1u);
    } else {
        atomicAdd(&stats.overflow, 1u);
    }
}

@compute @workgroup_size(64)
fn finish_rock_draws(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < rock_params.kind_count) {
        let drawn = atomicLoad(&rock_draws[id.x].instance_count);
        atomicStore(&rock_draws[id.x].instance_count, min(drawn, rock_params.slot_capacity));
    }
}

@compute @workgroup_size(64)
fn reset_grass_draws(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < 2u) {
        atomicStore(&grass_draws[id.x].instance_count, 0u);
    }
}

@compute @workgroup_size(64)
fn cull_grass(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= grass_params.instance_count) {
        return;
    }
    let blade = blades[i];
    let chunk = chunks[blade.chunk & CHUNK_SLOT_MASK];
    if (chunk.generation != (blade.chunk >> CHUNK_GENERATION_SHIFT)) {
        return;
    }
    let base = chunk.origin + quat_rotate(chunk.rotation, blade.local_position);
    let distance = length(base - grass_params.eye);
    let edge = min(grass_params.full_radius_m / blade.rank + grass_params.fade_m, grass_params.edge_cap_m);
    if (distance >= edge) {
        return;
    }
    let rotation = quat_mul(chunk.rotation, blade.rotation);
    let width = grass_params.width_min + grass_params.width_span * blade.tint;
    let height = blade.scale * grass_params.height_scale;
    let centre = base + quat_rotate(rotation, vec3<f32>(0.0, height * 0.5, 0.0));
    let radius = 0.5 * length(vec2<f32>(width, height));
    if (!sphere_in_frustum(grass_params.planes, centre - grass_params.eye, radius)) {
        return;
    }
    var slot = 1u;
    if (distance <= grass_params.near_m) {
        slot = 0u;
    }
    let n = atomicAdd(&grass_draws[slot].instance_count, 1u);
    if (n < grass_params.slot_capacity) {
        grass_visible[slot * grass_params.slot_capacity + n] = i;
        if (slot == 0u) {
            atomicAdd(&stats.grass_near, 1u);
        } else {
            atomicAdd(&stats.grass_mid, 1u);
        }
    } else {
        atomicAdd(&stats.overflow, 1u);
    }
}

@compute @workgroup_size(64)
fn finish_grass_draws(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < 2u) {
        let drawn = atomicLoad(&grass_draws[id.x].instance_count);
        atomicStore(&grass_draws[id.x].instance_count, min(drawn, grass_params.slot_capacity));
    }
}
