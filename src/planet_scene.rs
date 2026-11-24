use crate::globe::{GlobeWorld, align_lights_with_focus, light_vegetation, texture_dir};
use crate::planet_material::terrain::TerrainMaterialPlugin;
use crate::planet_sky::light::LightPlugin;
use crate::planet_lod::stream::PlanetLodStats;
use crate::planet_material::terrain::{TerrainMaterial, TerrainMaterialHandle};
use crate::planet_sky::lut::{AtmosphereLutPlugin, AtmosphereLuts, luts_ready};
use crate::planet_sky::sky::{PlanetSkyPlugin, SkyPlanet};
use crate::planet_sky::tonemap::ToneMapPlugin;
use crate::planet_veg::gpu::VegGpuPlugin;
use crate::planet_veg::gpu::store::VegFrame;
use crate::planet_veg::stream::VegStreamStats;
use crate::planet_veg::trees_render::TreesWitness;
use crate::planet_veg::grass_render::GrassRenderPlugin;
use crate::planet_veg::rock_render::RockRenderPlugin;
use crate::planet_veg::trees_render::TreesRenderPlugin;
use bevy::app::App;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetBytesPerFrame;
use std::sync::atomic::Ordering;

pub(crate) const SETTLED_STREAK: usize = 4;

pub(crate) fn settled(world: &World) -> bool {
    unsettled(world).is_empty()
}

pub(crate) fn unsettled(world: &World) -> Vec<&'static str> {
    let lod = world.resource::<PlanetLodStats>();
    let veg = world.resource::<VegStreamStats>();
    let uploads = world.resource::<VegFrame>();
    let trees = world.resource::<TreesWitness>();
    let layers = world.resource::<Assets<TerrainMaterial>>().get(&world.resource::<TerrainMaterialHandle>().0).is_some_and(|material| material.extension.params.surf_ready == 1.0);
    [
        (lod.live_patches > 0, "patches"),
        (lod.tasks_in_flight == 0, "patch-tasks"),
        (lod.waiting_on_climate == 0, "climate"),
        (veg.tasks_in_flight == 0, "veg-tasks"),
        (veg.queued_chunks == [0; 3], "veg-queued"),
        (uploads.writes.iter().all(Vec::is_empty), "veg-uploads"),
        (world.contains_resource::<AtmosphereLuts>(), "luts"),
        (trees.loaded.load(Ordering::SeqCst), "trees-loaded"),
        (trees.baked.load(Ordering::SeqCst), "trees-baked"),
        (layers, "layers"),
    ]
    .into_iter()
    .filter(|(ok, _)| !ok)
    .map(|(_, name)| name)
    .collect()
}

pub(crate) fn add_vegetation(app: &mut App, globe: &GlobeWorld) {
    app.add_plugins((VegGpuPlugin::default(), TreesRenderPlugin::default(), RockRenderPlugin, GrassRenderPlugin::default(), globe.veg_stream_plugin()))
        .add_systems(PostUpdate, light_vegetation.after(align_lights_with_focus));
}

fn warn_missing_textures() {
    if texture_dir().is_none() {
        warn!("planet textures not found: terrain keeps the placeholder look");
    }
}

pub(crate) const RENDER_ASSET_BYTES_PER_FRAME: usize = 4 << 20;

fn cap_render_asset_uploads(mut commands: Commands) {
    commands.insert_resource(RenderAssetBytesPerFrame::new(RENDER_ASSET_BYTES_PER_FRAME));
}


pub fn add_planet(app: &mut App, globe: &GlobeWorld) {
    app.add_plugins((
        globe.lod_plugin(),
        TerrainMaterialPlugin,
        globe.water_plugin(),
        AtmosphereLutPlugin,
        PlanetSkyPlugin,
        LightPlugin { quality: globe.quality },
        ToneMapPlugin,
    ))
    .insert_resource(globe.terrain_frame())
    .insert_resource(SkyPlanet { centre: globe.center, radius: globe.radius })
    .add_systems(Startup, warn_missing_textures)
    .add_systems(Update, cap_render_asset_uploads.run_if(luts_ready).run_if(not(resource_exists::<RenderAssetBytesPerFrame>)));
    add_vegetation(app, globe);
}

