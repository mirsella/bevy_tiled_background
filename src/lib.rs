//! A Bevy plugin for creating tiled, animated UI backgrounds.
//!
//! This crate provides a `UiMaterial` implementation that renders a repeating pattern
//! with an explicit lattice, independent image rotation, checkerboards, cover scaling, and
//! scrolling.
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
//!     let rotation = -35f32.to_radians();
//!     let cell_size = Vec2::new(165.0, 147.0);
//!     let lattice = Mat2::from_angle(rotation)
//!         * Mat2::from_cols(
//!             Vec2::new(cell_size.x, 0.0),
//!             Vec2::new(-cell_size.x * 0.5, cell_size.y),
//!         );
//!     let material = materials.add(TiledBackgroundMaterial {
//!         color: LinearRgba::WHITE,
//!         image_scale: 0.5,
//!         image_rotation: rotation,
//!         lattice_x_axis: lattice.x_axis,
//!         lattice_y_axis: lattice.y_axis,
//!         scroll_velocity: Vec2::new(20.0, 0.0),
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

#![recursion_limit = "256"]

use bevy::{
    asset::embedded_asset,
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
    ui_render::ui_material::UiMaterial,
    window::{PrimaryWindow, WindowScaleFactorChanged},
};

pub mod instancing;

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
                        .or_else(on_message::<WindowScaleFactorChanged>)
                        .or_else(
                            any_match_filter::<Changed<MaterialNode<TiledBackgroundMaterial>>>,
                        ),
                ),
            )
            .add_plugins(instancing::plugin);
    }
}

/// A UI material that renders a tiled, animated pattern.
#[derive(AsBindGroup, Asset, Debug, Clone, Reflect)]
#[uniform(0, TiledBackgroundUniform)]
pub struct TiledBackgroundMaterial {
    /// Tint color multiplied with the pattern texture. Use white for no tint.
    pub color: LinearRgba,
    /// Image size multiplier. `1.0` uses the texture's native size. Must be positive.
    pub image_scale: f32,
    /// Clockwise image rotation in radians, independent of the lattice orientation.
    pub image_rotation: f32,
    /// Vector from a cell center to its neighbor at `(1, 0)`, in local or reference pixels.
    ///
    /// When both lattice axes are [`Vec2::ZERO`], the material uses an axis-aligned lattice
    /// matching the scaled texture's native size. Otherwise, both axes must be finite and form an
    /// invertible basis.
    pub lattice_x_axis: Vec2,
    /// Vector from a cell center to its neighbor at `(0, 1)`, in local or reference pixels.
    pub lattice_y_axis: Vec2,
    /// Center of lattice cell `(0, 0)`.
    ///
    /// Coordinates are relative to the node center, or to the reference's top-left corner when
    /// [`Self::reference_size`] enables cover scaling.
    pub origin: Vec2,
    /// Design dimensions to cover and crop over the node.
    ///
    /// Set both dimensions to positive values to enable this mode, or use [`Vec2::ZERO`] for
    /// local node coordinates.
    pub reference_size: Vec2,
    /// Visual animation velocity in logical screen pixels per second.
    pub scroll_velocity: Vec2,
    /// Checkerboard cells to draw: `-1` draws all cells, `0` even cells, and `1` odd cells.
    pub checkerboard_parity: i32,
    /// Pixel scale used to convert physical render pixels to logical UI pixels.
    ///
    /// This is managed by [`TiledBackgroundPlugin`]. Leave it at `1.0` when
    /// constructing materials manually.
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
            image_scale: 1.0,
            image_rotation: 0.0,
            lattice_x_axis: Vec2::ZERO,
            lattice_y_axis: Vec2::ZERO,
            origin: Vec2::ZERO,
            reference_size: Vec2::ZERO,
            scroll_velocity: Vec2::ZERO,
            checkerboard_parity: -1,
            pixel_scale: 1.0,
            pattern_texture: Handle::default(),
        }
    }
}

/// The shader layout shared by reference, instanced, and composed backgrounds.
/// Construct it from a material so every rendering path uses the same fields.
#[derive(ShaderType, Clone, Copy, PartialEq)]
pub struct TiledBackgroundUniform {
    color: LinearRgba,
    image_scale: f32,
    image_rotation: f32,
    lattice_x_axis: Vec2,
    lattice_y_axis: Vec2,
    origin: Vec2,
    reference_size: Vec2,
    scroll_velocity: Vec2,
    checkerboard_parity: i32,
    pixel_scale: f32,
}

impl From<&TiledBackgroundMaterial> for TiledBackgroundUniform {
    fn from(pattern: &TiledBackgroundMaterial) -> Self {
        Self {
            color: pattern.color,
            image_scale: pattern.image_scale,
            image_rotation: pattern.image_rotation,
            lattice_x_axis: pattern.lattice_x_axis,
            lattice_y_axis: pattern.lattice_y_axis,
            origin: pattern.origin,
            reference_size: pattern.reference_size,
            scroll_velocity: pattern.scroll_velocity,
            checkerboard_parity: pattern.checkerboard_parity,
            pixel_scale: pattern.pixel_scale,
        }
    }
}

impl UiMaterial for TiledBackgroundMaterial {
    fn vertex_shader() -> ShaderRef {
        "embedded://bevy_tiled_background/tiled_background.wgsl".into()
    }

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

    let pixel_scale = window.scale_factor();
    if !pixel_scale.is_finite() || pixel_scale <= 0. {
        return warn_once!(
            pixel_scale,
            "tiled background scale update skipped due to invalid window scale factor"
        );
    }

    for handle in backgrounds.iter().map(|node| &node.0) {
        let Some(mut material) = materials.get_mut(handle) else {
            warn_once!("tiled background material asset is missing");
            continue;
        };

        if material.pixel_scale != pixel_scale {
            material.pixel_scale = pixel_scale;
        }
    }
}
