#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Precision {
    F32,
    F64,
}

pub type V3 = [f64; 3];

#[derive(Clone, Copy, Debug)]
pub struct Ops {
    single: bool,
}

pub fn u32_of(value: f64) -> u32 {
    (value.trunc() as i64) as u32
}

impl Ops {
    pub fn new(precision: Precision) -> Self {
        Self { single: precision == Precision::F32 }
    }

    pub fn round(self, value: f64) -> f64 {
        if self.single { value as f32 as f64 } else { value }
    }

    pub fn lit(self, value: f64) -> f64 {
        if value.fract() == 0.0 { value } else { self.round(value) }
    }

    pub fn add(self, a: f64, b: f64) -> f64 {
        self.round(a + b)
    }

    pub fn sub(self, a: f64, b: f64) -> f64 {
        self.round(a - b)
    }

    pub fn mul(self, a: f64, b: f64) -> f64 {
        self.round(a * b)
    }

    pub fn div(self, a: f64, b: f64) -> f64 {
        self.round(a / b)
    }

    pub fn neg(self, a: f64) -> f64 {
        self.round(-a)
    }

    pub fn floor(self, a: f64) -> f64 {
        self.round(a.floor())
    }

    pub fn fract(self, a: f64) -> f64 {
        a - a.floor()
    }

    pub fn abs(self, a: f64) -> f64 {
        self.round(a.abs())
    }

    pub fn pow(self, a: f64, b: f64) -> f64 {
        self.round(a.powf(b))
    }

    pub fn sin(self, a: f64) -> f64 {
        self.round(a.sin())
    }

    pub fn cos(self, a: f64) -> f64 {
        self.round(a.cos())
    }

    pub fn max(self, a: f64, b: f64) -> f64 {
        self.round(a.max(b))
    }

    pub fn min(self, a: f64, b: f64) -> f64 {
        self.round(a.min(b))
    }

    pub fn clamp(self, x: f64, lo: f64, hi: f64) -> f64 {
        self.round(x.max(lo).min(hi))
    }

    pub fn mix(self, a: f64, b: f64, t: f64) -> f64 {
        self.add(a, self.mul(self.sub(b, a), t))
    }

    pub fn dot(self, a: V3, b: V3) -> f64 {
        let s = self.mul(a[0], b[0]);
        let s = self.add(s, self.mul(a[1], b[1]));
        self.add(s, self.mul(a[2], b[2]))
    }

    pub fn ufloat(self, a: f64) -> f64 {
        self.round(a)
    }

    pub fn normalize(self, a: V3) -> V3 {
        let length = self.round(self.dot(a, a).sqrt());
        let length = if length == 0.0 || length.is_nan() { 1.0 } else { length };
        a.map(|c| self.div(c, length))
    }
}
