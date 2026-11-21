#![cfg_attr(not(test), allow(dead_code))]

use crate::character::ChartTransfer;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChartSnapshot {
    pub radius: f64,
    pub offset_y: f64,
    pub anchor_height: f64,
    pub east: [f64; 3],
    pub up: [f64; 3],
    pub north: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChartEpochTransfer {
    m: [f64; 9],
    from_lift: f64,
    to_lift: f64,
    tilt_rad: f64,
}

pub fn create_chart_transfer(from: &ChartSnapshot, to: &ChartSnapshot) -> ChartEpochTransfer {
    let from_axes = [from.east, from.up, from.north];
    let to_axes = [to.east, to.up, to.north];
    let mut m = [0.0; 9];
    for r in 0..3 {
        for c in 0..3 {
            m[r * 3 + c] = to_axes[r][0] * from_axes[c][0] + to_axes[r][1] * from_axes[c][1] + to_axes[r][2] * from_axes[c][2];
        }
    }
    ChartEpochTransfer {
        m,
        from_lift: from.radius + from.anchor_height - from.offset_y,
        to_lift: to.radius + to.anchor_height - to.offset_y,
        tilt_rad: m[4].clamp(-1.0, 1.0).acos(),
    }
}

impl ChartTransfer for ChartEpochTransfer {
    fn point(&self, p: [f64; 3]) -> [f64; 3] {
        let m = &self.m;
        let (a, b, c) = (p[0], p[1] + self.from_lift, p[2]);
        [
            m[0] * a + m[1] * b + m[2] * c,
            m[3] * a + m[4] * b + m[5] * c - self.to_lift,
            m[6] * a + m[7] * b + m[8] * c,
        ]
    }

    fn vec(&self, v: [f64; 3]) -> [f64; 3] {
        let m = &self.m;
        [
            m[0] * v[0] + m[1] * v[1] + m[2] * v[2],
            m[3] * v[0] + m[4] * v[1] + m[5] * v[2],
            m[6] * v[0] + m[7] * v[1] + m[8] * v[2],
        ]
    }

    fn tilt_rad(&self) -> f64 {
        self.tilt_rad
    }
}

#[cfg(test)]
fn json_vec3(v: &serde_json::Value) -> [f64; 3] {
    [v[0].as_f64().unwrap(), v[1].as_f64().unwrap(), v[2].as_f64().unwrap()]
}

#[cfg(test)]
pub fn fixture_snapshot(v: &serde_json::Value) -> ChartSnapshot {
    ChartSnapshot {
        radius: v["radius"].as_f64().unwrap(),
        offset_y: v["offsetY"].as_f64().unwrap(),
        anchor_height: v["anchorHeight"].as_f64().unwrap(),
        east: json_vec3(&v["east"]),
        up: json_vec3(&v["up"]),
        north: json_vec3(&v["north"]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const TOLERANCE: f64 = 1e-9;

    fn assert_close(got: [f64; 3], want: [f64; 3], what: &str) {
        for k in 0..3 {
            assert!((got[k] - want[k]).abs() <= TOLERANCE, "{what}: {got:?} against spoint {want:?}");
        }
    }

    #[test]
    fn chart_epoch_transfer_matches_spoint_create_chart_transfer() {
        let fixture: Value = serde_json::from_str(include_str!("../testdata/parity/spoint-chart-transfer.json")).unwrap();
        let transfer = create_chart_transfer(&fixture_snapshot(&fixture["from"]), &fixture_snapshot(&fixture["to"]));
        assert!((transfer.tilt_rad() - fixture["tiltRad"].as_f64().unwrap()).abs() <= TOLERANCE);
        let probes = &fixture["probes"];
        for (p, want) in probes["points"].as_array().unwrap().iter().zip(probes["pointResults"].as_array().unwrap()) {
            assert_close(transfer.point(json_vec3(p)), json_vec3(want), "point");
        }
        for (v, want) in probes["vectors"].as_array().unwrap().iter().zip(probes["vectorResults"].as_array().unwrap()) {
            assert_close(transfer.vec(json_vec3(v)), json_vec3(want), "vec");
        }
    }
}
