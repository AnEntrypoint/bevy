pub mod buffers;
pub mod cull;
pub mod draw;
pub mod store;

use bevy::core_pipeline::core_3d::main_opaque_pass_3d;
use bevy::core_pipeline::schedule::{Core3d, camera_driver};
use bevy::pbr::main_transmissive_pass_3d;
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponentPlugin;
use bevy::render::extract_resource::ExtractResourcePlugin;
use bevy::render::render_resource::SpecializedMeshPipelines;
use bevy::render::renderer::RenderGraph;
use bevy::render::{GpuResourceAppExt, Render, RenderApp, RenderStartup, RenderSystems};

use buffers::{VegBounds, VegGpuConfig, apply_bounds, apply_frame};
use cull::{CULL_SHADER, CULL_SHADER_PATH, CULL_SOURCE, VegCullSettings, VegWitness, init_cull, poll_stats, prepare_cull, run_cull};
use draw::{VegCamera, VegDrawPipeline, VegDrawSet, VegEnvironment, VegMaterialGroups, init_draw, prepare_draws, render_vegetation};
use store::{VegFrame, VegStore, publish_frame};

#[derive(Default)]
pub struct VegGpuPlugin {
    pub config: VegGpuConfig,
}

impl Plugin for VegGpuPlugin {
    fn build(&self, app: &mut App) {
        draw::insert_veg_shader(app, &CULL_SHADER, CULL_SOURCE, CULL_SHADER_PATH);
        let witness = VegWitness::default();
        app.insert_resource(self.config.clone())
            .insert_resource(VegStore::new(&self.config))
            .insert_resource(witness.clone())
            .init_resource::<VegFrame>()
            .init_resource::<VegCullSettings>()
            .init_resource::<VegBounds>()
            .init_resource::<VegEnvironment>()
            .add_plugins((
                ExtractResourcePlugin::<VegFrame>::default(),
                ExtractResourcePlugin::<VegCullSettings>::default(),
                ExtractResourcePlugin::<VegBounds>::default(),
                ExtractResourcePlugin::<VegEnvironment>::default(),
                ExtractComponentPlugin::<VegCamera>::default(),
                ExtractComponentPlugin::<VegDrawSet>::default(),
            ))
            .add_systems(PostUpdate, publish_frame);
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else { return };
        render_app
            .insert_resource(self.config.clone())
            .insert_resource(witness)
            .init_resource::<VegMaterialGroups>()
            .init_gpu_resource::<SpecializedMeshPipelines<VegDrawPipeline>>()
            .add_systems(RenderStartup, (init_cull, init_draw).chain())
            .add_systems(Render, (apply_frame, apply_bounds, poll_stats).in_set(RenderSystems::Prepare))
            .add_systems(Render, prepare_cull.in_set(RenderSystems::PrepareResources))
            .add_systems(Render, prepare_draws.in_set(RenderSystems::PrepareBindGroups))
            .add_systems(RenderGraph, run_cull.before(camera_driver))
            .add_systems(Core3d, render_vegetation.after(main_opaque_pass_3d).before(main_transmissive_pass_3d));
    }
}

#[cfg(test)]
mod render_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
pub(crate) mod twin;
