use crate::theme::ThemeStore;

#[derive(Default)]
pub struct ThemePanel {
    open: bool,
}

impl ThemePanel {
    pub fn ui(&mut self, ctx: &egui::Context, themes: &mut ThemeStore) {
        if !self.open {
            return;
        }

        egui::Window::new("Themes")
            .open(&mut self.open)
            .show(ctx, |ui| {
                for theme in themes.all() {
                    ui.horizontal(|ui| {
                        ui.label(&theme.name);
                        ui.label(&theme.author);
                    });
                }
            });
    }
}
