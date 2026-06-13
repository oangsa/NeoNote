#[derive(Default)]
pub struct TitleBar {
    maximized: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TitleBarAction {
    Minimize,
    ToggleMaximize,
    Close,
}

impl TitleBar {
    pub fn ui(&mut self, ui: &mut egui::Ui, title: &str, modified: bool) -> Option<TitleBarAction> {
        let mut action = None;
        let title = if modified {
            format!("{title} *")
        } else {
            title.to_string()
        };

        let available = ui.available_width();
        let drag_width = (available - 124.0).max(80.0);

        ui.horizontal(|ui| {
            let drag_response = ui
                .allocate_ui_with_layout(
                    egui::vec2(drag_width, 28.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.strong("NeoNote");
                        ui.separator();
                        ui.label(title);
                    },
                )
                .response
                .interact(egui::Sense::click_and_drag());

            if drag_response.drag_started() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if drag_response.double_clicked() {
                action = Some(TitleBarAction::ToggleMaximize);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("x").on_hover_text("Close").clicked() {
                    action = Some(TitleBarAction::Close);
                }
                let maximize_label = if self.maximized { "[]" } else { "[ ]" };
                if ui.button(maximize_label).on_hover_text("Maximize").clicked() {
                    action = Some(TitleBarAction::ToggleMaximize);
                }
                if ui.button("_").on_hover_text("Minimize").clicked() {
                    action = Some(TitleBarAction::Minimize);
                }
            });
        });

        action
    }

    pub fn toggle_maximized(&mut self) -> bool {
        self.maximized = !self.maximized;
        self.maximized
    }
}
