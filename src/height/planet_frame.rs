use super::ops_jsgen::V3;

pub fn normalise(v: V3) -> V3 {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [v[0] / length, v[1] / length, v[2] / length]
}

fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

pub struct Basis {
    pub up: V3,
    pub east: V3,
    pub north: V3,
}

pub fn anchor_basis(anchor: V3) -> Basis {
    let up = normalise(anchor);
    let reference = if up[1].abs() < 0.99 { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] };
    let east = normalise(cross(reference, up));
    let north = cross(east, up);
    Basis { up, east, north }
}

pub fn tangent_local_to_dir(basis: &Basis, radius: f64, x: f64, z: f64) -> V3 {
    normalise([0, 1, 2].map(|i| basis.up[i] * radius + basis.east[i] * x + basis.north[i] * z))
}

#[cfg_attr(not(test), allow(dead_code))]
pub struct Frame {
    pub offset_y: f64,
    pub anchor_height: f64,
    pub radius: f64,
}

#[cfg_attr(not(test), allow(dead_code))]
impl Frame {
    fn sphere_radius(&self) -> f64 {
        if self.radius.is_finite() && self.radius > 0.0 { self.radius } else { f64::INFINITY }
    }

    fn is_finite(&self) -> bool {
        self.offset_y.is_finite() && self.anchor_height.is_finite()
    }

    pub fn sphere_surface_local_y(&self, x: f64, z: f64, elevation: f64) -> Option<f64> {
        if !self.is_finite() || !elevation.is_finite() {
            return None;
        }
        let radius = self.sphere_radius();
        let drop = if radius == f64::INFINITY { 0.0 } else { sphere_drop_below_tangent(x * x + z * z, radius + elevation) };
        Some(self.offset_y + (elevation - self.anchor_height) - drop)
    }

    pub fn waterline_local_y(&self, x: f64, z: f64) -> Option<f64> {
        self.sphere_surface_local_y(x, z, 0.0)
    }

    pub fn elevation_at_local(&self, x: f64, y: f64, z: f64) -> Option<f64> {
        if !self.is_finite() {
            return None;
        }
        let radius = self.sphere_radius();
        if radius == f64::INFINITY {
            return Some(y - self.offset_y + self.anchor_height);
        }
        let t = radius + self.anchor_height + (y - self.offset_y);
        let r2 = x * x + z * z;
        let l = (t * t + r2).sqrt();
        Some((t - radius) + r2 / (l + t))
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn sphere_drop_below_tangent(r2: f64, sphere_radius: f64) -> f64 {
    let q = sphere_radius * sphere_radius - r2;
    if q > 0.0 { r2 / (sphere_radius + q.sqrt()) } else { sphere_radius }
}

#[cfg(test)]
mod tests {
    use super::{anchor_basis, tangent_local_to_dir, Frame};
    use serde_json::Value;

    const PLANET_FRAME_TOLERANCE_M: f64 = 1e-9;

    fn fixture() -> Value {
        serde_json::from_str(include_str!("../../testdata/parity/spoint-planet-frame.json")).unwrap()
    }

    fn vec3(value: &Value) -> [f64; 3] {
        [value[0].as_f64().unwrap(), value[1].as_f64().unwrap(), value[2].as_f64().unwrap()]
    }

    fn frame_of(value: &Value) -> Frame {
        let read = |key: &str| value[key].as_f64().unwrap_or(f64::NAN);
        Frame { offset_y: read("offsetY"), anchor_height: read("anchorHeight"), radius: read("radius") }
    }

    fn compare(worst: &mut f64, got: Option<f64>, want: &Value) {
        match (got, want.as_f64()) {
            (None, None) => {}
            (Some(g), Some(w)) => *worst = worst.max((g - w).abs()),
            (g, w) => panic!("spoint returns {w:?}, bevy returns {g:?}"),
        }
    }

    #[test]
    fn anchor_basis_matches_the_spoint_frame() {
        for case in fixture()["basis"].as_array().unwrap() {
            let basis = anchor_basis(vec3(&case["anchor"]));
            for (got, want) in [(basis.up, &case["up"]), (basis.east, &case["east"]), (basis.north, &case["north"])] {
                let want = vec3(want);
                for i in 0..3 {
                    assert!((got[i] - want[i]).abs() <= PLANET_FRAME_TOLERANCE_M, "basis {got:?} differs from spoint {want:?}");
                }
            }
        }
    }

    #[test]
    fn tangent_local_to_dir_matches_the_spoint_frame() {
        let mut worst = 0.0_f64;
        for case in fixture()["tangent"].as_array().unwrap() {
            let dir = tangent_local_to_dir(&anchor_basis(vec3(&case["anchor"])), case["radius"].as_f64().unwrap(), case["x"].as_f64().unwrap(), case["z"].as_f64().unwrap());
            let want = vec3(&case["dir"]);
            for i in 0..3 {
                worst = worst.max((dir[i] - want[i]).abs());
            }
        }
        assert!(worst <= PLANET_FRAME_TOLERANCE_M, "tangent direction differs from spoint by up to {worst}");
    }

    #[test]
    fn frame_geometry_matches_the_spoint_frame() {
        let mut worst = 0.0_f64;
        for frame_case in fixture()["frames"].as_array().unwrap() {
            let frame = frame_of(&frame_case["frame"]);
            for case in frame_case["cases"].as_array().unwrap() {
                let (x, y, z) = (case["x"].as_f64().unwrap(), case["y"].as_f64().unwrap(), case["z"].as_f64().unwrap());
                if case.get("waterline").is_some() {
                    compare(&mut worst, frame.waterline_local_y(x, z), &case["waterline"]);
                }
                if case.get("surface").is_some() {
                    compare(&mut worst, frame.sphere_surface_local_y(x, z, case["elevation"].as_f64().unwrap()), &case["surface"]);
                }
                compare(&mut worst, frame.elevation_at_local(x, y, z), &case["elevationAtLocal"]);
            }
        }
        assert!(worst <= PLANET_FRAME_TOLERANCE_M, "frame geometry differs from spoint by up to {worst} m");
    }
}
