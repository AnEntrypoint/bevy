
use crate::height::ops_jsgen::V3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SeaLevelChart {
    pub radius: f64,
    pub east: V3,
    pub up: V3,
    pub north: V3,
}

pub fn sea_level_dir_of(chart: &SeaLevelChart, x: f64, z: f64) -> V3 {
    let r2 = x * x + z * z;
    let lift = chart.radius - r2 / (chart.radius + (chart.radius * chart.radius - r2).sqrt());
    let a = [0, 1, 2].map(|k| chart.up[k] * lift + chart.east[k] * x + chart.north[k] * z);
    let length = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    [a[0] / length, a[1] / length, a[2] / length]
}

pub fn sea_level_xz_of_dir(chart: &SeaLevelChart, dir: V3) -> [f64; 2] {
    let along = |axis: V3| dir[0] * axis[0] + dir[1] * axis[1] + dir[2] * axis[2];
    [chart.radius * along(chart.east), chart.radius * along(chart.north)]
}

pub fn reanchored_sea_level_xz(from: &SeaLevelChart, to: &SeaLevelChart, x: f64, z: f64) -> [f64; 2] {
    sea_level_xz_of_dir(to, sea_level_dir_of(from, x, z))
}

mod tests {
    use super::*;
    use serde_json::Value;

    const TOLERANCE: f64 = 1e-9;

    fn fixture() -> Value {
        serde_json::from_str(include_str!("../testdata/parity/spoint-chart-local-point.json")).unwrap()
    }

    fn vec3(v: &Value) -> V3 {
        [v[0].as_f64().unwrap(), v[1].as_f64().unwrap(), v[2].as_f64().unwrap()]
    }

    fn chart_of(v: &Value) -> SeaLevelChart {
        SeaLevelChart { radius: v["radius"].as_f64().unwrap(), east: vec3(&v["east"]), up: vec3(&v["up"]), north: vec3(&v["north"]) }
    }

    fn assert_close(got: &[f64], want: &Value, what: &str) {
        let want: Vec<f64> = want.as_array().unwrap().iter().map(|c| c.as_f64().unwrap()).collect();
        assert_eq!(got.len(), want.len(), "{what}: length");
        for (g, w) in got.iter().zip(&want) {
            assert!((g - w).abs() <= TOLERANCE, "{what}: {got:?} against spoint {want:?}");
        }
    }

    #[test]
    fn sea_level_mapping_matches_spoint_chart_local_point() {
        let fixture = fixture();
        let from = chart_of(&fixture["from"]);
        let to = chart_of(&fixture["to"]);
        let grid = fixture["grid"].as_array().unwrap();
        assert_eq!(grid.len(), 25);
        for row in grid {
            let (x, z) = (row["x"].as_f64().unwrap(), row["z"].as_f64().unwrap());
            let dir = sea_level_dir_of(&from, x, z);
            assert_close(&dir, &row["dir"], "dir");
            assert_close(&sea_level_xz_of_dir(&from, dir), &row["xz"], "xz");
            assert_close(&reanchored_sea_level_xz(&from, &to, x, z), &row["reanchoredXZ"], "reanchored xz");
        }
        for probe in fixture["dirs"].as_array().unwrap() {
            let dir = vec3(&probe["dir"]);
            assert_close(&sea_level_xz_of_dir(&from, dir), &probe["fromXZ"], "from xz");
            assert_close(&sea_level_xz_of_dir(&to, dir), &probe["toXZ"], "to xz");
        }
    }
}
