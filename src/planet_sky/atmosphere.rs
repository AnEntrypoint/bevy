#[derive(Clone, Copy, Debug)]
pub struct AtmosphereParams {
    pub bottom: f64,
    pub top: f64,
    pub rayleigh_h: f64,
    pub mie_h: f64,
    pub rayleigh: [f64; 3],
    pub mie_ext: f64,
}

pub const ATMOSPHERE: AtmosphereParams = AtmosphereParams {
    bottom: 6360.0,
    top: 6500.0,
    rayleigh_h: 18.0,
    mie_h: 4.0,
    rayleigh: [0.005802, 0.013558, 0.0331],
    mie_ext: 0.003996 * (1.0 / 0.9),
};

pub const TRANSMITTANCE_WIDTH: usize = 256;
pub const TRANSMITTANCE_HEIGHT: usize = 128;
pub const TRANSMITTANCE_STEPS: usize = 500;
pub const SCATTERING_WIDTH: usize = 64;
pub const SCATTERING_HEIGHT: usize = 32;
pub const SCATTERING_LAYERS: usize = 24;
pub const SCATTERING_STEPS: usize = 64;
const MU_S_WARP_K: f64 = 1.4;

pub struct TransmittanceLut {
    pub width: usize,
    pub height: usize,
    pub data: Vec<f32>,
}

pub struct ScatteringLut {
    pub width: usize,
    pub height: usize,
    pub layers: usize,
    pub data: Vec<f32>,
}

fn texel_centre(index: usize, count: usize) -> f64 {
    (index as f64 + 0.5) / count as f64
}

fn midpoint(step: f64, index: usize) -> f64 {
    step * (index as f64 + 0.5)
}

fn rho_max(p: &AtmosphereParams) -> f64 {
    (p.top * p.top - p.bottom * p.bottom).sqrt()
}

fn rho_at(p: &AtmosphereParams, r: f64) -> f64 {
    (r * r - p.bottom * p.bottom).max(0.0).sqrt()
}

fn dist_to_top(p: &AtmosphereParams, r: f64, mu: f64) -> f64 {
    let disc = r * r * (mu * mu - 1.0) + p.top * p.top;
    if disc < 0.0 {
        return -1.0;
    }
    (-r * mu + disc.sqrt()).max(0.0)
}

fn dist_to_ground(p: &AtmosphereParams, r: f64, mu: f64) -> f64 {
    let disc = r * r * (mu * mu - 1.0) + p.bottom * p.bottom;
    if disc < 0.0 {
        return -1.0;
    }
    let d = -r * mu - disc.sqrt();
    if d >= 0.0 { d } else { -1.0 }
}

fn densities(p: &AtmosphereParams, altitude: f64) -> (f64, f64) {
    ((-altitude / p.rayleigh_h).exp(), (-altitude / p.mie_h).exp())
}

fn extinction(p: &AtmosphereParams, channel: usize, od_rayleigh: f64, od_mie: f64) -> f64 {
    (-(p.rayleigh[channel] * od_rayleigh + p.mie_ext * od_mie)).exp()
}

fn view_dir(mu: f64) -> [f64; 2] {
    [(1.0 - mu * mu).max(0.0).sqrt(), mu]
}

pub fn transmittance_at(p: &AtmosphereParams, r: f64, mu: f64, steps: usize) -> [f64; 3] {
    let d = dist_to_top(p, r, mu);
    if d <= 0.0 {
        return [1.0; 3];
    }
    let [dx, dz] = view_dir(mu);
    let dt = d / steps as f64;
    let (mut od_rayleigh, mut od_mie) = (0.0, 0.0);
    for i in 0..steps {
        let t = midpoint(dt, i);
        let pr = (dx * t).hypot(r + dz * t);
        let (rayleigh, mie) = densities(p, pr - p.bottom);
        od_rayleigh += rayleigh * dt;
        od_mie += mie * dt;
    }
    [0, 1, 2].map(|c| extinction(p, c, od_rayleigh, od_mie))
}

pub fn u_from_r(p: &AtmosphereParams, r: f64) -> f64 {
    (rho_at(p, r) / rho_max(p)).clamp(0.0, 1.0)
}

fn r_from_u(p: &AtmosphereParams, u: f64) -> f64 {
    let rho = u.clamp(0.0, 1.0) * rho_max(p);
    (rho * rho + p.bottom * p.bottom).sqrt()
}

pub fn v_from_mu(p: &AtmosphereParams, mu: f64, r: f64) -> f64 {
    let d_min = p.top - r;
    let d_max = rho_at(p, r) + rho_max(p);
    let d = dist_to_top(p, r, mu);
    if d < 0.0 || d_max <= d_min {
        return 0.0;
    }
    ((d - d_min) / (d_max - d_min)).clamp(0.0, 1.0)
}

fn mu_from_v(p: &AtmosphereParams, v: f64, r: f64) -> f64 {
    let d_min = p.top - r;
    let d_max = rho_at(p, r) + rho_max(p);
    let d = d_min + v.clamp(0.0, 1.0) * (d_max - d_min);
    if d <= 0.0 {
        return 1.0;
    }
    ((p.top * p.top - r * r - d * d) / (2.0 * d * r)).clamp(-1.0, 1.0)
}

pub fn mu_s_from_layer(layer: usize, layers: usize) -> f64 {
    let w = texel_centre(layer, layers);
    let t = w.clamp(0.0, 1.0) * 2.0 - 1.0;
    ((MU_S_WARP_K * t).tanh() / MU_S_WARP_K.tanh()).clamp(-1.0, 1.0)
}

#[cfg(test)]
pub fn layer_from_mu_s(mu_s: f64, layers: usize) -> usize {
    let t = (mu_s.clamp(-0.999999, 0.999999) * MU_S_WARP_K.tanh()).atanh() / MU_S_WARP_K;
    let w = ((t + 1.0) * 0.5).clamp(0.0, 1.0);
    ((w * layers as f64).floor().clamp(0.0, layers as f64 - 1.0)) as usize
}

pub fn bake_transmittance(p: &AtmosphereParams, width: usize, height: usize, steps: usize) -> TransmittanceLut {
    let mut data = Vec::with_capacity(width * height * 3);
    for y in 0..height {
        let v = texel_centre(y, height);
        for x in 0..width {
            let r = r_from_u(p, texel_centre(x, width));
            let mu = mu_from_v(p, v, r);
            data.extend(transmittance_at(p, r, mu, steps).map(|t| t as f32));
        }
    }
    TransmittanceLut { width, height, data }
}

fn sample_transmittance(lut: &TransmittanceLut, p: &AtmosphereParams, r: f64, mu: f64) -> [f64; 3] {
    let (w, h) = (lut.width, lut.height);
    let fx = u_from_r(p, r) * w as f64 - 0.5;
    let fy = v_from_mu(p, mu, r) * h as f64 - 0.5;
    let x0 = fx.floor().clamp(0.0, (w - 1) as f64) as usize;
    let y0 = fy.floor().clamp(0.0, (h - 1) as f64) as usize;
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let tx = (fx - fx.floor()).clamp(0.0, 1.0);
    let ty = (fy - fy.floor()).clamp(0.0, 1.0);
    let texel = |x: usize, y: usize, c: usize| lut.data[(y * w + x) * 3 + c] as f64;
    [0, 1, 2].map(|c| {
        let top = texel(x0, y0, c) * (1.0 - tx) + texel(x1, y0, c) * tx;
        let bottom = texel(x0, y1, c) * (1.0 - tx) + texel(x1, y1, c) * tx;
        top * (1.0 - ty) + bottom * ty
    })
}

pub fn inscatter_at(
    p: &AtmosphereParams,
    r: f64,
    mu: f64,
    mu_s: f64,
    transmittance: &TransmittanceLut,
    steps: usize,
) -> ([f64; 3], f64) {
    let d_top = dist_to_top(p, r, mu);
    if d_top <= 0.0 {
        return ([0.0; 3], 0.0);
    }
    let d_ground = dist_to_ground(p, r, mu);
    let d = if d_ground > 0.0 && d_ground < d_top { d_ground } else { d_top };
    let [vx, vz] = view_dir(mu);
    let [sx, sz] = view_dir(mu_s);
    let dt = d / steps as f64;
    let mut rayleigh_sum = [0.0; 3];
    let mut mie_sum = 0.0;
    let (mut od_rayleigh, mut od_mie) = (0.0, 0.0);
    for i in 0..steps {
        let t = midpoint(dt, i);
        let (px, pz) = (vx * t, r + vz * t);
        let pr = px.hypot(pz);
        let (rayleigh, mie) = densities(p, pr.max(p.bottom) - p.bottom);
        let (d_rayleigh, d_mie) = (rayleigh * dt, mie * dt);
        od_rayleigh += d_rayleigh;
        od_mie += d_mie;
        let view = [0, 1, 2].map(|c| extinction(p, c, od_rayleigh, od_mie));
        let sun = sample_transmittance(transmittance, p, pr, (px * sx + pz * sz) / pr);
        let mut mean = 0.0;
        for c in 0..3 {
            let attenuation = view[c] * sun[c];
            rayleigh_sum[c] += attenuation * d_rayleigh;
            mean += attenuation;
        }
        mie_sum += mean / 3.0 * d_mie;
    }
    (rayleigh_sum, mie_sum)
}

pub fn bake_scattering(
    p: &AtmosphereParams,
    width: usize,
    height: usize,
    layers: usize,
    steps: usize,
    transmittance: &TransmittanceLut,
) -> ScatteringLut {
    let mut data = Vec::with_capacity(width * height * layers * 4);
    for layer in 0..layers {
        let mu_s = mu_s_from_layer(layer, layers);
        for y in 0..height {
            let v = texel_centre(y, height);
            for x in 0..width {
                let r = r_from_u(p, texel_centre(x, width));
                let mu = mu_from_v(p, v, r);
                let (rayleigh, mie) = inscatter_at(p, r, mu, mu_s, transmittance, steps);
                data.extend(rayleigh.map(|c| c as f32));
                data.push(mie as f32);
            }
        }
    }
    ScatteringLut { width, height, layers, data }
}

#[cfg(test)]
pub fn sample_indices(count: usize, stride: usize) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..count).step_by(stride).collect();
    if indices.last() != Some(&(count - 1)) {
        indices.push(count - 1);
    }
    indices
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{cases, close, parse};
    use serde_json::Value;
    use std::sync::OnceLock;

    const PROBE_TOLERANCE: f64 = 1e-9;
    const TABLE_TOLERANCE: f64 = 1e-6;
    const TABLE_FLOOR: f64 = 1e-30;

    fn fixture() -> &'static Value {
        static FIXTURE: OnceLock<Value> = OnceLock::new();
        FIXTURE.get_or_init(|| parse(include_str!("../../testdata/parity/spoint-planet-atmosphere.json")))
    }

    fn transmittance_lut() -> &'static TransmittanceLut {
        static LUT: OnceLock<TransmittanceLut> = OnceLock::new();
        LUT.get_or_init(|| bake_transmittance(&ATMOSPHERE, TRANSMITTANCE_WIDTH, TRANSMITTANCE_HEIGHT, TRANSMITTANCE_STEPS))
    }

    fn scattering_lut() -> &'static ScatteringLut {
        static LUT: OnceLock<ScatteringLut> = OnceLock::new();
        LUT.get_or_init(|| {
            bake_scattering(&ATMOSPHERE, SCATTERING_WIDTH, SCATTERING_HEIGHT, SCATTERING_LAYERS, SCATTERING_STEPS, transmittance_lut())
        })
    }

    fn number(value: &Value) -> f64 {
        value.as_f64().unwrap()
    }

    fn numbers(value: &Value) -> Vec<f64> {
        value.as_array().unwrap().iter().map(number).collect()
    }

    fn indices(value: &Value) -> Vec<usize> {
        numbers(value).into_iter().map(|n| n as usize).collect()
    }

    fn close_relative(got: f64, want: f64, relative: f64, context: &str) {
        close(got, want, relative * want.abs(), context);
    }

    fn close_table_entry(got: f32, want: f64, context: &str) {
        close(got as f64, want, TABLE_TOLERANCE * want.abs() + TABLE_FLOOR, context);
    }

    #[test]
    fn constants_match_the_fixture() {
        let c = &fixture()["constants"];
        assert_eq!(ATMOSPHERE.bottom, number(&c["bottom"]));
        assert_eq!(ATMOSPHERE.top, number(&c["top"]));
        assert_eq!(ATMOSPHERE.rayleigh_h, number(&c["rayleighScaleHeight"]));
        assert_eq!(ATMOSPHERE.mie_h, number(&c["mieScaleHeight"]));
        assert_eq!(ATMOSPHERE.rayleigh.to_vec(), numbers(&c["rayleigh"]));
        close_relative(ATMOSPHERE.mie_ext, number(&c["mieExtinction"]), 1e-12, "mie extinction");
        assert_eq!(TRANSMITTANCE_WIDTH as f64, number(&c["transmittanceWidth"]));
        assert_eq!(TRANSMITTANCE_HEIGHT as f64, number(&c["transmittanceHeight"]));
        assert_eq!(SCATTERING_WIDTH as f64, number(&c["scatteringWidth"]));
        assert_eq!(SCATTERING_HEIGHT as f64, number(&c["scatteringHeight"]));
        assert_eq!(SCATTERING_LAYERS as f64, number(&c["scatteringLayers"]));
        assert_eq!(TRANSMITTANCE_STEPS as f64, number(&fixture()["transmittanceProbes"]["steps"]));
        assert_eq!(SCATTERING_STEPS as f64, number(&fixture()["inscatterProbes"]["steps"]));
    }

    #[test]
    fn transmittance_probes_match_the_reference() {
        let all = cases(&fixture()["transmittanceProbes"], "cases");
        assert_eq!(all.len(), 28);
        for case in all {
            let (r, mu) = (number(&case["r"]), number(&case["mu"]));
            let context = format!("transmittance r {r} mu {mu}");
            let got = transmittance_at(&ATMOSPHERE, r, mu, TRANSMITTANCE_STEPS);
            for (c, want) in numbers(&case["transmittance"]).into_iter().enumerate() {
                close_relative(got[c], want, PROBE_TOLERANCE, &format!("{context} channel {c}"));
            }
            close(u_from_r(&ATMOSPHERE, r), number(&case["u"]), 1e-12, &format!("{context} u"));
            close(v_from_mu(&ATMOSPHERE, mu, r), number(&case["v"]), 1e-12, &format!("{context} v"));
        }
    }

    #[test]
    fn inscatter_probes_match_the_reference() {
        let all = cases(&fixture()["inscatterProbes"], "cases");
        assert_eq!(all.len(), 60);
        for case in all {
            let (r, mu, mu_s) = (number(&case["r"]), number(&case["mu"]), number(&case["muS"]));
            let context = format!("inscatter r {r} mu {mu} muS {mu_s}");
            let (rayleigh, mie) = inscatter_at(&ATMOSPHERE, r, mu, mu_s, transmittance_lut(), SCATTERING_STEPS);
            for (c, want) in numbers(&case["inscatR"]).into_iter().enumerate() {
                close_relative(rayleigh[c], want, PROBE_TOLERANCE, &format!("{context} channel {c}"));
            }
            close_relative(mie, number(&case["inscatM"]), PROBE_TOLERANCE, &format!("{context} mie"));
        }
    }

    #[test]
    fn layer_mapping_matches_the_reference() {
        let layers = &fixture()["muSLayers"];
        let mu_s = numbers(&layers["muS"]);
        let back = indices(&layers["layerFromMuS"]);
        assert_eq!(mu_s.len(), SCATTERING_LAYERS);
        for layer in 0..SCATTERING_LAYERS {
            close_relative(mu_s_from_layer(layer, SCATTERING_LAYERS), mu_s[layer], 1e-12, &format!("layer {layer}"));
            assert_eq!(layer_from_mu_s(mu_s[layer], SCATTERING_LAYERS), back[layer], "layer {layer}");
            assert_eq!(back[layer], layer);
        }
    }

    #[test]
    fn sample_index_rule_matches_the_fixture() {
        let t = &fixture()["transmittanceTable"]["sample"];
        let s = &fixture()["scatteringTable"]["sample"];
        assert_eq!(sample_indices(TRANSMITTANCE_WIDTH, 8), indices(&t["xs"]));
        assert_eq!(sample_indices(TRANSMITTANCE_HEIGHT, 4), indices(&t["ys"]));
        assert_eq!(sample_indices(SCATTERING_WIDTH, 4), indices(&s["xs"]));
        assert_eq!(sample_indices(SCATTERING_HEIGHT, 4), indices(&s["ys"]));
        assert_eq!(sample_indices(SCATTERING_LAYERS, 3), indices(&s["layers"]));
        assert_eq!(sample_indices(9, 4), vec![0, 4, 8]);
        assert_eq!(sample_indices(10, 4), vec![0, 4, 8, 9]);
        assert_eq!(sample_indices(1, 5), vec![0]);
    }

    #[test]
    fn transmittance_table_matches_the_reference_samples() {
        let table = &fixture()["transmittanceTable"];
        let lut = transmittance_lut();
        assert_eq!((lut.width, lut.height), (TRANSMITTANCE_WIDTH, TRANSMITTANCE_HEIGHT));
        assert_eq!(lut.data.len() as f64, number(&table["entries"]));
        let sample = &table["sample"];
        let want = numbers(&sample["values"]);
        let (xs, ys) = (indices(&sample["xs"]), indices(&sample["ys"]));
        assert_eq!(want.len(), xs.len() * ys.len() * 3);
        let mut k = 0;
        for &y in &ys {
            for &x in &xs {
                for c in 0..3 {
                    close_table_entry(lut.data[(y * lut.width + x) * 3 + c], want[k], &format!("transmittance x {x} y {y} c {c}"));
                    k += 1;
                }
            }
        }
    }

    #[test]
    fn scattering_table_matches_the_reference_samples() {
        let table = &fixture()["scatteringTable"];
        let lut = scattering_lut();
        assert_eq!((lut.width, lut.height, lut.layers), (SCATTERING_WIDTH, SCATTERING_HEIGHT, SCATTERING_LAYERS));
        assert_eq!(lut.data.len() as f64, number(&table["entries"]));
        let sample = &table["sample"];
        let want = numbers(&sample["values"]);
        let (xs, ys, layers) = (indices(&sample["xs"]), indices(&sample["ys"]), indices(&sample["layers"]));
        assert_eq!(want.len(), xs.len() * ys.len() * layers.len() * 4);
        let mut k = 0;
        for &l in &layers {
            for &y in &ys {
                for &x in &xs {
                    for c in 0..4 {
                        close_table_entry(lut.data[((l * lut.height + y) * lut.width + x) * 4 + c], want[k], &format!("scattering x {x} y {y} layer {l} c {c}"));
                        k += 1;
                    }
                }
            }
        }
    }
}
