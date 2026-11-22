mod assets;
mod budget;
mod gpu;
#[cfg(test)]
mod tests;

pub use assets::{TreeAssets, TreeLook};
pub use budget::TextureBudget;

use crate::planet_veg::gpu::buffers::{VegBounds, VegGpuConfig};
use crate::planet_veg::gpu::draw::{VegDrawSet, VegMaterial};
use assets::build_tree_assets;
use bevy::asset::{RenderAssetUsages, uuid_handle};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::renderer::RenderGraph;
use bevy::render::{Render, RenderApp, RenderSystems};
use bevy::core_pipeline::schedule::camera_driver;
use bevy::shader::Shader;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures_lite::future};
use crate::refusal::Refusal;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const TREE_SHADER: Handle<Shader> = uuid_handle!("3f6a0d9e-71c2-4b58-a1d4-5e9c8b2f7a13");
const IMPOSTOR_SHADER: Handle<Shader> = uuid_handle!("b24e7c51-08d3-4f96-9c2a-6d1f3e5a8b70");
const CAPTURE_SHADER: Handle<Shader> = uuid_handle!("c97d15a3-4e60-4b2f-8a71-0f3d6c9e2b58");
const TREE_SOURCE: &str = concat!(
    include_str!("../../assets/shaders/planet/vegetation_common.wgsl"),
    include_str!("../../assets/shaders/planet/vegetation_tree_common.wgsl"),
    include_str!("../../assets/shaders/planet/vegetation_tree_material.wgsl"),
    include_str!("../../assets/shaders/planet/vegetation_tree.wgsl"),
);
const IMPOSTOR_SOURCE: &str = concat!(
    include_str!("../../assets/shaders/planet/vegetation_common.wgsl"),
    include_str!("../../assets/shaders/planet/vegetation_tree_common.wgsl"),
    include_str!("../../assets/shaders/planet/vegetation_octahedral.wgsl"),
    include_str!("../../assets/shaders/planet/vegetation_impostor.wgsl"),
);
const CAPTURE_SOURCE: &str = concat!(
    include_str!("../../assets/shaders/planet/vegetation_octahedral.wgsl"),
    include_str!("../../assets/shaders/planet/vegetation_tree_material.wgsl"),
    include_str!("../../assets/shaders/planet/vegetation_capture.wgsl"),
);
const TREE_SHADER_PATH: &str = "shaders/planet/vegetation_tree.wgsl";
const IMPOSTOR_SHADER_PATH: &str = "shaders/planet/vegetation_impostor.wgsl";
const CAPTURE_SHADER_PATH: &str = "shaders/planet/vegetation_capture.wgsl";

#[derive(Clone, Debug)]
pub struct TreesRenderPlugin {
    pub directory: PathBuf,
    pub budget: TextureBudget,
    pub look: TreeLook,
    pub bake_atlas: bool,
}

impl Default for TreesRenderPlugin {
    fn default() -> Self {
        Self { directory: PathBuf::from("assets/planet/vegetation"), budget: TextureBudget::default(), look: TreeLook::default(), bake_atlas: true }
    }
}

#[derive(Resource, Clone, Default)]
pub struct TreesWitness {
    pub loaded: Arc<AtomicBool>,
    pub baked: Arc<AtomicBool>,
    pub refusal: Arc<Mutex<Option<Refusal>>>,
}

#[derive(Resource, ExtractResource, Clone)]
#[extract_app(RenderApp)]
pub struct TreeRenderData {
    pub assets: Arc<TreeAssets>,
    pub bake_atlas: bool,
}

#[derive(Resource)]
struct TreeLoad {
    task: Task<Result<TreeAssets, Refusal>>,
    bake_atlas: bool,
}

fn tree_material() -> VegMaterial {
    VegMaterial { shader: TREE_SHADER, vertex_entry: "vs_tree", fragment_entry: "fs_tree", group2: Some(gpu::material_layout()), cull_back: false }
}

#[cfg(test)]
fn tree_probe_material() -> VegMaterial {
    VegMaterial { vertex_entry: "vs_tree_probe", fragment_entry: "fs_tree_probe", ..tree_material() }
}

fn impostor_material() -> VegMaterial {
    VegMaterial { shader: IMPOSTOR_SHADER, vertex_entry: "vs_impostor", fragment_entry: "fs_impostor", group2: Some(gpu::impostor_layout()), cull_back: false }
}

fn tier_mesh(geometry: &assets::MergedGeometry) -> Mesh {
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, geometry.positions.clone())
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, geometry.normals.clone())
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, geometry.uvs.clone())
        .with_inserted_indices(Indices::U32(geometry.indices.clone()))
}

fn corner_quad() -> Mesh {
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[-0.5, -0.5, 0.0], [0.5, -0.5, 0.0], [0.5, 0.5, 0.0], [-0.5, 0.5, 0.0]])
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4])
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]])
        .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]))
}

fn spawn_tree_draws(mut commands: Commands, assets: &TreeAssets, config: &VegGpuConfig, meshes: &mut Assets<Mesh>, bounds: &mut VegBounds) {
    let species_meshes: Vec<[Handle<Mesh>; 3]> = assets.tiers.iter().map(|tiers| tiers.each_ref().map(|geometry| meshes.add(tier_mesh(geometry)))).collect();
    let species_count = config.tree_species;
    bounds.tree_geometry = (0..config.tree_geo_count()).map(|geometry| assets.spheres[(geometry % species_count) as usize]).collect();
    bounds.impostor_dims = assets.spheres.clone();
    for geometry in 0..config.tree_geo_count() {
        let species = (geometry % species_count) as usize;
        commands.spawn(VegDrawSet::tree(config, geometry, species_meshes[species].clone(), tree_material()));
    }
    commands.spawn(VegDrawSet::impostor(config, meshes.add(corner_quad()), impostor_material()));
}

fn finish_load(mut commands: Commands, load: Option<ResMut<TreeLoad>>, config: Res<VegGpuConfig>, mut meshes: ResMut<Assets<Mesh>>, mut bounds: ResMut<VegBounds>, witness: Res<TreesWitness>) {
    let Some(mut load) = load else { return };
    let Some(result) = future::block_on(future::poll_once(&mut load.task)) else { return };
    commands.remove_resource::<TreeLoad>();
    match result {
        Ok(assets) if assets.species_count() as u32 == config.tree_species => {
            spawn_tree_draws(commands.reborrow(), &assets, &config, &mut meshes, &mut bounds);
            commands.insert_resource(TreeRenderData { assets: Arc::new(assets), bake_atlas: load.bake_atlas });
            witness.loaded.store(true, Ordering::SeqCst);
        }
        Ok(assets) => *witness.refusal.lock().unwrap() = Some(Refusal { name: "tree-species-count", message: format!("{} species loaded for {} configured", assets.species_count(), config.tree_species) }),
        Err(refusal) => *witness.refusal.lock().unwrap() = Some(refusal),
    }
}

impl Plugin for TreesRenderPlugin {
    fn build(&self, app: &mut App) {
        if let Some(mut shaders) = app.world_mut().get_resource_mut::<Assets<Shader>>() {
            for (handle, source, path) in [(TREE_SHADER, TREE_SOURCE, TREE_SHADER_PATH), (IMPOSTOR_SHADER, IMPOSTOR_SOURCE, IMPOSTOR_SHADER_PATH), (CAPTURE_SHADER, CAPTURE_SOURCE, CAPTURE_SHADER_PATH)] {
                let _ = shaders.insert(handle.id(), Shader::from_wgsl(source, path));
            }
        }
        let (directory, budget, look) = (self.directory.clone(), self.budget, self.look);
        let task = AsyncComputeTaskPool::get_or_init(Default::default).spawn(async move { build_tree_assets(&directory, budget, look) });
        let witness = TreesWitness::default();
        app.insert_resource(TreeLoad { task, bake_atlas: self.bake_atlas })
            .insert_resource(witness.clone())
            .add_plugins(ExtractResourcePlugin::<TreeRenderData>::default())
            .add_systems(Update, finish_load);
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else { return };
        render_app
            .insert_resource(witness)
            .add_systems(Render, (gpu::create_tree_gpu, gpu::write_impostor_params).chain().in_set(RenderSystems::PrepareResources))
            .add_systems(RenderGraph, gpu::run_atlas_bake.before(camera_driver));
    }
}
