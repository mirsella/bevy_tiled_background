// Instanced counterpart of tiled_background.wgsl; preserves its compositing space.
#import bevy_render::{globals::Globals, view::View}
struct Pattern {
    color: vec4<f32>, image_scale: f32, image_rotation: f32,
    lattice_x_axis: vec2<f32>, lattice_y_axis: vec2<f32>,
    origin: vec2<f32>, reference_size: vec2<f32>, scroll_velocity: vec2<f32>,
    checkerboard_parity: i32, pixel_scale: f32,
}
@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> pattern: Pattern;
@group(1) @binding(1) var image: texture_2d<f32>;
@group(1) @binding(2) var image_sampler: sampler;
struct Geometry { size: vec2<f32>, center: vec2<f32>, grid: vec2<u32>, padding: vec2<u32> }
@group(1) @binding(3) var<uniform> geometry: Geometry;
struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) center: vec2<f32>,
    @location(1) @interpolate(flat) image_basis_u: vec2<f32>,
    @location(2) @interpolate(flat) image_basis_v: vec2<f32>,
    @location(3) @interpolate(flat) cell_basis_u: vec2<f32>,
    @location(4) @interpolate(flat) cell_basis_v: vec2<f32>,
}
@vertex
fn vertex(@builtin(vertex_index) vertex: u32, @builtin(instance_index) instance: u32) -> Output {
    var out: Output;
    out.position = vec4(2.0, 2.0, 0.0, 1.0);
    let corners = array<vec2<f32>, 6>(vec2(0., 0.), vec2(1., 0.), vec2(1., 1.), vec2(0., 0.), vec2(1., 1.), vec2(0., 1.));
    let size = geometry.size;
    let logical_size = size / pattern.pixel_scale;
    var cover = 1.0;
    var reference_center = vec2(0.0);
    if all(pattern.reference_size > vec2(0.0)) {
        cover = max(logical_size.x / pattern.reference_size.x, logical_size.y / pattern.reference_size.y);
        reference_center = pattern.reference_size * 0.5;
    }
    let lattice = mat2x2(pattern.lattice_x_axis, pattern.lattice_y_axis);
    let inv_det = 1.0 / determinant(lattice);
    let inverse = mat2x2(vec2(lattice[1].y, -lattice[0].y) * inv_det, vec2(-lattice[1].x, lattice[0].x) * inv_det);
    // Share the CPU's integer grid with the draw count. Recomputing ceil here
    // could round differently and omit an edge row at fractional pixel scales.
    let grid = geometry.grid;
    let radius = vec2<i32>((grid - vec2(1u)) / vec2(2u));
    let scroll = pattern.scroll_velocity * globals.time / cover;
    let anchor = vec2<i32>(floor(inverse * (reference_center - pattern.origin - scroll)));
    let cell = anchor - radius + vec2<i32>(i32(instance % grid.x), i32(instance / grid.x));
    if pattern.checkerboard_parity >= 0 && ((cell.x + cell.y) & 1) != pattern.checkerboard_parity {
        return out;
    }
    let rotation = vec2(cos(pattern.image_rotation), sin(pattern.image_rotation));
    let image_size = vec2<f32>(textureDimensions(image)) * pattern.image_scale;
    // Rasterize beyond the image edge so half-open UV clipping, rather than the
    // rasterizer's rotated top-left edge rule, decides boundary pixel coverage.
    let padding = vec2(1.0) / (image_size * cover * pattern.pixel_scale);
    let uv = corners[vertex] * (vec2(1.0) + padding * 2.0) - padding;
    let image_offset = mat2x2(vec2(rotation.x, rotation.y), vec2(-rotation.y, rotation.x)) * ((uv - 0.5) * image_size);
    let scale = cover * pattern.pixel_scale;
    out.center = geometry.center + (lattice * vec2<f32>(cell) + pattern.origin + scroll - reference_center) * scale;
    out.image_basis_u = vec2(rotation.x, -rotation.y) / (image_size * scale);
    out.image_basis_v = vec2(rotation.y, rotation.x) / (image_size * scale);
    out.cell_basis_u = inverse[0] / scale;
    out.cell_basis_v = inverse[1] / scale;
    out.position = view.clip_from_world * vec4(out.center + image_offset * scale, 1.0, 1.0);
    return out;
}
@fragment
fn fragment(in: Output) -> @location(0) vec4<f32> {
    // UI views use physical pixels relative to their viewport. Evaluate at the
    // actual sample center: interpolating UVs over small quads shifts opaque
    // edges when the rasterizer snaps their vertices to its subpixel grid.
    let point = in.position.xy - view.viewport.xy;
    let offset = point - in.center;
    let uv = mat2x2(in.image_basis_u, in.image_basis_v) * offset + 0.5;
    if any(uv < vec2(0.0)) || any(uv >= vec2(1.0)) { discard; }
    // Match the source shader's cell clipping if a logo overlaps its lattice cell.
    let cell_offset = mat2x2(in.cell_basis_u, in.cell_basis_v) * offset;
    if any(cell_offset < vec2(-0.5)) || any(cell_offset >= vec2(0.5)) { discard; }
    if any(abs(point - geometry.center) > geometry.size * 0.5) { discard; }
    let color = textureSampleGrad(image, image_sampler, uv, in.image_basis_u, in.image_basis_v) * pattern.color;
#ifdef MANUAL_SRGB
    return vec4(pow(max(color.rgb, vec3(0.0)), vec3(1.0 / 2.2)), color.a);
#else
    return color;
#endif
}
