use crate::{
    pane_tree::PaneNode,
    ui::{DrawUiWith, general::UiAction},
};

#[derive(Default)]
pub struct ContextMenu {
    pub is_open: bool,
    pub pane_idx: usize,
    pub pos: egui::Pos2,
}

impl ContextMenu {
    pub fn open_context_menu(&mut self, screen_pos: [f32; 2], pane_idx: usize) {
        self.pane_idx = pane_idx;
        self.pos = egui::pos2(screen_pos[0], screen_pos[1]);
        self.is_open = true;
    }
}

pub struct ContextMenuState<'a> {
    pub node: &'a PaneNode,
    pub is_hidden: bool,
}

pub enum ContextMenuAction {
    HidePane(usize),
    ShowPane(usize),
    Action(UiAction),
}

impl DrawUiWith<ContextMenuState<'_>, Option<ContextMenuAction>> for ContextMenu {
    fn draw_with(
        &mut self,
        ui: &mut egui::Ui,
        state: ContextMenuState,
    ) -> Option<ContextMenuAction> {
        let mut out_action = None;

        if !self.is_open {
            return out_action;
        };

        let pane_idx = self.pane_idx;
        let pos = self.pos;

        let label = state.node.label.trim_end_matches("\0");
        let is_parts_content = state.node.parts_source.is_some();

        let area_response = egui::Area::new(egui::Id::new("pane_context_menu"))
        .fixed_pos(pos)
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(160.0);
                ui.label(egui::RichText::new(label).strong());
                ui.separator();

                let hidden = state.is_hidden;
                if ui.button(if hidden { "Show" } else { "Hide" }).clicked() {
                    if hidden {
                        out_action = Some(ContextMenuAction::ShowPane(pane_idx));
                    } else {
                        out_action = Some(ContextMenuAction::HidePane(pane_idx));
                    }
                }

                ui.separator();

                if is_parts_content {
                    ui.weak("Part of a linked layout - edit it via the PartsPane's overrides, not directly.");
                } else {
                    if ui.button("Duplicate").clicked() {
                        out_action = Some(ContextMenuAction::Action(UiAction::DuplicatePane(pane_idx)))
                    }

                    if ui
                        .add(egui::Button::new(
                            egui::RichText::new("Delete")
                                .color(egui::Color32::from_rgb(224, 96, 96)),
                        ))
                        .clicked()
                    {
                        out_action = Some(ContextMenuAction::Action(UiAction::DeletePane(pane_idx)));
                    }
                }
            });
        });

        if out_action.is_none() {
            let clicked_outside = ui.ctx().input(|i| i.pointer.any_click())
                && !area_response.response.contains_pointer();
            let escape_pressed = ui.ctx().input(|i| i.key_pressed(egui::Key::Escape));

            if clicked_outside || escape_pressed {
                self.is_open = false;
            }
        } else {
            self.is_open = false;
        }

        out_action
    }
}
