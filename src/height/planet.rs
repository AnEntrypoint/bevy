use super::anchor::sea_bias_faces;
use super::height_spec::{Carve, HashVersion, HeightSpec, Params};
use super::ops_jsgen::{Ops, Precision, V3};
use super::planet_frame::{anchor_basis, tangent_local_to_dir};

const RELIEF_DESIGN_RADIUS_M: f64 = 63_600_000.0;
const LAND_BIAS_M: f64 = -100_000.0;

#[derive(Clone, Debug, PartialEq)]
pub enum CarveConfig {
    Parsed { dir: V3, inner_m: f64, outer_m: f64 },
    Centre { x: f64, z: f64, radius_m: f64, falloff_m: f64 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlanetConfig {
    pub seed: i32,
    pub hpf_res: usize,
    pub hash: HashVersion,
    pub relief_scale: Option<f64>,
    pub anchor_dir: V3,
    pub radius: f64,
    pub carves: Vec<CarveConfig>,
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
    hash: HashVersion,
    relief_scale: Option<f64>,
    carves: Vec<ResolvedCarve>,
}

fn precision_of(hash: HashVersion) -> Precision {
    match hash {
        HashVersion::Float => Precision::F32,
        HashVersion::Integer => Precision::F64,
    }
}

fn spec(hpf: &[Vec<f32>], hpf_res: usize, hash: HashVersion, carves: Vec<Carve>) -> HeightSpec<'_> {
    let params = Params { hpf_res: hpf_res as f64, land_bias: LAND_BIAS_M, beach_shelf_m: 0.0, relief_scale: 0.0 };
    HeightSpec::new(hash, precision_of(hash), params, carves, hpf).expect("a planet spec within the carve limit always builds")
}

fn carve_chord2(radius_m: f64, planet_radius: f64) -> f64 {
    let a = radius_m / planet_radius;
    a * a
}

fn centre_dir(anchor: V3, radius: f64, x: f64, z: f64) -> V3 {
    tangent_local_to_dir(&anchor_basis(anchor), radius, x, z)
}

impl Planet {
    pub fn new(config: &PlanetConfig) -> Self {
        let hpf = sea_bias_faces(i64::from(config.seed), config.hpf_res);
        let carves: &[CarveConfig] = if config.hash == HashVersion::Float { &[] } else { &config.carves };
        let resolved: Vec<ResolvedCarve> = {
            let natural = spec(&hpf, config.hpf_res, config.hash, Vec::new());
            carves
                .iter()
                .map(|carve| {
                    let (dir, inner_m, outer_m) = match *carve {
                        CarveConfig::Parsed { dir, inner_m, outer_m } => (Ops::new(Precision::F64).normalize(dir), inner_m, outer_m),
                        CarveConfig::Centre { x, z, radius_m, falloff_m } => (centre_dir(config.anchor_dir, config.radius, x, z), radius_m, radius_m + falloff_m),
                    };
                    ResolvedCarve { dir, inner_m, outer_m, target_h: natural.compose(dir) }
                })
                .collect()
        };
        Self { hpf, hpf_res: config.hpf_res, hash: config.hash, relief_scale: config.relief_scale, carves: resolved }
    }

    pub fn relief(&self, radius: f64) -> f64 {
        self.relief_scale.unwrap_or(radius / RELIEF_DESIGN_RADIUS_M)
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
        spec(&self.hpf, self.hpf_res, self.hash, carves).compose(Ops::new(Precision::F64).normalize(dir)) * self.relief(radius)
    }
}

impl std::fmt::Debug for Planet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Planet")
    }
}
