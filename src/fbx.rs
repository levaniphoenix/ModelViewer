use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

use fbxcel_dom::any::AnyDocument;
use fbxcel_dom::v7400::data::mesh::layer::{TypedLayerElementHandle};
use fbxcel_dom::v7400::data::mesh::TriangleVertexIndex;
use fbxcel_dom::v7400::object::{model::TypedModelHandle, TypedObjectHandle};
use fbxcel_dom::v7400::Document;

use crate::mesh::{
    AlphaMode, LoadedModel, Material, MeshInfo, MeshPrimInfo, MeshPrimitive,
    SceneNode, SceneTree, Vertex,
};

pub fn load_fbx_all(path: &str) -> LoadedModel {
    let file = File::open(path).expect("failed to open FBX file");
    let reader = BufReader::new(file);
    let doc = match AnyDocument::from_seekable_reader(reader).expect("failed to parse FBX") {
        AnyDocument::V7400(_, doc) => *doc,
        _ => panic!("FBX version not supported (only 7.4+ binary FBX is supported)"),
    };

    let materials = collect_materials(&doc);
    let mat_id_to_global = build_material_index(&doc);
    let (primitives, meshes) = collect_meshes(&doc, &mat_id_to_global);
    let scene_tree = build_scene_tree(&doc);

    LoadedModel {
        primitives,
        scene_tree,
        bones: Vec::new(),
        materials,
        meshes,
    }
}

fn build_material_index(doc: &Document) -> HashMap<i64, usize> {
    let mut map = HashMap::new();
    let mut idx = 0usize;
    for obj in doc.objects() {
        if let TypedObjectHandle::Material(_) = obj.get_typed() {
            map.insert(obj.object_id().raw(), idx);
            idx += 1;
        }
    }
    map
}

fn collect_materials(doc: &Document) -> Vec<Material> {
    let mut out = Vec::new();
    let mut i = 0usize;
    for obj in doc.objects() {
        if let TypedObjectHandle::Material(mat) = obj.get_typed() {
            let props = mat.properties();
            let diffuse = props.diffuse_color_or_default().ok();
            let dr = diffuse.as_ref().map(|c| c.r).unwrap_or(0.8);
            let dg = diffuse.as_ref().map(|c| c.g).unwrap_or(0.8);
            let db = diffuse.as_ref().map(|c| c.b).unwrap_or(0.8);
            let factor = props.diffuse_factor_or_default().unwrap_or(1.0).clamp(0.0, 1.0);
            let emissive = props.emissive_color_or_default().ok();
            let er = emissive.as_ref().map(|c| c.r).unwrap_or(0.0);
            let eg = emissive.as_ref().map(|c| c.g).unwrap_or(0.0);
            let eb = emissive.as_ref().map(|c| c.b).unwrap_or(0.0);
            let alpha = (1.0 - props.transparency_factor_or_default().unwrap_or(0.0))
                .clamp(0.0, 1.0);
            let name = mat.name().map(str::to_string)
                .unwrap_or_else(|| format!("Material {i}"));
            out.push(Material {
                name,
                base_color: [
                    (dr * factor) as f32,
                    (dg * factor) as f32,
                    (db * factor) as f32,
                    alpha as f32,
                ],
                metallic: 0.0,
                roughness: 1.0,
                emissive: [er as f32, eg as f32, eb as f32],
                alpha_mode: if alpha < 0.999 { AlphaMode::Blend } else { AlphaMode::Opaque },
                alpha_cutoff: 0.5,
                double_sided: false,
                has_base_color_texture: mat.diffuse_texture().is_some(),
                has_metallic_roughness_texture: false,
                has_normal_texture: mat.normal_map_texture().is_some(),
                has_occlusion_texture: false,
                has_emissive_texture: mat.emissive_texture().is_some(),
                base_color_texture_path: None,
            });
            i += 1;
        }
    }
    out
}

fn material_name_at(doc: &Document, global_idx: usize) -> Option<String> {
    doc.objects()
        .filter_map(|o| match o.get_typed() {
            TypedObjectHandle::Material(m) => Some(m),
            _ => None,
        })
        .nth(global_idx)
        .and_then(|m| m.name().map(str::to_string))
}

fn collect_meshes(
    doc: &Document,
    mat_id_to_global: &HashMap<i64, usize>,
) -> (Vec<MeshPrimitive>, Vec<MeshInfo>) {
    let mut primitives: Vec<MeshPrimitive> = Vec::new();
    let mut meshes: Vec<MeshInfo> = Vec::new();
    let mut global_prim = 0usize;

    for obj in doc.objects() {
        let TypedObjectHandle::Model(TypedModelHandle::Mesh(model_mesh)) = obj.get_typed()
        else { continue };
        let Ok(geom) = model_mesh.geometry() else { continue };

        let mesh_name = model_mesh.name().unwrap_or("Mesh").to_string();
        let mesh_local_to_global: Vec<usize> = model_mesh.materials()
            .map(|m| mat_id_to_global.get(&m.object_id().raw()).copied().unwrap_or(0))
            .collect();

        let Ok(pv) = geom.polygon_vertices() else { continue };
        let Ok(tris) = pv.triangulate_each(triangulator) else { continue };

        let mut normals = None;
        let mut uvs = None;
        let mut materials_layer = None;
        for layer in geom.layers() {
            for entry in layer.layer_element_entries() {
                let Ok(typed) = entry.typed_layer_element() else { continue };
                match typed {
                    TypedLayerElementHandle::Normal(h) if normals.is_none() => {
                        normals = h.normals().ok();
                    }
                    TypedLayerElementHandle::Uv(h) if uvs.is_none() => {
                        uvs = h.uv().ok();
                    }
                    TypedLayerElementHandle::Material(h) if materials_layer.is_none() => {
                        materials_layer = h.materials().ok();
                    }
                    _ => {}
                }
            }
        }

        let tri_vis: Vec<TriangleVertexIndex> = tris.triangle_vertex_indices().collect();
        let tri_count = tri_vis.len() / 3;

        let mut groups: HashMap<Option<usize>, Vec<u32>> = HashMap::new();
        let mut vertices: Vec<Vertex> = Vec::with_capacity(tri_vis.len());

        for tri in 0..tri_count {
            let probe = tri_vis[tri * 3];
            let mat_local = if mesh_local_to_global.is_empty() {
                None
            } else if let Some(ml) = &materials_layer {
                ml.material_index(&tris, probe).ok().map(|m| m.to_u32() as usize)
            } else {
                Some(0)
            };
            let mat_global = mat_local.and_then(|i| mesh_local_to_global.get(i).copied());

            let group = groups.entry(mat_global).or_default();
            for c in 0..3 {
                let tri_vi = tri_vis[tri * 3 + c];
                let pos = tris.control_point(tri_vi)
                    .map(|p| [p.x as f32, p.y as f32, p.z as f32])
                    .unwrap_or([0.0; 3]);
                let normal = normals.as_ref()
                    .and_then(|n| n.normal(&tris, tri_vi).ok())
                    .map(|n| [n.x as f32, n.y as f32, n.z as f32])
                    .unwrap_or([0.0, 1.0, 0.0]);
                let uv = uvs.as_ref()
                    .and_then(|u| u.uv(&tris, tri_vi).ok())
                    .map(|u| [u.x as f32, 1.0 - u.y as f32])
                    .unwrap_or([0.0, 0.0]);
                let idx = vertices.len() as u32;
                vertices.push(Vertex { position: pos, normal, uv });
                group.push(idx);
            }
        }

        let mut prim_infos: Vec<MeshPrimInfo> = Vec::new();
        let mut group_keys: Vec<Option<usize>> = groups.keys().copied().collect();
        group_keys.sort_by_key(|k| k.unwrap_or(usize::MAX));
        for mat_global in group_keys {
            let indices = groups.remove(&mat_global).unwrap();
            let mut remap: HashMap<u32, u32> = HashMap::new();
            let mut new_verts: Vec<Vertex> = Vec::new();
            let mut new_indices: Vec<u32> = Vec::with_capacity(indices.len());
            for old in &indices {
                let entry = remap.entry(*old).or_insert_with(|| {
                    let i = new_verts.len() as u32;
                    new_verts.push(vertices[*old as usize]);
                    i
                });
                new_indices.push(*entry);
            }

            let prim_name = mat_global
                .and_then(|i| material_name_at(doc, i))
                .unwrap_or_else(|| format!("Primitive {}", prim_infos.len()));

            prim_infos.push(MeshPrimInfo {
                global_index: global_prim,
                name: prim_name,
                vertex_count: new_verts.len(),
                triangle_count: new_indices.len() / 3,
                material: mat_global,
            });

            primitives.push(MeshPrimitive {
                vertices: new_verts,
                indices: new_indices,
                material: mat_global,
            });
            global_prim += 1;
        }

        meshes.push(MeshInfo { name: mesh_name, primitives: prim_infos });
    }

    (primitives, meshes)
}

fn triangulator(
    _pv: &fbxcel_dom::v7400::data::mesh::PolygonVertices<'_>,
    poly: &[fbxcel_dom::v7400::data::mesh::PolygonVertexIndex],
    out: &mut Vec<[fbxcel_dom::v7400::data::mesh::PolygonVertexIndex; 3]>,
) -> anyhow::Result<()> {
    if poly.len() < 3 { return Ok(()) }
    for i in 1..(poly.len() - 1) {
        out.push([poly[0], poly[i], poly[i + 1]]);
    }
    Ok(())
}

fn build_scene_tree(doc: &Document) -> SceneTree {
    let mut roots = Vec::new();
    for obj in doc.objects() {
        let typed = match obj.get_typed() {
            TypedObjectHandle::Model(m) => m,
            _ => continue,
        };
        if typed.parent_model().is_none() {
            roots.push(build_node(typed));
        }
    }
    SceneTree { scene_name: "Scene".to_string(), roots }
}

fn build_node(model: TypedModelHandle<'_>) -> SceneNode {
    let name = model.name().unwrap_or("Node").to_string();
    let children = model.child_models().map(build_node).collect();
    SceneNode { name, children }
}
