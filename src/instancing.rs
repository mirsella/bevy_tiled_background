//! Sparse fullscreen patterns use instanced quads, with the original material as fallback.
pub(crate) mod render;

use crate::{TiledBackgroundMaterial, TiledBackgroundUniform};
use bevy::{
    asset::{AssetPath, embedded_asset, embedded_path},
    prelude::*,
    render::render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderType},
    shader::ShaderRef,
    ui::{UiGlobalTransform, UiSystems},
    ui_render::ui_material::UiMaterialKey,
    window::PrimaryWindow,
};

const INSTANCE_BUDGET: u32 = 512;

/// The default automatically selects sparse geometry where supported.
/// `Reference` keeps the original fullscreen shader for comparisons or debugging.
#[derive(Component, Default, Clone, Copy, PartialEq, Eq)]
pub enum TiledBackgroundMode {
    #[default]
    Auto,
    Reference,
}

/// Geometry selection, after UI layout and before asset extraction.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UpdateTiledBackgroundGeometry;

pub(crate) fn plugin(app: &mut App) {
    embedded_asset!(app, "instanced.wgsl");
    app.add_plugins(render::plugin).add_systems(
        PostUpdate,
        sync.in_set(UpdateTiledBackgroundGeometry)
            .after(UiSystems::PostLayout)
            .before(bevy::asset::AssetEventSystems),
    );
}

#[derive(ShaderType, Clone, Copy, PartialEq)]
struct Geometry {
    size: Vec2,
    center: Vec2,
    grid: UVec2,
    padding: UVec2,
}

#[derive(Asset, AsBindGroup, TypePath, Clone)]
struct PatternGeometryMaterial {
    owner: Entity,
    #[uniform(0)]
    pattern: TiledBackgroundUniform,
    #[texture(1)]
    #[sampler(2)]
    image: Handle<Image>,
    #[uniform(3)]
    geometry: Geometry,
}

impl UiMaterial for PatternGeometryMaterial {
    fn vertex_shader() -> ShaderRef {
        Self::fragment_shader()
    }

    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("instanced.wgsl")).with_source("embedded"),
        )
    }

    fn specialize(descriptor: &mut RenderPipelineDescriptor, _: UiMaterialKey<Self>) {
        // Quad corners come from vertex_index; the crate's draw command selects
        // the instance range. No Bevy trait extension or vertex buffer is needed.
        descriptor.vertex.buffers.clear();
    }
}

fn grid_size(pattern: &TiledBackgroundMaterial, physical_size: Vec2) -> Option<UVec2> {
    if !physical_size.is_finite()
        || physical_size.min_element() <= 0.
        || !pattern.pixel_scale.is_finite()
        || pattern.pixel_scale <= 0.
        || !pattern.image_scale.is_finite()
        || pattern.image_scale <= 0.
        || !pattern.image_rotation.is_finite()
        || !pattern.origin.is_finite()
        || !pattern.scroll_velocity.is_finite()
        || !pattern.reference_size.is_finite()
    {
        return None;
    }
    let lattice = Mat2::from_cols(pattern.lattice_x_axis, pattern.lattice_y_axis);
    if !lattice.is_finite() || lattice.determinant().abs() < 1e-6 {
        // This also retains the original shader for its dense automatic lattice.
        return None;
    }
    let logical = physical_size / pattern.pixel_scale;
    let cover = if pattern.reference_size.min_element() > 0. {
        (logical / pattern.reference_size).max_element()
    } else {
        1.
    };
    let half = logical / (2. * cover);
    let inverse = lattice.inverse();
    let radius =
        (inverse.x_axis.abs() * half.x + inverse.y_axis.abs() * half.y + Vec2::splat(2.)).ceil();
    let grid = radius * 2. + Vec2::ONE;
    let count = grid.element_product();
    (count.is_finite() && count <= INSTANCE_BUDGET as f32).then(|| grid.as_uvec2())
}

fn clip_covers_view(clip: Option<&CalculatedClip>, size: Vec2) -> bool {
    clip.is_none_or(|clip| {
        #[cfg(feature = "mightyminions")]
        {
            [
                Vec2::ZERO,
                Vec2::new(size.x, 0.),
                size,
                Vec2::new(0., size.y),
            ]
            .into_iter()
            .all(|corner| clip.contains_point(corner))
        }
        #[cfg(not(feature = "mightyminions"))]
        {
            clip.clip.min.cmple(Vec2::ZERO).all() && clip.clip.max.cmpge(size).all()
        }
    })
}

#[expect(
    clippy::type_complexity,
    reason = "query keeps source and derived material ownership together"
)]
fn sync(
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    patterns: Res<Assets<TiledBackgroundMaterial>>,
    images: Res<Assets<Image>>,
    mut materials: ResMut<Assets<PatternGeometryMaterial>>,
    nodes: Query<
        (
            Entity,
            Option<&TiledBackgroundMode>,
            &ComputedNode,
            &UiGlobalTransform,
            Option<&MaterialNode<TiledBackgroundMaterial>>,
            Option<&MaterialNode<PatternGeometryMaterial>>,
            Option<&CalculatedClip>,
        ),
        Or<(
            With<MaterialNode<TiledBackgroundMaterial>>,
            With<MaterialNode<PatternGeometryMaterial>>,
        )>,
    >,
) {
    let window_size = windows
        .single()
        .ok()
        .map(|window| window.physical_size().as_vec2());
    for (entity, mode, node, transform, source, current, clip) in &nodes {
        let pattern = source.and_then(|source| patterns.get(&source.0));
        let fullscreen = window_size.is_some_and(|size| {
            let affine = transform.affine();
            (node.size() - size).abs().max_element() < 0.01
                && (affine.translation - size * 0.5).abs().max_element() < 0.01
                && (affine.matrix2.x_axis - Vec2::X).abs().max_element() < 0.0001
                && (affine.matrix2.y_axis - Vec2::Y).abs().max_element() < 0.0001
        });
        let grid = (|| {
            if mode == Some(&TiledBackgroundMode::Reference)
                || !fullscreen
                || !clip_covers_view(clip, node.size())
            {
                return None;
            }
            let pattern = pattern?;
            let grid = grid_size(pattern, node.size())?;
            let image = images.get(&pattern.pattern_texture)?;
            let image_area = image.size().as_vec2().element_product() * pattern.image_scale.powi(2);
            let cell_area = Mat2::from_cols(pattern.lattice_x_axis, pattern.lattice_y_axis)
                .determinant()
                .abs();
            // Dense patterns already have a cheap fullscreen draw. Geometry is
            // useful when the lattice contains appreciable empty space.
            (image_area > 0. && image_area < cell_area * 0.8).then_some(grid)
        })();
        let Some(grid) = grid else {
            if current.is_some() {
                commands
                    .entity(entity)
                    .remove::<MaterialNode<PatternGeometryMaterial>>();
                debug!(?entity, "tiled background restored reference rendering");
            }
            continue;
        };
        let pattern = pattern.expect("eligible geometry requires its source material");
        let uniform = TiledBackgroundUniform::from(pattern);
        let geometry = Geometry {
            size: node.size(),
            center: transform.affine().translation,
            grid,
            padding: UVec2::ZERO,
        };
        // Entity cloning copies MaterialNode handles. Each instance needs its own
        // derived material, both for updates and to prevent UI material batching.
        let owned = current.filter(|current| {
            materials
                .get(&current.0)
                .expect("owned pattern geometry material")
                .owner
                == entity
        });
        if let Some(current) = owned {
            let mut material = materials
                .get_mut(&current.0)
                .expect("owned pattern geometry material");
            if material.pattern != uniform {
                material.pattern = uniform;
            }
            if material.geometry != geometry {
                material.geometry = geometry;
            }
            if material.image != pattern.pattern_texture {
                material.image = pattern.pattern_texture.clone();
            }
        } else {
            let material = materials.add(PatternGeometryMaterial {
                owner: entity,
                pattern: uniform,
                image: pattern.pattern_texture.clone(),
                geometry,
            });
            commands.entity(entity).insert(MaterialNode(material));
            debug!(
                ?entity,
                instances = grid.element_product(),
                "tiled background using instanced geometry"
            );
        }
    }
}

#[cfg(test)]
mod tests;
