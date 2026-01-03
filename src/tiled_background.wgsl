#import bevy_ui::ui_vertex_output::UiVertexOutput
#import bevy_render::globals::Globals

struct TiledMaterial {
    pattern_color: vec4<f32>,
    scale: f32,
    rotation: f32,
    stagger: f32,
    spacing: f32,
    scroll_speed: vec2<f32>,
}

@group(0) @binding(1) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> material: TiledMaterial;
@group(1) @binding(1) var pattern_texture: texture_2d<f32>;
@group(1) @binding(2) var pattern_sampler: sampler;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    // Use absolute pixel coordinates centered on the node
    let pos = (in.uv - 0.5) * in.size;

    // Rotate the coordinate space
    let s = sin(material.rotation);
    let c = cos(material.rotation);
    var rotated_pos = vec2<f32>(
        c * pos.x - s * pos.y,
        s * pos.x + c * pos.y
    );

    // Apply scrolling (scroll_speed is in pixels per second)
    rotated_pos += material.scroll_speed * globals.time;

    // Calculate tile dimensions based on scale (multiplier of native texture size)
    let tex_dims = textureDimensions(pattern_texture);
    let image_size = vec2<f32>(tex_dims) * material.scale;
    
    // Calculate gradients based on the continuous coordinate space to avoid seams at tile boundaries
    // and to support WebGPU which forbids implicit derivatives in non-uniform control flow.
    let uv_scale = 1.0 / image_size;
    let ddx = dpdx(rotated_pos) * uv_scale;
    let ddy = dpdy(rotated_pos) * uv_scale;

    // Cell size = image size + spacing (spacing is the gap in pixels between images)
    let cell_size = image_size + material.spacing;

    // Stagger rows for brick-like pattern
    let row = floor(rotated_pos.y / cell_size.y);
    rotated_pos.x += row * material.stagger * cell_size.x;

    // Position within the cell
    let cell_pos = rotated_pos - floor(rotated_pos / cell_size) * cell_size;

    // Check if we're within the image portion of the cell (not in the spacing gap)
    if (cell_pos.x >= 0.0 && cell_pos.x < image_size.x &&
        cell_pos.y >= 0.0 && cell_pos.y < image_size.y) {
        
        let sample_uv = cell_pos / image_size;
        let tex_color = textureSampleGrad(pattern_texture, pattern_sampler, sample_uv, ddx, ddy);
        return vec4<f32>(material.pattern_color.rgb, tex_color.a * material.pattern_color.a);
    }

    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
}
