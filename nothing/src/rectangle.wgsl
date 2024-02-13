// Updated WGSL shader code
@group(0) @binding(0)
var<uniform> rect_data: vec4<f32>; // Assuming x, y, width, height format

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    let x = rect_data.x;
    let y = rect_data.y;
    let width = rect_data.z;
    let height = rect_data.w;

    // Define the rectangle's corners based on the passed data
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(x, y), // Bottom left
        vec2<f32>(x + width, y), // Bottom right
        vec2<f32>(x, y + height), // Top left
        vec2<f32>(x + width, y), // Bottom right
        vec2<f32>(x + width, y + height), // Top right
        vec2<f32>(x, y + height), // Top left
    );

    let position = positions[vertex_index];
    // Convert from (x, y, width, height) to clip space coordinates if necessary
    // This step depends on your coordinate system and may require adjusting the positions
    let clip_space_pos = vec4<f32>(position, 0.0, 1.0);
    return clip_space_pos;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    // Set the fragment color
    return vec4<f32>(1.0, 0.0, 0.0, 1.0); // Example: Red color
}
