use super::*;
use crate::planet_lod::guard;
use serde_json::Value;

const REFERENCE: &str = include_str!("../../../testdata/parity/spoint-planet-grass-clump.json");

fn reference() -> Value {
    serde_json::from_str(REFERENCE).unwrap()
}

fn numbers(value: &Value) -> Vec<f64> {
    value.as_array().unwrap().iter().map(|number| number.as_f64().unwrap()).collect()
}

fn flat3(rows: &[[f32; 3]]) -> Vec<f64> {
    rows.iter().flatten().map(|value| f64::from(*value)).collect()
}

fn assert_close(got: &[f64], want: &[f64], tolerance: f64, label: &str) {
    assert_eq!(got.len(), want.len(), "{label} length");
    for (index, (got, want)) in got.iter().zip(want).enumerate() {
        assert!((got - want).abs() <= tolerance, "{label}[{index}]: got {got} want {want}");
    }
}

#[test]
fn clump_geometry_equals_the_reference_make_clump_geo() {
    let reference = reference();
    for (key, segments, cards) in [("2_1", NEAR_SEGMENTS, CLUMP_CARDS), ("1_1", MID_SEGMENTS, CLUMP_CARDS), ("3_3", 3, 3)] {
        let want = &reference["geo"][key];
        let got = clump_geometry(segments, cards);
        assert_close(&flat3(&got.positions), &numbers(&want["position"]), 1e-6, &format!("{key} position"));
        assert_close(&flat3(&got.normals), &numbers(&want["normal"]), 1e-6, &format!("{key} normal"));
        let uvs: Vec<f64> = got.uvs.iter().flatten().map(|value| f64::from(*value)).collect();
        assert_close(&uvs, &numbers(&want["uv"]), 1e-6, &format!("{key} uv"));
        let indices: Vec<f64> = got.indices.iter().map(|index| f64::from(*index)).collect();
        assert_eq!(indices, numbers(&want["index"]), "{key} index");
    }
}

#[test]
fn the_near_and_mid_lods_differ_in_segments_only() {
    let near = clump_geometry(NEAR_SEGMENTS, CLUMP_CARDS);
    let mid = clump_geometry(MID_SEGMENTS, CLUMP_CARDS);
    assert_eq!((near.indices.len(), near.positions.len()), (12, 6));
    assert_eq!((mid.indices.len(), mid.positions.len()), (6, 4));
}

#[test]
fn the_blade_table_follows_the_reference_painter_order() {
    let blades = clump_blades();
    assert_eq!(blades.len(), CLUMP_BLADES);
    assert!(blades.windows(2).all(|pair| pair[0].height <= pair[1].height));
    assert!(blades.iter().all(|blade| (0.1..=0.9).contains(&blade.base_x) && (0.5..=0.98).contains(&blade.height) && (0.62..=1.0).contains(&blade.shade)));
}

#[test]
fn the_baked_clump_texture_equals_the_reference_polygons_away_from_edges() {
    let reference = reference();
    let bytes = bake_clump_mips();
    let probes = reference["probes"].as_array().unwrap();
    let painted = probes.iter().filter(|probe| probe[2].as_f64().unwrap() > 0.0).count();
    assert!(painted >= 30, "{painted} painted probes");
    for probe in probes {
        let (x, y, want) = (probe[0].as_u64().unwrap() as usize, probe[1].as_u64().unwrap() as usize, probe[2].as_f64().unwrap());
        let got = f64::from(bytes[y * CLUMP_TEX_SIZE + x]);
        assert!((got - want).abs() <= 2.0, "pixel ({x}, {y}): got {got} want {want}");
    }
}

#[test]
fn the_clump_texture_carries_a_full_mip_chain() {
    let bytes = bake_clump_mips();
    let expected: usize = (0..CLUMP_MIP_LEVELS).map(|level| (CLUMP_TEX_SIZE >> level).pow(2)).sum();
    assert_eq!(bytes.len(), expected);
    let mean = |slice: &[u8]| slice.iter().map(|value| f64::from(*value)).sum::<f64>() / slice.len() as f64;
    let top = CLUMP_TEX_SIZE * CLUMP_TEX_SIZE;
    assert!((mean(&bytes[..top]) - mean(&bytes[top..top + top / 4])).abs() < 0.5);
    assert!(mean(&bytes[..top]) > 5.0);
}

#[test]
fn the_params_block_keeps_the_wgsl_layout() {
    let look = GrassLook { wind_origin: Vec2::new(3.0, 4.0), ..GrassLook::default() };
    let environment = VegEnvironment { time_s: 8.0, wind_strength: 2.0, ..VegEnvironment::default() };
    let bytes = grass_params_bytes(&look, &environment, &VegCullSettings::default().grass);
    assert_eq!(bytes.len(), GRASS_PARAMS_BYTES);
    let words: Vec<f32> = bytes.chunks_exact(4).map(|word| f32::from_le_bytes(word.try_into().unwrap())).collect();
    assert_eq!(words[..18], [1.0, 0.0, 0.0, 2.0, 0.0, 0.0, 1.0, 0.5, 3.0, 4.0, 0.8, 0.6, 80.0, 16.0, 5.0, 2.268, 1.176, 0.75]);
}

#[test]
fn the_clump_and_meshes_are_built_off_the_main_thread() {
    guard::mark_main_thread();
    let built = bevy::tasks::block_on(AsyncComputeTaskPool::get_or_init(Default::default).spawn(async { build_assets().clump.data.map(|data| data.len()) }));
    assert!(built.is_some());
    assert_eq!(guard::builds_on_main_thread(), 0);
}
