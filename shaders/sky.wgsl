struct Uniforms {
    view_projection : mat4x4<f32>,
    inv_view_projection : mat4x4<f32>,
    camera_pos : vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms : Uniforms;

struct VsOut {
    @builtin(position) position : vec4<f32>,
    @location(0) ndc : vec2<f32>,
}

// Fullscreen triangle — no vertex buffer needed.
@vertex
fn vs_main(@builtin(vertex_index) vi : u32) -> VsOut {
    let x = f32((vi << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(vi & 2u) * 2.0 - 1.0;
    var out : VsOut;
    out.position = vec4(x, y, 1.0, 1.0);
    out.ndc = vec2(x, y);
    return out;
}

@fragment
fn fs_main(in : VsOut) -> @location(0) vec4<f32> {
    let near_h = uniforms.inv_view_projection * vec4(in.ndc, 0.0, 1.0);
    let far_h  = uniforms.inv_view_projection * vec4(in.ndc, 1.0, 1.0);
    let near_p = near_h.xyz / near_h.w;
    let far_p  = far_h.xyz / far_h.w;
    let dir = normalize(far_p - near_p);

    let zenith    = vec3(0.30, 0.50, 0.80);
    let horizon   = vec3(0.75, 0.80, 0.85);
    let ground    = vec3(0.20, 0.18, 0.16);

    let up = smoothstep(0.0, 0.6, dir.y);
    let down = smoothstep(0.0, 0.3, -dir.y);
    let sky = mix(horizon, zenith, up);
    let color = mix(sky, ground, down);
    return vec4(color, 1.0);
}
