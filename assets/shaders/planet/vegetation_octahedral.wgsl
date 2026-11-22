struct PlaneBasis {
    tangent: vec3<f32>,
    bitangent: vec3<f32>,
}

fn sign_or_one(x: f32) -> f32 {
    if (x >= 0.0) {
        return 1.0;
    }
    return -1.0;
}

fn decode_direction(grid_index: vec2<f32>, sprites_minus_one: f32) -> vec3<f32> {
    let grid_uv = grid_index / sprites_minus_one;
    let px = (grid_uv.x - 0.5) * 2.0;
    let pz = (grid_uv.y - 0.5) * 2.0;
    let py = 1.0 - abs(px) - abs(pz);
    var x = px;
    var z = pz;
    if (py < 0.0) {
        x = sign_or_one(px) * (1.0 - abs(pz));
        z = sign_or_one(pz) * (1.0 - abs(px));
    }
    return normalize(vec3<f32>(x, py, z));
}

fn plane_basis(normal: vec3<f32>) -> PlaneBasis {
    var up = vec3<f32>(0.0, 1.0, 0.0);
    if (normal.y > 0.999) {
        up = vec3<f32>(-1.0, 0.0, 0.0);
    }
    if (normal.y < -0.999) {
        up = vec3<f32>(1.0, 0.0, 0.0);
    }
    let tangent = normalize(cross(up, normal));
    return PlaneBasis(tangent, cross(normal, tangent));
}
