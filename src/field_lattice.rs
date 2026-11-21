#![cfg_attr(not(test), allow(dead_code))]

use crate::character::ChartTransfer;
use crate::chart_transfer::{create_chart_transfer, ChartEpochTransfer, ChartSnapshot};
use crate::height::planet_frame::{anchor_basis, ChartFrame, SurfaceSolve};

pub const RESET_SURFACE_STEP_BOUND_M: f64 = 0.02;
const RAD_TO_DEG: f64 = 180.0 / std::f64::consts::PI;
const GROUND_FALLBACK_Y: f64 = 0.0;

pub fn tilt_deg_between(up_a: [f64; 3], up_b: [f64; 3]) -> f64 {
    let d = up_a[0] * up_b[0] + up_a[1] * up_b[1] + up_a[2] * up_b[2];
    d.clamp(-1.0, 1.0).acos() * RAD_TO_DEG
}

fn chart_snapshot<H: Fn([f64; 3]) -> f64>(radius: f64, offset_y: f64, dir: [f64; 3], height: H) -> ChartSnapshot {
    let basis = anchor_basis(dir);
    ChartSnapshot {
        radius,
        offset_y,
        anchor_height: height(basis.up),
        east: basis.east,
        up: basis.up,
        north: basis.north,
    }
}

fn finite_or_ground_fallback(y: f64) -> f64 {
    if y.is_finite() {
        y
    } else {
        GROUND_FALLBACK_Y
    }
}

fn transfer_matrix(transfer: &ChartEpochTransfer) -> [f64; 9] {
    let mut m = [0.0; 9];
    for c in 0..3 {
        let mut e = [0.0; 3];
        e[c] = 1.0;
        let col = transfer.vec(e);
        for r in 0..3 {
            m[r * 3 + c] = col[r];
        }
    }
    m
}

fn rotation_to_quat(m: [f64; 9]) -> [f64; 4] {
    let tr = m[0] + m[4] + m[8];
    let (x, y, z, w);
    if tr > 0.0 {
        let s = (tr + 1.0).sqrt() * 2.0;
        w = 0.25 * s;
        x = (m[7] - m[5]) / s;
        y = (m[2] - m[6]) / s;
        z = (m[3] - m[1]) / s;
    } else if m[0] > m[4] && m[0] > m[8] {
        let s = (1.0 + m[0] - m[4] - m[8]).sqrt() * 2.0;
        w = (m[7] - m[5]) / s;
        x = 0.25 * s;
        y = (m[1] + m[3]) / s;
        z = (m[2] + m[6]) / s;
    } else if m[4] > m[8] {
        let s = (1.0 + m[4] - m[0] - m[8]).sqrt() * 2.0;
        w = (m[2] - m[6]) / s;
        x = (m[1] + m[3]) / s;
        y = 0.25 * s;
        z = (m[5] + m[7]) / s;
    } else {
        let s = (1.0 + m[8] - m[0] - m[4]).sqrt() * 2.0;
        w = (m[3] - m[1]) / s;
        x = (m[2] + m[6]) / s;
        y = (m[5] + m[7]) / s;
        z = 0.25 * s;
    }
    let l = (x * x + y * y + z * z + w * w).sqrt();
    [x / l, y / l, z / l, w / l]
}

pub struct FieldLattice<S, L> {
    anchor: ChartSnapshot,
    live: ChartSnapshot,
    frame: ChartFrame,
    sampler: S,
    live_height: L,
    lattice_to_chart: ChartEpochTransfer,
    chart_to_lattice: ChartEpochTransfer,
    lattice_to_chart_quat: [f64; 4],
}

pub fn create_field_lattice<S, L>(
    radius: f64,
    offset_y: f64,
    relief_scale: f64,
    anchor_dir: [f64; 3],
    live_dir: [f64; 3],
    sampler: S,
    live_height: L,
) -> FieldLattice<S, L>
where
    S: Fn([f64; 3]) -> f64,
    L: Fn(f64, f64) -> f64,
{
    let frame = ChartFrame::new(anchor_dir, radius, offset_y, Some(relief_scale), &sampler);
    let anchor = chart_snapshot(radius, offset_y, anchor_dir, &sampler);
    let live = chart_snapshot(radius, offset_y, live_dir, &sampler);
    let lattice_to_chart = create_chart_transfer(&anchor, &live);
    let chart_to_lattice = create_chart_transfer(&live, &anchor);
    let lattice_to_chart_quat = rotation_to_quat(transfer_matrix(&lattice_to_chart));
    FieldLattice {
        anchor,
        live,
        frame,
        sampler,
        live_height,
        lattice_to_chart,
        chart_to_lattice,
        lattice_to_chart_quat,
    }
}

impl<S, L> FieldLattice<S, L>
where
    S: Fn([f64; 3]) -> f64,
    L: Fn(f64, f64) -> f64,
{
    pub fn anchor(&self) -> &ChartSnapshot {
        &self.anchor
    }

    pub fn live(&self) -> &ChartSnapshot {
        &self.live
    }

    pub fn tilt_deg(&self) -> f64 {
        tilt_deg_between(self.anchor.up, self.live.up)
    }

    pub fn placement(&self, corner_x: f64, corner_z: f64) -> ([f64; 3], [f64; 4]) {
        (self.lattice_to_chart.point([corner_x, 0.0, corner_z]), self.lattice_to_chart_quat)
    }

    pub fn ground_height(&mut self, x: f64, z: f64) -> Result<f64, SurfaceSolve> {
        let sampler = &self.sampler;
        self.frame.ground_height_local(x, z, |d| Some(sampler(d)))
    }

    pub fn to_lattice(&self, x: f64, z: f64) -> [f64; 2] {
        let y = finite_or_ground_fallback((self.live_height)(x, z));
        let p = self.chart_to_lattice.point([x, y, z]);
        [p[0], p[2]]
    }

    pub fn to_chart(&mut self, x: f64, z: f64) -> Result<[f64; 2], SurfaceSolve> {
        let y = self.ground_height(x, z)?;
        let p = self.lattice_to_chart.point([x, finite_or_ground_fallback(y), z]);
        Ok([p[0], p[2]])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart_transfer::fixture_snapshot;
    use serde_json::Value;

    const TOLERANCE: f64 = 1e-9;

    fn sampler(d: [f64; 3]) -> f64 {
        0.5 + 0.25 * d[0] - 0.1 * d[2]
    }

    fn live_height(x: f64, z: f64) -> f64 {
        0.3 + 0.05 * x - 0.02 * z
    }

    fn json_vec(v: &Value) -> Vec<f64> {
        v.as_array().unwrap().iter().map(|c| c.as_f64().unwrap()).collect()
    }

    fn json_vec3(v: &Value) -> [f64; 3] {
        let v = json_vec(v);
        [v[0], v[1], v[2]]
    }

    fn assert_close(got: f64, want: f64, what: &str) {
        assert!((got - want).abs() <= TOLERANCE, "{what}: {got} against spoint {want}");
    }

    fn assert_close_all(got: &[f64], want: &[f64], what: &str) {
        assert_eq!(got.len(), want.len(), "{what} length");
        for (k, (g, w)) in got.iter().zip(want).enumerate() {
            assert_close(*g, *w, &format!("{what}[{k}]"));
        }
    }

    fn assert_snapshot(got: &ChartSnapshot, want: &ChartSnapshot, what: &str) {
        assert_close(got.radius, want.radius, &format!("{what}.radius"));
        assert_close(got.offset_y, want.offset_y, &format!("{what}.offsetY"));
        assert_close(got.anchor_height, want.anchor_height, &format!("{what}.anchorHeight"));
        assert_close_all(&got.east, &want.east, &format!("{what}.east"));
        assert_close_all(&got.up, &want.up, &format!("{what}.up"));
        assert_close_all(&got.north, &want.north, &format!("{what}.north"));
    }

    #[test]
    fn field_lattice_matches_spoint_create_field_lattice() {
        let fixture: Value = serde_json::from_str(include_str!("../testdata/parity/spoint-field-lattice.json")).unwrap();
        let mut lattice = create_field_lattice(
            fixture["planetRadius"].as_f64().unwrap(),
            fixture["offsetY"].as_f64().unwrap(),
            fixture["reliefScale"].as_f64().unwrap(),
            json_vec3(&fixture["anchorUp"]),
            json_vec3(&fixture["liveDir"]),
            sampler,
            live_height,
        );
        assert_snapshot(lattice.anchor(), &fixture_snapshot(&fixture["anchor"]), "anchor");
        assert_snapshot(lattice.live(), &fixture_snapshot(&fixture["live"]), "live");
        assert_close(lattice.tilt_deg(), fixture["tiltDeg"].as_f64().unwrap(), "tiltDeg");
        for case in fixture["placements"].as_array().unwrap() {
            let corner = json_vec(&case["corner"]);
            let (position, rotation) = lattice.placement(corner[0], corner[1]);
            assert_close_all(&position, &json_vec(&case["position"]), "placement.position");
            assert_close_all(&rotation, &json_vec(&case["rotation"]), "placement.rotation");
        }
        for case in fixture["toLattice"].as_array().unwrap() {
            let point = json_vec(&case["point"]);
            assert_close_all(&lattice.to_lattice(point[0], point[1]), &json_vec(&case["out"]), "toLattice");
        }
        for case in fixture["toChart"].as_array().unwrap() {
            let point = json_vec(&case["point"]);
            let out = lattice.to_chart(point[0], point[1]).unwrap();
            assert_close_all(&out, &json_vec(&case["out"]), "toChart");
        }
        for case in fixture["heightFn"].as_array().unwrap() {
            let point = json_vec(&case["point"]);
            let height = lattice.ground_height(point[0], point[1]).unwrap();
            assert_close(height, case["height"].as_f64().unwrap(), "heightFn");
        }
    }

    #[test]
    fn field_lattice_tilt_deg_and_reset_bound_match_spoint() {
        let fixture: Value = serde_json::from_str(include_str!("../testdata/parity/spoint-field-lattice.json")).unwrap();
        assert_close(RESET_SURFACE_STEP_BOUND_M, fixture["resetSurfaceStepBoundM"].as_f64().unwrap(), "RESET_SURFACE_STEP_BOUND_M");
        for case in fixture["tiltDegBetween"].as_array().unwrap() {
            assert_close(tilt_deg_between(json_vec3(&case["a"]), json_vec3(&case["b"])), case["deg"].as_f64().unwrap(), "tiltDegBetween");
        }
    }
}
