use super::atmosphere::ATMOSPHERE;
use super::lut::{AtmosphereLuts, sampling_source};
use bevy::asset::uuid_handle;
use bevy::camera::{MainPassResolutionOverride, Viewport};
use bevy::core_pipeline::core_3d::{CORE_3D_DEPTH_FORMAT, main_opaque_pass_3d};
use bevy::core_pipeline::schedule::Core3d;
use bevy::pbr::main_transmissive_pass_3d;
use bevy::prelude::*;
use bevy::render::camera::ExtractedCamera;
use bevy::render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::binding_types::{sampler, texture_2d, texture_2d_array, uniform_buffer};
use bevy::render::render_resource::{
    BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, CachedRenderPipelineId, CompareFunction, DepthBiasState,
    DepthStencilState, FragmentState, PipelineCache, RenderPassDescriptor, RenderPipelineDescriptor, SamplerBindingType, ShaderStages, ShaderType,
    SpecializedRenderPipeline, SpecializedRenderPipelines, StencilFaceState, StencilState, StoreOp, TextureFormat, TextureSampleType, UniformBuffer,
    VertexState, ColorTargetState, ColorWrites,
};
use bevy::render::renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery};
use bevy::render::texture::GpuImage;
use bevy::render::view::{ExtractedView, Msaa, ViewDepthStencilTexture, ViewTarget};
use bevy::render::{GpuResourceAppExt, Render, RenderApp, RenderStartup, RenderSystems};
use bevy::shader::Shader;

pub const SKY_FADE_ALTITUDE_M: f32 = 100_000.0;
const SKY_SHADER: Handle<Shader> = uuid_handle!("3b0f5c2e-8d41-4f7a-9c36-5a1e72d0b4f8");
const SKY_SHADER_PATH: &str = "shaders/planet/atmosphere_sky.wgsl";
const SKY_DISPLAY_SOURCE: &str = include_str!("../../assets/shaders/planet/atmosphere_display.wgsl");
const SKY_SOURCE: &str = include_str!("../../assets/shaders/planet/atmosphere_sky.wgsl");
const SKY_UNIFORM_BINDING: u32 = 0;
const LUT_TRANSMITTANCE_BINDING: u32 = 1;
const LUT_SCATTERING_BINDING: u32 = 2;
const LUT_SAMPLER_BINDING: u32 = 3;
const REFERENCE_PLANET_RADIUS_M: f32 = 6_360_000.0;

#[derive(Resource, ExtractResource, Clone, Copy, Debug, PartialEq)]
#[extract_app(RenderApp)]
pub struct SkySun(pub Vec3);

impl Default for SkySun {
    fn default() -> Self {
        Self(Vec3::Y)
    }
}

#[derive(Resource, ExtractResource, Clone, Copy, Debug, PartialEq)]
#[extract_app(RenderApp)]
pub struct SkyPlanet {
    pub centre: Vec3,
    pub radius: f32,
}

impl Default for SkyPlanet {
    fn default() -> Self {
        Self { centre: Vec3::ZERO, radius: REFERENCE_PLANET_RADIUS_M }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SkyView {
    #[default]
    Sky,
    #[cfg(test)]
    ProbeTransmittance,
    #[cfg(test)]
    ProbeScattering,
    #[cfg(test)]
    ProbeAerialRadiance,
    #[cfg(test)]
    ProbeAerialTransmittance,
    #[cfg(test)]
    ProbeRadiance,
    #[cfg(test)]
    ProbeDisplay,
}

impl SkyView {
    fn fragment_entry(self) -> &'static str {
        match self {
            Self::Sky => "sky_fragment",
            #[cfg(test)]
            Self::ProbeTransmittance => "probe_transmittance",
            #[cfg(test)]
            Self::ProbeScattering => "probe_scattering",
            #[cfg(test)]
            Self::ProbeAerialRadiance => "probe_aerial_radiance",
            #[cfg(test)]
            Self::ProbeAerialTransmittance => "probe_aerial_transmittance",
            #[cfg(test)]
            Self::ProbeRadiance => "probe_radiance",
            #[cfg(test)]
            Self::ProbeDisplay => "probe_display",
        }
    }
}

#[derive(Resource, ExtractResource, Clone, Copy, Debug, PartialEq)]
#[extract_app(RenderApp)]
pub struct SkyConfig {
    pub enabled: bool,
    pub display_exposure: f32,
    pub aces: bool,
    pub view: SkyView,
}

impl Default for SkyConfig {
    fn default() -> Self {
        Self { enabled: true, display_exposure: 1.0, aces: false, view: SkyView::Sky }
    }
}

#[derive(Component, ExtractComponent, Clone, Copy, Debug, Default)]
#[extract_app(RenderApp)]
pub struct SkyCamera;

#[derive(Clone, Copy, Debug, PartialEq, ShaderType)]
pub struct SkyFrame {
    pub view_to_planet: Mat3,
    pub proj_diag: Vec2,
    pub fade: f32,
    pub exposure: f32,
    pub cam_atm: Vec3,
    pub aces: f32,
    pub sun_dir: Vec3,
    pub pad: f32,
}

pub fn sky_frame(world_from_view: &GlobalTransform, clip_from_view: Mat4, planet: &SkyPlanet, sun: &SkySun, config: &SkyConfig) -> SkyFrame {
    let from_centre = world_from_view.translation() - planet.centre;
    SkyFrame {
        view_to_planet: Mat3::from_quat(world_from_view.rotation()),
        proj_diag: Vec2::new(clip_from_view.x_axis.x, clip_from_view.y_axis.y),
        fade: (1.0 - (from_centre.length() - planet.radius) / SKY_FADE_ALTITUDE_M).max(0.0),
        exposure: config.display_exposure,
        cam_atm: from_centre * (ATMOSPHERE.bottom as f32 / planet.radius),
        aces: f32::from(config.aces),
        sun_dir: sun.0.normalize_or(Vec3::Y),
        pad: 0.0,
    }
}

#[derive(Resource)]
struct SkyPipeline {
    layout: BindGroupLayoutDescriptor,
    shader: Handle<Shader>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct SkyKey {
    target_format: TextureFormat,
    samples: u32,
    view: SkyView,
}

impl SpecializedRenderPipeline for SkyPipeline {
    type Key = SkyKey;

    fn specialize(&self, key: SkyKey) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("planet_sky_pipeline".into()),
            layout: vec![self.layout.clone()],
            vertex: VertexState { shader: self.shader.clone(), entry_point: Some("sky_vertex".into()), ..default() },
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(CompareFunction::GreaterEqual),
                stencil: StencilState { front: StencilFaceState::IGNORE, back: StencilFaceState::IGNORE, read_mask: 0, write_mask: 0 },
                bias: DepthBiasState { constant: 0, slope_scale: 0.0, clamp: 0.0 },
            }),
            multisample: bevy::render::render_resource::MultisampleState { count: key.samples, mask: !0, alpha_to_coverage_enabled: false },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                entry_point: Some(key.view.fragment_entry().into()),
                targets: vec![Some(ColorTargetState { format: key.target_format, blend: None, write_mask: ColorWrites::ALL })],
                ..default()
            }),
            ..default()
        }
    }
}

fn init_sky_pipeline(mut commands: Commands) {
    let layout = BindGroupLayoutDescriptor::new(
        "planet_sky_bind_group_layout",
        &BindGroupLayoutEntries::with_indices(
            ShaderStages::FRAGMENT,
            (
                (SKY_UNIFORM_BINDING, uniform_buffer::<SkyFrame>(false)),
                (LUT_TRANSMITTANCE_BINDING, texture_2d(TextureSampleType::Float { filterable: true })),
                (LUT_SCATTERING_BINDING, texture_2d_array(TextureSampleType::Float { filterable: true })),
                (LUT_SAMPLER_BINDING, sampler(SamplerBindingType::Filtering)),
            ),
        ),
    );
    commands.insert_resource(SkyPipeline { layout, shader: SKY_SHADER });
}

#[derive(Component)]
struct SkyPipelineId(CachedRenderPipelineId);

#[derive(Component)]
struct SkyBindGroup(BindGroup);

#[allow(clippy::too_many_arguments)]
fn prepare_sky(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<SkyPipeline>>,
    pipeline: Res<SkyPipeline>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    luts: Option<Res<AtmosphereLuts>>,
    config: Option<Res<SkyConfig>>,
    sun: Option<Res<SkySun>>,
    planet: Option<Res<SkyPlanet>>,
    views: Query<(Entity, &ExtractedView, &Msaa), With<SkyCamera>>,
) {
    let uploaded = luts.as_ref().and_then(|luts| Some((gpu_images.get(&luts.transmittance)?, gpu_images.get(&luts.scattering)?)));
    let (Some((transmittance, scattering)), Some(config), Some(sun), Some(planet)) = (uploaded, config, sun, planet) else {
        for (entity, ..) in &views {
            commands.entity(entity).remove::<(SkyPipelineId, SkyBindGroup)>();
        }
        return;
    };
    for (entity, view, msaa) in &views {
        if !config.enabled {
            commands.entity(entity).remove::<(SkyPipelineId, SkyBindGroup)>();
            continue;
        }
        let key = SkyKey { target_format: view.target_format, samples: msaa.samples(), view: config.view };
        let mut uniform = UniformBuffer::from(sky_frame(&view.world_from_view, view.clip_from_view, &planet, &sun, &config));
        uniform.write_buffer(&device, &queue);
        let bind_group = device.create_bind_group(
            "planet_sky_bind_group",
            &pipeline_cache.get_bind_group_layout(&pipeline.layout),
            &BindGroupEntries::with_indices((
                (SKY_UNIFORM_BINDING, uniform.binding().unwrap()),
                (LUT_TRANSMITTANCE_BINDING, &transmittance.texture_view),
                (LUT_SCATTERING_BINDING, &scattering.texture_view),
                (LUT_SAMPLER_BINDING, &transmittance.sampler),
            )),
        );
        commands.entity(entity).insert((SkyPipelineId(pipelines.specialize(&pipeline_cache, &pipeline, key)), SkyBindGroup(bind_group)));
    }
}

fn render_sky(
    view: ViewQuery<(&ExtractedCamera, &ViewTarget, &ViewDepthStencilTexture, &SkyPipelineId, &SkyBindGroup, Option<&MainPassResolutionOverride>)>,
    pipeline_cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let (camera, target, depth, pipeline_id, bind_group, resolution_override) = view.into_inner();
    let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_id.0) else { return };
    let color_attachments = [Some(target.get_color_attachment())];
    let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("planet_sky"),
        color_attachments: &color_attachments,
        depth_stencil_attachment: Some(depth.get_attachment(StoreOp::Store)),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    if let Some(viewport) = Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override) {
        pass.set_camera_viewport(&viewport);
    }
    pass.set_render_pipeline(pipeline);
    pass.set_bind_group(0, &bind_group.0, &[]);
    pass.draw(0..3, 0..1);
}

pub struct PlanetSkyPlugin;

impl Plugin for PlanetSkyPlugin {
    fn build(&self, app: &mut App) {
        let source = [SKY_DISPLAY_SOURCE, &sampling_source("0", LUT_TRANSMITTANCE_BINDING, LUT_SCATTERING_BINDING, LUT_SAMPLER_BINDING), SKY_SOURCE].concat();
        if let Some(mut shaders) = app.world_mut().get_resource_mut::<Assets<Shader>>() {
            let _ = shaders.insert(SKY_SHADER.id(), Shader::from_wgsl(source, SKY_SHADER_PATH));
        }
        app.init_resource::<SkyConfig>().init_resource::<SkySun>().init_resource::<SkyPlanet>().add_plugins((
            ExtractComponentPlugin::<SkyCamera>::default(),
            ExtractResourcePlugin::<SkyConfig>::default(),
            ExtractResourcePlugin::<SkySun>::default(),
            ExtractResourcePlugin::<SkyPlanet>::default(),
            ExtractResourcePlugin::<AtmosphereLuts>::default(),
        ));
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else { return };
        render_app
            .init_gpu_resource::<SpecializedRenderPipelines<SkyPipeline>>()
            .add_systems(RenderStartup, init_sky_pipeline)
            .add_systems(Render, prepare_sky.in_set(RenderSystems::PrepareBindGroups))
            .add_systems(Core3d, render_sky.after(main_opaque_pass_3d).before(main_transmissive_pass_3d));
    }
}

#[cfg(test)]
mod tests;
