#[derive(Clone, Debug, PartialEq)]
pub struct Refusal {
    pub name: &'static str,
    pub message: String,
}
