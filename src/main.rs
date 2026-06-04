use model_viewer::App;
use model_viewer::mesh;

fn main() -> eframe::Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "scenes/purah.fbx".to_string());
    let model = mesh::load_model(&path);
    println!(
        "Loaded {} primitives, {} bones, {} materials from {path}",
        model.primitives.len(),
        model.bones.len() / 24,
        model.materials.len(),
    );

    let options = eframe::NativeOptions {
        depth_buffer: 32,
        ..Default::default()
    };
    eframe::run_native(
        "3D Model Viewer",
        options,
        Box::new(move |cc| {
            Ok(Box::new(App::new(
                cc,
                &model.primitives,
                &model.bones,
                model.scene_tree,
                model.materials,
                model.meshes,
            )))
        }),
    )
}
