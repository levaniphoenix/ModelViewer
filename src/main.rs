use model_viewer::App;
use model_viewer::mesh;

fn main() -> eframe::Result<()> {
    let path = "scenes/lumine.glb";
    let primitives = mesh::load_gltf(path);
    let scene_tree = mesh::load_scene_tree(path);
    let bones = mesh::load_bones(path);
    let materials = mesh::load_materials(path);
    let meshes = mesh::load_mesh_list(path);
    println!(
        "Loaded {} primitives, {} bones, {} materials",
        primitives.len(), bones.len() / 24, materials.len(),
    );

    let options = eframe::NativeOptions {
        depth_buffer: 32,
        ..Default::default()
    };
    eframe::run_native(
        "3D Model Viewer",
        options,
        Box::new(move |cc| Ok(Box::new(
            App::new(cc, &primitives, &bones, scene_tree, materials, meshes)
        ))),
    )
}
