use eframe::{
    egui::{self, PaintCallbackInfo},
    wgpu,
};
use egui_wgpu::{CallbackResources, CallbackTrait};
use crate::camera::CameraUniform;
use crate::renderer::Resources;

struct ViewportCallback;

impl CallbackTrait for ViewportCallback {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let res = resources.get::<Resources>().unwrap();

        let camera_uniform = CameraUniform::new(&res.camera); // see step 2
        queue.write_buffer(&res.uniform_buffer, 0, bytemuck::cast_slice(&[camera_uniform]));

        Vec::new()
    }
    fn paint(
        &self,
        info: PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        resources: &CallbackResources,
    ) {
        let res = resources.get::<Resources>().unwrap();
        render_pass.set_pipeline(&res.pipeline);
        render_pass.set_bind_group(0, &res.uniform_buffer_bind_group, &[]);
        render_pass.draw(0..36, 0..1);
    }
}

#[derive(Default)]
pub struct App {
    roughness: f32,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self { 
        let render_state = cc.wgpu_render_state.as_ref().unwrap();
        let device  = &render_state.device;
        let res = Resources::new(device, render_state.target_format.into());
        render_state.renderer.write().callback_resources.insert(res);
        Self::default()
    }

    fn render_3d_viewport(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
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
            .default_size(250.0)
            .size_range(100.0..=500.0)
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