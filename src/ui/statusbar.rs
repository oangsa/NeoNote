#[derive(Default)]
pub struct StatusBar {
    mode: String,
}

impl StatusBar {
    pub fn ui(&mut self, ui: &mut egui::Ui, file_title: &str) {
        if self.mode.is_empty() {
            self.mode = "--".to_string();
        }

        ui.horizontal(|ui| {
            ui.monospace(format!("[{}]", self.mode));
            ui.separator();
            ui.label(file_title);
            ui.separator();
            ui.label("utf-8");
            ui.separator();
            ui.label("Ln 1  Col 1");
        });
    }
}
