use crate::height::ops_jsgen::V3;

const QUARTER_TURN: f64 = std::f64::consts::FRAC_PI_2;
const FACE_EDGE_ANGLE: f64 = std::f64::consts::FRAC_PI_4;
const FACE_COUNT: f64 = 6.0;
const MIN_AXIS_STRETCH: f64 = 0.66;

const FACE_BASIS: [f64; 54] = [
    1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 1.0, 0.0,
    -1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0,
    0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, -1.0,
    0.0, -1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
    0.0, 0.0, -1.0, -1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
];

pub fn cube_face_of(dx: f64, dy: f64, dz: f64) -> f64 {
    let (ax, ay, az) = (dx.abs(), dy.abs(), dz.abs());
    if ax >= ay && ax >= az {
        if dx >= 0.0 { 0.0 } else { 1.0 }
    } else if ay >= az {
        if dy >= 0.0 { 2.0 } else { 3.0 }
    } else if dz >= 0.0 {
        4.0
    } else {
        5.0
    }
}

fn face_angles(face: f64, d: V3) -> [f64; 2] {
    let b = face as usize * 9;
    let dn = d[0] * FACE_BASIS[b] + d[1] * FACE_BASIS[b + 1] + d[2] * FACE_BASIS[b + 2];
    let du = d[0] * FACE_BASIS[b + 3] + d[1] * FACE_BASIS[b + 4] + d[2] * FACE_BASIS[b + 5];
    let dv = d[0] * FACE_BASIS[b + 6] + d[1] * FACE_BASIS[b + 7] + d[2] * FACE_BASIS[b + 8];
    [du.atan2(dn), dv.atan2(dn)]
}

fn dir_of_face_angles(face: f64, au: f64, av: f64) -> V3 {
    let b = face as usize * 9;
    let (tu, tv) = (au.tan(), av.tan());
    let x = FACE_BASIS[b] + tu * FACE_BASIS[b + 3] + tv * FACE_BASIS[b + 6];
    let y = FACE_BASIS[b + 1] + tu * FACE_BASIS[b + 4] + tv * FACE_BASIS[b + 7];
    let z = FACE_BASIS[b + 2] + tu * FACE_BASIS[b + 5] + tv * FACE_BASIS[b + 8];
    let l = (x * x + y * y + z * z).sqrt();
    [x / l, y / l, z / l]
}

fn clamp_index(i: f64, n: f64) -> f64 {
    if i < 0.0 {
        0.0
    } else if i >= n {
        n - 1.0
    } else {
        i
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlacementLattice {
    pub radius: f64,
    pub chunk_m: f64,
    pub cell_m: f64,
    pub cells_per_chunk: f64,
    pub chunks_per_face: f64,
    pub cells_per_face: f64,
    pub cell_angle: f64,
    pub chunk_angle: f64,
    centre_cell_area_over_target: f64,
}

impl PlacementLattice {
    pub fn new(radius: f64, chunk_m: f64, cells_per_chunk: f64) -> Result<Self, String> {
        if !(radius.is_finite() && radius > 0.0) {
            return Err(format!("placement lattice needs a finite planet radius, got {radius}"));
        }
        if !(chunk_m > 0.0) || !(cells_per_chunk >= 1.0) {
            return Err(format!("placement lattice needs chunkM > 0 and cellsPerChunk >= 1, got {chunk_m}, {cells_per_chunk}"));
        }
        let chunks_per_face = (QUARTER_TURN * radius / chunk_m).ceil();
        let cells_per_face = chunks_per_face * cells_per_chunk;
        let cell_angle = QUARTER_TURN / cells_per_face;
        let chunk_angle = cell_angle * cells_per_chunk;
        let cell_m = chunk_m / cells_per_chunk;
        let centre_cell_area_over_target = (radius * cell_angle / cell_m).powi(2);
        Ok(Self {
            radius,
            chunk_m,
            cell_m,
            cells_per_chunk,
            chunks_per_face,
            cells_per_face,
            cell_angle,
            chunk_angle,
            centre_cell_area_over_target,
        })
    }

    pub fn cell_dir(&self, face: f64, fu: f64, fv: f64) -> V3 {
        dir_of_face_angles(face, fu * self.cell_angle - FACE_EDGE_ANGLE, fv * self.cell_angle - FACE_EDGE_ANGLE)
    }

    pub fn cell_area_over_target(&self, fu: f64, fv: f64) -> f64 {
        let tu = (fu * self.cell_angle - FACE_EDGE_ANGLE).tan();
        let tv = (fv * self.cell_angle - FACE_EDGE_ANGLE).tan();
        let su = 1.0 + tu * tu;
        let sv = 1.0 + tv * tv;
        let l2 = su + tv * tv;
        self.centre_cell_area_over_target * su * sv / (l2 * l2.sqrt())
    }

    pub fn chunk_key(&self, face: f64, ci: f64, cj: f64) -> f64 {
        (face * self.chunks_per_face + ci) * self.chunks_per_face + cj
    }

    pub fn decode_chunk(&self, key: f64) -> [f64; 3] {
        let cj = key % self.chunks_per_face;
        let rest = (key - cj) / self.chunks_per_face;
        let ci = rest % self.chunks_per_face;
        [(rest - ci) / self.chunks_per_face, ci, cj]
    }

    pub fn chunk_key_of_dir(&self, d: V3) -> f64 {
        let face = cube_face_of(d[0], d[1], d[2]);
        let a = face_angles(face, d);
        let ci = clamp_index(((a[0] + FACE_EDGE_ANGLE) / self.chunk_angle).floor(), self.chunks_per_face);
        let cj = clamp_index(((a[1] + FACE_EDGE_ANGLE) / self.chunk_angle).floor(), self.chunks_per_face);
        self.chunk_key(face, ci, cj)
    }

    pub fn chunk_centre_dir(&self, key: f64) -> V3 {
        let c = self.decode_chunk(key);
        self.cell_dir(c[0], (c[1] + 0.5) * self.cells_per_chunk, (c[2] + 0.5) * self.cells_per_chunk)
    }

    #[cfg(test)]
    pub fn chunk_corner_dir(&self, key: f64, corner_u: f64, corner_v: f64) -> V3 {
        let c = self.decode_chunk(key);
        self.cell_dir(c[0], (c[1] + corner_u) * self.cells_per_chunk, (c[2] + corner_v) * self.cells_per_chunk)
    }

    pub fn cell_id(&self, face: f64, i: f64, j: f64) -> f64 {
        (face * self.cells_per_face + i) * self.cells_per_face + j
    }

    pub fn hash_row(&self, face: f64, i: f64) -> f64 {
        face * self.cells_per_face + i
    }

    pub fn ring_around_dir(&self, d: V3, radius_m: f64) -> Vec<f64> {
        let max_angle = radius_m / self.radius;
        let max_chord_sq = 4.0 * (max_angle / 2.0).sin().powi(2);
        let reach = max_angle / (self.chunk_angle * MIN_AXIS_STRETCH);
        let reach_tan = (FACE_EDGE_ANGLE + reach * self.chunk_angle).tan();
        let mut found: Vec<(f64, f64)> = Vec::new();
        for face in 0..FACE_COUNT as usize {
            let face = face as f64;
            let b = face as usize * 9;
            let dn = d[0] * FACE_BASIS[b] + d[1] * FACE_BASIS[b + 1] + d[2] * FACE_BASIS[b + 2];
            if dn <= 0.0 {
                continue;
            }
            let du = d[0] * FACE_BASIS[b + 3] + d[1] * FACE_BASIS[b + 4] + d[2] * FACE_BASIS[b + 5];
            let du_limit = dn * reach_tan;
            if du > du_limit || du < -du_limit {
                continue;
            }
            let dv = d[0] * FACE_BASIS[b + 6] + d[1] * FACE_BASIS[b + 7] + d[2] * FACE_BASIS[b + 8];
            if dv > du_limit || dv < -du_limit {
                continue;
            }
            let a = face_angles(face, d);
            let fci = (a[0] + FACE_EDGE_ANGLE) / self.chunk_angle - 0.5;
            let fcj = (a[1] + FACE_EDGE_ANGLE) / self.chunk_angle - 0.5;
            let i0 = (fci - reach).ceil().max(0.0);
            let i1 = (fci + reach).floor().min(self.chunks_per_face - 1.0);
            let mut ci = i0;
            while ci <= i1 {
                let off = ci - fci;
                let rest = reach * reach - off * off;
                if rest > 0.0 {
                    let half = rest.sqrt();
                    let j0 = (fcj - half).ceil().max(0.0);
                    let j1 = (fcj + half).floor().min(self.chunks_per_face - 1.0);
                    let mut cj = j0;
                    while cj <= j1 {
                        let key = self.chunk_key(face, ci, cj);
                        let c = self.chunk_centre_dir(key);
                        let (ex, ey, ez) = (c[0] - d[0], c[1] - d[1], c[2] - d[2]);
                        let chord = ex * ex + ey * ey + ez * ez;
                        if !(chord > max_chord_sq) {
                            found.push((chord, key));
                        }
                        cj += 1.0;
                    }
                }
                ci += 1.0;
            }
        }
        found.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
        found.into_iter().map(|(_, key)| key).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn num(v: &Value) -> f64 {
        match v.as_str() {
            Some("NaN") => f64::NAN,
            _ => v.as_f64().unwrap(),
        }
    }

    fn v3(v: &Value) -> V3 {
        let a = v.as_array().unwrap();
        [num(&a[0]), num(&a[1]), num(&a[2])]
    }

    fn chord(c: V3, d: V3) -> f64 {
        (c[0] - d[0]).powi(2) + (c[1] - d[1]).powi(2) + (c[2] - d[2]).powi(2)
    }

    fn check(label: &str, got: f64, want: f64) {
        assert!(got == want || (got - want).abs() <= 1e-9, "{label}: bevy {got} against spoint {want}");
    }

    #[test]
    fn placement_lattice_matches_spoint() {
        let fixture: Value = serde_json::from_str(include_str!("../../testdata/parity/spoint-placement-lattice.json")).unwrap();
        for case in fixture["lattices"].as_array().unwrap() {
            let lat = PlacementLattice::new(num(&case["radius"]), num(&case["chunkM"]), num(&case["cellsPerChunk"])).unwrap();
            let s = &case["scalars"];
            check("chunksPerFace", lat.chunks_per_face, num(&s["chunksPerFace"]));
            check("cellsPerFace", lat.cells_per_face, num(&s["cellsPerFace"]));
            check("cellAngle", lat.cell_angle, num(&s["cellAngle"]));
            check("chunkAngle", lat.chunk_angle, num(&s["chunkAngle"]));
            check("cellM", lat.cell_m, num(&s["cellM"]));
            for row in case["faceOf"].as_array().unwrap() {
                let d = v3(&row[0]);
                check("cubeFaceOf", cube_face_of(d[0], d[1], d[2]), num(&row[1]));
            }
            for row in case["cellDir"].as_array().unwrap() {
                let v = lat.cell_dir(num(&row[0]), num(&row[1]), num(&row[2]));
                for i in 0..3 {
                    check("cellDir", v[i], num(&row[3][i]));
                }
            }
            for row in case["area"].as_array().unwrap() {
                check("cellAreaOverTarget", lat.cell_area_over_target(num(&row[0]), num(&row[1])), num(&row[2]));
            }
            for row in case["chunkKeys"].as_array().unwrap() {
                check("chunkKey", lat.chunk_key(num(&row[0]), num(&row[1]), num(&row[2])), num(&row[3]));
            }
            for row in case["decode"].as_array().unwrap() {
                let got = lat.decode_chunk(num(&row[0]));
                for i in 0..3 {
                    check("decodeChunk", got[i], num(&row[1][i]));
                }
            }
            for row in case["keyOfDir"].as_array().unwrap() {
                check("chunkKeyOfDir", lat.chunk_key_of_dir(v3(&row[0])), num(&row[1]));
            }
            for row in case["centreDir"].as_array().unwrap() {
                let v = lat.chunk_centre_dir(num(&row[0]));
                for i in 0..3 {
                    check("chunkCentreDir", v[i], num(&row[1][i]));
                }
            }
            for row in case["cornerDir"].as_array().unwrap() {
                let v = lat.chunk_corner_dir(num(&row[0]), num(&row[1]), num(&row[2]));
                for i in 0..3 {
                    check("chunkCornerDir", v[i], num(&row[3][i]));
                }
            }
            for row in case["cellIds"].as_array().unwrap() {
                check("cellId", lat.cell_id(num(&row[0]), num(&row[1]), num(&row[2])), num(&row[3]));
            }
            for row in case["hashRows"].as_array().unwrap() {
                check("hashRow", lat.hash_row(num(&row[0]), num(&row[1])), num(&row[2]));
            }
            for row in case["rings"].as_array().unwrap() {
                let dir = v3(&row[0]);
                let got = lat.ring_around_dir(dir, num(&row[1]));
                let want: Vec<f64> = row[2].as_array().unwrap().iter().map(num).collect();
                assert_eq!(got.len(), want.len(), "ringAroundDir({:?}, {}) length", dir, num(&row[1]));
                let (mut got_keys, mut want_keys) = (got.clone(), want.clone());
                got_keys.sort_by(f64::total_cmp);
                want_keys.sort_by(f64::total_cmp);
                assert_eq!(got_keys, want_keys, "ringAroundDir({:?}, {}) keys", dir, num(&row[1]));
                for (g, w) in got.iter().zip(want.iter()) {
                    let chord_got = chord(lat.chunk_centre_dir(*g), dir);
                    let chord_want = chord(lat.chunk_centre_dir(*w), dir);
                    assert!((chord_got - chord_want).abs() <= 1e-9, "ringAroundDir({:?}, {}) chord {chord_got} against spoint {chord_want}", dir, num(&row[1]));
                }
            }
        }
        for row in fixture["errors"].as_array().unwrap() {
            let err = PlacementLattice::new(num(&row["radius"]), num(&row["chunkM"]), num(&row["cellsPerChunk"])).unwrap_err();
            assert_eq!(err, row["message"].as_str().unwrap());
        }
    }
}
