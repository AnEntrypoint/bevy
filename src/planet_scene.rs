use crate::globe::{GlobeWorld, align_lights_with_focus, light_vegetation, texture_dir};
use crate::planet_material::terrain::TerrainMaterialPlugin;
use crate::planet_sky::light::LightPlugin;
use crate::planet_sky::lut::{AtmosphereLutPlugin, luts_ready};
use crate::planet_sky::sky::{PlanetSkyPlugin, SkyPlanet};
use crate::planet_sky::tonemap::ToneMapPlugin;
use crate::planet_veg::gpu::VegGpuPlugin;
use crate::planet_veg::grass_render::GrassRenderPlugin;
use crate::planet_veg::rock_render::RockRenderPlugin;
use crate::planet_veg::trees_render::TreesRenderPlugin;
use bevy::app::App;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetBytesPerFrame;

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

