use serde_json::Value;

pub struct BakedHeightField {
    n: usize,
    extent: f64,
    center: [f64; 2],
    heights: Vec<f64>,
}

impl BakedHeightField {
    pub fn from_json(value: &Value) -> Result<Self, String> {
        let n = value["N"].as_u64().filter(|&n| n >= 2).ok_or_else(|| format!("malformed: N is {}, expected an integer >= 2", value["N"]))? as usize;
        let extent = value["extent"].as_f64().ok_or_else(|| format!("malformed: extent is {}, expected a number", value["extent"]))?;
        let center = match value["center"].as_array() {
            Some(c) => [c.first().and_then(Value::as_f64).unwrap_or(0.0), c.get(1).and_then(Value::as_f64).unwrap_or(0.0)],
            None => [0.0, 0.0],
        };
        let raw = value["heights"].as_array().ok_or_else(|| "malformed: no height samples, expected an array at heights".to_string())?;
        if raw.len() != n * n {
            return Err(format!("sample count mismatch: N={n} declares {} heights, the array holds {}", n * n, raw.len()));
        }
        let heights = raw.iter().map(|h| h.as_f64().unwrap_or(0.0)).collect();
        Ok(Self { n, extent, center, heights })
    }

    pub fn height_at_local(&self, x: f64, z: f64) -> f64 {
        let step = self.extent / (self.n - 1) as f64;
        let half = self.extent / 2.0;
        let fx = (x - self.center[0] + half) / step;
        let fz = (z - self.center[1] + half) / step;
        let (ix, iz) = (fx.floor(), fz.floor());
        let (tx, tz) = (fx - ix, fz - iz);
        let (ix, iz) = (ix as i64, iz as i64);
        let h00 = self.at(ix, iz);
        let h10 = self.at(ix + 1, iz);
        let h01 = self.at(ix, iz + 1);
        let h11 = self.at(ix + 1, iz + 1);
        (h00 * (1.0 - tx) + h10 * tx) * (1.0 - tz) + (h01 * (1.0 - tx) + h11 * tx) * tz
    }

    fn at(&self, ix: i64, iz: i64) -> f64 {
        let last = (self.n - 1) as i64;
        let ix = ix.clamp(0, last) as usize;
        let iz = iz.clamp(0, last) as usize;
        self.heights[iz * self.n + ix]
    }
}
