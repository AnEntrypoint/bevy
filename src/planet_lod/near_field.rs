#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Leaf {
    pub ox: f64,
    pub oy: f64,
    pub l: f64,
}

pub const NEAR_FIELD_DEFAULT_GRID_STEPS: i64 = 9;
#[cfg(test)]
pub const NEAR_FIELD_DEFAULT_TILE_BUDGET: i64 = 64;
pub const NEAR_FIELD_FLOOR_TILES: f64 = 1.0;
pub const MIN_GROUND_PER_FACE_METRE: f64 = 0.6;
#[cfg(test)]
const OVERLAP_EPSILON: f64 = 1e-6;

#[derive(Clone, Copy, Debug, Default)]
pub struct NearFieldOptions {
    pub spacing_m: Option<f64>,
    pub extent_m: Option<f64>,
    pub grid_steps: Option<i64>,
    #[cfg(test)]
    pub tile_budget: Option<i64>,
    #[cfg(test)]
    pub face_half_span_m: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloorReach {
    pub ground_per_face_metre: f64,
    pub outer_m: f64,
    pub inner_m: f64,
    pub outer_span_leaves: f64,
    pub inner_span_leaves: f64,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tile {
    pub ox: f64,
    pub oy: f64,
    pub l: f64,
    pub level: u8,
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub tiles: Vec<Tile>,
    pub drop: Vec<usize>,
    pub tile_m: f64,
    pub step_m: f64,
    pub tile_count: usize,
    pub covered_leaf_count: usize,
    pub vertex_count: i64,
    pub ground_per_face_metre: f64,
    pub requested_half_extent_m: f64,
    pub covered_half_extent_m: f64,
    pub covered_half_extent_ground_m: f64,
    pub shortfall_m: f64,
    pub straddling_leaf_count: usize,
    pub suppressed_tiles: usize,
    pub tiles_per_axis: i64,
    pub instance_budget: Option<i64>,
}

#[derive(Clone, Copy, Debug)]
pub struct NearField {
    #[cfg(test)]
    pub spacing_m: f64,
    #[cfg(test)]
    pub grid_steps: i64,
    #[cfg(test)]
    pub tile_budget: i64,
    pub tile_m: f64,
    pub max_leaf_m: f64,
    pub requested_half_extent_m: f64,
    pub floor_span_leaves: f64,
    #[cfg(test)]
    face_half_span_m: Option<f64>,
    coverage_cap: f64,
}

fn positive_finite(value: Option<f64>) -> Option<f64> {
    value.filter(|v| v.is_finite() && *v > 0.0)
}

fn steps_or(value: Option<i64>, minimum: i64, fallback: i64) -> i64 {
    value.filter(|v| *v >= minimum).unwrap_or(fallback)
}

impl NearField {
    pub fn new(options: NearFieldOptions) -> Result<NearField, String> {
        let spacing_m = positive_finite(options.spacing_m).ok_or("near-field lattice: spacing_m must be a positive finite number of metres")?;
        let extent_m = positive_finite(options.extent_m).ok_or("near-field lattice: extent_m must be a positive finite number of metres")?;
        let grid_steps = steps_or(options.grid_steps, 2, NEAR_FIELD_DEFAULT_GRID_STEPS);
        let tile_m = grid_steps as f64 * spacing_m;
        let requested_half_extent_m = extent_m / 2.0;
        let mut lattice = NearField {
            #[cfg(test)]
            spacing_m,
            #[cfg(test)]
            grid_steps,
            #[cfg(test)]
            tile_budget: steps_or(options.tile_budget, 1, NEAR_FIELD_DEFAULT_TILE_BUDGET),
            tile_m,
            max_leaf_m: tile_m * NEAR_FIELD_FLOOR_TILES,
            requested_half_extent_m,
            floor_span_leaves: 0.0,
            #[cfg(test)]
            face_half_span_m: positive_finite(options.face_half_span_m),
            coverage_cap: (requested_half_extent_m / tile_m).floor().max(1.0),
        };
        lattice.floor_span_leaves = lattice.floor_reach_leaves(Some(MIN_GROUND_PER_FACE_METRE)).outer_span_leaves;
        Ok(lattice)
    }

    pub fn floor_reach_leaves(&self, ground_per_face_metre: Option<f64>) -> FloorReach {
        let gpf = positive_finite(ground_per_face_metre).unwrap_or(1.0);
        let outer_m = (self.coverage_cap * self.tile_m / 2.0 + self.tile_m) / MIN_GROUND_PER_FACE_METRE;
        let inner_m = (self.coverage_cap * self.tile_m / (2.0 * gpf) - self.max_leaf_m).max(0.0);
        FloorReach {
            ground_per_face_metre: gpf,
            outer_m,
            inner_m,
            outer_span_leaves: outer_m / self.max_leaf_m,
            inner_span_leaves: inner_m / self.max_leaf_m,
        }
    }

    #[cfg(test)]
    pub fn plan(&self, leaves: &[Leaf], camera: Option<[f64; 2]>, grid_steps_override: Option<i64>, instance_budget: Option<i64>, ground_per_face_metre: Option<f64>) -> Plan {
        let gs = steps_or(grid_steps_override, 2, self.grid_steps);
        let gpf = positive_finite(ground_per_face_metre).unwrap_or(1.0);
        let tile_m_local = self.tile_m / gpf;
        let half_extent_local = self.requested_half_extent_m / gpf;
        let empty = Plan {
            tiles: Vec::new(),
            drop: Vec::new(),
            tile_m: tile_m_local,
            step_m: self.spacing_m,
            tile_count: 0,
            covered_leaf_count: 0,
            vertex_count: 0,
            ground_per_face_metre: gpf,
            requested_half_extent_m: self.requested_half_extent_m,
            covered_half_extent_m: 0.0,
            covered_half_extent_ground_m: 0.0,
            shortfall_m: self.requested_half_extent_m,
            straddling_leaf_count: 0,
            suppressed_tiles: 0,
            tiles_per_axis: 0,
            instance_budget: None,
        };
        let [cam_x, cam_y] = match camera {
            Some(c) if !leaves.is_empty() && c[0].is_finite() && c[1].is_finite() => c,
            _ => return empty,
        };

        let budget = instance_budget.filter(|b| *b >= 1).unwrap_or(leaves.len() as i64);
        let wanted = (2.0 * half_extent_local / tile_m_local).ceil();
        let per_axis = (budget as f64).sqrt().floor().min(wanted).min(self.coverage_cap).max(1.0);
        let half = per_axis * tile_m_local / 2.0;
        let mut x0 = ((cam_x - half) / tile_m_local).floor() * tile_m_local;
        let mut y0 = ((cam_y - half) / tile_m_local).floor() * tile_m_local;
        if let Some(span) = self.face_half_span_m {
            x0 = x0.max(-span).min(span - per_axis * tile_m_local);
            y0 = y0.max(-span).min(span - per_axis * tile_m_local);
        }
        let x1 = x0 + per_axis * tile_m_local;
        let y1 = y0 + per_axis * tile_m_local;
        let eps = OVERLAP_EPSILON;

        let mut drop = Vec::new();
        let mut straddle = Vec::new();
        for (index, q) in leaves.iter().enumerate() {
            let qx1 = q.ox + q.l;
            let qy1 = q.oy + q.l;
            let overlaps = qx1 > x0 + eps && qy1 > y0 + eps && q.ox < x1 - eps && q.oy < y1 - eps;
            if !overlaps {
                continue;
            }
            if q.ox >= x0 - eps && q.oy >= y0 - eps && qx1 <= x1 + eps && qy1 <= y1 + eps {
                drop.push(index);
            } else {
                straddle.push(*q);
            }
        }

        let axis = per_axis as i64;
        let mut tiles = Vec::new();
        let mut suppressed = 0;
        for j in 0..axis {
            for i in 0..axis {
                let ox = x0 + i as f64 * tile_m_local;
                let oy = y0 + j as f64 * tile_m_local;
                let covered = straddle.iter().any(|q| {
                    ox >= q.ox - eps && oy >= q.oy - eps && ox + tile_m_local <= q.ox + q.l + eps && oy + tile_m_local <= q.oy + q.l + eps
                });
                if covered {
                    suppressed += 1;
                } else {
                    tiles.push(Tile { ox, oy, l: tile_m_local, level: 0 });
                }
            }
        }
        Plan {
            tile_count: tiles.len(),
            covered_leaf_count: drop.len(),
            vertex_count: tiles.len() as i64 * (gs + 3) * (gs + 3),
            covered_half_extent_m: half,
            covered_half_extent_ground_m: half * gpf,
            shortfall_m: (self.requested_half_extent_m - half * gpf).max(0.0),
            straddling_leaf_count: straddle.len(),
            suppressed_tiles: suppressed,
            tiles_per_axis: axis,
            instance_budget: Some(budget),
            tiles,
            drop,
            ..empty
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{cases, close, parse};
    use serde_json::Value;

    fn number(value: &Value) -> Option<f64> {
        value.as_f64()
    }

    fn integer(value: &Value) -> Option<i64> {
        value.as_i64()
    }

    fn options(value: &Value) -> NearFieldOptions {
        NearFieldOptions {
            spacing_m: number(&value["spacingM"]),
            extent_m: number(&value["extentM"]),
            grid_steps: integer(&value["gridSteps"]),
            tile_budget: integer(&value["tileBudget"]),
            face_half_span_m: number(&value["faceHalfSpanM"]),
        }
    }

    fn fixture() -> Value {
        parse(include_str!("../../testdata/parity/spoint-planet-near-field.json"))
    }

    #[test]
    fn creation_matches_spoint() {
        let fixture = fixture();
        let creations = cases(&fixture, "creations");
        assert!(creations.len() >= 12);
        for (n, case) in creations.iter().enumerate() {
            let context = format!("creation {n}");
            let made = NearField::new(options(&case["options"]));
            let want = &case["lattice"];
            if want.is_null() {
                assert!(made.is_err(), "{context}");
                continue;
            }
            let lattice = made.unwrap();
            assert_eq!(lattice.grid_steps, want["gridSteps"].as_i64().unwrap(), "{context}");
            assert_eq!(lattice.tile_budget, want["tileBudget"].as_i64().unwrap(), "{context}");
            for (got, key) in [
                (lattice.tile_m, "tileM"),
                (lattice.max_leaf_m, "maxLeafM"),
                (lattice.requested_half_extent_m, "requestedHalfExtentM"),
                (lattice.floor_span_leaves, "floorSpanLeaves"),
            ] {
                close(got, want[key].as_f64().unwrap(), 1e-9, &format!("{context} {key}"));
            }
            for reach in want["reach"].as_array().unwrap() {
                let got = lattice.floor_reach_leaves(number(&reach["gpf"]));
                let want = &reach["result"];
                for (value, key) in [
                    (got.ground_per_face_metre, "groundPerFaceMetre"),
                    (got.outer_m, "outerM"),
                    (got.inner_m, "innerM"),
                    (got.outer_span_leaves, "outerSpanLeaves"),
                    (got.inner_span_leaves, "innerSpanLeaves"),
                ] {
                    close(value, want[key].as_f64().unwrap(), 1e-9, &format!("{context} reach {} {key}", reach["gpf"]));
                }
            }
        }
    }

    #[test]
    fn plans_match_spoint() {
        let fixture = fixture();
        let creations = cases(&fixture, "creations");
        let plans = cases(&fixture, "plans");
        assert!(plans.len() >= 150);
        for (n, case) in plans.iter().enumerate() {
            let context = format!("plan {n}");
            let lattice = NearField::new(options(&creations[case["creation"].as_u64().unwrap() as usize]["options"])).unwrap();
            let leaves: Vec<Leaf> = fixture["leafSets"][case["set"].as_str().unwrap()]
                .as_array()
                .unwrap()
                .iter()
                .map(|q| Leaf { ox: q[0].as_f64().unwrap(), oy: q[1].as_f64().unwrap(), l: q[2].as_f64().unwrap() })
                .collect();
            let camera = case["camera"].as_array().map(|c| [c[0].as_f64().unwrap_or(f64::NAN), c[1].as_f64().unwrap_or(f64::NAN)]);
            let got = lattice.plan(&leaves, camera, integer(&case["gridSteps"]), integer(&case["budget"]), number(&case["gpf"]));
            let want = &case["plan"];
            let count = |key: &str| want[key].as_i64().unwrap();
            assert_eq!(got.tile_count as i64, count("tileCount"), "{context} tileCount");
            assert_eq!(got.covered_leaf_count as i64, count("coveredLeafCount"), "{context} coveredLeafCount");
            assert_eq!(got.vertex_count, count("vertexCount"), "{context} vertexCount");
            assert_eq!(got.straddling_leaf_count as i64, count("straddlingLeafCount"), "{context} straddlingLeafCount");
            assert_eq!(got.suppressed_tiles as i64, count("suppressedTiles"), "{context} suppressedTiles");
            assert_eq!(got.tiles_per_axis, count("tilesPerAxis"), "{context} tilesPerAxis");
            assert_eq!(got.instance_budget, want["instanceBudget"].as_i64(), "{context} instanceBudget");
            let want_drop: Vec<usize> = want["drop"].as_array().unwrap().iter().map(|i| i.as_u64().unwrap() as usize).collect();
            assert_eq!(got.drop, want_drop, "{context} drop");
            for (value, key) in [
                (got.tile_m, "tileM"),
                (got.step_m, "stepM"),
                (got.ground_per_face_metre, "groundPerFaceMetre"),
                (got.requested_half_extent_m, "requestedHalfExtentM"),
                (got.covered_half_extent_m, "coveredHalfExtentM"),
                (got.covered_half_extent_ground_m, "coveredHalfExtentGroundM"),
                (got.shortfall_m, "shortfallM"),
            ] {
                close(value, want[key].as_f64().unwrap(), 1e-9, &format!("{context} {key}"));
            }
            let want_tiles = want["tiles"].as_array().unwrap();
            assert_eq!(got.tiles.len(), want_tiles.len(), "{context} tiles");
            for (k, (tile, want_tile)) in got.tiles.iter().zip(want_tiles).enumerate() {
                close(tile.ox, want_tile["ox"].as_f64().unwrap(), 1e-9, &format!("{context} tile {k} ox"));
                close(tile.oy, want_tile["oy"].as_f64().unwrap(), 1e-9, &format!("{context} tile {k} oy"));
                close(tile.l, want_tile["l"].as_f64().unwrap(), 1e-9, &format!("{context} tile {k} l"));
                assert_eq!(tile.level as i64, want_tile["level"].as_i64().unwrap(), "{context} tile {k} level");
            }
        }
    }
}
