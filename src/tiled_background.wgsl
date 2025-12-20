#import bevy_ui::ui_vertex_output::UiVertexOutput
#import bevy_render::globals::Globals

@group(0) @binding(1) var<uniform> globals: Globals;

@group(1) @binding(0) var<uniform> pattern_color: vec4<f32>;
@group(1) @binding(1) var<uniform> scale: f32;
@group(1) @binding(2) var<uniform> rotation: f32;
@group(1) @binding(3) var<uniform> stagger: f32;
@group(1) @binding(4) var<uniform> spacing: f32;
@group(1) @binding(5) var<uniform> scroll_speed: vec2<f32>;
@group(1) @binding(6) var pattern_texture: texture_2d<f32>;
@group(1) @binding(7) var pattern_sampler: sampler;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    // Use absolute pixel coordinates centered on the node
    let pos = (in.uv - 0.5) * in.size;

    // Rotate the coordinate space
    let s = sin(rotation);
    let c = cos(rotation);
    var rotated_pos = vec2<f32>(
        c * pos.x - s * pos.y,
        s * pos.x + c * pos.y
    );

    // Apply scrolling (scroll_speed is in pixels per second)
    rotated_pos += scroll_speed * globals.time;

    // Calculate tile dimensions based on scale (multiplier of native texture size)
    let tex_dims = textureDimensions(pattern_texture);
    let tile_size = vec2<f32>(tex_dims) * scale;
    let tile_height = tile_size.y;
    let tile_width = tile_size.x;

    // Stagger rows for brick-like pattern
    let row = floor(rotated_pos.y / tile_height);
    rotated_pos.x += row * stagger * tile_width;

    // Local UV within the tile [0, 1]
    let local_uv = fract(rotated_pos / tile_size);

    // Spacing/Padding - creates gaps between tiles
    let margin = (1.0 - spacing) * 0.5;
    if (local_uv.x >= margin && local_uv.x <= (1.0 - margin) &&
        local_uv.y >= margin && local_uv.y <= (1.0 - margin)) {
        
        let sample_uv = (local_uv - margin) / spacing;
        let tex_color = textureSample(pattern_texture, pattern_sampler, sample_uv);
        return vec4<f32>(pattern_color.rgb, tex_color.a * pattern_color.a);
    }

    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
}
