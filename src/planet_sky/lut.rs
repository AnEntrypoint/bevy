use super::atmosphere::{
    ATMOSPHERE, SCATTERING_HEIGHT, SCATTERING_LAYERS, SCATTERING_STEPS, SCATTERING_WIDTH, ScatteringLut, TRANSMITTANCE_HEIGHT, TRANSMITTANCE_STEPS,
    TRANSMITTANCE_WIDTH, TransmittanceLut, bake_scattering, bake_transmittance,
};
use crate::planet_lod::guard::note_build;
use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::RenderApp;
use bevy::render::extract_resource::ExtractResource;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension};
use bevy::tasks::{AsyncComputeTaskPool, Task, futures::check_ready};

const ATMOSPHERE_CORE: &str = include_str!("../../assets/shaders/planet/atmosphere_core.wgsl");
const ATMOSPHERE_LUT_SAMPLE: &str = include_str!("../../assets/shaders/planet/atmosphere_lut_sample.wgsl");
const HALF_EXPONENT_BIAS: i32 = 15;
const HALF_INFINITY: u16 = 0x7c00;
const HALF_MANTISSA_BITS: u32 = 10;
const SINGLE_MANTISSA_BITS: u32 = 23;

#[derive(Resource, ExtractResource, Clone, Debug)]
#[extract_app(RenderApp)]
pub struct AtmosphereLuts {
    pub transmittance: Handle<Image>,
    pub scattering: Handle<Image>,
}

pub struct BakedLuts {
    pub transmittance: TransmittanceLut,
    pub scattering: ScatteringLut,
}

pub fn bake() -> BakedLuts {
    note_build();
    let transmittance = bake_transmittance(&ATMOSPHERE, TRANSMITTANCE_WIDTH, TRANSMITTANCE_HEIGHT, TRANSMITTANCE_STEPS);
    let scattering = bake_scattering(&ATMOSPHERE, SCATTERING_WIDTH, SCATTERING_HEIGHT, SCATTERING_LAYERS, SCATTERING_STEPS, &transmittance);
    BakedLuts { transmittance, scattering }
}

pub fn sampling_source(group: &str, transmittance_binding: u32, scattering_binding: u32, sampler_binding: u32) -> String {
    [ATMOSPHERE_CORE, ATMOSPHERE_LUT_SAMPLE]
        .concat()
        .replace("ATM_LUT_GROUP", group)
        .replace("ATM_LUT_BINDING_TRANSMITTANCE", &transmittance_binding.to_string())
        .replace("ATM_LUT_BINDING_SCATTERING", &scattering_binding.to_string())
        .replace("ATM_LUT_BINDING_SAMPLER", &sampler_binding.to_string())
}

fn half_bits(value: f32) -> u16 {
    let bits = value.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exponent = ((bits >> SINGLE_MANTISSA_BITS) & 0xff) as i32 - 127 + HALF_EXPONENT_BIAS;
    let mantissa = bits & 0x7f_ffff;
    if exponent >= 31 {
        return sign | HALF_INFINITY;
    }
    let (kept, dropped, dropped_bits) = if exponent > 0 {
        (((exponent as u32) << HALF_MANTISSA_BITS) | (mantissa >> 13), mantissa & 0x1fff, 13)
    } else if exponent >= -10 {
        let shift = (14 - exponent) as u32;
        let full = mantissa | 0x80_0000;
        (full >> shift, full & ((1 << shift) - 1), shift)
    } else {
        return sign;
    };
    let half = 1u32 << (dropped_bits - 1);
    let round_up = dropped > half || (dropped == half && kept & 1 == 1);
    sign | (kept + u32::from(round_up)) as u16
}

fn half_bytes(values: impl Iterator<Item = f32>) -> Vec<u8> {
    values.flat_map(|value| half_bits(value).to_le_bytes()).collect()
}

fn with_alpha_one(rgb: &[f32]) -> impl Iterator<Item = f32> + '_ {
    rgb.chunks_exact(3).flat_map(|texel| [texel[0], texel[1], texel[2], 1.0])
}

fn half_image(width: usize, height: usize, layers: usize, texels: impl Iterator<Item = f32>) -> Image {
    let size = Extent3d { width: width as u32, height: height as u32, depth_or_array_layers: layers as u32 };
    let mut image = Image::new(size, TextureDimension::D2, half_bytes(texels), TextureFormat::Rgba16Float, RenderAssetUsages::RENDER_WORLD);
    image.sampler = ImageSampler::linear();
    image
}

impl BakedLuts {
    pub fn images(&self) -> (Image, Image) {
        let (t, s) = (&self.transmittance, &self.scattering);
        let transmittance = half_image(t.width, t.height, 1, with_alpha_one(&t.data));
        let mut scattering = half_image(s.width, s.height, s.layers, s.data.iter().copied());
        scattering.texture_view_descriptor = Some(TextureViewDescriptor { dimension: Some(TextureViewDimension::D2Array), ..default() });
        (transmittance, scattering)
    }
}

#[derive(Resource)]
struct LutBake(Task<(Image, Image)>);

fn start_bake(mut commands: Commands) {
    commands.insert_resource(LutBake(AsyncComputeTaskPool::get().spawn(async { bake().images() })));
}

fn finish_bake(mut commands: Commands, mut bake: ResMut<LutBake>, mut images: ResMut<Assets<Image>>) {
    let Some((transmittance, scattering)) = check_ready(&mut bake.0) else { return };
    commands.remove_resource::<LutBake>();
    commands.insert_resource(AtmosphereLuts { transmittance: images.add(transmittance), scattering: images.add(scattering) });
}

pub fn luts_ready(luts: Option<Res<AtmosphereLuts>>) -> bool {
    luts.is_some()
}

pub struct AtmosphereLutPlugin;

impl Plugin for AtmosphereLutPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, start_bake).add_systems(Update, finish_bake.run_if(resource_exists::<LutBake>));
    }
}

#[cfg(test)]
pub(crate) fn half_value(bits: u16) -> f32 {
    let sign = if bits & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exponent = i32::from((bits >> 10) & 0x1f);
    let mantissa = f32::from(bits & 0x3ff);
    match exponent {
        0 => sign * mantissa * 2f32.powi(-24),
        31 => sign * f32::INFINITY,
        _ => sign * (1.0 + mantissa / 1024.0) * 2f32.powi(exponent - 15),
    }
}

#[cfg(test)]
fn through_half(data: &[f32]) -> Vec<f32> {
    data.iter().map(|&value| half_value(half_bits(value))).collect()
}

#[cfg(test)]
pub(crate) fn baked_through_half() -> &'static BakedLuts {
    static BAKED: std::sync::OnceLock<BakedLuts> = std::sync::OnceLock::new();
    BAKED.get_or_init(|| {
        let exact = baked();
        let (t, s) = (&exact.transmittance, &exact.scattering);
        BakedLuts {
            transmittance: TransmittanceLut { width: t.width, height: t.height, data: through_half(&t.data) },
            scattering: ScatteringLut { width: s.width, height: s.height, layers: s.layers, data: through_half(&s.data) },
        }
    })
}

#[cfg(test)]
pub(crate) fn baked() -> &'static BakedLuts {
    static BAKED: std::sync::OnceLock<BakedLuts> = std::sync::OnceLock::new();
    BAKED.get_or_init(bake)
}

#[cfg(test)]
mod tests;
