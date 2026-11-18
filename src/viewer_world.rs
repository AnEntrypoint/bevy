use avian3d::prelude::{Collider, ColliderOf};
use crate::basisu::TextureTarget;
use bevy::image::{CompressedImageFormatSupport, CompressedImageFormats};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::world_serialization::{WorldAsset, WorldAssetRoot};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::runtime::{FieldGrid, TerrainData};
use crate::spec::PrimitiveDesc;
use crate::viewer::{object_id, ViewerCache, ViewerObject, ViewerObjects, ViewerPlaceholder, ViewerScene, WorldRunner};
use crate::world::{ModelRef, ObjectId, PrimitiveInfo, ScriptedPrimitive};

#[derive(Component)]
pub struct WorldMirror(Entity);

#[derive(Component)]
pub struct TerrainMirror(String);

enum Drawn {
    Primitive(PrimitiveDesc),
    Model(String),
    Proxy(Vec3),
    Shape(MeshShape, &'static str),
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
    mut shapes: Local<HashMap<MeshShape, Handle<Mesh>>>,
    formats: Option<Res<CompressedImageFormatSupport>>,
) {
    let support = formats.map_or(CompressedImageFormats::empty(), |support| support.0);
    let target = (!basis_kept()).then(|| TextureTarget::for_gpu(support));
    scene.bc_supported = support.contains(CompressedImageFormats::BC);
    scene.texture_target = target.map_or("basis-uastc", |target| target.name());
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
    let mut collider_query = world.query::<(Entity, &Transform, &ColliderOf, &Collider)>();
    let mut scripted_children: Vec<(Entity, Entity, Transform, Drawn)> = Vec::new();
    for (entity, transform, collider_of, collider) in collider_query.iter(world) {
        if !scripted.contains(&collider_of.body) {
            continue;
        }
        let (transform, drawn) = scripted_geometry(transform, collider);
        scripted_children.push((collider_of.body, entity, transform, drawn));
    }
    scripted_children.sort_by_key(|(body, entity, _, _)| (body.to_bits(), entity.to_bits()));
    let mut children_per_body: HashMap<Entity, usize> = HashMap::new();
    for (body, _, _, _) in &scripted_children {
        *children_per_body.entry(*body).or_default() += 1;
    }
    let object_ids: HashMap<Entity, String> = world.query::<(Entity, &ObjectId)>().iter(world).map(|(entity, id)| (entity, id.0.clone())).collect();
    let mut query = world.query::<(Entity, &ObjectId, &Transform, Option<&PrimitiveInfo>, Option<&ModelRef>, Option<&Collider>)>();
    for (entity, id, transform, primitive, model, collider) in query.iter(world) {
        if scripted.contains(&entity) {
            continue;
        }
        if let Some(desc) = primitive.and_then(|primitive| primitive.0.clone()) {
            items.push(Item { entity, id: id.0.clone(), transform: *transform, drawn: Drawn::Primitive(desc) });
        } else if let Some(name) = model.and_then(|model| model_name(&model.0)) {
            items.push(Item { entity, id: id.0.clone(), transform: *transform, drawn: Drawn::Model(name) });
        } else if let Some(collider) = collider {
            if collider.shape_scaled().as_heightfield().is_some() {
                continue;
            }
            let aabb = collider.shape_scaled().compute_local_aabb();
            let size = Vec3::from(aabb.maxs) - Vec3::from(aabb.mins);
            let offset = Vec3::from(aabb.mins + aabb.maxs) * 0.5;
            let mut shape_transform = *transform;
            shape_transform.translation += transform.rotation * (offset * transform.scale);
            items.push(Item { entity, id: id.0.clone(), transform: shape_transform, drawn: Drawn::Proxy(size * transform.scale) });
            proxies_by_collider += 1;
        } else if !children_per_body.contains_key(&entity) {
            if model.is_some() {
                unmirrored_models += 1;
            }
            marker_ids.push(id.0.clone());
            items.push(Item { entity, id: id.0.clone(), transform: *transform, drawn: Drawn::Marker });
            markers_fallback += 1;
        }
    }
    let own_items: HashSet<Entity> = items.iter().map(|item| item.entity).collect();
    let mut proxies = proxies_by_collider;
    let mut numbered: HashMap<Entity, usize> = HashMap::new();
    for (body, entity, transform, drawn) in scripted_children {
        let index = numbered.entry(body).or_default();
        let number = *index + usize::from(own_items.contains(&body));
        *index += 1;
        let parent = object_ids.get(&body).cloned().unwrap_or_else(|| format!("scripted-{}", body.to_bits()));
        let id = if number == 0 { parent } else { format!("{parent}#{number}") };
        items.push(Item { entity, id, transform, drawn });
        proxies += 1;
    }
    for body in &scripted {
        if children_per_body.contains_key(body) {
            continue;
        }
        let transform = world.get::<Transform>(*body).copied().unwrap_or_default();
        let id = object_ids.get(body).cloned().unwrap_or_else(|| format!("scripted-{}", body.to_bits()));
        marker_ids.push(id.clone());
        items.push(Item { entity: *body, id, transform, drawn: Drawn::Marker });
        markers_fallback += 1;
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
                let mesh = shared_mesh(&mut meshes, &mut shapes, primitive_shape(desc));
                scene.mesh_requests += 1;
                commands.spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(palette[palette_slot(&desc.kind, palette.len())].clone()),
                    item.transform,
                    ViewerObject { id: item.id.clone(), kind: desc.kind.clone() },
                    WorldMirror(item.entity),
                    Name::new(item.id.clone()),
                ));
            }
            Drawn::Model(name) => {
                let handle = models
                    .entry(name.clone())
                    .or_insert_with(|| load_model(&directory, name, &cache.0, &asset_server, &mut scene, target))
                    .clone();
                let Some(handle) = handle else {
                    unmirrored += 1;
                    unmirrored_models += 1;
                    spawn_marker(&mut commands, &mut meshes, &mut shapes, &mut scene, item, proxy.clone());
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
            Drawn::Marker => spawn_marker(&mut commands, &mut meshes, &mut shapes, &mut scene, item, proxy.clone()),
            Drawn::Shape(shape, kind) => {
                let mesh = shared_mesh(&mut meshes, &mut shapes, *shape);
                scene.mesh_requests += 1;
                commands.spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(proxy.clone()),
                    item.transform,
                    ViewerObject { id: item.id.clone(), kind: kind.to_string() },
                    WorldMirror(item.entity),
                    Name::new(item.id.clone()),
                ));
            }
            Drawn::Proxy(size) => {
                let shape = MeshShape::Cuboid(size.x.max(0.05).to_bits(), size.y.max(0.05).to_bits(), size.z.max(0.05).to_bits());
                let mesh = shared_mesh(&mut meshes, &mut shapes, shape);
                scene.mesh_requests += 1;
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
    scene.shapes_created = shapes.len();
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
    let mut expected: HashSet<String> = items.iter().map(|item| item.id.clone()).collect();
    expected.extend(objects.0.iter().enumerate().map(|(index, object)| object_id(object, index)));
    expected.extend(spawned_terrains.iter().map(|name| format!("terrain/{name}")));
    let mut expected_ids: Vec<String> = expected.into_iter().collect();
    expected_ids.sort();
    scene.expected_ids = expected_ids;
    scene.terrains = spawned_terrains.len();
}

pub fn retire_placeholders(
    mut commands: Commands,
    placeholders: Query<(Entity, &ViewerPlaceholder)>,
    mirrors: Query<&ViewerObject, With<WorldMirror>>,
    mut scene: ResMut<ViewerScene>,
) {
    let mirrored: HashSet<String> = mirrors.iter().map(|object| object.id.clone()).collect();
    let entries: Vec<(Entity, String)> = placeholders.iter().map(|(entity, placeholder)| (entity, placeholder.0.clone())).collect();
    let retired = placeholders_to_retire(&entries, &mirrored);
    for entity in &retired {
        commands.entity(*entity).despawn();
    }
    scene.placeholders_retired += retired.len();
    scene.placeholders = entries.len() - retired.len();
}

pub fn placeholders_to_retire<T: Copy>(placeholders: &[(T, String)], mirrored: &HashSet<String>) -> Vec<T> {
    placeholders.iter().filter(|(_, id)| mirrored.contains(id)).map(|(entity, _)| *entity).collect()
}

pub fn duplicate_count(ids: &[String]) -> usize {
    ids.len() - ids.iter().collect::<HashSet<_>>().len()
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
    target: Option<TextureTarget>,
) -> Option<Handle<WorldAsset>> {
    let relative = name.trim_start_matches('/');
    let source = [directory.join(relative), directory.parent().unwrap_or(directory).join("assets").join(relative)]
        .into_iter()
        .find(|path| path.is_file());
    let Some(source) = source else {
        scene.models_missing.push(format!("missing: {name}"));
        return None;
    };
    let decoded = std::fs::read(&source).map_err(|error| error.to_string()).and_then(|bytes| match target {
        Some(target) => crate::glb::decode_with(&bytes, target),
        None => crate::glb::decode_kept_basis(&bytes),
    });
    let Ok(decoded) = decoded else {
        scene.models_missing.push(format!("failed: {name}"));
        return None;
    };
    let file = source.file_name()?.to_string_lossy().to_string();
    std::fs::create_dir_all(cache).ok()?;
    std::fs::write(cache.join(&file), &decoded.bytes).ok()?;
    Some(asset_server.load(format!("{file}#Scene0")))
}

pub(crate) fn basis_kept() -> bool {
    std::env::var("VIEWER_KEEP_BASIS").is_ok_and(|value| value == "1")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum MeshShape {
    Sphere(u32),
    Capsule(u32, u32),
    Cylinder(u32, u32),
    Cuboid(u32, u32, u32),
}

impl MeshShape {
    fn mesh(self) -> Mesh {
        match self {
            MeshShape::Sphere(radius) => Sphere::new(f32::from_bits(radius)).into(),
            MeshShape::Capsule(radius, length) => Capsule3d::new(f32::from_bits(radius), f32::from_bits(length)).into(),
            MeshShape::Cylinder(radius, height) => Cylinder::new(f32::from_bits(radius), f32::from_bits(height)).into(),
            MeshShape::Cuboid(x, y, z) => Cuboid::new(f32::from_bits(x), f32::from_bits(y), f32::from_bits(z)).into(),
        }
    }
}

const MARKER_RADIUS: f32 = 0.25;

fn primitive_shape(desc: &PrimitiveDesc) -> MeshShape {
    let kind = desc.kind.as_str();
    if kind.contains("sphere") || kind.contains("ball") {
        MeshShape::Sphere(desc.radius.to_bits())
    } else if kind.contains("capsule") {
        MeshShape::Capsule(desc.radius.to_bits(), (desc.capsule_half_height * 2.0).to_bits())
    } else if kind.contains("cylinder") {
        MeshShape::Cylinder(desc.radius.to_bits(), (desc.half_height * 2.0).to_bits())
    } else {
        MeshShape::Cuboid(desc.size.x.to_bits(), desc.size.y.to_bits(), desc.size.z.to_bits())
    }
}

fn shared_mesh(meshes: &mut Assets<Mesh>, shapes: &mut HashMap<MeshShape, Handle<Mesh>>, shape: MeshShape) -> Handle<Mesh> {
    shapes.entry(shape).or_insert_with(|| meshes.add(shape.mesh())).clone()
}

fn scripted_geometry(transform: &Transform, collider: &Collider) -> (Transform, Drawn) {
    let aabb = collider.shape_scaled().compute_local_aabb();
    let offset = Vec3::from(aabb.mins + aabb.maxs) * 0.5;
    let mut shape_transform = *transform;
    shape_transform.translation += transform.rotation * (offset * transform.scale);
    let drawn = match real_shape(collider, transform.scale) {
        Some((shape, kind)) => Drawn::Shape(shape, kind),
        None => Drawn::Proxy((Vec3::from(aabb.maxs) - Vec3::from(aabb.mins)) * transform.scale),
    };
    (shape_transform, drawn)
}

fn real_shape(collider: &Collider, scale: Vec3) -> Option<(MeshShape, &'static str)> {
    let shape = collider.shape_scaled();
    let planar = scale.x.max(scale.z);
    if let Some(ball) = shape.as_ball() {
        return Some((MeshShape::Sphere((ball.radius * scale.max_element()).to_bits()), "scripted-sphere"));
    }
    if let Some(cylinder) = shape.as_cylinder() {
        let height = cylinder.half_height * 2.0 * scale.y;
        return Some((MeshShape::Cylinder((cylinder.radius * planar).to_bits(), height.to_bits()), "scripted-cylinder"));
    }
    if let Some(capsule) = shape.as_capsule() {
        let length = (Vec3::from(capsule.segment.b) - Vec3::from(capsule.segment.a)).length() * scale.y;
        return Some((MeshShape::Capsule((capsule.radius * planar).to_bits(), length.to_bits()), "scripted-capsule"));
    }
    shape.as_cuboid().map(|cuboid| {
        let size = Vec3::from(cuboid.half_extents) * 2.0 * scale;
        (MeshShape::Cuboid(size.x.max(0.05).to_bits(), size.y.max(0.05).to_bits(), size.z.max(0.05).to_bits()), "scripted-cuboid")
    })
}

fn spawn_marker(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    shapes: &mut HashMap<MeshShape, Handle<Mesh>>,
    scene: &mut ViewerScene,
    item: &Item,
    material: Handle<StandardMaterial>,
) {
    let mesh = shared_mesh(meshes, shapes, MeshShape::Sphere(MARKER_RADIUS.to_bits()));
    scene.mesh_requests += 1;
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        item.transform,
        ViewerObject { id: item.id.clone(), kind: "marker".to_string() },
        WorldMirror(item.entity),
        Name::new(item.id.clone()),
    ));
}

pub(crate) fn palette_slot(kind: &str, slots: usize) -> usize {
    kind.bytes().fold(0usize, |hash, byte| hash.wrapping_mul(31).wrapping_add(byte as usize)) % slots
}

pub fn count_batches(
    draws: Query<(&Mesh3d, &MeshMaterial3d<StandardMaterial>)>,
    mirrored: Query<(&Mesh3d, &MeshMaterial3d<StandardMaterial>), With<WorldMirror>>,
    drawn_objects: Query<&ViewerObject>,
    objects: Query<(&Mesh3d, &MeshMaterial3d<StandardMaterial>), With<ViewerObject>>,
    mut scene: ResMut<ViewerScene>,
) {
    let mut object_batches = HashSet::new();
    let mut object_draws = 0;
    for (mesh, material) in &objects {
        object_draws += 1;
        object_batches.insert((mesh.0.id(), material.0.id()));
    }
    scene.object_draws = object_draws;
    scene.object_batch_keys = object_batches.len();
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    let drawn: Vec<String> = drawn_objects
        .iter()
        .map(|object| {
            *kinds.entry(object.kind.clone()).or_insert(0) += 1;
            object.id.clone()
        })
        .collect();
    scene.object_kinds = kinds;
    let drawn_set: HashSet<String> = drawn.iter().cloned().collect();
    let missing: Vec<String> = scene.expected_ids.iter().filter(|id| !drawn_set.contains(*id)).cloned().collect();
    scene.objects = drawn.len();
    scene.duplicate_objects = duplicate_count(&drawn);
    scene.expected_objects = scene.expected_ids.len();
    scene.missing_objects = missing;
    let mut meshes = HashSet::new();
    let mut materials = HashSet::new();
    let mut batches = HashSet::new();
    let mut entities = 0;
    for (mesh, material) in &draws {
        entities += 1;
        meshes.insert(mesh.0.id());
        materials.insert(material.0.id());
        batches.insert((mesh.0.id(), material.0.id()));
    }
    let mut mirror_batches = HashSet::new();
    let mut mirror_entities = 0;
    for (mesh, material) in &mirrored {
        mirror_entities += 1;
        mirror_batches.insert((mesh.0.id(), material.0.id()));
    }
    scene.mesh_entities = entities;
    scene.meshes = meshes.len();
    scene.unique_materials = materials.len();
    scene.batch_keys = batches.len();
    scene.mirror_entities = mirror_entities;
    scene.mirror_batch_keys = mirror_batches.len();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_retire_only_when_mirrored() {
        let mirrored = HashSet::from(["a".to_string()]);
        let placeholders = [(1, "a".to_string()), (2, "b".to_string())];
        assert_eq!(placeholders_to_retire(&placeholders, &mirrored), vec![1]);
    }

    #[test]
    fn duplicate_count_counts_repeated_ids() {
        assert_eq!(duplicate_count(&["a".to_string(), "b".to_string(), "a".to_string()]), 1);
        assert_eq!(duplicate_count(&["a".to_string(), "b".to_string()]), 0);
    }

    fn primitive(kind: &str, radius: f32) -> PrimitiveDesc {
        PrimitiveDesc { kind: kind.to_string(), size: Vec3::ONE, half_height: 0.5, radius, capsule_half_height: 0.5 }
    }

    #[test]
    fn identical_primitives_map_to_one_shape() {
        assert_eq!(primitive_shape(&primitive("sphere", 0.5)), primitive_shape(&primitive("sphere", 0.5)));
        assert_ne!(primitive_shape(&primitive("sphere", 0.5)), primitive_shape(&primitive("sphere", 0.75)));
    }

    #[test]
    fn shared_mesh_reuses_one_handle_per_shape() {
        let mut meshes = Assets::<Mesh>::default();
        let mut shapes = HashMap::new();
        let shape = MeshShape::Sphere(MARKER_RADIUS.to_bits());
        let first = shared_mesh(&mut meshes, &mut shapes, shape);
        let second = shared_mesh(&mut meshes, &mut shapes, shape);
        assert_eq!(first, second);
        assert_eq!(shapes.len(), 1);
        assert_eq!(meshes.len(), 1);
    }

    #[test]
    fn scripted_colliders_keep_their_shape_kind() {
        let (shape, kind) = real_shape(&Collider::sphere(0.5), Vec3::splat(2.0)).unwrap();
        assert_eq!((shape, kind), (MeshShape::Sphere(1.0_f32.to_bits()), "scripted-sphere"));
        let (shape, kind) = real_shape(&Collider::cylinder(0.5, 2.0), Vec3::ONE).unwrap();
        assert_eq!((shape, kind), (MeshShape::Cylinder(0.5_f32.to_bits(), 2.0_f32.to_bits()), "scripted-cylinder"));
        let (shape, kind) = real_shape(&Collider::capsule(0.25, 1.0), Vec3::ONE).unwrap();
        assert!(matches!(shape, MeshShape::Capsule(radius, _) if f32::from_bits(radius) == 0.25));
        assert_eq!(kind, "scripted-capsule");
        let (shape, kind) = real_shape(&Collider::cuboid(1.0, 2.0, 3.0), Vec3::ONE).unwrap();
        assert_eq!((shape, kind), (MeshShape::Cuboid(1.0_f32.to_bits(), 2.0_f32.to_bits(), 3.0_f32.to_bits()), "scripted-cuboid"));
    }

    #[test]
    fn palette_slot_is_stable_and_in_range() {
        assert_eq!(palette_slot("crate", 4), palette_slot("crate", 4));
        assert!((0..64).all(|index| palette_slot(&format!("kind-{index}"), 4) < 4));
    }
}
