use std::path::Path;

use bevy::asset::RenderAssetUsages;
use bevy::image::{
    CompressedImageFormats, Image, ImageAddressMode, ImageFilterMode, ImageSampler,
    ImageSamplerDescriptor, ImageType,
};
use bevy::render::render_resource::{
    Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
};

use crate::js_math::round_half_up;
use crate::terrain_config::Refusal;

pub const SIZE: usize = 1024;
pub const LAYER_COUNT: usize = 4;
pub const MIP_LEVELS: u32 = 11;
pub const ANISOTROPY: u16 = 8;
const LAYER_BYTES: usize = SIZE * SIZE * 4;
const GRID: usize = 32;
const CELL: usize = SIZE / GRID;
const GAMMA: f64 = 2.2;
const GAIN_EXPONENT: f64 = 0.8;
const GAIN_MIN: f64 = 0.4;
const GAIN_MAX: f64 = 2.5;
const BLUR_FLOOR: f64 = 1e-4;
const CONTRAST_STRETCH: f64 = 1.35;
const ROCK_LAYER: usize = 1;
const LUMA: [f64; 3] = [0.2126, 0.7152, 0.0722];
const MATERIALS: [&str; LAYER_COUNT] = ["grass", "rock", "sand", "snow"];
const NORMAL_FILES: [&str; LAYER_COUNT] = [
    "grass-normals.jpg",
    "ground-normals.jpg",
    "sand-normals.jpg",
    "snow-normals.jpg",
];

pub struct Layers {
    pub albedo: Vec<u8>,
    pub normal: Vec<u8>,
    pub rock_mean: [f32; 3],
    pub mean_l: [f32; 4],
}

fn refuse(path: &Path, reason: impl std::fmt::Display) -> Refusal {
    Refusal {
        name: "planet_layers",
        message: format!("{}: {reason}", path.display()),
    }
}

fn linear_table() -> [f32; 256] {
    std::array::from_fn(|v| (v as f64 / 255.0).powf(GAMMA) as f32)
}

fn delin8(linear: f64) -> u8 {
    round_half_up(linear.powf(1.0 / GAMMA) * 255.0).clamp(0.0, 255.0) as u8
}

fn decode_rgba(path: &Path) -> Result<Vec<u8>, Refusal> {
    let bytes = std::fs::read(path).map_err(|e| refuse(path, e))?;
    let image = Image::from_buffer(
        &bytes,
        ImageType::Extension("jpg"),
        CompressedImageFormats::NONE,
        false,
        ImageSampler::Default,
        RenderAssetUsages::default(),
    )
    .map_err(|e| refuse(path, e))?;
    if image.width() as usize != SIZE || image.height() as usize != SIZE {
        return Err(refuse(path, format!("{}x{}, expected {SIZE}x{SIZE}", image.width(), image.height())));
    }
    if image.texture_descriptor.format != TextureFormat::Rgba8Unorm {
        return Err(refuse(path, format!("decoded as {:?}", image.texture_descriptor.format)));
    }
    image.data.ok_or_else(|| refuse(path, "no pixel data"))
}

fn grid_taps() -> Vec<(usize, usize, f64)> {
    (0..SIZE)
        .map(|p| {
            let g = p as f64 / CELL as f64 - 0.5;
            let low = (g.floor() as i64 + GRID as i64) as usize % GRID;
            (low, (low + 1) % GRID, g - g.floor())
        })
        .collect()
}

fn deshade(colour: &mut [u8], linear: &[f32; 256]) {
    let mut grid = vec![0f32; GRID * GRID * 3];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let i = (y * SIZE + x) * 4;
            let g = ((y / CELL) * GRID + x / CELL) * 3;
            for ch in 0..3 {
                grid[g + ch] += linear[colour[i + ch] as usize];
            }
        }
    }
    let texels_per_cell = (CELL * CELL) as f64;
    let mut mean = [0f64; 3];
    for g in 0..GRID * GRID {
        for ch in 0..3 {
            let cell_mean = (grid[g * 3 + ch] as f64 / texels_per_cell) as f32;
            grid[g * 3 + ch] = cell_mean;
            mean[ch] += cell_mean as f64;
        }
    }
    mean.iter_mut().for_each(|m| *m /= (GRID * GRID) as f64);
    let taps = grid_taps();
    for y in 0..SIZE {
        let (y0, y1, fy) = taps[y];
        for x in 0..SIZE {
            let (x0, x1, fx) = taps[x];
            let i = (y * SIZE + x) * 4;
            for ch in 0..3 {
                let at = |gy: usize, gx: usize| grid[(gy * GRID + gx) * 3 + ch] as f64;
                let blur = (at(y0, x0) * (1.0 - fx) + at(y0, x1) * fx) * (1.0 - fy)
                    + (at(y1, x0) * (1.0 - fx) + at(y1, x1) * fx) * fy;
                let gain = (mean[ch] / blur.max(BLUR_FLOOR))
                    .powf(GAIN_EXPONENT)
                    .clamp(GAIN_MIN, GAIN_MAX);
                let shaded = linear[colour[i + ch] as usize] as f64 * gain;
                let stretched = (mean[ch] + (shaded - mean[ch]) * CONTRAST_STRETCH).max(0.0);
                colour[i + ch] = delin8(stretched);
            }
        }
    }
}

fn rock_mean(albedo: &[u8]) -> [f32; 3] {
    let rock = &albedo[ROCK_LAYER * LAYER_BYTES..(ROCK_LAYER + 1) * LAYER_BYTES];
    let texels = (SIZE * SIZE) as f64;
    std::array::from_fn(|ch| {
        let sum: u64 = rock.chunks_exact(4).map(|px| px[ch] as u64).sum();
        (sum as f64 / texels / 255.0).powf(GAMMA) as f32
    })
}

fn mean_l(albedo: &[u8], linear: &[f32; 256]) -> [f32; 4] {
    let texels = (SIZE * SIZE) as f64;
    std::array::from_fn(|layer| {
        let sum: f64 = albedo[layer * LAYER_BYTES..(layer + 1) * LAYER_BYTES]
            .chunks_exact(4)
            .map(|px| (0..3).map(|ch| LUMA[ch] * linear[px[ch] as usize] as f64).sum::<f64>())
            .sum();
        (sum / texels) as f32
    })
}

pub fn decode_layers(dir: &Path) -> Result<Layers, Refusal> {
    crate::planet_lod::guard::note_build();
    let linear = linear_table();
    let mut albedo = vec![0u8; LAYER_BYTES * LAYER_COUNT];
    let mut normal = vec![0u8; LAYER_BYTES * LAYER_COUNT];
    for layer in 0..LAYER_COUNT {
        let mut colour = decode_rgba(&dir.join(format!("{}-color.jpg", MATERIALS[layer])))?;
        let displacement = decode_rgba(&dir.join(format!("{}-displacement.jpg", MATERIALS[layer])))?;
        let normals = decode_rgba(&dir.join(NORMAL_FILES[layer]))?;
        deshade(&mut colour, &linear);
        let base = layer * LAYER_BYTES;
        for t in 0..SIZE * SIZE {
            let (o, i) = (base + t * 4, t * 4);
            albedo[o..o + 3].copy_from_slice(&colour[i..i + 3]);
            albedo[o + 3] = displacement[i];
            normal[o..o + 2].copy_from_slice(&normals[i..i + 2]);
            normal[o + 2] = displacement[i];
            normal[o + 3] = 255;
        }
    }
    Ok(Layers {
        rock_mean: rock_mean(&albedo),
        mean_l: mean_l(&albedo, &linear),
        albedo,
        normal,
    })
}

fn srgb_to_linear(v: u8) -> f32 {
    let c = v as f32 / 255.0;
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

fn linear_to_srgb(l: f32) -> u8 {
    let c = if l <= 0.0031308 { l * 12.92 } else { 1.055 * l.powf(1.0 / 2.4) - 0.055 };
    (c * 255.0).round().clamp(0.0, 255.0) as u8
}

fn next_mip(src: &[u8], size: usize, average_in_linear_light: bool, table: &[f32; 256]) -> Vec<u8> {
    let half = size / 2;
    let mut out = Vec::with_capacity(half * half * 4);
    for y in 0..half {
        for x in 0..half {
            let taps = [(2 * y, 2 * x), (2 * y, 2 * x + 1), (2 * y + 1, 2 * x), (2 * y + 1, 2 * x + 1)];
            for ch in 0..4 {
                let at = |&(ty, tx): &(usize, usize)| src[(ty * size + tx) * 4 + ch];
                out.push(if average_in_linear_light && ch < 3 {
                    linear_to_srgb(taps.iter().map(|t| table[at(t) as usize]).sum::<f32>() / 4.0)
                } else {
                    ((taps.iter().map(|t| at(t) as u32).sum::<u32>() + 2) / 4) as u8
                });
            }
        }
    }
    out
}

fn array_image(packed: &[u8], format: TextureFormat, average_in_linear_light: bool) -> Image {
    let table: [f32; 256] = std::array::from_fn(|v| srgb_to_linear(v as u8));
    let mut data = Vec::new();
    for layer in packed.chunks_exact(LAYER_BYTES) {
        let mut level = layer.to_vec();
        data.extend_from_slice(&level);
        for size in (1..MIP_LEVELS).map(|l| SIZE >> (l - 1)) {
            level = next_mip(&level, size, average_in_linear_light, &table);
            data.extend_from_slice(&level);
        }
    }
    let mut image = Image::new_uninit(
        Extent3d { width: SIZE as u32, height: SIZE as u32, depth_or_array_layers: LAYER_COUNT as u32 },
        TextureDimension::D2,
        format,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.data = Some(data);
    image.texture_descriptor.mip_level_count = MIP_LEVELS;
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: ANISOTROPY,
        ..Default::default()
    });
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::D2Array),
        ..Default::default()
    });
    image
}

pub fn build_images(layers: &Layers) -> (Image, Image) {
    (
        array_image(&layers.albedo, TextureFormat::Rgba8UnormSrgb, true),
        array_image(&layers.normal, TextureFormat::Rgba8Unorm, false),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{close, parse};
    use serde_json::Value;

    const FIXTURE: &str = include_str!("../../testdata/parity/spoint-planet-layers.json");
    const DECODER_TOLERANCE: i32 = 3;
    const NORMAL_TOLERANCE: i32 = 3;
    const DISPLACEMENT_TOLERANCE: i32 = 1;

    fn textures() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/planet/textures")
    }

    fn albedo_tolerance(channel: usize, reference: u8) -> i32 {
        match (channel, reference) {
            (3, _) => DISPLACEMENT_TOLERANCE,
            (_, 0..=31) => 24,
            (_, 32..=63) => 8,
            _ => 6,
        }
    }

    fn normal_tolerance(channel: usize, _reference: u8) -> i32 {
        if channel == 3 { 0 } else { NORMAL_TOLERANCE }
    }

    fn hex(text: &Value) -> Vec<u8> {
        let text = text.as_str().unwrap();
        (0..text.len() / 2).map(|i| u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).unwrap()).collect()
    }

    fn sample_points(stride_key: &str, fixture: &Value) -> Vec<usize> {
        let stride = fixture[stride_key].as_u64().unwrap() as usize;
        let offset = fixture["offset"].as_array().unwrap();
        let (ox, oy) = (offset[0].as_u64().unwrap() as usize, offset[1].as_u64().unwrap() as usize);
        (oy..SIZE).step_by(stride).flat_map(|y| (ox..SIZE).step_by(stride).map(move |x| y * SIZE + x)).collect()
    }

    fn assert_within(name: &str, packed: &[u8], layer: usize, points: &[usize], want: &[u8], tolerance: fn(usize, u8) -> i32) {
        assert_eq!(points.len() * 4, want.len(), "{name}: sample count");
        for (k, texel) in points.iter().enumerate() {
            for channel in 0..4 {
                let got = packed[layer * LAYER_BYTES + texel * 4 + channel] as i32;
                let reference = want[k * 4 + channel];
                let allowed = tolerance(channel, reference);
                assert!((got - reference as i32).abs() <= allowed, "{name} texel {texel} channel {channel}: {got} against spoint {reference}, tolerance {allowed}");
            }
        }
    }

    #[test]
    fn the_jpeg_decoder_stays_within_three_codes_of_the_reference_decoder() {
        let fixture = parse(FIXTURE);
        let points = sample_points("rawStride", &fixture);
        for want in fixture["rawDecodes"].as_array().unwrap() {
            let file = want["file"].as_str().unwrap();
            let got = decode_rgba(&textures().join(file)).unwrap();
            let reference = hex(&want["sample"]);
            assert_eq!(points.len() * 4, reference.len(), "{file}: sample count");
            for (k, texel) in points.iter().enumerate() {
                for channel in 0..4 {
                    let difference = (got[texel * 4 + channel] as i32 - reference[k * 4 + channel] as i32).abs();
                    let allowed = if channel == 3 { 0 } else { DECODER_TOLERANCE };
                    assert!(difference <= allowed, "{file} texel {texel} channel {channel}: {difference} codes apart");
                }
            }
        }
    }

    #[test]
    fn packed_arrays_match_the_reference_decode() {
        let fixture = parse(FIXTURE);
        let layers = decode_layers(&textures()).unwrap();
        let points = sample_points("stride", &fixture);
        for (index, want) in fixture["layers"].as_array().unwrap().iter().enumerate() {
            let name = want["name"].as_str().unwrap();
            assert_eq!(name, MATERIALS[index]);
            assert_within(&format!("{name} albedo"), &layers.albedo, index, &points, &hex(&want["albedoSample"]), albedo_tolerance);
            assert_within(&format!("{name} normal"), &layers.normal, index, &points, &hex(&want["normalSample"]), normal_tolerance);
        }
    }

    #[test]
    fn means_match_the_reference() {
        let fixture = parse(FIXTURE);
        let layers = decode_layers(&textures()).unwrap();
        for ch in 0..3 {
            close(layers.rock_mean[ch] as f64, fixture["rockMean"][ch].as_f64().unwrap(), 1e-3, &format!("rock_mean {ch}"));
        }
        for layer in 0..LAYER_COUNT {
            close(layers.mean_l[layer] as f64, fixture["meanL"][layer].as_f64().unwrap(), 1e-3, &format!("mean_l {layer}"));
        }
    }

    #[test]
    fn a_missing_texture_is_refused() {
        let refusal = decode_layers(Path::new("assets/planet/absent")).err().unwrap();
        assert_eq!(refusal.name, "planet_layers");
    }

    #[test]
    fn decoding_is_counted_as_a_build() {
        use crate::planet_lod::guard::{builds_on_main_thread, mark_main_thread};
        mark_main_thread();
        let before = builds_on_main_thread();
        decode_layers(Path::new("assets/planet/absent")).err();
        assert_eq!(builds_on_main_thread(), before + 1);
    }

    fn marked_layers() -> Layers {
        let mut albedo = vec![0u8; LAYER_BYTES * LAYER_COUNT];
        albedo[..4].copy_from_slice(&[255; 4]);
        albedo[LAYER_BYTES..LAYER_BYTES + 4].copy_from_slice(&[10, 20, 30, 40]);
        Layers { normal: albedo.clone(), albedo, rock_mean: [0.0; 3], mean_l: [0.0; 4] }
    }

    fn level_bytes() -> usize {
        (0..MIP_LEVELS).map(|l| (SIZE >> l) * (SIZE >> l) * 4).sum()
    }

    #[test]
    fn mip_chains_are_laid_out_layer_major() {
        let (albedo, normal) = build_images(&marked_layers());
        for image in [&albedo, &normal] {
            let data = image.data.as_ref().unwrap();
            assert_eq!(data.len(), level_bytes() * LAYER_COUNT);
            assert_eq!(&data[level_bytes()..level_bytes() + 4], &[10, 20, 30, 40]);
        }
    }

    #[test]
    fn albedo_mips_average_in_linear_light_and_normal_mips_in_codes() {
        let (albedo, normal) = build_images(&marked_layers());
        assert_eq!(&albedo.data.as_ref().unwrap()[LAYER_BYTES..LAYER_BYTES + 4], &[137, 137, 137, 64]);
        assert_eq!(&normal.data.as_ref().unwrap()[LAYER_BYTES..LAYER_BYTES + 4], &[64, 64, 64, 64]);
    }

    #[test]
    fn images_are_repeat_linear_anisotropic_array_textures() {
        let (albedo, normal) = build_images(&marked_layers());
        assert_eq!(albedo.texture_descriptor.format, TextureFormat::Rgba8UnormSrgb);
        assert_eq!(normal.texture_descriptor.format, TextureFormat::Rgba8Unorm);
        for image in [&albedo, &normal] {
            assert_eq!(image.texture_descriptor.mip_level_count, 11);
            assert_eq!(image.texture_descriptor.size.depth_or_array_layers, 4);
            assert_eq!(image.texture_view_descriptor.as_ref().unwrap().dimension, Some(TextureViewDimension::D2Array));
            let ImageSampler::Descriptor(sampler) = &image.sampler else { panic!("sampler is not a descriptor") };
            assert_eq!(sampler.address_mode_u, ImageAddressMode::Repeat);
            assert_eq!(sampler.address_mode_v, ImageAddressMode::Repeat);
            assert_eq!(sampler.mag_filter, ImageFilterMode::Linear);
            assert_eq!(sampler.min_filter, ImageFilterMode::Linear);
            assert_eq!(sampler.mipmap_filter, ImageFilterMode::Linear);
            assert_eq!(sampler.anisotropy_clamp, 8);
        }
    }
}
