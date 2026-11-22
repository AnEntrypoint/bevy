use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::planet_lod::guard::note_build;
use crate::refusal::Refusal;

pub const TARGET_H: [(&str, f32); 15] = [
    ("Oak Large", 9.0),
    ("Pine Medium", 12.0),
    ("Aspen Medium", 9.5),
    ("Ash Medium", 10.0),
    ("Bush", 2.8),
    ("Ash Small", 6.5),
    ("Ash Large", 13.0),
    ("Aspen Small", 6.0),
    ("Aspen Large", 13.0),
    ("Bush 2", 3.0),
    ("Bush 3", 3.2),
    ("Oak Small", 5.5),
    ("Oak Medium", 7.0),
    ("Pine Small", 8.0),
    ("Pine Large", 16.0),
];

pub const TIER_COUNT: usize = 3;
const MAGIC: &[u8; 4] = b"VEG1";
const PREFIX_BYTES: usize = 8;
const SHADOW_TIER: u32 = 2;

#[derive(Clone, Debug, PartialEq)]
pub struct PartGeometry {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

impl PartGeometry {
    #[cfg(test)]
    pub fn triangles(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    #[cfg(test)]
    pub fn bounds(&self) -> ([f32; 3], [f32; 3]) {
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for position in &self.positions {
            for axis in 0..3 {
                min[axis] = min[axis].min(position[axis]);
                max[axis] = max[axis].max(position[axis]);
            }
        }
        (min, max)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TierGeometry {
    pub branch: PartGeometry,
    pub leaf: PartGeometry,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct BarkMaps {
    pub color: PathBuf,
    pub normal: PathBuf,
    pub ao: PathBuf,
    pub roughness: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BarkMaterial {
    pub tint: u32,
    pub texture_scale: [f32; 2],
    pub flat_shading: bool,
    pub maps: BarkMaps,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeafMaterial {
    pub tint: u32,
    pub alpha_test: f32,
    pub map: PathBuf,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpeciesGeometry {
    pub name: &'static str,
    pub target_h: f32,
    pub width: f32,
    pub height: f32,
    pub box_min: [f32; 3],
    pub box_max: [f32; 3],
    pub sphere_centre: [f32; 3],
    pub sphere_radius: f32,
    pub bark: BarkMaterial,
    pub leaf: LeafMaterial,
    pub tiers: [TierGeometry; TIER_COUNT],
    pub shadow: PartGeometry,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PartEntry {
    part: String,
    tier: u32,
    vertices: usize,
    indices: usize,
    positions: usize,
    normals: usize,
    uvs: usize,
    index_data: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Header {
    name: String,
    target_h: f32,
    width: f32,
    height: f32,
    box_min: [f32; 3],
    box_max: [f32; 3],
    sphere_centre: [f32; 3],
    sphere_radius: f32,
    bark: BarkMaterial,
    leaf: LeafMaterial,
    parts: Vec<PartEntry>,
}

fn refuse(name: &'static str, message: String) -> Refusal {
    Refusal { name, message }
}

fn file_stem(species: &str) -> String {
    species.to_lowercase().replace(' ', "_")
}

fn words(bin: &[u8], offset: usize, count: usize) -> Result<Vec<f32>, Refusal> {
    let end = offset + count * 4;
    let bytes = bin
        .get(offset..end)
        .ok_or_else(|| refuse("veg-species-truncated", format!("binary range {offset}..{end} of {}", bin.len())))?;
    Ok(bytes.chunks_exact(4).map(|word| f32::from_le_bytes(word.try_into().unwrap())).collect())
}

fn indices(bin: &[u8], offset: usize, count: usize, vertices: usize) -> Result<Vec<u32>, Refusal> {
    let end = offset + count * 4;
    let bytes = bin
        .get(offset..end)
        .ok_or_else(|| refuse("veg-species-truncated", format!("index range {offset}..{end} of {}", bin.len())))?;
    let indices: Vec<u32> = bytes.chunks_exact(4).map(|word| u32::from_le_bytes(word.try_into().unwrap())).collect();
    match indices.iter().find(|index| **index as usize >= vertices) {
        Some(index) => Err(refuse("veg-species-index-range", format!("index {index} with {vertices} vertices"))),
        None => Ok(indices),
    }
}

fn part_geometry(bin: &[u8], entry: &PartEntry) -> Result<PartGeometry, Refusal> {
    let positions = words(bin, entry.positions, entry.vertices * 3)?;
    let normals = words(bin, entry.normals, entry.vertices * 3)?;
    let uvs = words(bin, entry.uvs, entry.vertices * 2)?;
    Ok(PartGeometry {
        positions: positions.chunks_exact(3).map(|p| [p[0], p[1], p[2]]).collect(),
        normals: normals.chunks_exact(3).map(|n| [n[0], n[1], n[2]]).collect(),
        uvs: uvs.chunks_exact(2).map(|t| [t[0], t[1]]).collect(),
        indices: indices(bin, entry.index_data, entry.indices, entry.vertices)?,
    })
}

fn find_part(bin: &[u8], header: &Header, part: &str, tier: u32) -> Result<PartGeometry, Refusal> {
    let entry = header
        .parts
        .iter()
        .find(|entry| entry.part == part && entry.tier == tier)
        .ok_or_else(|| refuse("veg-species-part-missing", format!("{} part {part} tier {tier}", header.name)))?;
    part_geometry(bin, entry)
}

fn tier_geometry(bin: &[u8], header: &Header, tier: u32) -> Result<TierGeometry, Refusal> {
    Ok(TierGeometry { branch: find_part(bin, header, "branch", tier)?, leaf: find_part(bin, header, "leaf", tier)? })
}

fn decode_species(name: &'static str, target_h: f32, bytes: &[u8]) -> Result<SpeciesGeometry, Refusal> {
    if bytes.get(..4) != Some(MAGIC.as_slice()) {
        return Err(refuse("veg-species-magic", format!("{name} does not start with VEG1")));
    }
    let header_bytes = bytes
        .get(4..PREFIX_BYTES)
        .map(|length| u32::from_le_bytes(length.try_into().unwrap()) as usize)
        .and_then(|length| bytes.get(PREFIX_BYTES..PREFIX_BYTES + length))
        .ok_or_else(|| refuse("veg-species-truncated", format!("{name} header")))?;
    let header: Header =
        serde_json::from_slice(header_bytes).map_err(|error| refuse("veg-species-header", format!("{name}: {error}")))?;
    if header.name != name || header.target_h != target_h {
        return Err(refuse(
            "veg-species-mismatch",
            format!("{name} {target_h} against file {} {}", header.name, header.target_h),
        ));
    }
    let bin = &bytes[PREFIX_BYTES + header_bytes.len()..];
    Ok(SpeciesGeometry {
        name,
        target_h,
        width: header.width,
        height: header.height,
        box_min: header.box_min,
        box_max: header.box_max,
        sphere_centre: header.sphere_centre,
        sphere_radius: header.sphere_radius,
        tiers: [tier_geometry(bin, &header, 0)?, tier_geometry(bin, &header, 1)?, tier_geometry(bin, &header, 2)?],
        shadow: find_part(bin, &header, "shadow", SHADOW_TIER)?,
        bark: header.bark,
        leaf: header.leaf,
    })
}

fn require_texture(dir: &Path, relative: &Path) -> Result<(), Refusal> {
    let path = dir.join(relative);
    path.is_file().then_some(()).ok_or_else(|| refuse("veg-texture-missing", path.display().to_string()))
}

fn load_one(dir: &Path, name: &'static str, target_h: f32) -> Result<SpeciesGeometry, Refusal> {
    let path = dir.join(file_stem(name) + ".veg");
    let bytes = std::fs::read(&path).map_err(|error| refuse("veg-species-file-missing", format!("{}: {error}", path.display())))?;
    let species = decode_species(name, target_h, &bytes)?;
    let maps = &species.bark.maps;
    [&maps.color, &maps.normal, &maps.ao, &maps.roughness, &species.leaf.map]
        .into_iter()
        .try_for_each(|texture| require_texture(dir, texture))?;
    Ok(species)
}

pub fn load_species(dir: &Path) -> Result<Vec<SpeciesGeometry>, Refusal> {
    note_build();
    TARGET_H.iter().map(|(name, target_h)| load_one(dir, name, *target_h)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet_lod::guard::{builds_on_main_thread, mark_main_thread};
    use crate::testkit::{cases, close, parse};

    const FIXTURE: &str = include_str!("../../testdata/parity/spoint-planet-species.json");
    const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/planet/vegetation");
    const PART_ORDER: [(&str, usize); 7] =
        [("branch", 0), ("leaf", 0), ("branch", 1), ("leaf", 1), ("branch", 2), ("leaf", 2), ("shadow", 2)];

    fn fnv1a(bytes: impl Iterator<Item = u8>) -> u32 {
        bytes.fold(2166136261u32, |hash, byte| (hash ^ byte as u32).wrapping_mul(16777619))
    }

    fn hashes(part: &PartGeometry) -> [u32; 4] {
        let floats = |values: Vec<f32>| fnv1a(values.into_iter().flat_map(f32::to_le_bytes));
        [
            floats(part.positions.iter().flatten().copied().collect()),
            floats(part.normals.iter().flatten().copied().collect()),
            floats(part.uvs.iter().flatten().copied().collect()),
            fnv1a(part.indices.iter().copied().flat_map(u32::to_le_bytes)),
        ]
    }

    fn part_of<'a>(species: &'a SpeciesGeometry, part: &str, tier: usize) -> &'a PartGeometry {
        match part {
            "branch" => &species.tiers[tier].branch,
            "leaf" => &species.tiers[tier].leaf,
            _ => &species.shadow,
        }
    }

    fn loaded() -> Vec<SpeciesGeometry> {
        load_species(Path::new(DIR)).unwrap()
    }

    fn vec3(value: &serde_json::Value) -> [f64; 3] {
        [value[0].as_f64().unwrap(), value[1].as_f64().unwrap(), value[2].as_f64().unwrap()]
    }

    #[test]
    fn species_come_in_the_reference_order_with_their_target_heights() {
        let fixture = parse(FIXTURE);
        let rows = cases(&fixture, "species");
        let species = loaded();
        assert_eq!(species.len(), 15);
        assert_eq!(rows.len(), species.len());
        for ((row, got), (name, target_h)) in rows.iter().zip(&species).zip(TARGET_H) {
            assert_eq!(row["name"].as_str().unwrap(), got.name);
            assert_eq!(got.name, name);
            close(got.target_h as f64, row["targetH"].as_f64().unwrap(), 1e-6, name);
            close(target_h as f64, row["targetH"].as_f64().unwrap(), 1e-6, name);
        }
    }

    #[test]
    fn every_species_tier_and_part_matches_the_reference_build() {
        let fixture = parse(FIXTURE);
        let species = loaded();
        let mut checked = 0;
        for (row, got) in cases(&fixture, "species").iter().zip(&species) {
            let parts = row["parts"].as_array().unwrap();
            assert_eq!(parts.len(), PART_ORDER.len());
            for (part_row, (part, tier)) in parts.iter().zip(PART_ORDER) {
                let context = format!("{} {part} tier {tier}", got.name);
                assert_eq!(part_row["part"].as_str().unwrap(), part, "{context}");
                assert_eq!(part_row["tier"].as_u64().unwrap() as usize, tier, "{context}");
                let geometry = part_of(got, part, tier);
                assert_eq!(geometry.triangles() as u64, part_row["deliveredTris"].as_u64().unwrap(), "{context} triangles");
                assert_eq!(geometry.vertex_count() as u64, part_row["vertices"].as_u64().unwrap(), "{context} vertices");
                assert_eq!(geometry.normals.len(), geometry.vertex_count(), "{context}");
                assert_eq!(geometry.uvs.len(), geometry.vertex_count(), "{context}");
                let (min, max) = geometry.bounds();
                for axis in 0..3 {
                    close(min[axis] as f64, vec3(&part_row["min"])[axis], 1e-6, &format!("{context} min {axis}"));
                    close(max[axis] as f64, vec3(&part_row["max"])[axis], 1e-6, &format!("{context} max {axis}"));
                }
                let want = &part_row["hashes"];
                let got_hashes = hashes(geometry);
                for (slot, key) in ["positions", "normals", "uvs", "indices"].into_iter().enumerate() {
                    assert_eq!(got_hashes[slot] as u64, want[key].as_u64().unwrap(), "{context} {key}");
                }
                checked += 1;
            }
        }
        assert_eq!(checked, 105);
    }

    #[test]
    fn cap_and_tier_triangle_budgets_follow_the_reference_rules() {
        let fixture = parse(FIXTURE);
        for (row, got) in cases(&fixture, "species").iter().zip(loaded()) {
            let branch0 = got.tiers[0].branch.triangles();
            assert!(branch0 <= 6000, "{} branch tier 0 {branch0}", got.name);
            for tier in 1..TIER_COUNT {
                assert!(got.tiers[tier].branch.triangles() < got.tiers[tier - 1].branch.triangles(), "{} branch {tier}", got.name);
                assert!(got.tiers[tier].leaf.triangles() < got.tiers[tier - 1].leaf.triangles(), "{} leaf {tier}", got.name);
            }
            assert_eq!(got.shadow.triangles(), got.tiers[2].branch.triangles(), "{}", got.name);
            assert_eq!(row["parts"][0]["sourceTris"].as_u64().unwrap() as usize >= branch0, true, "{}", got.name);
        }
    }

    #[test]
    fn tree_heights_equal_the_target_height() {
        let fixture = parse(FIXTURE);
        for (row, got) in cases(&fixture, "species").iter().zip(loaded()) {
            close(got.height as f64, got.target_h as f64, 1e-4, got.name);
            close(row["sourceHeight"].as_f64().unwrap(), got.target_h as f64, 1e-4, got.name);
            close((got.box_max[1] - got.box_min[1]) as f64, got.target_h as f64, 1e-4, got.name);
            close(got.box_min[1] as f64, 0.0, 1e-4, got.name);
        }
    }

    #[test]
    fn the_materials_name_files_that_exist() {
        let fixture = parse(FIXTURE);
        for (row, got) in cases(&fixture, "species").iter().zip(loaded()) {
            assert_eq!(got.leaf.map.to_str().unwrap(), row["leaf"]["map"].as_str().unwrap(), "{}", got.name);
            assert_eq!(got.bark.maps.color.to_str().unwrap(), row["bark"]["maps"]["color"].as_str().unwrap(), "{}", got.name);
            assert_eq!(got.bark.tint as u64, row["bark"]["tint"].as_u64().unwrap(), "{}", got.name);
            assert_eq!(got.leaf.tint as u64, row["leaf"]["tint"].as_u64().unwrap(), "{}", got.name);
            close(got.leaf.alpha_test as f64, row["leaf"]["alphaTest"].as_f64().unwrap(), 1e-9, got.name);
        }
    }

    #[test]
    fn a_missing_species_file_is_a_refusal() {
        let refusal = load_species(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/planet/no-such-vegetation"))).unwrap_err();
        assert_eq!(refusal.name, "veg-species-file-missing");
    }

    #[test]
    fn a_truncated_or_foreign_file_is_a_refusal() {
        let bytes = std::fs::read(Path::new(DIR).join("bush.veg")).unwrap();
        let truncated = decode_species("Bush", 2.8, &bytes[..bytes.len() - 4]).unwrap_err();
        assert_eq!(truncated.name, "veg-species-truncated");
        let foreign = decode_species("Bush", 2.8, b"GLTF0000").unwrap_err();
        assert_eq!(foreign.name, "veg-species-magic");
        let wrong_height = decode_species("Bush", 2.9, &bytes).unwrap_err();
        assert_eq!(wrong_height.name, "veg-species-mismatch");
    }

    #[test]
    fn loading_notes_one_build_on_the_marked_thread() {
        mark_main_thread();
        loaded();
        assert_eq!(builds_on_main_thread(), 1);
    }
}
