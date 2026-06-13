use crate::{persistence::RecentFiles, theme::ThemeStore};

#[derive(Default)]
pub struct LauncherView;

impl LauncherView {
    pub fn ui(&mut self, ui: &mut egui::Ui, recent_files: &RecentFiles, themes: &ThemeStore) {
        ui.vertical_centered(|ui| {
            ui.heading("NeoNote");
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                let _ = ui.button("New File");
                let _ = ui.button("Open File");
                let _ = ui.button("Open Folder");
            });
            ui.add_space(16.0);
            ui.label(format!("Recent files: {}", recent_files.entries().len()));
            ui.label(format!("Themes loaded: {}", themes.all().len()));
        });
    }
}
