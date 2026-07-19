use tomolib::formats::bntx::Bntx;

use crate::{renderer::texture::PreviewCallback, ui::DrawUiWith};

#[derive(Default)]
pub struct TextureEditor {
    pub is_editor_visible: bool,
    pub selected_texture: Option<String>,
}

impl DrawUiWith<Option<&Bntx>, ()> for TextureEditor {
    fn draw_with(&mut self, ui: &mut egui::Ui, main_bntx: Option<&Bntx>) {
        egui::Window::new("Texture Editor")
            .collapsible(false)
            .resizable(true)
            .default_size([560.0, 400.0])
            .open(&mut self.is_editor_visible)
            .fade_out(false)
            .show(ui, |ui| {
                egui::Panel::left("texture_editor_list")
                    .resizable(true)
                    .min_size(180.0)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("texture_editor_list_scroll")
                            .auto_shrink(false)
                            .show(ui, |ui| {
                                let Some(bntx) = main_bntx else {
                                    ui.weak("No textures in this layout's own file.");
                                    return;
                                };

                                ui.weak(format!("{} Total Textures", bntx.textures.len()));
                                ui.separator();

                                for tex in &bntx.textures {
                                    let is_selected =
                                        self.selected_texture.as_deref() == Some(&tex.name);

                                    if ui.selectable_label(is_selected, &tex.name).clicked() {
                                        self.selected_texture = Some(tex.name.clone());
                                    }
                                }
                            });
                    });

                egui::Frame::new()
                    .inner_margin(egui::Margin {
                        left: 16,
                        right: 16,
                        top: 0,
                        bottom: 0,
                    })
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("texture_editor_detail_scroll")
                            .auto_shrink(false)
                            .show(ui, |ui| {
                                let Some(selected_name) = self.selected_texture.clone() else {
                                    ui.label("Select a texture on the left.");
                                    return;
                                };

                                let Some(tex) = main_bntx
                                    .into_iter()
                                    .flat_map(|bntx| bntx.textures.iter())
                                    .find(|tex| tex.name == selected_name)
                                else {
                                    ui.label("This texture is no longer part of the loaded file.");
                                    return;
                                };

                                ui.heading(&tex.name);
                                ui.add_space(4.0);

                                egui::Grid::new("texture_editor_info_grid")
                                    .num_columns(2)
                                    .spacing([16.0, 6.0])
                                    .show(ui, |ui| {
                                        ui.label("Dimensions:");
                                        ui.label(format!(
                                            "{} x {}",
                                            tex.info.width, tex.info.height
                                        ));
                                        ui.end_row();

                                        ui.label("Format:");
                                        ui.label(tex.info.format.name());
                                        ui.end_row();

                                        ui.label("Mip Levels:");
                                        ui.label(tex.info.mip_count.to_string());
                                        ui.end_row();

                                        ui.label("Array Count:");
                                        ui.label(tex.info.array_count.to_string());
                                        ui.end_row();
                                    });

                                ui.add_space(8.0);
                                ui.separator();
                                ui.add_space(4.0);

                                let aspect =
                                    tex.info.width as f32 / (tex.info.height.max(1) as f32);
                                let available_space = ui.available_size();

                                let size = if available_space.x / available_space.y > aspect {
                                    egui::vec2(available_space.y * aspect, available_space.y)
                                } else {
                                    egui::vec2(available_space.x, available_space.x / aspect)
                                };

                                let (rect, _response) =
                                    ui.allocate_exact_size(size, egui::Sense::hover());

                                ui.painter().add(egui_wgpu::Callback::new_paint_callback(
                                    rect,
                                    PreviewCallback {
                                        texture_name: selected_name,
                                    },
                                ));
                            });
                    });
            });
    }
}
