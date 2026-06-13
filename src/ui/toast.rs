#[derive(Default)]
pub struct ToastStack {
    messages: Vec<String>,
}

impl ToastStack {
    pub fn push(&mut self, message: impl Into<String>) {
        self.messages.push(message.into());
    }

    pub fn ui(&mut self, ctx: &egui::Context) {
        if self.messages.is_empty() {
            return;
        }

        egui::Area::new("toast_stack".into())
            .anchor(egui::Align2::RIGHT_BOTTOM, [-16.0, -16.0])
            .show(ctx, |ui| {
                for message in &self.messages {
                    ui.group(|ui| {
                        ui.label(message);
                    });
                }
            });
    }
}
