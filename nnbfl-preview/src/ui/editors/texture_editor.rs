use tomolib::formats::bntx::Bntx;

use crate::{
    font::glyph_atlas::{GLYPH_ATLAS_TEXTURE_NAME, GlyphAtlas},
    renderer::texture::PreviewCallback,
    ui::DrawUiWith,
};

#[derive(Default)]
pub struct TextureEditor {
    pub is_editor_visible: bool,
    pub selected_texture: Option<String>,
}

impl DrawUiWith<(Option<&Bntx>, &GlyphAtlas), ()> for TextureEditor {
    fn draw_with(&mut self, ui: &mut egui::Ui, state: (Option<&Bntx>, &GlyphAtlas)) {
        let main_bntx = state.0;
        let atlas = state.1;

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
                                let total_count = main_bntx.map_or(1, |b| b.textures.len() + 1);
                                ui.weak(format!("{} Total Textures", total_count));
                                ui.separator();

                                let is_atlas_selected = self.selected_texture.as_deref()
                                    == Some(GLYPH_ATLAS_TEXTURE_NAME);

                                if ui
                                    .selectable_label(is_atlas_selected, "Glyph Atlas")
                                    .clicked()
                                {
                                    self.selected_texture =
                                        Some(GLYPH_ATLAS_TEXTURE_NAME.to_string());
                                }

                                ui.separator();

                                if let Some(bntx) = main_bntx {
                                    for tex in &bntx.textures {
                                        let is_selected =
                                            self.selected_texture.as_deref() == Some(&tex.name);

                                        if ui.selectable_label(is_selected, &tex.name).clicked() {
                                            self.selected_texture = Some(tex.name.clone());
                                        }
                                    }
                                } else {
                                    ui.weak("No additional textures in file.");
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

                                let is_atlas = selected_name == GLYPH_ATLAS_TEXTURE_NAME;

                                let (width, height, format_str, mips, arrays) = if is_atlas {
                                    let atlas_size = atlas.used_bounds();

                                    let active_w = atlas_size.0.max(64);
                                    let active_h = atlas_size.1.max(active_w);

                                    (active_w, active_h, "R8_UNORM / RGBA8".to_owned(), 1, 1)
                                } else if let Some(tex) = main_bntx
                                    .into_iter()
                                    .flat_map(|bntx| bntx.textures.iter())
                                    .find(|tex| tex.name == selected_name)
                                {
                                    (
                                        tex.info.width,
                                        tex.info.height,
                                        tex.info.format.name(),
                                        tex.info.mip_count,
                                        tex.info.array_count,
                                    )
                                } else {
                                    ui.label("This texture is no longer loaded.");
                                    return;
                                };

                                ui.heading(if is_atlas {
                                    "Glyph Atlas (System Font)"
                                } else {
                                    &selected_name
                                });

                                ui.add_space(4.0);

                                egui::Grid::new("texture_editor_info_grid")
                                    .num_columns(2)
                                    .spacing([16.0, 6.0])
                                    .show(ui, |ui| {
                                        ui.label("Dimensions:");
                                        ui.label(format!("{} x {}", width, height));
                                        ui.end_row();

                                        ui.label("Format:");
                                        ui.label(format_str);
                                        ui.end_row();

                                        ui.label("Mip Levels:");
                                        ui.label(mips.to_string());
                                        ui.end_row();

                                        ui.label("Array Count:");
                                        ui.label(arrays.to_string());
                                        ui.end_row();
                                    });

                                ui.add_space(8.0);
                                ui.separator();
                                ui.add_space(4.0);

                                let aspect = width as f32 / (height.max(1) as f32);

                                let avail_width = ui.available_width().max(64.0);
                                let preview_height = avail_width / aspect;

                                let size = egui::vec2(avail_width, preview_height);

                                let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());

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
