struct Uniforms{
    view_projection : mat4x4<f32>
}

@group(0) @binding(0)
var<uniform> uniforms : Uniforms;

struct VsOut {
    @builtin(position) position : vec4<f32>,
    @location(0) color : vec3<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) id: u32) -> VsOut {
    // Hardcoded unit cube centered at origin, 36 vertices (6 faces × 2 triangles)
    var positions = array<vec3<f32>, 36>(
        // Front face (z = 0.5)
        vec3(-0.5, -0.5,  0.5), vec3( 0.5, -0.5,  0.5), vec3( 0.5,  0.5,  0.5),
        vec3(-0.5, -0.5,  0.5), vec3( 0.5,  0.5,  0.5), vec3(-0.5,  0.5,  0.5),
        // Back face (z = -0.5)
        vec3( 0.5, -0.5, -0.5), vec3(-0.5, -0.5, -0.5), vec3(-0.5,  0.5, -0.5),
        vec3( 0.5, -0.5, -0.5), vec3(-0.5,  0.5, -0.5), vec3( 0.5,  0.5, -0.5),
        // Top face (y = 0.5)
        vec3(-0.5,  0.5,  0.5), vec3( 0.5,  0.5,  0.5), vec3( 0.5,  0.5, -0.5),
        vec3(-0.5,  0.5,  0.5), vec3( 0.5,  0.5, -0.5), vec3(-0.5,  0.5, -0.5),
        // Bottom face (y = -0.5)
        vec3(-0.5, -0.5, -0.5), vec3( 0.5, -0.5, -0.5), vec3( 0.5, -0.5,  0.5),
        vec3(-0.5, -0.5, -0.5), vec3( 0.5, -0.5,  0.5), vec3(-0.5, -0.5,  0.5),
        // Right face (x = 0.5)
        vec3( 0.5, -0.5,  0.5), vec3( 0.5, -0.5, -0.5), vec3( 0.5,  0.5, -0.5),
        vec3( 0.5, -0.5,  0.5), vec3( 0.5,  0.5, -0.5), vec3( 0.5,  0.5,  0.5),
        // Left face (x = -0.5)
        vec3(-0.5, -0.5, -0.5), vec3(-0.5, -0.5,  0.5), vec3(-0.5,  0.5,  0.5),
        vec3(-0.5, -0.5, -0.5), vec3(-0.5,  0.5,  0.5), vec3(-0.5,  0.5, -0.5),
    );

    // One color per face so you can tell them apart
    var face_colors = array<vec3<f32>, 6>(
        vec3(1.0, 0.0, 0.0), // front  - red
        vec3(0.0, 1.0, 0.0), // back   - green
        vec3(0.0, 0.0, 1.0), // top    - blue
        vec3(1.0, 1.0, 0.0), // bottom - yellow
        vec3(1.0, 0.0, 1.0), // right  - magenta
        vec3(0.0, 1.0, 1.0), // left   - cyan
    );

    var out : VsOut;
    out.position = uniforms.view_projection * vec4(positions[id], 1.0);
    out.color = face_colors[id / 6u];
    return out;
}

@fragment
fn fs_main(@location(0) color : vec3<f32>) -> @location(0) vec4<f32> {
    return vec4(color, 1.0);
}
