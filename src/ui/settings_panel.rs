use crate::persistence::AppConfig;

#[derive(Default)]
pub struct SettingsPanel {
    open: bool,
}

impl SettingsPanel {
    pub fn ui(&mut self, ctx: &egui::Context, config: &mut AppConfig) {
        if !self.open {
            return;
        }

        egui::Window::new("Settings")
            .open(&mut self.open)
            .show(ctx, |ui| {
                ui.label("General");
                ui.checkbox(&mut config.restore_last_session, "Restore last session");
                ui.checkbox(
                    &mut config.show_launcher_on_startup,
                    "Show launcher on startup",
                );
                ui.add(egui::Slider::new(&mut config.font_size, 8.0..=32.0).text("Font size"));
            });
    }
}
