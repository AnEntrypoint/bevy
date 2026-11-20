use crate::globe::GlobeWorld;
use bevy::prelude::*;

const TREES_PER_CHUNK: u32 = 6;

#[derive(Resource)]
pub struct VegetationAssets {
    trunk: Handle<Mesh>,
    canopy: Handle<Mesh>,
    trunk_material: Handle<StandardMaterial>,
    canopy_material: Handle<StandardMaterial>,
}

#[derive(Component)]
pub struct Tree;

pub fn prepare(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    commands.insert_resource(VegetationAssets {
        trunk: meshes.add(Cylinder::new(0.12, 1.2)),
        canopy: meshes.add(Cone::new(0.6, 2.0)),
        trunk_material: materials.add(StandardMaterial { base_color: Color::srgb(0.36, 0.23, 0.12), perceptual_roughness: 0.95, ..default() }),
        canopy_material: materials.add(StandardMaterial { base_color: Color::srgb(0.12, 0.42, 0.14), perceptual_roughness: 0.9, ..default() }),
    });
}

fn placements(globe: &GlobeWorld, bounds: (f32, f32, f32, f32), key: (i32, i32)) -> Vec<(f32, f32)> {
    (0..TREES_PER_CHUNK)
        .filter_map(|index| {
            let slot = index as i64;
            let latitude = bounds.0 + (bounds.1 - bounds.0) * globe.hash(key.0 as i64, key.1 as i64, 2 * slot);
            let longitude = bounds.2 + (bounds.3 - bounds.2) * globe.hash(key.0 as i64, key.1 as i64, 2 * slot + 1);
            (globe.elevation(latitude, longitude) > globe.sea_level).then_some((latitude, longitude))
        })
        .collect()
}

pub fn spawn_chunk_trees(commands: &mut Commands, globe: &GlobeWorld, assets: &VegetationAssets, chunk: Entity, key: (i32, i32), bounds: (f32, f32, f32, f32)) {
    commands.entity(chunk).with_children(|parent| {
        for (latitude, longitude) in placements(globe, bounds, key) {
            let base = globe.surface_position(latitude, longitude, 0.0);
            let rotation = globe.orientation(latitude, longitude, 0.0);
            parent.spawn((Tree, Transform::from_translation(base).with_rotation(rotation), Visibility::default())).with_children(|tree| {
                tree.spawn((Mesh3d(assets.trunk.clone()), MeshMaterial3d(assets.trunk_material.clone()), Transform::from_xyz(0.0, 0.6, 0.0)));
                tree.spawn((Mesh3d(assets.canopy.clone()), MeshMaterial3d(assets.canopy_material.clone()), Transform::from_xyz(0.0, 2.2, 0.0)));
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cells(globe: &GlobeWorld) -> Vec<((f32, f32, f32, f32), (i32, i32))> {
        let (rows, columns) = globe.grid();
        (0..rows).flat_map(|row| (0..columns).map(move |column| (globe.cell_bounds(row, column), (row, column)))).collect()
    }

    #[test]
    fn placements_are_on_land() {
        let globe = GlobeWorld::default();
        for (bounds, key) in cells(&globe) {
            for (latitude, longitude) in placements(&globe, bounds, key) {
                assert!(globe.elevation(latitude, longitude) > globe.sea_level);
            }
        }
    }

    #[test]
    fn tree_bases_sit_on_the_surface_above_the_radius() {
        let globe = GlobeWorld::default();
        for (bounds, key) in cells(&globe) {
            for (latitude, longitude) in placements(&globe, bounds, key) {
                let altitude = globe.surface_position(latitude, longitude, 0.0).distance(globe.center) - globe.radius;
                assert!(altitude > 0.0 && altitude <= globe.amplitude, "altitude {altitude}");
            }
        }
    }

    #[test]
    fn the_planet_has_trees() {
        let globe = GlobeWorld::default();
        let total: usize = cells(&globe).into_iter().map(|(bounds, key)| placements(&globe, bounds, key).len()).sum();
        assert!(total > 0);
    }
}
