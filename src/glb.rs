use crate::basisu;
use meshopt::ffi;
use serde_json::{json, Map, Value};
use std::ffi::c_void;

const GLB_MAGIC: u32 = 0x4654_6C67;
const GLB_VERSION: u32 = 2;
const JSON_CHUNK: u32 = 0x4E4F_534A;
const BIN_CHUNK: u32 = 0x004E_4942;
const EXTENSION: &str = "EXT_meshopt_compression";
const MESH_QUANTIZATION: &str = "KHR_mesh_quantization";
const MAX_VIEW_BYTES: usize = 1 << 30;
const MAX_VERTEX_STRIDE: usize = 256;

struct Container {
    json: Value,
    bin: Option<Vec<u8>>,
}

pub struct Decoded {
    pub bytes: Vec<u8>,
    pub meshopt_views: usize,
    pub basisu_textures: usize,
    pub ktx2_textures: Vec<usize>,
}

pub fn decode(bytes: &[u8]) -> Result<Decoded, String> {
    let container = read_container(bytes)?;
    let meshopt = uses_extension(&container.json);
    let (mut json, bin, meshopt_views) = if meshopt {
        unwrap_meshopt(container)?
    } else {
        (container.json, container.bin.unwrap_or_default(), 0)
    };
    let basisu_textures = basisu::promote(&mut json);
    let quantization_retired = retire_extension(&mut json, MESH_QUANTIZATION);
    let ktx2_textures = ktx2_texture_indices(&json);
    let bytes = if meshopt || basisu_textures > 0 || quantization_retired {
        write_container(&json, bin)?
    } else {
        bytes.to_vec()
    };
    Ok(Decoded { bytes, meshopt_views, basisu_textures, ktx2_textures })
}

fn ktx2_texture_indices(json: &Value) -> Vec<usize> {
    let images = array_field(json, "images");
    array_field(json, "textures")
        .iter()
        .enumerate()
        .filter(|(_, texture)| {
            texture
                .get("source")
                .and_then(Value::as_u64)
                .and_then(|source| images.get(source as usize))
                .and_then(|image| image.get("mimeType"))
                .and_then(Value::as_str)
                == Some("image/ktx2")
        })
        .map(|(index, _)| index)
        .collect()
}

fn read_container(bytes: &[u8]) -> Result<Container, String> {
    if read_u32(bytes, 0)? != GLB_MAGIC {
        return Err("input is not a GLB file".to_string());
    }
    if read_u32(bytes, 4)? != GLB_VERSION {
        return Err("GLB version is not 2".to_string());
    }
    let declared = read_u32(bytes, 8)? as usize;
    let file = bytes
        .get(..declared)
        .ok_or("GLB header declares more bytes than the input holds")?;
    let mut json = None;
    let mut bin = None;
    let mut cursor = 12;
    while cursor < declared {
        let length = read_u32(file, cursor)? as usize;
        let kind = read_u32(file, cursor + 4)?;
        if length % 4 != 0 {
            return Err("GLB chunk length is not a multiple of 4".to_string());
        }
        let start = cursor + 8;
        let end = start
            .checked_add(length)
            .filter(|end| *end <= declared)
            .ok_or("GLB chunk runs past the end of the file")?;
        let data = file
            .get(start..end)
            .ok_or("GLB chunk runs past the end of the file")?;
        match kind {
            JSON_CHUNK if json.is_none() => json = Some(parse_json(data)?),
            BIN_CHUNK if bin.is_none() => bin = Some(data.to_vec()),
            JSON_CHUNK | BIN_CHUNK => return Err("GLB has a duplicate chunk".to_string()),
            _ => {}
        }
        cursor = end;
    }
    let json = json.ok_or("GLB has no JSON chunk")?;
    Ok(Container { json, bin })
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, String> {
    bytes
        .get(offset..offset + 4)
        .and_then(|slice| <[u8; 4]>::try_from(slice).ok())
        .map(u32::from_le_bytes)
        .ok_or_else(|| "GLB is truncated".to_string())
}

fn parse_json(data: &[u8]) -> Result<Value, String> {
    serde_json::from_slice(data).map_err(|error| format!("GLB JSON chunk is invalid: {error}"))
}

fn write_container(json: &Value, mut bin: Vec<u8>) -> Result<Vec<u8>, String> {
    let mut json_bytes = serde_json::to_vec(json).map_err(|error| error.to_string())?;
    json_bytes.resize(json_bytes.len().next_multiple_of(4), b' ');
    bin.resize(bin.len().next_multiple_of(4), 0);
    let bin_section = if bin.is_empty() { 0 } else { 8 + bin.len() };
    let declared = u32::try_from(12 + 8 + json_bytes.len() + bin_section)
        .map_err(|_| "GLB exceeds 4 GiB".to_string())?;
    let mut out = Vec::with_capacity(declared as usize);
    out.extend_from_slice(&GLB_MAGIC.to_le_bytes());
    out.extend_from_slice(&GLB_VERSION.to_le_bytes());
    out.extend_from_slice(&declared.to_le_bytes());
    push_chunk(&mut out, JSON_CHUNK, &json_bytes)?;
    if !bin.is_empty() {
        push_chunk(&mut out, BIN_CHUNK, &bin)?;
    }
    Ok(out)
}

fn push_chunk(out: &mut Vec<u8>, kind: u32, data: &[u8]) -> Result<(), String> {
    let length = u32::try_from(data.len()).map_err(|_| "GLB chunk exceeds 4 GiB".to_string())?;
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(&kind.to_le_bytes());
    out.extend_from_slice(data);
    Ok(())
}

fn is_packed(view: &Value) -> bool {
    view.get("extensions").and_then(|ext| ext.get(EXTENSION)).is_some()
}

fn uses_extension(json: &Value) -> bool {
    let listed = ["extensionsUsed", "extensionsRequired"].iter().any(|key| {
        json.get(key)
            .and_then(Value::as_array)
            .is_some_and(|names| names.iter().any(|name| name.as_str() == Some(EXTENSION)))
    });
    let packed_view = json
        .get("bufferViews")
        .and_then(Value::as_array)
        .is_some_and(|views| {
            views
                .iter()
                .any(is_packed)
        });
    listed || packed_view
}

fn unwrap_meshopt(container: Container) -> Result<(Value, Vec<u8>, usize), String> {
    let Container { mut json, bin } = container;
    let buffers = array_field(&json, "buffers");
    if buffers.first().and_then(|buffer| buffer.get("uri")).is_some() {
        return Err("buffer 0 is external and is not handled".to_string());
    }
    let views = array_field(&json, "bufferViews");
    let meshopt_views = views.iter().filter(|view| is_packed(view)).count();
    let mut plain_bin = Vec::new();
    let mut plain_views = Vec::with_capacity(views.len());
    for (index, view) in views.iter().enumerate() {
        let plain = plain_view(view, &buffers, bin.as_deref(), &mut plain_bin)
            .map_err(|error| format!("bufferView {index}: {error}"))?;
        plain_views.push(plain);
    }
    let root = json.as_object_mut().ok_or("GLB JSON root is not an object")?;
    if root.contains_key("bufferViews") {
        root.insert("bufferViews".into(), Value::Array(plain_views));
    }
    if buffers.is_empty() && plain_bin.is_empty() {
        root.remove("buffers");
    } else {
        let mut buffer = buffers
            .first()
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        buffer.remove("extensions");
        buffer.insert("byteLength".into(), json!(plain_bin.len()));
        root.insert("buffers".into(), Value::Array(vec![Value::Object(buffer)]));
    }
    remove_extension_name(root, "extensionsUsed", EXTENSION);
    remove_extension_name(root, "extensionsRequired", EXTENSION);
    Ok((json, plain_bin, meshopt_views))
}

fn array_field(json: &Value, key: &str) -> Vec<Value> {
    json.get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn retire_extension(json: &mut Value, extension: &str) -> bool {
    let Some(root) = json.as_object_mut() else {
        return false;
    };
    let used = remove_extension_name(root, "extensionsUsed", extension);
    let required = remove_extension_name(root, "extensionsRequired", extension);
    used || required
}

fn remove_extension_name(root: &mut Map<String, Value>, key: &str, extension: &str) -> bool {
    let (removed, emptied) = match root.get_mut(key).and_then(Value::as_array_mut) {
        Some(names) => {
            let before = names.len();
            names.retain(|name| name.as_str() != Some(extension));
            (names.len() != before, names.is_empty())
        }
        None => (false, false),
    };
    if emptied {
        root.remove(key);
    }
    removed
}

fn plain_view(
    view: &Value,
    buffers: &[Value],
    bin: Option<&[u8]>,
    out: &mut Vec<u8>,
) -> Result<Value, String> {
    let mut object = view
        .as_object()
        .cloned()
        .ok_or("bufferView is not an object")?;
    let data = match packed_extension(&mut object) {
        Some(packed) => {
            let source = buffer_bytes(buffers, bin, optional_usize(&packed, "buffer")?)?;
            let encoded = slice_at(
                source,
                optional_usize(&packed, "byteOffset")?,
                usize_field(&packed, "byteLength")?,
            )?;
            decode_packed(&packed, encoded)?
        }
        None => {
            let source = buffer_bytes(buffers, bin, usize_field(&object, "buffer")?)?;
            slice_at(
                source,
                optional_usize(&object, "byteOffset")?,
                usize_field(&object, "byteLength")?,
            )?
            .to_vec()
        }
    };
    let offset = append_aligned(out, &data);
    object.insert("buffer".into(), json!(0));
    object.insert("byteOffset".into(), json!(offset));
    object.insert("byteLength".into(), json!(data.len()));
    Ok(Value::Object(object))
}

fn packed_extension(object: &mut Map<String, Value>) -> Option<Map<String, Value>> {
    let extensions = object.get_mut("extensions")?.as_object_mut()?;
    let packed = extensions.remove(EXTENSION)?.as_object().cloned()?;
    if extensions.is_empty() {
        object.remove("extensions");
    }
    Some(packed)
}

fn buffer_bytes<'a>(
    buffers: &[Value],
    bin: Option<&'a [u8]>,
    index: usize,
) -> Result<&'a [u8], String> {
    let buffer = buffers
        .get(index)
        .ok_or_else(|| format!("buffer {index} is missing"))?;
    if let Some(uri) = buffer.get("uri").and_then(Value::as_str) {
        return Err(format!("buffer {index} is external ({uri}) and is not handled"));
    }
    if index != 0 {
        return Err(format!("buffer {index} has no data inside the GLB"));
    }
    bin.ok_or_else(|| "GLB has no BIN chunk".to_string())
}

fn slice_at(bytes: &[u8], offset: usize, length: usize) -> Result<&[u8], String> {
    offset
        .checked_add(length)
        .and_then(|end| bytes.get(offset..end))
        .ok_or_else(|| {
            format!(
                "range {offset}+{length} lies outside a {} byte buffer",
                bytes.len()
            )
        })
}

fn usize_field(object: &Map<String, Value>, key: &str) -> Result<usize, String> {
    object
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| format!("missing or invalid {key}"))
}

fn optional_usize(object: &Map<String, Value>, key: &str) -> Result<usize, String> {
    if object.contains_key(key) {
        usize_field(object, key)
    } else {
        Ok(0)
    }
}

fn str_field<'a>(object: &'a Map<String, Value>, key: &str) -> Result<&'a str, String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing {key}"))
}

fn append_aligned(out: &mut Vec<u8>, data: &[u8]) -> usize {
    out.resize(out.len().next_multiple_of(4), 0);
    let offset = out.len();
    out.extend_from_slice(data);
    offset
}

fn decode_packed(packed: &Map<String, Value>, encoded: &[u8]) -> Result<Vec<u8>, String> {
    let count = usize_field(packed, "count")?;
    let stride = usize_field(packed, "byteStride")?;
    let mode = str_field(packed, "mode")?;
    let filter = packed
        .get("filter")
        .and_then(Value::as_str)
        .unwrap_or("NONE");
    let size = count
        .checked_mul(stride)
        .filter(|size| *size <= MAX_VIEW_BYTES)
        .ok_or("decoded bufferView is too large")?;
    if size == 0 {
        return Ok(Vec::new());
    }
    match mode {
        "ATTRIBUTES" => decode_attributes(encoded, count, stride, filter),
        "TRIANGLES" => {
            require_plain(filter)?;
            decode_indices(encoded, count, stride, false)
        }
        "INDICES" => {
            require_plain(filter)?;
            decode_indices(encoded, count, stride, true)
        }
        other => Err(format!("unknown meshopt mode {other}")),
    }
}

fn require_plain(filter: &str) -> Result<(), String> {
    if filter == "NONE" {
        Ok(())
    } else {
        Err(format!("filter {filter} does not apply to index data"))
    }
}

fn decode_indices(
    encoded: &[u8],
    count: usize,
    stride: usize,
    sequence: bool,
) -> Result<Vec<u8>, String> {
    let decoded = match (stride, sequence) {
        (2, false) => meshopt::decode_index_buffer::<u16>(encoded, count).map(u16_bytes),
        (4, false) => meshopt::decode_index_buffer::<u32>(encoded, count).map(u32_bytes),
        (2, true) => meshopt::decode_index_sequence::<u16>(encoded, count).map(u16_bytes),
        (4, true) => meshopt::decode_index_sequence::<u32>(encoded, count).map(u32_bytes),
        _ => return Err(format!("index stride {stride} is not 2 or 4")),
    };
    decoded.map_err(describe)
}

fn decode_attributes(
    encoded: &[u8],
    count: usize,
    stride: usize,
    filter: &str,
) -> Result<Vec<u8>, String> {
    if stride == 0 || stride > MAX_VERTEX_STRIDE || stride % 4 != 0 {
        return Err(format!(
            "vertex stride {stride} is not a multiple of 4 up to {MAX_VERTEX_STRIDE}"
        ));
    }
    let mut data = vec![0u8; count * stride];
    let code = unsafe {
        ffi::meshopt_decodeVertexBuffer(
            data.as_mut_ptr().cast(),
            count,
            stride,
            encoded.as_ptr(),
            encoded.len(),
        )
    };
    if code != 0 {
        return Err(format!("meshopt vertex decode failed with code {code}"));
    }
    apply_filter(&mut data, count, stride, filter)?;
    Ok(data)
}

fn apply_filter(data: &mut [u8], count: usize, stride: usize, filter: &str) -> Result<(), String> {
    let stride_ok = match filter {
        "NONE" => return Ok(()),
        "OCTAHEDRAL" => stride == 4 || stride == 8,
        "QUATERNION" => stride == 8,
        "EXPONENTIAL" => stride % 4 == 0,
        other => return Err(format!("unknown meshopt filter {other}")),
    };
    if !stride_ok {
        return Err(format!("filter {filter} does not support stride {stride}"));
    }
    let pointer = data.as_mut_ptr().cast::<c_void>();
    unsafe {
        match filter {
            "OCTAHEDRAL" => ffi::meshopt_decodeFilterOct(pointer, count, stride),
            "QUATERNION" => ffi::meshopt_decodeFilterQuat(pointer, count, stride),
            _ => ffi::meshopt_decodeFilterExp(pointer, count, stride),
        }
    }
    Ok(())
}

fn u16_bytes(values: Vec<u16>) -> Vec<u8> {
    values.into_iter().flat_map(u16::to_le_bytes).collect()
}

fn u32_bytes(values: Vec<u32>) -> Vec<u8> {
    values.into_iter().flat_map(u32::to_le_bytes).collect()
}

fn describe<E: std::fmt::Debug>(error: E) -> String {
    format!("meshopt decode failed: {error:?}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const CDN: &str = "/workspace/neon-syndicates/assets/cdn";
    const BASISU_EXTENSION: &str = "KHR_texture_basisu";

    fn smallest_glb_using(extension: &str) -> (PathBuf, Vec<u8>) {
        std::fs::read_dir(CDN)
            .expect("cdn directory is readable")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "glb"))
            .filter_map(|path| std::fs::read(&path).ok().map(|bytes| (path, bytes)))
            .filter(|(_, bytes)| {
                bytes
                    .windows(extension.len())
                    .any(|window| window == extension.as_bytes())
            })
            .min_by_key(|(_, bytes)| bytes.len())
            .expect("no GLB in the cdn directory uses the requested extension")
    }

    fn float_components(type_name: &str) -> Option<usize> {
        match type_name {
            "SCALAR" => Some(1),
            "VEC2" => Some(2),
            "VEC3" => Some(3),
            "VEC4" => Some(4),
            _ => None,
        }
    }

    #[test]
    fn decodes_smallest_meshopt_glb_into_plain_glb() {
        let (path, input) = smallest_glb_using(EXTENSION);
        let output = decode(&input).unwrap_or_else(|error| panic!("{}: {error}", path.display())).bytes;
        let source = read_container(&input).expect("input parses");
        let source_views = source.json["bufferViews"].as_array().expect("views").len();

        let container = read_container(&output).expect("decoded GLB parses");
        let json = &container.json;
        let bin = container.bin.expect("decoded GLB has a BIN chunk");
        for key in ["extensionsUsed", "extensionsRequired"] {
            let names = json.get(key).and_then(Value::as_array).cloned().unwrap_or_default();
            assert!(names.iter().all(|name| name.as_str() != Some(EXTENSION)), "{key} still lists the extension");
        }
        let views = json["bufferViews"].as_array().expect("bufferViews kept");
        assert_eq!(views.len(), source_views);
        for view in views {
            assert!(view.get("extensions").is_none());
            let offset = view.get("byteOffset").and_then(Value::as_u64).unwrap_or(0) as usize;
            let length = view["byteLength"].as_u64().expect("byteLength") as usize;
            assert!(offset + length <= bin.len(), "bufferView escapes the BIN chunk");
        }
        let buffers = json["buffers"].as_array().expect("buffers");
        assert_eq!(buffers.len(), 1);
        assert!(buffers[0]["byteLength"].as_u64().expect("byteLength") as usize <= bin.len());

        let mut checked = 0;
        for accessor in json["accessors"].as_array().expect("accessors") {
            if accessor["componentType"].as_u64() != Some(5126) {
                continue;
            }
            let (Some(view_index), Some(components)) = (
                accessor.get("bufferView").and_then(Value::as_u64),
                accessor["type"].as_str().and_then(float_components),
            ) else {
                continue;
            };
            let (Some(mins), Some(maxs)) = (accessor["min"].as_array(), accessor["max"].as_array()) else {
                continue;
            };
            if mins.len() != components || maxs.len() != components {
                continue;
            }
            let view = &views[view_index as usize];
            let stride = view
                .get("byteStride")
                .and_then(Value::as_u64)
                .map_or(components * 4, |stride| stride as usize);
            let base = view.get("byteOffset").and_then(Value::as_u64).unwrap_or(0) as usize
                + accessor.get("byteOffset").and_then(Value::as_u64).unwrap_or(0) as usize;
            let count = accessor["count"].as_u64().expect("count") as usize;
            for element in 0..count {
                for component in 0..components {
                    let at = base + element * stride + component * 4;
                    let bytes = bin.get(at..at + 4).expect("accessor lies inside the BIN chunk");
                    let value = f32::from_le_bytes(bytes.try_into().expect("four bytes")) as f64;
                    let low = mins[component].as_f64().expect("min");
                    let high = maxs[component].as_f64().expect("max");
                    assert!(value >= low - 1e-3 && value <= high + 1e-3, "decoded value {value} outside [{low}, {high}]");
                }
            }
            checked += 1;
        }
        assert!(checked > 0, "no float accessor with bounds was checked");
    }

    #[test]
    fn plain_glb_is_returned_unchanged() {
        let plain = write_container(&json!({ "asset": { "version": "2.0" } }), Vec::new()).expect("writes");
        assert_eq!(decode(&plain).expect("decodes").bytes, plain);
    }

    #[test]
    fn basisu_glb_decodes_to_plain_glb_with_ktx2_images() {
        let (path, input) = smallest_glb_using(BASISU_EXTENSION);
        let decoded = decode(&input).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert!(decoded.basisu_textures > 0, "no KHR_texture_basisu texture was promoted");
        assert!(!decoded.ktx2_textures.is_empty(), "no promoted texture resolves to image/ktx2");
        let container = read_container(&decoded.bytes).expect("decoded GLB parses");
        assert!(!container.json.to_string().contains(BASISU_EXTENSION), "decoded GLB still names the extension");
        assert!(!container.json.to_string().contains(MESH_QUANTIZATION), "decoded GLB still requires mesh quantization");
        let images = container.json["images"].as_array().expect("images kept");
        assert!(images.iter().any(|image| image["mimeType"] == "image/ktx2"));
    }

    #[test]
    fn malformed_input_is_rejected_without_panicking() {
        let (_, input) = smallest_glb_using(EXTENSION);
        assert!(decode(b"not a glb at all").is_err());
        assert!(decode(&input[..input.len() / 2]).is_err());
        for step in 1..64 {
            let mut corrupted = input.clone();
            let index = corrupted.len() - step * 37 % (corrupted.len() / 2);
            corrupted[index] ^= 0xA5;
            let _ = decode(&corrupted);
        }
    }
}
