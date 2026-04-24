use eframe::wgpu;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

impl Vertex {
    pub fn buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0, // position
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 12,
                    shader_location: 1, // normal
                },
            ],
        }
    }
}

pub struct MeshPrimitive {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LineVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl LineVertex {
    pub fn buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<LineVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 12,
                    shader_location: 1,
                },
            ],
        }
    }
}

/// Build a ground-plane grid at y=0 with colored X (red) and Z (blue) axes.
/// `half_extent` — grid goes from -half_extent..=half_extent on each axis.
pub fn build_grid(half_extent: i32) -> Vec<LineVertex> {
    let grid_color = [0.35, 0.35, 0.35];
    let x_axis_color = [0.85, 0.25, 0.25];
    let z_axis_color = [0.25, 0.45, 0.85];
    let e = half_extent as f32;
    let mut verts = Vec::new();

    for i in -half_extent..=half_extent {
        let t = i as f32;
        // Lines parallel to X at z = t. Skip z=0 (X axis drawn separately).
        if i != 0 {
            verts.push(LineVertex { position: [-e, 0.0, t], color: grid_color });
            verts.push(LineVertex { position: [ e, 0.0, t], color: grid_color });
        }
        // Lines parallel to Z at x = t. Skip x=0 (Z axis drawn separately).
        if i != 0 {
            verts.push(LineVertex { position: [t, 0.0, -e], color: grid_color });
            verts.push(LineVertex { position: [t, 0.0,  e], color: grid_color });
        }
    }

    verts.push(LineVertex { position: [-e, 0.0, 0.0], color: x_axis_color });
    verts.push(LineVertex { position: [ e, 0.0, 0.0], color: x_axis_color });
    verts.push(LineVertex { position: [0.0, 0.0, -e], color: z_axis_color });
    verts.push(LineVertex { position: [0.0, 0.0,  e], color: z_axis_color });

    verts
}

pub fn load_gltf(path: &str) -> Vec<MeshPrimitive> {
    let (gltf, buffers, _) = gltf::import(path).unwrap();
    let mut primitives = Vec::new();

    for mesh in gltf.meshes() {
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            let positions: Vec<[f32; 3]> = reader.read_positions()
                .expect("mesh missing positions")
                .collect();

            let normals: Vec<[f32; 3]> = reader.read_normals()
                .map(|n| n.collect())
                .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);

            let vertices: Vec<Vertex> = positions.iter().zip(normals.iter())
                .map(|(p, n)| Vertex { position: *p, normal: *n })
                .collect();

            let indices: Vec<u32> = reader.read_indices()
                .expect("mesh missing indices")
                .into_u32()
                .collect();

            primitives.push(MeshPrimitive { vertices, indices });
        }
    }

    primitives
}
