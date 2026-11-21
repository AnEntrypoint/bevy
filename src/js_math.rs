#[cfg(test)]
pub fn js_round(x: f64) -> f64 {
    let floor = x.floor();
    if x - floor >= 0.5 {
        floor + 1.0
    } else {
        floor
    }
}

pub fn round_half_up(x: f64) -> f64 {
    (x + 0.5).floor()
}
