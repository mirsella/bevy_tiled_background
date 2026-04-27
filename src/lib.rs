//! A Bevy plugin for creating tiled, animated UI backgrounds.
//!
//! This crate provides a `UiMaterial` implementation that renders a repeating pattern
//! with support for rotation, staggering, spacing, and scrolling animation.
//!
//! # Example
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_tiled_background::{TiledBackgroundPlugin, TiledBackgroundMaterial};
//!
//! fn main() {
//!     App::new()
//!         .add_plugins((DefaultPlugins, TiledBackgroundPlugin))
//!         .add_systems(Startup, setup)
//!         .run();
//! }
//!
//! fn setup(
//!     mut commands: Commands,
//!     asset_server: Res<AssetServer>,
//!     mut materials: ResMut<Assets<TiledBackgroundMaterial>>,
//! ) {
//!     commands.spawn(Camera2d);
//!
//!     let material = materials.add(TiledBackgroundMaterial {
//!         color: LinearRgba::WHITE,
//!         scale: 0.5,
//!         rotation: 35f32.to_radians(),
//!         stagger: 0.5,
//!         spacing: 10.0,
//!         scroll_speed: Vec2::new(20.0, 0.0),
//!         pattern_texture: asset_server.load("my_pattern.png"),
//!         ..default()
//!     });
//!
//!     commands.spawn((
//!         Node {
//!             width: Val::Percent(100.0),
//!             height: Val::Percent(100.0),
//!             ..default()
//!         },
//!         MaterialNode(material),
//!     ));
//! }
//! ```

use bevy::{
    asset::embedded_asset,
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    ui_render::ui_material::UiMaterial,
    window::{PrimaryWindow, WindowScaleFactorChanged},
};

/// Plugin that registers the [`TiledBackgroundMaterial`] for use in UI.
pub struct TiledBackgroundPlugin;

impl Plugin for TiledBackgroundPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "tiled_background.wgsl");

        app.register_type::<TiledBackgroundMaterial>()
            .register_asset_reflect::<TiledBackgroundMaterial>()
            .add_plugins(UiMaterialPlugin::<TiledBackgroundMaterial>::default())
            .add_systems(
                Update,
                update_tiled_background_pixel_scale.run_if(
                    run_once
                        .or(on_message::<WindowScaleFactorChanged>)
                        .or(any_match_filter::<Changed<MaterialNode<TiledBackgroundMaterial>>>),
                ),
            );
    }
}

/// A UI material that renders a tiled, animated pattern.
#[derive(AsBindGroup, Asset, Debug, Clone, Reflect)]
pub struct TiledBackgroundMaterial {
    /// Tint color multiplied with the pattern texture. Use white for no tint.
    #[uniform(0)]
    pub color: LinearRgba,
    /// Size multiplier for tiles. `1.0` = native texture size.
    #[uniform(0)]
    pub scale: f32,
    /// Rotation angle in radians.
    #[uniform(0)]
    pub rotation: f32,
    /// Row offset for brick-like patterns. `0.5` = half-tile shift.
    #[uniform(0)]
    pub stagger: f32,
    /// Gap between images in pixels. `0.0` = no gaps.
    #[uniform(0)]
    pub spacing: f32,
    /// Animation speed in pixels per second.
    #[uniform(0)]
    pub scroll_speed: Vec2,
    /// Pixel scale used to convert physical render pixels to logical UI pixels.
    ///
    /// This is managed by [`TiledBackgroundPlugin`]. Leave it at `1.0` when
    /// constructing materials manually.
    #[uniform(0)]
    pub pixel_scale: f32,
    /// The texture to tile.
    #[texture(1)]
    #[sampler(2)]
    pub pattern_texture: Handle<Image>,
}

impl Default for TiledBackgroundMaterial {
    fn default() -> Self {
        Self {
            color: LinearRgba::WHITE,
            scale: 1.0,
            rotation: 0.0,
            stagger: 0.0,
            spacing: 0.0,
            scroll_speed: Vec2::ZERO,
            pixel_scale: 1.0,
            pattern_texture: Handle::default(),
        }
    }
}

impl UiMaterial for TiledBackgroundMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://bevy_tiled_background/tiled_background.wgsl".into()
    }
}

fn update_tiled_background_pixel_scale(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut materials: ResMut<Assets<TiledBackgroundMaterial>>,
    backgrounds: Query<&MaterialNode<TiledBackgroundMaterial>>,
) {
    if backgrounds.is_empty() {
        return;
    }

    let Ok(window) = windows.single() else {
        return warn_once!(
            "tiled background scale update skipped because the primary window is missing"
        );
    };

    let pixel_scale = window.scale_factor() as f32;
    if !pixel_scale.is_finite() || pixel_scale <= 0. {
        return warn_once!(
            pixel_scale,
            "tiled background scale update skipped due to invalid window scale factor"
        );
    }

    for material_node in &backgrounds {
        let Some(material) = materials.get_mut(&material_node.0) else {
            warn_once!("tiled background material asset is missing");
            continue;
        };

        material.pixel_scale = pixel_scale;
    }
}
