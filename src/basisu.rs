use serde_json::{Map, Value};

const EXTENSION: &str = "KHR_texture_basisu";

pub fn promote(json: &mut Value) -> usize {
    let mut promoted = 0;
    if let Some(textures) = json.get_mut("textures").and_then(Value::as_array_mut) {
        for texture in textures.iter_mut() {
            if promote_texture(texture) {
                promoted += 1;
            }
        }
    }
    if !any_texture_uses_extension(json) {
        retire_extension_name(json, "extensionsUsed");
        retire_extension_name(json, "extensionsRequired");
    }
    promoted
}

fn promote_texture(texture: &mut Value) -> bool {
    let Some(object) = texture.as_object_mut() else {
        return false;
    };
    let Some(source) = basisu_source(object) else {
        return false;
    };
    let Some(extensions) = object.get_mut("extensions").and_then(Value::as_object_mut) else {
        return false;
    };
    extensions.remove(EXTENSION);
    let extensions_empty = extensions.is_empty();
    if extensions_empty {
        object.remove("extensions");
    }
    object.insert("source".to_owned(), Value::from(source));
    true
}

fn basisu_source(texture: &Map<String, Value>) -> Option<u64> {
    texture
        .get("extensions")?
        .get(EXTENSION)?
        .get("source")?
        .as_u64()
}

fn any_texture_uses_extension(json: &Value) -> bool {
    json.get("textures")
        .and_then(Value::as_array)
        .is_some_and(|textures| {
            textures.iter().any(|texture| {
                texture
                    .get("extensions")
                    .and_then(|extensions| extensions.get(EXTENSION))
                    .is_some()
            })
        })
}

fn retire_extension_name(json: &mut Value, list: &str) {
    if let Some(names) = json.get_mut(list).and_then(Value::as_array_mut) {
        names.retain(|name| name.as_str() != Some(EXTENSION));
    }
}

#[cfg(test)]
mod tests {
    use super::promote;
    use serde_json::json;

    #[test]
    fn basisu_texture_takes_its_source_index() {
        let mut document = json!({
            "images": [{"mimeType": "image/ktx2", "uri": "a.ktx2"}],
            "textures": [{"extensions": {"KHR_texture_basisu": {"source": 0}}}]
        });
        assert_eq!(promote(&mut document), 1);
        assert_eq!(document["textures"][0], json!({"source": 0}));
    }

    #[test]
    fn texture_without_extension_is_untouched() {
        let mut document = json!({"textures": [{"source": 2, "sampler": 0}]});
        assert_eq!(promote(&mut document), 0);
        assert_eq!(document, json!({"textures": [{"source": 2, "sampler": 0}]}));
    }

    #[test]
    fn extension_is_removed_from_used_and_required_lists() {
        let mut document = json!({
            "extensionsUsed": ["KHR_materials_unlit", "KHR_texture_basisu"],
            "extensionsRequired": ["KHR_texture_basisu"],
            "textures": [{"extensions": {"KHR_texture_basisu": {"source": 1}}}]
        });
        assert_eq!(promote(&mut document), 1);
        assert_eq!(document["extensionsUsed"], json!(["KHR_materials_unlit"]));
        assert_eq!(document["extensionsRequired"], json!([]));
    }

    #[test]
    fn empty_extensions_object_is_removed() {
        let mut document = json!({
            "textures": [{"name": "t", "extensions": {"KHR_texture_basisu": {"source": 3}}}]
        });
        assert_eq!(promote(&mut document), 1);
        assert_eq!(document["textures"][0], json!({"name": "t", "source": 3}));
        assert!(document["textures"][0].get("extensions").is_none());
    }

    #[test]
    fn other_texture_extensions_survive_promotion() {
        let mut document = json!({
            "extensionsUsed": ["KHR_texture_basisu", "EXT_texture_webp"],
            "textures": [{"extensions": {
                "KHR_texture_basisu": {"source": 1},
                "EXT_texture_webp": {"source": 4}
            }}]
        });
        assert_eq!(promote(&mut document), 1);
        assert_eq!(
            document["textures"][0],
            json!({"source": 1, "extensions": {"EXT_texture_webp": {"source": 4}}})
        );
        assert_eq!(document["extensionsUsed"], json!(["EXT_texture_webp"]));
    }

    #[test]
    fn extension_stays_listed_while_a_texture_still_uses_it() {
        let mut document = json!({
            "extensionsUsed": ["KHR_texture_basisu"],
            "textures": [
                {"extensions": {"KHR_texture_basisu": {"source": 0}}},
                {"extensions": {"KHR_texture_basisu": {}}}
            ]
        });
        assert_eq!(promote(&mut document), 1);
        assert_eq!(document["extensionsUsed"], json!(["KHR_texture_basisu"]));
        assert_eq!(
            document["textures"][1],
            json!({"extensions": {"KHR_texture_basisu": {}}})
        );
    }
}
