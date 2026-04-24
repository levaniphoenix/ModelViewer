use eframe::{
    egui::{self, PaintCallbackInfo},
    wgpu,
};
use egui_wgpu::{CallbackResources, CallbackTrait};
use crate::camera::CameraUniform;
use crate::mesh::{MeshPrimitive, SceneNode, SceneTree};
use crate::renderer::Resources;

#[derive(Clone, Copy, PartialEq, Eq)]
enum LeftPanelTab {
    Scene,
    Meshes,
    Materials,
}

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
    scene_tree: SceneTree,
    left_tab: LeftPanelTab,
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        mesh_primitives: &[MeshPrimitive],
        scene_tree: SceneTree,
    ) -> Self {
        install_cjk_fallback_font(&cc.egui_ctx);

        let render_state = cc.wgpu_render_state.as_ref().unwrap();
        let device = &render_state.device;
        let res = Resources::new(device, render_state.target_format.into(), mesh_primitives);
        render_state.renderer.write().callback_resources.insert(res);
        Self {
            roughness: 0.0,
            yaw: 0.0,
            pitch: 0.3,
            radius: 3.0,
            scene_tree,
            left_tab: LeftPanelTab::Scene,
        }
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
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.left_tab, LeftPanelTab::Scene, "Scene");
                    ui.selectable_value(&mut self.left_tab, LeftPanelTab::Meshes, "Meshes");
                    ui.selectable_value(&mut self.left_tab, LeftPanelTab::Materials, "Materials");
                });
                ui.separator();
                egui::ScrollArea::both().show(ui, |ui| {
                    match self.left_tab {
                        LeftPanelTab::Scene => show_scene_tab(ui, &self.scene_tree),
                        LeftPanelTab::Meshes => { ui.label("(not implemented yet)"); }
                        LeftPanelTab::Materials => { ui.label("(not implemented yet)"); }
                    }
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

fn install_cjk_fallback_font(ctx: &egui::Context) {
    let candidates = [
        "C:/Windows/Fonts/msyh.ttc",
        "C:/Windows/Fonts/YuGothM.ttc",
        "C:/Windows/Fonts/meiryo.ttc",
        "C:/Windows/Fonts/simsun.ttc",
        "C:/Windows/Fonts/msgothic.ttc",
        "/System/Library/Fonts/PingFang.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
    ];

    let Some(bytes) = candidates.iter().find_map(|p| std::fs::read(p).ok()) else {
        return;
    };

    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "cjk".to_owned(),
        std::sync::Arc::new(egui::FontData::from_owned(bytes)),
    );
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts.families.entry(family).or_default().push("cjk".to_owned());
    }
    ctx.set_fonts(fonts);
}

fn show_scene_tab(ui: &mut egui::Ui, tree: &SceneTree) {
    egui::CollapsingHeader::new(&tree.scene_name)
        .default_open(true)
        .show(ui, |ui| {
            for node in &tree.roots {
                show_node(ui, node);
            }
        });
}

fn show_node(ui: &mut egui::Ui, node: &SceneNode) {
    if node.children.is_empty() {
        ui.label(&node.name);
    } else {
        egui::CollapsingHeader::new(&node.name)
            .id_salt(node as *const _ as usize)
            .default_open(true)
            .show(ui, |ui| {
                for child in &node.children {
                    show_node(ui, child);
                }
            });
    }
}
