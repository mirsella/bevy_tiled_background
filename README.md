# bevy_tiled_background

[![Crates.io](https://img.shields.io/crates/v/bevy_tiled_background.svg)](https://crates.io/crates/bevy_tiled_background)
[![Docs.rs](https://docs.rs/bevy_tiled_background/badge.svg)](https://docs.rs/bevy_tiled_background)
[![Rust](https://github.com/mirsella/bevy_tiled_background/actions/workflows/rust.yml/badge.svg)](https://github.com/mirsella/bevy_tiled_background/actions/workflows/rust.yml)
[![License](https://img.shields.io/crates/l/bevy_tiled_background.svg)](https://github.com/mirsella/bevy_tiled_background)

A small Bevy UI material plugin for repeating image patterns as full-screen or panel backgrounds.
It keeps tiles in logical UI pixels, so high-DPI displays keep the same visual density as desktop.

![Example](https://raw.githubusercontent.com/mirsella/bevy_tiled_background/main/assets/screenshot.png)

## What It Does

`bevy_tiled_background` renders a texture repeatedly inside a Bevy `Node` using a custom `UiMaterial`.
Use it for animated menu backgrounds, decorative panels, loading screens, card backdrops, or any UI area that needs a tiled image pattern.

## Features

- Logical-pixel tiling that looks consistent across desktop, mobile, and high-DPI screens
- Native texture aspect ratio preservation without stretching
- Tint and opacity control through `color`
- Rotation for diagonal or stylized patterns
- Row staggering for brick-like layouts
- Pixel spacing between repeated images
- Smooth scrolling animation in any direction

## Installation

```bash
cargo add bevy_tiled_background
```

Or add it manually:

```toml
[dependencies]
bevy_tiled_background = "0.4"
```

## Quick Start

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

    let material = materials.add(TiledBackgroundMaterial {
        color: Color::WHITE.with_alpha(0.15).into(),
        scale: 0.5,
        rotation: 20f32.to_radians(),
        stagger: 0.5,
        spacing: 40.0,
        scroll_speed: Vec2::new(30.0, 0.0),
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

## Material Properties

| Property | Type | Description |
|----------|------|-------------|
| `color` | `LinearRgba` | Tint multiplied with the texture. Use white for no tint; alpha controls opacity. |
| `scale` | `f32` | Tile image size multiplier. `1.0` uses the texture's native size. |
| `rotation` | `f32` | Pattern rotation in radians. |
| `stagger` | `f32` | Horizontal row offset as a fraction of the tile width. `0.5` creates a half-tile brick pattern. |
| `spacing` | `f32` | Gap between repeated images in logical pixels. |
| `scroll_speed` | `Vec2` | Pattern animation speed in logical pixels per second. |
| `pattern_texture` | `Handle<Image>` | Texture image to repeat. |
| `pixel_scale` | `f32` | Plugin-managed window scale conversion. Leave this at the default value. |

## Bevy Compatibility

| Bevy | bevy_tiled_background |
|------|-----------------------|
| 0.17 | 0.4                   |

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
