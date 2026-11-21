use super::guard::note_build;
use super::quadtree::{NodeKey, QuadtreeConfig};
use crate::height::Planet;
use bevy::math::DVec3;

pub const GRID_CELLS: usize = 9;
const SIDE: usize = GRID_CELLS + 3;
const CORNERS: usize = GRID_CELLS + 1;
const SKIRT_PATCH_FRAC: f64 = 0.06;
const SKIRT_MIN_M: f64 = 30.0;
const HASH_MULTIPLIER: u32 = 0x45d9f3b;
const FLIP_BIT: u32 = 17;

#[derive(Clone, Debug)]
pub struct PatchMesh {
    pub origin: DVec3,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub min_elevation: f32,
    pub max_elevation: f32,
}

struct Corner {
    dir: DVec3,
    height: f64,
    normal: DVec3,
}

fn skirt_drop(size_m: f64, relief: f64) -> f64 {
    let relief_or_one = if relief > 0.0 { relief } else { 1.0 };
    (size_m * SKIRT_PATCH_FRAC).max(relief_or_one * SKIRT_MIN_M)
}

fn grid_coordinate(vertex_index: usize) -> f64 {
    let corner = vertex_index.saturating_sub(1).min(GRID_CELLS);
    (corner as f64 * (1.0 / GRID_CELLS as f64)).clamp(0.0, 1.0)
}

fn is_ring(vertex_index: usize) -> bool {
    vertex_index == 0 || vertex_index == SIDE - 1
}

fn diagonal_flipped(x: u32, y: u32) -> bool {
    let mix = |h: u32| (h ^ (h >> 16)).wrapping_mul(HASH_MULTIPLIER);
    let hashed = mix(mix(x | (y << 16)));
    ((hashed ^ (hashed >> 16)) >> FLIP_BIT) & 1 == 1
}

fn patch_indices() -> Vec<u32> {
    let cells = SIDE - 1;
    let mut indices = Vec::with_capacity(cells * cells * 6);
    for y in 0..cells {
        for x in 0..cells {
            let a = (y * SIDE + x) as u32;
            let (b, c, d) = (a + 1, a + SIDE as u32, a + SIDE as u32 + 1);
            let triangles = if diagonal_flipped(x as u32, y as u32) { [a, c, d, a, d, b] } else { [a, c, b, b, c, d] };
            indices.extend(triangles);
        }
    }
    indices
}

fn height_at(planet: &Planet, radius: f64, dir: DVec3) -> f64 {
    planet.height(dir.to_array(), radius)
}

fn corner(planet: &Planet, radius: f64, key: NodeKey, u: f64, v: f64) -> Corner {
    let step = 1.0 / GRID_CELLS as f64;
    let at = |u: f64, v: f64| {
        let dir = key.dir(u, v);
        let height = height_at(planet, radius, dir);
        (dir, height, dir * (radius + height))
    };
    let (dir, height, here) = at(u, v);
    let (_, _, along_u) = at(u + step, v);
    let (_, _, along_v) = at(u, v + step);
    let facing = (along_u - here).cross(along_v - here).normalize();
    let normal = if facing.dot(dir) < 0.0 { -facing } else { facing };
    Corner { dir, height, normal }
}

fn to_f32(v: DVec3) -> [f32; 3] {
    [v.x as f32, v.y as f32, v.z as f32]
}

pub fn build_patch(planet: &Planet, config: &QuadtreeConfig, key: NodeKey) -> PatchMesh {
    note_build();
    let radius = config.radius;
    let origin = key.centre_dir() * radius;
    let drop = skirt_drop(key.size_m(radius), planet.relief(radius));
    let corners: Vec<Corner> = (0..CORNERS * CORNERS)
        .map(|n| corner(planet, radius, key, grid_coordinate(n % CORNERS + 1), grid_coordinate(n / CORNERS + 1)))
        .collect();
    let mut positions = Vec::with_capacity(SIDE * SIDE);
    let mut normals = Vec::with_capacity(SIDE * SIDE);
    for y in 0..SIDE {
        for x in 0..SIDE {
            let c = &corners[y.saturating_sub(1).min(GRID_CELLS) * CORNERS + x.saturating_sub(1).min(GRID_CELLS)];
            let skirt = if is_ring(x) || is_ring(y) { drop } else { 0.0 };
            positions.push(to_f32(c.dir * (radius + c.height - skirt) - origin));
            normals.push(to_f32(c.normal));
        }
    }
    let heights = corners.iter().map(|c| c.height);
    PatchMesh {
        origin,
        positions,
        normals,
        indices: patch_indices(),
        min_elevation: heights.clone().fold(f64::INFINITY, f64::min) as f32,
        max_elevation: heights.fold(f64::NEG_INFINITY, f64::max) as f32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::height::{HashVersion, PlanetConfig};
    use crate::testkit::{cases, close, parse};
    use serde_json::Value;
    use std::f64::consts::FRAC_PI_3;

    const FIXTURE: &str = include_str!("../../testdata/parity/spoint-planet-patch.json");
    const SEED: i32 = 0x5eed1234;
    const HPF_RES: usize = 128;
    const F32_HALF_ULP: f64 = 1.2e-7;
    const LAYOUT_F32_ERROR: f64 = 6e-8;
    const POSITION_FLOOR_M: f64 = 2e-3;
    const NORMAL_TOLERANCE: f64 = 1e-4;
    const ELEVATION_FLOOR_M: f64 = 1e-3;
    const SLOPE_BOUND: f64 = 0.5;

    fn numbers(value: &Value) -> Vec<f64> {
        value.as_array().unwrap().iter().map(|n| n.as_f64().unwrap()).collect()
    }

    fn planet_of(radius: f64) -> Planet {
        Planet::new(&PlanetConfig { seed: SEED, hpf_res: HPF_RES, hash: HashVersion::Integer, relief_scale: None, anchor_dir: [0.0, 1.0, 0.0], radius, carves: Vec::new() })
    }

    fn key_of(case: &Value) -> NodeKey {
        let field = |name: &str| case[name].as_u64().unwrap();
        NodeKey { face: field("face") as u8, level: field("level") as u8, x: field("x") as u32, y: field("y") as u32 }
    }

    fn built(case: &Value) -> PatchMesh {
        let radius = case["radius"].as_f64().unwrap();
        build_patch(&planet_of(radius), &QuadtreeConfig::reference(radius, 1080.0, FRAC_PI_3), key_of(case))
    }

    #[test]
    fn vertex_and_triangle_counts_follow_the_skirted_grid() {
        let fixture = parse(FIXTURE);
        let case = &cases(&fixture, "cases")[0];
        let mesh = built(case);
        assert_eq!(mesh.positions.len(), 144);
        assert_eq!(mesh.normals.len(), 144);
        assert_eq!(mesh.indices.len() / 3, 242);
        assert!(mesh.indices.iter().all(|&i| (i as usize) < mesh.positions.len()));
    }

    #[test]
    fn index_buffer_and_grid_layout_match_spoint() {
        let fixture = parse(FIXTURE);
        let want: Vec<u32> = numbers(&fixture["indices"]).iter().map(|&i| i as u32).collect();
        assert_eq!(patch_indices(), want);
        let layout = cases(&fixture, "layout");
        assert_eq!(layout.len(), SIDE * SIDE);
        for (n, vertex) in layout.iter().enumerate() {
            let [px, py, ring] = numbers(vertex)[..] else { panic!("layout entry {n} is not a triple") };
            let (x, y) = (n % SIDE, n / SIDE);
            close(grid_coordinate(x), px, LAYOUT_F32_ERROR, &format!("u of vertex {n}"));
            close(grid_coordinate(y), py, LAYOUT_F32_ERROR, &format!("v of vertex {n}"));
            assert_eq!(f64::from(is_ring(x) || is_ring(y)), ring, "ring flag of vertex {n}");
        }
    }

    #[test]
    fn patches_match_spoint_positions_normals_and_elevation() {
        let fixture = parse(FIXTURE);
        let all = cases(&fixture, "cases");
        assert_eq!(all.len(), 24);
        for case in all {
            let mesh = built(case);
            let name = format!("{:?} radius {}", key_of(case), case["radius"]);
            let origin = numbers(&case["origin"]);
            close(mesh.origin.distance(DVec3::from_slice(&origin)), 0.0, POSITION_FLOOR_M, &format!("{name} origin"));
            let size = case["size"].as_f64().unwrap();
            let (positions, normals) = (numbers(&case["positions"]), numbers(&case["normals"]));
            for (n, (position, normal)) in mesh.positions.iter().zip(&mesh.normals).enumerate() {
                for k in 0..3 {
                    let want = positions[3 * n + k] - origin[k];
                    let tolerance = POSITION_FLOOR_M + F32_HALF_ULP * want.abs() + LAYOUT_F32_ERROR * size;
                    close(f64::from(position[k]), want, tolerance, &format!("{name} vertex {n} position {k}"));
                    close(f64::from(normal[k]), normals[3 * n + k], NORMAL_TOLERANCE, &format!("{name} vertex {n} normal {k}"));
                }
            }
            for (got, field) in [(mesh.min_elevation, "minElevation"), (mesh.max_elevation, "maxElevation")] {
                let want = case[field].as_f64().unwrap();
                close(f64::from(got), want, ELEVATION_FLOOR_M + F32_HALF_ULP * want.abs() + SLOPE_BOUND * LAYOUT_F32_ERROR * size, &format!("{name} {field}"));
            }
        }
    }

    #[test]
    fn the_skirt_only_drops_ring_vertices() {
        let fixture = parse(FIXTURE);
        for case in cases(&fixture, "cases") {
            let mesh = built(case);
            let name = format!("{:?} radius {}", key_of(case), case["radius"]);
            let skirt = case["skirtM"].as_f64().unwrap();
            let size = case["size"].as_f64().unwrap();
            let tolerance = POSITION_FLOOR_M + 1e-6 * mesh.origin.length() + LAYOUT_F32_ERROR * size;
            for y in 0..SIDE {
                for x in 0..SIDE {
                    let (gx, gy) = (x.saturating_sub(1).min(GRID_CELLS) + 1, y.saturating_sub(1).min(GRID_CELLS) + 1);
                    let own = mesh.positions[y * SIDE + x];
                    let inner = mesh.positions[gy * SIDE + gx];
                    let dropped = DVec3::from_array(inner.map(f64::from)).distance(DVec3::from_array(own.map(f64::from)));
                    let expected = if is_ring(x) || is_ring(y) { skirt } else { 0.0 };
                    close(dropped, expected, tolerance, &format!("{name} vertex ({x},{y}) skirt drop"));
                    assert_eq!(mesh.normals[y * SIDE + x], mesh.normals[gy * SIDE + gx], "{name} ring normal ({x},{y})");
                }
            }
        }
    }

    #[test]
    fn building_counts_on_the_marked_main_thread() {
        let before = crate::planet_lod::guard::builds_on_main_thread();
        crate::planet_lod::guard::mark_main_thread();
        let fixture = parse(FIXTURE);
        built(&cases(&fixture, "cases")[0]);
        assert_eq!(crate::planet_lod::guard::builds_on_main_thread(), before + 1);
    }
}
