// Add a uniform to hold the viewport size (width, height)
@group(1) @binding(0)
var<uniform> viewport_size: vec2<f32>; // Width, Height

@group(0) @binding(0)
var<uniform> rect_data: vec4<f32>; // x, y, width, height in pixels


@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    let x = rect_data.x / viewport_size.x * 2.0 - 1.0;
    // Flip the y-coordinate
    let y = 1.0 - rect_data.y / viewport_size.y * 2.0; 
    let width = rect_data.z / viewport_size.x * 2.0;
    // No need to flip the height as it's a magnitude
    let height = rect_data.w / viewport_size.y * 2.0;

    // Define the rectangle's corners in NDC
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(x, y - height), // Top left
        vec2<f32>(x + width, y - height), // Top right
        vec2<f32>(x, y), // Bottom left
        vec2<f32>(x + width, y - height), // Top right
        vec2<f32>(x + width, y), // Bottom right
        vec2<f32>(x, y), // Bottom left
    );

    let position = positions[vertex_index];
    let clip_space_pos = vec4<f32>(position.x, position.y, 0.0, 1.0);
    return clip_space_pos;
}

@group(0) @binding(1)
var<uniform> rect_color: vec4<f32>;

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    // Use the color from the uniform
    return rect_color;
}