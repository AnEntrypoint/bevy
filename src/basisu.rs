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
const VK_FORMAT_UNDEFINED: u32 = 0;
const VK_FORMAT_R8G8B8A8_UNORM: u32 = 37;

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
    fn bevy_basislz_transcode_level(data: *const u8, size: u32, level: u32, out: *mut u8, capacity: u64) -> i64;
}

pub fn transcode_basis_images(json: &mut Value, bin: &mut Vec<u8>) -> Result<usize, String> {
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
                let rgba = transcode_basis(source).map_err(|error| format!("image {image_index}: {error}"))?;
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

pub fn transcode_basis(bytes: &[u8]) -> Result<Vec<u8>, String> {
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
        let capacity = level_width * level_height * 4;
        let mut buffer = vec![0u8; capacity];
        let written = unsafe {
            bevy_basislz_transcode_level(bytes.as_ptr(), size, level, buffer.as_mut_ptr(), capacity as u64)
        };
        if written != capacity as i64 {
            return Err(format!("basis transcoder failed on level {level} (status {written})"));
        }
        pixels.push(buffer);
    }
    Ok(rgba8_ktx2_bytes(width, height, &pixels))
}

pub fn rgba8_ktx2_bytes(width: u32, height: u32, levels: &[Vec<u8>]) -> Vec<u8> {
    let dfd = rgba8_dfd();
    let dfd_offset = KTX2_HEADER_LEN + KTX2_LEVEL_ENTRY_LEN * levels.len();
    let mut offsets = vec![0usize; levels.len()];
    let mut cursor = dfd_offset + dfd.len();
    for level in (0..levels.len()).rev() {
        offsets[level] = cursor;
        cursor = (cursor + levels[level].len()).next_multiple_of(4);
    }
    let mut out = Vec::with_capacity(cursor);
    out.extend_from_slice(&KTX2_IDENTIFIER);
    for value in [VK_FORMAT_R8G8B8A8_UNORM, 1, width, height, 0, 0, 1, levels.len() as u32, 0] {
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
    let mut dfd = Vec::with_capacity(block.len() + 4);
    dfd.extend_from_slice(&((block.len() + 4) as u32).to_le_bytes());
    dfd.extend_from_slice(&block);
    dfd
}

#[cfg(test)]
mod tests {
    use super::{is_basis_ktx2, promote, rgba8_ktx2_bytes, transcode_basis, transcode_basis_images};
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
        let bytes = rgba8_ktx2_bytes(4, 2, &[vec![7u8; 32], vec![9u8; 8]]);
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
    fn transcoding_rejects_bytes_that_are_not_ktx2() {
        assert!(transcode_basis(&[0u8; 96]).is_err());
    }

    #[test]
    fn images_that_are_not_basis_keep_their_buffer_view() {
        let original = rgba8_ktx2_bytes(1, 1, &[vec![1, 2, 3, 4]]);
        let mut bin = original.clone();
        let mut document = json!({
            "images": [{"mimeType": "image/ktx2", "bufferView": 0}],
            "bufferViews": [{"buffer": 0, "byteOffset": 0, "byteLength": original.len()}]
        });
        assert_eq!(transcode_basis_images(&mut document, &mut bin).unwrap(), 0);
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
