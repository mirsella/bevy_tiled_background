#import bevy_render::{globals::Globals, view::View}

struct TiledMaterial {
    color: vec4<f32>,
    scale: f32,
    rotation: f32,
    stagger: f32,
    spacing: f32,
    scroll_speed: vec2<f32>,
    pixel_scale: f32,
}

@group(0) @binding(1) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> material: TiledMaterial;
@group(1) @binding(1) var pattern_texture: texture_2d<f32>;
@group(1) @binding(2) var pattern_sampler: sampler;

struct TiledVertexOutput {
    @location(0) tile_position: vec2<f32>,
    @location(1) @interpolate(flat) image_size: vec2<f32>,
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
    let pos = (vertex_uv - 0.5) * size / material.pixel_scale;
    let rotation = vec2(cos(material.rotation), sin(material.rotation));
    out.tile_position = mat2x2(rotation, vec2(-rotation.y, rotation.x)) * pos
        + material.scroll_speed * globals.time;
    out.image_size = vec2<f32>(textureDimensions(pattern_texture)) * material.scale;
    return out;
}

@fragment
fn fragment(in: TiledVertexOutput) -> @location(0) vec4<f32> {
    var tile_position = in.tile_position;
    let cell_size = in.image_size + material.spacing;

    // Explicit gradients avoid seams and WebGPU's ban on implicit derivatives in branches.
    let uv_scale = 1.0 / in.image_size;
    let ddx = dpdx(tile_position) * uv_scale;
    let ddy = dpdy(tile_position) * uv_scale;

    let row = floor(tile_position.y / cell_size.y);
    tile_position.x += row * material.stagger * cell_size.x;

    let cell_position = tile_position - floor(tile_position / cell_size) * cell_size;

    if all(cell_position >= vec2(0.0)) && all(cell_position < in.image_size) {
        return textureSampleGrad(
            pattern_texture,
            pattern_sampler,
            cell_position / in.image_size,
            ddx,
            ddy,
        ) * material.color;
    }

    return vec4(0.0);
}
