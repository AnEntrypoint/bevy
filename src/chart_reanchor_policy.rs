#![cfg_attr(not(test), allow(dead_code))]

use std::cmp::Ordering;

use crate::chart_anchor::{chart_anchor_angle_deg, chart_anchor_for_dir, ChartAnchorFrame};
use crate::chart_reanchor_service::WALKABLE_CHART_LIMIT_DEG;
use crate::height::ops_jsgen::V3;
use crate::height::placement::PlacementLattice;

pub const CHART_REANCHOR_HYSTERESIS_DEG: f64 = 0.75;
const SAME_ANCHOR_TOLERANCE: f64 = 1e-9;

#[derive(Clone, Debug, PartialEq)]
pub struct ChartAnchorDecisionError {
    pub reason: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayersStraddlePlanetError;

#[derive(Clone, Debug, PartialEq)]
pub enum PlayerCentroidError {
    Decision(ChartAnchorDecisionError),
    Straddle(PlayersStraddlePlanetError),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChartAnchorDecision {
    Stay {
        reason: &'static str,
        gain_deg: Option<f64>,
    },
    Refusal {
        reason: &'static str,
        worst_player_angle_deg: Option<f64>,
        walkable_limit_deg: Option<f64>,
    },
    Reanchor {
        anchor_dir: V3,
        centroid_dir: V3,
        gain_deg: f64,
        worst_player_angle_deg: f64,
    },
}

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn norm(a: V3) -> f64 {
    dot(a, a).sqrt()
}

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn compare_dirs(a: &V3, b: &V3) -> Ordering {
    a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])).then(a[2].total_cmp(&b[2]))
}

pub fn player_centroid_dir(dirs: &[V3]) -> Result<Option<V3>, PlayerCentroidError> {
    if dirs.is_empty() {
        return Ok(None);
    }
    let mut units: Vec<V3> = Vec::with_capacity(dirs.len());
    for d in dirs {
        let length = norm(*d);
        if !(length > 0.0) {
            return Err(PlayerCentroidError::Decision(ChartAnchorDecisionError { reason: "player-direction-has-no-length" }));
        }
        units.push([d[0] / length, d[1] / length, d[2] / length]);
    }
    units.sort_by(compare_dirs);
    let sum = units.iter().fold([0.0; 3], |acc, u| [acc[0] + u[0], acc[1] + u[1], acc[2] + u[2]]);
    let length = norm(sum);
    if !(length > 0.0) {
        return Err(PlayerCentroidError::Straddle(PlayersStraddlePlanetError));
    }
    Ok(Some([sum[0] / length, sum[1] / length, sum[2] / length]))
}

fn refusal(reason: &'static str) -> ChartAnchorDecision {
    ChartAnchorDecision::Refusal { reason, worst_player_angle_deg: None, walkable_limit_deg: None }
}

pub fn chart_anchor_decision(lattice: &PlacementLattice, frame: &ChartAnchorFrame, dirs: &[V3], hysteresis_deg: f64, walkable_limit_deg: f64) -> ChartAnchorDecision {
    let centroid_dir = match player_centroid_dir(dirs) {
        Ok(Some(centroid)) => centroid,
        Ok(None) => return ChartAnchorDecision::Stay { reason: "no-players", gain_deg: None },
        Err(PlayerCentroidError::Straddle(_)) => return refusal("players-straddle-the-planet"),
        Err(PlayerCentroidError::Decision(e)) => return refusal(e.reason),
    };
    let up = frame.up();
    let anchor_dir = chart_anchor_for_dir(lattice, centroid_dir);
    let worst_player_angle_deg = dirs.iter().fold(0.0, |worst: f64, d: &V3| worst.max(chart_anchor_angle_deg(anchor_dir, *d)));
    if worst_player_angle_deg >= walkable_limit_deg {
        return ChartAnchorDecision::Refusal {
            reason: "party-spread-exceeds-walkable-chart",
            worst_player_angle_deg: Some(worst_player_angle_deg),
            walkable_limit_deg: Some(walkable_limit_deg),
        };
    }
    if norm(sub(anchor_dir, up)) < SAME_ANCHOR_TOLERANCE {
        return ChartAnchorDecision::Stay { reason: "anchor-unchanged", gain_deg: Some(0.0) };
    }
    let gain_deg = chart_anchor_angle_deg(up, centroid_dir) - chart_anchor_angle_deg(anchor_dir, centroid_dir);
    if gain_deg < hysteresis_deg {
        return ChartAnchorDecision::Stay { reason: "gain-below-hysteresis", gain_deg: Some(gain_deg) };
    }
    ChartAnchorDecision::Reanchor { anchor_dir, centroid_dir, gain_deg, worst_player_angle_deg }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart_anchor::{create_chart_anchor_lattice, CHART_ANCHORS_PER_FACE};
    use serde_json::Value;

    const TOLERANCE: f64 = 1e-9;
    const ANGLE_TOLERANCE_DEG: f64 = 1e-6;

    fn fixture() -> Value {
        serde_json::from_str(include_str!("../testdata/parity/spoint-chart-reanchor-policy.json")).unwrap()
    }

    fn vec3(v: &Value) -> V3 {
        [v[0].as_f64().unwrap(), v[1].as_f64().unwrap(), v[2].as_f64().unwrap()]
    }

    fn height_at(d: V3) -> f64 {
        0.5 + 0.25 * d[0] - 0.1 * d[2]
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

    fn assert_decision(got: ChartAnchorDecision, want: &Value, name: &str) {
        let kind = want["kind"].as_str().unwrap();
        match got {
            ChartAnchorDecision::Stay { reason, gain_deg } => {
                assert_eq!(kind, "stay", "{name}: kind");
                assert_eq!(reason, want["reason"].as_str().unwrap(), "{name}: reason");
                assert_eq!(gain_deg.is_some(), !want["gainDeg"].is_null(), "{name}: gainDeg presence");
                if let Some(g) = gain_deg {
                    assert_angle(g, &want["gainDeg"], name);
                }
            }
            ChartAnchorDecision::Refusal { reason, worst_player_angle_deg, walkable_limit_deg } => {
                assert_eq!(kind, "refusal", "{name}: kind");
                assert_eq!(reason, want["reason"].as_str().unwrap(), "{name}: reason");
                assert_eq!(worst_player_angle_deg.is_some(), !want["worstPlayerAngleDeg"].is_null(), "{name}: worst presence");
                if let Some(w) = worst_player_angle_deg {
                    assert_angle(w, &want["worstPlayerAngleDeg"], name);
                }
                if let Some(l) = walkable_limit_deg {
                    assert_close(l, &want["walkableLimitDeg"], name);
                }
            }
            ChartAnchorDecision::Reanchor { anchor_dir, centroid_dir, gain_deg, worst_player_angle_deg } => {
                assert_eq!(kind, "reanchor", "{name}: kind");
                assert_vec(anchor_dir, &want["anchorDir"], name);
                assert_vec(centroid_dir, &want["centroidDir"], name);
                assert_angle(gain_deg, &want["gainDeg"], name);
                assert_angle(worst_player_angle_deg, &want["worstPlayerAngleDeg"], name);
            }
        }
    }

    #[test]
    fn player_centroid_dir_matches_spoint() {
        let f = fixture();
        for case in f["playerCentroid"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let dirs: Vec<V3> = case["dirs"].as_array().unwrap().iter().map(vec3).collect();
            match (player_centroid_dir(&dirs), case["kind"].as_str().unwrap()) {
                (Ok(None), "none") => {}
                (Ok(Some(c)), "ok") => assert_vec(c, &case["centroidDir"], name),
                (Err(PlayerCentroidError::Decision(e)), "error") => {
                    assert_eq!(case["errorName"].as_str().unwrap(), "ChartAnchorDecisionError", "{name}: error name");
                    assert_eq!(e.reason, case["reason"].as_str().unwrap(), "{name}: reason");
                }
                (Err(PlayerCentroidError::Straddle(_)), "error") => {
                    assert_eq!(case["errorName"].as_str().unwrap(), "PlayersStraddlePlanetError", "{name}: error name");
                }
                (got, want) => panic!("{name}: {got:?} against spoint {want}"),
            }
        }
    }

    #[test]
    fn chart_anchor_decision_matches_spoint() {
        let f = fixture();
        let lattice = create_chart_anchor_lattice(f["planetRadius"].as_f64().unwrap(), CHART_ANCHORS_PER_FACE).unwrap();
        assert_close(CHART_REANCHOR_HYSTERESIS_DEG, &f["hysteresisDeg"], "hysteresisDeg");
        assert_close(WALKABLE_CHART_LIMIT_DEG, &f["walkableLimitDeg"], "walkableLimitDeg");
        for case in f["decisions"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let frame = ChartAnchorFrame::new(vec3(&case["up"]), f["planetRadius"].as_f64().unwrap(), 0.0, height_at);
            let dirs: Vec<V3> = case["dirs"].as_array().unwrap().iter().map(vec3).collect();
            let got = chart_anchor_decision(&lattice, &frame, &dirs, case["hysteresisDeg"].as_f64().unwrap(), case["walkableLimitDeg"].as_f64().unwrap());
            assert_decision(got, &case["result"], name);
        }
    }
}
