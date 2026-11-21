use crate::planet_lod::quadtree::NodeKey;
use bevy::math::DVec3;
use std::collections::HashSet;

pub const WATER_GRID: usize = 4;
pub const WATER_MAX_LEVEL: u8 = 11;
pub const SEA_LEVEL_M: f64 = 0.0;
pub const LAND_ABOVE_SEA_DISCARD_M: f64 = 1.0;
pub const LATTICE: usize = WATER_GRID + 1;
pub const LATTICE_POINTS: usize = LATTICE * LATTICE;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterInstance {
    pub key: NodeKey,
    pub land: [f32; LATTICE_POINTS],
}

pub fn clamp_level(key: NodeKey) -> NodeKey {
    let shift = key.level.saturating_sub(WATER_MAX_LEVEL);
    NodeKey { level: key.level - shift, x: key.x >> shift, y: key.y >> shift, ..key }
}

pub fn lattice_coordinate(point: usize) -> [f64; 2] {
    [(point % LATTICE) as f64 / WATER_GRID as f64, (point / LATTICE) as f64 / WATER_GRID as f64]
}

pub fn lattice_dir(key: NodeKey, point: usize) -> DVec3 {
    let [u, v] = lattice_coordinate(point);
    key.dir(u, v)
}

fn land_heights(key: NodeKey, land_height: &impl Fn(DVec3) -> f64) -> [f64; LATTICE_POINTS] {
    std::array::from_fn(|point| land_height(lattice_dir(key, point)))
}

pub fn water_instances(leaves: &[NodeKey], land_height: impl Fn(DVec3) -> f64) -> Vec<WaterInstance> {
    let mut seen = HashSet::new();
    leaves
        .iter()
        .map(|&leaf| clamp_level(leaf))
        .filter(|key| seen.insert(*key))
        .filter_map(|key| {
            let land = land_heights(key, &land_height);
            let dry = land.iter().all(|&height| height > LAND_ABOVE_SEA_DISCARD_M);
            (!dry).then(|| WaterInstance { key, land: land.map(|height| height as f32) })
        })
        .collect()
}
