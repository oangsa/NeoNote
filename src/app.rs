use std::path::PathBuf;

use crate::{
    notes::{NoteDocument, VimMode},
    persistence::{AppConfig, AppDataPaths, RecentFiles, SessionState},
    theme::ThemeStore,
    ui::{
        launcher::{LauncherAction, LauncherView},
        settings_panel::SettingsPanel,
        statusbar::StatusBar,
        theme_panel::{ThemePanel, ThemePanelAction},
        titlebar::{TitleBar, TitleBarAction},
        toast::ToastStack,
    },
};

pub struct NeoNoteApp {
    paths: AppDataPaths,
    config: AppConfig,
    session: SessionState,
    recent_files: RecentFiles,
    themes: ThemeStore,
    note: NoteDocument,
    titlebar: TitleBar,
    statusbar: StatusBar,
    launcher: LauncherView,
    theme_panel: ThemePanel,
    settings_panel: SettingsPanel,
    toasts: ToastStack,
    focus_editor: bool,
}

impl NeoNoteApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let paths = AppDataPaths::new();
        let config = AppConfig::load_or_default(&paths);
        let session = SessionState::load_or_default(&paths);
        let recent_files = RecentFiles::load_or_default(&paths);
        let themes = ThemeStore::load(&paths, config.active_theme.as_deref());

        if let Some(theme) = themes.active_theme() {
            cc.egui_ctx.set_visuals(theme.to_visuals());
        }

        Self {
            paths,
            config,
            session,
            recent_files,
            themes,
            note: NoteDocument::default(),
            titlebar: TitleBar::default(),
            statusbar: StatusBar::default(),
            launcher: LauncherView,
            theme_panel: ThemePanel::default(),
            settings_panel: SettingsPanel::default(),
            toasts: ToastStack::default(),
            focus_editor: false,
        }
    }

    fn new_note(&mut self) {
        if self.note.dirty() {
            self.toasts.push("Save the current note before creating a new one.");
            return;
        }
        self.note.new_blank();
        self.focus_editor = true;
    }

    fn open_note_dialog(&mut self) {
        if self.note.dirty() {
            self.toasts.push("Save the current note before opening another file.");
            return;
        }

        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Text", &["txt"])
            .add_filter("Markdown", &["md", "markdown"])
            .set_file_name("note.txt")
            .pick_file()
        {
            self.open_path(path);
        }
    }

    fn open_path(&mut self, path: PathBuf) {
        match self.note.open(&path) {
            Ok(()) => {
                self.remember_recent(path);
                self.focus_editor = true;
            }
            Err(error) => self.toasts.push(format!("Could not open note: {error}")),
        }
    }

    fn save_note(&mut self) {
        if self.note.path_string().is_some() {
            match self.note.save() {
                Ok(()) => {
                    if let Some(path) = self.note.path_string() {
                        self.remember_recent(PathBuf::from(path));
                    }
                }
                Err(error) => self.toasts.push(format!("Could not save note: {error}")),
            }
        } else {
            self.save_note_as();
        }
    }

    fn save_note_as(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Text", &["txt"])
            .add_filter("Markdown", &["md"])
            .set_file_name("note.txt")
            .save_file()
        {
            match self.note.save_as(&path) {
                Ok(()) => self.remember_recent(path),
                Err(error) => self.toasts.push(format!("Could not save note: {error}")),
            }
        }
    }

    fn remember_recent(&mut self, path: PathBuf) {
        self.recent_files
            .upsert(path.display().to_string(), chrono_like_now());
        if let Err(error) = self.recent_files.save(&self.paths) {
            self.toasts
                .push(format!("Could not update recent files: {error}"));
        }
    }

    fn render_menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menubar")
            .exact_height(28.0)
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("New File").clicked() {
                            self.new_note();
                            ui.close_menu();
                        }
                        if ui.button("Open File").clicked() {
                            self.open_note_dialog();
                            ui.close_menu();
                        }
                        if ui.button("Open Folder").clicked() {
                            self.toasts.push("Folder browsing is planned for the sidebar.");
                            ui.close_menu();
                        }
                        if ui.button("Save").clicked() {
                            self.save_note();
                            ui.close_menu();
                        }
                        if ui.button("Save As").clicked() {
                            self.save_note_as();
                            ui.close_menu();
                        }
                        ui.menu_button("Recent Files", |ui| {
                            let entries = self.recent_files.entries().to_vec();
                            for entry in entries {
                                if ui.button(&entry.path).clicked() {
                                    if self.note.dirty() {
                                        self.toasts.push(
                                            "Save the current note before opening another file.",
                                        );
                                    } else {
                                        self.open_path(PathBuf::from(entry.path));
                                    }
                                    ui.close_menu();
                                }
                            }
                        });
                        if ui.button("Exit").clicked() {
                            if !self.note.dirty() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            } else {
                                self.toasts.push("Save the current note before exiting.");
                            }
                            ui.close_menu();
                        }
                    });
                    ui.menu_button("View", |ui| {
                        if ui.button("Toggle Status Bar").clicked() {
                            self.toasts
                                .push("Status bar toggle will be persisted in Phase 6 settings.");
                            ui.close_menu();
                        }
                        if ui.button("Zen Mode").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
                            ui.close_menu();
                        }
                    });
                    ui.menu_button("Theme", |ui| {
                        if ui.button("Theme Panel").clicked() {
                            self.theme_panel.open();
                            ui.close_menu();
                        }
                    });
                });
            });
    }

    fn render_editor_shell(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("titlebar")
            .exact_height(36.0)
            .show(ctx, |ui| {
                if let Some(action) = self.titlebar.ui(ui, &self.note.title(), self.note.dirty()) {
                    self.handle_titlebar_action(ctx, action);
                }
            });

        self.render_menu_bar(ctx);

        egui::TopBottomPanel::bottom("statusbar")
            .exact_height(26.0)
            .show(ctx, |ui| {
                self.statusbar.ui_note(ui, &self.note);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if !self.note.is_open() {
                ui.centered_and_justified(|ui| {
                    match self.launcher.ui(ui, &self.recent_files, &self.themes) {
                        Some(LauncherAction::NewFile) => self.new_note(),
                        Some(LauncherAction::OpenFile) => self.open_note_dialog(),
                        Some(LauncherAction::OpenFolder) => {
                            self.toasts.push("Folder browsing is planned for the sidebar.");
                        }
                        None => {}
                    }
                });
            } else {
                self.render_vim_editor(ctx, ui);
            }
        });
    }

    fn render_vim_editor(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::click());
        if response.clicked() || self.focus_editor {
            response.request_focus();
            self.focus_editor = false;
        }

        if response.has_focus() {
            self.handle_editor_input(ctx);
        }

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, ui.visuals().panel_fill);
        let font = egui::FontId::monospace(15.0);
        let row_height = 20.0;
        let gutter_width = 54.0;
        let cursor_line = self.note.cursor_line();

        for (index, line) in self.note.content().split('\n').enumerate() {
            let y = rect.top() + 8.0 + index as f32 * row_height;
            if y > rect.bottom() {
                break;
            }

            let line_rect = egui::Rect::from_min_size(
                egui::pos2(rect.left(), y - 2.0),
                egui::vec2(rect.width(), row_height),
            );
            if index == cursor_line {
                painter.rect_filled(line_rect, 0.0, ui.visuals().faint_bg_color);
            }

            painter.text(
                egui::pos2(rect.left() + 8.0, y),
                egui::Align2::LEFT_TOP,
                format!("{:>4}", index + 1),
                font.clone(),
                ui.visuals().weak_text_color(),
            );
            painter.text(
                egui::pos2(rect.left() + gutter_width, y),
                egui::Align2::LEFT_TOP,
                self.note.normal_mode_line_with_cursor(line, index),
                font.clone(),
                ui.visuals().strong_text_color(),
            );
        }

        if self.note.content().is_empty() {
            painter.text(
                egui::pos2(rect.left() + gutter_width, rect.top() + 8.0),
                egui::Align2::LEFT_TOP,
                "|",
                font,
                ui.visuals().strong_text_color(),
            );
        }
    }

    fn handle_editor_input(&mut self, ctx: &egui::Context) {
        let events = ctx.input(|input| input.events.clone());
        for event in events {
            match event {
                egui::Event::Key {
                    key: egui::Key::Escape,
                    pressed: true,
                    ..
                } => self.note.enter_normal(),
                egui::Event::Key {
                    key: egui::Key::Enter,
                    pressed: true,
                    ..
                } if self.note.mode() == VimMode::Insert => self.note.insert_newline(),
                egui::Event::Key {
                    key: egui::Key::Backspace,
                    pressed: true,
                    ..
                } if self.note.mode() == VimMode::Insert => self.note.backspace(),
                egui::Event::Key {
                    key: egui::Key::Delete,
                    pressed: true,
                    ..
                } if self.note.mode() == VimMode::Insert => self.note.delete_at_cursor(),
                egui::Event::Text(text) if self.note.mode() == VimMode::Insert => {
                    self.note.handle_insert_text(&text);
                }
                egui::Event::Paste(text) if self.note.mode() == VimMode::Insert => {
                    self.note.handle_insert_text(&text);
                }
                egui::Event::Text(text) if self.note.mode() == VimMode::Normal => {
                    self.note.handle_normal_input(&text);
                }
                _ => {}
            }
        }
    }

    fn handle_titlebar_action(&mut self, ctx: &egui::Context, action: TitleBarAction) {
        match action {
            TitleBarAction::Minimize => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
            TitleBarAction::ToggleMaximize => {
                let maximized = self.titlebar.toggle_maximized();
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(maximized));
            }
            TitleBarAction::Close => {
                if !self.note.dirty() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                } else {
                    self.toasts.push("Save the current note before closing.");
                }
            }
        }
    }

    fn handle_theme_action(&mut self, ctx: &egui::Context, action: ThemePanelAction) {
        match action {
            ThemePanelAction::Preview(index) => {
                self.themes.preview(index);
                if let Some(theme) = self.themes.active_theme() {
                    ctx.set_visuals(theme.to_visuals());
                }
            }
            ThemePanelAction::ClearPreview => {
                self.themes.clear_preview();
                if let Some(theme) = self.themes.active_theme() {
                    ctx.set_visuals(theme.to_visuals());
                }
            }
            ThemePanelAction::Apply(index) => {
                let theme = self.themes.commit(index).cloned();
                if let Some(theme) = theme {
                    ctx.set_visuals(theme.to_visuals());
                    self.config.active_theme = Some(theme.slug());
                    if let Err(error) = self.config.save(&self.paths) {
                        self.toasts.push(format!("Could not save config: {error}"));
                    }
                }
            }
            ThemePanelAction::Imported(name) => self.toasts.push(format!("Imported theme: {name}")),
            ThemePanelAction::ImportFailed(error) => {
                self.toasts.push(format!("Theme import failed: {error}"))
            }
            ThemePanelAction::Saved(path) => self.toasts.push(format!("Saved theme: {path}")),
            ThemePanelAction::SaveFailed(error) => {
                self.toasts.push(format!("Theme save failed: {error}"))
            }
        }
    }
}

impl eframe::App for NeoNoteApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let _ = frame;
        if ctx.input(|input| {
            input.key_pressed(egui::Key::T) && input.modifiers.ctrl && input.modifiers.shift
        }) {
            self.theme_panel.open();
        }
        self.render_editor_shell(ctx);
        if let Some(action) = self.theme_panel.ui(ctx, &self.paths, &mut self.themes) {
            self.handle_theme_action(ctx, action);
        }
        self.settings_panel.ui(ctx, &mut self.config);
        self.toasts.ui(ctx);

        let _ = (&self.paths, &self.session);
    }
}

fn chrono_like_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    format!("{seconds}")
}
