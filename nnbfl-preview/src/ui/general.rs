use std::collections::HashSet;

use egui::Ui;
use nnbfl::{
    bflyt::{file::BflytSection, list::Layout, pane::Pane},
    ui2d::types::{Vector2f, Vector3f},
};

use crate::{
    RenderContext,
    bflyt_view::BflytView,
    pane_tree::DirtyFlags,
    traits::Displaying,
    ui::{
        DrawUi, DrawUiWith,
        archive_browser::ArchiveBrowser,
        context_menu::{ContextMenu, ContextMenuAction, ContextMenuState},
        editors::{DetailedPaneEditor, GroupEditor, MaterialEditor},
        shortcuts::Shortcuts,
        timeline::TimelineState,
    },
};

pub const SUPPORTED_SARC_EXTENSIONS: &[&str] = &[
    "blarc",
    "sarc",
    "Nin_NX_NVN",
    "blarc.zs",
    "sarc.zs",
    "Nin_NX_NVN.zs",
];

#[derive(Default)]
pub struct UiState {
    pub selected_pane: Option<usize>,
    pub hidden_panes: HashSet<usize>,
    pub error_message: Option<String>,
    pub pending_action: Option<UiAction>,
    pub visiblity_flags: PaneVisibilityFlags,
    pub anim_names: Vec<String>,
    pub pending_play_anim: Option<String>,
    pub sidebar_tab: SidebarTab,
    pub right_sidebar_tab: SidebarRightTab,
    pub active_debug_stage: u32,

    pub context_menu: ContextMenu,
    pub archive_browser: ArchiveBrowser,
    pub shortcuts_window: Shortcuts,

    pub timeline: TimelineState,
    pub material_editor: MaterialEditor,
    pub detailed_pane_editor: DetailedPaneEditor,
    pub group_editor: GroupEditor,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PaneVisibilityFlags {
    /// Clips panes to fit within root pane.
    pub clip_to_root: bool,

    /// Hide every plain (non-texture) quad.
    pub only_textured: bool,

    /// Hide every textured quad (draw only outlines).
    pub no_textured: bool,

    /// Show the plain outline on top of panes that have a texture.
    pub quad_for_textured: bool,
}

impl PaneVisibilityFlags {
    pub fn plain_color(&self, q: &crate::renderer::quad::Quad, hidden: bool) -> [f32; 4] {
        if hidden || self.only_textured || (q.has_textured && !self.quad_for_textured) {
            [0.0; 4]
        } else {
            q.color
        }
    }

    pub fn textured_tint(
        &self,
        tq: &crate::renderer::textured_quad::TexturedQuad,
        hidden: bool,
    ) -> [f32; 4] {
        if hidden || self.no_textured {
            [0.0; 4]
        } else {
            tq.tint
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SidebarTab {
    #[default]
    Panes,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SidebarRightTab {
    #[default]
    Properties,
    Animations,
}

pub enum UiAction {
    LoadFile,
    SetBlarcDir,
    StartArchiveScan,
    CancelArchiveScan,
    LoadArchiveEntry(crate::archive_browser::ArchiveEntry),
    SaveFile,

    DeletePane(usize),
    DuplicatePane(usize),
    Undo,
    Redo,
}

pub fn draw_ui(ui: &mut Ui, ctx: &mut RenderContext<'_>, screen_w: f32, screen_h: f32) {
    crate::keybinds::handle(ui.ctx(), ctx.ui_state);

    if let Some(ref mut view) = ctx.bflyt_view {
        let viewport_rect = ui.content_rect();
        let painter = ui.painter().with_clip_rect(viewport_rect);

        for node in view.tree.iter() {
            let i = node.pane_idx;

            if let BflytSection::TextBoxPane(text_box) = &node.section
                && !ctx.ui_state.hidden_panes.contains(&i)
                && node.visible
            {
                let display_text = text_box.text.as_deref().unwrap_or("");

                let tl = node.plain_quad.corners[0];
                let br = node.plain_quad.corners[3];

                let center_x = (tl[0] + br[0]) * 0.5;
                let center_y = (tl[1] + br[1]) * 0.5;
                let screen_pos =
                    ctx.camera
                        .world_to_screen([center_x, center_y], screen_w, screen_h);

                let font_size = (32.0 * ctx.camera.zoom).clamp(8.0, 128.0);
                let font_id = egui::FontId::proportional(font_size);

                let shadow_offset = (font_size * 0.08).max(1.5);
                let shadow_pos =
                    egui::pos2(screen_pos.x + shadow_offset, screen_pos.y + shadow_offset);

                painter.text(
                    shadow_pos,
                    egui::Align2::CENTER_CENTER,
                    display_text,
                    font_id.clone(),
                    egui::Color32::from_black_alpha(220),
                );

                painter.text(
                    screen_pos,
                    egui::Align2::CENTER_CENTER,
                    display_text,
                    font_id,
                    egui::Color32::WHITE,
                );
            }
        }
    }

    if let Some(bflyt_view) = &ctx.bflyt_view
        && let Some(node) = bflyt_view
            .tree
            .find_by_idx(ctx.ui_state.context_menu.pane_idx)
    {
        let state = ContextMenuState {
            node,
            is_hidden: ctx
                .ui_state
                .hidden_panes
                .contains(&ctx.ui_state.context_menu.pane_idx),
        };

        if let Some(action) = ctx.ui_state.context_menu.draw_with(ui, state) {
            match action {
                ContextMenuAction::HidePane(pane_idx) => {
                    ctx.ui_state.hidden_panes.insert(pane_idx);
                }
                ContextMenuAction::ShowPane(pane_idx) => {
                    ctx.ui_state.hidden_panes.remove(&pane_idx);
                }
                ContextMenuAction::Action(ui_action) => {
                    ctx.ui_state.pending_action = Some(ui_action);
                }
            };
        }
    };

    egui::Panel::top("menu_bar").show(ui, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Load File...").clicked() {
                    ctx.ui_state.pending_action = Some(UiAction::LoadFile);
                    ctx.ui_state.hidden_panes.clear();
                    ctx.ui_state.selected_pane = None;

                    ui.close();
                }

                if ui.button("Set Layout Folder...").clicked() {
                    ctx.ui_state.pending_action = Some(UiAction::SetBlarcDir);
                    ui.close();
                }

                if ui.button("Save File As...").clicked() {
                    ctx.ui_state.pending_action = Some(UiAction::SaveFile);
                    ui.close();
                }
            });

            if ui.button("Browse Archives...").clicked() {
                ctx.ui_state.archive_browser.is_visible = true;
            }

            if let Some(ref view) = ctx.bflyt_view {
                ui.menu_button("Shader Pass", |ui| {
                    let stages = [
                        (0, "Disabled"),
                        (1, "1. Layer 0 Raw Texture"),
                        (2, "2. Layer 1 Raw Texture"),
                        (3, "3. Layer 2 Raw Texture"),
                        (4, "4. Post-Texture Combiner Blend"),
                        (5, "5. Indirect Raw Vector Offset"),
                        (6, "6. Indirect Displaced UV Coordinates"),
                        (7, "7. Indirect Isolated Sample Output"),
                        (8, "8. Composite Layer Alpha (Grayscale)"),
                    ];

                    for (stage_idx, label) in stages {
                        if ui
                            .radio_value(&mut ctx.ui_state.active_debug_stage, stage_idx, label)
                            .clicked()
                        {
                            ui.close();
                        }
                    }
                });

                if ui.button("Group Editor").clicked() {
                    ctx.ui_state.group_editor.is_editor_visible = true;
                }

                if view.tree.material_list.is_some() && ui.button("Material Editor").clicked() {
                    ctx.ui_state.material_editor.is_editor_visible = true;
                }

                if ctx.ui_state.selected_pane.is_some()
                    && ui.button("Detailed Pane Editor").clicked()
                {
                    ctx.ui_state.detailed_pane_editor.is_editor_visible = true;
                };
            }

            ui.menu_button("Help", |ui| {
                if ui.button("Keyboard Shortcuts").clicked() {
                    ctx.ui_state.shortcuts_window.is_visible = true;
                    ui.close();
                }
            });
        })
    });

    egui::Panel::left("pane_tree")
        .default_size(240.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut ctx.ui_state.sidebar_tab,
                    SidebarTab::Panes,
                    "Pane Tree",
                );
            });
            ui.separator();

            match ctx.ui_state.sidebar_tab {
                SidebarTab::Panes => {
                    ui.heading("Pane Tree");
                    ui.checkbox(
                        &mut ctx.ui_state.visiblity_flags.clip_to_root,
                        "Clip to root pane",
                    );
                    ui.checkbox(
                        &mut ctx.ui_state.visiblity_flags.only_textured,
                        "Draw only textures",
                    );
                    ui.checkbox(
                        &mut ctx.ui_state.visiblity_flags.quad_for_textured,
                        "Draw pane outlines for textures",
                    );
                    ui.checkbox(
                        &mut ctx.ui_state.visiblity_flags.no_textured,
                        "Draw only pane outlines",
                    );
                    ui.separator();

                    let total_rows = ctx
                        .bflyt_view
                        .as_ref()
                        .map_or(1, |v| v.tree.flatten().len());
                    egui::ScrollArea::vertical().auto_shrink(false).show_rows(
                        ui,
                        24.0,
                        total_rows,
                        |ui, row_range| {
                            if let Some(ref view) = ctx.bflyt_view {
                                let nodes = view.tree.flatten();

                                for idx in row_range {
                                    let node = nodes[idx];
                                    let i = node.pane_idx;

                                    let indent = node.depth as f32 * 12.0;
                                    ui.horizontal(|ui| {
                                        ui.add_space(indent);

                                        let selected = ctx.ui_state.selected_pane == Some(i);
                                        let is_parts_content = node.parts_source.is_some();

                                        let label_text = if is_parts_content {
                                            format!("[{}] {} (linked)", node.kind, node.label)
                                        } else {
                                            format!("[{}] {}", node.kind, node.label)
                                        };
                                        let label = if is_parts_content {
                                            egui::RichText::new(label_text).weak()
                                        } else {
                                            egui::RichText::new(label_text)
                                        };

                                        let is_hidden = ctx.ui_state.hidden_panes.contains(&i);

                                        let response = ui.selectable_label(selected, label);
                                        response.context_menu(|ui| {
                                            if !is_hidden && ui.button("Hide").clicked() {
                                                ctx.ui_state.hidden_panes.insert(i);
                                                ui.close();
                                            }
                                            if !is_hidden && ui.button("Hide All").clicked() {
                                                hide_pane_recursive(
                                                    i,
                                                    view,
                                                    &mut ctx.ui_state.hidden_panes,
                                                );
                                                ui.close();
                                            }
                                            if is_hidden && ui.button("Show").clicked() {
                                                ctx.ui_state.hidden_panes.remove(&i);
                                                ui.close();
                                            }
                                            if is_hidden && ui.button("Show All").clicked() {
                                                show_pane_recursive(
                                                    i,
                                                    view,
                                                    &mut ctx.ui_state.hidden_panes,
                                                );
                                                ui.close();
                                            }
                                        });

                                        if response.clicked() {
                                            ctx.ui_state.selected_pane = Some(i);
                                        }

                                        if is_hidden {
                                            ui.label("Hidden");
                                        }
                                    });
                                }
                            } else {
                                ui.label("No .bflyt file loaded");
                            }
                        },
                    );
                }
            }
        });

    if ctx.bflyt_view.is_some() {
        egui::Panel::right("right_control_panel")
            .default_size(300.0)
            .resizable(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut ctx.ui_state.right_sidebar_tab,
                        SidebarRightTab::Properties,
                        "Properties",
                    );

                    ui.selectable_value(
                        &mut ctx.ui_state.right_sidebar_tab,
                        SidebarRightTab::Animations,
                        "Animations",
                    );
                });
                ui.separator();

                match ctx.ui_state.right_sidebar_tab {
                    SidebarRightTab::Properties => {
                        if let Some(ref mut view) = ctx.bflyt_view {
                            ui.vertical(|ui| {
                                if let Some(idx) = ctx.ui_state.selected_pane {
                                    let changed = {
                                        if let Some(node) = view.tree.find_node_mut(idx) {
                                            draw_pane_properties(ui, node)
                                        } else {
                                            false
                                        }
                                    };

                                    if changed {
                                        if let Some(node) = view.tree.find_node_mut(idx) {
                                            node.mark_transform_dirty();
                                        }
                                        view.tree.recompute_dirty();
                                    }
                                } else {
                                    ui.centered_and_justified(|ui| {
                                        ui.label("Select a pane in the tree to inspect it.");
                                    });
                                }
                            });
                        } else {
                            ui.label("No .bflyt file loaded");
                        }
                    }
                    SidebarRightTab::Animations => {
                        ui.vertical(|ui| {
                            ui.heading("Animations");
                            ui.horizontal(|ui| {
                                if ctx.ui_state.timeline.anim_player.is_playing() {
                                    ui.label(
                                        egui::RichText::new("Playing")
                                            .color(egui::Color32::GREEN)
                                            .strong(),
                                    );
                                } else if ctx.ui_state.timeline.anim_player.active.is_some() {
                                    ui.label(
                                        egui::RichText::new("Paused").color(egui::Color32::GOLD),
                                    );
                                } else {
                                    ui.label(
                                        egui::RichText::new("Idle").color(egui::Color32::GRAY),
                                    );
                                }
                            });

                            ui.separator();

                            egui::ScrollArea::vertical()
                                .id_salt("anim_selection_grid")
                                .max_height(120.0)
                                .show(ui, |ui| {
                                    if !ctx.ui_state.anim_names.is_empty() {
                                        ui.horizontal_wrapped(|ui| {
                                            for (idx, name) in
                                                ctx.ui_state.anim_names.iter().enumerate()
                                            {
                                                let is_active =
                                                    ctx.ui_state.timeline.anim_player.active
                                                        == Some(idx);
                                                if ui.selectable_label(is_active, name).clicked() {
                                                    ctx.ui_state.pending_play_anim =
                                                        Some(name.clone());
                                                }
                                            }
                                        });
                                    } else {
                                        ui.label("No animations found.");
                                    }
                                });

                            ui.add_space(8.0);

                            if let Some(idx) = ctx.ui_state.timeline.anim_player.active
                                && let Some(anim) =
                                    ctx.ui_state.timeline.anim_player.anims.get_mut(idx)
                            {
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(&anim.name).strong());
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.small(format!(
                                                    "F: {:.1} / {:.0}",
                                                    anim.frame,
                                                    anim.frame_count()
                                                ));
                                            },
                                        );
                                    });

                                    ui.horizontal(|ui| {
                                        let play_toggle =
                                            if anim.playing { "Pause" } else { "Play" };
                                        if ui.button(play_toggle).clicked() {
                                            anim.playing = !anim.playing;
                                            anim.autoplay = !anim.autoplay;
                                        }

                                        if ui.button("Stop").clicked() {
                                            anim.frame = 0.0;
                                            anim.playing = false;
                                            anim.autoplay = false;
                                        }

                                        if ui.button("Loop").clicked() {
                                            anim.toggle_looping();
                                            anim.frame = 0.0;
                                            anim.playing = true;
                                        }
                                    });

                                    let max_frame = anim.frame_count();
                                    let mut temporary_frame = anim.frame;
                                    let slider_res = ui.add(
                                        egui::Slider::new(&mut temporary_frame, 0.0..=max_frame)
                                            .show_value(false)
                                            .trailing_fill(true),
                                    );

                                    if slider_res.changed() {
                                        anim.frame = temporary_frame;
                                        if slider_res.dragged() {
                                            anim.playing = false;
                                        }
                                    }

                                    if slider_res.drag_stopped() && anim.autoplay {
                                        anim.playing = true;
                                    }
                                });
                            }
                        });
                    }
                }
            });
    }

    if let Some(err) = ctx.ui_state.error_message.to_owned() {
        egui::Window::new("Error")
            .collapsible(false)
            .resizable(false)
            .show(ui, |ui| {
                ui.colored_label(egui::Color32::RED, err);

                if ui.button("Close").clicked() {
                    ctx.ui_state.error_message = None;
                }
            });
    };

    if let Some(ref mut view) = ctx.bflyt_view {
        if let Some(material_list) = view.tree.material_list.as_mut() {
            let changed = ctx.ui_state.material_editor.draw_with(ui, material_list);

            if changed {
                view.tree.for_each_mut(|node| {
                    node.dirty.insert(DirtyFlags::MATERIAL);
                });

                ctx.ui_state.material_editor.pending_upload = true;
            }
        }

        if let Some(idx) = ctx.ui_state.selected_pane
            && let Some(node) = view.tree.find_node_mut(idx)
        {
            ctx.ui_state.detailed_pane_editor.draw_with(ui, node);
        }

        ctx.ui_state.group_editor.draw_with(ui, &mut view.tree);
    }

    ctx.ui_state.timeline.draw(ui);
    ctx.ui_state.archive_browser.draw(ui);
    ctx.ui_state.shortcuts_window.draw(ui);
}

fn hide_pane_recursive(idx: usize, view: &BflytView, hidden_set: &mut HashSet<usize>) {
    hidden_set.insert(idx);
    for child in view.descendants(idx) {
        hidden_set.insert(child);
    }
}

fn show_pane_recursive(idx: usize, view: &BflytView, hidden_set: &mut HashSet<usize>) {
    hidden_set.remove(&idx);
    for child in view.descendants(idx) {
        hidden_set.remove(&child);
    }
}

fn draw_pane_properties(ui: &mut Ui, pane: &mut crate::pane_tree::PaneNode) -> bool {
    let mut changed = false;
    egui::ScrollArea::vertical()
        .id_salt("pane_properties_scroll")
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.heading("Core Properties");
            ui.add_space(4.0);

            egui::Grid::new("pane_info_core")
                .num_columns(2)
                .striped(true)
                .spacing([12.0, 4.0])
                .show(ui, |ui| {
                    draw_string(ui, "Name", &pane.label);
                    draw_string(ui, "Kind", &pane.kind);
                    draw_prop_f32(ui, "World X", pane.world_pos.x);
                    draw_prop_f32(ui, "World Y", pane.world_pos.y);
                    draw_prop_f32(ui, "Width", pane.world_size.x);
                    draw_prop_f32(ui, "Height", pane.world_size.y);
                    draw_prop(ui, "Depth", pane.depth);
                    draw_prop(ui, "Visible", pane.visible);
                    draw_prop(ui, "Pane Index", pane.pane_idx);
                    if let Some(source) = &pane.parts_source {
                        draw_string(ui, "Parts Source", source);
                    }
                });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);
            ui.heading("Transform");
            ui.add_space(4.0);

            if let Some(base) = pane.section.get_base_pane_mut() {
                egui::Grid::new("pane_transform_grid")
                    .num_columns(2)
                    .striped(true)
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Translate X");
                        if ui
                            .add(egui::DragValue::new(&mut base.translation.x).speed(0.5))
                            .changed()
                        {
                            changed = true;
                        }
                        ui.end_row();

                        ui.label("Translate Y");
                        if ui
                            .add(egui::DragValue::new(&mut base.translation.y).speed(0.5))
                            .changed()
                        {
                            changed = true;
                        }
                        ui.end_row();

                        ui.label("Translate Z");
                        if ui
                            .add(egui::DragValue::new(&mut base.translation.z).speed(0.5))
                            .changed()
                        {
                            changed = true;
                        }
                        ui.end_row();

                        ui.label("Rotate X");
                        if ui
                            .add(egui::DragValue::new(&mut base.rotation.x).speed(0.1))
                            .changed()
                        {
                            changed = true;
                        }
                        ui.end_row();

                        ui.label("Rotate Y");
                        if ui
                            .add(egui::DragValue::new(&mut base.rotation.y).speed(0.1))
                            .changed()
                        {
                            changed = true;
                        }
                        ui.end_row();

                        ui.label("Rotate Z");
                        if ui
                            .add(egui::DragValue::new(&mut base.rotation.z).speed(0.1))
                            .changed()
                        {
                            changed = true;
                        }
                        ui.end_row();

                        ui.label("Scale X");
                        if ui
                            .add(egui::DragValue::new(&mut base.scale.x).speed(0.01))
                            .changed()
                        {
                            changed = true;
                        }
                        ui.end_row();

                        ui.label("Scale Y");
                        if ui
                            .add(egui::DragValue::new(&mut base.scale.y).speed(0.01))
                            .changed()
                        {
                            changed = true;
                        }
                        ui.end_row();

                        ui.label("Size X");
                        if ui
                            .add(egui::DragValue::new(&mut base.size.x).speed(0.5))
                            .changed()
                        {
                            changed = true;
                        }
                        ui.end_row();

                        ui.label("Size Y");
                        if ui
                            .add(egui::DragValue::new(&mut base.size.y).speed(0.5))
                            .changed()
                        {
                            changed = true;
                        }
                        ui.end_row();
                    });
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);
            ui.heading("Section Details");
            ui.add_space(4.0);

            match &pane.section {
                BflytSection::Layout(layout) => {
                    egui::Grid::new("bflyt_layout_grid")
                        .num_columns(2)
                        .striped(true)
                        .spacing([12.0, 4.0])
                        .show(ui, |ui| {
                            draw_layout_section(ui, layout);
                        });
                }
                BflytSection::Pane(pane_detail) => {
                    egui::Grid::new("bflyt_pane_grid")
                        .num_columns(2)
                        .striped(true)
                        .spacing([12.0, 4.0])
                        .show(ui, |ui| {
                            draw_pane_section(ui, pane_detail);
                        });
                }
                _ => {
                    ui.weak("Section details not yet editable for this type.");
                }
            }
        });
    changed
}

fn draw_layout_section(ui: &mut Ui, layout: &Layout) {
    draw_string(ui, "Name", &layout.name);
    draw_prop(ui, "Centered", layout.is_centered);
    draw_prop_f32(ui, "Width", layout.width);
    draw_prop_f32(ui, "Height", layout.height);
    draw_prop_f32(ui, "Parts Width", layout.parts_width);
    draw_prop_f32(ui, "Parts Height", layout.parts_height);
}

fn draw_pane_section(ui: &mut Ui, pane: &Pane) {
    draw_string(ui, "Name", &pane.pane_name);
    draw_prop_debug(ui, "Origin X", pane.origin.origin_x);
    draw_prop_debug(ui, "Origin Y", pane.origin.origin_y);
    draw_prop_debug(ui, "Parent Origin X", pane.origin.parent_origin_x);
    draw_prop_debug(ui, "Parent Origin Y", pane.origin.parent_origin_y);

    draw_vector_3f(ui, "Translation", pane.translation);
    draw_vector_3f(ui, "Rotation", pane.rotation);
    draw_vector_2f(ui, "Scale", pane.scale);
    draw_vector_2f(ui, "Size", pane.size);

    draw_prop(ui, "Alpha", pane.alpha);
    draw_prop(ui, "Influenced Alpha", pane.pane_flags.influenced_alpha);
    draw_prop(ui, "Visible", pane.pane_flags.is_visible);

    draw_prop(ui, "Extended User Data", pane.flag_ex.is_ext_user_data);
    draw_prop(ui, "No Scale By Parts", pane.flag_ex.is_no_scale_by_parts);
    draw_prop(
        ui,
        "Scale Size By Parts Root",
        pane.flag_ex.is_scale_size_by_parts_root,
    );
}

fn draw_vector_2f(ui: &mut egui::Ui, label: &str, vector: Vector2f) {
    ui.strong(label);
    ui.label(format!("({:.2}, {:.2})", vector.x, vector.y));
    ui.end_row();
}

fn draw_vector_3f(ui: &mut egui::Ui, label: &str, vector: Vector3f) {
    ui.strong(label);
    ui.label(format!(
        "({:.2}, {:.2}, {:.2})",
        vector.x, vector.y, vector.z
    ));
    ui.end_row();
}

fn draw_prop(ui: &mut egui::Ui, label: &str, value: impl std::fmt::Display) {
    ui.strong(label);
    ui.label(value.to_string());
    ui.end_row();
}

fn draw_prop_debug(ui: &mut egui::Ui, label: &str, value: impl std::fmt::Debug) {
    ui.strong(label);
    ui.label(format!("{:?}", value));
    ui.end_row();
}

fn draw_string(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.strong(label);
    ui.label(value);
    ui.end_row();
}

fn draw_prop_f32(ui: &mut egui::Ui, label: &str, value: f32) {
    ui.strong(label);
    ui.label(format!("{:.2}", value));
    ui.end_row();
}
