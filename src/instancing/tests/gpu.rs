//! Opt-in full-renderer regression: source material versus the crate-owned draw.
use super::*;
use crate::TiledBackgroundPlugin;
use bevy::{
    app::PluginsState,
    asset::RenderAssetUsages,
    camera::RenderTarget,
    core_pipeline::tonemapping::Tonemapping,
    image::{ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    log::LogPlugin,
    render::{
        RenderApp, RenderPlugin,
        pipelined_rendering::PipelinedRenderingPlugin,
        render_phase::{DrawFunctions, ViewSortedRenderPhases},
        render_resource::{Extent3d, PollType, TextureDimension, TextureFormat},
        renderer::{RenderAdapterInfo, RenderDevice},
        settings::{Backends, RenderCreation, WgpuSettings},
        view::window::screenshot::{Screenshot, ScreenshotCaptured},
    },
    time::TimeUpdateStrategy,
    ui_render::TransparentUi,
    window::ExitCondition,
    winit::WinitPlugin,
};
use std::time::{Duration, Instant};

#[derive(Resource, Default)]
struct Capture(Option<Image>);

fn frame(app: &mut App) {
    app.update();
    app.sub_app(RenderApp)
        .world()
        .resource::<RenderDevice>()
        .poll(PollType::wait_indefinitely())
        .expect("GPU polling failed");
}

fn draw_count<D: bevy::render::render_phase::RenderCommand<TransparentUi> + 'static>(
    app: &App,
) -> usize {
    let world = app.sub_app(RenderApp).world();
    let id = world
        .resource::<DrawFunctions<TransparentUi>>()
        .read()
        .id::<D>();
    world
        .resource::<ViewSortedRenderPhases<TransparentUi>>()
        .values()
        .map(|phase| {
            phase
                .items
                .values()
                .filter(|item| item.draw_function == id)
                .count()
        })
        .sum()
}

fn capture(
    app: &mut App,
    target: &Handle<Image>,
    entity: Entity,
    mode: TiledBackgroundMode,
    active: bool,
) -> Image {
    app.world_mut().entity_mut(entity).insert(mode);
    for _ in 0..12 {
        frame(app);
    }
    assert_eq!(
        draw_count::<crate::instancing::render::DrawPattern>(app) > 0,
        active,
        "effective render path differs from requested case"
    );
    assert_eq!(
        draw_count::<bevy::ui_render::DrawUiMaterial<TiledBackgroundMaterial>>(app),
        usize::from(!active)
    );
    app.world_mut().resource_mut::<Capture>().0 = None;
    app.world_mut()
        .spawn(Screenshot::image(target.clone()))
        .observe(
            |event: On<ScreenshotCaptured>, mut captured: ResMut<Capture>| {
                captured.0 = Some(event.image.clone());
            },
        );
    for _ in 0..120 {
        frame(app);
        if let Some(image) = app.world_mut().resource_mut::<Capture>().0.take() {
            return image;
        }
    }
    panic!("GPU screenshot did not complete");
}

fn texture(mips: u32) -> Image {
    let mut data = Vec::new();
    for level in 0..mips {
        let size = 128 >> level;
        for y in 0..size {
            for x in 0..size {
                let u = x as f32 / (size - 1) as f32;
                let v = y as f32 / (size - 1) as f32;
                let a = ((0.47 - Vec2::new(u - 0.5, v - 0.5).length()) * 12.).clamp(0., 1.);
                data.extend([
                    ((0.2 + u * 0.7) * 255.) as u8,
                    ((0.15 + v * 0.8) * 255.) as u8,
                    160 + level as u8 * 20,
                    (a * 210.) as u8,
                ]);
            }
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: 128,
            height: 128,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![0; 128 * 128 * 4],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.mip_level_count = mips;
    image.data = Some(data);
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        min_filter: ImageFilterMode::Linear,
        mag_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        ..default()
    });
    image
}

fn check(backend: Backends) {
    let mut app = App::new();
    static LOG: std::sync::Once = std::sync::Once::new();
    LOG.call_once(|| {
        app.add_plugins(LogPlugin::default());
    });
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(RenderPlugin {
                render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                    backends: Some(backend),
                    ..default()
                })),
                synchronous_pipeline_compilation: false,
                ..default()
            })
            .disable::<LogPlugin>()
            .disable::<WinitPlugin>()
            .disable::<PipelinedRenderingPlugin>(),
    )
    .add_plugins(TiledBackgroundPlugin)
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
    .init_resource::<Capture>();
    let deadline = Instant::now() + Duration::from_secs(30);
    while app.plugins_state() != PluginsState::Ready {
        assert!(
            Instant::now() < deadline,
            "renderer initialization timed out"
        );
        bevy::tasks::tick_global_task_pools_on_main_thread();
    }
    app.finish();
    app.cleanup();
    let info = app
        .sub_app(RenderApp)
        .world()
        .resource::<RenderAdapterInfo>();
    let name = info.name.to_lowercase();
    assert!(
        !["llvmpipe", "lavapipe", "swiftshader", "software"]
            .iter()
            .any(|s| name.contains(s)),
        "hardware GPU required: {}",
        info.name
    );
    eprintln!(
        "TILED HARDWARE {} {:?} {:?} {}",
        info.name, info.backend, info.device_type, info.driver_info
    );
    let width = 321;
    let height = 639;
    app.world_mut().spawn((
        Window {
            resolution: WindowResolution::new(width, height),
            ..default()
        },
        PrimaryWindow,
    ));
    let target = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new_target_texture(
            width,
            height,
            TextureFormat::Rgba8UnormSrgb,
            None,
        ));
    let camera = app
        .world_mut()
        .spawn((
            Camera2d,
            RenderTarget::Image(target.clone().into()),
            Msaa::Off,
            Tonemapping::None,
        ))
        .id();
    let image = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(texture(1));
    let source = app
        .world_mut()
        .resource_mut::<Assets<TiledBackgroundMaterial>>()
        .add(TiledBackgroundMaterial {
            pattern_texture: image,
            image_scale: 0.4,
            lattice_x_axis: Vec2::new(64., 0.),
            lattice_y_axis: Vec2::new(0., 64.),
            ..default()
        });
    let entity = app
        .world_mut()
        .spawn((
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                ..default()
            },
            UiTargetCamera(camera),
            BackgroundColor(Color::srgb(0.08, 0.14, 0.28)),
            MaterialNode(source.clone()),
            TiledBackgroundMode::Auto,
        ))
        .id();
    // Start optimized from a cold pipeline cache, as ordinary apps do. The
    // reference must survive asynchronous compilation before the custom draw
    // takes over; starting with a warmed reference misses this startup race.
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut saw_reference = false;
    loop {
        frame(&mut app);
        saw_reference |=
            draw_count::<bevy::ui_render::DrawUiMaterial<TiledBackgroundMaterial>>(&app) > 0;
        if draw_count::<crate::instancing::render::DrawPattern>(&app) > 0 {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "automatic draw never became ready"
        );
    }
    assert!(
        saw_reference,
        "cold startup did not exercise the reference fallback"
    );
    let mut cases = 0;
    for (pixel_scale, lattice_x_axis, lattice_y_axis) in [
        (1., Vec2::new(64., 0.), Vec2::new(0., 64.)),
        (2.625, Vec2::new(64., -16.), Vec2::new(20., 72.)),
        (1.5, Vec2::new(-64., 16.), Vec2::new(20., 72.)),
    ] {
        for mips in [1, 3] {
            let image = app
                .world_mut()
                .resource_mut::<Assets<Image>>()
                .add(texture(mips));
            for reference_size in [Vec2::ZERO, Vec2::new(160., 320.)] {
                for parity in [-1, 0, 1] {
                    for rotation in [0., 0.7] {
                        *app.world_mut()
                            .resource_mut::<Assets<TiledBackgroundMaterial>>()
                            .get_mut(&source)
                            .unwrap() = TiledBackgroundMaterial {
                            pattern_texture: image.clone(),
                            color: LinearRgba::new(0.8, 0.6, 0.9, 0.4),
                            image_scale: 0.4,
                            image_rotation: rotation,
                            lattice_x_axis,
                            lattice_y_axis,
                            pixel_scale,
                            origin: Vec2::splat(0.5),
                            reference_size,
                            checkerboard_parity: parity,
                            scroll_velocity: Vec2::new(9., -3.),
                        };
                        app.world_mut()
                            .resource_mut::<Time<Virtual>>()
                            .advance_by(Duration::from_millis(137));
                        let original = capture(
                            &mut app,
                            &target,
                            entity,
                            TiledBackgroundMode::Reference,
                            false,
                        )
                        .try_into_dynamic()
                        .unwrap()
                        .to_rgba8();
                        let optimized =
                            capture(&mut app, &target, entity, TiledBackgroundMode::Auto, true)
                                .try_into_dynamic()
                                .unwrap()
                                .to_rgba8();
                        let maximum = original
                            .as_raw()
                            .iter()
                            .zip(optimized.as_raw())
                            .map(|(a, b)| a.abs_diff(*b))
                            .max()
                            .unwrap();
                        if let Some(directory) = std::env::var_os("TILED_GPU_OUTPUT") {
                            let directory = std::path::PathBuf::from(directory);
                            std::fs::create_dir_all(&directory).unwrap();
                            original
                                .save(directory.join(format!("{backend:?}-{cases}-reference.png")))
                                .unwrap();
                            optimized
                                .save(directory.join(format!("{backend:?}-{cases}-auto.png")))
                                .unwrap();
                        }
                        assert!(
                            maximum <= 3,
                            "{backend:?} case {cases}: max {maximum}/255, scale {pixel_scale}, mips {mips}, reference {reference_size}, parity {parity}, rotation {rotation}"
                        );
                        cases += 1;
                    }
                }
            }
        }
    }
    // Opaque borders expose coverage differences hidden by the logo's soft alpha.
    let mut opaque = texture(1);
    for pixel in opaque.data.as_mut().unwrap().as_chunks_mut::<4>().0 {
        pixel[3] = 255;
    }
    let opaque = app.world_mut().resource_mut::<Assets<Image>>().add(opaque);
    for rotation in [0., 0.7, std::f32::consts::FRAC_PI_2, std::f32::consts::PI] {
        for origin in [
            Vec2::ZERO,
            Vec2::splat(0.5),
            Vec2::splat(0.001),
            Vec2::new(0.37, 0.19),
        ] {
            // Keep Auto active while changing the source, including its texture.
            let old_geometry = app
                .world()
                .entity(entity)
                .get::<MaterialNode<PatternGeometryMaterial>>()
                .unwrap()
                .0
                .clone();
            *app.world_mut()
                .resource_mut::<Assets<TiledBackgroundMaterial>>()
                .get_mut(&source)
                .unwrap() = TiledBackgroundMaterial {
                pattern_texture: opaque.clone(),
                image_scale: 0.125,
                image_rotation: rotation,
                origin,
                color: LinearRgba::new(0.5, 0.8, 0.4, 0.7),
                lattice_x_axis: Vec2::new(64., 0.),
                lattice_y_axis: Vec2::new(0., 64.),
                ..default()
            };
            let optimized = capture(&mut app, &target, entity, TiledBackgroundMode::Auto, true)
                .try_into_dynamic()
                .unwrap()
                .to_rgba8();
            assert_eq!(
                app.world()
                    .entity(entity)
                    .get::<MaterialNode<PatternGeometryMaterial>>()
                    .unwrap()
                    .0,
                old_geometry
            );
            let original = capture(
                &mut app,
                &target,
                entity,
                TiledBackgroundMode::Reference,
                false,
            )
            .try_into_dynamic()
            .unwrap()
            .to_rgba8();
            let maximum = original
                .enumerate_pixels()
                .filter(|(x, y, _)| {
                    let offset = bevy::math::DVec2::new(
                        f64::from(*x) + 0.5 - f64::from(width) / 2.,
                        f64::from(*y) + 0.5 - f64::from(height) / 2.,
                    ) - origin.as_dvec2();
                    let cell_offset =
                        offset - (offset / 64. + bevy::math::DVec2::splat(0.5)).floor() * 64.;
                    let (sin, cos) = f64::from(rotation).sin_cos();
                    let image_offset = bevy::math::DVec2::new(
                        cos * cell_offset.x + sin * cell_offset.y,
                        -sin * cell_offset.x + cos * cell_offset.y,
                    );
                    // Distinct triangles interpolate differently at an exact
                    // discontinuity. Exclude only centers within 0.0001 physical
                    // pixels of the analytic edge, far below raster subpixel size.
                    // The 0.001-origin cases still catch tight-quad edge loss.
                    let distance = image_offset.abs();
                    let edge = (distance - bevy::math::DVec2::splat(8.)).abs();
                    !((edge.x < 0.0001 && distance.y <= 8.0001)
                        || (edge.y < 0.0001 && distance.x <= 8.0001))
                })
                .flat_map(|(x, y, a)| {
                    a.0.into_iter()
                        .zip(optimized.get_pixel(x, y).0)
                        .map(|(a, b)| a.abs_diff(b))
                })
                .max()
                .unwrap();
            assert!(
                maximum <= 3,
                "{backend:?} opaque edge: {maximum}/255, rotation {rotation}, origin {origin}"
            );
            app.world_mut()
                .entity_mut(entity)
                .insert(TiledBackgroundMode::Auto);
            frame(&mut app);
            cases += 1;
        }
    }
    // Fragment coordinates include the viewport origin, while UI coordinates do not.
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .get_mut(&target)
        .unwrap()
        .resize(Extent3d {
            width: width + 34,
            height: height + 46,
            depth_or_array_layers: 1,
        });
    app.world_mut()
        .entity_mut(camera)
        .get_mut::<Camera>()
        .unwrap()
        .viewport = Some(bevy::camera::Viewport {
        physical_position: UVec2::new(17, 23),
        physical_size: UVec2::new(width, height),
        ..default()
    });
    let soft = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(texture(1));
    app.world_mut()
        .resource_mut::<Assets<TiledBackgroundMaterial>>()
        .get_mut(&source)
        .unwrap()
        .pattern_texture = soft;
    let a = capture(&mut app, &target, entity, TiledBackgroundMode::Auto, true)
        .data
        .unwrap();
    let b = capture(
        &mut app,
        &target,
        entity,
        TiledBackgroundMode::Reference,
        false,
    )
    .data
    .unwrap();
    assert!(
        a.iter().zip(&b).all(|(a, b)| a.abs_diff(*b) <= 3),
        "offset viewport changed pixels"
    );
    cases += 1;
    app.world_mut()
        .entity_mut(entity)
        .get_mut::<Node>()
        .unwrap()
        .width = Val::Percent(75.);
    let a = capture(
        &mut app,
        &target,
        entity,
        TiledBackgroundMode::Reference,
        false,
    )
    .data
    .unwrap();
    let b = capture(&mut app, &target, entity, TiledBackgroundMode::Auto, false)
        .data
        .unwrap();
    assert_eq!(a, b, "cropped-layout fallback changed pixels");
    eprintln!("TILED {backend:?}: {cases} actual instanced comparisons passed; fallback exact");
}

#[test]
#[ignore = "requires a hardware Vulkan adapter"]
fn gpu_vulkan_matches_reference() {
    check(Backends::VULKAN);
}

#[test]
#[ignore = "requires a hardware GLES adapter"]
fn gpu_gl_matches_reference() {
    check(Backends::GL);
}
