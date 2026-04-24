use model_viewer::App;
use model_viewer::mesh;

fn main() -> eframe::Result<()> {
    let path = "scenes/lumine.glb";
    let primitives = mesh::load_gltf(path);
    let scene_tree = mesh::load_scene_tree(path);
    println!("Loaded {} primitives", primitives.len());

    let options = eframe::NativeOptions {
        depth_buffer: 32,
        ..Default::default()
    };
    eframe::run_native(
        "3D Model Viewer",
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, &primitives, scene_tree)))),
    )
}
