pub fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[cfg(test)]
pub fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn length(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

pub fn normalise(a: [f64; 3]) -> [f64; 3] {
    let len = length(a);
    [a[0] / len, a[1] / len, a[2] / len]
}

#[cfg(test)]
pub fn normalise_checked(a: [f64; 3]) -> Option<[f64; 3]> {
    let len = length(a);
    if len == 0.0 {
        return None;
    }
    let k = 1.0 / len;
    Some([a[0] * k, a[1] * k, a[2] * k])
}
