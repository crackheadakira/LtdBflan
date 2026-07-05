pub trait DrawUiWith<T> {
    fn draw_with_mut(&mut self, _ui: &mut egui::Ui, _state: &mut T) -> bool {
        false
    }

    fn draw_with(&mut self, _ui: &mut egui::Ui, _state: T) -> bool {
        false
    }
}

pub trait DrawUi {
    fn draw(&mut self, ui: &mut egui::Ui) -> bool;
}
