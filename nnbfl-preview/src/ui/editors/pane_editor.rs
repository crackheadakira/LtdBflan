use nnbfl::{
    bflyt::pane::PANE_NAME_LEN,
    ui2d::userdata::{UserData, UserDataContent},
};

use crate::{
    pane_tree::PaneNode,
    ui::{DrawUi, DrawUiWith},
};

#[derive(Debug, Clone, Copy, Default)]
pub struct PaneEditor {
    pub is_editor_visible: bool,
}

impl DrawUiWith<&mut PaneNode> for PaneEditor {
    fn draw_with(&mut self, ui: &mut egui::Ui, pane: &mut PaneNode) -> bool {
        let mut changed = false;

        egui::Window::new("Detailed Pane Editor")
            .collapsible(false)
            .resizable(true)
            .open(&mut self.is_editor_visible)
            .show(ui, |ui| {
                ui.heading("Pane Name");

                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.add(egui::TextEdit::singleline(&mut pane.label).char_limit(PANE_NAME_LEN));
                });

                ui.separator();

                ui.horizontal(|ui| {
                    ui.heading("User Data");

                    if pane.user_data.is_none() && ui.button("➕ Add").clicked() {
                        pane.user_data = Some(Default::default());
                        changed |= true;
                    }
                });

                if let Some(user_data_array) = &mut pane.user_data {
                    for user_data in user_data_array.user_data.iter_mut() {
                        changed |= user_data.draw(ui);

                        ui.add_space(8.0);
                    }
                }
            });

        changed
    }
}

impl DrawUi for UserData {
    fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        ui.horizontal(|ui| {
            changed |= ui
                .add(egui::TextEdit::singleline(&mut self.o_name))
                .changed();

            let current_type = match &self.content {
                UserDataContent::String(_) => "String",
                UserDataContent::S32(_) => "Int Array",
                UserDataContent::Float(_) => "Float Array",
                UserDataContent::SystemData(_) => "System Data Array",
            };

            egui::ComboBox::from_id_salt(format!("type_select_{}", self.o_name))
                .selected_text(current_type)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(current_type == "String", "String")
                        .clicked()
                    {
                        self.content = UserDataContent::String(Default::default());
                        changed |= true;
                    }

                    if ui
                        .selectable_label(current_type == "Int Array", "Int Array")
                        .clicked()
                    {
                        self.content = UserDataContent::S32(Default::default());
                        changed |= true;
                    }

                    if ui
                        .selectable_label(current_type == "Float Array", "Float Array")
                        .clicked()
                    {
                        self.content = UserDataContent::Float(Default::default());
                        changed |= true;
                    }

                    if ui
                        .selectable_label(current_type == "System Data Array", "System Data Array")
                        .clicked()
                    {
                        self.content = UserDataContent::SystemData(Default::default());
                        changed |= true;
                    }
                });
        });

        ui.add_space(4.0);

        changed |= match &mut self.content {
            UserDataContent::String(string) => ui.add(egui::TextEdit::singleline(string)).changed(),
            UserDataContent::S32(items) => {
                ui.horizontal(|ui| {
                    let mut vec_changed = false;

                    for item in items.iter_mut() {
                        let mut val_str = item.to_string();
                        let response =
                            ui.add(egui::TextEdit::singleline(&mut val_str).desired_width(40.0));

                        if response.changed()
                            && let Ok(parsed) = val_str.parse::<i32>()
                        {
                            *item = parsed;
                            vec_changed = true;
                        }
                    }

                    if ui.button("➕").clicked() {
                        items.push(0);
                        vec_changed = true;
                    }

                    if !items.is_empty() && ui.button("🗑").clicked() {
                        items.pop();
                        vec_changed = true;
                    }

                    vec_changed
                })
                .inner
            }
            UserDataContent::Float(items) => {
                ui.horizontal(|ui| {
                    let mut vec_changed = false;

                    for item in items.iter_mut() {
                        let mut val_str = item.to_string();
                        let response =
                            ui.add(egui::TextEdit::singleline(&mut val_str).desired_width(50.0));

                        if response.changed()
                            && let Ok(parsed) = val_str.parse::<f32>()
                        {
                            *item = parsed;
                            vec_changed = true;
                        }
                    }

                    if ui.button("➕").clicked() {
                        items.push(0.0);
                        vec_changed = true;
                    }
                    if !items.is_empty() && ui.button("🗑").clicked() {
                        items.pop();
                        vec_changed = true;
                    }

                    vec_changed
                })
                .inner
            }
            UserDataContent::SystemData(_items) => {
                ui.weak("System Data Blob (Editing Unsupported)");
                false
            }
        };

        changed
    }
}
