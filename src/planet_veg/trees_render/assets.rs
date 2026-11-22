use super::budget::{TextureBudget, halved_to_budget, mip_chain, mip_levels};
use crate::planet_veg::species::{PartGeometry, SpeciesGeometry, TierGeometry, TIER_COUNT, load_species};
use crate::refusal::Refusal;
use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, Image, ImageSampler, ImageType};
use bevy::render::render_resource::TextureFormat;
use std::path::{Path, PathBuf};

pub const BARK_UV_MARK: f32 = -1.0;
pub const TABLE_SPECIES: usize = 16;
pub const TABLE_BYTES: usize = TABLE_SPECIES * 48 + 16;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TreeLook {
    pub alpha_test: bool,
    pub tint: bool,
}

impl Default for TreeLook {
    fn default() -> Self {
        Self { alpha_test: true, tint: true }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MergedGeometry {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayerStack {
    pub size_px: u32,
    pub layers: u32,
    pub mips: u32,
    pub data: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpeciesMaterial {
    pub bark_tint: [f32; 3],
    pub bark_layer: u32,
    pub leaf_tint: [f32; 3],
    pub leaf_layer: u32,
    pub bark_scale: [f32; 2],
    pub leaf_alpha_test: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TreeAssets {
    pub tiers: Vec<[MergedGeometry; TIER_COUNT]>,
    pub spheres: Vec<[f32; 4]>,
    pub bark: LayerStack,
    pub leaf: LayerStack,
    pub materials: Vec<SpeciesMaterial>,
    pub look: TreeLook,
}

const LEAF_ALPHA_TEST_FLOOR: f32 = 0.5;

fn refuse(name: &'static str, message: String) -> Refusal {
    Refusal { name, message }
}

fn bark_part(part: &PartGeometry) -> impl Iterator<Item = [f32; 2]> + '_ {
    part.uvs.iter().map(|uv| [BARK_UV_MARK - uv[0], uv[1]])
}

pub fn merge_tier(tier: &TierGeometry) -> MergedGeometry {
    let (branch, leaf) = (&tier.branch, &tier.leaf);
    let leaf_base = branch.vertex_count() as u32;
    MergedGeometry {
        positions: branch.positions.iter().chain(&leaf.positions).copied().collect(),
        normals: branch.normals.iter().chain(&leaf.normals).copied().collect(),
        uvs: bark_part(branch).chain(leaf.uvs.iter().copied()).collect(),
        indices: branch.indices.iter().copied().chain(leaf.indices.iter().map(|index| index + leaf_base)).collect(),
    }
}

fn srgb_to_linear(channel: f32) -> f32 {
    if channel <= 0.04045 { channel / 12.92 } else { ((channel + 0.055) / 1.055).powf(2.4) }
}

fn hex_to_linear(hex: u32) -> [f32; 3] {
    [16, 8, 0].map(|shift| srgb_to_linear(((hex >> shift) & 255) as f32 / 255.0))
}

fn decode_rgba(path: &Path) -> Result<(Vec<u8>, u32), Refusal> {
    let bytes = std::fs::read(path).map_err(|error| refuse("tree-texture-unreadable", format!("{}: {error}", path.display())))?;
    let extension = path.extension().and_then(|extension| extension.to_str()).unwrap_or_default().to_owned();
    let image = Image::from_buffer(&bytes, ImageType::Extension(&extension), CompressedImageFormats::NONE, true, ImageSampler::Default, RenderAssetUsages::default())
        .map_err(|error| refuse("tree-texture-undecodable", format!("{}: {error}", path.display())))?;
    let (width, height) = (image.width(), image.height());
    let format = image.texture_descriptor.format;
    match (format, width == height, image.data) {
        (TextureFormat::Rgba8UnormSrgb, true, Some(data)) => Ok((data, width)),
        _ => Err(refuse("tree-texture-format", format!("{}: {format:?} {width}x{height}", path.display()))),
    }
}

fn layer_stack(dir: &Path, files: &[PathBuf], max_px: u32) -> Result<LayerStack, Refusal> {
    let mut size_px = 0;
    let mut data = Vec::new();
    for file in files {
        let (rgba, source_px) = decode_rgba(&dir.join(file))?;
        let (rgba, budgeted_px) = halved_to_budget(rgba, source_px, max_px)?;
        if size_px != 0 && size_px != budgeted_px {
            return Err(refuse("tree-texture-sizes-differ", format!("{} is {budgeted_px} px beside {size_px} px", file.display())));
        }
        size_px = budgeted_px;
        data.extend(mip_chain(&rgba, budgeted_px));
    }
    Ok(LayerStack { size_px, layers: files.len() as u32, mips: mip_levels(size_px), data })
}

fn layer_of(files: &mut Vec<PathBuf>, file: &Path) -> u32 {
    let at = files.iter().position(|known| known == file).unwrap_or_else(|| {
        files.push(file.to_path_buf());
        files.len() - 1
    });
    at as u32
}

fn materials_and_files(species: &[SpeciesGeometry]) -> (Vec<SpeciesMaterial>, Vec<PathBuf>, Vec<PathBuf>) {
    let (mut bark_files, mut leaf_files) = (Vec::new(), Vec::new());
    let materials = species
        .iter()
        .map(|geometry| SpeciesMaterial {
            bark_tint: hex_to_linear(geometry.bark.tint),
            bark_layer: layer_of(&mut bark_files, &geometry.bark.maps.color),
            leaf_tint: hex_to_linear(geometry.leaf.tint),
            leaf_layer: layer_of(&mut leaf_files, &geometry.leaf.map),
            bark_scale: geometry.bark.texture_scale,
            leaf_alpha_test: geometry.leaf.alpha_test.max(LEAF_ALPHA_TEST_FLOOR),
        })
        .collect();
    (materials, bark_files, leaf_files)
}

pub fn build_tree_assets(dir: &Path, budget: TextureBudget, look: TreeLook) -> Result<TreeAssets, Refusal> {
    let species = load_species(dir)?;
    let (materials, bark_files, leaf_files) = materials_and_files(&species);
    Ok(TreeAssets {
        tiers: species.iter().map(|geometry| geometry.tiers.each_ref().map(merge_tier)).collect(),
        spheres: species.iter().map(|geometry| [geometry.sphere_centre[0], geometry.sphere_centre[1], geometry.sphere_centre[2], geometry.sphere_radius]).collect(),
        bark: layer_stack(dir, &bark_files, budget.bark_max_px)?,
        leaf: layer_stack(dir, &leaf_files, budget.leaf_max_px)?,
        materials,
        look,
    })
}

impl TreeAssets {
    pub fn table_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(TABLE_BYTES);
        let mut push = |values: &[f32]| values.iter().for_each(|value| bytes.extend_from_slice(&value.to_le_bytes()));
        for material in &self.materials {
            push(&[material.bark_tint[0], material.bark_tint[1], material.bark_tint[2], material.bark_layer as f32]);
            push(&[material.leaf_tint[0], material.leaf_tint[1], material.leaf_tint[2], material.leaf_layer as f32]);
            push(&[material.bark_scale[0], material.bark_scale[1], material.leaf_alpha_test, 0.0]);
        }
        let padding = (TABLE_SPECIES - self.materials.len()) * 12;
        push(&vec![0.0; padding]);
        push(&[f32::from(u8::from(self.look.alpha_test)), f32::from(u8::from(self.look.tint)), 0.0, 0.0]);
        bytes
    }

    pub fn species_count(&self) -> usize {
        self.tiers.len()
    }
}

#[cfg(test)]
pub fn species_directory() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/planet/vegetation"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loaded() -> TreeAssets {
        build_tree_assets(&species_directory(), TextureBudget::default(), TreeLook::default()).unwrap()
    }

    #[test]
    fn merged_tiers_keep_every_vertex_and_triangle_and_mark_the_bark_uvs() {
        let species = load_species(&species_directory()).unwrap();
        let assets = loaded();
        assert_eq!(assets.species_count(), 15);
        for (geometry, tiers) in species.iter().zip(&assets.tiers) {
            for (tier, merged) in geometry.tiers.iter().zip(tiers) {
                let branch_vertices = tier.branch.vertex_count();
                assert_eq!(merged.positions.len(), branch_vertices + tier.leaf.vertex_count(), "{}", geometry.name);
                assert_eq!(merged.indices.len(), tier.branch.indices.len() + tier.leaf.indices.len(), "{}", geometry.name);
                assert!(merged.indices.iter().all(|index| (*index as usize) < merged.positions.len()), "{}", geometry.name);
                assert!(merged.uvs[..branch_vertices].iter().all(|uv| uv[0] < -0.5), "{} bark uvs", geometry.name);
                assert!(merged.uvs[branch_vertices..].iter().all(|uv| uv[0] > -0.5), "{} leaf uvs", geometry.name);
            }
        }
    }

    #[test]
    fn the_budget_sets_the_texture_sizes_and_the_layers_are_the_unique_files() {
        let assets = loaded();
        assert_eq!((assets.leaf.size_px, assets.bark.size_px), (128, 64));
        assert_eq!((assets.leaf.layers, assets.bark.layers), (4, 3));
        assert_eq!((assets.leaf.mips, assets.bark.mips), (8, 7));
        assert_eq!(assets.leaf.data.len() as u64, 4 * super::super::budget::mip_chain_bytes(128, 128));
        assert_eq!(assets.bark.data.len() as u64, 3 * super::super::budget::mip_chain_bytes(64, 64));
        let larger = build_tree_assets(&species_directory(), TextureBudget::new(256, 128).unwrap(), TreeLook::default()).unwrap();
        assert_eq!((larger.leaf.size_px, larger.bark.size_px), (256, 128));
    }

    #[test]
    fn the_material_table_carries_tints_layers_and_the_alpha_floor() {
        let assets = loaded();
        let species = load_species(&species_directory()).unwrap();
        assert!(assets.materials.iter().zip(&species).all(|(material, geometry)| material.leaf_alpha_test == geometry.leaf.alpha_test.max(0.5)));
        assert!(assets.materials.iter().all(|material| material.bark_layer < 3 && material.leaf_layer < 4));
        assert_eq!(assets.table_bytes().len(), TABLE_BYTES);
        let oak = &assets.materials[0];
        assert!((oak.leaf_tint[0] - srgb_to_linear(((14013901u32 >> 16) & 255) as f32 / 255.0)).abs() < 1e-6);
        assert_eq!(oak.bark_scale, [1.0, 10.0]);
    }
}
