use bevy::prelude::*;
use bevy_tiled_background::{TiledBackgroundMaterial, TiledBackgroundPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TiledBackgroundPlugin)
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
        pattern_color: Color::WHITE.with_alpha(0.15).into(),
        scale: 0.5,
        rotation: 20f32.to_radians(),
        stagger: 0.5,
        spacing: 0.7,
        scroll_speed: Vec2::new(30.0, 0.0),
        pattern_texture: asset_server.load("background_logo.png"),
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
