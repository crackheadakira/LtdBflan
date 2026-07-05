use std::collections::HashSet;

use egui::Ui;
use nnbfl::{
    bflan::{anim_info::AnimInfo, curves::Curve, targets::TargetIndex},
    bflyt::{
        file::BflytSection,
        list::{Layout, MaterialBlendMode, MaterialList},
        pane::Pane,
    },
    ui2d::types::{Vector2f, Vector3f},
};

use crate::{
    anim_state::AnimPlayer,
    bflyt_view::BflytView,
    camera::Camera,
    detailed_pane_editor::DetailedPaneEditor,
    material_editor::{DrawUiWith, MaterialEditor},
    pane_tree::DirtyFlags,
    renderer::timeline::{
        PendingKeyEdit, TIMELINE_MIN_VISIBLE_FRAMES, TimelineDrag, TimelineGeometry,
        TimelineLayout, TimelineRow,
    },
    traits::Displaying,
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

    pub context_menu: Option<ContextMenuState>,
    pub archive_browser_open: bool,
    pub shortcuts_window_open: bool,

    pub timeline: TimelineState,
    pub material_editor: MaterialEditor,
    pub detailed_pane_editor: DetailedPaneEditor,
}

pub struct TimelineState {
    pub geometry: Option<TimelineGeometry>,
    pub drag: Option<TimelineDrag>,
    pub pending_key_edit: Option<PendingKeyEdit>,
    pub expanded_anim_panes: HashSet<usize>,
    pub zoom: f32,

    /// First visible frame from the left edge of the graph.
    pub pan_frame: f32,
    pub panning: bool,
    pub frame_rate: f32,
}

impl Default for TimelineState {
    fn default() -> Self {
        Self {
            geometry: None,
            drag: None,
            pending_key_edit: None,
            expanded_anim_panes: HashSet::new(),
            zoom: 0.0,
            pan_frame: 0.0,
            panning: false,
            frame_rate: 30.0,
        }
    }
}

pub struct ContextMenuState {
    pub pane_idx: usize,
    pub pos: egui::Pos2,
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
    Materials,
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

pub fn draw_ui(
    ui: &mut Ui,
    view: &mut Option<BflytView>,
    state: &mut UiState,
    camera: &Camera,
    anim_player: &mut AnimPlayer,
    screen_w: f32,
    screen_h: f32,
    blarc_dir: Option<&std::path::PathBuf>,
    archive_scan: Option<&crate::archive_browser::ArchiveScan>,
) {
    crate::keybinds::handle(ui.ctx(), state, anim_player);

    if let Some(view) = view {
        let viewport_rect = ui.content_rect();
        let painter = ui.painter().with_clip_rect(viewport_rect);

        for node in view.tree.iter() {
            let i = node.pane_idx;

            if let BflytSection::TextBoxPane(text_box) = &node.section
                && !state.hidden_panes.contains(&i)
                && node.visible
            {
                let display_text = text_box.text.as_deref().unwrap_or("");

                let tl = node.plain_quad.corners[0];
                let br = node.plain_quad.corners[3];

                let center_x = (tl[0] + br[0]) * 0.5;
                let center_y = (tl[1] + br[1]) * 0.5;
                let screen_pos = camera.world_to_screen([center_x, center_y], screen_w, screen_h);

                let font_size = (32.0 * camera.zoom).clamp(8.0, 128.0);
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

    draw_context_menu(ui, state, view);

    egui::Panel::top("menu_bar").show(ui, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Load File...").clicked() {
                    state.pending_action = Some(UiAction::LoadFile);
                    state.hidden_panes.clear();
                    state.selected_pane = None;

                    ui.close();
                }

                if ui.button("Set Layout Folder...").clicked() {
                    state.pending_action = Some(UiAction::SetBlarcDir);
                    ui.close();
                }

                if ui.button("Save File As...").clicked() {
                    state.pending_action = Some(UiAction::SaveFile);
                    ui.close();
                }
            });

            if ui.button("Browse Archives...").clicked() {
                state.archive_browser_open = true;
            }

            if let Some(view) = view {
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
                            .radio_value(&mut state.active_debug_stage, stage_idx, label)
                            .clicked()
                        {
                            ui.close();
                        }
                    }
                });

                if view.tree.material_list.is_some() {
                    if ui.button("Material Editor").clicked() {
                        state.material_editor.is_editor_visible = true;
                    }
                }

                if state.selected_pane.is_some() {
                    if ui.button("Detailed Pane Editor").clicked() {
                        state.detailed_pane_editor.is_editor_visible = true;
                    };
                };
            }

            ui.menu_button("Help", |ui| {
                if ui.button("Keyboard Shortcuts").clicked() {
                    state.shortcuts_window_open = true;
                    ui.close();
                }
            });
        })
    });

    egui::Panel::left("pane_tree")
        .default_size(240.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut state.sidebar_tab, SidebarTab::Panes, "Pane Tree");
                ui.selectable_value(&mut state.sidebar_tab, SidebarTab::Materials, "Materials");
            });
            ui.separator();

            match state.sidebar_tab {
                SidebarTab::Panes => {
                    ui.heading("Pane Tree");
                    ui.checkbox(&mut state.visiblity_flags.clip_to_root, "Clip to root pane");
                    ui.checkbox(
                        &mut state.visiblity_flags.only_textured,
                        "Draw only textures",
                    );
                    ui.checkbox(
                        &mut state.visiblity_flags.quad_for_textured,
                        "Draw pane outlines for textures",
                    );
                    ui.checkbox(
                        &mut state.visiblity_flags.no_textured,
                        "Draw only pane outlines",
                    );
                    ui.separator();

                    let total_rows = view.as_ref().map_or(1, |v| v.tree.flatten().len());
                    egui::ScrollArea::vertical().auto_shrink(false).show_rows(
                        ui,
                        24.0,
                        total_rows,
                        |ui, row_range| {
                            if let Some(view) = view {
                                let nodes = view.tree.flatten();

                                for idx in row_range {
                                    let node = nodes[idx];
                                    let i = node.pane_idx;

                                    let indent = node.depth as f32 * 12.0;
                                    ui.horizontal(|ui| {
                                        ui.add_space(indent);

                                        let selected = state.selected_pane == Some(i);
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

                                        let is_hidden = state.hidden_panes.contains(&i);

                                        let response = ui.selectable_label(selected, label);
                                        response.context_menu(|ui| {
                                            if !is_hidden && ui.button("Hide").clicked() {
                                                state.hidden_panes.insert(i);
                                                ui.close();
                                            }
                                            if !is_hidden && ui.button("Hide All").clicked() {
                                                hide_pane_recursive(
                                                    i,
                                                    view,
                                                    &mut state.hidden_panes,
                                                );
                                                ui.close();
                                            }
                                            if is_hidden && ui.button("Show").clicked() {
                                                state.hidden_panes.remove(&i);
                                                ui.close();
                                            }
                                            if is_hidden && ui.button("Show All").clicked() {
                                                show_pane_recursive(
                                                    i,
                                                    view,
                                                    &mut state.hidden_panes,
                                                );
                                                ui.close();
                                            }
                                        });

                                        if response.clicked() {
                                            state.selected_pane = Some(i);
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
                SidebarTab::Materials => {
                    ui.heading("Material List");
                    ui.separator();

                    if let Some(view) = view {
                        if let Some(material_list) = &view.tree.material_list {
                            draw_material_list(ui, material_list);
                        } else {
                            ui.label("Bflyt file has no material list");
                        }
                    } else {
                        ui.label("No .bflyt file loaded");
                    }
                }
            }
        });

    if view.is_some() {
        egui::Panel::right("right_control_panel")
            .default_size(300.0)
            .resizable(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut state.right_sidebar_tab,
                        SidebarRightTab::Properties,
                        "Properties",
                    );

                    ui.selectable_value(
                        &mut state.right_sidebar_tab,
                        SidebarRightTab::Animations,
                        "Animations",
                    );
                });
                ui.separator();

                match state.right_sidebar_tab {
                    SidebarRightTab::Properties => {
                        if let Some(view) = view {
                            ui.vertical(|ui| {
                                if let Some(idx) = state.selected_pane {
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
                                if anim_player.is_playing() {
                                    ui.label(
                                        egui::RichText::new("Playing")
                                            .color(egui::Color32::GREEN)
                                            .strong(),
                                    );
                                } else if anim_player.active.is_some() {
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
                                    if !state.anim_names.is_empty() {
                                        ui.horizontal_wrapped(|ui| {
                                            for (idx, name) in state.anim_names.iter().enumerate() {
                                                let is_active = anim_player.active == Some(idx);
                                                if ui.selectable_label(is_active, name).clicked() {
                                                    state.pending_play_anim = Some(name.clone());
                                                }
                                            }
                                        });
                                    } else {
                                        ui.label("No animations found.");
                                    }
                                });

                            ui.add_space(8.0);

                            if let Some(idx) = anim_player.active
                                && let Some(anim) = anim_player.anims.get_mut(idx)
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

                                ui.add_space(8.0);
                                ui.separator();
                                ui.heading("Animation Details");

                                egui::ScrollArea::vertical()
                                    .id_salt("anim_debug_scroll")
                                    .auto_shrink(false)
                                    .show(ui, |ui| {
                                        for content in &anim.bflan.anim_info.contents {
                                            let target_name = &content.name;

                                            egui::CollapsingHeader::new(target_name)
                                                .id_salt(format!("col_target_{}", target_name))
                                                .default_open(true)
                                                .show(ui, |ui| {
                                                    for (info_idx, info) in
                                                        content.infos.iter().enumerate()
                                                    {
                                                        match info {
                                                            AnimInfo::Standard {
                                                                anim_type,
                                                                targets,
                                                            } => {
                                                                let info_id = format!(
                                                                    "{target_name}_{anim_type:?}_{info_idx}"
                                                                );

                                                                for (target_idx, anim_target) in
                                                                    targets.iter().enumerate()
                                                                {
                                                                    let channel_label =
                                                                        get_target_index_label(
                                                                            &anim_target.target,
                                                                            anim_target.layer,
                                                                        );

                                                                    let curve_id = format!(
                                                                        "{info_id}_{target_idx}"
                                                                    );

                                                                    ui.collapsing(
                                                                        channel_label,
                                                                        |ui| {
                                                                            draw_keyframe_inspector(
                                                                                ui,
                                                                                &anim_target.curve,
                                                                                anim.frame,
                                                                                &curve_id,
                                                                            );
                                                                        },
                                                                    );
                                                                }
                                                            }

                                                            AnimInfo::ExtendedUserData {
                                                                anim_type,
                                                                ..
                                                            } => {
                                                                ui.small(format!(
                                                                    "User Data Track: {anim_type:?}",
                                                                ));
                                                            }
                                                        }
                                                    }
                                                });
                                        }
                                    });
                            }
                        });
                    }
                }
            });
    }

    if let Some(err) = state.error_message.to_owned() {
        egui::Window::new("Error")
            .collapsible(false)
            .resizable(false)
            .show(ui, |ui| {
                ui.colored_label(egui::Color32::RED, err);

                if ui.button("Close").clicked() {
                    state.error_message = None;
                }
            });
    };

    if let Some(view) = view {
        if let Some(material_list) = view.tree.material_list.as_mut() {
            let changed = state.material_editor.draw_with_mut(ui, material_list);

            if changed {
                view.tree.for_each_mut(|node| {
                    node.dirty.insert(DirtyFlags::MATERIAL);
                });

                state.material_editor.pending_upload = true;
            }
        }

        if let Some(idx) = state.selected_pane
            && let Some(node) = view.tree.find_node_mut(idx)
        {
            state.detailed_pane_editor.draw_with_mut(ui, node);
        }
    }

    draw_timeline_panel(ui, state, anim_player);
    draw_archive_browser_window(ui, state, blarc_dir, archive_scan);
    draw_shortcuts_window(ui, state);
}

fn get_target_index_label(target: &TargetIndex, layer: u8) -> String {
    let base_name = match target {
        TargetIndex::PaneSrt(t) => format!("SRT, {t:?}"),
        TargetIndex::VertexColor(t) => format!("Vertex Color, {t:?}"),
        TargetIndex::MaterialColor(t) => format!("Material Color, {t:?}"),
        TargetIndex::TextureSrt(t) => format!("Texture SRT, {t:?}"),
        TargetIndex::TexturePattern(t) => format!("Texture Pattern, {t:?}"),
        TargetIndex::IndirectSrt(t) => format!("Indirect SRT, {t:?}"),
        TargetIndex::PerCharacterTransformCurve(t) => {
            format!("Per Character Transform Curve, {t:?}")
        }
        TargetIndex::PerCharacterTransform(t) => format!("Per Character Transform, {t:?}"),
        TargetIndex::DropShadow(t) => format!("Drop Shadow, {t:?}"),
        TargetIndex::MaskTexture(t) => format!("Mask Texture, {t:?}"),
        TargetIndex::ProceduralShape(t) => format!("Procedural Shape, {t:?}"),
        TargetIndex::Window(t) => format!("Window, {t:?}"),
        TargetIndex::FontShadow(t) => format!("Font Shadow, {t:?}"),
        TargetIndex::BrickRepeat(t) => format!("Brick Repeat, {t:?}"),
        other => format!("{other:?}"),
    };

    if layer > 0 {
        format!("{} [Layer {}]", base_name, layer)
    } else {
        base_name
    }
}

fn draw_shortcuts_window(ui: &mut egui::Ui, state: &mut UiState) {
    if !state.shortcuts_window_open {
        return;
    }

    let mut open = true;

    egui::Window::new("Keyboard Shortcuts")
        .open(&mut open)
        .resizable(false)
        .default_width(360.0)
        .show(ui.ctx(), |ui| {
            egui::ScrollArea::vertical()
                .max_height(400.0)
                .show(ui, |ui| {
                    egui::Grid::new("shortcuts_grid")
                        .num_columns(2)
                        .spacing([16.0, 8.0])
                        .striped(true)
                        .show(ui, |ui| {
                            for bind in crate::keybinds::BINDINGS {
                                let mut mods = String::new();
                                if bind.modifiers.command {
                                    mods.push_str("Ctrl+");
                                }

                                if bind.modifiers.shift {
                                    mods.push_str("Shift+");
                                }

                                if bind.modifiers.alt {
                                    mods.push_str("Alt+");
                                }

                                let shortcut_text = format!("{mods}{:?}", bind.key);

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.colored_label(
                                            egui::Color32::from_gray(140),
                                            shortcut_text,
                                        );
                                    },
                                );

                                ui.label(bind.description);
                                ui.end_row();
                            }
                        });
                });
        });

    if !open {
        state.shortcuts_window_open = false;
    }
}

fn draw_keyframe_inspector(ui: &mut egui::Ui, curve: &Curve, current_frame: f32, unique_id: &str) {
    match curve {
        Curve::Hermite(keyframes) => {
            ui.label("Hermite Spline");

            egui::Grid::new(format!("grid_hermite_{}", unique_id))
                .striped(true)
                .num_columns(3)
                .spacing([12.0, 4.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Frame").strong().small());
                    ui.label(egui::RichText::new("Value").strong().small());
                    ui.label(egui::RichText::new("Slope").strong().small());
                    ui.end_row();

                    for kf in keyframes {
                        let is_active = (kf.frame - current_frame).abs() < 0.2;
                        let text_color = if is_active {
                            egui::Color32::GREEN
                        } else {
                            ui.visuals().text_color()
                        };

                        ui.label(egui::RichText::new(format!("{:.1}", kf.frame)).color(text_color));
                        ui.label(egui::RichText::new(format!("{:.4}", kf.value)).color(text_color));
                        ui.label(egui::RichText::new(format!("{:.4}", kf.slope)).color(text_color));
                        ui.end_row();
                    }
                });
        }

        Curve::Step(keyframes) => {
            ui.label("Discrete Step");

            egui::Grid::new(format!("grid_step_{}", unique_id))
                .striped(true)
                .num_columns(2)
                .spacing([12.0, 4.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Frame").strong().small());
                    ui.label(egui::RichText::new("Value").strong().small());
                    ui.end_row();

                    for kf in keyframes {
                        let is_active = (kf.frame - current_frame).abs() < 0.2;
                        let text_color = if is_active {
                            egui::Color32::GREEN
                        } else {
                            ui.visuals().text_color()
                        };

                        ui.label(egui::RichText::new(format!("{:.1}", kf.frame)).color(text_color));
                        ui.label(egui::RichText::new(format!("{:.4}", kf.value)).color(text_color));
                        ui.end_row();
                    }
                });
        }

        Curve::Constant(values) => {
            ui.label("Static Constant");

            ui.horizontal_wrapped(|ui| {
                ui.label("Values:");
                for val in values {
                    ui.label(format!("[{:.4}]", val));
                }
            });
        }
    }
}

fn draw_context_menu(ui: &mut Ui, state: &mut UiState, view: &Option<BflytView>) {
    let Some(menu) = &state.context_menu else {
        return;
    };

    let pane_idx = menu.pane_idx;
    let pos = menu.pos;

    let node = view
        .as_ref()
        .and_then(|v| v.tree.iter().find(|n| n.pane_idx == pane_idx));

    let label = node
        .map(|n| n.label.trim_end_matches('\0').to_string())
        .unwrap_or_else(|| "Pane".to_string());

    let is_parts_content = node.is_some_and(|n| n.parts_source.is_some());
    let mut close = false;

    let area_response = egui::Area::new(egui::Id::new("pane_context_menu"))
        .fixed_pos(pos)
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(160.0);
                ui.label(egui::RichText::new(label).strong());
                ui.separator();

                let hidden = state.hidden_panes.contains(&pane_idx);
                if ui.button(if hidden { "Show" } else { "Hide" }).clicked() {
                    if hidden {
                        state.hidden_panes.remove(&pane_idx);
                    } else {
                        state.hidden_panes.insert(pane_idx);
                    }

                    close = true;
                }

                ui.separator();

                if is_parts_content {
                    ui.weak("Part of a linked layout - edit it via the");
                    ui.weak("PartsPane's overrides, not directly.");
                } else {
                    if ui.button("Duplicate").clicked() {
                        state.pending_action = Some(UiAction::DuplicatePane(pane_idx));
                        close = true;
                    }

                    if ui
                        .add(egui::Button::new(
                            egui::RichText::new("Delete")
                                .color(egui::Color32::from_rgb(224, 96, 96)),
                        ))
                        .clicked()
                    {
                        state.pending_action = Some(UiAction::DeletePane(pane_idx));
                        close = true;
                    }
                }
            });
        });

    let clicked_outside =
        ui.ctx().input(|i| i.pointer.any_click()) && !area_response.response.contains_pointer();
    let escape_pressed = ui.ctx().input(|i| i.key_pressed(egui::Key::Escape));

    if close || clicked_outside || escape_pressed {
        state.context_menu = None;
    }
}

fn draw_timeline_panel(ui: &mut Ui, state: &mut UiState, anim_player: &AnimPlayer) {
    state.timeline.geometry = None;

    let Some(anim) = anim_player.active.and_then(|i| anim_player.anims.get(i)) else {
        return;
    };

    egui::Panel::bottom("timeline_panel")
        .resizable(true)
        .default_size(260.0)
        .min_size(80.0)
        .show(ui, |ui| {
            let frame_count = anim.frame_count().max(1.0);
            let zoom = state.timeline.zoom.max(1.0);
            let visible_span = (frame_count / zoom).clamp(TIMELINE_MIN_VISIBLE_FRAMES, frame_count);
            let max_pan = (frame_count - visible_span).max(0.0);
            state.timeline.pan_frame = state.timeline.pan_frame.clamp(0.0, max_pan);

            ui.horizontal(|ui| {
                ui.heading("Timeline");
                ui.separator();

                ui.label(format!(
                    "{} - frame {:.1} / {frame_count:.0}",
                    anim.name, anim.frame
                ));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(egui::DragValue::new(&mut state.timeline.frame_rate).speed(1));
                    ui.label("FPS:");

                    ui.separator();

                    if ui.small_button("Fit").clicked() {
                        state.timeline.zoom = 1.0;
                        state.timeline.pan_frame = 0.0;
                    }

                    ui.label(format!("Zoom {zoom:.1}x"));
                });
            });

            ui.separator();

            let rows = TimelineRow::build(anim, &state.timeline.expanded_anim_panes);

            if rows.is_empty() {
                ui.weak("No animated panes in this animation.");
                return;
            }

            const LABEL_COL_WIDTH: f32 = 190.0;
            const RULER_HEIGHT: f32 = 22.0;

            let row_h = TimelineRow::total_height();
            let avail = ui.available_rect_before_wrap();

            for (i, row) in rows.iter().enumerate() {
                let row_top = avail.min.y + RULER_HEIGHT + i as f32 * row_h;
                if row_top > avail.max.y {
                    break;
                }

                let row_rect = egui::Rect::from_min_size(
                    egui::pos2(avail.min.x, row_top),
                    egui::vec2(LABEL_COL_WIDTH, row_h),
                );

                match row {
                    TimelineRow::PaneHeader { content_idx, label } => {
                        let expanded = state.timeline.expanded_anim_panes.contains(content_idx);
                        let header_response = ui.interact(
                            row_rect,
                            ui.id().with(("timeline_header", *content_idx)),
                            egui::Sense::click(),
                        );

                        if header_response.clicked() {
                            if expanded {
                                state.timeline.expanded_anim_panes.remove(content_idx);
                            } else {
                                state.timeline.expanded_anim_panes.insert(*content_idx);
                            }
                        }
                        if header_response.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }

                        let status = if expanded { "Close" } else { "Open" };
                        ui.painter().text(
                            egui::pos2(avail.min.x + 4.0, row_top + row_h * 0.5 - 2.0),
                            egui::Align2::LEFT_CENTER,
                            format!("{status}, {label}"),
                            egui::FontId::proportional(12.0),
                            egui::Color32::WHITE,
                        );
                    }

                    TimelineRow::Track(track) => {
                        ui.painter().text(
                            egui::pos2(avail.min.x + 16.0, row_top + row_h * 0.5 - 2.0),
                            egui::Align2::LEFT_CENTER,
                            &track.label,
                            egui::FontId::proportional(11.0),
                            egui::Color32::from_gray(200),
                        );
                    }
                }
            }

            let graph_rect = egui::Rect::from_min_max(
                egui::pos2(avail.min.x + LABEL_COL_WIDTH, avail.min.y),
                avail.max,
            );

            let ppp = ui.ctx().pixels_per_point();
            let graph_rect_px = (
                graph_rect.min.x * ppp,
                graph_rect.min.y * ppp,
                graph_rect.width() * ppp,
                graph_rect.height() * ppp,
            );

            let layout = TimelineLayout::new(
                anim,
                &rows,
                state.timeline.pan_frame,
                visible_span,
                graph_rect_px.0,
                graph_rect_px.1,
                graph_rect_px.2,
                graph_rect_px.3,
            );

            let graph_response = ui.interact(
                graph_rect,
                ui.id().with("timeline_graph"),
                egui::Sense::click_and_drag(),
            );

            let current_pointer = graph_response
                .hover_pos()
                .or_else(|| graph_response.interact_pointer_pos());

            let hovered_key = current_pointer.and_then(|pos| {
                let pointer_px = [pos.x * ppp, pos.y * ppp];

                TimelineDrag::find_nearest_key(anim, &rows, &layout, pointer_px)
            });

            if graph_response.drag_started() {
                if let Some(target_key) = hovered_key.clone() {
                    state.timeline.drag = Some(target_key);
                }

                state.timeline.panning = state.timeline.drag.is_none();
            }

            if let Some(drag) = state.timeline.drag.clone()
                && graph_response.dragged()
                && let Some(pos) = graph_response.interact_pointer_pos()
            {
                let pointer_px = [pos.x * ppp, pos.y * ppp];

                state.timeline.pending_key_edit = Some(PendingKeyEdit {
                    frame: layout.x_to_frame(pointer_px[0]),
                    value: layout.y_to_value(drag.row, pointer_px[1]),
                    track: drag.track,
                    key_idx: drag.key_idx,
                });
            } else if state.timeline.panning && graph_response.dragged() {
                let px_per_frame = graph_rect_px.2 / visible_span;
                let delta_frames = -(graph_response.drag_delta().x * ppp) / px_per_frame.max(0.001);

                state.timeline.pan_frame =
                    (state.timeline.pan_frame + delta_frames).clamp(0.0, max_pan);
            }

            if graph_response.drag_stopped() {
                state.timeline.drag = None;
                state.timeline.panning = false;
            }

            if graph_response.hovered() {
                let scroll = ui.input(|i| i.smooth_scroll_delta.y);

                if scroll.abs() > f32::EPSILON
                    && let Some(pos) = graph_response.hover_pos()
                {
                    let pointer_frame = layout.x_to_frame(pos.x * ppp);
                    let max_zoom = frame_count / TIMELINE_MIN_VISIBLE_FRAMES;
                    let new_zoom = (zoom * (1.0 + scroll * 0.002)).clamp(1.0, max_zoom.max(1.0));
                    let new_span =
                        (frame_count / new_zoom).clamp(TIMELINE_MIN_VISIBLE_FRAMES, frame_count);

                    let t = (pointer_frame - state.timeline.pan_frame) / visible_span;
                    let new_max_pan = (frame_count - new_span).max(0.0);

                    state.timeline.pan_frame =
                        (pointer_frame - t * new_span).clamp(0.0, new_max_pan);
                    state.timeline.zoom = new_zoom;
                }

                let cursor = if state.timeline.drag.is_some() {
                    egui::CursorIcon::Grabbing
                } else if state.timeline.panning {
                    egui::CursorIcon::Grab
                } else if hovered_key.is_some() {
                    egui::CursorIcon::PointingHand
                } else {
                    egui::CursorIcon::Default
                };

                ui.ctx().set_cursor_icon(cursor);
            }

            state.timeline.geometry = Some(TimelineGeometry::build(
                anim,
                &rows,
                anim.frame,
                state.timeline.pan_frame,
                visible_span,
                graph_rect_px,
            ));

            ui.allocate_rect(avail, egui::Sense::hover());
        });
}

fn draw_archive_browser_window(
    ui: &mut Ui,
    state: &mut UiState,
    blarc_dir: Option<&std::path::PathBuf>,
    archive_scan: Option<&crate::archive_browser::ArchiveScan>,
) {
    if !state.archive_browser_open {
        return;
    }

    let mut open = true;

    egui::Window::new("Browse Archives")
        .open(&mut open)
        .default_width(420.0)
        .default_height(420.0)
        .show(ui, |ui| {
            let Some(dir) = blarc_dir else {
                ui.label("Set a layout folder first (File > Set Layout Folder...).");
                return;
            };

            ui.label(format!("Directory: {}", dir.display()));
            ui.separator();

            match archive_scan {
                None => {
                    ui.label(
                        "Not scanned yet. Scanning reads and unpacks every archive in this \
                         directory to check for BFLYT layouts, which can take a while on a \
                         large directory.",
                    );
                    if ui.button("Scan directory").clicked() {
                        state.pending_action = Some(UiAction::StartArchiveScan);
                    }
                }

                Some(scan) if scan.root() != dir => {
                    ui.label("The layout folder changed since the last scan.");
                    if ui.button("Scan directory").clicked() {
                        state.pending_action = Some(UiAction::StartArchiveScan);
                    }
                }

                Some(scan) => {
                    ui.horizontal(|ui| {
                        if scan.done {
                            ui.label(format!(
                                "Found {} BFLYT-containing archive(s) out of {} scanned.",
                                scan.entries.len(),
                                scan.scanned
                            ));
                        } else if scan.cancelled {
                            ui.label(format!(
                                "Cancelled after scanning {} of {}.",
                                scan.scanned, scan.total
                            ));
                        } else {
                            ui.spinner();
                            ui.label(format!(
                                "Scanning... {} / {}",
                                scan.scanned,
                                scan.total.max(scan.scanned)
                            ));
                            if ui.button("Cancel").clicked() {
                                state.pending_action = Some(UiAction::CancelArchiveScan);
                            }
                        }

                        if (scan.done || scan.cancelled) && ui.button("Rescan").clicked() {
                            state.pending_action = Some(UiAction::StartArchiveScan);
                        }
                    });

                    if !scan.done && !scan.cancelled && scan.total > 0 {
                        ui.add(egui::ProgressBar::new(
                            scan.scanned as f32 / scan.total.max(1) as f32,
                        ));
                    }

                    ui.separator();

                    egui::ScrollArea::vertical().auto_shrink(false).show_rows(
                        ui,
                        24.0,
                        scan.entries.len(),
                        |ui, row_range| {
                            if scan.entries.is_empty() && scan.done {
                                ui.weak("No BFLYT-containing archives found.");
                            }

                            for i in row_range {
                                let entry = &scan.entries[i];

                                ui.horizontal(|ui| {
                                    ui.label(&entry.display_name);
                                    if ui.button("Load").clicked() {
                                        state.pending_action =
                                            Some(UiAction::LoadArchiveEntry(entry.clone()));
                                        state.hidden_panes.clear();
                                        state.selected_pane = None;
                                    }
                                });
                            }
                        },
                    );
                }
            }
        });

    if !open {
        state.archive_browser_open = false;
    }
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

fn draw_material_list(ui: &mut Ui, list: &MaterialList) {
    ui.label(format!("Total Materials: {}", list.materials.len()));
    ui.add_space(4.0);

    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .id_salt("material_sidebar_scroll")
        .show(ui, |ui| {
            for (idx, material) in list.materials.iter().enumerate() {
                let header_text = format!("[{idx}] {}", material.material_name);

                egui::CollapsingHeader::new(header_text)
                    .id_salt(ui.id().with(idx))
                    .show(ui, |ui| {
                        if !material.colors.is_empty() {
                            egui::CollapsingHeader::new(format!(
                                "Colors ({})",
                                material.colors.len()
                            ))
                            .id_salt("colors")
                            .show(ui, |ui| {
                                draw_vec_grid(
                                    ui,
                                    "colors_grid",
                                    &material.colors,
                                    |ui, i, color| {
                                        if let Some(color) = &color.color_f32 {
                                            draw_prop_f32(ui, &format!("[{i}] Red"), color.r);
                                            draw_prop_f32(ui, &format!("[{i}] Green"), color.g);
                                            draw_prop_f32(ui, &format!("[{i}] Blue"), color.b);
                                            draw_prop_f32(ui, &format!("[{i}] Alpha"), color.a);
                                        } else if let Some(color) = &color.color_u8 {
                                            draw_prop(ui, &format!("[{i}] Red"), color.r);
                                            draw_prop(ui, &format!("[{i}] Green"), color.g);
                                            draw_prop(ui, &format!("[{i}] Blue"), color.b);
                                            draw_prop(ui, &format!("[{i}] Alpha"), color.a);
                                        }
                                    },
                                );
                            });
                        }

                        if !material.tex_maps.is_empty() {
                            egui::CollapsingHeader::new(format!(
                                "Texture Maps ({})",
                                material.tex_maps.len()
                            ))
                            .id_salt("tex_sub")
                            .show(ui, |ui| {
                                draw_vec_grid(ui, "tex_grid", &material.tex_maps, |ui, i, tex| {
                                    draw_string(ui, &format!("[{i}] Name"), &tex.texture_name);
                                    draw_prop_debug(
                                        ui,
                                        &format!("[{i}] U Filter"),
                                        tex.u_options.filter,
                                    );
                                    draw_prop_debug(
                                        ui,
                                        &format!("[{i}] V Filter"),
                                        tex.v_options.filter,
                                    );
                                    draw_prop_debug(
                                        ui,
                                        &format!("[{i}] U Wrap"),
                                        tex.u_options.wrap_mode,
                                    );
                                    draw_prop_debug(
                                        ui,
                                        &format!("[{i}] V Wrap"),
                                        tex.v_options.wrap_mode,
                                    );
                                });
                            });
                        }

                        if !material.tex_extensions.is_empty() {
                            egui::CollapsingHeader::new(format!(
                                "Texture Extensions ({})",
                                material.tex_extensions.len()
                            ))
                            .id_salt("tex_ext")
                            .show(ui, |ui| {
                                draw_vec_grid(
                                    ui,
                                    "tex_ext_grid",
                                    &material.tex_extensions,
                                    |ui, i, ext| {
                                        draw_prop(
                                            ui,
                                            &format!("[{i}] Capture Tex"),
                                            ext.is_capture_texture,
                                        );
                                        draw_prop(
                                            ui,
                                            &format!("[{i}] Vector Tex"),
                                            ext.is_vector_texture,
                                        );
                                    },
                                );
                            });
                        }

                        if !material.tex_srts.is_empty() {
                            egui::CollapsingHeader::new(format!(
                                "Texture SRTs ({})",
                                material.tex_srts.len()
                            ))
                            .id_salt("tex_srt")
                            .show(ui, |ui| {
                                draw_vec_grid(ui, "srt_grid", &material.tex_srts, |ui, i, srt| {
                                    draw_prop_f32(ui, &format!("[{i}] Rotate"), srt.rotate);
                                    draw_prop_f32(ui, &format!("[{i}] Scale U"), srt.scale_u);
                                    draw_prop_f32(ui, &format!("[{i}] Scale V"), srt.scale_v);
                                    draw_prop_f32(
                                        ui,
                                        &format!("[{i}] Translate U"),
                                        srt.translate_u,
                                    );
                                    draw_prop_f32(
                                        ui,
                                        &format!("[{i}] Translate V"),
                                        srt.translate_v,
                                    );
                                });
                            });
                        }

                        if !material.tex_coord_gens.is_empty() {
                            egui::CollapsingHeader::new(format!(
                                "Texture Coord Gens ({})",
                                material.tex_coord_gens.len()
                            ))
                            .id_salt("tex_gen")
                            .show(ui, |ui| {
                                draw_vec_grid(
                                    ui,
                                    "coord_gen_grid",
                                    &material.tex_coord_gens,
                                    |ui, i, coord_gen| {
                                        draw_prop_debug(
                                            ui,
                                            &format!("[{i}] Source"),
                                            coord_gen.tex_gen_source,
                                        );
                                    },
                                );
                            });
                        }

                        if !material.projection_tex_gens.is_empty() {
                            egui::CollapsingHeader::new(format!(
                                "Projection Tex Gens ({})",
                                material.projection_tex_gens.len()
                            ))
                            .id_salt("proj_gen")
                            .show(ui, |ui| {
                                draw_vec_grid(
                                    ui,
                                    "proj_gen_grid",
                                    &material.projection_tex_gens,
                                    |ui, i, proj_gen| {
                                        draw_prop(
                                            ui,
                                            &format!("[{i}] Adjust Projection Scale Rotate"),
                                            proj_gen.flags.adjust_projection_scale_rotate,
                                        );

                                        draw_prop(
                                            ui,
                                            &format!("[{i}] Fitting Layout Size"),
                                            proj_gen.flags.fitting_layout_size,
                                        );

                                        draw_prop(
                                            ui,
                                            &format!("[{i}] Fitting Pane Size"),
                                            proj_gen.flags.fitting_pane_size,
                                        );

                                        draw_vector_2f(
                                            ui,
                                            &format!("[{i}] Translation"),
                                            proj_gen.scale,
                                        );
                                        draw_vector_2f(
                                            ui,
                                            &format!("[{i}] Scale"),
                                            proj_gen.translation,
                                        );
                                    },
                                );
                            });
                        }

                        if !material.tev_combiners.is_empty() {
                            egui::CollapsingHeader::new(format!(
                                "Texture Environment Combiners ({})",
                                material.tev_combiners.len()
                            ))
                            .id_salt("tev_comb")
                            .show(ui, |ui| {
                                draw_vec_grid(
                                    ui,
                                    "tev_grid",
                                    &material.tev_combiners,
                                    |ui, i, combiner| {
                                        draw_prop_debug(
                                            ui,
                                            &format!("[{i}] RGB Mode"),
                                            combiner.rgb_mode,
                                        );
                                        draw_prop_debug(
                                            ui,
                                            &format!("[{i}] Alpha Mode"),
                                            combiner.alpha_mode,
                                        );
                                    },
                                );
                            });
                        }

                        if material.alpha_compare.is_some() {
                            egui::CollapsingHeader::new("Alpha Compare")
                                .id_salt("alp_comp")
                                .show(ui, |ui| {
                                    if let Some(compare) = &material.alpha_compare {
                                        egui::Grid::new(ui.id().with("alpha_comp_grid"))
                                            .striped(true)
                                            .show(ui, |ui| {
                                                draw_prop_debug(ui, "Compare OP", compare.compare);
                                                draw_prop_f32(
                                                    ui,
                                                    "Reference Value",
                                                    compare.alpha_compare_ref_value,
                                                );
                                            });
                                    } else {
                                        ui.weak("None");
                                    }
                                });
                        }

                        if material.blend_mode.is_some() {
                            egui::CollapsingHeader::new("Blend Mode")
                                .id_salt("blend_mode")
                                .show(ui, |ui| {
                                    egui::Grid::new(ui.id().with("blend_grid"))
                                        .striped(true)
                                        .show(ui, |ui| {
                                            draw_blend_mode(ui, &material.blend_mode);
                                        });
                                });
                        }

                        if material.blend_mode_alpha.is_some() {
                            egui::CollapsingHeader::new("Alpha Blend Mode")
                                .id_salt("alp_blend_mode")
                                .show(ui, |ui| {
                                    egui::Grid::new(ui.id().with("alpha_blend_grid"))
                                        .striped(true)
                                        .show(ui, |ui| {
                                            draw_blend_mode(ui, &material.blend_mode_alpha);
                                        });
                                });
                        }

                        if let Some(indirect_matrix) = &material.indirect_matrix {
                            egui::CollapsingHeader::new("Indirect Matrix")
                                .id_salt("ind_mtx")
                                .show(ui, |ui| {
                                    egui::Grid::new(ui.id().with("ind_mtx_grid"))
                                        .striped(true)
                                        .show(ui, |ui| {
                                            draw_prop(ui, "Rotation", indirect_matrix.rotation);
                                            draw_vector_2f(ui, "Scale", indirect_matrix.scale);
                                        });
                                });
                        }

                        if let Some(fcs) = &material.font_shadow_color {
                            egui::CollapsingHeader::new("Font Shadow Color")
                                .id_salt("f_sh_clr")
                                .show(ui, |ui| {
                                    egui::Grid::new(ui.id().with("f_sh_clr_grid"))
                                        .striped(true)
                                        .show(ui, |ui| {
                                            draw_prop(ui, "Color 1, Red", fcs.black_color.r);
                                            draw_prop(ui, "Color 1, Green", fcs.black_color.g);
                                            draw_prop(ui, "Color 1, Blue", fcs.black_color.b);
                                            draw_prop(ui, "Color 1, Alpha", fcs.black_color.a);
                                            draw_prop(ui, "Color 2, Red", fcs.white_color.r);
                                            draw_prop(ui, "Color 2, Green", fcs.white_color.g);
                                            draw_prop(ui, "Color 2, Blue", fcs.white_color.b);
                                            draw_prop(ui, "Color 2, Alpha", fcs.white_color.a);
                                        });
                                });
                        }

                        if let Some(dc) = &material.detailed_combiner {
                            egui::CollapsingHeader::new("Detailed Combiner")
                                .id_salt("dc_comb")
                                .show(ui, |ui| {
                                    egui::Grid::new(ui.id().with("dc_comb_grid"))
                                        .striped(true)
                                        .show(ui, |ui| {
                                            draw_prop(ui, "Stage Flags", dc.stage_flags);

                                            draw_prop(ui, "Color 1, Red", dc.color1.r);
                                            draw_prop(ui, "Color 1, Green", dc.color1.g);
                                            draw_prop(ui, "Color 1, Blue", dc.color1.b);
                                            draw_prop(ui, "Color 1, Alpha", dc.color1.a);
                                            draw_prop(ui, "Color 2, Red", dc.color2.r);
                                            draw_prop(ui, "Color 2, Green", dc.color2.g);
                                            draw_prop(ui, "Color 2, Blue", dc.color2.b);
                                            draw_prop(ui, "Color 2, Alpha", dc.color2.a);

                                            draw_prop(ui, "Color 3, Red", dc.color3.r);
                                            draw_prop(ui, "Color 3, Green", dc.color3.g);
                                            draw_prop(ui, "Color 3, Blue", dc.color3.b);
                                            draw_prop(ui, "Color 3, Alpha", dc.color3.a);
                                            draw_prop(ui, "Color 4, Red", dc.color4.r);
                                            draw_prop(ui, "Color 4, Green", dc.color4.g);
                                            draw_prop(ui, "Color 4, Blue", dc.color4.b);
                                            draw_prop(ui, "Color 4, Alpha", dc.color4.a);

                                            draw_prop(ui, "Color 5, Red", dc.color5.r);
                                            draw_prop(ui, "Color 5, Green", dc.color5.g);
                                            draw_prop(ui, "Color 5, Blue", dc.color5.b);
                                            draw_prop(ui, "Color 5, Alpha", dc.color5.a);

                                            draw_vec_grid(
                                                ui,
                                                "dc_entries",
                                                &dc.entries,
                                                |ui, i, entry| {
                                                    draw_prop(
                                                        ui,
                                                        &format!("[{i}] Alpha Config Copy Reg"),
                                                        entry.alpha_config.copy_reg,
                                                    );

                                                    draw_prop_debug(
                                                        ui,
                                                        &format!("[{i}] Alpha Config Konst Sel"),
                                                        entry.alpha_config.konst_sel,
                                                    );

                                                    draw_prop_debug(
                                                        ui,
                                                        &format!("[{i}] Alpha Config Mode"),
                                                        entry.alpha_config.mode,
                                                    );

                                                    draw_prop_debug(
                                                        ui,
                                                        &format!("[{i}] Alpha Config Scale"),
                                                        entry.alpha_config.scale,
                                                    );

                                                    draw_prop_debug(
                                                        ui,
                                                        &format!("[{i}] Alpha Config Operand 1"),
                                                        entry.alpha_config.operands[0],
                                                    );

                                                    draw_prop_debug(
                                                        ui,
                                                        &format!("[{i}] Alpha Config Operand 2"),
                                                        entry.alpha_config.operands[1],
                                                    );

                                                    draw_prop_debug(
                                                        ui,
                                                        &format!("[{i}] Alpha Config Operand 3"),
                                                        entry.alpha_config.operands[2],
                                                    );

                                                    draw_prop_debug(
                                                        ui,
                                                        &format!("[{i}] Alpha Config Source 1"),
                                                        entry.alpha_config.sources[0],
                                                    );

                                                    draw_prop_debug(
                                                        ui,
                                                        &format!("[{i}] Alpha Config Source 2"),
                                                        entry.alpha_config.sources[1],
                                                    );

                                                    draw_prop_debug(
                                                        ui,
                                                        &format!("[{i}] Alpha Config Source 3"),
                                                        entry.alpha_config.sources[2],
                                                    );

                                                    draw_prop(
                                                        ui,
                                                        &format!("[{i}] Color Config Copy Reg"),
                                                        entry.color_config.copy_reg,
                                                    );

                                                    draw_prop_debug(
                                                        ui,
                                                        &format!("[{i}] Color Config Konst Sel"),
                                                        entry.color_config.konst_sel,
                                                    );

                                                    draw_prop_debug(
                                                        ui,
                                                        &format!("[{i}] Color Config Mode"),
                                                        entry.color_config.mode,
                                                    );

                                                    draw_prop_debug(
                                                        ui,
                                                        &format!("[{i}] Color Config Scale"),
                                                        entry.color_config.scale,
                                                    );

                                                    draw_prop_debug(
                                                        ui,
                                                        &format!("[{i}] Color Config Operand 1"),
                                                        entry.color_config.operands[0],
                                                    );

                                                    draw_prop_debug(
                                                        ui,
                                                        &format!("[{i}] Color Config Operand 2"),
                                                        entry.color_config.operands[1],
                                                    );

                                                    draw_prop_debug(
                                                        ui,
                                                        &format!("[{i}] Color Config Operand 3"),
                                                        entry.color_config.operands[2],
                                                    );

                                                    draw_prop_debug(
                                                        ui,
                                                        &format!("[{i}] Color Config Source 1"),
                                                        entry.color_config.sources[0],
                                                    );

                                                    draw_prop_debug(
                                                        ui,
                                                        &format!("[{i}] Color Config Source 2"),
                                                        entry.color_config.sources[1],
                                                    );

                                                    draw_prop_debug(
                                                        ui,
                                                        &format!("[{i}] Color Config Source 3"),
                                                        entry.color_config.sources[2],
                                                    );
                                                },
                                            );
                                        });
                                });
                        }
                    });
            }
        });
}

fn draw_vec_grid<T>(
    ui: &mut Ui,
    id_source: &str,
    items: &[T],
    mut draw_item: impl FnMut(&mut Ui, usize, &T),
) {
    if items.is_empty() {
        ui.weak("None");
        return;
    }

    let len = items.len();
    egui::Grid::new(ui.id().with(id_source))
        .striped(true)
        .show(ui, |ui| {
            for (i, item) in items.iter().enumerate() {
                draw_item(ui, i, item);

                if i < len - 1 {
                    ui.label("-");
                    ui.label("-");
                    ui.end_row();
                }
            }
        });
}

fn draw_blend_mode(ui: &mut Ui, blend_mode: &Option<MaterialBlendMode>) {
    if let Some(blend_mode) = blend_mode {
        match blend_mode {
            MaterialBlendMode::None => {
                ui.weak("None");
            }
            MaterialBlendMode::Logic { logic_op } => {
                draw_prop_debug(ui, "Logic OP", logic_op);
            }
            MaterialBlendMode::Blend {
                blend_op,
                function_source,
                function_destination,
            } => {
                draw_prop_debug(ui, "Blend OP", blend_op);
                draw_prop_debug(ui, "Function Source", function_source);
                draw_prop_debug(ui, "Function Destination", function_destination);
            }
        }
    } else {
        ui.weak("None");
    }
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
