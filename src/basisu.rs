use bevy::image::CompressedImageFormats;
use serde_json::{Map, Value, json};
use std::collections::HashMap;

const EXTENSION: &str = "KHR_texture_basisu";

pub fn promote(json: &mut Value) -> usize {
    let mut promoted = 0;
    if let Some(textures) = json.get_mut("textures").and_then(Value::as_array_mut) {
        for texture in textures.iter_mut() {
            if promote_texture(texture) {
                promoted += 1;
            }
        }
    }
    if !any_texture_uses_extension(json) {
        retire_extension_name(json, "extensionsUsed");
        retire_extension_name(json, "extensionsRequired");
    }
    promoted
}

fn promote_texture(texture: &mut Value) -> bool {
    let Some(object) = texture.as_object_mut() else {
        return false;
    };
    let Some(source) = basisu_source(object) else {
        return false;
    };
    let Some(extensions) = object.get_mut("extensions").and_then(Value::as_object_mut) else {
        return false;
    };
    extensions.remove(EXTENSION);
    let extensions_empty = extensions.is_empty();
    if extensions_empty {
        object.remove("extensions");
    }
    object.insert("source".to_owned(), Value::from(source));
    true
}

fn basisu_source(texture: &Map<String, Value>) -> Option<u64> {
    texture
        .get("extensions")?
        .get(EXTENSION)?
        .get("source")?
        .as_u64()
}

fn any_texture_uses_extension(json: &Value) -> bool {
    json.get("textures")
        .and_then(Value::as_array)
        .is_some_and(|textures| {
            textures.iter().any(|texture| {
                texture
                    .get("extensions")
                    .and_then(|extensions| extensions.get(EXTENSION))
                    .is_some()
            })
        })
}

fn retire_extension_name(json: &mut Value, list: &str) {
    if let Some(names) = json.get_mut(list).and_then(Value::as_array_mut) {
        names.retain(|name| name.as_str() != Some(EXTENSION));
    }
}

const KTX2_IDENTIFIER: [u8; 12] = [0xAB, b'K', b'T', b'X', b' ', b'2', b'0', 0xBB, b'\r', b'\n', 0x1A, b'\n'];
const KTX2_HEADER_LEN: usize = 80;
const KTX2_LEVEL_ENTRY_LEN: usize = 24;
const KTX2_VK_FORMAT_OFFSET: usize = 12;
const KTX2_LEVEL_COUNT_OFFSET: usize = 40;
const KTX2_SUPERCOMPRESSION_OFFSET: usize = 44;
const KTX2_SGD_LENGTH_OFFSET: usize = 72;
const VK_FORMAT_UNDEFINED: u32 = 0;
const SUPERCOMPRESSION_ZSTD: u32 = 2;
const VK_FORMAT_R8G8B8A8_UNORM: u32 = 37;
const VK_FORMAT_BC7_UNORM: u32 = 145;
const KHR_DF_MODEL_BC7: u8 = 134;
const VK_FORMAT_ETC2_R8G8B8A8_UNORM: u32 = 151;
const VK_FORMAT_ASTC_4X4_UNORM: u32 = 157;
const KHR_DF_MODEL_ETC2: u8 = 161;
const KHR_DF_MODEL_ASTC: u8 = 162;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureTarget {
    Rgba8,
    Bc7,
    Etc2,
    Astc4x4,
}

impl TextureTarget {
    pub fn for_gpu(formats: CompressedImageFormats) -> Self {
        if formats.contains(CompressedImageFormats::BC) {
            Self::Bc7
        } else if formats.contains(CompressedImageFormats::ASTC_LDR) {
            Self::Astc4x4
        } else if formats.contains(CompressedImageFormats::ETC2) {
            Self::Etc2
        } else {
            Self::Rgba8
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        [Self::Rgba8, Self::Bc7, Self::Etc2, Self::Astc4x4].into_iter().find(|target| target.name() == name)
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Rgba8 => "rgba8",
            Self::Bc7 => "bc7",
            Self::Etc2 => "etc2",
            Self::Astc4x4 => "astc4x4",
        }
    }

    pub fn cache_dir(self) -> &'static str {
        match self {
            Self::Rgba8 => "target/gltf-cache-rgba8",
            Self::Bc7 => "target/gltf-cache",
            Self::Etc2 => "target/gltf-cache-etc2",
            Self::Astc4x4 => "target/gltf-cache-astc",
        }
    }

    fn vk_format(self) -> u32 {
        match self {
            Self::Rgba8 => VK_FORMAT_R8G8B8A8_UNORM,
            Self::Bc7 => VK_FORMAT_BC7_UNORM,
            Self::Etc2 => VK_FORMAT_ETC2_R8G8B8A8_UNORM,
            Self::Astc4x4 => VK_FORMAT_ASTC_4X4_UNORM,
        }
    }

    fn level_bytes(self, width: usize, height: usize) -> usize {
        match self {
            Self::Rgba8 => width * height * 4,
            Self::Bc7 | Self::Etc2 | Self::Astc4x4 => width.div_ceil(4) * height.div_ceil(4) * 16,
        }
    }

    fn transcoder_format(self) -> u32 {
        match self {
            Self::Rgba8 => 0,
            Self::Bc7 => 1,
            Self::Etc2 => 2,
            Self::Astc4x4 => 3,
        }
    }

    fn dfd(self) -> Vec<u8> {
        match self {
            Self::Rgba8 => rgba8_dfd(),
            Self::Bc7 => wrap_dfd(block_format_dfd(KHR_DF_MODEL_BC7)),
            Self::Etc2 => wrap_dfd(block_format_dfd(KHR_DF_MODEL_ETC2)),
            Self::Astc4x4 => wrap_dfd(block_format_dfd(KHR_DF_MODEL_ASTC)),
        }
    }
}

unsafe extern "C" {
    fn bevy_basislz_info(
        data: *const u8,
        size: u32,
        width: *mut u32,
        height: *mut u32,
        levels: *mut u32,
        layers: *mut u32,
        faces: *mut u32,
    ) -> i32;
    fn bevy_basislz_transcode_level(data: *const u8, size: u32, level: u32, out: *mut u8, capacity: u64, format: u32) -> i64;
}

pub fn transcode_basis_images(json: &mut Value, bin: &mut Vec<u8>, target: TextureTarget) -> Result<usize, String> {
    let targets: Vec<(usize, usize)> = json
        .get("images")
        .and_then(Value::as_array)
        .map(|images| {
            images
                .iter()
                .enumerate()
                .filter(|(_, image)| image.get("mimeType").and_then(Value::as_str) == Some("image/ktx2"))
                .filter_map(|(index, image)| Some((index, image.get("bufferView")?.as_u64()? as usize)))
                .collect()
        })
        .unwrap_or_default();
    let views = json.get("bufferViews").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut rewritten: HashMap<usize, usize> = HashMap::new();
    let mut count = 0;
    for (image_index, view_index) in targets {
        let view = views
            .get(view_index)
            .ok_or_else(|| format!("image {image_index} names missing buffer view {view_index}"))?;
        let offset = view.get("byteOffset").and_then(Value::as_u64).unwrap_or(0) as usize;
        let length = view
            .get("byteLength")
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("buffer view {view_index} has no byteLength"))? as usize;
        let source = bin
            .get(offset..offset + length)
            .ok_or_else(|| format!("buffer view {view_index} runs past the BIN chunk"))?;
        if !is_basis_ktx2(source) {
            continue;
        }
        let new_view = match rewritten.get(&view_index) {
            Some(index) => *index,
            None => {
                let rgba = transcode_basis(source, target).map_err(|error| format!("image {image_index}: {error}"))?;
                let buffer = view.get("buffer").and_then(Value::as_u64).unwrap_or(0);
                let aligned = bin.len().next_multiple_of(4);
                bin.resize(aligned, 0);
                let new_offset = bin.len();
                bin.extend_from_slice(&rgba);
                let views_mut = json
                    .get_mut("bufferViews")
                    .and_then(Value::as_array_mut)
                    .ok_or("GLB has no bufferViews array")?;
                views_mut.push(json!({"buffer": buffer, "byteOffset": new_offset, "byteLength": rgba.len()}));
                let index = views_mut.len() - 1;
                rewritten.insert(view_index, index);
                index
            }
        };
        let image = json
            .get_mut("images")
            .and_then(Value::as_array_mut)
            .and_then(|images| images.get_mut(image_index))
            .ok_or("image index is out of range")?;
        image["bufferView"] = Value::from(new_view);
        count += 1;
    }
    Ok(count)
}

pub fn is_basis_ktx2(bytes: &[u8]) -> bool {
    bytes.starts_with(&KTX2_IDENTIFIER) && read_u32(bytes, KTX2_VK_FORMAT_OFFSET) == Some(VK_FORMAT_UNDEFINED)
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    bytes
        .get(offset..offset + 4)
        .and_then(|slice| <[u8; 4]>::try_from(slice).ok())
        .map(u32::from_le_bytes)
}

pub fn transcode_basis(input: &[u8], target: TextureTarget) -> Result<Vec<u8>, String> {
    let inflated = inflate_zstd_levels(input)?;
    let bytes = inflated.as_deref().unwrap_or(input);
    let size = u32::try_from(bytes.len()).map_err(|_| "KTX2 texture is larger than 4 GiB".to_string())?;
    let (mut width, mut height, mut levels, mut layers, mut faces) = (0u32, 0u32, 0u32, 0u32, 0u32);
    let status = unsafe {
        bevy_basislz_info(bytes.as_ptr(), size, &mut width, &mut height, &mut levels, &mut layers, &mut faces)
    };
    if status != 0 {
        return Err(format!("basis transcoder rejected the KTX2 header (status {status})"));
    }
    if layers > 1 || faces > 1 {
        return Err(format!("BasisLZ KTX2 with {layers} layers and {faces} faces is not supported"));
    }
    let mut pixels = Vec::with_capacity(levels.max(1) as usize);
    for level in 0..levels.max(1) {
        let level_width = width.checked_shr(level).unwrap_or(0).max(1) as usize;
        let level_height = height.checked_shr(level).unwrap_or(0).max(1) as usize;
        let capacity = target.level_bytes(level_width, level_height);
        let mut buffer = vec![0u8; capacity];
        let written = unsafe {
            bevy_basislz_transcode_level(bytes.as_ptr(), size, level, buffer.as_mut_ptr(), capacity as u64, target.transcoder_format())
        };
        if written != capacity as i64 {
            return Err(format!("basis transcoder failed on level {level} (status {written})"));
        }
        pixels.push(buffer);
    }
    Ok(ktx2_bytes(target, width, height, &pixels))
}

fn inflate_zstd_levels(bytes: &[u8]) -> Result<Option<Vec<u8>>, String> {
    if read_u32(bytes, KTX2_SUPERCOMPRESSION_OFFSET) != Some(SUPERCOMPRESSION_ZSTD) {
        return Ok(None);
    }
    if read_u64(bytes, KTX2_SGD_LENGTH_OFFSET) != Some(0) {
        return Err(String::from("Zstandard KTX2 with supercompression global data is not supported"));
    }
    let levels = read_u32(bytes, KTX2_LEVEL_COUNT_OFFSET).ok_or_else(|| String::from("KTX2 header is truncated"))?.max(1) as usize;
    let index_end = KTX2_HEADER_LEN + KTX2_LEVEL_ENTRY_LEN * levels;
    let mut entries = Vec::with_capacity(levels);
    for level in 0..levels {
        let at = KTX2_HEADER_LEN + KTX2_LEVEL_ENTRY_LEN * level;
        let truncated = || String::from("KTX2 level index is truncated");
        let offset = read_u64(bytes, at).ok_or_else(truncated)? as usize;
        let length = read_u64(bytes, at + 8).ok_or_else(truncated)? as usize;
        let uncompressed = read_u64(bytes, at + 16).ok_or_else(truncated)? as usize;
        entries.push((offset, length, uncompressed));
    }
    let first = entries.iter().map(|entry| entry.0).min().unwrap_or(index_end).max(index_end);
    let middle = bytes.get(index_end..first).ok_or_else(|| String::from("KTX2 level data starts inside the level index"))?;
    let mut plain = Vec::with_capacity(levels);
    for (level, &(offset, length, uncompressed)) in entries.iter().enumerate() {
        let source = bytes.get(offset..offset + length).ok_or_else(|| format!("Zstandard level {level} runs past the file"))?;
        let mut output = vec![0u8; uncompressed];
        let written = ruzstd::decoding::FrameDecoder::new()
            .decode_all(source, &mut output)
            .map_err(|error| format!("Zstandard level {level}: {error}"))?;
        if written != uncompressed {
            return Err(format!("Zstandard level {level} decoded to {written} bytes, expected {uncompressed}"));
        }
        plain.push(output);
    }
    let mut cursor = index_end + middle.len();
    let mut offsets = Vec::with_capacity(levels);
    for level in &plain {
        cursor = cursor.next_multiple_of(8);
        offsets.push(cursor);
        cursor += level.len();
    }
    let mut out = bytes[..KTX2_HEADER_LEN].to_vec();
    out[KTX2_SUPERCOMPRESSION_OFFSET..KTX2_SUPERCOMPRESSION_OFFSET + 4].copy_from_slice(&0u32.to_le_bytes());
    for (offset, data) in offsets.iter().zip(&plain) {
        out.extend_from_slice(&(*offset as u64).to_le_bytes());
        out.extend_from_slice(&(data.len() as u64).to_le_bytes());
        out.extend_from_slice(&(data.len() as u64).to_le_bytes());
    }
    out.extend_from_slice(middle);
    for (offset, data) in offsets.iter().zip(&plain) {
        out.resize(*offset, 0);
        out.extend_from_slice(data);
    }
    Ok(Some(out))
}

fn read_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    bytes
        .get(offset..offset + 8)
        .and_then(|slice| <[u8; 8]>::try_from(slice).ok())
        .map(u64::from_le_bytes)
}

pub fn ktx2_bytes(target: TextureTarget, width: u32, height: u32, levels: &[Vec<u8>]) -> Vec<u8> {
    let dfd = target.dfd();
    let dfd_offset = KTX2_HEADER_LEN + KTX2_LEVEL_ENTRY_LEN * levels.len();
    let mut offsets = vec![0usize; levels.len()];
    let mut cursor = dfd_offset + dfd.len();
    for level in (0..levels.len()).rev() {
        offsets[level] = cursor;
        cursor = (cursor + levels[level].len()).next_multiple_of(4);
    }
    let mut out = Vec::with_capacity(cursor);
    out.extend_from_slice(&KTX2_IDENTIFIER);
    for value in [target.vk_format(), 1, width, height, 0, 0, 1, levels.len() as u32, 0] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    for value in [dfd_offset as u32, dfd.len() as u32, 0, 0] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.extend_from_slice(&0u64.to_le_bytes());
    out.extend_from_slice(&0u64.to_le_bytes());
    for (level, bytes) in levels.iter().enumerate() {
        out.extend_from_slice(&(offsets[level] as u64).to_le_bytes());
        out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    }
    out.extend_from_slice(&dfd);
    for level in (0..levels.len()).rev() {
        out.resize(offsets[level], 0);
        out.extend_from_slice(&levels[level]);
    }
    out
}

fn rgba8_dfd() -> Vec<u8> {
    let mut block = Vec::with_capacity(88);
    block.extend_from_slice(&0u32.to_le_bytes());
    block.extend_from_slice(&2u16.to_le_bytes());
    block.extend_from_slice(&88u16.to_le_bytes());
    block.extend_from_slice(&[1, 1, 1, 0]);
    block.extend_from_slice(&[0, 0, 0, 0]);
    block.extend_from_slice(&[4, 0, 0, 0, 0, 0, 0, 0]);
    for (bit_offset, channel) in [(0u16, 0u8), (8, 1), (16, 2), (24, 15)] {
        block.extend_from_slice(&bit_offset.to_le_bytes());
        block.push(7);
        block.push(channel);
        block.extend_from_slice(&[0, 0, 0, 0]);
        block.extend_from_slice(&0u32.to_le_bytes());
        block.extend_from_slice(&255u32.to_le_bytes());
    }
    wrap_dfd(block)
}

fn block_format_dfd(model: u8) -> Vec<u8> {
    let mut block = Vec::with_capacity(24);
    block.extend_from_slice(&0u32.to_le_bytes());
    block.extend_from_slice(&2u16.to_le_bytes());
    block.extend_from_slice(&24u16.to_le_bytes());
    block.extend_from_slice(&[model, 1, 1, 0]);
    block.extend_from_slice(&[3, 3, 0, 0]);
    block.extend_from_slice(&[16, 0, 0, 0, 0, 0, 0, 0]);
    block
}

fn wrap_dfd(block: Vec<u8>) -> Vec<u8> {
    let mut dfd = Vec::with_capacity(block.len() + 4);
    dfd.extend_from_slice(&((block.len() + 4) as u32).to_le_bytes());
    dfd.extend_from_slice(&block);
    dfd
}

#[cfg(test)]
mod tests {
    use super::{is_basis_ktx2, ktx2_bytes, promote, transcode_basis, transcode_basis_images, TextureTarget};
    use bevy::image::CompressedImageFormats;
    use serde_json::json;

    #[test]
    fn basis_detection_reads_the_vk_format() {
        let mut header = vec![0xAB, b'K', b'T', b'X', b' ', b'2', b'0', 0xBB, b'\r', b'\n', 0x1A, b'\n'];
        header.resize(44, 0);
        header.extend_from_slice(&1u32.to_le_bytes());
        assert!(is_basis_ktx2(&header));
        header[44] = 2;
        assert!(is_basis_ktx2(&header));
        header[12..16].copy_from_slice(&37u32.to_le_bytes());
        assert!(!is_basis_ktx2(&header));
        assert!(!is_basis_ktx2(b"glTF"));
    }

    #[test]
    fn rgba8_ktx2_lists_levels_smallest_first_and_writes_a_dfd() {
        let bytes = ktx2_bytes(TextureTarget::Rgba8, 4, 2, &[vec![7u8; 32], vec![9u8; 8]]);
        assert_eq!(&bytes[..12], &[0xAB, b'K', b'T', b'X', b' ', b'2', b'0', 0xBB, b'\r', b'\n', 0x1A, b'\n']);
        assert_eq!(u32::from_le_bytes(bytes[12..16].try_into().unwrap()), 37);
        assert_eq!(u32::from_le_bytes(bytes[20..24].try_into().unwrap()), 4);
        assert_eq!(u32::from_le_bytes(bytes[24..28].try_into().unwrap()), 2);
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 2);
        assert_eq!(u32::from_le_bytes(bytes[44..48].try_into().unwrap()), 0);
        let dfd_offset = u32::from_le_bytes(bytes[48..52].try_into().unwrap()) as usize;
        assert_eq!(dfd_offset, 128);
        assert_eq!(u32::from_le_bytes(bytes[dfd_offset..dfd_offset + 4].try_into().unwrap()), 92);
        let level0 = u64::from_le_bytes(bytes[80..88].try_into().unwrap()) as usize;
        let level1 = u64::from_le_bytes(bytes[104..112].try_into().unwrap()) as usize;
        assert_eq!(&bytes[level0..level0 + 32], &[7u8; 32]);
        assert_eq!(&bytes[level1..level1 + 8], &[9u8; 8]);
        assert!(level1 < level0);
    }

    #[test]
    fn gpu_picks_the_first_compressed_format_it_supports_in_bc_astc_etc2_order() {
        let none = CompressedImageFormats::empty();
        assert_eq!(TextureTarget::for_gpu(none), TextureTarget::Rgba8);
        assert_eq!(TextureTarget::for_gpu(CompressedImageFormats::ETC2), TextureTarget::Etc2);
        assert_eq!(TextureTarget::for_gpu(CompressedImageFormats::ETC2 | CompressedImageFormats::ASTC_LDR), TextureTarget::Astc4x4);
        assert_eq!(TextureTarget::for_gpu(CompressedImageFormats::ETC2 | CompressedImageFormats::ASTC_LDR | CompressedImageFormats::BC), TextureTarget::Bc7);
        assert_eq!(TextureTarget::for_gpu(none).name(), "rgba8");
        assert_eq!(TextureTarget::for_gpu(none).cache_dir(), "target/gltf-cache-rgba8");
        assert_eq!(TextureTarget::for_gpu(CompressedImageFormats::ETC2).cache_dir(), "target/gltf-cache-etc2");
        assert_eq!(TextureTarget::for_gpu(CompressedImageFormats::ASTC_LDR).cache_dir(), "target/gltf-cache-astc");
    }

    #[test]
    fn every_target_round_trips_through_its_name() {
        for target in [TextureTarget::Rgba8, TextureTarget::Bc7, TextureTarget::Etc2, TextureTarget::Astc4x4] {
            assert_eq!(TextureTarget::from_name(target.name()), Some(target));
        }
        assert_eq!(TextureTarget::from_name("dxt1"), None);
    }

    #[test]
    fn etc2_and_astc_ktx2_declare_their_vulkan_format_and_block_size() {
        assert_eq!(TextureTarget::Etc2.level_bytes(5, 3), 2 * 16);
        assert_eq!(TextureTarget::Astc4x4.level_bytes(5, 3), 2 * 16);
        for (target, vk_format, model) in [(TextureTarget::Etc2, 151u32, 161u8), (TextureTarget::Astc4x4, 157u32, 162u8)] {
            let bytes = ktx2_bytes(target, 8, 4, &[vec![3u8; 32], vec![5u8; 16]]);
            assert_eq!(u32::from_le_bytes(bytes[12..16].try_into().unwrap()), vk_format);
            assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 2);
            let dfd = u32::from_le_bytes(bytes[48..52].try_into().unwrap()) as usize;
            assert_eq!(bytes[dfd + 12], model);
        }
        assert_eq!(TextureTarget::Etc2.transcoder_format(), 2);
        assert_eq!(TextureTarget::Astc4x4.transcoder_format(), 3);
    }

    #[test]
    fn bc7_ktx2_declares_bc7_and_sizes_levels_in_blocks() {
        assert_eq!(TextureTarget::Bc7.level_bytes(5, 3), 2 * 1 * 16);
        assert_eq!(TextureTarget::Rgba8.level_bytes(5, 3), 60);
        let bytes = ktx2_bytes(TextureTarget::Bc7, 8, 4, &[vec![3u8; 32], vec![5u8; 16]]);
        assert_eq!(u32::from_le_bytes(bytes[12..16].try_into().unwrap()), 145);
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 2);
        let dfd = u32::from_le_bytes(bytes[48..52].try_into().unwrap()) as usize;
        assert_eq!(bytes[dfd + 12], 134);
    }

    fn zstd_raw_frame(data: &[u8]) -> Vec<u8> {
        let mut frame = vec![0x28, 0xB5, 0x2F, 0xFD, 0x20, data.len() as u8];
        frame.extend_from_slice(&((((data.len() as u32) << 3) | 1).to_le_bytes())[..3]);
        frame.extend_from_slice(data);
        frame
    }

    #[test]
    fn zstd_levels_are_inflated_and_the_scheme_is_cleared() {
        let levels: [Vec<u8>; 2] = [(0u8..64).collect(), vec![7u8; 16]];
        let mut file = vec![0u8; 80];
        file[..12].copy_from_slice(&[0xAB, b'K', b'T', b'X', b' ', b'2', b'0', 0xBB, b'\r', b'\n', 0x1A, b'\n']);
        file[40..44].copy_from_slice(&2u32.to_le_bytes());
        file[44..48].copy_from_slice(&2u32.to_le_bytes());
        let mut cursor = 80 + 24 * levels.len();
        let mut index = Vec::new();
        let mut data = Vec::new();
        for level in &levels {
            let frame = zstd_raw_frame(level);
            let frame_len = frame.len();
            cursor = cursor.next_multiple_of(8);
            index.extend_from_slice(&(cursor as u64).to_le_bytes());
            index.extend_from_slice(&(frame_len as u64).to_le_bytes());
            index.extend_from_slice(&(level.len() as u64).to_le_bytes());
            data.push((cursor, frame));
            cursor += frame_len;
        }
        file.extend_from_slice(&index);
        for (offset, frame) in &data {
            file.resize(*offset, 0);
            file.extend_from_slice(frame);
        }
        let inflated = super::inflate_zstd_levels(&file).expect("inflates").expect("zstd scheme");
        assert_eq!(super::read_u32(&inflated, 44), Some(0));
        for (position, level) in levels.iter().enumerate() {
            let at = 80 + 24 * position;
            let offset = super::read_u64(&inflated, at).expect("offset") as usize;
            let length = super::read_u64(&inflated, at + 8).expect("length") as usize;
            assert_eq!(&inflated[offset..offset + length], level.as_slice());
        }
        assert!(super::inflate_zstd_levels(&inflated).expect("plain").is_none());
    }

    #[test]
    fn transcoding_rejects_bytes_that_are_not_ktx2() {
        assert!(transcode_basis(&[0u8; 96], TextureTarget::Rgba8).is_err());
        assert!(transcode_basis(&[0u8; 96], TextureTarget::Bc7).is_err());
    }

    #[test]
    fn images_that_are_not_basis_keep_their_buffer_view() {
        let original = ktx2_bytes(TextureTarget::Rgba8, 1, 1, &[vec![1, 2, 3, 4]]);
        let mut bin = original.clone();
        let mut document = json!({
            "images": [{"mimeType": "image/ktx2", "bufferView": 0}],
            "bufferViews": [{"buffer": 0, "byteOffset": 0, "byteLength": original.len()}]
        });
        assert_eq!(transcode_basis_images(&mut document, &mut bin, TextureTarget::Bc7).unwrap(), 0);
        assert_eq!(bin, original);
        assert_eq!(document["images"][0]["bufferView"], json!(0));
        assert_eq!(document["bufferViews"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn basisu_texture_takes_its_source_index() {
        let mut document = json!({
            "images": [{"mimeType": "image/ktx2", "uri": "a.ktx2"}],
            "textures": [{"extensions": {"KHR_texture_basisu": {"source": 0}}}]
        });
        assert_eq!(promote(&mut document), 1);
        assert_eq!(document["textures"][0], json!({"source": 0}));
    }

    #[test]
    fn texture_without_extension_is_untouched() {
        let mut document = json!({"textures": [{"source": 2, "sampler": 0}]});
        assert_eq!(promote(&mut document), 0);
        assert_eq!(document, json!({"textures": [{"source": 2, "sampler": 0}]}));
    }

    #[test]
    fn extension_is_removed_from_used_and_required_lists() {
        let mut document = json!({
            "extensionsUsed": ["KHR_materials_unlit", "KHR_texture_basisu"],
            "extensionsRequired": ["KHR_texture_basisu"],
            "textures": [{"extensions": {"KHR_texture_basisu": {"source": 1}}}]
        });
        assert_eq!(promote(&mut document), 1);
        assert_eq!(document["extensionsUsed"], json!(["KHR_materials_unlit"]));
        assert_eq!(document["extensionsRequired"], json!([]));
    }

    #[test]
    fn empty_extensions_object_is_removed() {
        let mut document = json!({
            "textures": [{"name": "t", "extensions": {"KHR_texture_basisu": {"source": 3}}}]
        });
        assert_eq!(promote(&mut document), 1);
        assert_eq!(document["textures"][0], json!({"name": "t", "source": 3}));
        assert!(document["textures"][0].get("extensions").is_none());
    }

    #[test]
    fn other_texture_extensions_survive_promotion() {
        let mut document = json!({
            "extensionsUsed": ["KHR_texture_basisu", "EXT_texture_webp"],
            "textures": [{"extensions": {
                "KHR_texture_basisu": {"source": 1},
                "EXT_texture_webp": {"source": 4}
            }}]
        });
        assert_eq!(promote(&mut document), 1);
        assert_eq!(
            document["textures"][0],
            json!({"source": 1, "extensions": {"EXT_texture_webp": {"source": 4}}})
        );
        assert_eq!(document["extensionsUsed"], json!(["EXT_texture_webp"]));
    }

    #[test]
    fn extension_stays_listed_while_a_texture_still_uses_it() {
        let mut document = json!({
            "extensionsUsed": ["KHR_texture_basisu"],
            "textures": [
                {"extensions": {"KHR_texture_basisu": {"source": 0}}},
                {"extensions": {"KHR_texture_basisu": {}}}
            ]
        });
        assert_eq!(promote(&mut document), 1);
        assert_eq!(document["extensionsUsed"], json!(["KHR_texture_basisu"]));
        assert_eq!(
            document["textures"][1],
            json!({"extensions": {"KHR_texture_basisu": {}}})
        );
    }
}
