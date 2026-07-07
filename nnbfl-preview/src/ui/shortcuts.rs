use crate::ui::DrawUi;

#[derive(Default)]
pub struct Shortcuts {
    pub is_visible: bool,
}

impl DrawUi<()> for Shortcuts {
    fn draw(&mut self, ui: &mut egui::Ui) {
        egui::Window::new("Keyboard Shortcuts")
            .open(&mut self.is_visible)
            .collapsible(false)
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
    }
}
