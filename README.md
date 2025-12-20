# bevy_tiled_background

[![Crates.io](https://img.shields.io/crates/v/bevy_tiled_background.svg)](https://crates.io/crates/bevy_tiled_background)
[![Docs.rs](https://docs.rs/bevy_tiled_background/badge.svg)](https://docs.rs/bevy_tiled_background)
[![License](https://img.shields.io/crates/l/bevy_tiled_background.svg)](https://github.com/mirsella/bevy_tiled_background)

A Bevy plugin for creating tiled, animated UI backgrounds with support for rotation, staggering, spacing, and scrolling animation.

![Example](https://raw.githubusercontent.com/mirsella/bevy_tiled_background/main/assets/screenshot.png)

## Features

- **Responsive tiling** - Tiles maintain constant pixel size; more tiles appear on larger screens
- **Aspect ratio preservation** - Images never stretch
- **Rotation** - Rotate the entire pattern
- **Row staggering** - Create brick-like offset patterns
- **Spacing** - Add gaps between tiles
- **Scrolling animation** - Smooth horizontal/vertical movement

## Installation

```bash
cargo add bevy_tiled_background
```

Or add to your `Cargo.toml`:

```toml
[dependencies]
bevy_tiled_background = "0.1"
```

## Usage

```rust
use bevy::prelude::*;
use bevy_tiled_background::{TiledBackgroundPlugin, TiledBackgroundMaterial};

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
        pattern_color: LinearRgba::WHITE,
        scale: 0.5,
        rotation: 35f32.to_radians(),
        stagger: 0.5,
        spacing: 0.8,
        scroll_speed: Vec2::new(20.0, 0.0),
        pattern_texture: asset_server.load("my_pattern.png"),
    });

    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        MaterialNode(material),
    ));
}
```

## Material Properties

| Property | Type | Description |
|----------|------|-------------|
| `pattern_color` | `LinearRgba` | Tint color (alpha controls opacity) |
| `scale` | `f32` | Size multiplier (1.0 = native texture size) |
| `rotation` | `f32` | Rotation angle in radians |
| `stagger` | `f32` | Row offset (0.5 = half-tile shift for brick pattern) |
| `spacing` | `f32` | How much of each tile the image fills (0.0-1.0) |
| `scroll_speed` | `Vec2` | Animation speed in pixels per second |
| `pattern_texture` | `Handle<Image>` | The texture to tile |

## Bevy Compatibility

| bevy | bevy_tiled_background |
|------|----------------------|
| 0.16 | 0.1                  |

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
