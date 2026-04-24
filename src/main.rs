use model_viewer::App;
use model_viewer::mesh;

fn main() -> eframe::Result<()> {
    let primitives = mesh::load_gltf("scenes/lumine.glb");
    println!("Loaded {} primitives", primitives.len());

    let options = eframe::NativeOptions {
        depth_buffer: 32,
        ..Default::default()
    };
    eframe::run_native(
        "3D Model Viewer",
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, &primitives)))),
    )
}
