use eframe::{
    egui::{self, PaintCallbackInfo},
    wgpu,
};
use egui_wgpu::{CallbackResources, CallbackTrait};
use crate::camera::CameraUniform;
use crate::mesh::{AlphaMode, LineVertex, Material, MeshPrimitive, SceneNode, SceneTree};
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
    show_bones: bool,
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

        if self.show_bones && res.bone_vertex_count > 0 {
            if let Some(buf) = &res.bone_vertex_buffer {
                render_pass.set_pipeline(&res.bone_pipeline);
                render_pass.set_vertex_buffer(0, buf.slice(..));
                render_pass.draw(0..res.bone_vertex_count, 0..1);
            }
        }
    }
}

pub struct App {
    roughness: f32,
    yaw: f32,
    pitch: f32,
    radius: f32,
    scene_tree: SceneTree,
    left_tab: LeftPanelTab,
    show_bones: bool,
    has_bones: bool,
    materials: Vec<Material>,
    selected_material: Option<usize>,
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        mesh_primitives: &[MeshPrimitive],
        bones: &[LineVertex],
        scene_tree: SceneTree,
        materials: Vec<Material>,
    ) -> Self {
        install_cjk_fallback_font(&cc.egui_ctx);

        let render_state = cc.wgpu_render_state.as_ref().unwrap();
        let device = &render_state.device;
        let res = Resources::new(device, render_state.target_format.into(), mesh_primitives, bones);
        render_state.renderer.write().callback_resources.insert(res);
        Self {
            roughness: 0.0,
            yaw: 0.0,
            pitch: 0.3,
            radius: 3.0,
            scene_tree,
            left_tab: LeftPanelTab::Scene,
            show_bones: false,
            has_bones: !bones.is_empty(),
            materials,
            selected_material: None,
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
                show_bones: self.show_bones,
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
                        LeftPanelTab::Materials => show_materials_tab(
                            ui, &self.materials, &mut self.selected_material,
                        ),
                    }
                });
            });

        egui::Panel::right("inspector_panel")
            .resizable(true)
            .show_inside(ui, |ui| {
                ui.heading("Properties");
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if self.left_tab == LeftPanelTab::Materials {
                        match self.selected_material.and_then(|i| self.materials.get(i)) {
                            Some(mat) => show_material_inspector(ui, mat),
                            None => { ui.weak("Select a material"); }
                        }
                    } else {
                        ui.add_enabled(
                            self.has_bones,
                            egui::Checkbox::new(&mut self.show_bones, "Show bones"),
                        );
                        if !self.has_bones {
                            ui.weak("(no skin in this glTF)");
                        }
                    }
                });
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

fn show_materials_tab(
    ui: &mut egui::Ui,
    materials: &[Material],
    selected: &mut Option<usize>,
) {
    if materials.is_empty() {
        ui.weak("(no materials)");
        return;
    }
    for (i, mat) in materials.iter().enumerate() {
        if ui.selectable_label(*selected == Some(i), &mat.name).clicked() {
            *selected = Some(i);
        }
    }
}

fn show_material_inspector(ui: &mut egui::Ui, mat: &Material) {
    ui.strong(&mat.name);
    ui.separator();

    let swatch_size = egui::vec2(32.0, 18.0);
    egui::Grid::new("material_props").num_columns(2).striped(true).show(ui, |ui| {
        ui.label("Base color");
        let c = mat.base_color;
        egui::color_picker::show_color(
            ui,
            egui::Color32::from_rgba_unmultiplied(
                (c[0] * 255.0) as u8, (c[1] * 255.0) as u8,
                (c[2] * 255.0) as u8, (c[3] * 255.0) as u8,
            ),
            swatch_size,
        );
        ui.end_row();

        ui.label("Metallic");
        ui.monospace(format!("{:.3}", mat.metallic));
        ui.end_row();

        ui.label("Roughness");
        ui.monospace(format!("{:.3}", mat.roughness));
        ui.end_row();

        ui.label("Emissive");
        let e = mat.emissive;
        egui::color_picker::show_color(
            ui,
            egui::Color32::from_rgb(
                (e[0].clamp(0.0, 1.0) * 255.0) as u8,
                (e[1].clamp(0.0, 1.0) * 255.0) as u8,
                (e[2].clamp(0.0, 1.0) * 255.0) as u8,
            ),
            swatch_size,
        );
        ui.end_row();

        ui.label("Alpha mode");
        ui.monospace(format!("{:?}", mat.alpha_mode));
        ui.end_row();

        if matches!(mat.alpha_mode, AlphaMode::Mask) {
            ui.label("Alpha cutoff");
            ui.monospace(format!("{:.3}", mat.alpha_cutoff));
            ui.end_row();
        }

        ui.label("Double sided");
        ui.monospace(mat.double_sided.to_string());
        ui.end_row();
    });

    ui.separator();
    ui.label("Textures");
    let yesno = |b: bool| if b { "yes" } else { "—" };
    egui::Grid::new("material_textures").num_columns(2).striped(true).show(ui, |ui| {
        ui.label("Base color");         ui.monospace(yesno(mat.has_base_color_texture)); ui.end_row();
        ui.label("Metallic/roughness"); ui.monospace(yesno(mat.has_metallic_roughness_texture)); ui.end_row();
        ui.label("Normal");             ui.monospace(yesno(mat.has_normal_texture)); ui.end_row();
        ui.label("Occlusion");          ui.monospace(yesno(mat.has_occlusion_texture)); ui.end_row();
        ui.label("Emissive");           ui.monospace(yesno(mat.has_emissive_texture)); ui.end_row();
    });
}
