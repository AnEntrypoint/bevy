use super::layers::{build_images, decode_layers};
use crate::planet_lod::origin::FloatingOrigin;
use crate::planet_sky::lut::{AtmosphereLuts, sampling_source};
use crate::refusal::Refusal;
use bevy::asset::{RenderAssetUsages, uuid_handle};
use bevy::math::DVec3;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::render::render_resource::Face;
use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError, TextureDimension, TextureFormat,
    TextureViewDescriptor, TextureViewDimension,
};
use bevy::shader::{Shader, ShaderDefVal, ShaderRef};
use bevy::tasks::{AsyncComputeTaskPool, Task, futures::check_ready};
use std::path::Path;

pub const PLANET_RADIUS_M: f64 = 6_360_000.0;
pub const TEX_TILE_M: f64 = 2400.0;
pub const TEX_WRAP_TILES: f64 = 8.0;
pub const LAYER_COUNT: u32 = 4;
const TEX_NRM_K: f32 = 0.4;
const RELIEF_SHADE: f32 = 6.0;
const FLAT_NORMAL: f32 = 0.0;
const WETNESS: f32 = 0.0;
const POOL_COVER: f32 = 0.18;
const POOL_DISP_LO: [f32; 4] = [0.455, 0.408, 0.275, 0.404];
const POOL_DISP_HI: [f32; 4] = [0.498, 0.545, 0.388, 0.486];
const POOL_SPEC_EXP_ROUGH: f32 = 24.0;
const POOL_SPEC_EXP_SHARP: f32 = 256.0;
const POOL_SLOPE: [f32; 2] = [0.08, 0.25];
const BC_ROCK: [f32; 3] = [0.48, 0.40, 0.30];
const MEAN_L: [f32; 4] = [0.2, 0.2, 0.2, 0.5];
const PLACEHOLDER_ALBEDO: [u8; 4] = [128, 128, 128, 255];
const PLACEHOLDER_NORMAL: [u8; 4] = [128, 128, 255, 255];
const TERRAIN_SHADER: Handle<Shader> = uuid_handle!("7d1c3a52-5e0b-4c58-9a57-3b6f1f0c2e11");
const TEXTURE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/planet/textures");
const TERRAIN_SHADER_PATH: &str = "shaders/planet/terrain_material.wesl";
const TERRAIN_SHADER_SOURCE: &str = include_str!("../../assets/shaders/planet/terrain_material.wesl");
const LUT_TRANSMITTANCE_BINDING: u32 = 105;
const LUT_SCATTERING_BINDING: u32 = 106;
const LUT_SAMPLER_BINDING: u32 = 107;
const PLACEHOLDER_LUT: [u8; 4] = [255, 255, 255, 255];

pub type TerrainMaterial = ExtendedMaterial<StandardMaterial, TerrainExtension>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Census {
    pub remove_layer: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct TerrainKey {
    pub hash_float: bool,
    pub census: Option<Census>,
}

impl TerrainKey {
    fn shader_defs(self) -> Vec<ShaderDefVal> {
        let hash = self.hash_float.then(|| ShaderDefVal::Bool("HASH_FLOAT".into(), true));
        let census = self.census.map(|_| ShaderDefVal::Bool("LAYER_CENSUS".into(), true));
        let removed = self.census.and_then(|c| c.remove_layer).map(|layer| ShaderDefVal::UInt("CENSUS_REMOVE_LAYER".into(), layer));
        hash.into_iter().chain(census).chain(removed).collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Reflect, ShaderType)]
pub struct TerrainParams {
    pub planet_center: Vec3,
    pub def_radius: f32,
    pub tex_cam_frac: Vec3,
    pub relief_scale: f32,
    pub sun_dir: Vec3,
    pub wetness: f32,
    pub bc_rock: Vec3,
    pub pool_cover: f32,
    pub mean_l: Vec4,
    pub pool_lo: Vec4,
    pub pool_hi: Vec4,
    pub pool_spec: Vec4,
    pub albedo_override: Vec4,
    pub surf_ready: f32,
    pub tex_nrm_k: f32,
    pub relief_shade: f32,
    pub flat_normal: f32,
    pub atm_ready: f32,
}

impl Default for TerrainParams {
    fn default() -> Self {
        Self {
            planet_center: Vec3::ZERO,
            def_radius: PLANET_RADIUS_M as f32,
            tex_cam_frac: Vec3::ZERO,
            relief_scale: 0.1,
            sun_dir: Vec3::Y,
            wetness: WETNESS,
            bc_rock: BC_ROCK.into(),
            pool_cover: POOL_COVER,
            mean_l: MEAN_L.into(),
            pool_lo: POOL_DISP_LO.into(),
            pool_hi: POOL_DISP_HI.into(),
            pool_spec: Vec4::new(POOL_SPEC_EXP_ROUGH, POOL_SPEC_EXP_SHARP, POOL_SLOPE[0], POOL_SLOPE[1]),
            albedo_override: Vec4::ZERO,
            surf_ready: 0.0,
            tex_nrm_k: TEX_NRM_K,
            relief_shade: RELIEF_SHADE,
            flat_normal: FLAT_NORMAL,
            atm_ready: 0.0,
        }
    }
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
#[bind_group_data(TerrainKey)]
pub struct TerrainExtension {
    #[uniform(100)]
    pub params: TerrainParams,
    #[texture(101, dimension = "2d_array")]
    #[sampler(102)]
    pub albedo: Handle<Image>,
    #[texture(103, dimension = "2d_array")]
    #[sampler(104)]
    pub normals: Handle<Image>,
    #[texture(105)]
    #[sampler(107)]
    pub transmittance: Handle<Image>,
    #[texture(106, dimension = "2d_array")]
    pub scattering: Handle<Image>,
    #[reflect(ignore)]
    pub key: TerrainKey,
}

impl From<&TerrainExtension> for TerrainKey {
    fn from(extension: &TerrainExtension) -> Self {
        extension.key
    }
}

impl MaterialExtension for TerrainExtension {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(TERRAIN_SHADER)
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.shader_defs.extend(key.bind_group_data.shader_defs());
        }
        Ok(())
    }
}

pub fn terrain_material(key: TerrainKey, images: &mut Assets<Image>) -> TerrainMaterial {
    let base = StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 1.0,
        metallic: 0.0,
        reflectance: 0.0,
        cull_mode: Some(Face::Front),
        ..default()
    };
    let extension = TerrainExtension {
        params: TerrainParams::default(),
        albedo: images.add(placeholder_array(TextureFormat::Rgba8UnormSrgb, PLACEHOLDER_ALBEDO)),
        normals: images.add(placeholder_array(TextureFormat::Rgba8Unorm, PLACEHOLDER_NORMAL)),
        transmittance: images.add(placeholder_flat(PLACEHOLDER_LUT)),
        scattering: images.add(placeholder_array(TextureFormat::Rgba8Unorm, PLACEHOLDER_LUT)),
        key,
    };
    ExtendedMaterial { base, extension }
}

fn placeholder_flat(texel: [u8; 4]) -> Image {
    Image::new(Extent3d { width: 1, height: 1, depth_or_array_layers: 1 }, TextureDimension::D2, texel.to_vec(), TextureFormat::Rgba8Unorm, RenderAssetUsages::RENDER_WORLD)
}

fn placeholder_array(format: TextureFormat, texel: [u8; 4]) -> Image {
    let data = texel.repeat(LAYER_COUNT as usize);
    let size = Extent3d { width: 1, height: 1, depth_or_array_layers: LAYER_COUNT };
    let mut image = Image::new(size, TextureDimension::D2, data, format, RenderAssetUsages::RENDER_WORLD);
    image.texture_view_descriptor = Some(TextureViewDescriptor { dimension: Some(TextureViewDimension::D2Array), ..default() });
    image
}

#[derive(Resource)]
pub struct TerrainMaterialHandle(pub Handle<TerrainMaterial>);

#[derive(Resource, Clone, Copy, Debug)]
pub struct TerrainFrame {
    pub centre: DVec3,
    pub radius: f64,
    pub relief_scale: f64,
}

pub fn tex_tile_m(radius: f64) -> f64 {
    TEX_TILE_M * radius / PLANET_RADIUS_M
}

fn wrapped(period: f64, value: f64) -> f64 {
    value - (value / period).floor() * period
}

pub fn tex_cam_frac(camera_from_centre: DVec3, radius: f64) -> Vec3 {
    let period = tex_tile_m(radius) * TEX_WRAP_TILES;
    DVec3::new(wrapped(period, camera_from_centre.x), wrapped(period, camera_from_centre.y), wrapped(period, camera_from_centre.z)).as_vec3()
}

pub fn framed(mut params: TerrainParams, frame: &TerrainFrame, origin: &FloatingOrigin, camera_local: Vec3, sun_forward: Vec3) -> TerrainParams {
    params.planet_center = origin.to_local(frame.centre);
    params.def_radius = frame.radius as f32;
    params.relief_scale = frame.relief_scale as f32;
    params.tex_cam_frac = tex_cam_frac(origin.to_world(camera_local) - frame.centre, frame.radius);
    params.sun_dir = -sun_forward.normalize_or_zero();
    params
}

fn update_frame(
    frame: Res<TerrainFrame>,
    origin: Res<FloatingOrigin>,
    camera: Query<&Transform, With<Camera3d>>,
    sun: Query<&GlobalTransform, With<DirectionalLight>>,
    handle: Res<TerrainMaterialHandle>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
) {
    let (Ok(camera), Ok(sun), Some(material)) = (camera.single(), sun.single(), materials.get(&handle.0)) else { return };
    let params = framed(material.extension.params, &frame, &origin, camera.translation, sun.forward().as_vec3());
    if params != material.extension.params {
        if let Some(mut material) = materials.get_mut(&handle.0) {
            material.extension.params = params;
        }
    }
}

fn bind_atmosphere(luts: Res<AtmosphereLuts>, handle: Res<TerrainMaterialHandle>, mut materials: ResMut<Assets<TerrainMaterial>>) {
    let Some(material) = materials.get(&handle.0) else { return };
    if material.extension.params.atm_ready > 0.5 && material.extension.transmittance == luts.transmittance && material.extension.scattering == luts.scattering {
        return;
    }
    if let Some(mut material) = materials.get_mut(&handle.0) {
        material.extension.transmittance = luts.transmittance.clone();
        material.extension.scattering = luts.scattering.clone();
        material.extension.params.atm_ready = 1.0;
    }
}

fn insert_material(mut commands: Commands, mut materials: ResMut<Assets<TerrainMaterial>>, mut images: ResMut<Assets<Image>>) {
    commands.insert_resource(TerrainMaterialHandle(materials.add(terrain_material(TerrainKey::default(), &mut images))));
}

pub struct TerrainPipelinePlugin;

impl Plugin for TerrainPipelinePlugin {
    fn build(&self, app: &mut App) {
        let source = [TERRAIN_SHADER_SOURCE, "\n", &sampling_source("constants::MATERIAL_BIND_GROUP", LUT_TRANSMITTANCE_BINDING, LUT_SCATTERING_BINDING, LUT_SAMPLER_BINDING)].concat();
        let shader = Shader::from_wesl(source, TERRAIN_SHADER_PATH);
        if let Some(mut shaders) = app.world_mut().get_resource_mut::<Assets<Shader>>() {
            let _ = shaders.insert(TERRAIN_SHADER.id(), shader);
        }
        app.add_plugins(MaterialPlugin::<TerrainMaterial>::default())
            .add_systems(Startup, insert_material)
            .add_systems(Update, update_frame.run_if(resource_exists::<TerrainFrame>).run_if(resource_exists::<TerrainMaterialHandle>))
            .add_systems(Update, bind_atmosphere.run_if(resource_exists::<AtmosphereLuts>).run_if(resource_exists::<TerrainMaterialHandle>));
    }
}

struct DecodedLayers {
    albedo: Image,
    normals: Image,
    rock_mean: [f32; 3],
    mean_l: [f32; 4],
}

#[derive(Resource)]
struct LayerDecode(Task<Result<DecodedLayers, Refusal>>);

fn decode_all() -> Result<DecodedLayers, Refusal> {
    let layers = decode_layers(&crate::globe::texture_dir().unwrap_or_else(|| Path::new(TEXTURE_DIR).to_path_buf()))?;
    let (albedo, normals) = build_images(&layers);
    Ok(DecodedLayers { albedo, normals, rock_mean: layers.rock_mean, mean_l: layers.mean_l })
}

fn start_decode(mut commands: Commands) {
    commands.insert_resource(LayerDecode(AsyncComputeTaskPool::get().spawn(async { decode_all() })));
}

fn finish_decode(
    mut commands: Commands,
    mut decode: ResMut<LayerDecode>,
    handle: Res<TerrainMaterialHandle>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(result) = check_ready(&mut decode.0) else { return };
    commands.remove_resource::<LayerDecode>();
    let decoded = match result {
        Ok(decoded) => decoded,
        Err(refusal) => return error!("{}: {}", refusal.name, refusal.message),
    };
    let (albedo, normals) = (images.add(decoded.albedo), images.add(decoded.normals));
    if let Some(mut material) = materials.get_mut(&handle.0) {
        material.extension.albedo = albedo;
        material.extension.normals = normals;
        material.extension.params.bc_rock = decoded.rock_mean.into();
        material.extension.params.mean_l = decoded.mean_l.into();
        material.extension.params.surf_ready = 1.0;
    }
}

pub struct TerrainLayersPlugin;

impl Plugin for TerrainLayersPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, start_decode.after(insert_material))
            .add_systems(Update, finish_decode.run_if(resource_exists::<LayerDecode>).run_if(resource_exists::<TerrainMaterialHandle>));
    }
}

pub struct TerrainMaterialPlugin;

impl Plugin for TerrainMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((TerrainPipelinePlugin, TerrainLayersPlugin));
    }
}

#[cfg(test)]
mod tests;
