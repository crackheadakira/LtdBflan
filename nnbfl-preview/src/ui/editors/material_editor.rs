use nnbfl::{
    bflyt::list::{
        BlendFactor, BlendOp, CombinerTevMode, DetailedCombinerAlphaStageConfig,
        DetailedCombinerColorStageConfig, DetailedCombinerStageMode, LogicOp, Material,
        MaterialBlendMode, MaterialList, MaterialTextureOptions, TevAlphaOp, TevColorOp,
        TevKonstSel, TevScale, TevSource,
    },
    ui2d::types::{Color4f, Color4u8, Vector2f},
};

use crate::ui::{DrawUi, DrawUiWith};

#[derive(Default)]
pub struct MaterialEditor {
    pub selected_material: usize,
    pub pending_upload: bool,
    pub is_editor_visible: bool,
}

impl DrawUiWith<&mut MaterialList> for MaterialEditor {
    fn draw_with(&mut self, ui: &mut egui::Ui, material_list: &mut MaterialList) -> bool {
        let mut changed = false;

        egui::Window::new("Material Editor")
            .collapsible(false)
            .resizable(true)
            .open(&mut self.is_editor_visible)
            .show(ui, |ui| {
                egui::Panel::left("mat_editor_list")
                    .resizable(true)
                    .min_size(144.0)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink(false)
                            .show(ui, |ui| {
                                ui.weak(format!(
                                    "{} Total Materials",
                                    material_list.materials.len()
                                ));

                                for (idx, material) in material_list.materials.iter().enumerate() {
                                    let text = format!("[{}] {}", idx + 1, material.material_name);
                                    let is_selected = self.selected_material == idx;

                                    if ui.selectable_label(is_selected, text).clicked() {
                                        self.selected_material = idx;
                                    }
                                }
                            })
                    });

                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        egui::Frame::new()
                            .inner_margin(egui::Margin {
                                left: 12,
                                right: 4,
                                top: 4,
                                bottom: 4,
                            })
                            .show(ui, |ui| {
                                if let Some(material) =
                                    material_list.materials.get_mut(self.selected_material)
                                {
                                    changed |= material.draw(ui)
                                }
                            });
                    })
            });

        changed
    }
}

impl DrawUi for Material {
    fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        changed |= ui
            .checkbox(&mut self.use_texture_only, "Use Texture Only")
            .changed();

        changed |= ui
            .checkbox(
                &mut self.use_thresholding_alpha_interpolation,
                "Use Thresholding Alpha Interpolation",
            )
            .changed();

        if !self.colors.is_empty() {
            ui.heading("Material Colors");
        }

        for (idx, color) in self.colors.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                let color_label = &format!("Color {}:", idx + 1);

                if let Some(ref mut color) = color.color_f32 {
                    changed |= color.draw_with(ui, color_label)
                } else if let Some(ref mut color) = color.color_u8 {
                    changed |= color.draw_with(ui, color_label)
                }
            });
        }

        if !self.tex_maps.is_empty() {
            ui.separator();
            ui.heading("Texture Maps");
        }

        for (idx, tex_map) in self.tex_maps.iter_mut().enumerate() {
            ui.label(&tex_map.texture_name);

            ui.horizontal(|ui| {
                changed |= tex_map.u_options.draw_with(ui, ("U", idx));
                changed |= tex_map.v_options.draw_with(ui, ("V", idx));
            });

            if let Some(tex_ext) = self.tex_extensions.get_mut(idx) {
                ui.horizontal(|ui| {
                    changed |= ui
                        .checkbox(
                            &mut tex_ext.is_capture_texture,
                            format!("Is Texture {} Capture", idx + 1),
                        )
                        .clicked();

                    changed |= ui
                        .checkbox(
                            &mut tex_ext.is_vector_texture,
                            format!("Is Texture {} Vector", idx + 1),
                        )
                        .clicked();
                });
            }

            ui.add_space(12.0);
        }

        if !self.tex_srts.is_empty() {
            ui.separator();
            ui.heading("Texture SRTs");

            ui.horizontal_wrapped(|ui| {
                for (idx, tex_srt) in self.tex_srts.iter_mut().enumerate() {
                    ui.vertical(|ui| {
                        ui.weak(format!("Texture {}", idx + 1));

                        egui::Grid::new(format!("tex_srt_grid_{idx}"))
                            .num_columns(2)
                            .spacing([12.0, 6.0])
                            .show(ui, |ui| {
                                ui.label("Rotate");
                                changed |= ui
                                    .add(egui::DragValue::new(&mut tex_srt.rotate).speed(0.1))
                                    .changed();
                                ui.end_row();

                                ui.label("Translate U");
                                changed |= ui
                                    .add(egui::DragValue::new(&mut tex_srt.translate_u).speed(0.5))
                                    .changed();
                                ui.end_row();

                                ui.label("Translate V");
                                changed |= ui
                                    .add(egui::DragValue::new(&mut tex_srt.translate_v).speed(0.5))
                                    .changed();
                                ui.end_row();

                                ui.label("Scale U");
                                changed |= ui
                                    .add(egui::DragValue::new(&mut tex_srt.scale_u).speed(0.01))
                                    .changed();
                                ui.end_row();

                                ui.label("Scale V");
                                changed |= ui
                                    .add(egui::DragValue::new(&mut tex_srt.scale_v).speed(0.01))
                                    .changed();
                                ui.end_row();
                            });
                    });

                    ui.add_space(24.0);
                }
            });
        }

        if !self.tex_coord_gens.is_empty() {
            ui.separator();
            ui.heading("Texture Coordinate Generators");
        }

        for (idx, tex_coord_gen) in self.tex_coord_gens.iter_mut().enumerate() {
            egui::ComboBox::new(
                format!("tex_coord_gen_{idx}"),
                format!("Texture {} Source", idx + 1),
            )
            .selected_text(format!("{:?}", tex_coord_gen.tex_gen_source))
            .show_ui(ui, |ui| {
                use nnbfl::bflyt::list::TexGenSrc;

                changed |= ui
                    .selectable_value(
                        &mut tex_coord_gen.tex_gen_source,
                        TexGenSrc::Tex0,
                        "Texture 0",
                    )
                    .clicked();

                changed |= ui
                    .selectable_value(
                        &mut tex_coord_gen.tex_gen_source,
                        TexGenSrc::Tex1,
                        "Texture 1",
                    )
                    .clicked();

                changed |= ui
                    .selectable_value(
                        &mut tex_coord_gen.tex_gen_source,
                        TexGenSrc::Tex2,
                        "Texture 2",
                    )
                    .clicked();

                changed |= ui
                    .selectable_value(
                        &mut tex_coord_gen.tex_gen_source,
                        TexGenSrc::OrthogonalProjection,
                        "Orthogonal Projection",
                    )
                    .clicked();

                changed |= ui
                    .selectable_value(
                        &mut tex_coord_gen.tex_gen_source,
                        TexGenSrc::PaneBasedOrthogonalProjection,
                        "Pane Based Orthogonal Projection",
                    )
                    .clicked();

                changed |= ui
                    .selectable_value(
                        &mut tex_coord_gen.tex_gen_source,
                        TexGenSrc::PaneBasedPerspectiveProjection,
                        "Pane Based Perspective Projection",
                    )
                    .clicked();

                changed |= ui
                    .selectable_value(
                        &mut tex_coord_gen.tex_gen_source,
                        TexGenSrc::PerspectiveProjection,
                        "Perspective Projection",
                    )
                    .clicked();

                changed |= ui
                    .selectable_value(
                        &mut tex_coord_gen.tex_gen_source,
                        TexGenSrc::BrickRepeat,
                        "Brick Repeat",
                    )
                    .clicked();
            });

            ui.add_space(12.0);
        }

        if !self.projection_tex_gens.is_empty() {
            ui.separator();
            ui.heading("Projection Texture Generators");
        }

        for (idx, proj_tex_gen) in self.projection_tex_gens.iter_mut().enumerate() {
            ui.weak(format!("Texture {}", idx + 1));

            changed |= ui
                .checkbox(
                    &mut proj_tex_gen.flags.fitting_layout_size,
                    "Fitting Layout Size",
                )
                .changed();

            changed |= ui
                .checkbox(
                    &mut proj_tex_gen.flags.fitting_pane_size,
                    "Fitting Pane Size",
                )
                .changed();

            changed |= ui
                .checkbox(
                    &mut proj_tex_gen.flags.adjust_projection_scale_rotate,
                    "Adjust Projection, Scale & Rotate",
                )
                .changed();

            proj_tex_gen.translation.draw_with(ui, "Translation");

            ui.add_space(4.0);

            proj_tex_gen.scale.draw_with(ui, "Scale");

            ui.add_space(12.0);
        }

        if !self.tev_combiners.is_empty() {
            ui.separator();
            ui.heading("Texture Environment Unit Combiners");
        }

        for (idx, tev_combiner) in self.tev_combiners.iter_mut().enumerate() {
            ui.weak(format!("Combiner {}", idx + 1));

            tev_combiner.alpha_mode.draw_with(ui, ("Alpha Mode", idx));
            tev_combiner.rgb_mode.draw_with(ui, ("RGB Mode", idx));

            ui.add_space(12.0);
        }

        if self.alpha_compare.is_some() {
            ui.separator();
            ui.heading("Alpha Compare");
        }

        if let Some(alpha_compare) = self.alpha_compare.as_mut() {
            egui::ComboBox::new("mat_editor_alpha_compare_op", "Compare Operand")
                .selected_text(format!("{alpha_compare:?}"))
                .show_ui(ui, |ui| {
                    use nnbfl::bflyt::list::AlphaCompare;

                    ui.label("Reference Value");
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut alpha_compare.alpha_compare_ref_value)
                                .speed(0.5),
                        )
                        .changed();

                    changed |= ui
                        .selectable_value(&mut alpha_compare.compare, AlphaCompare::Never, "Never")
                        .clicked();

                    changed |= ui
                        .selectable_value(&mut alpha_compare.compare, AlphaCompare::Less, "Less")
                        .clicked();

                    changed |= ui
                        .selectable_value(
                            &mut alpha_compare.compare,
                            AlphaCompare::LessThanEqual,
                            "Less Than Equal",
                        )
                        .clicked();

                    changed |= ui
                        .selectable_value(&mut alpha_compare.compare, AlphaCompare::Equal, "Equal")
                        .clicked();

                    changed |= ui
                        .selectable_value(
                            &mut alpha_compare.compare,
                            AlphaCompare::NeverEqual,
                            "Never Equal",
                        )
                        .clicked();

                    changed |= ui
                        .selectable_value(
                            &mut alpha_compare.compare,
                            AlphaCompare::GreaterThanEqual,
                            "Greater Than Equal",
                        )
                        .clicked();

                    changed |= ui
                        .selectable_value(
                            &mut alpha_compare.compare,
                            AlphaCompare::Greater,
                            "Greater",
                        )
                        .clicked();

                    changed |= ui
                        .selectable_value(
                            &mut alpha_compare.compare,
                            AlphaCompare::Always,
                            "Always",
                        )
                        .clicked();
                });
        }

        if self.blend_mode.is_some() {
            ui.separator();
            ui.heading("Blend Mode");
        }

        if let Some(blend_mode) = self.blend_mode.as_mut() {
            changed |= blend_mode.draw_with(ui, "blend_mode");
        }

        if self.blend_mode_alpha.is_some() {
            ui.separator();
            ui.heading("Alpha Blend Mode");
        }

        if let Some(blend_mode_alpha) = self.blend_mode_alpha.as_mut() {
            changed |= blend_mode_alpha.draw_with(ui, "blend_mode_alpha");
        }

        if self.indirect_matrix.is_some() {
            ui.separator();
            ui.heading("Indirect Matrix");
        }

        if let Some(indirect_matrix) = self.indirect_matrix.as_mut() {
            ui.label("Rotation");
            changed |= ui
                .add(egui::DragValue::new(&mut indirect_matrix.rotation).speed(0.5))
                .changed();

            changed |= indirect_matrix.scale.draw_with(ui, "Scale");
        }

        if self.font_shadow_color.is_some() {
            ui.separator();
            ui.heading("Font Shadow Color");
        }

        if let Some(font_shadow_color) = self.font_shadow_color.as_mut() {
            changed |= font_shadow_color.black_color.draw_with(ui, "Black Color");
            changed |= font_shadow_color.white_color.draw_with(ui, "White Color");
        }

        if self.detailed_combiner.is_some() {
            ui.separator();
            ui.heading("Detailed Combiner");
        }

        if let Some(detailed_combiner) = self.detailed_combiner.as_mut() {
            changed |= detailed_combiner.color1.draw_with(ui, "Color 1");
            changed |= detailed_combiner.color2.draw_with(ui, "Color 2");
            changed |= detailed_combiner.color3.draw_with(ui, "Color 3");
            changed |= detailed_combiner.color4.draw_with(ui, "Color 4");
            changed |= detailed_combiner.color5.draw_with(ui, "Color 5");

            ui.horizontal_wrapped(|ui| {
                for (i, entry) in detailed_combiner.entries.iter_mut().enumerate() {
                    ui.vertical(|ui| {
                        ui.set_min_width(150.0);

                        ui.weak(format!("Combiner {}", i + 1));

                        ui.label("Alpha Config");
                        changed |= entry.alpha_config.draw(ui);

                        ui.add_space(12.0);

                        ui.label("Color Config");
                        changed |= entry.color_config.draw(ui);
                    });

                    ui.add_space(24.0);
                }
            });
        }

        changed
    }
}

impl DrawUiWith<&str> for Color4f {
    fn draw_with(&mut self, ui: &mut egui::Ui, label: &str) -> bool {
        let mut changed = false;

        ui.horizontal(|ui| {
            ui.label(label);

            let mut color_array = (*self).into();

            if ui
                .color_edit_button_rgba_unmultiplied(&mut color_array)
                .changed()
            {
                *self = color_array.into();
                changed = true;
            }
        });

        changed
    }
}

impl DrawUiWith<&str> for Color4u8 {
    fn draw_with(&mut self, ui: &mut egui::Ui, label: &str) -> bool {
        let mut changed = false;

        ui.horizontal(|ui| {
            ui.label(label);

            let mut color_array = (*self).into();

            if ui
                .color_edit_button_rgba_unmultiplied(&mut color_array)
                .changed()
            {
                *self = color_array.into();
                changed = true;
            }
        });

        changed
    }
}

impl DrawUiWith<&str> for Vector2f {
    fn draw_with(&mut self, ui: &mut egui::Ui, label: &str) -> bool {
        let mut changed = false;

        ui.label(label);
        ui.horizontal(|ui| {
            ui.weak("X");
            changed |= ui
                .add(egui::DragValue::new(&mut self.x).speed(0.1))
                .changed();

            ui.add_space(4.0);

            ui.weak("Y");
            changed |= ui
                .add(egui::DragValue::new(&mut self.y).speed(0.1))
                .changed();
        });

        changed
    }
}

impl DrawUiWith<(&str, usize)> for MaterialTextureOptions {
    fn draw_with(&mut self, ui: &mut egui::Ui, state: (&str, usize)) -> bool {
        let (prefix, idx) = state;
        let mut changed = false;

        ui.vertical(|ui| {
            {
                let combo_id = format!("tex_map_{}_filter_{idx}", prefix.to_lowercase());
                let combo_label = format!("{prefix} Filter");

                egui::ComboBox::new(combo_id, combo_label)
                    .selected_text(format!("{:?}", self.filter))
                    .show_ui(ui, |ui| {
                        use nnbfl::bflyt::flags::TexFilter;

                        changed |= ui
                            .selectable_value(&mut self.filter, TexFilter::Linear, "Linear")
                            .clicked();

                        changed |= ui
                            .selectable_value(&mut self.filter, TexFilter::Near, "Near")
                            .clicked();
                    });
            }

            {
                let combo_id = format!("tex_map_{}_wrap_mode_{idx}", prefix.to_lowercase());
                let combo_label = format!("{prefix} Wrap Mode");

                egui::ComboBox::new(combo_id, combo_label)
                    .selected_text(format!("{:?}", self.wrap_mode))
                    .show_ui(ui, |ui| {
                        use nnbfl::bflyt::flags::TexWrapMode;

                        changed |= ui
                            .selectable_value(&mut self.wrap_mode, TexWrapMode::Clamp, "Clamp")
                            .clicked();

                        changed |= ui
                            .selectable_value(&mut self.wrap_mode, TexWrapMode::Repeat, "Repeat")
                            .clicked();

                        changed |= ui
                            .selectable_value(&mut self.wrap_mode, TexWrapMode::Mirror, "Mirror")
                            .clicked();
                    });
            }
        });

        changed
    }
}

impl DrawUiWith<(&str, usize)> for CombinerTevMode {
    fn draw_with(&mut self, ui: &mut egui::Ui, state: (&str, usize)) -> bool {
        let (prefix, idx) = state;

        let mut changed = false;

        let combo_id = format!("{}_combiner_tev_mode_{idx}", prefix.to_lowercase());

        egui::ComboBox::new(combo_id, prefix)
            .selected_text(format!("{self:?}"))
            .show_ui(ui, |ui| {
                changed |= ui
                    .selectable_value(self, Self::Replace, "Replace")
                    .clicked();

                changed |= ui
                    .selectable_value(self, Self::Modulate, "Modulate")
                    .clicked();

                changed |= ui.selectable_value(self, Self::Add, "Add").clicked();
                changed |= ui
                    .selectable_value(self, Self::AddSigned, "Add Signed")
                    .clicked();

                changed |= ui
                    .selectable_value(self, Self::Interpolate, "Interpolate")
                    .clicked();

                changed |= ui
                    .selectable_value(self, Self::Subtract, "Subtract")
                    .clicked();

                changed |= ui
                    .selectable_value(self, Self::AddMultiply, "Add Multiply")
                    .clicked();

                changed |= ui
                    .selectable_value(self, Self::MultiplyAdd, "Multiply Add")
                    .clicked();

                changed |= ui
                    .selectable_value(self, Self::Overlay, "Overlay")
                    .clicked();

                changed |= ui
                    .selectable_value(self, Self::Lighten, "Lighten")
                    .clicked();

                changed |= ui.selectable_value(self, Self::Darken, "Darken").clicked();

                changed |= ui
                    .selectable_value(self, Self::Indirect, "Indirect")
                    .clicked();

                changed |= ui
                    .selectable_value(self, Self::BlendIndirect, "Blend Indirect")
                    .clicked();

                changed |= ui
                    .selectable_value(self, Self::EachIndirect, "Each Indirect")
                    .clicked();
            });

        changed
    }
}

impl DrawUiWith<&str> for MaterialBlendMode {
    fn draw_with(&mut self, ui: &mut egui::Ui, label: &str) -> bool {
        let mut changed = false;

        let current_variant_name = match self {
            MaterialBlendMode::None => "None",
            MaterialBlendMode::Blend { .. } => "Blend",
            MaterialBlendMode::Logic { .. } => "Logic",
        };

        ui.horizontal(|ui| {
            ui.label("Blend Mode:");

            egui::ComboBox::from_id_salt(format!("material_blend_mode_select_{label}"))
                .selected_text(current_variant_name)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(current_variant_name == "None", "None")
                        .clicked()
                    {
                        *self = MaterialBlendMode::None;

                        changed = true;
                    }

                    if ui
                        .selectable_label(current_variant_name == "Blend", "Blend")
                        .clicked()
                    {
                        *self = MaterialBlendMode::Blend {
                            blend_op: BlendOp::Add,
                            function_source: BlendFactor::V0,
                            function_destination: BlendFactor::V0,
                        };

                        changed = true;
                    }

                    if ui
                        .selectable_label(current_variant_name == "Logic", "Logic")
                        .clicked()
                    {
                        *self = MaterialBlendMode::Logic {
                            logic_op: LogicOp::NoOp,
                        };

                        changed = true;
                    }
                });
        });

        match self {
            MaterialBlendMode::None => {}
            MaterialBlendMode::Logic { logic_op } => {
                ui.indent(format!("logic_properties_{label}"), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Logic OP:");

                        egui::ComboBox::from_id_salt(format!("logic_op_select_{label}"))
                            .selected_text(format!("{logic_op:?}"))
                            .show_ui(ui, |ui| {
                                let ops = [
                                    LogicOp::NoOp,
                                    LogicOp::Clear,
                                    LogicOp::Set,
                                    LogicOp::Copy,
                                    LogicOp::InvCopy,
                                    LogicOp::Inv,
                                    LogicOp::And,
                                    LogicOp::Nand,
                                    LogicOp::Or,
                                    LogicOp::Nor,
                                    LogicOp::Xor,
                                    LogicOp::Equiv,
                                    LogicOp::RevAnd,
                                    LogicOp::InvAnd,
                                    LogicOp::RevOr,
                                    LogicOp::InvOr,
                                ];

                                for op in ops {
                                    changed |= ui
                                        .selectable_value(logic_op, op, format!("{op:?}"))
                                        .clicked();
                                }
                            });
                    });
                });
            }
            MaterialBlendMode::Blend {
                blend_op,
                function_source,
                function_destination,
            } => {
                ui.indent(format!("blend_properties_{label}"), |ui| {
                    egui::Grid::new(format!("blend_mode_grid_{label}"))
                        .num_columns(2)
                        .spacing([12.0, 6.0])
                        .show(ui, |ui| {
                            ui.label("Blend Operand:");
                            egui::ComboBox::from_id_salt(format!("blend_op_select_{label}"))
                                .selected_text(format!("{blend_op:?}"))
                                .show_ui(ui, |ui| {
                                    let ops = [
                                        BlendOp::Add,
                                        BlendOp::Subtract,
                                        BlendOp::ReverseSubtract,
                                        BlendOp::SelectMin,
                                        BlendOp::SelectMax,
                                    ];

                                    for op in ops {
                                        changed |= ui
                                            .selectable_value(blend_op, op, format!("{op:?}"))
                                            .clicked();
                                    }
                                });
                            ui.end_row();

                            ui.label("Source Factor:");
                            changed |= function_source.draw(ui);
                            ui.end_row();

                            ui.label("Destination Factor:");
                            changed |= function_destination.draw(ui);
                            ui.end_row();
                        });
                });
            }
        }

        changed
    }
}

impl DrawUi for BlendFactor {
    fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::ComboBox::from_id_salt(ui.next_auto_id())
            .selected_text(format!("{self:?}"))
            .show_ui(ui, |ui| {
                changed |= ui.selectable_value(self, Self::V0, "V0").clicked();
                changed |= ui.selectable_value(self, Self::V1_0, "V1_0").clicked();

                changed |= ui
                    .selectable_value(self, Self::SrcColor, "Source Color")
                    .clicked();

                changed |= ui
                    .selectable_value(self, Self::DstColor, "DST Color")
                    .clicked();

                changed |= ui
                    .selectable_value(self, Self::SrcAlpha, "Source Alpha")
                    .clicked();

                changed |= ui
                    .selectable_value(self, Self::DstAlpha, "DST Alpha")
                    .clicked();

                changed |= ui
                    .selectable_value(self, Self::InvSrcColor, "Inverse Source Color")
                    .clicked();

                changed |= ui
                    .selectable_value(self, Self::InvDstColor, "Inverse DST Color")
                    .clicked();

                changed |= ui
                    .selectable_value(self, Self::InvSrcAlpha, "Inverse Source Alpha")
                    .clicked();

                changed |= ui
                    .selectable_value(self, Self::InvDstAlpha, "Inverse DST Alpha")
                    .clicked();
            });

        changed
    }
}

impl DrawUi for TevSource {
    fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::ComboBox::from_id_salt(ui.next_auto_id())
            .selected_text(format!("{self:?}"))
            .show_ui(ui, |ui| {
                let sources = [
                    Self::Primary,
                    Self::Texture0,
                    Self::Texture1,
                    Self::Texture2,
                    Self::Texture3,
                    Self::Register,
                    Self::Constant,
                    Self::Previous,
                ];

                for source in sources {
                    changed |= ui
                        .selectable_value(self, source, format!("{source:?}"))
                        .clicked();
                }
            });

        changed
    }
}

impl DrawUi for DetailedCombinerStageMode {
    fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::ComboBox::from_id_salt(ui.next_auto_id())
            .selected_text(format!("{self:?}"))
            .show_ui(ui, |ui| {
                let stages = [
                    Self::Replace,
                    Self::Modulate,
                    Self::Add,
                    Self::AddSigned,
                    Self::Interpolate,
                    Self::Subtract,
                    Self::AddMult,
                    Self::MultiplicateAdd,
                ];

                for stage in stages {
                    changed |= ui
                        .selectable_value(self, stage, format!("{stage:?}"))
                        .clicked();
                }
            });

        changed
    }
}

impl DrawUi for TevScale {
    fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::ComboBox::from_id_salt(ui.next_auto_id())
            .selected_text(format!("{self:?}"))
            .show_ui(ui, |ui| {
                let scales = [Self::V1, Self::V2, Self::V4];

                for scale in scales {
                    changed |= ui
                        .selectable_value(self, scale, format!("{scale:?}"))
                        .clicked();
                }
            });

        changed
    }
}

impl DrawUi for TevKonstSel {
    fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::ComboBox::from_id_salt(ui.next_auto_id())
            .selected_text(format!("{self:?}"))
            .show_ui(ui, |ui| {
                let konsts = [
                    Self::BlackColor,
                    Self::WhiteColor,
                    Self::K0,
                    Self::K1,
                    Self::K2,
                    Self::K3,
                    Self::K4,
                ];

                for konst in konsts {
                    changed |= ui
                        .selectable_value(self, konst, format!("{konst:?}"))
                        .clicked();
                }
            });

        changed
    }
}

impl DrawUi for TevColorOp {
    fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::ComboBox::from_id_salt(ui.next_auto_id())
            .selected_text(format!("{self:?}"))
            .show_ui(ui, |ui| {
                let ops = [
                    Self::Rgb,
                    Self::InvRgb,
                    Self::Alpha,
                    Self::InvAlpha,
                    Self::Rrr,
                    Self::InvRrr,
                    Self::Ggg,
                    Self::InvGgg,
                    Self::Bbb,
                    Self::InvBbb,
                ];

                for op in ops {
                    changed |= ui.selectable_value(self, op, format!("{op:?}")).clicked();
                }
            });

        changed
    }
}

impl DrawUi for TevAlphaOp {
    fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::ComboBox::from_id_salt(ui.next_auto_id())
            .selected_text(format!("{self:?}"))
            .show_ui(ui, |ui| {
                let ops = [
                    Self::Alpha,
                    Self::InvAlpha,
                    Self::R,
                    Self::InvR,
                    Self::G,
                    Self::InvG,
                    Self::B,
                    Self::InvB,
                ];

                for op in ops {
                    changed |= ui.selectable_value(self, op, format!("{op:?}")).clicked();
                }
            });

        changed
    }
}

impl DrawUi for DetailedCombinerColorStageConfig {
    fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::Grid::new(ui.next_auto_id())
            .num_columns(2)
            .spacing([16.0, 8.0])
            .show(ui, |ui| {
                ui.label("Stage Mode:");
                changed |= self.mode.draw(ui);
                ui.end_row();

                ui.label("Scale:");
                changed |= self.scale.draw(ui);
                ui.end_row();

                ui.label("Konst Sel:");
                changed |= self.konst_sel.draw(ui);
                ui.end_row();

                for (i, source) in self.sources.iter_mut().enumerate() {
                    ui.label(format!("Source {}:", i + 1));
                    changed |= source.draw(ui);
                    ui.end_row();
                }

                for (i, operand) in self.operands.iter_mut().enumerate() {
                    ui.label(format!("Color Operand {}:", i + 1));
                    changed |= operand.draw(ui);
                    ui.end_row();
                }
            });

        changed |= ui.checkbox(&mut self.copy_reg, "Copy Reg").changed();

        changed
    }
}

impl DrawUi for DetailedCombinerAlphaStageConfig {
    fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::Grid::new(ui.next_auto_id())
            .num_columns(2)
            .spacing([16.0, 8.0])
            .show(ui, |ui| {
                ui.label("Stage Mode:");
                changed |= self.mode.draw(ui);
                ui.end_row();

                ui.label("Scale:");
                changed |= self.scale.draw(ui);
                ui.end_row();

                ui.label("Konst Sel:");
                changed |= self.konst_sel.draw(ui);
                ui.end_row();

                for (i, source) in self.sources.iter_mut().enumerate() {
                    ui.label(format!("Source {}:", i + 1));
                    changed |= source.draw(ui);
                    ui.end_row();
                }

                for (i, operand) in self.operands.iter_mut().enumerate() {
                    ui.label(format!("Color Operand {}:", i + 1));
                    changed |= operand.draw(ui);
                    ui.end_row();
                }
            });

        changed |= ui.checkbox(&mut self.copy_reg, "Copy Reg").changed();

        changed
    }
}
