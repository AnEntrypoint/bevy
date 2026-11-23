use crate::offscreen::Frame;
use bevy::math::DVec3;

pub const CENSUS_ANCHOR: DVec3 = DVec3::new(-40815.595807093756, 16288.03339696503, 46081.50808984984);
pub const STRAIGHT_DOWN_DEG: f64 = 90.0;
pub const CENSUS_PITCH_DOWN_DEG: f64 = 6.0;
pub const TELEPORT_CLEARANCE_M: f64 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pose {
    Orbit,
    High,
    Low,
    Ground,
}

impl Pose {
    pub fn altitude_m(self) -> Option<f64> {
        match self {
            Self::Orbit => Some(100_000.0),
            Self::High => Some(20_000.0),
            Self::Low => Some(5_000.0),
            Self::Ground => None,
        }
    }

    pub fn frame(self, radius: f64, ground: impl Fn(DVec3) -> f64) -> Frame {
        match self.altitude_m() {
            Some(altitude) => Frame::above(CENSUS_ANCHOR.normalize(), radius, altitude, STRAIGHT_DOWN_DEG),
            None => on_ground(radius, ground, 0.0),
        }
    }
}

pub fn mix(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

pub fn on_ground(radius: f64, ground: impl Fn(DVec3) -> f64, metres_north: f64) -> Frame {
    let start = CENSUS_ANCHOR.normalize();
    let north = Frame::above(start, radius, 0.0, 0.0).forward;
    let angle = metres_north / CENSUS_ANCHOR.length();
    let dir = start * angle.cos() + north * angle.sin();
    let eye = (CENSUS_ANCHOR.length() - radius - ground(start)).max(TELEPORT_CLEARANCE_M);
    Frame::above(dir, radius + ground(dir), eye, CENSUS_PITCH_DOWN_DEG)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RADIUS_M: f64 = 63_600.0;
    const TOLERANCE_M: f64 = 1e-6;

    #[test]
    fn the_orbit_high_and_low_poses_look_straight_down_from_100_20_and_5_km_over_the_anchor() {
        for (pose, altitude) in [(Pose::Orbit, 100_000.0), (Pose::High, 20_000.0), (Pose::Low, 5_000.0)] {
            let frame = pose.frame(RADIUS_M, |_| 0.0);
            assert!((frame.position.length() - RADIUS_M - altitude).abs() < TOLERANCE_M, "{pose:?} altitude");
            assert!(frame.forward.dot(frame.position.normalize()) < -0.999_999, "{pose:?} looks straight down");
            assert!(frame.position.normalize().dot(CENSUS_ANCHOR.normalize()) > 0.999_999, "{pose:?} is over the anchor");
        }
    }

    #[test]
    fn the_ground_pose_is_the_census_eye_at_the_anchor_looking_6_degrees_down() {
        let frame = Pose::Ground.frame(RADIUS_M, |_| 0.0);
        assert!((frame.position - CENSUS_ANCHOR).length() < TOLERANCE_M);
        let down = -frame.position.normalize();
        assert!((frame.forward.dot(down) - CENSUS_PITCH_DOWN_DEG.to_radians().sin()).abs() < 1e-9);
    }
}
