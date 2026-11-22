use crate::planet_lod::guard::note_build;
use crate::planet_veg::gpu::buffers::VegBounds;
use crate::planet_veg::gpu::draw::{VegDrawSet, VegMaterial};
use crate::planet_veg::gpu::draw::{insert_veg_shader, veg_mesh};
use crate::rock_shapes::generate_rock_hull_data;
use bevy::asset::uuid_handle;
use bevy::prelude::*;
use bevy::shader::Shader;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures::check_ready};

pub const ROCK_TYPES: usize = 6;
pub const ROCK_BASE_SEED: i32 = 1337;
pub const ROCK_MESH_RES: usize = 16;
pub const ROCK_SHADER: Handle<Shader> = uuid_handle!("3b7e5a91-4c2d-4e08-b6f3-1a9d8c70e254");
pub const ROCK_SHADER_PATH: &str = "shaders/planet/vegetation_rock.wgsl";
pub const ROCK_SOURCE: &str = concat!(include_str!("../../assets/shaders/planet/vegetation_common.wgsl"), include_str!("../../assets/shaders/planet/vegetation_rock.wgsl"));

pub struct RockMeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub bounds: [f32; 4],
}

fn smooth_normals(positions: &[Vec3], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut sums = vec![Vec3::ZERO; positions.len()];
    for triangle in indices.chunks_exact(3) {
        let [a, b, c] = [0, 1, 2].map(|corner| positions[triangle[corner] as usize]);
        let area_weighted = (c - b).cross(a - b);
        triangle.iter().for_each(|vertex| sums[*vertex as usize] += area_weighted);
    }
    sums.into_iter().map(|sum| sum.normalize_or_zero().to_array()).collect()
}

fn bounding_sphere(positions: &[Vec3]) -> [f32; 4] {
    let (low, high) = positions.iter().fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(low, high), point| (low.min(*point), high.max(*point)));
    let centre = (low + high) * 0.5;
    let radius = positions.iter().map(|point| point.distance(centre)).fold(0.0, f32::max);
    [centre.x, centre.y, centre.z, radius]
}

pub fn build_rock_meshes() -> Vec<RockMeshData> {
    note_build();
    generate_rock_hull_data(ROCK_TYPES, ROCK_BASE_SEED, ROCK_MESH_RES)
        .into_iter()
        .map(|hull| {
            let points: Vec<Vec3> = hull.positions.chunks_exact(3).map(|p| Vec3::new(p[0], p[1], p[2])).collect();
            RockMeshData {
                normals: smooth_normals(&points, &hull.indices),
                bounds: bounding_sphere(&points),
                positions: points.iter().map(|point| point.to_array()).collect(),
                indices: hull.indices,
            }
        })
        .collect()
}

pub fn rock_material() -> VegMaterial {
    VegMaterial { shader: ROCK_SHADER, vertex_entry: "vs_rock", fragment_entry: "fs_rock", group2: None, cull_back: true }
}

#[derive(Resource)]
struct RockBuild(Task<Vec<RockMeshData>>);

fn start_build(mut commands: Commands) {
    commands.insert_resource(RockBuild(AsyncComputeTaskPool::get().spawn(async { build_rock_meshes() })));
}

fn finish_build(mut commands: Commands, mut build: ResMut<RockBuild>, mut meshes: ResMut<Assets<Mesh>>, mut bounds: ResMut<VegBounds>) {
    let Some(built) = check_ready(&mut build.0) else { return };
    commands.remove_resource::<RockBuild>();
    bounds.rock_kind = built.iter().map(|rock| rock.bounds).collect();
    for (kind, rock) in built.into_iter().enumerate() {
        let uvs = vec![[0.0, 0.0]; rock.positions.len()];
        let mesh = meshes.add(veg_mesh(rock.positions, rock.normals, uvs, rock.indices));
        commands.spawn(VegDrawSet::rock(kind as u32, mesh, rock_material()));
    }
}

pub struct RockRenderPlugin;

impl Plugin for RockRenderPlugin {
    fn build(&self, app: &mut App) {
        insert_veg_shader(app, &ROCK_SHADER, ROCK_SOURCE, ROCK_SHADER_PATH);
        app.add_systems(Startup, start_build).add_systems(Update, finish_build.run_if(resource_exists::<RockBuild>));
    }
}

#[cfg(test)]
mod tests;
