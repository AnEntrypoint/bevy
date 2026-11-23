use bevy::math::DVec3;
use bevy::prelude::*;

pub const REBASE_DISTANCE_M: f64 = 1000.0;

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct FloatingOrigin {
    pub position: DVec3,
}

#[derive(Component)]
pub struct OriginRelative;

impl FloatingOrigin {
    pub fn to_local(&self, world: DVec3) -> Vec3 {
        (world - self.position).as_vec3()
    }

    pub fn to_world(&self, local: Vec3) -> DVec3 {
        self.position + local.as_dvec3()
    }

    pub fn rebase(&mut self, camera_local: Vec3) -> Option<Vec3> {
        let beyond = camera_local.as_dvec3().length() >= REBASE_DISTANCE_M;
        beyond.then(|| {
            self.position += camera_local.as_dvec3();
            camera_local
        })
    }
}

pub fn rebase_system(mut origin: ResMut<FloatingOrigin>, mut movers: Query<(&mut Transform, Has<Camera3d>), With<OriginRelative>>) {
    crate::frame_clock::timed(11, || {
    let camera = movers.iter().find_map(|(transform, is_camera)| is_camera.then_some(transform.translation));
    let Some(shift) = camera.and_then(|local| origin.rebase(local)) else { return };
    for (mut transform, _) in &mut movers {
        transform.translation -= shift;
    }
});
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rebase_moves_the_origin_to_the_camera_and_keeps_world_positions() {
        let mut app = App::new();
        app.insert_resource(FloatingOrigin { position: DVec3::new(6_360_000.0, 10.0, -20.0) });
        app.add_systems(Update, rebase_system);
        let camera = app.world_mut().spawn((Camera3d::default(), Transform::from_xyz(1500.0, 3.0, -4.0), OriginRelative)).id();
        let patch = app.world_mut().spawn((Transform::from_xyz(2000.0, 5.0, 7.0), OriginRelative)).id();
        let before = |app: &App, entity| {
            let origin = *app.world().resource::<FloatingOrigin>();
            origin.to_world(app.world().get::<Transform>(entity).unwrap().translation)
        };
        let (camera_world, patch_world) = (before(&app, camera), before(&app, patch));
        app.update();
        assert_eq!(app.world().get::<Transform>(camera).unwrap().translation, Vec3::ZERO);
        assert_eq!(app.world().get::<Transform>(patch).unwrap().translation, Vec3::new(500.0, 2.0, 11.0));
        assert!((before(&app, camera) - camera_world).length() < 1e-9);
        assert!((before(&app, patch) - patch_world).length() < 1e-9);
    }

    #[test]
    fn rebase_waits_until_the_camera_is_a_kilometre_out() {
        let mut origin = FloatingOrigin::default();
        assert_eq!(origin.rebase(Vec3::new(999.0, 0.0, 0.0)), None);
        assert_eq!(origin.rebase(Vec3::new(1000.0, 0.0, 0.0)), Some(Vec3::new(1000.0, 0.0, 0.0)));
        assert_eq!(origin.position, DVec3::new(1000.0, 0.0, 0.0));
    }

    #[test]
    fn local_precision_holds_after_many_rebases_at_the_engine_radius() {
        let mut origin = FloatingOrigin { position: DVec3::new(6_360_000.0, 0.0, 0.0) };
        let mut world = DVec3::new(6_360_000.123_456, 0.0, 0.0);
        for step in 0..2000 {
            world += DVec3::new(0.7, 0.0, 0.4 * f64::from(step % 3));
            let local = origin.to_local(world);
            origin.rebase(local);
        }
        let local = origin.to_local(world);
        assert!((origin.to_world(local) - world).length() < 1e-3);
        assert!(local.length() < 2000.0);
    }
}
