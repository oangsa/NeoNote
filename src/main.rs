#![allow(dead_code)]

mod app;
mod notes;
mod persistence;
mod platform;
mod theme;
mod ui;

use app::NeoNoteApp;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("NeoNote")
            .with_inner_size([1280.0, 800.0])
            .with_decorations(false),
        ..Default::default()
    };

    eframe::run_native(
        "NeoNote",
        options,
        Box::new(|cc| Ok(Box::new(NeoNoteApp::new(cc)))),
    )
}
