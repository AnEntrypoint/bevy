#![cfg_attr(not(test), allow(dead_code))]

use std::collections::HashMap;

use crate::height::ops_jsgen::V3;
use crate::height::placement::PlacementLattice;

pub const SECTOR_M: f64 = 8.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Climate {
    pub temp: f64,
    pub humidity: f64,
    pub erosion: f64,
    pub sea_bias: f64,
}

pub trait ClimateField {
    fn sample_dir(&mut self, dir: V3) -> Option<Climate>;
}

pub trait PlanetFrame {
    fn radius(&self) -> f64;
    fn local_to_dir(&self, x: f64, z: f64) -> V3;
}

pub struct CachedAnchorField<F: ClimateField, P: PlanetFrame> {
    field: F,
    frame: P,
    lattice: PlacementLattice,
    values: HashMap<u64, Option<Climate>>,
}

impl<F: ClimateField, P: PlanetFrame> CachedAnchorField<F, P> {
    pub fn new(field: F, frame: P) -> Result<Self, String> {
        let lattice = PlacementLattice::new(frame.radius(), SECTOR_M, 1.0)?;
        Ok(Self { field, frame, lattice, values: HashMap::new() })
    }

    pub fn climate_at_local(&mut self, x: f64, z: f64, dir: Option<V3>) -> Option<Climate> {
        let dir = dir.unwrap_or_else(|| self.frame.local_to_dir(x, z));
        self.climate_at_dir(dir)
    }

    fn climate_at_dir(&mut self, dir: V3) -> Option<Climate> {
        let key = self.lattice.chunk_key_of_dir(dir);
        if let Some(cached) = self.values.get(&(key as u64)) {
            return *cached;
        }
        let centre = self.lattice.chunk_centre_dir(key);
        let sampled = self.field.sample_dir(centre);
        self.values.insert(key as u64, sampled);
        sampled
    }

    pub fn size(&self) -> usize {
        self.values.len()
    }

    pub fn field(&self) -> &F {
        &self.field
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const TOLERANCE: f64 = 1e-9;

    struct SyntheticFrame {
        radius: f64,
    }

    impl PlanetFrame for SyntheticFrame {
        fn radius(&self) -> f64 {
            self.radius
        }

        fn local_to_dir(&self, x: f64, z: f64) -> V3 {
            let v = [x / self.radius, 1.0, z / self.radius];
            let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            [v[0] / l, v[1] / l, v[2] / l]
        }
    }

    #[derive(Default)]
    struct CountingField {
        calls: usize,
    }

    impl ClimateField for CountingField {
        fn sample_dir(&mut self, dir: V3) -> Option<Climate> {
            self.calls += 1;
            if dir[0] < -0.02 {
                return None;
            }
            Some(Climate {
                temp: 15.0 + 20.0 * dir[1],
                humidity: 0.5 + 0.3 * dir[2],
                erosion: 0.2 + 0.1 * dir[0],
                sea_bias: dir[0] * dir[1],
            })
        }
    }

    fn number(value: &Value) -> f64 {
        value.as_f64().unwrap()
    }

    fn assert_close(label: &str, got: f64, want: f64) {
        assert!((got - want).abs() <= TOLERANCE, "{label}: bevy {got} against spoint {want}");
    }

    fn assert_climate(label: &str, got: Option<Climate>, want: &Value) {
        match (got, want.is_null()) {
            (None, true) => {}
            (Some(g), false) => {
                assert_close(&format!("{label} temp"), g.temp, number(&want["temp"]));
                assert_close(&format!("{label} humidity"), g.humidity, number(&want["humidity"]));
                assert_close(&format!("{label} erosion"), g.erosion, number(&want["erosion"]));
                assert_close(&format!("{label} seaBias"), g.sea_bias, number(&want["seaBias"]));
            }
            (got, _) => panic!("{label}: bevy {got:?} against spoint {want}"),
        }
    }

    #[test]
    fn cached_anchor_field_matches_spoint() {
        let fixture: Value = serde_json::from_str(include_str!("../testdata/parity/spoint-climate-cache.json")).unwrap();
        assert_eq!(number(&fixture["sectorM"]), SECTOR_M);
        let radius = number(&fixture["radius"]);
        let mut cached = CachedAnchorField::new(CountingField::default(), SyntheticFrame { radius }).unwrap();
        for (i, query) in fixture["queries"].as_array().unwrap().iter().enumerate() {
            let dir = query["dir"].as_array().map(|d| [number(&d[0]), number(&d[1]), number(&d[2])]);
            let got = cached.climate_at_local(number(&query["x"]), number(&query["z"]), dir);
            assert_climate(&format!("query {i} climate"), got, &query["climate"]);
            assert_eq!(cached.size() as f64, number(&query["size"]), "query {i} cache size");
            assert_eq!(cached.field().calls as f64, number(&query["calls"]), "query {i} sampleDir calls");
        }
    }
}
