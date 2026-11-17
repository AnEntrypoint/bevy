use crate::basisu::TextureTarget;
use crate::glb;
use serde_json::{Value, json};
use std::path::Path;

const KTX2_SUPERCOMPRESSION_OFFSET: usize = 44;
const KTX2_LEVEL_INDEX_OFFSET: usize = 80;
const SUPERCOMPRESSION_ZSTD: u32 = 2;
const VK_FORMAT_R8G8B8A8_UNORM: u32 = 37;
const VK_FORMAT_BC7_UNORM: u32 = 145;
const MIN_IMAGE_PSNR: f64 = 35.0;
const MIN_MEDIAN_PSNR: f64 = 40.0;

struct Level<'a> {
    width: usize,
    height: usize,
    bytes: &'a [u8],
}

#[derive(Default)]
struct ErrorSum {
    squared: f64,
    samples: u64,
    max: u8,
}

impl ErrorSum {
    fn add(&mut self, a: u8, b: u8) {
        let delta = a.abs_diff(b);
        self.squared += f64::from(delta) * f64::from(delta);
        self.samples += 1;
        self.max = self.max.max(delta);
    }

    fn psnr(&self) -> Option<f64> {
        if self.samples == 0 || self.squared == 0.0 {
            return None;
        }
        let mse = self.squared / self.samples as f64;
        Some(10.0 * (255.0 * 255.0 / mse).log10())
    }
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    bytes.get(offset..offset + 4).map(|slice| u32::from_le_bytes(slice.try_into().unwrap()))
}

fn read_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    bytes.get(offset..offset + 8).map(|slice| u64::from_le_bytes(slice.try_into().unwrap()))
}

fn levels(ktx2: &[u8]) -> Option<Vec<Level<'_>>> {
    let width = read_u32(ktx2, 20)? as usize;
    let height = read_u32(ktx2, 24)? as usize;
    let count = read_u32(ktx2, 40)?.max(1) as usize;
    (0..count)
        .map(|level| {
            let entry = KTX2_LEVEL_INDEX_OFFSET + level * 24;
            let offset = read_u64(ktx2, entry)? as usize;
            let length = read_u64(ktx2, entry + 8)? as usize;
            Some(Level {
                width: (width >> level).max(1),
                height: (height >> level).max(1),
                bytes: ktx2.get(offset..offset + length)?,
            })
        })
        .collect()
}

fn compare_level(rgba: &Level, bc7: &Level, sum: &mut ErrorSum) -> Result<(), String> {
    let blocks_x = rgba.width.div_ceil(4);
    let blocks_y = rgba.height.div_ceil(4);
    if rgba.bytes.len() < rgba.width * rgba.height * 4 || bc7.bytes.len() < blocks_x * blocks_y * 16 {
        return Err(format!("level {}x{} is shorter than its size", rgba.width, rgba.height));
    }
    let mut block = [0u8; 64];
    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let start = (by * blocks_x + bx) * 16;
            bcdec_rs::bc7(&bc7.bytes[start..start + 16], &mut block, 16);
            for y in 0..4 {
                for x in 0..4 {
                    let (px, py) = (bx * 4 + x, by * 4 + y);
                    if px >= rgba.width || py >= rgba.height {
                        continue;
                    }
                    let source = (py * rgba.width + px) * 4;
                    let decoded = (y * 4 + x) * 4;
                    for channel in 0..4 {
                        sum.add(rgba.bytes[source + channel], block[decoded + channel]);
                    }
                }
            }
        }
    }
    Ok(())
}

fn source_is_zstd(bytes: &[u8]) -> bool {
    glb::ktx2_images(bytes)
        .map(|images| images.iter().any(|image| read_u32(image, KTX2_SUPERCOMPRESSION_OFFSET) == Some(SUPERCOMPRESSION_ZSTD)))
        .unwrap_or(false)
}

fn measure_model(name: &str, bytes: &[u8]) -> Result<Vec<Value>, String> {
    let rgba = glb::ktx2_images(&glb::decode_with(bytes, TextureTarget::Rgba8)?.bytes)?;
    let bc7 = glb::ktx2_images(&glb::decode_with(bytes, TextureTarget::Bc7)?.bytes)?;
    if rgba.len() != bc7.len() {
        return Err(format!("{name}: {} RGBA8 images against {} BC7 images", rgba.len(), bc7.len()));
    }
    let mut rows = Vec::new();
    for (index, (a, b)) in rgba.iter().zip(&bc7).enumerate() {
        if read_u32(a, 12) != Some(VK_FORMAT_R8G8B8A8_UNORM) || read_u32(b, 12) != Some(VK_FORMAT_BC7_UNORM) {
            continue;
        }
        let a_levels = levels(a).ok_or_else(|| format!("{name} image {index}: bad RGBA8 KTX2"))?;
        let b_levels = levels(b).ok_or_else(|| format!("{name} image {index}: bad BC7 KTX2"))?;
        if a_levels.len() != b_levels.len() {
            return Err(format!("{name} image {index}: level counts differ"));
        }
        let mut first = ErrorSum::default();
        let mut all = ErrorSum::default();
        for (level, (x, y)) in a_levels.iter().zip(&b_levels).enumerate() {
            if level == 0 {
                compare_level(x, y, &mut first)?;
            }
            compare_level(x, y, &mut all)?;
        }
        rows.push(json!({
            "model": name,
            "image": index,
            "width": a_levels[0].width,
            "height": a_levels[0].height,
            "levels": a_levels.len(),
            "psnrLevel0": first.psnr(),
            "psnrAllLevels": all.psnr(),
            "maxError": all.max,
        }));
    }
    Ok(rows)
}

pub fn run(directory: &str) -> bool {
    let mut paths = std::fs::read_dir(directory)
        .map(|entries| entries.filter_map(Result::ok).map(|entry| entry.path()).collect::<Vec<_>>())
        .unwrap_or_default();
    paths.retain(|path| path.extension().and_then(|ext| ext.to_str()) == Some("glb"));
    paths.sort();
    let mut models = Vec::new();
    let mut rows = Vec::new();
    let mut errors = Vec::new();
    for path in &paths {
        let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default().to_string();
        let Ok(bytes) = std::fs::read(path) else {
            errors.push(format!("{name}: unreadable"));
            continue;
        };
        if !source_is_zstd(&bytes) {
            continue;
        }
        models.push(name.clone());
        match measure_model(&name, &bytes) {
            Ok(model_rows) => rows.extend(model_rows),
            Err(error) => errors.push(error),
        }
    }
    let mut level0 = rows
        .iter()
        .map(|row| row["psnrLevel0"].as_f64().unwrap_or(f64::INFINITY))
        .collect::<Vec<_>>();
    level0.sort_by(f64::total_cmp);
    let min = level0.first().copied();
    let median = level0.get(level0.len() / 2).copied();
    let max_error = rows.iter().filter_map(|row| row["maxError"].as_u64()).max();
    let ok = errors.is_empty()
        && !rows.is_empty()
        && min.is_some_and(|value| value >= MIN_IMAGE_PSNR)
        && median.is_some_and(|value| value >= MIN_MEDIAN_PSNR);
    let finite = |value: Option<f64>| value.filter(|value| value.is_finite());
    let report = json!({
        "kind": "texture-quality",
        "ok": ok,
        "directory": Path::new(directory).display().to_string(),
        "criteria": { "minImagePsnr": MIN_IMAGE_PSNR, "minMedianPsnr": MIN_MEDIAN_PSNR },
        "models": models,
        "images": rows.len(),
        "psnrLevel0Min": finite(min),
        "psnrLevel0Median": finite(median),
        "maxError": max_error,
        "errors": errors,
        "rows": rows,
    });
    println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
    ok
}
