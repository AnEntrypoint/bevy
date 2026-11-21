#[cfg_attr(not(test), allow(dead_code))]
pub const TERRAIN_FIELD_NAMES: [&str; 3] = ["terrain:height", "terrain:water", "terrain:natural-mask"];
#[cfg_attr(not(test), allow(dead_code))]
pub const FIELD_LAYER_NAMES: [&str; 2] = ["PLAYER", "CREATOR"];
#[cfg_attr(not(test), allow(dead_code))]
pub const PAINTED_HEIGHT_FIELD: &str = "terrain:height";
#[cfg_attr(not(test), allow(dead_code))]
pub const NATURAL_MASK_FIELD: &str = "terrain:natural-mask";

#[cfg(test)]
mod tests {
    use super::{FIELD_LAYER_NAMES, NATURAL_MASK_FIELD, PAINTED_HEIGHT_FIELD, TERRAIN_FIELD_NAMES};
    use serde_json::Value;

    fn fixture() -> Value {
        serde_json::from_str(include_str!("../testdata/parity/spoint-field-names.json")).unwrap()
    }

    fn strings(value: &Value) -> Vec<&str> {
        value.as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect()
    }

    #[test]
    fn field_names_match_the_spoint_exports() {
        let fixture = fixture();
        assert_eq!(TERRAIN_FIELD_NAMES.to_vec(), strings(&fixture["terrainFieldNames"]), "TERRAIN_FIELD_NAMES");
        assert_eq!(FIELD_LAYER_NAMES.to_vec(), strings(&fixture["fieldLayerNames"]), "FIELD_LAYER_NAMES");
        assert_eq!(PAINTED_HEIGHT_FIELD, fixture["paintedHeightField"].as_str().unwrap(), "PAINTED_HEIGHT_FIELD");
        assert_eq!(NATURAL_MASK_FIELD, fixture["naturalMaskField"].as_str().unwrap(), "NATURAL_MASK_FIELD");
    }
}
