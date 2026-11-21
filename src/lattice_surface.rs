#![cfg_attr(not(test), allow(dead_code))]
use crate::height::baked;
use serde_json::{json, Value};

const LEGACY_HASH_VERSION: u64 = 1;
const DIRECTION_TOLERANCE: f64 = 1e-6;

pub struct LatticeTerrain {
    pub hash_version: u64,
    pub bake_key: String,
    pub bake_slice_text: String,
    pub radius: f64,
    pub relief_scale: Option<f64>,
    pub anchor_dir: [f64; 3],
}

pub struct LatticeSurface {
    field: baked::BakedHeightField,
    n: usize,
    extent: f64,
    center: [f64; 2],
    chart_epoch: f64,
}

pub fn load_lattice_surface(bytes: &[u8], terrain: &LatticeTerrain, live_epoch: f64) -> Option<LatticeSurface> {
    let field = baked::BakedHeightField::from_hf(bytes).ok()?;
    let (header, _) = baked::split_hf(bytes).ok()?;
    if baked_hash_version_of(&header) != terrain.hash_version {
        return None;
    }
    if baked_terrain_mismatch(&header, terrain, live_epoch).is_some() {
        return None;
    }
    LatticeSurface::from_field(field, &header)
}

pub fn baked_epoch_of(header: &Value) -> f64 {
    header.get("chartEpoch").and_then(Value::as_f64).unwrap_or(0.0)
}

pub fn baked_hash_version_of(header: &Value) -> u64 {
    js_integer(header.get("hashVersion")).filter(|v| *v >= 0.0).map_or(LEGACY_HASH_VERSION, |v| v as u64)
}

pub fn baked_terrain_mismatch(header: &Value, terrain: &LatticeTerrain, live_epoch: f64) -> Option<String> {
    let baked_epoch = baked_epoch_of(header);
    if baked_epoch != live_epoch {
        return Some(format!(
            "baked at chartEpoch {}, world frame is at chartEpoch {}: the chart was reanchored, so its chart-local heights name different ground",
            js_num(baked_epoch),
            js_num(live_epoch)
        ));
    }
    if let Some(key) = header.get("terrainKey") {
        return (key.as_str() != Some(terrain.bake_key.as_str())).then(|| {
            format!("terrain key {} != world {} (slice {})", js_text(key), terrain.bake_key, terrain.bake_slice_text)
        });
    }
    if header.get("radius").and_then(Value::as_f64) != Some(terrain.radius) {
        return Some(format!("radius {} != world {}", js_show(header.get("radius")), js_num(terrain.radius)));
    }
    if !same_relief_scale(header.get("reliefScale"), terrain.relief_scale) {
        return Some(format!(
            "reliefScale {} != world {}",
            js_show(header.get("reliefScale")),
            terrain.relief_scale.map_or_else(|| "null".to_string(), js_num)
        ));
    }
    if !same_direction(header.get("anchorDir"), terrain.anchor_dir) {
        return Some(format!("anchorDir {} != world {}", js_show(header.get("anchorDir")), js_json(&json!(terrain.anchor_dir))));
    }
    None
}

fn same_relief_scale(baked: Option<&Value>, world: Option<f64>) -> bool {
    match (baked.filter(|v| !v.is_null()), world) {
        (None, None) => true,
        (Some(v), Some(w)) => v.as_f64() == Some(w),
        _ => false,
    }
}

fn same_direction(artifact: Option<&Value>, want: [f64; 3]) -> bool {
    let Some(values) = artifact.and_then(Value::as_array).filter(|v| v.len() == 3) else {
        return false;
    };
    let got = [0_usize, 1, 2].map(|i| values[i].as_f64().unwrap_or(f64::NAN));
    let (a, b) = (unit(got), unit(want));
    (0..3).all(|i| (a[i] - b[i]).abs() < DIRECTION_TOLERANCE)
}

fn unit(v: [f64; 3]) -> [f64; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    v.map(|c| c / length)
}

fn centre_of(header: &Value) -> [f64; 2] {
    [header["center"][0].as_f64().unwrap_or(0.0), header["center"][1].as_f64().unwrap_or(0.0)]
}

pub(crate) fn js_integer(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::as_f64).filter(|n| n.fract() == 0.0)
}

pub(crate) fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64() != Some(0.0),
        Some(Value::String(s)) => !s.is_empty(),
        Some(_) => true,
    }
}

pub(crate) fn js_num(n: f64) -> String {
    if n.is_nan() {
        "NaN".to_string()
    } else if n.is_infinite() {
        if n > 0.0 { "Infinity" } else { "-Infinity" }.to_string()
    } else if n == 0.0 {
        "0".to_string()
    } else {
        format!("{n}")
    }
}

pub(crate) fn js_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => js_json(other),
    }
}

pub(crate) fn js_json(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js_num(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => serde_json::to_string(s).unwrap_or_default(),
        Value::Array(items) => format!("[{}]", items.iter().map(js_json).collect::<Vec<_>>().join(",")),
        Value::Object(map) => format!(
            "{{{}}}",
            map.iter()
                .map(|(k, v)| format!("{}:{}", serde_json::to_string(k).unwrap_or_default(), js_json(v)))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

pub(crate) fn js_show(v: Option<&Value>) -> String {
    v.map_or_else(|| "undefined".to_string(), js_text)
}

pub(crate) fn js_json_or_null(v: Option<&Value>) -> String {
    js_json(v.unwrap_or(&Value::Null))
}

impl LatticeSurface {
    pub fn from_field(field: baked::BakedHeightField, header: &Value) -> Option<Self> {
        let n = js_integer(header.get("N")).filter(|n| *n >= 2.0)? as usize;
        let extent = header.get("extent").and_then(Value::as_f64).filter(|e| e.is_finite())?;
        Some(Self {
            field,
            n,
            extent,
            center: centre_of(header),
            chart_epoch: baked_epoch_of(header),
        })
    }

    pub fn n(&self) -> usize {
        self.n
    }

    pub fn extent(&self) -> f64 {
        self.extent
    }

    pub fn step_m(&self) -> f64 {
        self.extent / (self.n - 1) as f64
    }

    pub fn chart_epoch(&self) -> f64 {
        self.chart_epoch
    }

    pub fn is_current(&self, live_epoch: f64) -> bool {
        live_epoch == self.chart_epoch
    }

    pub fn covers(&self, x: f64, z: f64, live_epoch: f64) -> bool {
        let half = self.extent / 2.0;
        self.is_current(live_epoch) && (x - self.center[0]).abs() <= half && (z - self.center[1]).abs() <= half
    }

    pub fn height_at_local(&self, x: f64, z: f64, live_epoch: f64) -> f64 {
        if !self.is_current(live_epoch) {
            return f64::NAN;
        }
        self.field.height_at_local(x, z)
    }
}

#[cfg(test)]
pub(crate) mod witness {
    use super::LatticeTerrain;
    use serde_json::Value;

    pub fn num(v: &Value) -> f64 {
        match v.as_str() {
            Some("NaN") => f64::NAN,
            _ => v.as_f64().unwrap(),
        }
    }

    pub fn bytes_of(hex: &str) -> Vec<u8> {
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect()
    }

    pub fn assert_close(got: f64, want: f64, what: &str) {
        assert!(
            (got.is_nan() && want.is_nan()) || (got - want).abs() <= 1e-9,
            "{what}: bevy {got} against spoint {want}"
        );
    }

    pub fn terrain(t: &Value) -> LatticeTerrain {
        LatticeTerrain {
            hash_version: t["hashVersion"].as_u64().unwrap(),
            bake_key: t["bakeKey"].as_str().unwrap().to_string(),
            bake_slice_text: t["bakeSliceText"].as_str().unwrap().to_string(),
            radius: t["radius"].as_f64().unwrap(),
            relief_scale: t["reliefScale"].as_f64(),
            anchor_dir: [0_usize, 1, 2].map(|i| t["anchorDir"][i].as_f64().unwrap()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::witness::{assert_close, bytes_of, num, terrain};
    use super::*;

    #[test]
    fn lattice_surface_matches_spoint_load_lattice_surface() {
        let fixture: Value = serde_json::from_str(include_str!("../testdata/parity/spoint-lattice-surface.json")).unwrap();
        let terrain = terrain(&fixture["terrain"]);
        for case in fixture["cases"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let frame_epoch = num(&case["frameEpoch"]);
            let got = load_lattice_surface(&bytes_of(case["bytesHex"].as_str().unwrap()), &terrain, frame_epoch);
            let want_loaded = case["loaded"].as_bool().unwrap();
            assert_eq!(got.is_some(), want_loaded, "{name}: loaded, spoint says {want_loaded}");
            let Some(surface) = got else {
                continue;
            };
            assert_eq!(surface.n() as u64, case["N"].as_u64().unwrap(), "{name}: N");
            assert_close(surface.extent(), num(&case["extent"]), &format!("{name}: extent"));
            assert_close(surface.step_m(), num(&case["stepM"]), &format!("{name}: stepM"));
            assert_close(surface.chart_epoch(), num(&case["chartEpoch"]), &format!("{name}: chartEpoch"));
            let stale = frame_epoch + 2.0;
            let probes = case["probes"].as_array().unwrap();
            let stale_probes = case["staleProbes"].as_array().unwrap();
            assert_eq!(probes.len(), stale_probes.len(), "{name}: probe count");
            for (k, (probe, stale_probe)) in probes.iter().zip(stale_probes).enumerate() {
                let (x, z) = (num(&probe["x"]), num(&probe["z"]));
                assert_eq!(surface.covers(x, z, frame_epoch), probe["covers"].as_bool().unwrap(), "{name}: probe {k} covers");
                assert_close(surface.height_at_local(x, z, frame_epoch), num(&probe["height"]), &format!("{name}: probe {k} height"));
                assert_eq!(surface.covers(x, z, stale), stale_probe["covers"].as_bool().unwrap(), "{name}: stale probe {k} covers");
                assert_close(surface.height_at_local(x, z, stale), num(&stale_probe["height"]), &format!("{name}: stale probe {k} height"));
            }
        }
    }

    #[test]
    fn baked_terrain_mismatch_matches_spoint_bake_checks() {
        let fixture: Value = serde_json::from_str(include_str!("../testdata/parity/spoint-lattice-surface.json")).unwrap();
        let terrain = terrain(&fixture["terrain"]);
        for case in fixture["cases"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let decoded = case["decoded"].as_bool().unwrap();
            let bytes = bytes_of(case["bytesHex"].as_str().unwrap());
            let Ok((header, _)) = baked::split_hf(&bytes) else {
                assert!(!decoded, "{name}: spoint decodes it, bevy does not");
                continue;
            };
            assert!(decoded, "{name}: bevy decodes it, spoint does not");
            assert_eq!(baked_epoch_of(&header), num(&case["bakedEpoch"]), "{name}: bakedEpoch");
            assert_eq!(baked_hash_version_of(&header), case["bakedHashVersion"].as_u64().unwrap(), "{name}: bakedHashVersion");
            let got = baked_terrain_mismatch(&header, &terrain, num(&case["frameEpoch"]));
            assert_eq!(got.as_deref(), case["mismatch"].as_str(), "{name}: mismatch");
        }
    }
}
