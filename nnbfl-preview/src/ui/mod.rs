pub mod archive_browser;
pub mod context_menu;
pub mod editors;
pub mod general;
pub mod shortcuts;
pub mod timeline;
pub mod tree_view;

pub trait DrawUiWith<T = (), O = bool> {
    fn draw_with(&mut self, ui: &mut egui::Ui, state: T) -> O;
}

pub trait DrawUi<O = bool> {
    fn draw(&mut self, ui: &mut egui::Ui) -> O;
}

impl<U: DrawUiWith<()>> DrawUi for U {
    fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        self.draw_with(ui, ())
    }
}
