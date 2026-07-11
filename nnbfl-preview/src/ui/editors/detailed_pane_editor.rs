use nnbfl::bflyt::pane::PANE_NAME_LEN;

use crate::{pane_tree::PaneNode, ui::DrawUiWith};

#[derive(Debug, Clone, Copy, Default)]
pub struct DetailedPaneEditor {
    pub is_editor_visible: bool,
}

impl DrawUiWith<&mut PaneNode> for DetailedPaneEditor {
    fn draw_with(&mut self, ui: &mut egui::Ui, pane: &mut PaneNode) -> bool {
        let mut changed = false;

        egui::Window::new("Detailed Pane Editor")
            .collapsible(false)
            .resizable(true)
            .open(&mut self.is_editor_visible)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Label:");
                    ui.add(egui::TextEdit::singleline(&mut pane.label).char_limit(PANE_NAME_LEN));
                });
            });

        changed
    }
}
