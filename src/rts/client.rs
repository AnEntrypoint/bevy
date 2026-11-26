use super::{load_image, script, Order};
use crate::offscreen::{add_target, capture, headless_app, render_lock, write_png};
use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use sim::Sim;
use std::path::Path;

const IMAGE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/rts/duel-routes.osdt");
const ORDERS: &str = include_str!("../../testdata/rts/duel-routes.orders");
const SIZE: u32 = 512;

pub struct View {
    pub ticks: u32,
    pub yaw: f32,
}

pub fn stepped(ticks: u32) -> Sim {
    let mut sim = load_image(Path::new(IMAGE));
    let orders = script::parse(ORDERS).expect("orders");
    let mut next = 0;
    for _ in 0..ticks {
        let tick = sim.ticks();
        while next < orders.len() && orders[next].0 <= tick {
            orders[next].1.apply(&mut sim);
            next += 1;
        }
        sim.tick();
    }
    sim
}

fn units(sim: &Sim) -> Vec<Vec3> {
    (0..sim.count())
        .filter(|&i| sim.alive(i))
        .map(|i| {
            let (x, y) = sim.position_fixed(i);
            Vec3::new(x as f32 / 65536.0, 0.5, y as f32 / 65536.0)
        })
        .collect()
}

pub fn hud(sim: &Sim) -> String {
    format!("tick {} units {}", sim.ticks(), units(sim).len())
}

pub fn render(sim: &Sim, view: &View) -> Image {
    let _serial = render_lock();
    let mut app = headless_app();
    let target = add_target(&mut app, TextureFormat::Rgba8UnormSrgb);
    let positions = units(sim);
    let (cx, cz) = positions
        .iter()
        .fold((0.0_f32, 0.0_f32), |(x, z), p| (x + p.x, z + p.z));
    let count = positions.len().max(1) as f32;
    let centre = Vec3::new(cx / count, 0.0, cz / count);
    let world = app.world_mut();
    let mesh_unit = world.resource_mut::<Assets<Mesh>>().add(Cuboid::new(0.6, 1.0, 0.6));
    let mesh_ground = world.resource_mut::<Assets<Mesh>>().add(Cuboid::new(200.0, 0.2, 200.0));
    let mat_unit = world.resource_mut::<Assets<StandardMaterial>>().add(Color::srgb(0.85, 0.3, 0.2));
    let mat_ground = world.resource_mut::<Assets<StandardMaterial>>().add(Color::srgb(0.2, 0.35, 0.2));
    world.spawn((Mesh3d(mesh_ground), MeshMaterial3d(mat_ground), Transform::from_xyz(0.0, -0.1, 0.0)));
    for position in positions {
        world.spawn((Mesh3d(mesh_unit.clone()), MeshMaterial3d(mat_unit.clone()), Transform::from_translation(position)));
    }
    world.spawn((Text::new(hud(sim)), Node { position_type: PositionType::Absolute, left: Val::Px(8.0), top: Val::Px(8.0), ..default() }));
    world.spawn((DirectionalLight { illuminance: 8000.0, ..default() }, Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y)));
    let eye = centre + Vec3::new(view.yaw.sin(), 1.0, view.yaw.cos()) * 40.0;
    world.spawn((
        Camera3d::default(),
        RenderTarget::Image(target.clone().into()),
        Transform::from_translation(eye).looking_at(centre, Vec3::Y),
    ));
    app.finish();
    app.cleanup();
    capture(&mut app, target, &format!("rts-client-{}", view.ticks))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn changed_pixels(a: &Image, b: &Image) -> usize {
        a.data.as_ref().unwrap().chunks_exact(4).zip(b.data.as_ref().unwrap().chunks_exact(4)).filter(|(x, y)| x.iter().zip(y.iter()).take(3).any(|(p, q)| p.abs_diff(*q) > 8)).count()
    }

    #[test]
    fn hud_names_the_tick_and_the_units_drawn() {
        let sim = stepped(64);
        assert_eq!(hud(&sim), format!("tick 64 units {}", units(&sim).len()));
    }

    #[test]
    fn the_same_state_renders_the_same_pixels() {
        let sim = stepped(64);
        let view = View { ticks: 64, yaw: 0.0 };
        let first = render(&sim, &view);
        let second = render(&sim, &view);
        assert_eq!(changed_pixels(&first, &second), 0);
        write_png(&first, "rts-client-64");
    }

    #[test]
    fn a_different_camera_changes_the_pixels() {
        let sim = stepped(64);
        let first = render(&sim, &View { ticks: 64, yaw: 0.0 });
        let second = render(&sim, &View { ticks: 64, yaw: 1.2 });
        assert!(changed_pixels(&first, &second) > 20, "the camera change did not reach the image");
    }
}
