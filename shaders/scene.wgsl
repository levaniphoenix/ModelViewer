struct Uniforms {
    view_projection : mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms : Uniforms;

struct VsOut {
    @builtin(position) position : vec4<f32>,
    @location(0) normal : vec3<f32>,
}

@vertex
fn vs_main(
    @location(0) position : vec3<f32>,
    @location(1) normal : vec3<f32>,
) -> VsOut {
    var out : VsOut;
    out.position = uniforms.view_projection * vec4(position, 1.0);
    out.normal = normal;
    return out;
}

@fragment
fn fs_main(@location(0) normal : vec3<f32>) -> @location(0) vec4<f32> {
    let n = normalize(normal);
    let light_dir = normalize(vec3(1.0, 1.0, 1.0));
    let ndotl = max(dot(n, light_dir), 0.0);
    let ambient = 0.15;
    let color = vec3(0.8, 0.8, 0.8) * (ambient + ndotl * (1.0 - ambient));
    return vec4(color, 1.0);
}
