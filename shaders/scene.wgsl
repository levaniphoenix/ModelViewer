struct Uniforms{
    view_projection : mat4x4<f32>
}

@group(0) @binding(0)
var<uniform> uniforms : Uniforms;

@vertex
fn vs_main(@builtin(vertex_index) id: u32) -> @builtin(position) vec4<f32> {
    // Hardcoded fullscreen triangle, no vertex buffer needed
    var pos = array<vec2<f32>, 3>(
        vec2(-1.0, -1.0),
        vec2( 3.0, -1.0),
        vec2(-1.0,  3.0),
    );
    return vec4(pos[id], 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4(1.0, 0.0, 0.0, 1.0); // red
}