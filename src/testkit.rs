use serde_json::Value;

pub fn parse(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

pub fn cases<'a>(fixture: &'a Value, name: &str) -> &'a [Value] {
    fixture[name].as_array().unwrap()
}

pub fn close(got: f64, want: f64, tolerance: f64, context: &str) {
    assert!((got - want).abs() <= tolerance, "{context}: {got} against spoint {want}");
}
