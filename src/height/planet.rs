use super::anchor::sea_bias_faces;
use super::height_spec::{Carve, HashVersion, HeightSpec, Params};
use super::ops_jsgen::{Ops, Precision, V3};

const RELIEF_DESIGN_RADIUS_M: f64 = 63_600_000.0;
const LAND_BIAS_M: f64 = -100_000.0;

#[derive(Clone, Debug)]
pub struct CarveConfig {
    pub dir: V3,
    pub inner_m: f64,
    pub outer_m: f64,
}

#[derive(Clone)]
struct ResolvedCarve {
    dir: V3,
    inner_m: f64,
    outer_m: f64,
    target_h: f64,
}

#[derive(Clone)]
pub struct Planet {
    pub(super) hpf: Vec<Vec<f32>>,
    hpf_res: usize,
    carves: Vec<ResolvedCarve>,
}

fn spec(hpf: &[Vec<f32>], hpf_res: usize, carves: Vec<Carve>) -> HeightSpec<'_> {
    let params = Params { hpf_res: hpf_res as f64, land_bias: LAND_BIAS_M, beach_shelf_m: 0.0, relief_scale: 0.0 };
    HeightSpec::new(HashVersion::Integer, Precision::F64, params, carves, hpf).expect("a planet spec within the carve limit always builds")
}

fn carve_chord2(radius_m: f64, planet_radius: f64) -> f64 {
    let a = radius_m / planet_radius;
    a * a
}

impl Planet {
    pub fn new(seed: i32, hpf_res: usize, carves: &[CarveConfig]) -> Self {
        let hpf = sea_bias_faces(i64::from(seed), hpf_res);
        let natural = spec(&hpf, hpf_res, Vec::new());
        let carves = carves
            .iter()
            .map(|carve| {
                let dir = Ops::new(Precision::F64).normalize(carve.dir);
                ResolvedCarve { dir, inner_m: carve.inner_m, outer_m: carve.outer_m, target_h: natural.compose(dir) }
            })
            .collect();
        Self { hpf, hpf_res, carves }
    }

    pub fn height(&self, dir: V3, radius: f64) -> f64 {
        let carves = self
            .carves
            .iter()
            .map(|carve| Carve {
                dir: carve.dir,
                inner_chord2: carve_chord2(carve.inner_m, radius),
                outer_chord2: carve_chord2(carve.outer_m, radius),
                target_h: carve.target_h,
            })
            .collect();
        spec(&self.hpf, self.hpf_res, carves).compose(Ops::new(Precision::F64).normalize(dir)) * (radius / RELIEF_DESIGN_RADIUS_M)
    }
}

impl std::fmt::Debug for Planet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Planet")
    }
}
