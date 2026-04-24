use eframe::{
    egui::{self, PaintCallbackInfo},
    wgpu,
};
use egui_wgpu::{CallbackResources, CallbackTrait};
use crate::camera::CameraUniform;
use crate::mesh::MeshPrimitive;
use crate::renderer::Resources;

struct ViewportCallback {
    yaw: f32,
    pitch: f32,
    radius: f32,
    aspect: f32,
}

impl CallbackTrait for ViewportCallback {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let res = resources.get_mut::<Resources>().unwrap();
        res.camera.aspect = self.aspect;
        res.camera.orbit(self.yaw, self.pitch, self.radius);

        let camera_uniform = CameraUniform::new(&res.camera);
        queue.write_buffer(&res.uniform_buffer, 0, bytemuck::cast_slice(&[camera_uniform]));

        Vec::new()
    }

    fn paint(
        &self,
        _info: PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        resources: &CallbackResources,
    ) {
        let res = resources.get::<Resources>().unwrap();
        render_pass.set_bind_group(0, &res.uniform_buffer_bind_group, &[]);

        render_pass.set_pipeline(&res.sky_pipeline);
        render_pass.draw(0..3, 0..1);

        render_pass.set_pipeline(&res.pipeline);
        for prim in &res.primitives {
            render_pass.set_vertex_buffer(0, prim.vertex_buffer.slice(..));
            render_pass.set_index_buffer(prim.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..prim.index_count, 0, 0..1);
        }

        render_pass.set_pipeline(&res.grid_pipeline);
        render_pass.set_vertex_buffer(0, res.grid_vertex_buffer.slice(..));
        render_pass.draw(0..res.grid_vertex_count, 0..1);
    }
}

pub struct App {
    roughness: f32,
    yaw: f32,
    pitch: f32,
    radius: f32,
}

impl Default for App {
    fn default() -> Self {
        Self {
            roughness: 0.0,
            yaw: 0.0,
            pitch: 0.3,
            radius: 3.0,
        }
    }
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, mesh_primitives: &[MeshPrimitive]) -> Self {
        let render_state = cc.wgpu_render_state.as_ref().unwrap();
        let device = &render_state.device;
        let res = Resources::new(device, render_state.target_format.into(), mesh_primitives);
        render_state.renderer.write().callback_resources.insert(res);
        Self::default()
    }

    fn render_3d_viewport(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let (rect, response) =
            ui.allocate_exact_size(ui.available_size(), egui::Sense::drag());

        if response.dragged() {
            let delta = response.drag_delta();
            self.yaw   -= delta.x * 0.01;
            self.pitch += delta.y * 0.01;
        }

        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            self.radius = (self.radius - scroll * 0.01).max(0.5);
        }

        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
            rect,
            ViewportCallback {
                yaw: self.yaw,
                pitch: self.pitch,
                radius: self.radius,
                aspect: rect.width() / rect.height(),
            },
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
                    ui.label("Model 1: lumine.glb");
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
