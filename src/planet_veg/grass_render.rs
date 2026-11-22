use super::gpu::buffers::Packer;
use super::gpu::cull::{GrassCullSettings, VegCullSettings};
use super::gpu::draw::{VegDrawSet, VegEnvironment, VegMaterial, VegMaterialGroups, insert_veg_shader, prepare_draws, veg_mesh};
use crate::planet_lod::guard;
use bevy::asset::{RenderAssetUsages, uuid_handle};
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{
    BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferInitDescriptor, BufferUsages, Extent3d, PipelineCache, SamplerBindingType,
    ShaderStages, TextureDimension, TextureFormat, TextureSampleType, TextureViewDimension,
};
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::render::texture::GpuImage;
use bevy::render::{Render, RenderApp, RenderSystems};
use bevy::shader::Shader;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures::check_ready};
use std::f64::consts::FRAC_PI_2;

pub const GRASS_SHADER: Handle<Shader> = uuid_handle!("3b7f5e21-8c04-4d19-a6e2-91d0c4f78a35");
const GRASS_SHADER_PATH: &str = "shaders/planet/vegetation_grass.wgsl";
const GRASS_SOURCE: &str = concat!(include_str!("../../assets/shaders/planet/vegetation_common.wgsl"), include_str!("../../assets/shaders/planet/vegetation_grass.wgsl"));

pub const NEAR_SEGMENTS: u32 = 2;
pub const MID_SEGMENTS: u32 = 1;
pub const CLUMP_CARDS: u32 = 1;

pub const CLUMP_TEX_SIZE: usize = 256;
pub const CLUMP_BLADES: usize = 22;
pub const CLUMP_SEED: u32 = 0x2f6e2b1;
pub const BLADE_STEPS: usize = 14;
pub const CLUMP_MIP_LEVELS: u32 = 9;
const CLUMP_SUPERSAMPLES: usize = 4;
const CLUMP_ANISOTROPY: u16 = 4;

pub const GRASS_ANIM_SPEED: f32 = 0.25;
pub const GRASS_PARAMS_BYTES: usize = 80;

#[derive(Resource, ExtractResource, Clone, Copy, Debug, PartialEq)]
#[extract_app(RenderApp)]
pub struct GrassLook {
    pub wind_origin: Vec2,
    pub wind_axis_x: Vec3,
    pub wind_axis_z: Vec3,
    pub wind_direction: Vec2,
    pub wind_amplitude: f32,
}

impl Default for GrassLook {
    fn default() -> Self {
        Self { wind_origin: Vec2::ZERO, wind_axis_x: Vec3::X, wind_axis_z: Vec3::Z, wind_direction: Vec2::new(0.8, 0.6), wind_amplitude: 0.25 }
    }
}

#[derive(Resource, ExtractResource, Clone, Debug)]
#[extract_app(RenderApp)]
struct GrassClumpImage(Handle<Image>);

#[derive(Clone, Debug, PartialEq)]
pub struct ClumpGeometry {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

pub fn clump_geometry(segments: u32, cards: u32) -> ClumpGeometry {
    let (segments, cards) = (segments.max(1), cards.max(1));
    let mut geometry = ClumpGeometry { positions: Vec::new(), normals: Vec::new(), uvs: Vec::new(), indices: Vec::new() };
    for card in 0..cards {
        let angle = f64::from(card) / f64::from(cards) * std::f64::consts::PI;
        let (along_x, along_z) = (angle.cos() as f32, angle.sin() as f32);
        let first = geometry.positions.len() as u32;
        for step in 0..=segments {
            let v = step as f32 / segments as f32;
            for u in [-0.5f32, 0.5] {
                geometry.positions.push([along_x * u, v, along_z * u]);
                geometry.normals.push([-along_z, 0.0, along_x]);
                geometry.uvs.push([u + 0.5, v]);
            }
        }
        for step in 0..segments {
            let row = first + step * 2;
            geometry.indices.extend_from_slice(&[row, row + 1, row + 2, row + 1, row + 3, row + 2]);
        }
    }
    geometry
}

pub fn clump_mesh(segments: u32) -> Mesh {
    let geometry = clump_geometry(segments, CLUMP_CARDS);
    veg_mesh(geometry.positions, geometry.normals, geometry.uvs, geometry.indices)
}

struct Mulberry32(u32);

impl Mulberry32 {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x6d2b79f5);
        let a = self.0;
        let mut t = (a ^ (a >> 15)).wrapping_mul(a | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        f64::from(t ^ (t >> 14)) / 4_294_967_296.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClumpBlade {
    pub base_x: f64,
    pub height: f64,
    pub lean: f64,
    pub curve: f64,
    pub half_width: f64,
    pub shade: f64,
}

pub fn clump_blades() -> Vec<ClumpBlade> {
    let mut rand = Mulberry32(CLUMP_SEED);
    let mut blades: Vec<ClumpBlade> = (0..CLUMP_BLADES)
        .map(|index| {
            let t = (index as f64 + rand.next() * 0.6) / CLUMP_BLADES as f64;
            let base_x = 0.1 + 0.8 * t;
            let outward = (base_x - 0.5) * 2.0;
            let height = 0.5 + 0.48 * (1.0 - outward.abs() * 0.55) * (0.65 + 0.35 * rand.next());
            let lean = outward * (0.03 + 0.05 * rand.next()) + (rand.next() - 0.5) * 0.02;
            let curve = (rand.next() - 0.5) * 0.06 + outward * 0.02;
            let half_width = 0.03 + 0.022 * rand.next();
            let shade = 0.62 + 0.38 * rand.next();
            ClumpBlade { base_x, height, lean, curve, half_width, shade }
        })
        .collect();
    blades.sort_by(|a, b| a.height.total_cmp(&b.height));
    blades
}

impl ClumpBlade {
    fn grey(&self) -> f32 {
        (255.0 * self.shade).round() as f32 / 255.0
    }

    fn edges(&self) -> [(f64, f64); BLADE_STEPS + 1] {
        let size = CLUMP_TEX_SIZE as f64;
        std::array::from_fn(|step| {
            let v = step as f64 / BLADE_STEPS as f64;
            let centre = self.base_x + (self.lean * v + self.curve * v * v) * size;
            let span = self.half_width * size * (1.0 - v).powf(0.85) * (0.55 + 0.45 * ((v * 3.0 + 0.35).min(1.0) * FRAC_PI_2).sin());
            (centre, span)
        })
    }

    fn covers(&self, edges: &[(f64, f64); BLADE_STEPS + 1], x: f64, y: f64) -> bool {
        let size = CLUMP_TEX_SIZE as f64;
        let t = (size - y) / (self.height * size);
        if !(0.0..=1.0).contains(&t) {
            return false;
        }
        let scaled = t * BLADE_STEPS as f64;
        let step = (scaled.floor() as usize).min(BLADE_STEPS - 1);
        let fraction = scaled - step as f64;
        let (low, high) = (edges[step], edges[step + 1]);
        let centre = low.0 + (high.0 - low.0) * fraction;
        let span = low.1 + (high.1 - low.1) * fraction;
        (x - centre).abs() <= span
    }
}

pub fn bake_clump_mips() -> Vec<u8> {
    guard::note_build();
    let blades = clump_blades();
    let edges: Vec<_> = blades.iter().map(ClumpBlade::edges).collect();
    let greys: Vec<_> = blades.iter().map(ClumpBlade::grey).collect();
    let mut level: Vec<f32> = Vec::with_capacity(CLUMP_TEX_SIZE * CLUMP_TEX_SIZE);
    let samples = (CLUMP_SUPERSAMPLES * CLUMP_SUPERSAMPLES) as f32;
    for row in 0..CLUMP_TEX_SIZE {
        for column in 0..CLUMP_TEX_SIZE {
            let mut total = 0.0;
            for sy in 0..CLUMP_SUPERSAMPLES {
                for sx in 0..CLUMP_SUPERSAMPLES {
                    let x = column as f64 + (sx as f64 + 0.5) / CLUMP_SUPERSAMPLES as f64;
                    let y = row as f64 + (sy as f64 + 0.5) / CLUMP_SUPERSAMPLES as f64;
                    let painted = (0..blades.len()).rev().find(|blade| blades[*blade].covers(&edges[*blade], x, y));
                    total += painted.map_or(0.0, |blade| greys[blade]);
                }
            }
            level.push(total / samples);
        }
    }
    let mut bytes = Vec::new();
    let mut side = CLUMP_TEX_SIZE;
    loop {
        bytes.extend(level.iter().map(|value| (value * 255.0).round() as u8));
        if side == 1 {
            break;
        }
        let half = side / 2;
        level = (0..half * half)
            .map(|at| {
                let (row, column) = (at / half * 2, at % half * 2);
                (level[row * side + column] + level[row * side + column + 1] + level[(row + 1) * side + column] + level[(row + 1) * side + column + 1]) / 4.0
            })
            .collect();
        side = half;
    }
    bytes
}

fn clump_image(bytes: Vec<u8>) -> Image {
    let size = Extent3d { width: CLUMP_TEX_SIZE as u32, height: CLUMP_TEX_SIZE as u32, depth_or_array_layers: 1 };
    let mut image = Image::new_uninit(size, TextureDimension::D2, TextureFormat::R8Unorm, RenderAssetUsages::RENDER_WORLD);
    image.data = Some(bytes);
    image.texture_descriptor.mip_level_count = CLUMP_MIP_LEVELS;
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: CLUMP_ANISOTROPY,
        ..default()
    });
    image
}

struct GrassAssets {
    near: Mesh,
    mid: Mesh,
    clump: Image,
}

fn build_assets() -> GrassAssets {
    guard::note_build();
    GrassAssets { near: clump_mesh(NEAR_SEGMENTS), mid: clump_mesh(MID_SEGMENTS), clump: clump_image(bake_clump_mips()) }
}

#[derive(Resource)]
struct GrassBuild(Task<GrassAssets>);

#[derive(Resource, Clone, Copy)]
struct GrassEntries(&'static str, &'static str);

fn start_bake(mut commands: Commands) {
    commands.insert_resource(GrassBuild(AsyncComputeTaskPool::get().spawn(async { build_assets() })));
}

fn group_layout() -> BindGroupLayoutDescriptor {
    let entry = |binding: u32, visibility: ShaderStages, ty: BindingType| BindGroupLayoutEntry { binding, visibility, ty, count: None };
    BindGroupLayoutDescriptor::new(
        "veg_grass_material",
        &[
            entry(0, ShaderStages::FRAGMENT, BindingType::Texture { sample_type: TextureSampleType::Float { filterable: true }, view_dimension: TextureViewDimension::D2, multisampled: false }),
            entry(1, ShaderStages::FRAGMENT, BindingType::Sampler(SamplerBindingType::Filtering)),
            entry(2, ShaderStages::VERTEX, BindingType::Buffer { ty: BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }),
        ],
    )
}

fn finish_bake(mut commands: Commands, mut build: ResMut<GrassBuild>, entries: Res<GrassEntries>, mut meshes: ResMut<Assets<Mesh>>, mut images: ResMut<Assets<Image>>) {
    let Some(assets) = check_ready(&mut build.0) else { return };
    commands.remove_resource::<GrassBuild>();
    commands.insert_resource(GrassClumpImage(images.add(assets.clump)));
    let material = VegMaterial { shader: GRASS_SHADER, vertex_entry: entries.0, fragment_entry: entries.1, group2: Some(group_layout()), cull_back: false };
    commands.spawn(VegDrawSet::grass(meshes.add(assets.near), meshes.add(assets.mid), material));
}

pub fn grass_params_bytes(look: &GrassLook, environment: &VegEnvironment, settings: &GrassCullSettings) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(GRASS_PARAMS_BYTES);
    Packer(&mut bytes)
        .f32s(&look.wind_axis_x.to_array())
        .f32s(&[environment.time_s * GRASS_ANIM_SPEED])
        .f32s(&look.wind_axis_z.to_array())
        .f32s(&[look.wind_amplitude * environment.wind_strength])
        .f32s(&look.wind_origin.to_array())
        .f32s(&look.wind_direction.to_array())
        .f32s(&[settings.edge_cap_m, settings.full_radius_m, settings.fade_m, settings.width_min, settings.width_span, settings.height_scale, 0.0, 0.0]);
    bytes
}

#[allow(clippy::too_many_arguments)]
fn prepare_grass_group(
    mut groups: ResMut<VegMaterialGroups>,
    mut uniform: Local<Option<Buffer>>,
    clump: Option<Res<GrassClumpImage>>,
    look: Option<Res<GrassLook>>,
    environment: Option<Res<VegEnvironment>>,
    settings: Option<Res<VegCullSettings>>,
    images: Res<RenderAssets<GpuImage>>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
) {
    let (Some(look), Some(environment), Some(settings)) = (look, environment, settings) else { return };
    let bytes = grass_params_bytes(&look, &environment, &settings.grass);
    if let Some(buffer) = uniform.as_ref() {
        queue.write_buffer(buffer, 0, &bytes);
        return;
    }
    let Some(image) = clump.and_then(|clump| images.get(&clump.0)) else { return };
    let buffer = device.create_buffer_with_data(&BufferInitDescriptor { label: Some("veg_grass_params"), contents: &bytes, usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST });
    let group = device.create_bind_group(
        "veg_grass_material_group",
        &cache.get_bind_group_layout(&group_layout()),
        &BindGroupEntries::with_indices(((0, &image.texture_view), (1, &image.sampler), (2, buffer.as_entire_binding()))),
    );
    groups.0.insert(GRASS_SHADER.id(), group);
    *uniform = Some(buffer);
}

pub struct GrassRenderPlugin {
    pub entries: (&'static str, &'static str),
}

impl Default for GrassRenderPlugin {
    fn default() -> Self {
        Self { entries: ("vs_grass", "fs_grass") }
    }
}

impl Plugin for GrassRenderPlugin {
    fn build(&self, app: &mut App) {
        insert_veg_shader(app, &GRASS_SHADER, GRASS_SOURCE, GRASS_SHADER_PATH);
        app.insert_resource(GrassEntries(self.entries.0, self.entries.1))
            .init_resource::<GrassLook>()
            .add_plugins((ExtractResourcePlugin::<GrassLook>::default(), ExtractResourcePlugin::<GrassClumpImage>::default()))
            .add_systems(Startup, start_bake)
            .add_systems(Update, finish_bake.run_if(resource_exists::<GrassBuild>));
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else { return };
        render_app.add_systems(Render, prepare_grass_group.in_set(RenderSystems::PrepareBindGroups).before(prepare_draws));
    }
}

#[cfg(test)]
mod render_tests;
#[cfg(test)]
mod tests;
