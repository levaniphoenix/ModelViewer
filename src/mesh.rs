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
