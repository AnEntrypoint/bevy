struct ImpostorParams {
    sprites_per_side: f32,
    alpha_clamp: f32,
    atlas_grid_side: f32,
    near_cutoff_m: f32,
    fade_band_m: f32,
    reserved0: f32,
    reserved1: f32,
    reserved2: f32,
}

struct ImpostorVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) corner: vec3<f32>,
}

struct ImpostorVarying {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) sprites_a: vec4<f32>,
    @location(1) @interpolate(flat) sprite_c_tile_distance: vec4<f32>,
    @location(2) @interpolate(flat) weights: vec4<f32>,
    @location(3) uv_a: vec4<f32>,
    @location(4) uv_c: vec2<f32>,
    @location(5) @interpolate(flat) tint: vec3<f32>,
}

@group(1) @binding(3) var<storage, read> impostor_dims: array<vec4<f32>>;

@group(2) @binding(0) var atlas_albedo: texture_2d<f32>;
@group(2) @binding(1) var atlas_normal_depth: texture_2d<f32>;
@group(2) @binding(2) var atlas_linear_sampler: sampler;
@group(2) @binding(3) var<uniform> impostor: ImpostorParams;
@group(2) @binding(4) var atlas_nearest_sampler: sampler;

fn encode_direction(direction: vec3<f32>) -> vec2<f32> {
    let sum = abs(direction.x) + abs(direction.y) + abs(direction.z);
    let o = direction / sum;
    var ox = o.x;
    var oz = o.z;
    if (o.y < 0.0) {
        ox = sign_or_one(o.x) * (1.0 - abs(o.z));
        oz = sign_or_one(o.z) * (1.0 - abs(o.x));
    }
    return vec2<f32>(ox * 0.5 + 0.5, oz * 0.5 + 0.5);
}

fn project_to_plane_uv(normal: vec3<f32>, basis: PlaneBasis, camera: vec3<f32>, view_direction: vec3<f32>) -> vec2<f32> {
    let t = -dot(camera, normal) / dot(view_direction, normal);
    let hit = camera + view_direction * t;
    return vec2<f32>(dot(basis.tangent, hit), dot(basis.bitangent, hit)) + 0.5;
}

fn sprite_weights(fraction: vec2<f32>) -> vec4<f32> {
    return vec4<f32>(
        min(1.0 - fraction.x, 1.0 - fraction.y),
        abs(fraction.x - fraction.y),
        min(fraction.x, fraction.y),
        ceil(fraction.x - fraction.y),
    );
}

fn sprite_uv(raw_uv: vec2<f32>, sprite: vec2<f32>, sprite_size: f32) -> vec2<f32> {
    let clamped = clamp(raw_uv, vec2<f32>(0.0), vec2<f32>(1.0));
    return clamp(sprite_size * (sprite + clamped), vec2<f32>(0.0), vec2<f32>(1.0));
}

fn atlas_uv(local_uv: vec2<f32>, tile: f32) -> vec2<f32> {
    let tile_base = vec2<f32>(tile % impostor.atlas_grid_side, floor(tile / impostor.atlas_grid_side));
    let uv = (tile_base + local_uv) / impostor.atlas_grid_side;
    return vec2<f32>(uv.x, 1.0 - uv.y);
}

@vertex
fn vs_impostor(vertex: ImpostorVertex) -> ImpostorVarying {
    let tree = trees[visible_index(vertex.instance_index)];
    let chunk = chunk_of(tree.chunk);
    let rotation = quat_mul(chunk.rotation, tree.tilt);
    let species = tree.species_and_shape & 255u;
    let dims = impostor_dims[species];
    let base = chunk.origin + quat_rotate(chunk.rotation, tree.local_position);
    let centre = base + quat_rotate(rotation, wind_displacement(dims.xyz, tree.wind_phase) * tree.scale);
    let size = dims.w * 2.0 * tree.scale;
    let inverse_rotation = vec4<f32>(-rotation.xyz, rotation.w);
    let camera_local = quat_rotate(inverse_rotation, view.eye - centre) / size;
    let camera_direction = normalize(camera_local);

    let sprites_minus_one = impostor.sprites_per_side - 1.0;
    let corner = vertex.corner.xy;
    let camera_basis = plane_basis(camera_direction);
    let projected = camera_basis.tangent * corner.x + camera_basis.bitangent * corner.y;
    let view_direction = normalize(projected - camera_local);

    let grid = encode_direction(camera_direction) * sprites_minus_one;
    let sprite_a = min(floor(grid), vec2<f32>(sprites_minus_one));
    let weights = sprite_weights(fract(grid));
    let sprite_b = min(sprite_a + mix(vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), weights.w), vec2<f32>(sprites_minus_one));
    let sprite_c = min(sprite_a + vec2<f32>(1.0, 1.0), vec2<f32>(sprites_minus_one));

    let normal_a = decode_direction(sprite_a, sprites_minus_one);
    let normal_b = decode_direction(sprite_b, sprites_minus_one);
    let normal_c = decode_direction(sprite_c, sprites_minus_one);
    let uv_a = project_to_plane_uv(normal_a, plane_basis(normal_a), camera_local, view_direction);
    let uv_b = project_to_plane_uv(normal_b, plane_basis(normal_b), camera_local, view_direction);
    let uv_c = project_to_plane_uv(normal_c, plane_basis(normal_c), camera_local, view_direction);

    let world_position = centre + quat_rotate(rotation, projected * size);

    var out: ImpostorVarying;
    out.clip_position = view.clip_from_world * vec4<f32>(world_position, 1.0);
    out.sprites_a = vec4<f32>(sprite_a, sprite_b);
    out.sprite_c_tile_distance = vec4<f32>(sprite_c, f32(species), distance(view.eye, centre));
    out.weights = weights;
    out.uv_a = vec4<f32>(uv_a, uv_b);
    out.uv_c = uv_c;
    out.tint = unpack_tint(tree.tint_rgb8);
    return out;
}

fn decode_stored_normal(texel: vec4<f32>) -> vec3<f32> {
    return texel.rgb * 2.0 - 1.0;
}

@fragment
fn fs_impostor(in: ImpostorVarying) -> @location(0) vec4<f32> {
    let sprite_size = 1.0 / impostor.sprites_per_side;
    let tile = in.sprite_c_tile_distance.z;
    let uv_1 = atlas_uv(sprite_uv(in.uv_a.xy, in.sprites_a.xy, sprite_size), tile);
    let uv_2 = atlas_uv(sprite_uv(in.uv_a.zw, in.sprites_a.zw, sprite_size), tile);
    let uv_3 = atlas_uv(sprite_uv(in.uv_c, in.sprite_c_tile_distance.xy, sprite_size), tile);
    let color_1 = textureSample(atlas_albedo, atlas_linear_sampler, uv_1);
    let color_2 = textureSample(atlas_albedo, atlas_linear_sampler, uv_2);
    let color_3 = textureSample(atlas_albedo, atlas_linear_sampler, uv_3);
    let normal_1 = textureSample(atlas_normal_depth, atlas_nearest_sampler, uv_1);
    let normal_2 = textureSample(atlas_normal_depth, atlas_nearest_sampler, uv_2);
    let normal_3 = textureSample(atlas_normal_depth, atlas_nearest_sampler, uv_3);

    let weights = in.weights;
    let clamp_alpha = impostor.alpha_clamp;
    let dominant_test = 1.0 - clamp_alpha;
    if (weights.x >= dominant_test) {
        if (color_1.a <= clamp_alpha) {
            discard;
        }
    } else if (weights.y >= dominant_test) {
        if (color_2.a <= clamp_alpha) {
            discard;
        }
    } else if (weights.z >= dominant_test) {
        if (color_3.a <= clamp_alpha) {
            discard;
        }
    }
    let blended = color_1 * weights.x + color_2 * weights.y + color_3 * weights.z;
    if (blended.a <= clamp_alpha) {
        discard;
    }

    let fade = clamp(
        (in.sprite_c_tile_distance.w - (impostor.near_cutoff_m - impostor.fade_band_m)) / impostor.fade_band_m,
        0.0,
        1.0,
    );
    let dither = fract(52.9829189 * fract(dot(in.clip_position.xy, vec2<f32>(0.06711056, 0.00583715))));
    if (dither > fade) {
        discard;
    }

    let view_normal = normalize(
        decode_stored_normal(normal_1) * weights.x
            + decode_stored_normal(normal_2) * weights.y
            + decode_stored_normal(normal_3) * weights.z,
    );
    let world_normal = normalize((view.world_from_view * vec4<f32>(view_normal, 0.0)).xyz);
    let albedo = blended.rgb / blended.a * in.tint;
    return vec4<f32>(light_surface(albedo, world_normal), 1.0);
}
