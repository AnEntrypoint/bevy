use bevy::asset::RenderAssetUsages;
use bevy::camera::{ClearColorConfig, RenderTarget};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::math::DVec3;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::pipelined_rendering::PipelinedRenderingPlugin;
use bevy::render::render_resource::{PollType, TextureFormat};
use bevy::render::renderer::RenderDevice;
use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};
use bevy::window::ExitCondition;
use bevy::winit::WinitPlugin;
use std::sync::Once;

#[cfg(test)]
pub(crate) use readback::*;

const LAVAPIPE_ICD: &str = "/usr/share/vulkan/icd.d/lvp_icd.json";
const NEAR_M: f32 = 1.0;
const FAR_M: f32 = 3.0e7;
pub(crate) const WIDTH: u32 = 320;
pub(crate) const HEIGHT: u32 = 180;
pub(crate) const FOV_Y: f64 = std::f64::consts::FRAC_PI_4;

pub(crate) struct Frame {
    pub(crate) position: DVec3,
    pub(crate) forward: DVec3,
    pub(crate) up: DVec3,
    pub(crate) sun_to: DVec3,
}

impl Frame {
    pub(crate) fn above(dir: DVec3, surface_radius: f64, altitude_m: f64, pitch_down_deg: f64) -> Self {
        let east = DVec3::Y.cross(dir).try_normalize().unwrap_or(DVec3::X);
        let north = dir.cross(east);
        let pitch = pitch_down_deg.to_radians();
        Self {
            position: dir * (surface_radius + altitude_m),
            forward: north * pitch.cos() - dir * pitch.sin(),
            up: north * pitch.sin() + dir * pitch.cos(),
            sun_to: (dir * 0.7 + east * 0.5 + north * 0.2).normalize(),
        }
    }
}

fn use_lavapipe() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| unsafe {
        std::env::set_var("VK_ICD_FILENAMES", LAVAPIPE_ICD);
        std::env::set_var("WGPU_BACKEND", "vulkan");
    });
}

pub(crate) fn headless_app() -> App {
    use_lavapipe();
    let window = WindowPlugin { primary_window: None, exit_condition: ExitCondition::DontExit, close_when_requested: false, ..default() };
    let render = RenderPlugin {
        render_creation: RenderCreation::Automatic(Box::new(WgpuSettings { backends: Some(Backends::VULKAN), ..default() })),
        synchronous_pipeline_compilation: true,
        ..default()
    };
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .build()
            .disable::<bevy::log::LogPlugin>()
            .disable::<WinitPlugin>()
            .disable::<PipelinedRenderingPlugin>()
            .set(window)
            .set(render),
    );
    app
}

pub(crate) fn pump(app: &mut App) {
    app.update();
    let device = app.world().resource::<RenderDevice>().wgpu_device().clone();
    device.poll(PollType::Wait { submission_index: None, timeout: None }).unwrap();
}

pub(crate) fn add_target(app: &mut App, format: TextureFormat) -> Handle<Image> {
    let mut target = Image::new_target_texture(WIDTH, HEIGHT, format, None);
    target.asset_usage = RenderAssetUsages::RENDER_WORLD;
    app.world_mut().resource_mut::<Assets<Image>>().add(target)
}

pub(crate) fn spawn_view(app: &mut App, target: &Handle<Image>, clear: Color, frame: &Frame, sun_side: f64) {
    let projection = PerspectiveProjection { fov: FOV_Y as f32, near: NEAR_M, far: FAR_M, ..default() };
    app.world_mut().spawn((
        Camera3d::default(),
        Camera { clear_color: ClearColorConfig::Custom(clear), ..default() },
        RenderTarget::from(target.clone()),
        Projection::Perspective(projection),
        Msaa::Off,
        Tonemapping::None,
        Transform::IDENTITY.looking_to(frame.forward.as_vec3(), frame.up.as_vec3()),
    ));
    app.world_mut().spawn((
        DirectionalLight { shadow_maps_enabled: false, ..default() },
        Transform::IDENTITY.looking_to((-frame.sun_to * sun_side).as_vec3(), frame.up.as_vec3()),
    ));
}

#[cfg(test)]
mod readback {
    use super::*;
    use bevy::render::RenderApp;
    use bevy::render::render_resource::{CachedPipelineState, PipelineCache};
    use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
    use std::collections::HashSet;
    use std::sync::{Arc, Mutex, MutexGuard};

    const WARMUP_FRAMES: usize = 6;
    const CAPTURE_FRAME_LIMIT: usize = 30;
    pub(crate) const SKY_COLOUR: Color = Color::srgb(1.0, 0.0, 1.0);
    pub(crate) const SKY_TEXEL: [u8; 4] = [255, 0, 255, 255];
    static RENDER_LOCK: Mutex<()> = Mutex::new(());

    impl Frame {
        pub(crate) fn pixel_ray(&self, x: u32, y: u32) -> DVec3 {
            let right = self.forward.cross(self.up).normalize();
            let up = right.cross(self.forward);
            let half = (FOV_Y / 2.0).tan();
            let aspect = f64::from(WIDTH) / f64::from(HEIGHT);
            let ndc_x = 2.0 * (f64::from(x) + 0.5) / f64::from(WIDTH) - 1.0;
            let ndc_y = 1.0 - 2.0 * (f64::from(y) + 0.5) / f64::from(HEIGHT);
            (self.forward + right * (ndc_x * half * aspect) + up * (ndc_y * half)).normalize()
        }
    }

    pub(crate) fn render_lock() -> MutexGuard<'static, ()> {
        RENDER_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub(crate) fn pipeline_states(app: &App) -> (usize, Vec<String>) {
        let cache = app.sub_app(RenderApp).world().resource::<PipelineCache>();
        let compiled = cache.pipelines().filter(|pipeline| matches!(pipeline.state, CachedPipelineState::Ok(_))).count();
        let errors = cache
            .pipelines()
            .filter_map(|pipeline| match &pipeline.state {
                CachedPipelineState::Err(error) => Some(error.to_string()),
                _ => None,
            })
            .collect();
        (compiled, errors)
    }

    pub(crate) fn capture(app: &mut App, target: Handle<Image>, label: &str) -> Image {
        for _ in 0..WARMUP_FRAMES {
            pump(app);
        }
        let captured = Arc::new(Mutex::new(None));
        let slot = captured.clone();
        app.world_mut().spawn(Screenshot::image(target)).observe(move |event: On<ScreenshotCaptured>| {
            *slot.lock().unwrap() = Some(event.image.clone());
        });
        for _ in 0..CAPTURE_FRAME_LIMIT {
            pump(app);
            if captured.lock().unwrap().is_some() {
                break;
            }
        }
        let (compiled, errors) = pipeline_states(app);
        assert!(errors.is_empty(), "pipeline errors for {label}:\n{}", errors.join("\n"));
        assert!(compiled > 0, "no pipeline reached the Ok state for {label}");
        let image = captured.lock().unwrap().take();
        image.unwrap_or_else(|| panic!("no capture within {CAPTURE_FRAME_LIMIT} frames for {label}"))
    }

    pub(crate) fn texel(image: &Image, x: u32, y: u32) -> [u8; 4] {
        let data = image.data.as_ref().unwrap();
        let at = ((y * image.width() + x) * 4) as usize;
        [data[at], data[at + 1], data[at + 2], data[at + 3]]
    }

    pub(crate) struct ImageStats {
        pub(crate) covered: f64,
        pub(crate) luma_std: f64,
        pub(crate) distinct_colours: usize,
    }

    pub(crate) fn image_stats(image: &Image) -> ImageStats {
        let mut lumas = Vec::new();
        let mut colours = HashSet::new();
        for y in 0..image.height() {
            for x in 0..image.width() {
                let t = texel(image, x, y);
                if t == SKY_TEXEL {
                    continue;
                }
                lumas.push((0.2126 * f64::from(t[0]) + 0.7152 * f64::from(t[1]) + 0.0722 * f64::from(t[2])) / 255.0);
                colours.insert([t[0] >> 2, t[1] >> 2, t[2] >> 2]);
            }
        }
        let count = lumas.len().max(1) as f64;
        let mean = lumas.iter().sum::<f64>() / count;
        ImageStats {
            covered: lumas.len() as f64 / f64::from(image.width() * image.height()),
            luma_std: (lumas.iter().map(|l| (l - mean).powi(2)).sum::<f64>() / count).sqrt(),
            distinct_colours: colours.len(),
        }
    }

    pub(crate) fn mean_change(a: &Image, b: &Image) -> f64 {
        let (a, b) = (a.data.as_ref().unwrap(), b.data.as_ref().unwrap());
        let pairs = a.chunks_exact(4).zip(b.chunks_exact(4)).filter(|(x, _)| x[..] != SKY_TEXEL[..]);
        let (sum, count) = pairs.fold((0.0, 0usize), |(sum, count), (x, y)| (sum + (0..3).map(|c| (f64::from(x[c]) - f64::from(y[c])).abs()).sum::<f64>() / 3.0 / 255.0, count + 1));
        sum / count.max(1) as f64
    }

    pub(crate) fn write_png(image: &Image, file_stem: &str) {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/witness").join(format!("{file_stem}.png"));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut encoder = png::Encoder::new(std::io::BufWriter::new(std::fs::File::create(path).unwrap()), image.width(), image.height());
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(image.data.as_ref().unwrap()).unwrap();
    }
}
