#![cfg_attr(not(test), allow(dead_code))]

use crate::height::baked;

use serde_json::Value;

const LEGACY_HASH_VERSION: u64 = 1;
const DIRECTION_TOLERANCE: f64 = 1e-6;

pub struct LatticeTerrain {
    pub hash_version: u64,
    pub bake_key: String,
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
    let n = header["N"].as_u64().filter(|&n| n >= 2)? as usize;
    let extent = header["extent"].as_f64().filter(|e| e.is_finite())?;
    if header["hashVersion"].as_u64().unwrap_or(LEGACY_HASH_VERSION) != terrain.hash_version {
        return None;
    }
    if !matches_terrain(&header, terrain, live_epoch) {
        return None;
    }
    Some(LatticeSurface {
        field,
        n,
        extent,
        center: centre_of(&header),
        chart_epoch: chart_epoch_of(&header),
    })
}

impl LatticeSurface {
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

fn chart_epoch_of(header: &Value) -> f64 {
    header["chartEpoch"].as_f64().unwrap_or(0.0)
}

fn centre_of(header: &Value) -> [f64; 2] {
    [header["center"][0].as_f64().unwrap_or(0.0), header["center"][1].as_f64().unwrap_or(0.0)]
}

fn matches_terrain(header: &Value, terrain: &LatticeTerrain, live_epoch: f64) -> bool {
    if chart_epoch_of(header) != live_epoch {
        return false;
    }
    match header.get("terrainKey") {
        Some(key) => key.as_str() == Some(terrain.bake_key.as_str()),
        None => {
            header["radius"].as_f64() == Some(terrain.radius)
                && header["reliefScale"].as_f64() == terrain.relief_scale
                && same_direction(&header["anchorDir"], terrain.anchor_dir)
        }
    }
}

fn same_direction(artifact: &Value, want: [f64; 3]) -> bool {
    let Some(values) = artifact.as_array().filter(|v| v.len() == 3) else {
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

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f64 = 1e-9;

    fn num(v: &Value) -> f64 {
        match v.as_str() {
            Some("NaN") => f64::NAN,
            _ => v.as_f64().unwrap(),
        }
    }

    fn bytes_of(hex: &str) -> Vec<u8> {
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect()
    }

    fn assert_close(got: f64, want: f64, what: &str) {
        assert!(
            (got.is_nan() && want.is_nan()) || (got - want).abs() <= TOLERANCE,
            "{what}: bevy {got} against spoint {want}"
        );
    }

    #[test]
    fn lattice_surface_matches_spoint_load_lattice_surface() {
        let fixture: Value = serde_json::from_str(include_str!("../testdata/parity/spoint-lattice-surface.json")).unwrap();
        let t = &fixture["terrain"];
        let terrain = LatticeTerrain {
            hash_version: t["hashVersion"].as_u64().unwrap(),
            bake_key: t["bakeKey"].as_str().unwrap().to_string(),
            radius: t["radius"].as_f64().unwrap(),
            relief_scale: t["reliefScale"].as_f64(),
            anchor_dir: [0_usize, 1, 2].map(|i| t["anchorDir"][i].as_f64().unwrap()),
        };
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
}
