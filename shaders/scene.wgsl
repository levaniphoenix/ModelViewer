struct Uniforms {
    view_projection : mat4x4<f32>,
    inv_view_projection : mat4x4<f32>,
    camera_pos : vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms : Uniforms;

struct VsOut {
    @builtin(position) position : vec4<f32>,
    @location(0) world_pos : vec3<f32>,
    @location(1) normal : vec3<f32>,
}

@vertex
fn vs_main(
    @location(0) position : vec3<f32>,
    @location(1) normal : vec3<f32>,
) -> VsOut {
    var out : VsOut;
    out.position = uniforms.view_projection * vec4(position, 1.0);
    out.world_pos = position;
    out.normal = normal;
    return out;
}

@fragment
fn fs_main(in : VsOut) -> @location(0) vec4<f32> {
    let base_color = vec3(0.8, 0.8, 0.8);
    let light_color = vec3(1.0, 1.0, 1.0);
    let light_dir = normalize(vec3(1.0, 1.0, 1.0));
    let ambient_strength = 0.15;
    let specular_strength = 0.5;
    let shininess = 32.0;

    let n = normalize(in.normal);
    let v = normalize(uniforms.camera_pos.xyz - in.world_pos);
    let h = normalize(light_dir + v);

    let diffuse = max(dot(n, light_dir), 0.0);
    let specular = pow(max(dot(n, h), 0.0), shininess) * specular_strength;

    let color = base_color * (ambient_strength + diffuse * light_color)
              + light_color * specular;
    return vec4(color, 1.0);
}
