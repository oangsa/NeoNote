#[derive(Default)]
pub struct Sidebar;

impl Sidebar {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label("Sidebar");
    }
}
