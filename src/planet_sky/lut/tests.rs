use super::*;
use crate::planet_lod::guard::{builds_on_main_thread, mark_main_thread};

const BAKE_WAIT_MS: u64 = 50;
const BAKE_WAIT_LIMIT: usize = 4800;
const HALF_RELATIVE_ERROR: f32 = 1.0 / 2048.0;

#[test]
fn half_conversion_matches_the_ieee_binary16_values() {
    let cases: [(f32, u16); 9] = [
        (0.0, 0x0000),
        (1.0, 0x3c00),
        (0.5, 0x3800),
        (-2.0, 0xc000),
        (65504.0, 0x7bff),
        (65520.0, 0x7c00),
        (6.1035156e-5, 0x0400),
        (5.9604645e-8, 0x0001),
        (0.333_333_34, 0x3555),
    ];
    for (value, bits) in cases {
        assert_eq!(half_bits(value), bits, "{value}");
    }
    assert_eq!(half_bits(1.0 + 1.0 / 2048.0), 0x3c00, "ties round to even");
    assert_eq!(half_bits(1.0 + 3.0 / 2048.0), 0x3c02, "ties round to even upward");
}

#[test]
fn every_baked_value_survives_the_half_float_upload_within_half_precision() {
    let baked = baked();
    let (transmittance, scattering) = baked.images();
    let check = |image: &Image, expected: Vec<f32>| {
        let data = image.data.as_ref().unwrap();
        assert_eq!(data.len(), expected.len() * 2);
        for (chunk, want) in data.chunks_exact(2).zip(&expected) {
            let got = half_value(u16::from_le_bytes([chunk[0], chunk[1]]));
            let tolerance = want.abs() * HALF_RELATIVE_ERROR + 6.0e-8;
            assert!((got - want).abs() <= tolerance, "{got} against {want}");
        }
    };
    check(&transmittance, baked.transmittance.data.chunks_exact(3).flat_map(|texel| [texel[0], texel[1], texel[2], 1.0]).collect());
    check(&scattering, baked.scattering.data.clone());
}

#[test]
fn the_images_have_the_reference_sizes_formats_views_and_samplers() {
    let (transmittance, scattering) = baked().images();
    assert_eq!((transmittance.width(), transmittance.height(), transmittance.texture_descriptor.size.depth_or_array_layers), (256, 128, 1));
    assert_eq!((scattering.width(), scattering.height(), scattering.texture_descriptor.size.depth_or_array_layers), (64, 32, 24));
    for image in [&transmittance, &scattering] {
        assert_eq!(image.texture_descriptor.format, TextureFormat::Rgba16Float);
        assert!(matches!(&image.sampler, ImageSampler::Descriptor(d) if d.mag_filter == bevy::image::ImageFilterMode::Linear && d.address_mode_u == bevy::image::ImageAddressMode::ClampToEdge));
    }
    assert_eq!(scattering.texture_view_descriptor.as_ref().unwrap().dimension, Some(TextureViewDimension::D2Array));
    assert!(transmittance.texture_view_descriptor.is_none());
}

#[test]
fn the_sampling_source_binds_the_requested_slots_and_leaves_no_token() {
    let source = sampling_source("2", 105, 106, 107);
    assert!(source.contains("@group(2) @binding(105) var atm_transmittance_lut"));
    assert!(source.contains("@group(2) @binding(106) var atm_scattering_lut"));
    assert!(source.contains("@group(2) @binding(107) var atm_lut_sampler"));
    assert!(!source.contains("ATM_LUT_"));
}

#[test]
fn the_plugin_bakes_off_the_main_thread_and_reports_readiness_when_the_tables_are_uploaded() {
    mark_main_thread();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.init_asset::<Image>().add_plugins(AtmosphereLutPlugin);
    assert!(!app.world_mut().run_system_cached(luts_ready).unwrap());
    app.update();
    assert!(app.world().get_resource::<AtmosphereLuts>().is_none(), "ready before the bake could have finished");
    let mut ticks = 0;
    while app.world().get_resource::<AtmosphereLuts>().is_none() {
        assert!(ticks < BAKE_WAIT_LIMIT, "bake did not finish");
        std::thread::sleep(std::time::Duration::from_millis(BAKE_WAIT_MS));
        app.update();
        ticks += 1;
    }
    let luts = app.world().resource::<AtmosphereLuts>().clone();
    let images = app.world().resource::<Assets<Image>>();
    assert_eq!(images.get(&luts.transmittance).unwrap().width(), 256);
    assert_eq!(images.get(&luts.scattering).unwrap().texture_descriptor.size.depth_or_array_layers, 24);
    assert_eq!(builds_on_main_thread(), 0, "the bake ran on the main thread");
    assert!(app.world().get_resource::<LutBake>().is_none());
    assert!(app.world_mut().run_system_cached(luts_ready).unwrap());
}
