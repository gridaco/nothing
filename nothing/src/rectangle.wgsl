@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), // Bottom left
        vec2<f32>(1.0, -1.0),  // Bottom right
        vec2<f32>(-1.0, 1.0),  // Top left
        vec2<f32>(1.0, -1.0),  // Bottom right
        vec2<f32>(1.0, 1.0),   // Top right
        vec2<f32>(-1.0, 1.0)   // Top left
    );
    let position = positions[vertex_index];
    return vec4<f32>(position, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    // RGBA: Red color, fully opaque
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
