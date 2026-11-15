use crate::spec::{ObjectSpec, PlaceDef, WorldConfig};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct Place {
    pub name: String,
    pub config: PlaceDef,
    pub objects: Vec<ObjectSpec>,
}

#[derive(Clone, Debug)]
pub struct WorldTree {
    pub config: WorldConfig,
    pub places: BTreeMap<String, Place>,
}

impl WorldTree {
    pub fn load(root: &Path) -> Result<Self, String> {
        let config_path = root.join("world.config.yaml");
        let config_text = fs::read_to_string(&config_path)
            .map_err(|error| format!("{}: {error}", config_path.display()))?;
        let config: WorldConfig = serde_yaml::from_str(&config_text)
            .map_err(|error| format!("{}: {error}", config_path.display()))?;
        let mut places = BTreeMap::new();
        let places_dir = root.join("places");
        if places_dir.is_dir() {
            let mut directories = fs::read_dir(&places_dir)
                .map_err(|error| format!("{}: {error}", places_dir.display()))?
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| path.is_dir())
                .collect::<Vec<_>>();
            directories.sort();
            for directory in directories {
                let name = directory
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_default();
                let place = load_place(&name, &directory)?;
                places.insert(name, place);
            }
        }
        Ok(Self {

            config,
            places,
        })
    }
}

fn load_place(name: &str, directory: &Path) -> Result<Place, String> {
    let config_path = directory.join("config.yaml");
    let config = if config_path.is_file() {
        let text = fs::read_to_string(&config_path)
            .map_err(|error| format!("{}: {error}", config_path.display()))?;
        serde_yaml::from_str(&text)
            .map_err(|error| format!("{}: {error}", config_path.display()))?
    } else {
        PlaceDef::default()
    };
    let mut objects = Vec::new();
    let cells = directory.join("cells");
    if cells.is_dir() {
        let mut files = fs::read_dir(&cells)
            .map_err(|error| format!("{}: {error}", cells.display()))?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|extension| extension.to_string_lossy() == "scene"))
            .collect::<Vec<_>>();
        files.sort();
        for file in files {
            objects.extend(load_scene(&file)?);
        }
    }
    let loose = directory.join("objects.scene");
    if loose.is_file() {
        objects.extend(load_scene(&loose)?);
    }
    for object in &mut objects {
        if object.place.is_none() {
            object.place = Some(name.to_string());
        }
    }
    Ok(Place {
        name: name.to_string(),
        config,
        objects,
    })
}

fn load_scene(path: &Path) -> Result<Vec<ObjectSpec>, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    let body = text
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    if body.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut map: BTreeMap<String, Value> = serde_yaml::from_str(&body)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    let mut objects = Vec::new();
    for (id, value) in std::mem::take(&mut map) {
        objects.push(object_from_value(id, value, path)?);
    }
    Ok(objects)
}

fn object_from_value(id: String, mut value: Value, path: &Path) -> Result<ObjectSpec, String> {
    if let Value::Object(object) = &mut value {
        object
            .entry("id".to_string())
            .or_insert_with(|| Value::String(id.clone()));
    }
    let mut spec: ObjectSpec = serde_json::from_value(value)
        .map_err(|error| format!("{}: object {id}: {error}", path.display()))?;
    if spec.id.is_none() {
        spec.id = Some(id);
    }
    Ok(spec)
}
