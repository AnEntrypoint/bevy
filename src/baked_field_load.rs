#![cfg_attr(not(test), allow(dead_code))]
use crate::height::baked;
use crate::lattice_surface::{baked_hash_version_of, baked_terrain_mismatch, js_integer, js_json, js_json_or_null, js_num, truthy, LatticeSurface, LatticeTerrain};
use serde_json::Value;

const SOURCE_FNS: [&str; 3] = ["readBytes", "heightfieldCodec", "bakeCodeVersion"];
const HF_MAGIC: u32 = 0x4846_4d31;
const HF_HEADER_BYTES: usize = 8;
const HF_SUPPORTED_BITS: [f64; 2] = [8.0, 16.0];

#[derive(Debug, PartialEq, Eq)]
pub struct BakedSourceMissing {
    pub url: String,
    pub missing: &'static str,
}

impl BakedSourceMissing {
    pub fn name(&self) -> &'static str {
        "BakedSourceMissingError"
    }

    pub fn code(&self) -> &'static str {
        "baked-source-missing"
    }

    pub fn message(&self) -> String {
        format!(
            "[baked-heightfield] {}: no baked artifact source supplies {}(), so this loader refuses to guess which platform reads the bytes -- pass nodeBakedSource() or browserBakedSource() from src/terrain/BakedSourceAdapter.js",
            self.url, self.missing
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Failure {
    Missing(BakedSourceMissing),
    Error(String),
    Unported(String),
}

pub enum Outcome {
    Absent,
    Ignored(String),
    Loaded { surface: LatticeSurface, code_version_verified: bool },
}

pub struct BakedRequest<'a> {
    pub url: &'a str,
    pub supplied: &'a [&'a str],
    pub bytes: Option<&'a [u8]>,
    pub bake_code: Option<&'a str>,
    pub hash_version: u64,
}

enum Kind {
    Hf,
    Json,
}

pub fn load_baked_heightfield(request: &BakedRequest, terrain: &LatticeTerrain, live_epoch: f64) -> Result<Outcome, Failure> {
    let url = request.url;
    if url.is_empty() {
        return Ok(Outcome::Absent);
    }
    require_baked_source(url, request.supplied)?;
    let Some(bytes) = request.bytes else {
        return Ok(Outcome::Absent);
    };
    let (kind, header) = if url.to_ascii_lowercase().ends_with(".hf") {
        (Kind::Hf, hf_header(url, bytes)?)
    } else {
        (Kind::Json, json_artifact(url, bytes)?)
    };
    let baked_version = baked_hash_version_of(&header);
    if baked_version != request.hash_version {
        return Ok(Outcome::Ignored(format!(
            "[terrain] ignoring baked heightfield {url}: baked with terrain hashVersion {baked_version}, world uses {} -> exact CPU height",
            request.hash_version
        )));
    }
    let code_version = header.get("codeVersion");
    match request.bake_code {
        Some(code) if code_version.and_then(Value::as_str) != Some(code) => {
            return Err(Failure::Error(baked_field_error(
                url,
                &format!(
                    "stale bake: stamped with height code version {}, this tree bakes {code}: the height-generating code changed under the artifact, re-bake it",
                    js_json_or_null(code_version)
                ),
            )));
        }
        None if !truthy(code_version) => {
            return Ok(Outcome::Ignored(format!(
                "[terrain] ignoring baked heightfield {url}: it carries no height code version and this runtime has no filesystem to rehash the bake sources into one -> exact CPU height"
            )));
        }
        _ => {}
    }
    if let Some(mismatch) = baked_terrain_mismatch(&header, terrain, live_epoch) {
        return Ok(Outcome::Ignored(format!(
            "[terrain] ignoring baked heightfield {url}: {mismatch} -> exact CPU height"
        )));
    }
    let field = match kind {
        Kind::Hf => baked::BakedHeightField::from_hf(bytes)
            .map_err(|e| Failure::Unported(format!("height/baked.rs from_hf: {e}")))?,
        Kind::Json => json_field(&header)?,
    };
    let surface = LatticeSurface::from_field(field, &header).ok_or_else(|| {
        Failure::Unported("LatticeSurface::from_field refuses an N or extent that spoint's createBakedHeightField accepts".to_string())
    })?;
    Ok(Outcome::Loaded {
        surface,
        code_version_verified: request.bake_code.is_some(),
    })
}

fn baked_field_error(url: &str, why: &str) -> String {
    format!("[baked-heightfield] {url}: {why}")
}

fn require_baked_source(url: &str, supplied: &[&str]) -> Result<(), Failure> {
    match SOURCE_FNS.into_iter().find(|f| !supplied.contains(f)) {
        Some(missing) => Err(Failure::Missing(BakedSourceMissing {
            url: url.to_string(),
            missing,
        })),
        None => Ok(()),
    }
}

fn hf_header(url: &str, bytes: &[u8]) -> Result<Value, Failure> {
    let refuse = |why: String| Failure::Error(baked_field_error(url, &why));
    if bytes.len() < HF_HEADER_BYTES {
        return Err(refuse(format!(
            "truncated: {} bytes, a heightfield header alone is {HF_HEADER_BYTES} bytes",
            bytes.len()
        )));
    }
    let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    if magic != HF_MAGIC {
        return Err(refuse(format!(
            "not a heightfield: bytes 0..3 are 0x{magic:08x}, expected magic 0x{HF_MAGIC:x}"
        )));
    }
    let hlen = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    if hlen == 0 || HF_HEADER_BYTES + hlen > bytes.len() {
        return Err(refuse(format!(
            "malformed: the header-length field says {hlen} bytes, the file is {} bytes",
            bytes.len()
        )));
    }
    let header: Value = serde_json::from_slice(&bytes[HF_HEADER_BYTES..HF_HEADER_BYTES + hlen])
        .map_err(|e| refuse(format!("unreadable header: {e}")))?;
    let n = js_integer(header.get("N"))
        .filter(|n| *n >= 2.0)
        .ok_or_else(|| {
            refuse(format!(
                "malformed header: N is {}, expected an integer >= 2",
                js_json_or_null(header.get("N"))
            ))
        })?;
    let sectors = header.get("sectors").filter(|s| truthy(Some(*s)));
    let grid = js_integer(sectors.and_then(|s| s.get("gridS"))).filter(|g| *g >= 1.0);
    let per_sector = js_integer(sectors.and_then(|s| s.get("nodesPerSector")));
    let (Some(grid), Some(_)) = (grid, per_sector) else {
        return Err(refuse(format!(
            "malformed header: sectors is {}, expected integer gridS >= 1 and nodesPerSector",
            js_json_or_null(header.get("sectors"))
        )));
    };
    let bits_raw = sectors.and_then(|s| s.get("bits"));
    let bits = if truthy(bits_raw) { bits_raw.and_then(Value::as_f64) } else { Some(8.0) };
    let Some(bits) = bits.filter(|b| HF_SUPPORTED_BITS.contains(b)) else {
        return Err(refuse(format!(
            "malformed header: sectors.bits is {}, expected 8 or 16",
            js_json_or_null(bits_raw)
        )));
    };
    let node_bytes = if bits == 16.0 { 2.0 } else { 1.0 };
    let expected = (HF_HEADER_BYTES + hlen) as f64 + 8.0 * grid * grid + node_bytes * n * n;
    let len = bytes.len() as f64;
    if len != expected {
        let kind = if len < expected { "truncated" } else { "longer than declared" };
        return Err(refuse(format!(
            "{kind}: the file is {} bytes, but N={} with sectors.gridS={} at {}-bit samples is exactly {} bytes",
            bytes.len(),
            js_num(n),
            js_num(grid),
            js_num(bits),
            js_num(expected)
        )));
    }
    Ok(header)
}

fn json_artifact(url: &str, bytes: &[u8]) -> Result<Value, Failure> {
    let refuse = |why: String| Failure::Error(baked_field_error(url, &why));
    let json: Value = serde_json::from_slice(bytes).map_err(|e| refuse(format!("not valid JSON: {e}")))?;
    if !truthy(Some(&json)) || !matches!(json, Value::Object(_) | Value::Array(_)) {
        return Err(refuse(format!("malformed: expected a JSON object, got {}", js_json(&json))));
    }
    let Some(n) = json.get("N").and_then(Value::as_f64).filter(|n| *n > 0.0) else {
        return Err(refuse(format!(
            "malformed: N is {}, expected a positive sample count",
            js_json_or_null(json.get("N"))
        )));
    };
    let heights = json.get("heights").and_then(Value::as_array);
    let sectors = json.get("sectors").filter(|s| truthy(Some(*s)));
    let q = sectors.and_then(|s| s.get("q")).and_then(Value::as_array);
    if heights.is_none() && q.is_none() {
        let tail = if json.get("sectors").is_none() { "" } else { " or at sectors.q" };
        return Err(refuse(format!("malformed: no height samples, expected an array at heights{tail}")));
    }
    let cells = n * n;
    if let Some(h) = heights.filter(|h| h.len() as f64 != cells) {
        return Err(refuse(format!(
            "sample count mismatch: N={} declares {} heights, the array holds {}",
            js_num(n),
            js_num(cells),
            h.len()
        )));
    }
    if let Some(samples) = q.filter(|samples| samples.len() as f64 != cells) {
        return Err(refuse(format!(
            "sample count mismatch: N={} declares {} quantized samples, sectors.q holds {}",
            js_num(n),
            js_num(cells),
            samples.len()
        )));
    }
    Ok(json)
}

fn json_field(header: &Value) -> Result<baked::BakedHeightField, Failure> {
    if truthy(header.get("sectors")) {
        return Err(Failure::Unported(
            "sectors.q bakes are dequantized by createBakedHeightField, not ported".to_string(),
        ));
    }
    baked::BakedHeightField::from_json(header).map_err(|e| Failure::Unported(format!("height/baked.rs from_json: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lattice_surface::witness::{assert_close, bytes_of, num, terrain};

    #[test]
    fn baked_field_load_matches_spoint_refusals() {
        let fixture: Value = serde_json::from_str(include_str!("../testdata/parity/spoint-baked-field-load.json")).unwrap();
        let terrain = terrain(&fixture["terrain"]);
        for case in fixture["cases"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let url = case["url"].as_str().unwrap();
            let supplied: Vec<&str> = case["supplied"]
                .as_array()
                .map_or_else(Vec::new, |names| names.iter().map(|s| s.as_str().unwrap()).collect());
            let bytes = case["bytesHex"].as_str().map(bytes_of);
            let request = BakedRequest {
                url,
                supplied: &supplied,
                bytes: bytes.as_deref(),
                bake_code: case["bakeCode"].as_str(),
                hash_version: case["hashVersion"].as_u64().unwrap(),
            };
            let got = load_baked_heightfield(&request, &terrain, num(&case["frameEpoch"]));
            let logged: Vec<String> = match &got {
                Ok(Outcome::Ignored(message)) => vec![message.clone()],
                _ => Vec::new(),
            };
            let want_warnings: Vec<String> = case["warnings"]
                .as_array()
                .unwrap()
                .iter()
                .map(|w| w.as_str().unwrap().to_string())
                .collect();
            assert_eq!(logged, want_warnings, "{name}: warnings");
            let want = case["kind"].as_str().unwrap();
            match (want, got) {
                ("absent", Ok(Outcome::Absent)) | ("ignored", Ok(Outcome::Ignored(_))) => {}
                ("loaded", Ok(Outcome::Loaded { surface, code_version_verified })) => {
                    let frame_epoch = num(&case["frameEpoch"]);
                    assert_eq!(code_version_verified, case["codeVersionVerified"].as_bool().unwrap(), "{name}: codeVersionVerified");
                    assert_eq!(surface.n() as u64, case["N"].as_u64().unwrap(), "{name}: N");
                    assert_close(surface.extent(), num(&case["extent"]), &format!("{name}: extent"));
                    assert_close(surface.chart_epoch(), num(&case["chartEpoch"]), &format!("{name}: chartEpoch"));
                    for (k, probe) in case["probes"].as_array().unwrap().iter().enumerate() {
                        let (x, z) = (num(&probe["x"]), num(&probe["z"]));
                        assert_eq!(surface.covers(x, z, frame_epoch), probe["covers"].as_bool().unwrap(), "{name}: probe {k} covers");
                        assert_close(surface.height_at_local(x, z, frame_epoch), num(&probe["height"]), &format!("{name}: probe {k} height"));
                    }
                }
                ("missing", Err(Failure::Missing(missing))) => {
                    assert_eq!(missing.missing, case["missing"].as_str().unwrap(), "{name}: missing");
                    assert_eq!(missing.message(), case["message"].as_str().unwrap(), "{name}: message");
                    assert_eq!(missing.code(), case["errorCode"].as_str().unwrap(), "{name}: code");
                    assert_eq!(missing.name(), case["errorName"].as_str().unwrap(), "{name}: name");
                }
                ("error", Err(Failure::Error(message))) => {
                    let want_message = case["message"].as_str().unwrap();
                    if case["messageIsPrefix"].as_bool().unwrap_or(false) {
                        assert!(message.starts_with(want_message), "{name}: message {message:?} against spoint prefix {want_message:?}");
                    } else {
                        assert_eq!(message, want_message, "{name}: message");
                    }
                }
                (want, _) => panic!("{name}: spoint says {want}, bevy does not match"),
            }
        }
    }
}
