# bevy_tiled_background

[![Crates.io](https://img.shields.io/crates/v/bevy_tiled_background.svg)](https://crates.io/crates/bevy_tiled_background)
[![Docs.rs](https://docs.rs/bevy_tiled_background/badge.svg)](https://docs.rs/bevy_tiled_background)
[![Rust](https://github.com/mirsella/bevy_tiled_background/actions/workflows/rust.yml/badge.svg)](https://github.com/mirsella/bevy_tiled_background/actions/workflows/rust.yml)
[![License](https://img.shields.io/crates/l/bevy_tiled_background.svg)](https://github.com/mirsella/bevy_tiled_background)

A small Bevy UI material plugin for repeating image patterns as full-screen or panel backgrounds.
It keeps tiles in logical UI pixels, so high-DPI displays keep the same visual density as desktop.

![Example](https://raw.githubusercontent.com/mirsella/bevy_tiled_background/main/assets/screenshot.png)

## What it does

`bevy_tiled_background` renders a texture repeatedly inside a Bevy `Node` using a custom `UiMaterial`.
Use it for animated menu backgrounds, decorative panels, loading screens, card backdrops, or any UI area that needs a tiled image pattern.

## Features

- Logical-pixel tiling that looks consistent across desktop, mobile, and high-DPI screens
- Native texture aspect ratio preservation without stretching
- Tint and opacity control through `color`
- An explicit two-axis lattice for cell size, rotation, and skew
- Image scale and rotation independent of the lattice
- Alternating checkerboard layers for patterns made from multiple images
- Optional design-reference cover scaling and cropping
- Gaps set by making lattice cells larger than their images
- Smooth scrolling animation in any direction

## Installation

```bash
cargo add bevy_tiled_background
```

Or add it manually:

```toml
[dependencies]
bevy_tiled_background = "0.6"
```

## Quick start

Add `TiledBackgroundPlugin`, create a `TiledBackgroundMaterial`, then attach it to any UI `Node` with `MaterialNode`.

```rust
use bevy::prelude::*;
use bevy_tiled_background::{TiledBackgroundMaterial, TiledBackgroundPlugin};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TiledBackgroundPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<TiledBackgroundMaterial>>,
) {
    commands.spawn(Camera2d);

    let rotation = -20f32.to_radians();
    let cell_size = Vec2::new(165.0, 147.0);
    let material = materials.add(TiledBackgroundMaterial {
        color: Color::WHITE.with_alpha(0.15).into(),
        image_scale: 0.5,
        image_rotation: rotation,
        lattice: Mat2::from_angle(rotation)
            * Mat2::from_cols(
                Vec2::new(cell_size.x, 0.0),
                Vec2::new(-cell_size.x * 0.5, cell_size.y),
            ),
        scroll_velocity: Vec2::new(30.0, 0.0),
        pattern_texture: asset_server.load("background_logo.png"),
        ..default()
    });

    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.1, 0.1, 0.15)),
        MaterialNode(material),
    ));
}
```

Run the included example with:

```bash
cargo run --example simple
```

## Material properties

| Property | Type | Description |
|----------|------|-------------|
| `color` | `LinearRgba` | Tint multiplied with the texture. Use white for no tint; alpha controls opacity. |
| `image_scale` | `f32` | Positive image size multiplier. `1.0` uses the texture's native size. |
| `image_rotation` | `f32` | Clockwise image rotation in radians, independent of the lattice. |
| `lattice` | `Mat2` | Invertible cell basis in local or reference pixels. `Mat2::ZERO` uses the scaled texture's native size. |
| `origin` | `Vec2` | Center of cell `(0, 0)`, relative to the node center or reference top-left corner. |
| `reference_size` | `Vec2` | Positive design dimensions used to cover and crop the node. `Vec2::ZERO` disables this mode. |
| `scroll_velocity` | `Vec2` | Visual pattern velocity in logical screen pixels per second. |
| `checkerboard_parity` | `i32` | `-1` draws every cell; `0` or `1` draws one checkerboard parity. |
| `pattern_texture` | `Handle<Image>` | Texture image to repeat. |
| `pixel_scale` | `f32` | Plugin-managed window scale conversion. Leave this at the default value. |

`lattice.x_axis` and `lattice.y_axis` point from the center of cell `(0, 0)` to the centers of cells `(1, 0)` and `(0, 1)`. Rotate both the lattice and `image_rotation` to rotate the whole pattern. Change only `image_rotation` to turn each image inside fixed cells.

With a positive `reference_size`, lattice and origin values use reference pixels measured from its top-left corner. The shader cover-scales that reference rectangle over the node. `scroll_velocity` remains in logical screen pixels per second.

## Bevy compatibility

| Bevy | bevy_tiled_background |
|------|-----------------------|
| 0.19 | 0.5-0.6               |
| 0.17 | 0.4                   |

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
