use super::leaves::*;
use super::mesh::*;
use super::surface::UNDERWATER_BELOW_M;
use super::waves::*;
use crate::planet_lod::quadtree::NodeKey;
use crate::testkit::{cases, close, parse};
use bevy::math::DVec3;
use serde_json::Value;
use std::ops::Range;
use std::sync::OnceLock;

const EXACT: f64 = 1e-9;
const MEAN_TOLERANCE: f64 = 1e-9;
const RADIUS_TOLERANCE_M: f64 = 1e-3;
const TILE_FIRST_NODE: NodeKey = NodeKey { face: 4, level: 11, x: 1000, y: 1500 };

pub(super) fn fixture() -> &'static Value {
    static FIXTURE: OnceLock<Value> = OnceLock::new();
    FIXTURE.get_or_init(|| parse(include_str!("../../testdata/parity/spoint-planet-waves.json")))
}

fn number(value: &Value, name: &str) -> f64 {
    value[name].as_f64().unwrap()
}

fn point(value: &Value) -> Point {
    [value[0].as_f64().unwrap(), value[1].as_f64().unwrap()]
}

fn octaves(case: &Value) -> Range<usize> {
    number(case, "start") as usize..number(case, "end") as usize
}

fn key(value: &Value) -> NodeKey {
    let at = |i: usize| value[i].as_u64().unwrap();
    NodeKey { face: at(0) as u8, level: at(1) as u8, x: at(2) as u32, y: at(3) as u32 }
}

#[test]
fn constants_match_the_reference_sources() {
    let constants = &fixture()["constants"];
    let (tsl, water, mean) = (&constants["tsl/sea-waves-tsl.js"], &constants["tsl/water-tsl.js"], &constants["sea-waves.js"]);
    let expected = [
        (tsl, "SEA_OCTAVES", SEA_OCTAVES as f64),
        (tsl, "SEA_FREQ", SEA_FREQ),
        (tsl, "SEA_SPEED", SEA_SPEED),
        (tsl, "SEA_AMP_M", SEA_AMP_M),
        (tsl, "SEA_OCTAVE_ANGLE", SEA_OCTAVE_ANGLE),
        (tsl, "SEA_FREQ_LF", SEA_FREQ_LF),
        (mean, "BASE_AMP_M", SEA_AMP_M),
        (mean, "OCTAVES", SEA_OCTAVES as f64),
        (water, "SLOPE_EPS_M", SLOPE_EPS_M),
        (water, "SLOPE_EPS_LF_M", SLOPE_EPS_LF_M),
        (water, "WAVE_TILE_M", WAVE_TILE_M),
        (water, "WATER_GRID", WATER_GRID as f64),
        (water, "WATER_MAX_LEVEL", f64::from(WATER_MAX_LEVEL)),
        (water, "LAND_ABOVE_SEA_DISCARD_M", LAND_ABOVE_SEA_DISCARD_M),
        (water, "UNDERWATER_BELOW_M", UNDERWATER_BELOW_M),
    ];
    for (source, name, got) in expected {
        close(got, source[name].as_f64().unwrap(), 0.0, name);
    }
    let ocean = &water["DEFAULT_OCEAN"];
    close(DEFAULT_AMPLITUDE, number(ocean, "amplitude"), 0.0, "default amplitude");
    close(DEFAULT_CHOPPINESS, number(ocean, "choppiness"), 0.0, "default choppiness");
    close(DEFAULT_FOAM, number(ocean, "foam"), 0.0, "default foam");
}

#[test]
fn noise_and_octave_match_the_reference_nodes() {
    let noise = cases(fixture(), "noise");
    let octave_cases = cases(fixture(), "octave");
    assert_eq!((noise.len(), octave_cases.len()), (7, 35));
    for case in noise {
        close(super::waves::noise(point(&case["p"])), number(case, "v"), EXACT, &format!("noise {}", case["p"]));
    }
    for case in octave_cases {
        close(octave(point(&case["p"]), number(case, "choppy")), number(case, "v"), EXACT, &format!("octave {} {}", case["p"], case["choppy"]));
    }
}

#[test]
fn sea_height_matches_the_reference_over_time_windows_and_oceans() {
    let heights = cases(fixture(), "height");
    assert_eq!(heights.len(), 6 * 4 * 3 * 5);
    for case in heights {
        let got = sea_height(point(&case["p"]), number(case, "t"), octaves(case), number(case, "amp"), number(case, "choppy"));
        close(got, number(case, "h"), EXACT, &format!("seaHeight {case}"));
    }
}

#[test]
fn low_frequency_height_and_slope_match_the_reference() {
    let lows = cases(fixture(), "heightLF");
    assert_eq!(lows.len(), 6 * 4 * 3);
    for case in lows {
        let (p, t, amp, choppy) = (point(&case["p"]), number(case, "t"), number(case, "amp"), number(case, "choppy"));
        close(sea_height_lf(p, t, amp, choppy), number(case, "h"), EXACT, &format!("seaHeightLF {case}"));
        let slope = sea_slope_lf(p, t, amp, choppy);
        for axis in 0..2 {
            close(slope[axis], case["s"][axis].as_f64().unwrap(), 1e-7, &format!("slope LF axis {axis} {case}"));
        }
    }
}

#[test]
fn high_frequency_slope_matches_the_reference_differences() {
    let slopes = cases(fixture(), "slope");
    assert_eq!(slopes.len(), 6 * 4 * 3 * 5);
    for case in slopes {
        let slope = sea_slope(point(&case["p"]), number(case, "t"), octaves(case), number(case, "amp"), number(case, "choppy"));
        for axis in 0..2 {
            close(slope[axis], case["s"][axis].as_f64().unwrap(), 1e-7, &format!("slope axis {axis} {case}"));
        }
    }
}

#[test]
fn tangent_normal_is_the_unit_normal_of_the_slope() {
    let flat = tangent_normal([0.0, 0.0]);
    assert_eq!(flat, [0.0, 0.0, 1.0]);
    let tilted = tangent_normal([0.6, -0.8]);
    close(tilted[0], -0.6 / 2f64.sqrt(), EXACT, "normal x");
    close(tilted[1], 0.8 / 2f64.sqrt(), EXACT, "normal y");
    close(tilted[2], 1.0 / 2f64.sqrt(), EXACT, "normal z");
}

#[test]
fn sea_height_mean_matches_the_reference_grid_average() {
    let means = cases(fixture(), "mean");
    assert_eq!(means.len(), 3);
    for case in means {
        close(sea_height_mean(number(case, "choppy"), number(case, "amp")), number(case, "v"), MEAN_TOLERANCE, &format!("seaHeightMean {case}"));
    }
}

#[test]
fn wave_tile_offset_wraps_into_one_tile() {
    close(wave_tile_offset(-1.0), WAVE_TILE_M - 1.0, 0.0, "negative");
    close(wave_tile_offset(WAVE_TILE_M * 3.0 + 5.5), 5.5, 0.0, "positive");
}

#[test]
fn lattice_and_indices_match_the_reference_grid_geometry() {
    let grid = &fixture()["grid"];
    let vertices = grid["vertices"].as_array().unwrap();
    assert_eq!(vertices.len(), LATTICE_POINTS * 3);
    for point in 0..LATTICE_POINTS {
        let [u, v] = lattice_coordinate(point);
        close(u, vertices[point * 3].as_f64().unwrap(), 1e-7, &format!("u {point}"));
        close(v, vertices[point * 3 + 1].as_f64().unwrap(), 1e-7, &format!("v {point}"));
    }
    let wanted: Vec<u32> = grid["indices"].as_array().unwrap().iter().map(|i| i.as_u64().unwrap() as u32).collect();
    assert_eq!(instance_indices(), wanted);
}

fn gate_height(dir: DVec3) -> f64 {
    let gate = &fixture()["surface"]["gate"];
    number(gate, "amplitude") * (number(gate, "kx") * dir.x + number(gate, "phase")).sin() * (number(gate, "ky") * dir.y).cos() + number(gate, "tilt") * dir.z + number(gate, "base")
}

#[test]
fn leaf_set_clamps_dedupes_and_drops_dry_instances_like_setquads() {
    let surface = &fixture()["surface"];
    let leaves: Vec<NodeKey> = surface["input"].as_array().unwrap().iter().map(key).collect();
    let wanted: Vec<NodeKey> = surface["kept"].as_array().unwrap().iter().map(key).collect();
    assert_eq!((leaves.len(), wanted.len()), (21, 8));
    let got: Vec<NodeKey> = water_instances(&leaves, gate_height).iter().map(|instance| instance.key).collect();
    assert_eq!(got, wanted);
}

#[test]
fn instances_carry_the_land_height_of_every_lattice_point() {
    let instances = water_instances(&[TILE_FIRST_NODE], |dir| dir.x * 100.0 - 50.0);
    assert_eq!(instances.len(), 1);
    for point in 0..LATTICE_POINTS {
        let want = lattice_dir(TILE_FIRST_NODE, point).x * 100.0 - 50.0;
        close(f64::from(instances[0].land[point]), want, 1e-4, &format!("land {point}"));
    }
}

#[test]
fn water_mesh_has_the_reference_grid_and_sits_exactly_on_the_sea_radius() {
    const RADIUS: f64 = 6_360_000.0;
    let reference = &fixture()["grid"];
    let (points, indices) = (reference["vertices"].as_array().unwrap().len() / 3, reference["indices"].as_array().unwrap().len());
    let keys = [TILE_FIRST_NODE, NodeKey { face: 2, level: 11, x: 1024, y: 1024 }];
    let instances = water_instances(&keys, |_| -10.0);
    let origin = TILE_FIRST_NODE.centre_dir() * RADIUS;
    let mesh = build_water_mesh(&instances, RADIUS, origin);
    assert_eq!((mesh.positions.len(), mesh.indices.len()), (points * 2, indices * 2));
    assert_eq!(mesh.indices[indices..indices + 6], mesh.indices[..6].iter().map(|index| index + points as u32).collect::<Vec<_>>()[..]);
    for position in &mesh.positions[..points] {
        let world = DVec3::new(f64::from(position[0]), f64::from(position[1]), f64::from(position[2])) + origin;
        close(world.length(), RADIUS, RADIUS_TOLERANCE_M, "vertex radius");
    }
    assert!(mesh.land.iter().all(|land| land[0] == -10.0));
}

#[test]
fn leaves_deeper_than_the_water_maximum_collapse_onto_their_level_eleven_ancestor() {
    let deep = NodeKey { face: 1, level: 14, x: 8000, y: 12000 };
    let ancestor = clamp_level(deep);
    assert_eq!(ancestor, NodeKey { face: 1, level: WATER_MAX_LEVEL, x: 1000, y: 1500 });
    assert_eq!(clamp_level(ancestor), ancestor);
    let shallow = NodeKey { face: 1, level: 6, x: 3, y: 5 };
    assert_eq!(clamp_level(shallow), shallow);
}
