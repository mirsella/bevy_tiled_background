use super::*;
use bevy::window::WindowResolution;

mod gpu;

fn pattern() -> TiledBackgroundMaterial {
    TiledBackgroundMaterial {
        lattice_x_axis: Vec2::new(197.19499, -99.99),
        lattice_y_axis: Vec2::new(99.485, 196.85),
        reference_size: Vec2::new(512., 1024.),
        pixel_scale: 3.,
        ..default()
    }
}

fn scene() -> (App, Entity, Handle<TiledBackgroundMaterial>) {
    let mut app = App::new();
    app.init_resource::<Assets<TiledBackgroundMaterial>>()
        .init_resource::<Assets<PatternGeometryMaterial>>()
        .init_resource::<Assets<Image>>()
        .add_systems(Update, sync);
    let image = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default());
    let source = app
        .world_mut()
        .resource_mut::<Assets<TiledBackgroundMaterial>>()
        .add(TiledBackgroundMaterial {
            pattern_texture: image,
            ..pattern()
        });
    let size = Vec2::new(1080., 2400.);
    app.world_mut().spawn((
        Window {
            resolution: WindowResolution::new(1080, 2400),
            ..default()
        },
        PrimaryWindow,
    ));
    let entity = app
        .world_mut()
        .spawn((
            ComputedNode { size, ..default() },
            UiGlobalTransform::from_translation(size * 0.5),
            MaterialNode(source.clone()),
        ))
        .id();
    (app, entity, source)
}

#[test]
fn automatic_geometry_keeps_source_component_and_tracks_replacement() {
    let (mut app, entity, source) = scene();
    app.update();
    let geometry = app
        .world()
        .entity(entity)
        .get::<MaterialNode<PatternGeometryMaterial>>()
        .unwrap()
        .0
        .clone();
    assert_eq!(
        app.world()
            .entity(entity)
            .get::<MaterialNode<TiledBackgroundMaterial>>()
            .unwrap()
            .0,
        source
    );
    let old = app
        .world()
        .resource::<Assets<PatternGeometryMaterial>>()
        .get(&geometry)
        .unwrap();
    assert_eq!(
        old.geometry.grid.element_product(),
        grid_size(&pattern(), Vec2::new(1080., 2400.))
            .unwrap()
            .element_product()
    );
    assert!(old.geometry.grid.element_product() < INSTANCE_BUDGET);
    let mut replacement = app
        .world()
        .resource::<Assets<TiledBackgroundMaterial>>()
        .get(&source)
        .unwrap()
        .clone();
    replacement.color = LinearRgba::BLACK;
    let new = app
        .world_mut()
        .resource_mut::<Assets<TiledBackgroundMaterial>>()
        .add(replacement);
    app.world_mut()
        .entity_mut(entity)
        .insert(MaterialNode(new.clone()));
    app.update();
    let material = app
        .world()
        .resource::<Assets<PatternGeometryMaterial>>()
        .get(&geometry)
        .unwrap();
    assert_eq!(material.pattern.color, LinearRgba::BLACK);
    assert_eq!(
        app.world()
            .entity(entity)
            .get::<MaterialNode<TiledBackgroundMaterial>>()
            .unwrap()
            .0,
        new
    );
}

#[test]
fn cloned_nodes_keep_independent_derived_materials() {
    let (mut app, entity, source) = scene();
    app.update();
    let clone = app.world_mut().entity_mut(entity).clone_and_spawn();
    let mut replacement = app
        .world()
        .resource::<Assets<TiledBackgroundMaterial>>()
        .get(&source)
        .unwrap()
        .clone();
    replacement.color = LinearRgba::BLACK;
    let replacement = app
        .world_mut()
        .resource_mut::<Assets<TiledBackgroundMaterial>>()
        .add(replacement);
    app.world_mut()
        .entity_mut(clone)
        .insert(MaterialNode(replacement));
    app.update();
    let a = &app
        .world()
        .entity(entity)
        .get::<MaterialNode<PatternGeometryMaterial>>()
        .unwrap()
        .0;
    let b = &app
        .world()
        .entity(clone)
        .get::<MaterialNode<PatternGeometryMaterial>>()
        .unwrap()
        .0;
    assert_ne!(a, b);
    let materials = app.world().resource::<Assets<PatternGeometryMaterial>>();
    assert_eq!(materials.get(a).unwrap().pattern.color, LinearRgba::WHITE);
    assert_eq!(materials.get(b).unwrap().pattern.color, LinearRgba::BLACK);
}

#[test]
fn reference_mode_and_source_removal_stop_optimized_drawing() {
    let (mut app, entity, source) = scene();
    app.update();
    app.world_mut()
        .entity_mut(entity)
        .insert(TiledBackgroundMode::Reference);
    app.update();
    assert!(
        !app.world()
            .entity(entity)
            .contains::<MaterialNode<PatternGeometryMaterial>>()
    );
    assert_eq!(
        app.world()
            .entity(entity)
            .get::<MaterialNode<TiledBackgroundMaterial>>()
            .unwrap()
            .0,
        source
    );
    app.world_mut()
        .entity_mut(entity)
        .remove::<TiledBackgroundMode>();
    app.update();
    assert!(
        app.world()
            .entity(entity)
            .contains::<MaterialNode<PatternGeometryMaterial>>()
    );
    app.world_mut()
        .entity_mut(entity)
        .remove::<MaterialNode<TiledBackgroundMaterial>>();
    app.update();
    assert!(
        !app.world()
            .entity(entity)
            .contains::<MaterialNode<PatternGeometryMaterial>>()
    );
}

#[test]
fn missing_source_and_unsupported_layout_restore_reference() {
    let (mut app, entity, source) = scene();
    app.update();
    app.world_mut()
        .entity_mut(entity)
        .get_mut::<ComputedNode>()
        .unwrap()
        .size
        .x *= 0.5;
    app.update();
    assert!(
        !app.world()
            .entity(entity)
            .contains::<MaterialNode<PatternGeometryMaterial>>()
    );
    app.world_mut()
        .entity_mut(entity)
        .get_mut::<ComputedNode>()
        .unwrap()
        .size
        .x *= 2.;
    app.update();
    assert!(
        app.world()
            .entity(entity)
            .contains::<MaterialNode<PatternGeometryMaterial>>()
    );
    app.world_mut()
        .resource_mut::<Assets<TiledBackgroundMaterial>>()
        .remove(source.id());
    app.update();
    assert!(
        !app.world()
            .entity(entity)
            .contains::<MaterialNode<PatternGeometryMaterial>>()
    );
    assert!(
        app.world()
            .entity(entity)
            .contains::<MaterialNode<TiledBackgroundMaterial>>()
    );
}

#[test]
fn grid_supports_reference_and_local_coordinates_but_bounds_work() {
    for reference_size in [Vec2::ZERO, Vec2::new(512., 1024.)] {
        let pattern = TiledBackgroundMaterial {
            reference_size,
            ..pattern()
        };
        for size in [
            Vec2::new(1080., 2160.),
            Vec2::new(2160., 1080.),
            Vec2::splat(2048.),
        ] {
            assert!(grid_size(&pattern, size).is_some_and(
                |grid| grid.element_product() > 0 && grid.element_product() < INSTANCE_BUDGET
            ));
        }
    }
    assert!(grid_size(&pattern(), Vec2::ZERO).is_none());
    assert!(grid_size(&TiledBackgroundMaterial::default(), Vec2::splat(100.)).is_none());
    assert!(
        grid_size(
            &TiledBackgroundMaterial {
                lattice_x_axis: Vec2::X,
                lattice_y_axis: Vec2::Y,
                reference_size: Vec2::ZERO,
                ..pattern()
            },
            Vec2::splat(4096.)
        )
        .is_none()
    );
}

#[test]
fn clipping_rejects_actual_crops_but_not_full_view() {
    let size = Vec2::new(1080., 2400.);
    #[cfg(feature = "mightyminions")]
    let make = |rect| CalculatedClip::default().with_rect(rect, &UiGlobalTransform::default());
    #[cfg(not(feature = "mightyminions"))]
    let make = |rect| CalculatedClip { clip: rect };
    assert!(clip_covers_view(None, size));
    assert!(clip_covers_view(
        Some(&make(Rect::from_corners(Vec2::ZERO, size))),
        size
    ));
    assert!(!clip_covers_view(
        Some(&make(Rect::from_corners(Vec2::ONE, size))),
        size
    ));
    #[cfg(feature = "mightyminions")]
    {
        assert!(clip_covers_view(Some(&CalculatedClip::default()), size));
        assert!(!clip_covers_view(Some(&CalculatedClip::FullyClipped), size));
    }
}
