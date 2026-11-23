use crate::offscreen::Frame;
use bevy::math::DVec3;

pub const CENSUS_ANCHOR: DVec3 = DVec3::new(-40815.595807093756, 16288.03339696503, 46081.50808984984);
pub const STRAIGHT_DOWN_DEG: f64 = 90.0;
pub const CENSUS_PITCH_DOWN_DEG: f64 = 6.0;
pub const TELEPORT_CLEARANCE_M: f64 = 2.0;
const SHORE_ALTITUDE_M: f64 = 150.0;
const SHORE_PITCH_DOWN_DEG: f64 = 10.0;
const SEARCH_POINTS: usize = 40_000;
const SHORE_STEP_RAD: f64 = 0.002;
const SHORE_STEPS: usize = 3000;
const SHORE_BISECTIONS: usize = 40;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pose {
    Orbit,
    High,
    Low,
    Ground,
    #[allow(dead_code)]
    Shoreline,
}

impl Pose {
    pub fn altitude_m(self) -> Option<f64> {
        match self {
            Self::Orbit => Some(100_000.0),
            Self::High => Some(20_000.0),
            Self::Low => Some(5_000.0),
            Self::Ground | Self::Shoreline => None,
        }
    }

    pub fn frame(self, radius: f64, ground: impl Fn(DVec3) -> f64) -> Frame {
        match (self.altitude_m(), self) {
            (Some(altitude), _) => Frame::above(CENSUS_ANCHOR.normalize(), radius, altitude, STRAIGHT_DOWN_DEG),
            (None, Self::Shoreline) => shoreline_frame(radius, &ground),
            (None, _) => on_ground(radius, ground, 0.0),
        }
    }
}

fn fibonacci(index: usize) -> DVec3 {
    let golden = std::f64::consts::PI * (3.0 - 5f64.sqrt());
    let y = 1.0 - 2.0 * (index as f64 + 0.5) / SEARCH_POINTS as f64;
    let ring = (1.0 - y * y).sqrt();
    DVec3::new(ring * (golden * index as f64).cos(), y, ring * (golden * index as f64).sin())
}

fn shoreline(ground: &impl Fn(DVec3) -> f64) -> (DVec3, DVec3) {
    let wet = (0..SEARCH_POINTS).map(fibonacci).min_by(|a, b| ground(*a).total_cmp(&ground(*b))).expect("the search points");
    let east = DVec3::Y.cross(wet).normalize();
    let north = wet.cross(east);
    for heading in 0..16 {
        let angle = f64::from(heading) * std::f64::consts::TAU / 16.0;
        let course = east * angle.cos() + north * angle.sin();
        let along = |step: f64| (wet * step.cos() + course * step.sin()).normalize();
        let Some(land) = (1..SHORE_STEPS).map(|n| n as f64 * SHORE_STEP_RAD).find(|&step| ground(along(step)) > 0.0) else { continue };
        let (mut low, mut high) = (land - SHORE_STEP_RAD, land);
        for _ in 0..SHORE_BISECTIONS {
            let middle = (low + high) / 2.0;
            if ground(along(middle)) > 0.0 { high = middle } else { low = middle }
        }
        return (along(low), (course * low.cos() - wet * low.sin()).normalize());
    }
    panic!("no coastline found from the deepest ocean");
}

fn shoreline_frame(radius: f64, ground: &impl Fn(DVec3) -> f64) -> Frame {
    let (dir, heading) = shoreline(ground);
    let base = Frame::above(dir, radius, SHORE_ALTITUDE_M, SHORE_PITCH_DOWN_DEG);
    let heading = (heading - dir * heading.dot(dir)).normalize();
    let pitch = SHORE_PITCH_DOWN_DEG.to_radians();
    Frame { forward: heading * pitch.cos() - dir * pitch.sin(), up: heading * pitch.sin() + dir * pitch.cos(), ..base }
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
        assert!((frame.forward.dot(down) - 6.0_f64.to_radians().sin()).abs() < 1e-9);
    }

    #[test]
    fn the_shoreline_pose_stands_150_m_up_at_the_coast_looking_10_degrees_down_towards_land() {
        let coast = |dir: DVec3| if dir.y > 0.2 { 100.0 } else { -100.0 };
        let frame = Pose::Shoreline.frame(RADIUS_M, coast);
        let dir = frame.position.normalize();
        assert!((frame.position.length() - RADIUS_M - 150.0).abs() < TOLERANCE_M);
        assert!((dir.y - 0.2).abs() < 1e-6, "the pose is on the coast, not at {}", dir.y);
        assert!((frame.forward.dot(-dir) - 10.0_f64.to_radians().sin()).abs() < 1e-9);
        let ahead = (frame.position + frame.forward * 2_000.0).normalize();
        assert!(coast(ahead) > 0.0, "the pose faces the sea, not the land");
    }
}
