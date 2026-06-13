use crate::notes::NoteDocument;

#[derive(Default)]
pub struct StatusBar;

impl StatusBar {
    pub fn ui_note(&mut self, ui: &mut egui::Ui, note: &NoteDocument) {
        let stats = note.stats();
        ui.horizontal(|ui| {
            ui.monospace(format!(" {} ", note.mode().label()));
            ui.separator();
            ui.label(note.title());
            if let Some(path) = note.path_string() {
                ui.separator();
                ui.label(path);
            }
            ui.separator();
            ui.label(format!("Ln {}  Col {}", note.cursor_line() + 1, note.cursor_col() + 1));
            ui.separator();
            ui.label(format!("{} lines", stats.line_count));
            ui.separator();
            ui.label(format!("{} words", stats.word_count));
            ui.separator();
            ui.label(format!("{} chars", stats.char_count));
        });
    }
}
