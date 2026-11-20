use serde_json::Value;

const HF_MAGIC: u32 = 0x48464d31;

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
        let center = centre_of(value);
        let raw = value["heights"].as_array().ok_or_else(|| "malformed: no height samples, expected an array at heights".to_string())?;
        if raw.len() != n * n {
            return Err(format!("sample count mismatch: N={n} declares {} heights, the array holds {}", n * n, raw.len()));
        }
        let heights = raw.iter().map(|h| h.as_f64().unwrap_or(0.0)).collect();
        Ok(Self { n, extent, center, heights })
    }

    pub fn from_hf(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 8 {
            return Err(format!("heightfield-short: {} byte(s) is shorter than the 8-byte prefix", bytes.len()));
        }
        if u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) != HF_MAGIC {
            return Err("heightfield-bad-magic: buffer does not start with the heightfield magic".to_string());
        }
        let hlen = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
        if 8 + hlen > bytes.len() {
            return Err(format!("heightfield-truncated: header of {hlen} byte(s) runs past the {}-byte buffer", bytes.len()));
        }
        let header: Value = serde_json::from_slice(&bytes[8..8 + hlen]).map_err(|e| format!("heightfield-header: {e}"))?;
        let n = header["N"].as_u64().filter(|&n| n >= 2).ok_or_else(|| "heightfield-header: N is not an integer >= 2".to_string())? as usize;
        let extent = header["extent"].as_f64().ok_or_else(|| "heightfield-header: extent is not a number".to_string())?;
        let center = centre_of(&header);
        let sectors = &header["sectors"];
        let grid = sectors["gridS"].as_u64().filter(|&g| g >= 1).ok_or_else(|| "heightfield-header: sectors.gridS is not an integer >= 1".to_string())? as usize;
        let per = sectors["nodesPerSector"].as_u64().filter(|&s| s >= 1).ok_or_else(|| "heightfield-header: sectors.nodesPerSector is not an integer >= 1".to_string())? as usize;
        let bits = sectors["bits"].as_u64().unwrap_or(8);
        if bits != 8 && bits != 16 {
            return Err(format!("heightfield-bits: sectors.bits is {bits}, expected 8 or 16"));
        }
        let sector_count = grid * grid;
        let node_bytes = if bits == 16 { 2 } else { 1 };
        let required = 8 + hlen + 8 * sector_count + node_bytes * n * n;
        if bytes.len() < required {
            return Err(format!("heightfield-truncated: {} byte(s) is short of the {required} the header describes", bytes.len()));
        }
        let f32_at = |offset: usize| f32::from_le_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]]);
        let min_at = 8 + hlen;
        let max_at = min_at + 4 * sector_count;
        let q_at = max_at + 4 * sector_count;
        let qmax = f64::from((1u32 << bits) - 1);
        let mut heights = Vec::with_capacity(n * n);
        for iz in 0..n {
            let sz = (iz / per).min(grid - 1);
            for ix in 0..n {
                let sx = (ix / per).min(grid - 1);
                let si = sz * grid + sx;
                let lo = f64::from(f32_at(min_at + 4 * si));
                let hi = f64::from(f32_at(max_at + 4 * si));
                let node = iz * n + ix;
                let qv = if bits == 16 {
                    f64::from(u16::from_le_bytes([bytes[q_at + 2 * node], bytes[q_at + 2 * node + 1]]))
                } else {
                    f64::from(bytes[q_at + node])
                };
                let value = if hi > lo { lo + (qv / qmax) * (hi - lo) } else { lo };
                heights.push(f64::from(value as f32));
            }
        }
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

fn centre_of(value: &Value) -> [f64; 2] {
    match value["center"].as_array() {
        Some(c) => [c.first().and_then(Value::as_f64).unwrap_or(0.0), c.get(1).and_then(Value::as_f64).unwrap_or(0.0)],
        None => [0.0, 0.0],
    }
}
