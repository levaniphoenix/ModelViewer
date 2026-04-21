use eframe::{
    egui::{self, PaintCallbackInfo},
    wgpu,
};
use egui_wgpu::{CallbackResources, CallbackTrait};

struct ViewportCallback;

impl CallbackTrait for ViewportCallback {
    fn paint(
        &self,
        info: PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        resources: &CallbackResources,
    ) {
        // your rendering code here
    }
}

#[derive(Default)]
pub struct App {
    roughness: f32,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_global_style.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        Self::default()
    }

    fn render_3d_viewport(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::drag());

        // if response.dragged() {
        //     self.camera.rotate(response.drag_delta());
        // }

        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
            rect,
            ViewportCallback,
        ));
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        egui::Panel::left("scene_panel")
            .resizable(true)
            .show_inside(ui, |ui| {
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.heading("Scene Node");
                    ui.label("Model 1: Super_long_filename_that_normally_breaks_layout.gltf");
                });
            });

        egui::Panel::right("inspector_panel")
            .resizable(true)
            .show_inside(ui, |ui| {
                ui.heading("Properties");
                ui.add(egui::Slider::new(&mut self.roughness, 0.0..=1.0).text("Roughness"));
            });

        egui::Panel::bottom("console_panel")
            .resizable(true)
            .size_range(100.0..=200.0)
            .show_inside(ui, |ui| {
                ui.label("System loaded successfully.");
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.render_3d_viewport(ui, frame);
        });
    }
}
