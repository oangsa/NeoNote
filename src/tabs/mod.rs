pub mod tab;

use egui::Ui;

use self::tab::Tab;

#[derive(Default)]
pub struct TabManager {
    tabs: Vec<Tab>,
    active_index: Option<usize>,
}

impl TabManager {
    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    pub fn active_title(&self) -> &str {
        self.active_index
            .and_then(|index| self.tabs.get(index))
            .map(|tab| tab.title.as_str())
            .unwrap_or("NeoNote")
    }

    pub fn push(&mut self, tab: Tab) {
        self.tabs.push(tab);
        self.active_index = Some(self.tabs.len() - 1);
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if self.tabs.is_empty() {
                ui.label("No tabs");
            }

            for (index, tab) in self.tabs.iter().enumerate() {
                let selected = Some(index) == self.active_index;
                let label = if tab.modified {
                    format!("* {}", tab.title)
                } else {
                    tab.title.clone()
                };
                if ui.selectable_label(selected, label).clicked() {
                    self.active_index = Some(index);
                }
            }

            ui.button("+").on_hover_text("New tab");
        });
    }
}
