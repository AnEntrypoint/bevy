use super::anchor::sea_bias_faces;
use super::height_spec::{HashVersion, HeightSpec, Params};
use super::ops_jsgen::{Ops, Precision, V3};

pub const HPF_RES: usize = 128;
const RELIEF_DESIGN_RADIUS_M: f64 = 63_600_000.0;
const LAND_BIAS_M: f64 = -100_000.0;

#[derive(Clone)]
pub struct Planet {
    pub(super) hpf: Vec<Vec<f32>>,
}

impl Planet {
    pub fn new(seed: i32) -> Self {
        Self { hpf: sea_bias_faces(i64::from(seed), HPF_RES) }
    }

    pub fn height(&self, dir: V3, radius: f64) -> f64 {
        let params = Params { hpf_res: HPF_RES as f64, land_bias: LAND_BIAS_M, beach_shelf_m: 0.0, relief_scale: 0.0 };
        let spec = HeightSpec::new(HashVersion::Integer, Precision::F64, params, Vec::new(), &self.hpf)
            .expect("a spec without carves always builds");
        spec.compose(Ops::new(Precision::F64).normalize(dir)) * (radius / RELIEF_DESIGN_RADIUS_M)
    }
}

impl std::fmt::Debug for Planet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Planet")
    }
}
