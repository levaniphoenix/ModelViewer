use ::model_viewer::App;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        // Request wgpu as the rendering backend
        wgpu_options: eframe::egui_wgpu::WgpuConfiguration {
            ..Default::default()
        },
        ..Default::default()
    };
    eframe::run_native(
        "3D Model Viewer",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
