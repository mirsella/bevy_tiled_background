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

    let rotation = -20f32.to_radians();
    let cell_size = Vec2::new(165.0, 147.0);
    let lattice = Mat2::from_angle(rotation)
        * Mat2::from_cols(
            Vec2::new(cell_size.x, 0.0),
            Vec2::new(-cell_size.x * 0.5, cell_size.y),
        );
    let material = materials.add(TiledBackgroundMaterial {
        color: Color::WHITE.with_alpha(0.15).into(),
        image_scale: 0.5,
        image_rotation: rotation,
        lattice_x_axis: lattice.x_axis,
        lattice_y_axis: lattice.y_axis,
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
