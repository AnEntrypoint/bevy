use avian3d::prelude::{Collider, ColliderOf};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::world_serialization::{WorldAsset, WorldAssetRoot};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::runtime::{FieldGrid, TerrainData};
use crate::spec::PrimitiveDesc;
use crate::viewer::{ViewerCache, ViewerObject, ViewerObjects, ViewerScene, WorldRunner};
use crate::world::{ModelRef, ObjectId, PrimitiveInfo, ScriptedPrimitive};

#[derive(Component)]
pub struct WorldMirror(Entity);

#[derive(Component)]
pub struct TerrainMirror(String);

enum Drawn {
    Primitive(PrimitiveDesc),
    Model(String),
    Proxy(Vec3),
    Marker,
}

struct Item {
    entity: Entity,
    id: String,
    transform: Transform,
    drawn: Drawn,
}

pub fn mirror_world(
    mut commands: Commands,
    mut runner: NonSendMut<WorldRunner>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    cache: Res<ViewerCache>,
    mut scene: ResMut<ViewerScene>,
    objects: Res<ViewerObjects>,
    mut mirrors: Query<(Entity, &WorldMirror, &mut Transform)>,
    terrains: Query<&TerrainMirror>,
    mut palette: Local<Vec<Handle<StandardMaterial>>>,
    mut proxy_material: Local<Option<Handle<StandardMaterial>>>,
    mut models: Local<HashMap<String, Option<Handle<WorldAsset>>>>,
) {
    let directory = PathBuf::from(&scene.directory);
    let world = runner.0.world_mut();
    let scripted: HashSet<Entity> = world
        .query::<(Entity, &ScriptedPrimitive)>()
        .iter(world)
        .map(|(entity, _)| entity)
        .collect();
    let mut items: Vec<Item> = Vec::new();
    let mut unmirrored = 0;
    let mut unmirrored_models = 0;
    let mut marker_ids = Vec::new();
    let mut proxies_by_collider = 0;
    let mut markers_fallback = 0;
    let mut query = world.query::<(Entity, &ObjectId, &Transform, Option<&PrimitiveInfo>, Option<&ModelRef>, Option<&Collider>)>();
    for (entity, id, transform, primitive, model, collider) in query.iter(world) {
        if let Some(desc) = primitive.and_then(|primitive| primitive.0.clone()) {
            items.push(Item { entity, id: id.0.clone(), transform: *transform, drawn: Drawn::Primitive(desc) });
        } else if let Some(name) = model.and_then(|model| model_name(&model.0)) {
            items.push(Item { entity, id: id.0.clone(), transform: *transform, drawn: Drawn::Model(name) });
        } else if let Some(collider) = collider {
            let aabb = collider.shape_scaled().compute_local_aabb();
            let size = Vec3::from(aabb.maxs) - Vec3::from(aabb.mins);
            let offset = Vec3::from(aabb.mins + aabb.maxs) * 0.5;
            let mut shape_transform = *transform;
            shape_transform.translation += transform.rotation * (offset * transform.scale);
            items.push(Item { entity, id: id.0.clone(), transform: shape_transform, drawn: Drawn::Proxy(size * transform.scale) });
            proxies_by_collider += 1;
        } else {
            if model.is_some() {
                unmirrored_models += 1;
            }
            marker_ids.push(id.0.clone());
            items.push(Item { entity, id: id.0.clone(), transform: *transform, drawn: Drawn::Marker });
            markers_fallback += 1;
        }
    }
    let mut collider_query = world.query::<(Entity, &Transform, &ColliderOf, &Collider)>();
    let mut proxies = proxies_by_collider;
    for (entity, transform, collider_of, collider) in collider_query.iter(world) {
        if !scripted.contains(&collider_of.body) {
            continue;
        }
        let aabb = collider.shape_scaled().compute_local_aabb();
        let size = Vec3::from(aabb.maxs) - Vec3::from(aabb.mins);
        let offset = Vec3::from(aabb.mins + aabb.maxs) * 0.5;
        let mut transform = *transform;
        transform.translation += transform.rotation * (offset * transform.scale);
        items.push(Item { entity, id: format!("scripted-collider-{}", entity.to_bits()), transform, drawn: Drawn::Proxy(size * transform.scale) });
        proxies += 1;
    }
    let live: HashSet<Entity> = items.iter().map(|item| item.entity).collect();
    let by_entity: HashMap<Entity, &Item> = items.iter().map(|item| (item.entity, item)).collect();
    let mut placed = HashSet::new();
    for (viewer, mirror, mut transform) in &mut mirrors {
        match by_entity.get(&mirror.0) {
            Some(item) => {
                *transform = item.transform;
                placed.insert(mirror.0);
            }
            None if live.contains(&mirror.0) => {}
            None => commands.entity(viewer).despawn(),
        }
    }
    if palette.is_empty() {
        let colors = [
            Color::srgb(0.24, 0.62, 0.95),
            Color::srgb(0.95, 0.55, 0.25),
            Color::srgb(0.42, 0.82, 0.45),
            Color::srgb(0.78, 0.42, 0.88),
        ];
        palette.extend(colors.into_iter().map(|color| materials.add(StandardMaterial { base_color: color, ..default() })));
    }
    if proxy_material.is_none() {
        *proxy_material = Some(materials.add(StandardMaterial { base_color: Color::srgb(0.98, 0.82, 0.25), ..default() }));
    }
    let proxy = proxy_material.clone().unwrap_or_default();
    for item in &items {
        if placed.contains(&item.entity) {
            continue;
        }
        match &item.drawn {
            Drawn::Primitive(desc) => {
                let mesh = primitive_mesh(&mut meshes, desc);
                commands.spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(palette[item.entity.to_bits() as usize % palette.len()].clone()),
                    item.transform,
                    ViewerObject { id: item.id.clone(), kind: desc.kind.clone() },
                    WorldMirror(item.entity),
                    Name::new(item.id.clone()),
                ));
            }
            Drawn::Model(name) => {
                let handle = models
                    .entry(name.clone())
                    .or_insert_with(|| load_model(&directory, name, &cache.0, &asset_server, &mut scene))
                    .clone();
                let Some(handle) = handle else {
                    unmirrored += 1;
                    unmirrored_models += 1;
                    continue;
                };
                commands.spawn((
                    WorldAssetRoot(handle),
                    item.transform,
                    ViewerObject { id: item.id.clone(), kind: "model".to_string() },
                    WorldMirror(item.entity),
                    Name::new(item.id.clone()),
                ));
            }
            Drawn::Marker => {
                let mesh = meshes.add(Sphere::new(0.25));
                commands.spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(proxy.clone()),
                    item.transform,
                    ViewerObject { id: item.id.clone(), kind: "scripted-marker".to_string() },
                    WorldMirror(item.entity),
                    Name::new(item.id.clone()),
                ));
            }
            Drawn::Proxy(size) => {
                let mesh = meshes.add(Cuboid::new(size.x.max(0.05), size.y.max(0.05), size.z.max(0.05)));
                commands.spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(proxy.clone()),
                    item.transform,
                    ViewerObject { id: item.id.clone(), kind: "scripted-proxy".to_string() },
                    WorldMirror(item.entity),
                    Name::new(item.id.clone()),
                ));
            }
        }
    }
    let mut spawned_terrains = terrains.iter().map(|terrain| terrain.0.clone()).collect::<HashSet<String>>();
    let terrain_fields: Vec<(String, FieldGrid)> = world
        .get_resource::<TerrainData>()
        .map(|terrain| terrain.fields.iter().map(|(name, field)| (name.clone(), field.clone())).collect())
        .unwrap_or_default();
    for (name, field) in terrain_fields {
        if spawned_terrains.contains(&name) || field.cols < 2 || field.rows < 2 || field.heights.len() < field.cols * field.rows {
            continue;
        }
        spawned_terrains.insert(name.clone());
        let mesh = meshes.add(terrain_mesh(&field));
        let material = materials.add(StandardMaterial { base_color: Color::srgb(0.32, 0.36, 0.30), perceptual_roughness: 0.95, ..default() });
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::IDENTITY,
            ViewerObject { id: format!("terrain/{name}"), kind: "terrain".to_string() },
            TerrainMirror(name.clone()),
            Name::new(format!("terrain/{name}")),
        ));
    }
    scene.objects = objects.0.len() + items.len();
    scene.meshes = objects.0.len() + items.len();
    scene.unmirrored = unmirrored;
    scene.unmirrored_models = unmirrored_models;
    scene.models_loaded = items
        .iter()
        .filter(|item| match &item.drawn {
            Drawn::Model(name) => matches!(models.get(name), Some(Some(_))),
            _ => false,
        })
        .count();
    scene.proxies = proxies;
    scene.markers = markers_fallback;
    scene.models_requested = models.keys().cloned().collect::<Vec<String>>();
    scene.models_requested.sort();
    scene.marker_ids = marker_ids;
    scene.terrains = spawned_terrains.len();
}

fn model_name(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Object(map) => map.get("id").and_then(Value::as_str).map(str::to_string),
        _ => None,
    }
}

fn load_model(
    directory: &Path,
    name: &str,
    cache: &Path,
    asset_server: &AssetServer,
    scene: &mut ViewerScene,
) -> Option<Handle<WorldAsset>> {
    let relative = name.trim_start_matches('/');
    let source = [directory.join(relative), directory.parent().unwrap_or(directory).join("assets").join(relative)]
        .into_iter()
        .find(|path| path.is_file());
    let Some(source) = source else {
        scene.models_missing.push(format!("missing: {name}"));
        return None;
    };
    let decoded = std::fs::read(&source).map_err(|error| error.to_string()).and_then(|bytes| crate::glb::decode(&bytes));
    let Ok(decoded) = decoded else {
        scene.models_missing.push(format!("failed: {name}"));
        return None;
    };
    let file = source.file_name()?.to_string_lossy().to_string();
    std::fs::create_dir_all(cache).ok()?;
    std::fs::write(cache.join(&file), &decoded.bytes).ok()?;
    Some(asset_server.load(format!("{file}#Scene0")))
}

fn primitive_mesh(meshes: &mut Assets<Mesh>, desc: &PrimitiveDesc) -> Handle<Mesh> {
    let kind = desc.kind.as_str();
    if kind.contains("sphere") || kind.contains("ball") {
        meshes.add(Sphere::new(desc.radius))
    } else if kind.contains("capsule") {
        meshes.add(Capsule3d::new(desc.radius, desc.capsule_half_height * 2.0))
    } else if kind.contains("cylinder") {
        meshes.add(Cylinder::new(desc.radius, desc.half_height * 2.0))
    } else {
        meshes.add(Cuboid::new(desc.size.x, desc.size.y, desc.size.z))
    }
}

fn terrain_mesh(field: &FieldGrid) -> Mesh {
    let (cols, rows) = (field.cols, field.rows);
    let step_x = (field.max.x - field.min.x) / (cols - 1) as f32;
    let step_z = (field.max.y - field.min.y) / (rows - 1) as f32;
    let mut positions = Vec::with_capacity(cols * rows);
    for z in 0..rows {
        for x in 0..cols {
            positions.push([field.min.x + x as f32 * step_x, field.heights[z * cols + x], field.min.y + z as f32 * step_z]);
        }
    }
    let mut indices: Vec<u32> = Vec::with_capacity((cols - 1) * (rows - 1) * 6);
    for z in 0..rows - 1 {
        for x in 0..cols - 1 {
            let a = (z * cols + x) as u32;
            let b = a + cols as u32;
            let c = a + 1;
            let d = b + 1;
            indices.extend([a, b, d, a, d, c]);
        }
    }
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_indices(Indices::U32(indices));
    mesh.compute_smooth_normals();
    mesh
}
