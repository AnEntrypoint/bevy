use super::buffers::{ChunkRecord, RockRecord, TreeRecord, GrassRecord, VegGpuConfig, VegKind, SHADOW_TIER, TREE_SLOTS_PER_GEO};
use super::cull::{CullStats, CullView, GrassCullSettings, RockCullSettings, TreeCullSettings};
use bevy::prelude::*;

pub const STATE_READY: u32 = 256;
pub const STATE_NO_MESH: u32 = 512;
pub const STATE_SHADOW: u32 = 1024;
pub const STATE_IN_RING: u32 = 2048;
const STATE_TIER_MASK: u32 = 3;
const TIER_NONE: u32 = 3;
const CHUNK_SLOT_MASK: u32 = 65535;
const CHUNK_GENERATION_SHIFT: u32 = 16;

fn quat_mul(a: Vec4, b: Vec4) -> Vec4 {
    Vec4::new(
        a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y,
        a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x,
        a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w,
        a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z,
    )
}

fn quat_rotate(q: Vec4, v: Vec3) -> Vec3 {
    let t = 2.0 * q.truncate().cross(v);
    v + q.w * t + q.truncate().cross(t)
}

pub fn sphere_in_frustum(planes: &[Vec4; 6], centre: Vec3, radius: f32) -> bool {
    planes.iter().all(|plane| plane.truncate().dot(centre) + plane.w >= -radius)
}

fn tree_coin(trunk_id: u32) -> f32 {
    let mut x = trunk_id ^ 0x9e3779b1;
    x = (x ^ (x >> 16)).wrapping_mul(0x85ebca6b);
    x ^= x >> 13;
    x as f32 / 4294967296.0
}

pub fn keep_probability(settings: &TreeCullSettings, distance: f32) -> f32 {
    if distance <= settings.falloff_near_m {
        return 1.0;
    }
    if distance >= settings.render_distance_m {
        return settings.falloff_floor;
    }
    let t = (distance - settings.falloff_near_m) / (settings.render_distance_m - settings.falloff_near_m).max(1.0);
    1.0 - (1.0 - settings.falloff_floor) * (t * t * (3.0 - 2.0 * t))
}

fn raw_mesh_tier(settings: &TreeCullSettings, distance_sq: f32) -> u32 {
    let h = settings.hysteresis;
    let enter2 = settings.d2_m * (1.0 + h);
    let enter1 = settings.d1_m * (1.0 + h);
    if distance_sq >= enter2 * enter2 {
        return 2;
    }
    if distance_sq >= enter1 * enter1 {
        return 1;
    }
    0
}

fn leave_distance(settings: &TreeCullSettings, tier: u32) -> f32 {
    let h = settings.hysteresis;
    if tier == 2 { settings.d2_m * (1.0 - h) } else { settings.d1_m * (1.0 - h) }
}

fn mesh_reentry_distance(settings: &TreeCullSettings) -> f32 {
    let h = settings.hysteresis;
    let nominal = settings.mesh_far_m / (1.0 + h);
    (nominal * (1.0 - h)).max(settings.mesh_reentry_m)
}

fn next_mesh_tier(settings: &TreeCullSettings, distance: f32, ready: bool, was_no_mesh: bool, was_tier: u32) -> u32 {
    let distance_sq = distance * distance;
    if distance >= settings.mesh_far_m {
        return TIER_NONE;
    }
    let raw = raw_mesh_tier(settings, distance_sq);
    if !ready {
        return raw;
    }
    if was_no_mesh {
        let reentry = mesh_reentry_distance(settings);
        if distance_sq >= reentry * reentry {
            return TIER_NONE;
        }
        return raw;
    }
    if was_tier > raw {
        let leave = leave_distance(settings, was_tier);
        if distance_sq >= leave * leave {
            return was_tier;
        }
    }
    raw
}

#[derive(Clone, Copy, Debug)]
pub struct TwinTree {
    pub record: TreeRecord,
    pub chunk: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct TwinRock {
    pub record: RockRecord,
    pub chunk: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct TwinBlade {
    pub record: GrassRecord,
    pub chunk: u32,
}

#[derive(Clone, Debug, Default)]
pub struct TwinOutcome {
    pub lists: Vec<Vec<u32>>,
    pub states: Vec<u32>,
    pub stats: CullStats,
}

impl TwinOutcome {
    pub fn overflow_for(&self, config: &VegGpuConfig, kind: VegKind) -> u32 {
        self.lists.iter().enumerate().map(|(slot, list)| (list.len() as u32).saturating_sub(config.slot_capacity(kind, slot as u32))).sum()
    }
}

fn chunk_frame(chunks: &[ChunkRecord], packed: u32) -> Option<(Vec3, Vec4)> {
    let chunk = chunks[(packed & CHUNK_SLOT_MASK) as usize];
    (chunk.generation == packed >> CHUNK_GENERATION_SHIFT).then_some((chunk.origin, Vec4::from(chunk.rotation.to_array())))
}

pub fn cull_trees(trees: &[TwinTree], chunks: &[ChunkRecord], bounds: &[[f32; 4]], config: &VegGpuConfig, settings: &TreeCullSettings, view: &CullView, states_in: &[u32]) -> TwinOutcome {
    let mut outcome = TwinOutcome { lists: vec![Vec::new(); config.slot_count(VegKind::Tree) as usize], states: vec![0; trees.len()], stats: CullStats::default() };
    for (index, tree) in trees.iter().enumerate() {
        let Some((origin, chunk_rotation)) = chunk_frame(chunks, tree.chunk) else { continue };
        let base = origin + quat_rotate(chunk_rotation, Vec3::from(tree.record.local_position));
        let distance = (base - view.eye).length();
        let flags = states_in[index];
        let in_ring = distance <= settings.ring_enter_m || (flags & STATE_IN_RING != 0 && distance <= settings.ring_drop_m);
        if !in_ring {
            continue;
        }
        if distance > settings.falloff_near_m && tree_coin(tree.record.trunk_id) >= keep_probability(settings, distance) {
            outcome.stats.tree_thinned += 1;
            continue;
        }
        outcome.stats.tree_resident += 1;
        let tier = next_mesh_tier(settings, distance, flags & STATE_READY != 0, flags & STATE_NO_MESH != 0, flags & STATE_TIER_MASK);
        let shadow_on = settings.shadow_distance_m * (1.0 - settings.hysteresis);
        let shadow_off = settings.shadow_distance_m * (1.0 + settings.hysteresis);
        let shadow_wanted = distance <= shadow_on || (flags & STATE_SHADOW != 0 && distance <= shadow_off);
        let mut next = STATE_READY | STATE_IN_RING;
        next |= if tier == TIER_NONE { STATE_NO_MESH } else { tier };
        if shadow_wanted {
            next |= STATE_SHADOW;
        }
        outcome.states[index] = next;
        let geo = u32::from(tree.record.shape) * config.tree_species + u32::from(tree.record.species);
        let index = index as u32;
        if shadow_wanted {
            outcome.lists[(geo * TREE_SLOTS_PER_GEO + SHADOW_TIER) as usize].push(index);
            outcome.stats.tree_shadow += 1;
        }
        let rotation = quat_mul(chunk_rotation, Vec4::from(tree.record.tilt));
        let sphere = Vec4::from(bounds[geo as usize]);
        let centre = base + quat_rotate(rotation, sphere.truncate() * tree.record.scale);
        if !sphere_in_frustum(&view.planes, centre - view.eye, sphere.w * tree.record.scale) {
            outcome.stats.tree_frustum_culled += 1;
            continue;
        }
        if tier != TIER_NONE {
            outcome.lists[(geo * TREE_SLOTS_PER_GEO + tier) as usize].push(index);
            outcome.stats.tree_mesh += 1;
        }
        if distance >= settings.impostor_start_m {
            outcome.lists[config.impostor_slot() as usize].push(index);
            outcome.stats.tree_impostor += 1;
        }
    }
    outcome
}

pub fn cull_rocks(rocks: &[TwinRock], chunks: &[ChunkRecord], bounds: &[[f32; 4]], config: &VegGpuConfig, settings: &RockCullSettings, view: &CullView, states_in: &[u32]) -> TwinOutcome {
    let mut outcome = TwinOutcome { lists: vec![Vec::new(); config.rock_kinds as usize], states: vec![0; rocks.len()], stats: CullStats::default() };
    for (index, rock) in rocks.iter().enumerate() {
        let Some((origin, chunk_rotation)) = chunk_frame(chunks, rock.chunk) else { continue };
        let base = origin + quat_rotate(chunk_rotation, Vec3::from(rock.record.local_position));
        let distance = (base - view.eye).length();
        let in_ring = distance <= settings.ring_enter_m || (states_in[index] & STATE_IN_RING != 0 && distance <= settings.ring_drop_m);
        if !in_ring {
            continue;
        }
        outcome.states[index] = STATE_IN_RING;
        let rotation = quat_mul(chunk_rotation, Vec4::from(rock.record.rotation));
        let sphere = Vec4::from(bounds[rock.record.kind as usize]);
        let radius = sphere.w * rock.record.scale * rock.record.squash.max(1.0);
        let centre = base + quat_rotate(rotation, sphere.truncate() * Vec3::new(rock.record.scale, rock.record.scale * rock.record.squash, rock.record.scale));
        if !sphere_in_frustum(&view.planes, centre - view.eye, radius) {
            continue;
        }
        outcome.lists[rock.record.kind as usize].push(index as u32);
        outcome.stats.rock_visible += 1;
    }
    outcome
}

pub fn cull_grass(blades: &[TwinBlade], chunks: &[ChunkRecord], settings: &GrassCullSettings, view: &CullView) -> TwinOutcome {
    let mut outcome = TwinOutcome { lists: vec![Vec::new(); 2], states: Vec::new(), stats: CullStats::default() };
    for (index, blade) in blades.iter().enumerate() {
        let Some((origin, chunk_rotation)) = chunk_frame(chunks, blade.chunk) else { continue };
        let base = origin + quat_rotate(chunk_rotation, Vec3::from(blade.record.local_position));
        let distance = (base - view.eye).length();
        let edge = (settings.full_radius_m / blade.record.rank + settings.fade_m).min(settings.edge_cap_m);
        if distance >= edge {
            continue;
        }
        let rotation = quat_mul(chunk_rotation, Vec4::from(blade.record.rotation));
        let width = settings.width_min + settings.width_span * blade.record.tint;
        let height = blade.record.scale * settings.height_scale;
        let centre = base + quat_rotate(rotation, Vec3::new(0.0, height * 0.5, 0.0));
        let radius = 0.5 * Vec2::new(width, height).length();
        if !sphere_in_frustum(&view.planes, centre - view.eye, radius) {
            continue;
        }
        let slot = usize::from(distance > settings.near_m);
        outcome.lists[slot].push(index as u32);
        if slot == 0 {
            outcome.stats.grass_near += 1;
        } else {
            outcome.stats.grass_mid += 1;
        }
    }
    outcome
}
