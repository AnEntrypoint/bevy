use super::leaves::{LATTICE, LATTICE_POINTS, SEA_LEVEL_M, WATER_GRID, WaterInstance, lattice_dir};
use bevy::math::DVec3;

const HASH_MULTIPLIER: u32 = 0x45d9f3b;
const FLIP_BIT: u32 = 17;

#[derive(Clone, Debug)]
pub struct WaterMesh {
    pub origin: DVec3,
    pub positions: Vec<[f32; 3]>,
    pub dirs: Vec<[f32; 3]>,
    pub land: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

fn diagonal_flipped(x: u32, y: u32) -> bool {
    let mix = |h: u32| (h ^ (h >> 16)).wrapping_mul(HASH_MULTIPLIER);
    let hashed = mix(mix(x | (y << 16)));
    ((hashed ^ (hashed >> 16)) >> FLIP_BIT) & 1 == 1
}

pub fn instance_indices() -> Vec<u32> {
    let mut indices = Vec::with_capacity(WATER_GRID * WATER_GRID * 6);
    for y in 1..=WATER_GRID as u32 {
        for x in 1..=WATER_GRID as u32 {
            let a = (y - 1) * LATTICE as u32 + (x - 1);
            let (b, c, d) = (a + 1, a + LATTICE as u32, a + LATTICE as u32 + 1);
            indices.extend(if diagonal_flipped(x, y) { [a, c, d, a, d, b] } else { [a, c, b, b, c, d] });
        }
    }
    indices
}

fn to_f32(v: DVec3) -> [f32; 3] {
    [v.x as f32, v.y as f32, v.z as f32]
}

pub fn build_water_mesh(instances: &[WaterInstance], radius: f64, origin: DVec3) -> WaterMesh {
    let local = instance_indices();
    let mut mesh = WaterMesh { origin, positions: Vec::new(), dirs: Vec::new(), land: Vec::new(), indices: Vec::new() };
    for instance in instances {
        let base = mesh.positions.len() as u32;
        for point in 0..LATTICE_POINTS {
            let dir = lattice_dir(instance.key, point);
            mesh.positions.push(to_f32(dir * (radius + SEA_LEVEL_M) - origin));
            mesh.dirs.push(to_f32(dir));
            mesh.land.push([instance.land[point], 0.0]);
        }
        mesh.indices.extend(local.iter().map(|index| base + index));
    }
    mesh
}
