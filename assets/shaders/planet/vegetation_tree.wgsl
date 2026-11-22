struct TreeVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct TreeVarying {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) tint: vec3<f32>,
    @location(3) @interpolate(flat) species: u32,
}

struct ProbeVarying {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) probe: vec3<f32>,
}

struct PlacedVertex {
    tree: TreeInstance,
    rotation: vec4<f32>,
    clip_position: vec4<f32>,
}

fn yaw_quat(yaw: f32) -> vec4<f32> {
    return vec4<f32>(0.0, sin(yaw * 0.5), 0.0, cos(yaw * 0.5));
}

fn place(vertex: TreeVertex) -> PlacedVertex {
    let tree = trees[visible_index(vertex.instance_index)];
    let chunk = chunk_of(tree.chunk);
    let rotation = quat_mul(quat_mul(chunk.rotation, tree.tilt), yaw_quat(tree.yaw));
    let world = chunk.origin
        + quat_rotate(chunk.rotation, tree.local_position)
        + quat_rotate(rotation, wind_displacement(vertex.position, tree.wind_phase) * tree.scale);
    return PlacedVertex(tree, rotation, view.clip_from_world * vec4<f32>(world, 1.0));
}

@vertex
fn vs_tree(vertex: TreeVertex) -> TreeVarying {
    let placed = place(vertex);
    var out: TreeVarying;
    out.clip_position = placed.clip_position;
    out.world_normal = quat_rotate(placed.rotation, vertex.normal);
    out.uv = vertex.uv;
    out.tint = select(vec3<f32>(1.0), unpack_tint(placed.tree.tint_rgb8), materials.look.y > 0.5);
    out.species = placed.tree.species_and_shape & 255u;
    return out;
}

@fragment
fn fs_tree(in: TreeVarying, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    let surface = sample_surface(in.uv, in.species);
    if (surface.texel.a < surface.alpha_cutoff) {
        discard;
    }
    let normal = normalize(in.world_normal) * select(-1.0, 1.0, front_facing);
    let tint = select(in.tint, vec3<f32>(dot(in.tint, LUMA)), surface.is_bark);
    return vec4<f32>(light_surface(surface.texel.rgb * tint, normal), 1.0);
}

@vertex
fn vs_tree_probe(vertex: TreeVertex) -> ProbeVarying {
    let placed = place(vertex);
    let displaced = wind_displacement(vertex.position, placed.tree.wind_phase);
    var out: ProbeVarying;
    out.clip_position = placed.clip_position;
    out.probe = vec3<f32>(displaced.x - vertex.position.x, displaced.z - vertex.position.z, vertex.position.y);
    return out;
}

@fragment
fn fs_tree_probe(in: ProbeVarying) -> @location(0) vec4<f32> {
    return vec4<f32>(in.probe, 1.0);
}
