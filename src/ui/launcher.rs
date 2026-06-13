use crate::{persistence::RecentFiles, theme::ThemeStore};

#[derive(Default)]
pub struct LauncherView;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LauncherAction {
    NewFile,
    OpenFile,
    OpenFolder,
}

impl LauncherView {
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        recent_files: &RecentFiles,
        themes: &ThemeStore,
    ) -> Option<LauncherAction> {
        let mut action = None;

        ui.vertical_centered(|ui| {
            ui.heading("NeoNote");
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.button("New File").clicked() {
                    action = Some(LauncherAction::NewFile);
                }
                if ui.button("Open File").clicked() {
                    action = Some(LauncherAction::OpenFile);
                }
                if ui.button("Open Folder").clicked() {
                    action = Some(LauncherAction::OpenFolder);
                }
            });
            ui.add_space(16.0);
            ui.label(format!("Recent files: {}", recent_files.entries().len()));
            ui.label(format!("Themes loaded: {}", themes.all().len()));
        });

        action
    }
}
