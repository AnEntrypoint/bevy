use crate::height::anchor::FACE_FRAME;
use crate::vec3::{dot, normalise};
use bevy::math::DVec3;
use std::f64::consts::PI;

pub const DEFAULT_MAX_LEVEL: u8 = 11;
pub const DEFAULT_SPLIT_FACTOR: f64 = 0.25;
const REFERENCE_RADIUS: f64 = 6_360_000.0;
const LOD_STEP: f64 = 7.2;
const LOD_POP_ALTITUDE_MUL: f64 = 8.0;
const MIN_SPLIT_DIST: f64 = 1.1;
const LEVEL_CEILING: u8 = 32;
const FACE_COUNT: usize = 6;
const NEAR_PROTECT_FLOOR_M: f64 = 10_000.0;
const FAR_FLOOR_KNEE_M: f64 = 5_000.0;
const DEFAULT_FLOOR_SPAN: f64 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NearFloor {
    leaf_m: f64,
    span_leaves: f64,
    inner_span_leaves: f64,
    ground_clearance_m: Option<f64>,
}

fn positive_or(value: f64, fallback: f64) -> f64 {
    if value.is_finite() && value > 0.0 { value } else { fallback }
}

impl NearFloor {
    pub fn new(leaf_m: f64, span_leaves: f64, ground_clearance_m: Option<f64>, inner_span_leaves: f64) -> Option<Self> {
        (leaf_m.is_finite() && leaf_m > 0.0).then(|| Self {
            leaf_m,
            span_leaves: positive_or(span_leaves, DEFAULT_FLOOR_SPAN),
            inner_span_leaves: positive_or(inner_span_leaves, 0.0),
            ground_clearance_m: ground_clearance_m.filter(|c| c.is_finite() && *c >= 0.0),
        })
    }
}

#[derive(Clone, Debug)]
pub struct QuadtreeConfig {
    pub radius: f64,
    pub max_level: u8,
    pub split_factor: f64,
    pub viewport_height: f64,
    pub fov_y_rad: f64,
    pub near_floor: Option<NearFloor>,
    pub aim: Option<DVec3>,
}

impl QuadtreeConfig {
    pub fn reference(radius: f64, viewport_height: f64, fov_y_rad: f64) -> Self {
        Self { radius, max_level: DEFAULT_MAX_LEVEL, split_factor: DEFAULT_SPLIT_FACTOR, viewport_height, fov_y_rad, near_floor: None, aim: None }
    }

    fn split_dist(&self) -> f64 {
        let tan_40 = (40.0 / 180.0 * PI).tan();
        let sd = self.split_factor * LOD_STEP * self.viewport_height / 1024.0 * tan_40 / (self.fov_y_rad / 2.0).tan();
        if sd >= MIN_SPLIT_DIST && sd.is_finite() { sd } else { MIN_SPLIT_DIST }
    }

    fn dist_factor(&self) -> f64 {
        self.split_factor * LOD_POP_ALTITUDE_MUL
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct NodeKey {
    pub face: u8,
    pub level: u8,
    pub x: u32,
    pub y: u32,
}

impl NodeKey {
    pub fn dir(&self, u: f64, v: f64) -> DVec3 {
        let cells = (1u64 << self.level) as f64;
        let warp = |index: u32, t: f64| (std::f64::consts::FRAC_PI_4 * (-1.0 + (index as f64 + t) * 2.0 / cells)).tan();
        let (c, fu, fv) = FACE_FRAME[self.face as usize];
        let (wx, wy) = (warp(self.x, u), warp(self.y, v));
        let d = normalise([0, 1, 2].map(|k| wx * fu[k] + wy * fv[k] + c[k]));
        DVec3::from_array(d)
    }

    pub fn centre_dir(&self) -> DVec3 {
        self.dir(0.5, 0.5)
    }

    pub fn size_m(&self, radius: f64) -> f64 {
        2.0 * radius / (1u64 << self.level) as f64
    }

    pub fn children(&self) -> [NodeKey; 4] {
        let child = |dx: u32, dy: u32| NodeKey { face: self.face, level: self.level + 1, x: 2 * self.x + dx, y: 2 * self.y + dy };
        [child(0, 0), child(1, 0), child(0, 1), child(1, 1)]
    }

    pub fn parent(&self) -> Option<NodeKey> {
        (self.level > 0).then(|| NodeKey { face: self.face, level: self.level - 1, x: self.x >> 1, y: self.y >> 1 })
    }
}

fn face_local(face: usize, p: DVec3, radius: f64) -> [f64; 2] {
    let (c, u, v) = FACE_FRAME[face];
    let p = p.to_array();
    let (cu, cv, cc) = (dot(p, u), dot(p, v), dot(p, c));
    if cc > 1.0 {
        let atan_inv_k = 4.0 / PI;
        [atan_inv_k * radius * (cu / cc).atan(), atan_inv_k * radius * (cv / cc).atan()]
    } else {
        let seam = |c: f64| if c >= 0.0 { radius } else { -radius };
        [seam(cu), seam(cv)]
    }
}

fn pick_face(camera: DVec3) -> usize {
    let len = camera.length();
    let dir = camera.to_array().map(|c| c / if len == 0.0 { 1.0 } else { len });
    let mut best = (0, f64::NEG_INFINITY);
    for (face, frame) in FACE_FRAME.iter().enumerate() {
        let d = dot(dir, frame.0);
        if d > best.1 {
            best = (face, d);
        }
    }
    best.0
}

pub fn aim_ground_point(camera: DVec3, forward: DVec3, radius: f64) -> Option<DVec3> {
    let fl = forward.length();
    let f = forward / if fl == 0.0 { 1.0 } else { fl };
    let cam_dist = camera.length();
    let b = camera.dot(f);
    let disc = b * b - (cam_dist * cam_dist - radius * radius);
    if !(disc > 0.0) {
        return None;
    }
    let t = -b - disc.sqrt();
    (t > 0.0).then(|| camera + f * t)
}

struct FaceWalk<'a> {
    face: u8,
    config: &'a QuadtreeConfig,
    cam: [f64; 2],
    aim: Option<[f64; 2]>,
    altitude_term: f64,
    near_protect: f64,
    far_floor: f64,
    split_dist: f64,
    floor: Option<NearFloor>,
    leaves: &'a mut Vec<NodeKey>,
}

impl FaceWalk<'_> {
    fn camera_dist(&self, ox: f64, oy: f64, l: f64) -> f64 {
        let dx = (self.cam[0] - ox).abs().min((self.cam[0] - (ox + l)).abs());
        let dy = (self.cam[1] - oy).abs().min((self.cam[1] - (oy + l)).abs());
        self.altitude_term.max(dx.max(dy))
    }

    fn lateral(&self, point: [f64; 2], ox: f64, oy: f64, l: f64) -> f64 {
        let nx = ox.max(point[0].min(ox + l));
        let ny = oy.max(point[1].min(oy + l));
        (point[0] - nx).abs().max((point[1] - ny).abs())
    }

    fn forced(&self, floor: NearFloor, level: u8, ox: f64, oy: f64, l: f64) -> bool {
        if !(l > floor.leaf_m && level < LEVEL_CEILING) {
            return false;
        }
        let [cx, cy] = self.cam;
        let gx = (ox - cx).max(0.0).max(cx - (ox + l));
        let gy = (oy - cy).max(0.0).max(cy - (oy + l));
        let fx = (cx - ox).abs().max((cx - (ox + l)).abs());
        let fy = (cy - oy).abs().max((cy - (oy + l)).abs());
        let floor_altitude_term = match floor.ground_clearance_m {
            None => self.altitude_term,
            Some(clearance) => (clearance / self.config.dist_factor()).max(0.0),
        };
        let gap = floor_altitude_term.max(gx).max(gy);
        let far_gap = floor_altitude_term.max(fx).max(fy);
        gap <= floor.leaf_m * floor.span_leaves && far_gap > floor.leaf_m * floor.inner_span_leaves
    }

    fn recurse(&mut self, level: u8, tx: u32, ty: u32, ox: f64, oy: f64, l: f64) {
        let dist = self.camera_dist(ox, oy, l);
        let mut lat = self.lateral(self.cam, ox, oy, l);
        if let Some(aim) = self.aim {
            lat = lat.min(self.lateral(aim, ox, oy, l));
        }
        let fall = 1.0 / (1.0 + (lat - self.near_protect).max(0.0) / self.near_protect);
        let eff_split = self.split_dist * self.far_floor.max(fall);
        let forced = self.floor.is_some_and(|floor| self.forced(floor, level, ox, oy, l));
        if (dist < l * eff_split && level < self.config.max_level) || forced {
            let hl = l / 2.0;
            self.recurse(level + 1, 2 * tx, 2 * ty, ox, oy, hl);
            self.recurse(level + 1, 2 * tx + 1, 2 * ty, ox + hl, oy, hl);
            self.recurse(level + 1, 2 * tx, 2 * ty + 1, ox, oy + hl, hl);
            self.recurse(level + 1, 2 * tx + 1, 2 * ty + 1, ox + hl, oy + hl, hl);
        } else {
            self.leaves.push(NodeKey { face: self.face, level, x: tx, y: ty });
        }
    }
}

pub fn select_leaves(config: &QuadtreeConfig, camera: DVec3, camera_altitude: f64) -> Vec<NodeKey> {
    let radius = config.radius;
    let horizon = (2.0 * radius * camera_altitude.max(0.0)).sqrt();
    let far_octaves = (camera_altitude.max(FAR_FLOOR_KNEE_M) / FAR_FLOOR_KNEE_M).log2();
    let near_face = pick_face(camera);
    let split_dist = config.split_dist();
    let mut leaves = Vec::new();
    for face in 0..FACE_COUNT {
        let cam = face_local(face, camera, radius);
        FaceWalk {
            face: face as u8,
            config,
            cam,
            aim: config.aim.map(|aim| face_local(face, aim, radius)),
            altitude_term: (camera_altitude / config.dist_factor()).max(0.0),
            near_protect: (camera_altitude * 2.0)
                .max(horizon * 0.2)
                .max(NEAR_PROTECT_FLOOR_M * (radius / REFERENCE_RADIUS)),
            far_floor: (0.60 - far_octaves * 0.05).min(0.60).max(0.30),
            split_dist,
            floor: config.near_floor.filter(|_| face == near_face),
            leaves: &mut leaves,
        }
        .recurse(0, 0, 0, -radius, -radius, 2.0 * radius);
    }
    leaves
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{cases, close, parse};

    const FIXTURE: &str = include_str!("../../testdata/parity/spoint-planet-quadtree.json");

    fn vec3(value: &serde_json::Value) -> DVec3 {
        DVec3::new(value[0].as_f64().unwrap(), value[1].as_f64().unwrap(), value[2].as_f64().unwrap())
    }

    fn near_floor(value: &serde_json::Value) -> Option<NearFloor> {
        if value.is_null() {
            return None;
        }
        NearFloor::new(value["leafM"].as_f64().unwrap(), value["spanLeaves"].as_f64().unwrap(), value["groundClearanceM"].as_f64(), value["innerSpanLeaves"].as_f64().unwrap())
    }

    fn key(face: u64, level: u64, x: u64, y: u64) -> NodeKey {
        NodeKey { face: face as u8, level: level as u8, x: x as u32, y: y as u32 }
    }

    fn parse_leaves(text: &str) -> Vec<NodeKey> {
        text.split(' ')
            .map(|leaf| {
                let [f, l, x, y]: [u64; 4] = leaf.split(':').map(|n| n.parse().unwrap()).collect::<Vec<_>>().try_into().unwrap();
                key(f, l, x, y)
            })
            .collect()
    }

    #[test]
    fn defaults_match_spoint() {
        let defaults = &parse(FIXTURE)["defaults"];
        let config = QuadtreeConfig::reference(REFERENCE_RADIUS, 1080.0, 1.0);
        assert_eq!(config.max_level as u64, defaults["maxLevel"].as_u64().unwrap());
        assert_eq!(config.split_factor, defaults["splitFactor"].as_f64().unwrap());
        assert_eq!(REFERENCE_RADIUS, defaults["radius"].as_f64().unwrap());
        assert_eq!(LOD_STEP, defaults["lodStep"].as_f64().unwrap());
        assert_eq!(LOD_POP_ALTITUDE_MUL, defaults["popAltitudeMul"].as_f64().unwrap());
    }

    #[test]
    fn leaf_sets_match_spoint() {
        let fixture = parse(FIXTURE);
        let all = cases(&fixture, "cases");
        assert_eq!(all.len(), 172);
        for case in all {
            let name = case["name"].as_str().unwrap();
            let radius = case["radius"].as_f64().unwrap();
            let camera = vec3(&case["camera"]);
            let mut config = QuadtreeConfig::reference(radius, case["viewportH"].as_f64().unwrap(), case["fovDegrees"].as_f64().unwrap() / 180.0 * PI);
            config.max_level = case["maxLevel"].as_u64().unwrap() as u8;
            config.split_factor = case["splitFactor"].as_f64().unwrap();
            config.near_floor = near_floor(&case["nearFloor"]);
            close(config.split_dist(), case["splitDist"].as_f64().unwrap(), 1e-12, name);
            config.aim = (!case["forward"].is_null()).then(|| aim_ground_point(camera, vec3(&case["forward"]), radius)).flatten();
            assert_eq!(config.aim.is_some(), !case["aim"].is_null(), "{name}: aim presence");
            if let Some(aim) = config.aim {
                let want = vec3(&case["aim"]);
                close(aim.distance(want), 0.0, 1e-6 * radius, name);
            }
            let leaves = select_leaves(&config, camera, camera.length() - radius);
            let want = parse_leaves(case["leaves"].as_str().unwrap());
            assert_eq!(leaves.len(), case["leafCount"].as_u64().unwrap() as usize, "{name}: leaf count");
            assert!(leaves == want, "{name}: leaf set differs from spoint");
        }
    }

    #[test]
    fn node_geometry_matches_spoint() {
        let fixture = parse(FIXTURE);
        let nodes = cases(&fixture, "nodes");
        assert_eq!(nodes.len(), 111);
        for node in nodes {
            let k = key(node["face"].as_u64().unwrap(), node["level"].as_u64().unwrap(), node["x"].as_u64().unwrap(), node["y"].as_u64().unwrap());
            let name = format!("{k:?}");
            close(k.size_m(node["radius"].as_f64().unwrap()), node["size"].as_f64().unwrap(), 1e-9, &name);
            for corner in node["corners"].as_array().unwrap() {
                let uv = corner["uv"].as_array().unwrap();
                let got = k.dir(uv[0].as_f64().unwrap(), uv[1].as_f64().unwrap());
                let want = vec3(&corner["dir"]);
                close(got.distance(want), 0.0, 1e-9, &format!("{name} dir {uv:?}"));
            }
            close(k.centre_dir().distance(vec3(&node["corners"][4]["dir"])), 0.0, 1e-9, &name);
            let want_children: Vec<NodeKey> = node["children"].as_array().unwrap().iter().map(|c| key(k.face as u64, c[0].as_u64().unwrap(), c[1].as_u64().unwrap(), c[2].as_u64().unwrap())).collect();
            assert_eq!(k.children().to_vec(), want_children, "{name}: children");
            let want_parent = (!node["parent"].is_null()).then(|| key(k.face as u64, node["parent"][0].as_u64().unwrap(), node["parent"][1].as_u64().unwrap(), node["parent"][2].as_u64().unwrap()));
            assert_eq!(k.parent(), want_parent, "{name}: parent");
        }
    }
}
