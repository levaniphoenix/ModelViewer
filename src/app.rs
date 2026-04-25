use std::path::Path;

use eframe::{
    egui::{self, PaintCallbackInfo},
    wgpu,
};
use egui_wgpu::{CallbackResources, CallbackTrait, RenderState};
use crate::camera::CameraUniform;
use crate::mesh::{AlphaMode, LineVertex, Material, MeshInfo, MeshPrimitive, SceneNode, SceneTree};
use crate::renderer::{MaterialUniform, Resources};
use crate::state::{self, MaterialState, ViewerState};

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
    material_colors: Vec<[f32; 4]>,
    primitive_visible: Vec<bool>,
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

        for (i, color) in self.material_colors.iter().enumerate() {
            if let Some(mat) = res.materials.get(i) {
                queue.write_buffer(
                    &mat.buffer, 0,
                    bytemuck::cast_slice(&[MaterialUniform { base_color: *color }]),
                );
            }
        }

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
        for (i, prim) in res.primitives.iter().enumerate() {
            if !self.primitive_visible.get(i).copied().unwrap_or(true) {
                continue;
            }
            let mat = prim.material
                .and_then(|i| res.materials.get(i))
                .unwrap_or(&res.default_material);
            render_pass.set_bind_group(1, &mat.bind_group, &[]);
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
    meshes: Vec<MeshInfo>,
    primitive_visible: Vec<bool>,
    selected_primitive: Option<usize>,
}

fn primitive_key(mesh_name: &str, prim_name: &str) -> String {
    format!("{mesh_name}/{prim_name}")
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        mesh_primitives: &[MeshPrimitive],
        bones: &[LineVertex],
        scene_tree: SceneTree,
        mut materials: Vec<Material>,
        meshes: Vec<MeshInfo>,
    ) -> Self {
        install_cjk_fallback_font(&cc.egui_ctx);

        let saved = state::load();

        if let Some(s) = &saved {
            for mat in materials.iter_mut() {
                if let Some(ms) = s.materials.get(&mat.name) {
                    mat.base_color = ms.base_color;
                }
            }
        }

        let render_state = cc.wgpu_render_state.as_ref().unwrap();
        let device = &render_state.device;
        let queue = &render_state.queue;
        let res = Resources::new(
            device, queue, render_state.target_format.into(),
            mesh_primitives, bones, &materials,
        );
        render_state.renderer.write().callback_resources.insert(res);

        if let Some(s) = &saved {
            for i in 0..materials.len() {
                let path = s.materials.get(&materials[i].name)
                    .and_then(|ms| ms.base_color_texture_path.clone());
                if let Some(path) = path {
                    if let Err(e) = apply_base_color_texture(render_state, &mut materials, i, &path) {
                        eprintln!("could not restore texture for '{}': {e}", materials[i].name);
                    }
                }
            }
        }

        let (yaw, pitch, radius, show_bones) = saved
            .as_ref()
            .map(|s| (s.yaw, s.pitch, s.radius, s.show_bones))
            .unwrap_or((0.0, 0.3, 3.0, false));

        let mut primitive_visible = vec![true; mesh_primitives.len()];
        if let Some(s) = &saved {
            let hidden: std::collections::HashSet<&String> = s.hidden_primitives.iter().collect();
            for mesh in &meshes {
                for prim in &mesh.primitives {
                    if hidden.contains(&primitive_key(&mesh.name, &prim.name)) {
                        if let Some(v) = primitive_visible.get_mut(prim.global_index) {
                            *v = false;
                        }
                    }
                }
            }
        }

        Self {
            roughness: 0.0,
            yaw,
            pitch,
            radius,
            scene_tree,
            left_tab: LeftPanelTab::Scene,
            show_bones: show_bones && !bones.is_empty(),
            has_bones: !bones.is_empty(),
            materials,
            selected_material: None,
            meshes,
            primitive_visible,
            selected_primitive: None,
        }
    }

    fn snapshot_state(&self) -> ViewerState {
        let materials = self.materials.iter().map(|m| {
            (m.name.clone(), MaterialState {
                base_color: m.base_color,
                base_color_texture_path: m.base_color_texture_path.clone(),
            })
        }).collect();
        let mut hidden_primitives = Vec::new();
        for mesh in &self.meshes {
            for prim in &mesh.primitives {
                let visible = self.primitive_visible
                    .get(prim.global_index).copied().unwrap_or(true);
                if !visible {
                    hidden_primitives.push(primitive_key(&mesh.name, &prim.name));
                }
            }
        }
        ViewerState {
            yaw: self.yaw,
            pitch: self.pitch,
            radius: self.radius,
            show_bones: self.show_bones,
            materials,
            hidden_primitives,
        }
    }

    fn handle_pick_texture(&mut self, idx: usize, frame: &eframe::Frame) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Image", &["png", "jpg", "jpeg"])
            .pick_file()
        else { return };

        let Some(render_state) = frame.wgpu_render_state() else { return };
        if let Err(e) = apply_base_color_texture(render_state, &mut self.materials, idx, &path) {
            eprintln!("failed to load {}: {e}", path.display());
            return;
        }
        if let Some(m) = self.materials.get_mut(idx) {
            m.base_color_texture_path = Some(path);
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
                material_colors: self.materials.iter().map(|m| m.base_color).collect(),
                primitive_visible: self.primitive_visible.clone(),
            },
        ));
    }
}

fn apply_base_color_texture(
    render_state: &RenderState,
    materials: &mut [Material],
    idx: usize,
    path: &Path,
) -> Result<(), String> {
    let img = image::open(path)
        .map_err(|e| e.to_string())?
        .to_rgba8();
    let (w, h) = (img.width(), img.height());

    let device = &render_state.device;
    let queue = &render_state.queue;
    let mut renderer = render_state.renderer.write();
    let res = renderer.callback_resources.get_mut::<Resources>()
        .ok_or_else(|| "renderer resources not initialized".to_string())?;
    let mat = res.materials.get_mut(idx)
        .ok_or_else(|| format!("no GPU material at index {idx}"))?;

    let size = wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("picked_base_color"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        texture.as_image_copy(),
        &img,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(w * 4),
            rows_per_image: Some(h),
        },
        size,
    );
    mat.texture_view = texture.create_view(&Default::default());
    mat.texture = texture;
    mat.rebuild_bind_group(device, &res.material_bind_group_layout, Some("picked_base_color"));

    if let Some(m) = materials.get_mut(idx) {
        m.has_base_color_texture = true;
    }
    Ok(())
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
                        LeftPanelTab::Meshes => show_meshes_tab(
                            ui,
                            &self.meshes,
                            &mut self.primitive_visible,
                            &mut self.selected_primitive,
                        ),
                        LeftPanelTab::Materials => show_materials_tab(
                            ui, &self.materials, &mut self.selected_material,
                        ),
                    }
                });
            });

        let mut pick_texture_for: Option<usize> = None;
        let mut jump_to_material: Option<usize> = None;
        egui::Panel::right("inspector_panel")
            .resizable(true)
            .show_inside(ui, |ui| {
                ui.heading("Properties");
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    match self.left_tab {
                        LeftPanelTab::Materials => {
                            if let Some(idx) = self.selected_material {
                                if let Some(mat) = self.materials.get_mut(idx) {
                                    if show_material_inspector(ui, mat) {
                                        pick_texture_for = Some(idx);
                                    }
                                } else {
                                    ui.weak("Select a material");
                                }
                            } else {
                                ui.weak("Select a material");
                            }
                        }
                        LeftPanelTab::Meshes => {
                            jump_to_material = show_mesh_inspector(
                                ui,
                                &self.meshes,
                                &self.materials,
                                &mut self.primitive_visible,
                                self.selected_primitive,
                            );
                        }
                        LeftPanelTab::Scene => {
                            ui.add_enabled(
                                self.has_bones,
                                egui::Checkbox::new(&mut self.show_bones, "Show bones"),
                            );
                            if !self.has_bones {
                                ui.weak("(no skin in this glTF)");
                            }
                        }
                    }
                });
            });
        if let Some(idx) = jump_to_material {
            self.selected_material = Some(idx);
            self.left_tab = LeftPanelTab::Materials;
        }
        if let Some(idx) = pick_texture_for {
            self.handle_pick_texture(idx, frame);
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.render_3d_viewport(ui, frame);
        });
    }

    fn on_exit(&mut self) {
        state::save(&self.snapshot_state());
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

fn show_meshes_tab(
    ui: &mut egui::Ui,
    meshes: &[MeshInfo],
    visible: &mut [bool],
    selected: &mut Option<usize>,
) {
    if meshes.is_empty() {
        ui.weak("(no meshes)");
        return;
    }

    ui.horizontal(|ui| {
        if ui.small_button("Show all").clicked() {
            for v in visible.iter_mut() { *v = true; }
        }
        if ui.small_button("Hide all").clicked() {
            for v in visible.iter_mut() { *v = false; }
        }
    });
    ui.separator();

    for (mi, mesh) in meshes.iter().enumerate() {
        let header = format!("{} ({})", mesh.name, mesh.primitives.len());
        egui::CollapsingHeader::new(header)
            .id_salt(("mesh", mi))
            .default_open(true)
            .show(ui, |ui| {
                for prim in &mesh.primitives {
                    let row = ui.horizontal(|ui| {
                        if let Some(v) = visible.get_mut(prim.global_index) {
                            ui.checkbox(v, "");
                        }
                        ui.selectable_label(
                            *selected == Some(prim.global_index),
                            &prim.name,
                        )
                    });
                    if row.inner.clicked() {
                        *selected = Some(prim.global_index);
                    }
                }
            });
    }
}

fn show_mesh_inspector(
    ui: &mut egui::Ui,
    meshes: &[MeshInfo],
    materials: &[Material],
    visible: &mut [bool],
    selected: Option<usize>,
) -> Option<usize> {
    let Some(global) = selected else {
        ui.weak("Select a primitive");
        return None;
    };
    let Some((mesh, prim)) = meshes.iter().find_map(|m| {
        m.primitives.iter().find(|p| p.global_index == global).map(|p| (m, p))
    }) else {
        ui.weak("Select a primitive");
        return None;
    };

    ui.strong(&prim.name);
    ui.weak(format!("in mesh “{}”", mesh.name));
    ui.separator();

    let mut jump = None;
    egui::Grid::new("mesh_prim_props").num_columns(2).striped(true).show(ui, |ui| {
        ui.label("Vertices");
        ui.monospace(format_count(prim.vertex_count));
        ui.end_row();

        ui.label("Triangles");
        ui.monospace(format_count(prim.triangle_count));
        ui.end_row();

        ui.label("Material");
        match prim.material.and_then(|i| materials.get(i).map(|m| (i, m))) {
            Some((i, mat)) => {
                if ui.link(&mat.name).on_hover_text("Open in Materials tab").clicked() {
                    jump = Some(i);
                }
            }
            None => { ui.weak("(default)"); }
        }
        ui.end_row();

        ui.label("Visible");
        if let Some(v) = visible.get_mut(global) {
            ui.checkbox(v, "");
        }
        ui.end_row();
    });

    jump
}

fn format_count(n: usize) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*b as char);
    }
    out
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

fn show_material_inspector(ui: &mut egui::Ui, mat: &mut Material) -> bool {
    let mut pick_texture = false;
    ui.strong(&mat.name);
    ui.separator();

    let swatch_size = egui::vec2(32.0, 18.0);
    egui::Grid::new("material_props").num_columns(2).striped(true).show(ui, |ui| {
        ui.label("Base color");
        ui.color_edit_button_rgba_unmultiplied(&mut mat.base_color);
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
        ui.label("Base color");
        ui.horizontal(|ui| {
            ui.monospace(yesno(mat.has_base_color_texture));
            if ui.button("Pick…").clicked() {
                pick_texture = true;
            }
        });
        ui.end_row();
        ui.label("Metallic/roughness"); ui.monospace(yesno(mat.has_metallic_roughness_texture)); ui.end_row();
        ui.label("Normal");             ui.monospace(yesno(mat.has_normal_texture)); ui.end_row();
        ui.label("Occlusion");          ui.monospace(yesno(mat.has_occlusion_texture)); ui.end_row();
        ui.label("Emissive");           ui.monospace(yesno(mat.has_emissive_texture)); ui.end_row();
    });

    pick_texture
}
