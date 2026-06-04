use std::path::PathBuf;

use eframe::wgpu;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
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
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 24,
                    shader_location: 2, // uv
                },
            ],
        }
    }
}

pub struct MeshPrimitive {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub material: Option<usize>,
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

            let uvs: Vec<[f32; 2]> = reader.read_tex_coords(0)
                .map(|t| t.into_f32().collect())
                .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);

            let vertices: Vec<Vertex> = positions.iter().zip(normals.iter()).zip(uvs.iter())
                .map(|((p, n), uv)| Vertex { position: *p, normal: *n, uv: *uv })
                .collect();

            let indices: Vec<u32> = reader.read_indices()
                .expect("mesh missing indices")
                .into_u32()
                .collect();

            let material = primitive.material().index();
            primitives.push(MeshPrimitive { vertices, indices, material });
        }
    }

    primitives
}

#[derive(Clone, Copy, Debug)]
pub enum AlphaMode { Opaque, Mask, Blend }

pub struct Material {
    pub name: String,
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: [f32; 3],
    pub alpha_mode: AlphaMode,
    pub alpha_cutoff: f32,
    pub double_sided: bool,
    pub has_base_color_texture: bool,
    pub has_metallic_roughness_texture: bool,
    pub has_normal_texture: bool,
    pub has_occlusion_texture: bool,
    pub has_emissive_texture: bool,
    pub base_color_texture_path: Option<PathBuf>,
}

pub fn load_materials(path: &str) -> Vec<Material> {
    let (gltf, _buffers, _) = gltf::import(path).unwrap();
    gltf.materials().enumerate().map(|(i, mat)| {
        let pbr = mat.pbr_metallic_roughness();
        let alpha_mode = match mat.alpha_mode() {
            gltf::material::AlphaMode::Opaque => AlphaMode::Opaque,
            gltf::material::AlphaMode::Mask => AlphaMode::Mask,
            gltf::material::AlphaMode::Blend => AlphaMode::Blend,
        };
        Material {
            name: mat.name().map(|s| s.to_string())
                .unwrap_or_else(|| format!("Material {i}")),
            base_color: pbr.base_color_factor(),
            metallic: pbr.metallic_factor(),
            roughness: pbr.roughness_factor(),
            emissive: mat.emissive_factor(),
            alpha_mode,
            alpha_cutoff: mat.alpha_cutoff().unwrap_or(0.5),
            double_sided: mat.double_sided(),
            has_base_color_texture: pbr.base_color_texture().is_some(),
            has_metallic_roughness_texture: pbr.metallic_roughness_texture().is_some(),
            has_normal_texture: mat.normal_texture().is_some(),
            has_occlusion_texture: mat.occlusion_texture().is_some(),
            has_emissive_texture: mat.emissive_texture().is_some(),
            base_color_texture_path: None,
        }
    }).collect()
}

pub struct MeshInfo {
    pub name: String,
    pub primitives: Vec<MeshPrimInfo>,
}

pub struct MeshPrimInfo {
    pub global_index: usize,
    pub name: String,
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub material: Option<usize>,
}

pub struct LoadedModel {
    pub primitives: Vec<MeshPrimitive>,
    pub scene_tree: SceneTree,
    pub bones: Vec<LineVertex>,
    pub materials: Vec<Material>,
    pub meshes: Vec<MeshInfo>,
}

pub fn load_model(path: &str) -> LoadedModel {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ext == "fbx" {
        crate::fbx::load_fbx_all(path)
    } else {
        LoadedModel {
            primitives: load_gltf(path),
            scene_tree: load_scene_tree(path),
            bones: load_bones(path),
            materials: load_materials(path),
            meshes: load_mesh_list(path),
        }
    }
}

pub fn load_mesh_list(path: &str) -> Vec<MeshInfo> {
    let (gltf, buffers, _) = gltf::import(path).unwrap();
    let mut meshes = Vec::new();
    let mut global = 0usize;
    for mesh in gltf.meshes() {
        let mesh_name = mesh.name().unwrap_or("Mesh").to_string();
        let mut prims = Vec::new();
        for (i, primitive) in mesh.primitives().enumerate() {
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
            let vertex_count = reader.read_positions().map(|p| p.count()).unwrap_or(0);
            let index_count = reader.read_indices()
                .map(|idx| idx.into_u32().count())
                .unwrap_or(0);
            let name = primitive.material().name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("Primitive {i}"));
            prims.push(MeshPrimInfo {
                global_index: global,
                name,
                vertex_count,
                triangle_count: index_count / 3,
                material: primitive.material().index(),
            });
            global += 1;
        }
        meshes.push(MeshInfo { name: mesh_name, primitives: prims });
    }
    meshes
}

pub struct SceneNode {
    pub name: String,
    pub children: Vec<SceneNode>,
}

pub struct SceneTree {
    pub scene_name: String,
    pub roots: Vec<SceneNode>,
}

/// Returns a world-space line list tracing the edges of Blender-style
/// octahedral bone shapes — 12 edges per joint → child-joint pair.
/// Empty when the file has no skins.
pub fn load_bones(path: &str) -> Vec<LineVertex> {
    let (gltf, _buffers, _) = gltf::import(path).unwrap();
    let mut world: Vec<Option<glam::Mat4>> = vec![None; gltf.nodes().count()];
    let scene = gltf.default_scene().or_else(|| gltf.scenes().next())
        .expect("glTF has no scenes");
    for root in scene.nodes() {
        accumulate_world(&root, glam::Mat4::IDENTITY, &mut world);
    }

    let mut verts = Vec::new();
    for skin in gltf.skins() {
        let joint_set: std::collections::HashSet<usize> =
            skin.joints().map(|j| j.index()).collect();
        for joint in skin.joints() {
            let Some(parent_w) = world[joint.index()] else { continue };
            let head = parent_w.col(3).truncate();
            for child in joint.children() {
                if !joint_set.contains(&child.index()) { continue }
                let Some(child_w) = world[child.index()] else { continue };
                let tail = child_w.col(3).truncate();
                append_octahedral_bone(head, tail, &mut verts);
            }
        }
    }
    verts
}

fn append_octahedral_bone(head: glam::Vec3, tail: glam::Vec3, out: &mut Vec<LineVertex>) {
    let dir = tail - head;
    let length = dir.length();
    if length < 1e-6 { return }
    let axis = dir / length;
    let rot = glam::Quat::from_rotation_arc(glam::Vec3::Y, axis);

    let r = 0.1_f32;
    let h = 0.1_f32;
    let ring_l = [
        glam::Vec3::new( r, h,  0.0),
        glam::Vec3::new(0.0, h,  r),
        glam::Vec3::new(-r, h,  0.0),
        glam::Vec3::new(0.0, h, -r),
    ];
    let to_world = |v: glam::Vec3| head + rot * (v * length);
    let head_w = to_world(glam::Vec3::ZERO);
    let tail_w = to_world(glam::Vec3::Y);
    let ring_w: [glam::Vec3; 4] = std::array::from_fn(|i| to_world(ring_l[i]));

    let color = [1.0, 0.55, 0.10];
    let mut edge = |a: glam::Vec3, b: glam::Vec3| {
        out.push(LineVertex { position: a.to_array(), color });
        out.push(LineVertex { position: b.to_array(), color });
    };
    // Head spokes, ring loop, tail spokes — 12 edges total.
    for i in 0..4 { edge(head_w, ring_w[i]); }
    for i in 0..4 { edge(ring_w[i], ring_w[(i + 1) % 4]); }
    for i in 0..4 { edge(ring_w[i], tail_w); }
}

fn accumulate_world(node: &gltf::Node, parent: glam::Mat4, out: &mut [Option<glam::Mat4>]) {
    let local = glam::Mat4::from_cols_array_2d(&node.transform().matrix());
    let w = parent * local;
    out[node.index()] = Some(w);
    for child in node.children() {
        accumulate_world(&child, w, out);
    }
}

pub fn load_scene_tree(path: &str) -> SceneTree {
    let (gltf, _buffers, _) = gltf::import(path).unwrap();
    let scene = gltf.default_scene().or_else(|| gltf.scenes().next())
        .expect("glTF has no scenes");
    let scene_name = scene.name().unwrap_or("Scene").to_string();
    let roots = scene.nodes().map(|n| convert_node(&n)).collect();
    SceneTree { scene_name, roots }
}

fn convert_node(node: &gltf::Node) -> SceneNode {
    let name = node.name().unwrap_or("Node").to_string();
    let mut children: Vec<SceneNode> = node.children().map(|c| convert_node(&c)).collect();

    if let Some(mesh) = node.mesh() {
        let mesh_name = mesh.name().unwrap_or("Mesh").to_string();
        let prim_children: Vec<SceneNode> = mesh.primitives().enumerate().map(|(i, prim)| {
            let prim_name = prim.material().name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("Primitive {i}"));
            SceneNode { name: prim_name, children: Vec::new() }
        }).collect();
        children.push(SceneNode { name: mesh_name, children: prim_children });
    }

    SceneNode { name, children }
}
