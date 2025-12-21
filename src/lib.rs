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
//!         pattern_color: LinearRgba::WHITE,
//!         scale: 0.5,
//!         rotation: 35f32.to_radians(),
//!         stagger: 0.5,
//!         spacing: 10.0,
//!         scroll_speed: Vec2::new(20.0, 0.0),
//!         pattern_texture: asset_server.load("my_pattern.png"),
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
    asset::embedded_asset, prelude::*, render::render_resource::AsBindGroup, shader::ShaderRef,
    ui_render::ui_material::UiMaterial,
};

/// Plugin that registers the [`TiledBackgroundMaterial`] for use in UI.
pub struct TiledBackgroundPlugin;

impl Plugin for TiledBackgroundPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "tiled_background.wgsl");

        app.register_type::<TiledBackgroundMaterial>()
            .register_asset_reflect::<TiledBackgroundMaterial>()
            .add_plugins(UiMaterialPlugin::<TiledBackgroundMaterial>::default());
    }
}

/// A UI material that renders a tiled, animated pattern.
#[derive(AsBindGroup, Asset, Debug, Clone, Reflect)]
pub struct TiledBackgroundMaterial {
    /// Tint color applied to the pattern. Alpha controls opacity.
    #[uniform(0)]
    pub pattern_color: LinearRgba,
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
    /// The texture to tile.
    #[texture(1)]
    #[sampler(2)]
    pub pattern_texture: Handle<Image>,
}

impl Default for TiledBackgroundMaterial {
    fn default() -> Self {
        Self {
            pattern_color: LinearRgba::WHITE,
            scale: 1.0,
            rotation: 0.0,
            stagger: 0.0,
            spacing: 0.0,
            scroll_speed: Vec2::ZERO,
            pattern_texture: Handle::default(),
        }
    }
}

impl UiMaterial for TiledBackgroundMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://bevy_tiled_background/tiled_background.wgsl".into()
    }
}
