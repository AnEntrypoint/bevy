#![cfg_attr(not(test), allow(dead_code))]

#[derive(Clone, Copy, Debug)]
pub struct FlightOpts {
    pub air_density: f64,
    pub wing_area: f64,
    pub lift_slope: f64,
    pub max_lift_coeff: f64,
    pub parasite_drag: f64,
    pub induced_drag_factor: f64,
    pub thrust: f64,
    pub stall_speed: f64,
}

impl Default for FlightOpts {
    fn default() -> Self {
        Self {
            air_density: 1.225,
            wing_area: 10.0,
            lift_slope: 4.5,
            max_lift_coeff: 1.4,
            parasite_drag: 0.03,
            induced_drag_factor: 0.04,
            thrust: 0.0,
            stall_speed: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FlightForces {
    pub force: [f64; 3],
    pub speed: f64,
    pub aoa: f64,
    pub lift_coeff: f64,
    pub stalled: bool,
}

pub fn rotate_by_quat(q: [f64; 4], v: [f64; 3]) -> [f64; 3] {
    let [qx, qy, qz, qw] = q;
    let [vx, vy, vz] = v;
    let tx = 2.0 * (qy * vz - qz * vy);
    let ty = 2.0 * (qz * vx - qx * vz);
    let tz = 2.0 * (qx * vy - qy * vx);
    [
        vx + qw * tx + (qy * tz - qz * ty),
        vy + qw * ty + (qz * tx - qx * tz),
        vz + qw * tz + (qx * ty - qy * tx),
    ]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn flight_forces(velocity: [f64; 3], rotation: [f64; 4], p: &FlightOpts) -> FlightForces {
    let speed = dot(velocity, velocity).sqrt();
    let forward = rotate_by_quat(rotation, [0.0, 0.0, 1.0]);
    let up = rotate_by_quat(rotation, [0.0, 1.0, 0.0]);
    let q_dyn = 0.5 * p.air_density * speed * speed;
    let inv = if speed > 0.0 { 1.0 / speed } else { 0.0 };
    let v_hat = velocity.map(|c| c * inv);
    let aoa = dot(forward, v_hat).clamp(-1.0, 1.0).acos();
    let up_dot_v = dot(up, v_hat);
    let aoa_sign = if up_dot_v < 0.0 { -1.0 } else { 1.0 };
    let lift_coeff = p.max_lift_coeff.min(p.lift_slope * aoa.sin()) * -aoa_sign;
    let lift_magnitude = lift_coeff * q_dyn * p.wing_area;
    let lift_dir: [f64; 3] = std::array::from_fn(|i| up[i] - v_hat[i] * up_dot_v);
    let lift_len = dot(lift_dir, lift_dir).sqrt();
    let lift_len = if lift_len > 0.0 { lift_len } else { 1.0 };
    let cd = p.parasite_drag + p.induced_drag_factor * lift_coeff * lift_coeff;
    let drag_magnitude = cd * q_dyn * p.wing_area;
    let force = std::array::from_fn(|i| (lift_dir[i] / lift_len) * lift_magnitude - v_hat[i] * drag_magnitude + forward[i] * p.thrust);
    FlightForces { force, speed, aoa, lift_coeff, stalled: speed < p.stall_speed }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const TOLERANCE: f64 = 1e-9;

    fn triple(v: &Value) -> [f64; 3] {
        [v[0].as_f64().unwrap(), v[1].as_f64().unwrap(), v[2].as_f64().unwrap()]
    }

    fn quad(v: &Value) -> [f64; 4] {
        [v[0].as_f64().unwrap(), v[1].as_f64().unwrap(), v[2].as_f64().unwrap(), v[3].as_f64().unwrap()]
    }

    fn opts(v: &Value) -> FlightOpts {
        let d = FlightOpts::default();
        let field = |name: &str, fallback: f64| v.get(name).and_then(Value::as_f64).unwrap_or(fallback);
        FlightOpts {
            air_density: field("airDensity", d.air_density),
            wing_area: field("wingArea", d.wing_area),
            lift_slope: field("liftSlope", d.lift_slope),
            max_lift_coeff: field("maxLiftCoeff", d.max_lift_coeff),
            parasite_drag: field("parasiteDrag", d.parasite_drag),
            induced_drag_factor: field("inducedDragFactor", d.induced_drag_factor),
            thrust: field("thrust", d.thrust),
            stall_speed: field("stallSpeed", d.stall_speed),
        }
    }

    fn assert_close(got: &[f64], want: &[f64], what: &str) {
        assert_eq!(got.len(), want.len(), "{what}");
        for k in 0..got.len() {
            assert!((got[k] - want[k]).abs() <= TOLERANCE, "{what}: {got:?} against spoint {want:?}");
        }
    }

    fn assert_scalar(got: f64, want: &Value, what: &str) {
        let want = want.as_f64().unwrap();
        assert!((got - want).abs() <= TOLERANCE, "{what}: {got} against spoint {want}");
    }

    #[test]
    fn flight_model_matches_spoint() {
        let fixture: Value = serde_json::from_str(include_str!("../testdata/parity/spoint-flight-model.json")).unwrap();
        for case in fixture["rotate"].as_array().unwrap() {
            let got = rotate_by_quat(quad(&case["q"]), triple(&case["v"]));
            assert_close(&got, &triple(&case["expect"]), "rotateByQuat");
        }
        for case in fixture["flight"].as_array().unwrap() {
            let got = flight_forces(triple(&case["velocity"]), quad(&case["rotation"]), &opts(&case["opts"]));
            let want = &case["expect"];
            assert_close(&got.force, &triple(&want["force"]), "flightForces force");
            assert_scalar(got.speed, &want["speed"], "flightForces speed");
            assert_scalar(got.aoa, &want["aoa"], "flightForces aoa");
            assert_scalar(got.lift_coeff, &want["liftCoeff"], "flightForces liftCoeff");
            assert_eq!(got.stalled, want["stalled"].as_bool().unwrap(), "flightForces stalled");
        }
    }
}
