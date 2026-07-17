struct Uniforms {
    proj: mat4x4<f32>,
    resolution: vec2<f32>,
    zoom: f32,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> @builtin(position) vec4<f32> {
    var p = vec2<f32>(0.0, 0.0);

    switch in_vertex_index {
        case 0u: { p = vec2<f32>(-1.0, -1.0); }
        case 1u: { p = vec2<f32>(1.0, -1.0); }
        case 2u: { p = vec2<f32>(-1.0, 1.0); }
        case 3u: { p = vec2<f32>(1.0, -1.0); }
        case 4u: { p = vec2<f32>(1.0, 1.0); }
        case 5u: { p = vec2<f32>(-1.0, 1.0); }
        default: { p = vec2<f32>(0.0, 0.0); }
    }

    return vec4<f32>(p, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    let grid_size = 64.0 * uniforms.zoom;
    let line_width = 0.5;

    let coord = frag_coord.xy;
    let grid_x = coord.x % grid_size;
    let grid_y = coord.y % grid_size;

    let is_x_line = grid_x < line_width || grid_x > (grid_size - line_width);
    let is_y_line = grid_y < line_width || grid_y > (grid_size - line_width);

    if is_x_line || is_y_line {
        return vec4<f32>(0.032, 0.032, 0.051, 1.0);
    }

    return vec4<f32>(0.010, 0.010, 0.014, 1.0);
}