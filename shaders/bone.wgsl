struct Uniforms {
    view_projection : mat4x4<f32>,
    inv_view_projection : mat4x4<f32>,
    camera_pos : vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms : Uniforms;

struct VsOut {
    @builtin(position) position : vec4<f32>,
    @location(0) color : vec3<f32>,
}

@vertex
fn vs_main(
    @location(0) position : vec3<f32>,
    @location(1) color : vec3<f32>,
) -> VsOut {
    var out : VsOut;
    out.position = uniforms.view_projection * vec4(position, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_main(in : VsOut) -> @location(0) vec4<f32> {
    return vec4(in.color, 1.0);
}
