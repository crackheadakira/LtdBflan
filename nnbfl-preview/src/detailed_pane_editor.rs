use nnbfl::bflyt::pane::PANE_NAME_LEN;

use crate::{material_editor::DrawUiWith, pane_tree::PaneNode};

#[derive(Debug, Clone, Copy, Default)]
pub struct DetailedPaneEditor {
    pub is_editor_visible: bool,
}

impl DrawUiWith<PaneNode> for DetailedPaneEditor {
    fn draw_with_mut(&mut self, ui: &mut egui::Ui, pane: &mut PaneNode) -> bool {
        let mut changed = false;

        egui::Window::new("Detailed Pane Editor")
            .collapsible(false)
            .resizable(true)
            .open(&mut self.is_editor_visible)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Label:");
                    changed |= ui
                        .add(egui::TextEdit::singleline(&mut pane.label).char_limit(PANE_NAME_LEN))
                        .clicked();
                });
            });

        changed
    }
}
