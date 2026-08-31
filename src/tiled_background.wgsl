#import bevy_render::{globals::Globals, view::View}

struct TiledMaterial {
    color: vec4<f32>,
    image_scale: f32,
    image_rotation: f32,
    lattice_x_axis: vec2<f32>,
    lattice_y_axis: vec2<f32>,
    origin: vec2<f32>,
    reference_size: vec2<f32>,
    scroll_velocity: vec2<f32>,
    checkerboard_parity: i32,
    pixel_scale: f32,
}

@group(0) @binding(1) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> material: TiledMaterial;
@group(1) @binding(1) var pattern_texture: texture_2d<f32>;
@group(1) @binding(2) var pattern_sampler: sampler;

struct TiledVertexOutput {
    @location(0) lattice_position: vec2<f32>,
    @location(1) @interpolate(flat) image_basis_u: vec2<f32>,
    @location(2) @interpolate(flat) image_basis_v: vec2<f32>,
    @builtin(position) clip_position: vec4<f32>,
}

@group(0) @binding(0) var<uniform> view: View;

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>,
    @location(2) size: vec2<f32>,
) -> TiledVertexOutput {
    var out: TiledVertexOutput;
    out.clip_position = view.clip_from_world * vec4<f32>(vertex_position, 1.0);

    // UI vertex sizes are physical render pixels; material values use logical pixels.
    let logical_size = size / material.pixel_scale;
    var pattern_position = (vertex_uv - 0.5) * logical_size;
    var cover_scale = 1.0;
    if all(material.reference_size > vec2(0.0)) {
        cover_scale = max(
            logical_size.x / material.reference_size.x,
            logical_size.y / material.reference_size.y,
        );
        pattern_position = pattern_position / cover_scale + material.reference_size * 0.5;
    }

    let image_size = vec2<f32>(textureDimensions(pattern_texture)) * material.image_scale;
    var lattice = mat2x2(material.lattice_x_axis, material.lattice_y_axis);
    if all(lattice[0] == vec2(0.0)) && all(lattice[1] == vec2(0.0)) {
        lattice = mat2x2(
            vec2(image_size.x, 0.0),
            vec2(0.0, image_size.y),
        );
    }
    let offset = pattern_position
        - material.origin
        - material.scroll_velocity * globals.time / cover_scale;
    out.lattice_position = vec2(
        dot(vec2(lattice[1].y, -lattice[1].x), offset),
        dot(vec2(-lattice[0].y, lattice[0].x), offset),
    ) / determinant(lattice) + 0.5;

    let rotation = vec2(cos(material.image_rotation), sin(material.image_rotation));
    let image_from_pattern = mat2x2(
        vec2(rotation.x, -rotation.y) / image_size,
        vec2(rotation.y, rotation.x) / image_size,
    );
    out.image_basis_u = image_from_pattern * lattice[0];
    out.image_basis_v = image_from_pattern * lattice[1];
    return out;
}

fn manual_srgb(color: vec4<f32>) -> vec4<f32> {
#ifdef MANUAL_SRGB
    return vec4(pow(max(color.rgb, vec3(0.0)), vec3(1.0 / 2.2)), color.a);
#else
    return color;
#endif
}

@fragment
fn fragment(in: TiledVertexOutput) -> @location(0) vec4<f32> {
    let cell = floor(in.lattice_position);
    let image_from_lattice = mat2x2(in.image_basis_u, in.image_basis_v);
    let image_position = image_from_lattice * (in.lattice_position - cell - 0.5) + 0.5;

    // Derive gradients from the continuous lattice coordinates, not the wrapped image position.
    let ddx = image_from_lattice * dpdx(in.lattice_position);
    let ddy = image_from_lattice * dpdy(in.lattice_position);

    if material.checkerboard_parity >= 0
        && ((i32(cell.x) + i32(cell.y)) & 1) != material.checkerboard_parity
    {
        return vec4(0.0);
    }

    if all(image_position >= vec2(0.0)) && all(image_position < vec2(1.0)) {
        return manual_srgb(
            textureSampleGrad(
                pattern_texture,
                pattern_sampler,
                image_position,
                ddx,
                ddy,
            ) * material.color,
        );
    }

    return vec4(0.0);
}
