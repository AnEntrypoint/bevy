
use std::f64::consts::PI;

use crate::chart_transfer::{create_chart_transfer, ChartEpochTransfer, ChartSnapshot};
use crate::height::ops_jsgen::V3;
use crate::height::placement::PlacementLattice;
use crate::height::planet_frame::anchor_basis;

const QUARTER_TURN: f64 = PI / 2.0;
const CHUNK_M_SAFETY: f64 = 1.0 + 1e-12;
const RAD_TO_DEG: f64 = 180.0 / PI;
const SAME_ANCHOR_SQ_TOLERANCE: f64 = 1e-18;

pub const CHART_ANCHORS_PER_FACE: f64 = 32.0;
pub const CHART_REANCHOR_ANGLE_DEG: f64 = 28.0;

#[derive(Clone, Debug, PartialEq)]
pub struct ChartAnchorLatticeError {
    pub reasons: Vec<&'static str>,
}

impl ChartAnchorLatticeError {
    pub fn reason(&self) -> String {
        self.reasons.join("+")
    }
}

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn create_chart_anchor_lattice(radius: f64, anchors_per_face: f64) -> Result<PlacementLattice, ChartAnchorLatticeError> {
    let mut reasons = Vec::new();
    if !(radius.is_finite() && radius > 0.0) {
        reasons.push("radius-not-positive-finite");
    }
    if !(anchors_per_face >= 1.0) {
        reasons.push("anchors-per-face-below-one");
    }
    if !reasons.is_empty() {
        return Err(ChartAnchorLatticeError { reasons });
    }
    let chunk_m = (QUARTER_TURN * radius / anchors_per_face) * CHUNK_M_SAFETY;
    Ok(PlacementLattice::new(radius, chunk_m, 1.0).expect("a positive radius and anchors per face >= 1 give a positive chunk size"))
}

pub fn chart_anchor_key_of_dir(lattice: &PlacementLattice, dir: V3) -> f64 {
    lattice.chunk_key_of_dir(dir)
}

pub fn chart_anchor_dir_of_key(lattice: &PlacementLattice, key: f64) -> V3 {
    lattice.chunk_centre_dir(key)
}

pub fn chart_anchor_for_dir(lattice: &PlacementLattice, dir: V3) -> V3 {
    chart_anchor_dir_of_key(lattice, chart_anchor_key_of_dir(lattice, dir))
}

pub fn chart_anchor_angle_deg(up: V3, dir: V3) -> f64 {
    let length = dot(dir, dir).sqrt();
    let length = if length > 0.0 { length } else { 1.0 };
    (dot(dir, up) / length).clamp(-1.0, 1.0).acos() * RAD_TO_DEG
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChartAnchorSnapshot {
    pub radius: f64,
    pub offset_y: f64,
    pub anchor_height: f64,
    pub chart_epoch: u32,
    pub east: V3,
    pub up: V3,
    pub north: V3,
}

impl ChartAnchorSnapshot {
    pub fn transfer_snapshot(&self) -> ChartSnapshot {
        ChartSnapshot {
            radius: self.radius,
            offset_y: self.offset_y,
            anchor_height: self.anchor_height,
            east: self.east,
            up: self.up,
            north: self.north,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChartAnchorFrame {
    state: ChartAnchorSnapshot,
}

impl ChartAnchorFrame {
    pub fn new(anchor_dir: V3, radius: f64, offset_y: f64, height: impl Fn(V3) -> f64) -> Self {
        let basis = anchor_basis(anchor_dir);
        Self {
            state: ChartAnchorSnapshot {
                radius,
                offset_y,
                anchor_height: height(basis.up),
                chart_epoch: 0,
                east: basis.east,
                up: basis.up,
                north: basis.north,
            },
        }
    }

    pub fn up(&self) -> V3 {
        self.state.up
    }

    pub fn snapshot(&self) -> ChartAnchorSnapshot {
        self.state
    }

    pub fn reanchor(&mut self, dir: V3, height: impl Fn(V3) -> f64) {
        let basis = anchor_basis(dir);
        self.state.east = basis.east;
        self.state.up = basis.up;
        self.state.north = basis.north;
        self.state.anchor_height = height(basis.up);
        self.state.chart_epoch += 1;
    }
}

pub fn snapshot_chart(frame: &ChartAnchorFrame) -> ChartAnchorSnapshot {
    frame.snapshot()
}

pub fn chart_needs_reanchor(frame: &ChartAnchorFrame, dir: V3, threshold_deg: f64) -> bool {
    chart_anchor_angle_deg(frame.up(), dir) > threshold_deg
}

pub fn chart_needs_reanchor_for_cell(lattice: &PlacementLattice, frame: &ChartAnchorFrame, dir: V3) -> bool {
    let offset = sub(chart_anchor_for_dir(lattice, dir), frame.up());
    dot(offset, offset) > SAME_ANCHOR_SQ_TOLERANCE
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartReanchor {
    pub from: ChartAnchorSnapshot,
    pub to: ChartAnchorSnapshot,
    pub anchor_dir: V3,
    pub angle_before_deg: f64,
    pub angle_after_deg: f64,
    pub transfer: ChartEpochTransfer,
}

pub fn reanchor_chart_for(frame: &mut ChartAnchorFrame, lattice: &PlacementLattice, dir: V3, threshold_deg: f64, cell_trigger: bool, height: impl Fn(V3) -> f64) -> Option<ChartReanchor> {
    let needed = if cell_trigger {
        chart_needs_reanchor_for_cell(lattice, frame, dir)
    } else {
        chart_needs_reanchor(frame, dir, threshold_deg)
    };
    if !needed {
        return None;
    }
    let from = snapshot_chart(frame);
    frame.reanchor(chart_anchor_for_dir(lattice, dir), height);
    let to = snapshot_chart(frame);
    Some(ChartReanchor {
        from,
        to,
        anchor_dir: to.up,
        angle_before_deg: chart_anchor_angle_deg(from.up, dir),
        angle_after_deg: chart_anchor_angle_deg(to.up, dir),
        transfer: create_chart_transfer(&from.transfer_snapshot(), &to.transfer_snapshot()),
    })
}

mod tests {
    use super::*;
    use crate::character::ChartTransfer;
    use serde_json::Value;

    const TOLERANCE: f64 = 1e-9;
    const ANGLE_TOLERANCE_DEG: f64 = 1e-6;

    fn fixture() -> Value {
        serde_json::from_str(include_str!("../testdata/parity/spoint-chart-anchor.json")).unwrap()
    }

    fn vec3(v: &Value) -> V3 {
        [v[0].as_f64().unwrap(), v[1].as_f64().unwrap(), v[2].as_f64().unwrap()]
    }

    fn height_at(d: V3) -> f64 {
        0.5 + 0.25 * d[0] - 0.1 * d[2]
    }

    fn lattice_for(f: &Value) -> PlacementLattice {
        create_chart_anchor_lattice(f["planetRadius"].as_f64().unwrap(), f["constants"]["anchorsPerFace"].as_f64().unwrap()).unwrap()
    }

    fn assert_close(got: f64, want: &Value, what: &str) {
        let want = want.as_f64().unwrap();
        assert!((got - want).abs() <= TOLERANCE, "{what}: {got} against spoint {want}");
    }

    fn assert_angle(got: f64, want: &Value, what: &str) {
        let want = want.as_f64().unwrap();
        assert!((got - want).abs() <= ANGLE_TOLERANCE_DEG, "{what}: {got} against spoint {want}");
    }

    fn assert_vec(got: V3, want: &Value, what: &str) {
        for k in 0..3 {
            assert_close(got[k], &want[k], what);
        }
    }

    fn assert_snapshot(got: ChartAnchorSnapshot, want: &Value, what: &str) {
        assert_close(got.radius, &want["radius"], what);
        assert_close(got.offset_y, &want["offsetY"], what);
        assert_close(got.anchor_height, &want["anchorHeight"], what);
        assert_eq!(got.chart_epoch as u64, want["chartEpoch"].as_u64().unwrap(), "{what}: chartEpoch");
        assert_vec(got.east, &want["east"], what);
        assert_vec(got.up, &want["up"], what);
        assert_vec(got.north, &want["north"], what);
    }

    #[test]
    fn create_chart_anchor_lattice_matches_spoint() {
        let f = fixture();
        let lattice = lattice_for(&f);
        let want = &f["lattice"];
        assert_close(CHART_ANCHORS_PER_FACE, &f["constants"]["anchorsPerFace"], "anchorsPerFace");
        assert_close(lattice.chunk_m, &want["chunkM"], "chunkM");
        assert_close(lattice.cell_m, &want["cellM"], "cellM");
        assert_close(lattice.cells_per_chunk, &want["cellsPerChunk"], "cellsPerChunk");
        assert_close(lattice.chunks_per_face, &want["chunksPerFace"], "chunksPerFace");
        assert_close(lattice.cells_per_face, &want["cellsPerFace"], "cellsPerFace");
        assert_close(lattice.cell_angle, &want["cellAngle"], "cellAngle");
        assert_close(lattice.chunk_angle, &want["chunkAngle"], "chunkAngle");
        for case in f["latticeCases"].as_array().unwrap() {
            let radius = case["radius"].as_f64().unwrap();
            let anchors = case["anchorsPerFace"].as_f64().unwrap();
            match create_chart_anchor_lattice(radius, anchors) {
                Ok(l) => {
                    assert!(case["ok"].as_bool().unwrap(), "{case}: built against spoint error");
                    assert_close(l.chunk_m, &case["chunkM"], "case chunkM");
                    assert_close(l.chunks_per_face, &case["chunksPerFace"], "case chunksPerFace");
                }
                Err(e) => {
                    assert!(!case["ok"].as_bool().unwrap(), "{case}: rejected against spoint success");
                    assert_eq!(e.reason(), case["reason"].as_str().unwrap(), "{case}: reason");
                }
            }
        }
    }

    #[test]
    fn chart_anchor_key_and_centre_match_spoint() {
        let f = fixture();
        let lattice = lattice_for(&f);
        for probe in f["keyProbes"].as_array().unwrap() {
            let dir = vec3(&probe["dir"]);
            let key = probe["key"].as_f64().unwrap();
            assert_eq!(chart_anchor_key_of_dir(&lattice, dir), key, "key of {dir:?}");
            assert_vec(chart_anchor_for_dir(&lattice, dir), &probe["centre"], "centre");
            assert_vec(chart_anchor_dir_of_key(&lattice, key), &probe["centre"], "centre of key");
        }
    }

    #[test]
    fn chart_anchor_angle_and_reanchor_need_match_spoint() {
        let f = fixture();
        let lattice = lattice_for(&f);
        let frame = ChartAnchorFrame::new(vec3(&f["anchorDir"]), f["planetRadius"].as_f64().unwrap(), f["offsetY"].as_f64().unwrap(), height_at);
        assert_close(CHART_REANCHOR_ANGLE_DEG, &f["constants"]["reanchorAngleDeg"], "reanchor angle");
        for probe in f["angleProbes"].as_array().unwrap() {
            let dir = vec3(&probe["dir"]);
            assert_angle(chart_anchor_angle_deg(frame.up(), dir), &probe["angleDeg"], "angle");
            assert_eq!(chart_needs_reanchor(&frame, dir, CHART_REANCHOR_ANGLE_DEG), probe["needs"].as_bool().unwrap(), "needs of {dir:?}");
            assert_eq!(chart_needs_reanchor_for_cell(&lattice, &frame, dir), probe["needsCell"].as_bool().unwrap(), "needsCell of {dir:?}");
        }
    }

    #[test]
    fn reanchor_chart_for_matches_spoint() {
        let f = fixture();
        let lattice = lattice_for(&f);
        let mut frame = ChartAnchorFrame::new(vec3(&f["anchorDir"]), f["planetRadius"].as_f64().unwrap(), f["offsetY"].as_f64().unwrap(), height_at);
        assert_snapshot(snapshot_chart(&frame), &f["initial"], "initial");
        for step in f["steps"].as_array().unwrap() {
            let name = step["name"].as_str().unwrap();
            let got = reanchor_chart_for(&mut frame, &lattice, vec3(&step["dir"]), step["thresholdDeg"].as_f64().unwrap(), step["cellTrigger"].as_bool().unwrap(), height_at);
            let want = &step["result"];
            match got {
                None => assert!(want.is_null(), "{name}: no reanchor against spoint {want}"),
                Some(r) => {
                    assert!(!want.is_null(), "{name}: reanchored against spoint null");
                    assert_snapshot(r.from, &want["from"], name);
                    assert_snapshot(r.to, &want["to"], name);
                    assert_vec(r.anchor_dir, &want["anchorDir"], name);
                    assert_angle(r.angle_before_deg, &want["angleBeforeDeg"], name);
                    assert_angle(r.angle_after_deg, &want["angleAfterDeg"], name);
                    assert_close(r.transfer.tilt_rad(), &want["tiltRad"], name);
                    let transfer = &want["transfer"];
                    for (p, wp) in transfer["points"].as_array().unwrap().iter().zip(transfer["pointResults"].as_array().unwrap()) {
                        assert_vec(r.transfer.point(vec3(p)), wp, name);
                    }
                    for (v, wv) in transfer["vectors"].as_array().unwrap().iter().zip(transfer["vectorResults"].as_array().unwrap()) {
                        assert_vec(r.transfer.vec(vec3(v)), wv, name);
                    }
                }
            }
            assert_snapshot(snapshot_chart(&frame), &step["frameAfter"], name);
        }
    }
}
