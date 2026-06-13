#[derive(Default)]
pub struct TitleBar;

impl TitleBar {
    pub fn ui(&mut self, ui: &mut egui::Ui, title: &str) {
        ui.horizontal(|ui| {
            ui.strong("NeoNote");
            ui.separator();
            ui.label(title);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.button("x").on_hover_text("Close");
                ui.button("[]").on_hover_text("Maximize");
                ui.button("_").on_hover_text("Minimize");
            });
        });
    }
}
