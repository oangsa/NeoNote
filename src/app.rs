use std::path::PathBuf;

use crate::{
    notes::{NoteDocument, RegisterSnapshot, VimMode, PendingCommand, ExCommandAction},
    persistence::{AppConfig, AppDataPaths, RecentFiles, SessionState},
    platform::clipboard,
    theme::ThemeStore,
    vim::key::{normalize_insert_key, normalize_normal_key, InsertKey, NormalKey},
};

pub struct AppController {
    paths: AppDataPaths,
    config: AppConfig,
    session: SessionState,
    recent_files: RecentFiles,
    themes: ThemeStore,
    documents: Vec<NoteDocument>,
    active_document: usize,
    mouse_selection: Option<MouseSelection>,
    theme_panel_open: bool,
    settings_panel_open: bool,
    last_message: String,
}

#[derive(Clone, Debug, Default)]
pub struct AppSnapshot {
    pub file_title: String,
    pub file_path: String,
    pub editor_lines: Vec<EditorLineSnapshot>,
    pub document_tabs: String,
    pub status_text: String,
    pub status_right: String,
    pub mode_text: String,
    pub mode_color: String,
    pub cursor_line: i32,
    pub cursor_column: i32,
    pub cursor_prefix: String,
    pub cursor_cell: String,
    pub cursor_suffix: String,
    pub cursor_block: bool,
    pub message: String,
    pub has_document: bool,
    pub editor_font_family: String,
    pub editor_font_size: f32,
    pub editor_line_height: f32,
    pub theme_panel_open: bool,
    pub theme_items: Vec<ThemeItemSnapshot>,
    pub settings_panel_open: bool,
    pub settings: SettingsSnapshot,
    pub theme: ThemeSnapshot,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ThemeItemSnapshot {
    pub index: i32,
    pub name: String,
    pub variant: String,
    pub author: String,
    pub is_active: bool,
    pub is_preview: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SettingsSnapshot {
    pub font_family: String,
    pub font_size: f32,
    pub font_size_label: String,
    pub line_height: f32,
    pub line_height_label: String,
    pub tab_size: i32,
    pub tab_size_label: String,
    pub word_wrap: bool,
    pub sync_clipboard: bool,
    pub restore_last_session: bool,
    pub show_launcher_on_startup: bool,
    pub remember_window_geometry: bool,
    pub blur_behind: bool,
    pub window_opacity: i32,
    pub window_opacity_label: String,
}

impl Default for SettingsSnapshot {
    fn default() -> Self {
        let config = AppConfig::default();
        Self::from_config(&config)
    }
}

impl SettingsSnapshot {
    fn from_config(config: &AppConfig) -> Self {
        Self {
            font_family: config.font_family.clone(),
            font_size: config.font_size,
            font_size_label: format!("{:.0}", config.font_size),
            line_height: config.line_height,
            line_height_label: format!("{:.1}", config.line_height),
            tab_size: i32::from(config.tab_size),
            tab_size_label: config.tab_size.to_string(),
            word_wrap: config.word_wrap,
            sync_clipboard: config.sync_clipboard,
            restore_last_session: config.restore_last_session,
            show_launcher_on_startup: config.show_launcher_on_startup,
            remember_window_geometry: config.remember_window_geometry,
            blur_behind: config.blur_behind,
            window_opacity: i32::from(config.window_opacity),
            window_opacity_label: format!("{}%", config.window_opacity),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EditorLineSnapshot {
    pub number: i32,
    pub text: String,
    pub is_cursor_line: bool,
    pub selected_prefix: String,
    pub selected_text: String,
    pub is_search_match: bool,
    pub cursor_column: i32,
    pub cursor_prefix: String,
    pub cursor_cell: String,
    pub cursor_suffix: String,
    pub cursor_block: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MouseSelection {
    anchor_line: usize,
    focus_line: usize,
}

impl MouseSelection {
    fn includes(self, line: usize) -> bool {
        let start = self.anchor_line.min(self.focus_line);
        let end = self.anchor_line.max(self.focus_line);
        start != end && (start..=end).contains(&line)
    }
}

#[derive(Clone, Debug)]
pub struct ThemeSnapshot {
    pub background: String,
    pub background_alt: String,
    pub surface: String,
    pub border: String,
    pub text: String,
    pub text_muted: String,
    pub accent_primary: String,
    pub accent_secondary: String,
    pub success: String,
    pub warning: String,
    pub error: String,
    pub cursor: String,
}

impl Default for ThemeSnapshot {
    fn default() -> Self {
        Self {
            background: "#0f1117".to_string(),
            background_alt: "#161922".to_string(),
            surface: "#1d2430".to_string(),
            border: "#303848".to_string(),
            text: "#d7dae0".to_string(),
            text_muted: "#7f8490".to_string(),
            accent_primary: "#7aa2f7".to_string(),
            accent_secondary: "#bb9af7".to_string(),
            success: "#9ece6a".to_string(),
            warning: "#e0af68".to_string(),
            error: "#f7768e".to_string(),
            cursor: "#c0caf5".to_string(),
        }
    }
}

impl AppController {
    pub fn new() -> Self {
        let paths = AppDataPaths::new();
        Self::with_paths(paths)
    }

    fn with_paths(paths: AppDataPaths) -> Self {
        let mut config = AppConfig::load_or_default(&paths);
        migrate_old_default_theme(&paths, &mut config);
        let session = SessionState::load_or_default(&paths);
        let recent_files = RecentFiles::load_or_default(&paths);
        let themes = ThemeStore::load(&paths, config.active_theme.as_deref());

        let mut initial_doc = NoteDocument::default();
        initial_doc.defer_enabled = true;

        Self {
            paths,
            config,
            session,
            recent_files,
            themes,
            documents: vec![initial_doc],
            active_document: 0,
            mouse_selection: None,
            theme_panel_open: false,
            settings_panel_open: false,
            last_message: String::new(),
        }
    }

    pub fn new_file(&mut self) {
        let mut note = NoteDocument::default();
        note.defer_enabled = true;
        note.new_blank();
        if self.documents.len() == 1 && !self.active_note().is_open() {
            self.documents[0] = note;
            self.active_document = 0;
        } else {
            self.documents.push(note);
            self.active_document = self.documents.len().saturating_sub(1);
        }
        self.last_message = "New note ready.".to_string();
        self.mouse_selection = None;
    }

    pub fn open_file_dialog(&mut self) {
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter("Text", &["txt"])
            .add_filter("Markdown", &["md", "markdown"])
            .set_file_name("note.txt")
            .pick_files()
        {
            for path in paths {
                self.open_path(path);
            }
        }
    }

    pub fn save(&mut self) {
        if !self.active_note().is_open() {
            self.new_file();
        }

        if self.active_note().path_string().is_some() {
            match self.active_note_mut().save() {
                Ok(()) => {
                    if let Some(path) = self.active_note().path_string() {
                        self.remember_recent(PathBuf::from(path));
                    }
                    self.last_message = "Saved.".to_string();
                }
                Err(error) => self.last_message = format!("Could not save note: {error}"),
            }
        } else {
            self.save_as();
        }
    }

    pub fn save_as(&mut self) {
        if !self.active_note().is_open() {
            self.active_note_mut().new_blank();
        }

        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Text", &["txt"])
            .add_filter("Markdown", &["md"])
            .set_file_name("note.txt")
            .save_file()
        {
            match self.active_note_mut().save_as(&path) {
                Ok(()) => {
                    self.remember_recent(path);
                    self.last_message = "Saved.".to_string();
                }
                Err(error) => self.last_message = format!("Could not save note: {error}"),
            }
        }
    }

    pub fn open_theme_panel(&mut self) {
        self.theme_panel_open = true;
        self.settings_panel_open = false;
        self.themes.clear_preview();
        self.last_message = "Choose a theme.".to_string();
    }

    pub fn preview_theme(&mut self, index: i32) {
        let Ok(index) = usize::try_from(index) else {
            return;
        };

        self.themes.preview(index);
        self.last_message = self
            .themes
            .active_theme()
            .map(|theme| format!("Previewing {}.", theme.name))
            .unwrap_or_default();
    }

    pub fn apply_theme(&mut self, index: i32) {
        let Ok(index) = usize::try_from(index) else {
            return;
        };

        let Some(slug) = self.themes.commit(index).map(|theme| theme.slug()) else {
            return;
        };

        self.config.active_theme = Some(slug);
        match self.config.save(&self.paths) {
            Ok(()) => self.last_message = "Theme applied.".to_string(),
            Err(error) => {
                self.last_message = format!("Theme applied but config was not saved: {error}")
            }
        }
        self.theme_panel_open = false;
    }

    pub fn close_theme_panel(&mut self) {
        self.themes.clear_preview();
        self.theme_panel_open = false;
        self.last_message = "Theme selection canceled.".to_string();
    }

    pub fn open_settings_panel(&mut self) {
        self.themes.clear_preview();
        self.theme_panel_open = false;
        self.settings_panel_open = true;
        self.last_message = "Adjust settings.".to_string();
    }

    pub fn close_settings_panel(&mut self) {
        self.settings_panel_open = false;
        self.last_message = "Settings closed.".to_string();
    }

    pub fn adjust_font_size(&mut self, delta: i32) {
        self.config.font_size = (self.config.font_size + delta as f32).clamp(8.0, 32.0);
        self.save_config_message("Font size updated.");
    }

    pub fn adjust_line_height(&mut self, delta_tenths: i32) {
        self.config.line_height =
            (self.config.line_height + (delta_tenths as f32 / 10.0)).clamp(1.0, 2.4);
        self.save_config_message("Line height updated.");
    }

    pub fn adjust_tab_size(&mut self, delta: i32) {
        self.config.tab_size = ((i32::from(self.config.tab_size) + delta).clamp(2, 8)) as u8;
        self.save_config_message("Tab size updated.");
    }

    pub fn adjust_window_opacity(&mut self, delta: i32) {
        self.config.window_opacity =
            ((i32::from(self.config.window_opacity) + delta).clamp(40, 100)) as u8;
        self.save_config_message("Window opacity updated.");
    }

    pub fn toggle_word_wrap(&mut self) {
        self.config.word_wrap = !self.config.word_wrap;
        self.save_config_message("Word wrap updated.");
    }

    pub fn toggle_sync_clipboard(&mut self) {
        self.config.sync_clipboard = !self.config.sync_clipboard;
        self.save_config_message("Clipboard sync updated.");
    }

    pub fn toggle_restore_last_session(&mut self) {
        self.config.restore_last_session = !self.config.restore_last_session;
        self.save_config_message("Session restore updated.");
    }

    pub fn toggle_show_launcher_on_startup(&mut self) {
        self.config.show_launcher_on_startup = !self.config.show_launcher_on_startup;
        self.save_config_message("Startup launcher updated.");
    }

    pub fn toggle_remember_window_geometry(&mut self) {
        self.config.remember_window_geometry = !self.config.remember_window_geometry;
        self.save_config_message("Window geometry setting updated.");
    }

    pub fn toggle_blur_behind(&mut self) {
        self.config.blur_behind = !self.config.blur_behind;
        self.save_config_message("Window blur setting updated.");
    }

    pub fn flush_deferred_action(&mut self) -> bool {
        if !self.active_note().is_open() {
            return false;
        }

        let before_register = self
            .config
            .sync_clipboard
            .then(|| self.active_note().unnamed_register_snapshot());

        let result = self.active_note_mut().flush_deferred_action();
        if result {
            self.export_unnamed_register_if_changed(before_register);
        }
        result
    }

    pub fn handle_editor_key(&mut self, key: &str) {
        if !self.active_note().is_open() {
            return;
        }

        self.mouse_selection = None;
        self.flush_deferred_action();
        self.active_note_mut().clear_yank_highlight();
        match self.active_note().mode() {
            VimMode::Insert | VimMode::Replace => self.handle_insert_editor_key(key),
            _ => self.handle_normal_editor_key(key),
        }
    }

    pub fn handle_editor_pointer(&mut self, line: i32, x_pixels: f32, event_kind: &str) {
        if !self.active_note().is_open() || line < 0 {
            return;
        }

        self.flush_deferred_action();
        self.active_note_mut().clear_yank_highlight();
        let line = line as usize;
        let column = pointer_column_from_x(x_pixels, self.config.font_size);
        self.active_note_mut().set_cursor_from_pointer(line, column);

        match event_kind {
            "down" => {
                let cursor_line = self.active_note().cursor_line();
                self.mouse_selection = Some(MouseSelection {
                    anchor_line: cursor_line,
                    focus_line: cursor_line,
                });
            }
            "move" => {
                let focus_line = self.active_note().cursor_line();
                if let Some(selection) = &mut self.mouse_selection {
                    selection.focus_line = focus_line;
                }
            }
            "up" => {
                if let Some(selection) = self.mouse_selection {
                    if selection.anchor_line == selection.focus_line {
                        self.mouse_selection = None;
                    }
                }
            }
            _ => {}
        }

        self.last_message.clear();
    }

    pub fn previous_document(&mut self) {
        if self.documents.is_empty() {
            return;
        }

        self.active_document = if self.active_document == 0 {
            self.documents.len().saturating_sub(1)
        } else {
            self.active_document - 1
        };
        self.last_message = format!("Switched to {}.", self.active_note().title());
        self.mouse_selection = None;
    }

    pub fn next_document(&mut self) {
        if self.documents.is_empty() {
            return;
        }

        self.active_document = (self.active_document + 1) % self.documents.len();
        self.last_message = format!("Switched to {}.", self.active_note().title());
        self.mouse_selection = None;
    }

    pub fn snapshot(&self) -> AppSnapshot {
        let note = self.active_note();
        let stats = note.stats();
        let cursor = cursor_snapshot(note);
        let mode_text = if note.is_open() {
            note.mode().label().to_string()
        } else {
            "READY".to_string()
        };
        let theme = self.theme_snapshot();
        AppSnapshot {
            file_title: note.title(),
            file_path: note.path_string().unwrap_or_default(),
            editor_lines: editor_lines(note, self.mouse_selection),
            document_tabs: self.document_tabs(),
            status_text: if note.is_open() {
                match note.vim_state.mode {
                    VimMode::Command => {
                        format!(":{}", note.vim_state.command_line.input)
                    }
                    VimMode::Search(_) => {
                        note.vim_state.command_line.input.clone()
                    }
                    _ => {
                        if let Some(PendingCommand::SubstituteConfirm { replacement, .. }) = &note.vim_state.pending_command {
                            format!("replace with {} (y/n/a/q/l)?", replacement)
                        } else {
                            format!(
                                " {}{}",
                                note.title(),
                                if note.dirty() { " [+]" } else { "" },
                            )
                        }
                    }
                }
            } else {
                " [No Name]".to_string()
            },
            status_right: if note.is_open() {
                format!(
                    "Doc {}/{}  |  Ln {}, Col {}{}  |  {} lines, {} words, {} chars  ",
                    self.active_document + 1,
                    self.open_document_count(),
                    note.cursor_line() + 1,
                    note.display_cursor_col() + 1,
                    note.search_pattern()
                        .map(|pattern| format!("  |  /{pattern}"))
                        .unwrap_or_default(),
                    stats.line_count,
                    stats.word_count,
                    stats.char_count
                )
            } else {
                "NeoNote native Vim core  ".to_string()
            },
            mode_color: self.mode_color(&mode_text),
            cursor_line: note.cursor_line() as i32,
            cursor_column: note.display_cursor_col() as i32,
            cursor_prefix: cursor.prefix,
            cursor_cell: cursor.cell,
            cursor_suffix: cursor.suffix,
            cursor_block: note.mode() != VimMode::Insert,
            mode_text,
            message: self.last_message.clone(),
            has_document: note.is_open(),
            editor_font_family: crate::platform::fonts::select_editor_font(
                &self.config.font_family,
            ),
            editor_font_size: self.config.font_size,
            editor_line_height: self.config.font_size * self.config.line_height,
            theme_panel_open: self.theme_panel_open,
            theme_items: self.theme_items(),
            settings_panel_open: self.settings_panel_open,
            settings: SettingsSnapshot::from_config(&self.config),
            theme,
        }
    }

    fn handle_normal_editor_key(&mut self, key: &str) {
        let normal_key = match normalize_normal_key(key) {
            NormalKey::EnterNormal => {
                self.active_note_mut().enter_normal();
                return;
            }
            NormalKey::Input(normal_key) => normal_key,
            NormalKey::Ignore => return,
        };

        if self.config.sync_clipboard && is_clipboard_paste_key(normal_key) {
            self.import_clipboard_to_unnamed_register();
        }

        let before_register = self
            .config
            .sync_clipboard
            .then(|| self.active_note().unnamed_register_snapshot());

        if self.active_note_mut().handle_normal_input(normal_key) {
            self.last_message.clear();
        }

        self.export_unnamed_register_if_changed(before_register);

        if let Some(action) = self.active_note_mut().take_ex_action() {
            self.handle_ex_action(action);
        }
    }

    fn close_active_document(&mut self, force: bool) -> bool {
        if self.active_note().dirty() && !force {
            self.last_message = "No write since last change (add ! to override)".to_string();
            return false;
        }

        if self.documents.len() <= 1 {
            slint::quit_event_loop().ok();
            true
        } else {
            self.documents.remove(self.active_document);
            self.active_document = self.active_document.min(self.documents.len() - 1);
            self.last_message = "Closed note.".to_string();
            self.mouse_selection = None;
            true
        }
    }

    fn handle_ex_action(&mut self, action: ExCommandAction) {
        match action {
            ExCommandAction::Save(Some(path_str)) => {
                let path = PathBuf::from(path_str);
                match self.active_note_mut().save_as(&path) {
                    Ok(()) => {
                        self.remember_recent(path);
                        self.last_message = "Saved.".to_string();
                    }
                    Err(error) => self.last_message = format!("Could not save note: {error}"),
                }
            }
            ExCommandAction::Save(None) => {
                self.save();
            }
            ExCommandAction::SaveAll => {
                for i in 0..self.documents.len() {
                    let prev_active = self.active_document;
                    self.active_document = i;
                    self.save();
                    self.active_document = prev_active;
                }
            }
            ExCommandAction::Quit { force } => {
                self.close_active_document(force);
            }
            ExCommandAction::SaveAndQuit => {
                self.save();
                if !self.active_note().dirty() {
                    self.close_active_document(false);
                }
            }
            ExCommandAction::Edit(path_str) => {
                let path = PathBuf::from(path_str);
                self.open_path(path);
            }
            ExCommandAction::EditNew => {
                self.new_file();
            }
            ExCommandAction::ShowMessage(message) => {
                self.last_message = message;
            }
        }
    }

    fn handle_insert_editor_key(&mut self, key: &str) {
        if self.active_note_mut().resolve_insert_register_paste(key) {
            return;
        }
        match normalize_insert_key(key) {
            InsertKey::EnterNormal => self.active_note_mut().enter_normal(),
            InsertKey::Cancel => self.active_note_mut().cancel_insert(),
            InsertKey::RegisterPaste => self.active_note_mut().begin_insert_register_paste(),
            InsertKey::DeleteWord => self.active_note_mut().delete_word_insert(),
            InsertKey::DeleteLine => self.active_note_mut().delete_line_insert(),
            InsertKey::Newline => self.active_note_mut().insert_newline(),
            InsertKey::Backspace => self.active_note_mut().backspace(),
            InsertKey::Delete => self.active_note_mut().delete_at_cursor(),
            InsertKey::Text(text) => self.active_note_mut().handle_insert_text(text),
            InsertKey::Ignore => {}
        }
    }

    fn open_path(&mut self, path: PathBuf) {
        if let Some(index) = self.document_index_for_path(&path) {
            self.active_document = index;
            self.mouse_selection = None;
            self.last_message = "File already open.".to_string();
            return;
        }

        let mut note = NoteDocument::default();
        note.defer_enabled = true;
        match note.open(&path) {
            Ok(()) => {
                if self.documents.len() == 1 && !self.active_note().is_open() {
                    self.documents[0] = note;
                    self.active_document = 0;
                } else {
                    self.documents.push(note);
                    self.active_document = self.documents.len().saturating_sub(1);
                }
                self.mouse_selection = None;
                self.remember_recent(path);
                self.last_message = "Opened note.".to_string();
            }
            Err(error) => self.last_message = format!("Could not open note: {error}"),
        }
    }

    fn remember_recent(&mut self, path: PathBuf) {
        self.recent_files
            .upsert(path.display().to_string(), chrono_like_now());
        if let Err(error) = self.recent_files.save(&self.paths) {
            self.last_message = format!("Could not update recent files: {error}");
        }
    }

    fn theme_snapshot(&self) -> ThemeSnapshot {
        let Some(theme) = self.themes.active_theme() else {
            return ThemeSnapshot::default();
        };

        ThemeSnapshot {
            background: theme.colors.background.clone(),
            background_alt: theme.colors.background_alt.clone(),
            surface: theme.colors.surface.clone(),
            border: theme.colors.border.clone(),
            text: theme.colors.text.clone(),
            text_muted: theme.colors.text_muted.clone(),
            accent_primary: theme.colors.accent_primary.clone(),
            accent_secondary: theme.colors.accent_secondary.clone(),
            success: theme.colors.success.clone(),
            warning: theme.colors.warning.clone(),
            error: theme.colors.error.clone(),
            cursor: theme.colors.cursor.clone(),
        }
    }

    fn mode_color(&self, mode: &str) -> String {
        let Some(theme) = self.themes.active_theme() else {
            return ThemeSnapshot::default().accent_primary;
        };

        match mode {
            "NORMAL" => theme.vim_modes.normal.clone(),
            "INSERT" => theme.vim_modes.insert.clone(),
            "VISUAL" | "V-LINE" | "V-BLOCK" => theme.vim_modes.visual.clone(),
            "COMMAND" | "/" | "?" => theme.vim_modes.command.clone(),
            "REPLACE" => theme.vim_modes.replace.clone(),
            _ => theme.colors.accent_primary.clone(),
        }
    }

    fn theme_items(&self) -> Vec<ThemeItemSnapshot> {
        self.themes
            .all()
            .iter()
            .enumerate()
            .map(|(index, theme)| ThemeItemSnapshot {
                index: index as i32,
                name: theme.name.clone(),
                variant: format!("{:?}", theme.variant).to_lowercase(),
                author: theme.author.clone(),
                is_active: self.themes.is_active(index),
                is_preview: self.themes.is_preview(index),
            })
            .collect()
    }

    fn save_config_message(&mut self, success_message: &str) {
        match self.config.save(&self.paths) {
            Ok(()) => self.last_message = success_message.to_string(),
            Err(error) => self.last_message = format!("Could not save settings: {error}"),
        }
    }

    fn import_clipboard_to_unnamed_register(&mut self) {
        match clipboard::read_text() {
            Ok(text) => {
                let text = text.replace("\r\n", "\n");
                if self.active_note().clipboard_register_text() == text {
                    return;
                }
                let linewise = text.ends_with('\n');
                self.active_note_mut()
                    .set_clipboard_register(text, linewise);
            }
            Err(error) => self.last_message = format!("Could not read clipboard: {error}"),
        }
    }

    fn export_unnamed_register_if_changed(&mut self, before: Option<RegisterSnapshot>) {
        let Some(before) = before else {
            return;
        };

        let after = self.active_note().unnamed_register_snapshot();
        if after == before || after.text.is_empty() {
            return;
        }

        if let Err(error) = clipboard::write_text(&after.text) {
            self.last_message = format!("Could not update clipboard: {error}");
        }
    }

    pub(crate) fn active_note(&self) -> &NoteDocument {
        &self.documents[self
            .active_document
            .min(self.documents.len().saturating_sub(1))]
    }

    pub(crate) fn active_note_mut(&mut self) -> &mut NoteDocument {
        let index = self
            .active_document
            .min(self.documents.len().saturating_sub(1));
        &mut self.documents[index]
    }

    fn document_index_for_path(&self, path: &std::path::Path) -> Option<usize> {
        self.documents.iter().position(|note| {
            note.path_string()
                .map(|open_path| PathBuf::from(open_path) == path)
                .unwrap_or(false)
        })
    }

    fn document_tabs(&self) -> String {
        self.documents
            .iter()
            .enumerate()
            .filter(|(_, note)| note.is_open())
            .map(|(index, note)| {
                let active_marker = if index == self.active_document {
                    ">"
                } else {
                    " "
                };
                let dirty_marker = if note.dirty() { " +" } else { "" };
                format!("{active_marker} {}{dirty_marker}", note.title())
            })
            .collect::<Vec<_>>()
            .join("   ")
    }

    fn open_document_count(&self) -> usize {
        self.documents
            .iter()
            .filter(|note| note.is_open())
            .count()
            .max(1)
    }
}

impl Default for AppController {
    fn default() -> Self {
        Self::new()
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

fn is_clipboard_paste_key(key: &str) -> bool {
    matches!(key, "p" | "P")
}

fn pointer_column_from_x(x_pixels: f32, font_size: f32) -> usize {
    let char_width = (font_size * 0.62).max(1.0);
    (x_pixels.max(0.0) / char_width).round() as usize
}

fn migrate_old_default_theme(paths: &AppDataPaths, config: &mut AppConfig) {
    if config.active_theme.as_deref() == Some("catppuccin-mocha") {
        config.active_theme = Some("neovim-dark".to_string());
        let _ = config.save(paths);
    }
}

struct CursorSnapshot {
    prefix: String,
    cell: String,
    suffix: String,
}

fn cursor_snapshot(note: &NoteDocument) -> CursorSnapshot {
    let line = note
        .content()
        .split('\n')
        .nth(note.cursor_line())
        .unwrap_or_default();
    let col = note.display_cursor_col();
    let prefix = line.chars().take(col).collect::<String>();
    let suffix_start = if note.mode() == VimMode::Normal {
        col.saturating_add(1)
    } else {
        col
    };
    let cell = if note.mode() == VimMode::Normal {
        line.chars().nth(col).unwrap_or(' ').to_string()
    } else {
        " ".to_string()
    };
    let suffix = line.chars().skip(suffix_start).collect::<String>();

    CursorSnapshot {
        prefix,
        cell,
        suffix,
    }
}

fn editor_lines(
    note: &NoteDocument,
    mouse_selection: Option<MouseSelection>,
) -> Vec<EditorLineSnapshot> {
    let cursor_line = note.cursor_line();
    let cursor_column = note.display_cursor_col() as i32;
    let cursor_block = note.mode() != VimMode::Insert;
    let cursor = cursor_snapshot(note);

    content_lines(note)
        .into_iter()
        .enumerate()
        .map(|(index, text)| {
            let (selected_prefix, selected_text) = {
                let mut cols = note.line_selection_cols(index);
                if let Some(selection) = mouse_selection {
                    if selection.includes(index) {
                        cols = Some((0, usize::MAX));
                    }
                }
                if let Some((start_col, end_col)) = cols {
                    let chars: Vec<char> = text.chars().collect();
                    let start = start_col.min(chars.len());
                    let end = end_col.min(chars.len()).max(start);
                    (chars[..start].iter().collect(), chars[start..end].iter().collect())
                } else {
                    (String::new(), String::new())
                }
            };

            EditorLineSnapshot {
            number: (index + 1) as i32,
            is_cursor_line: index == cursor_line,
            selected_prefix,
            selected_text,
            is_search_match: note.line_has_search_match(index),
            cursor_column,
            cursor_prefix: if index == cursor_line {
                cursor.prefix.clone()
            } else {
                String::new()
            },
            cursor_cell: if index == cursor_line {
                cursor.cell.clone()
            } else {
                String::new()
            },
            cursor_suffix: if index == cursor_line {
                cursor.suffix.clone()
            } else {
                String::new()
            },
            text,
            cursor_block,
        }
        })
        .collect()
}

fn content_lines(note: &NoteDocument) -> Vec<String> {
    if note.content().is_empty() {
        vec![String::new()]
    } else {
        note.content().split('\n').map(ToOwned::to_owned).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_file_keeps_multiple_untitled_documents() {
        let mut controller = AppController::new();
        controller.new_file();
        controller.handle_editor_key("i");
        controller.handle_editor_key("a");

        controller.new_file();
        controller.handle_editor_key("i");
        controller.handle_editor_key("b");

        assert_eq!(controller.active_note().content(), "b");
        controller.previous_document();
        assert_eq!(controller.active_note().content(), "a");
    }

    #[test]
    fn open_path_adds_multiple_file_documents() {
        let root = std::env::temp_dir().join(format!("neonote-test-{}", chrono_like_now()));
        std::fs::create_dir_all(&root).unwrap();
        let first = root.join("first.txt");
        let second = root.join("second.txt");
        std::fs::write(&first, "one").unwrap();
        std::fs::write(&second, "two").unwrap();

        let mut controller = AppController::new();
        controller.open_path(first);
        controller.open_path(second);

        assert_eq!(controller.open_document_count(), 2);
        assert_eq!(controller.active_note().content(), "two");
        controller.previous_document();
        assert_eq!(controller.active_note().content(), "one");
    }

    #[test]
    fn cursor_snapshot_uses_prefix_and_cell_for_normal_mode() {
        let mut controller = AppController::new();
        controller.new_file();
        controller.handle_editor_key("i");
        for key in ["a", "b", "c", "escape", "h"] {
            controller.handle_editor_key(key);
        }

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.cursor_prefix, "a");
        assert_eq!(snapshot.cursor_cell, "b");
        assert_eq!(snapshot.cursor_suffix, "c");
        assert!(snapshot.cursor_block);
    }

    #[test]
    fn editor_lines_preserve_content_and_cursor_row() {
        let mut controller = AppController::new();
        controller.new_file();
        controller.handle_editor_key("i");
        for key in ["o", "n", "e", "return", "t", "w", "o", "escape"] {
            controller.handle_editor_key(key);
        }

        let snapshot = controller.snapshot();
        assert_eq!(
            snapshot.editor_lines,
            vec![
                EditorLineSnapshot {
                    number: 1,
                    text: "one".to_string(),
                    is_cursor_line: false,
                    selected_prefix: String::new(),
                    selected_text: String::new(),
                    is_search_match: false,
                    cursor_column: 2,
                    cursor_prefix: String::new(),
                    cursor_cell: String::new(),
                    cursor_suffix: String::new(),
                    cursor_block: true,
                },
                EditorLineSnapshot {
                    number: 2,
                    text: "two".to_string(),
                    is_cursor_line: true,
                    selected_prefix: String::new(),
                    selected_text: String::new(),
                    is_search_match: false,
                    cursor_column: 2,
                    cursor_prefix: "tw".to_string(),
                    cursor_cell: "o".to_string(),
                    cursor_suffix: String::new(),
                    cursor_block: true,
                },
            ]
        );
        assert_eq!(snapshot.cursor_prefix, "tw");
        assert_eq!(snapshot.cursor_cell, "o");
        assert_eq!(snapshot.cursor_suffix, "");
    }

    #[test]
    fn editor_lines_handle_column_zero_middle_end_and_empty_lines() {
        let mut controller = AppController::new();
        controller.new_file();
        controller.handle_editor_key("i");
        for key in ["a", "b", "c", "return", "return", "d", "e", "f", "escape"] {
            controller.handle_editor_key(key);
        }

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.editor_lines.len(), 3);
        assert_eq!(snapshot.editor_lines[2].cursor_column, 2);
        assert!(snapshot.editor_lines[2].is_cursor_line);

        controller.handle_editor_key("0");
        let snapshot = controller.snapshot();
        assert_eq!(snapshot.editor_lines[2].cursor_column, 0);

        controller.handle_editor_key("k");
        let snapshot = controller.snapshot();
        assert_eq!(snapshot.editor_lines[1].text, "");
        assert_eq!(snapshot.editor_lines[1].cursor_column, 0);
        assert!(snapshot.editor_lines[1].is_cursor_line);

        controller.handle_editor_key("k");
        controller.handle_editor_key("$");
        let snapshot = controller.snapshot();
        assert_eq!(snapshot.editor_lines[0].cursor_column, 2);
        assert!(snapshot.editor_lines[0].is_cursor_line);
    }

    #[test]
    fn editor_lines_allow_insert_cursor_after_line_end() {
        let mut controller = AppController::new();
        controller.new_file();
        controller.handle_editor_key("i");
        for key in ["a", "b", "c"] {
            controller.handle_editor_key(key);
        }

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.editor_lines[0].text, "abc");
        assert_eq!(snapshot.editor_lines[0].cursor_column, 3);
        assert_eq!(snapshot.editor_lines[0].cursor_prefix, "abc");
        assert_eq!(snapshot.editor_lines[0].cursor_cell, " ");
        assert_eq!(snapshot.editor_lines[0].cursor_suffix, "");
        assert!(!snapshot.editor_lines[0].cursor_block);
    }

    #[test]
    fn editor_line_cursor_prefix_uses_actual_text_before_cursor() {
        let mut controller = AppController::new();
        controller.new_file();
        controller.handle_editor_key("i");
        for key in ["d", "a", "w", "d", "w", "a", "d", "escape"] {
            controller.handle_editor_key(key);
        }

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.editor_lines[0].cursor_column, 6);
        assert_eq!(snapshot.editor_lines[0].cursor_prefix, "dawdwa");
        assert_eq!(snapshot.editor_lines[0].cursor_cell, "d");
        assert_eq!(snapshot.editor_lines[0].cursor_suffix, "");

        controller.handle_editor_key("0");
        let snapshot = controller.snapshot();
        assert_eq!(snapshot.editor_lines[0].cursor_column, 0);
        assert_eq!(snapshot.editor_lines[0].cursor_prefix, "");
        assert_eq!(snapshot.editor_lines[0].cursor_cell, "d");
        assert_eq!(snapshot.editor_lines[0].cursor_suffix, "awdwad");
    }

    #[test]
    fn editor_pointer_places_cursor_and_highlights_dragged_lines() {
        let mut controller = AppController::new();
        controller.new_file();
        controller.handle_editor_key("i");
        for key in [
            "a", "b", "c", "return", "d", "e", "f", "return", "g", "h", "i", "escape",
        ] {
            controller.handle_editor_key(key);
        }

        controller.handle_editor_pointer(0, 18.0, "down");
        controller.handle_editor_pointer(2, 0.0, "move");

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.cursor_line, 2);
        assert!(!snapshot.editor_lines[0].selected_text.is_empty());
        assert!(!snapshot.editor_lines[1].selected_text.is_empty());
        assert!(!snapshot.editor_lines[2].selected_text.is_empty());

        controller.handle_editor_pointer(2, 0.0, "up");
        assert!(!controller.snapshot().editor_lines[1].selected_text.is_empty());

        controller.handle_editor_key("j");
        assert!(controller
            .snapshot()
            .editor_lines
            .iter()
            .all(|line| line.selected_text.is_empty()));
    }

    #[test]
    fn editor_lines_render_visual_selection_and_search_matches() {
        let mut controller = AppController::new();
        controller.new_file();
        *controller.active_note_mut().content_mut() = "one\ntwo\nthree".to_string();
        controller.active_note_mut().enter_normal();

        controller.handle_editor_key("g");
        controller.handle_editor_key("g");
        controller.handle_editor_key("V");
        controller.handle_editor_key("j");

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.mode_text, "V-LINE");
        assert!(!snapshot.editor_lines[0].selected_text.is_empty());
        assert!(!snapshot.editor_lines[1].selected_text.is_empty());

        controller.handle_editor_key("escape");
        for key in ["/", "t", "w", "o", "return"] {
            controller.handle_editor_key(key);
        }

        let snapshot = controller.snapshot();
        assert!(snapshot.editor_lines[1].is_search_match);
        assert!(snapshot.status_right.contains("/two"));
    }

    #[test]
    fn theme_items_include_current_active_theme() {
        let (_root, controller) = test_controller();

        let snapshot = controller.snapshot();
        assert!(!snapshot.theme_items.is_empty());
        assert!(snapshot.theme_items.iter().any(|item| item.is_active));
    }

    #[test]
    fn theme_preview_changes_snapshot_without_persisting_config() {
        let (root, mut controller) = test_controller();
        let preview_index = first_inactive_theme_index(&controller);
        let committed_background = controller.snapshot().theme.background;

        controller.open_theme_panel();
        controller.preview_theme(preview_index as i32);

        let snapshot = controller.snapshot();
        assert_ne!(snapshot.theme.background, committed_background);
        assert!(snapshot.theme_items[preview_index].is_preview);
        assert!(!root.join("config.json").exists());
    }

    #[test]
    fn theme_panel_close_clears_preview_and_restores_committed_theme() {
        let (_root, mut controller) = test_controller();
        let preview_index = first_inactive_theme_index(&controller);
        let committed_background = controller.snapshot().theme.background;

        controller.open_theme_panel();
        controller.preview_theme(preview_index as i32);
        assert_ne!(controller.snapshot().theme.background, committed_background);

        controller.close_theme_panel();

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.theme.background, committed_background);
        assert!(!snapshot.theme_panel_open);
        assert!(snapshot.theme_items.iter().all(|item| !item.is_preview));
    }

    #[test]
    fn theme_apply_commits_active_theme_and_updates_config_value() {
        let (_root, mut controller) = test_controller();
        let apply_index = first_inactive_theme_index(&controller);
        let expected_slug = controller.themes.all()[apply_index].slug();
        let paths = controller.paths.clone();

        controller.open_theme_panel();
        controller.preview_theme(apply_index as i32);
        controller.apply_theme(apply_index as i32);

        let snapshot = controller.snapshot();
        let saved_config = AppConfig::load_or_default(&paths);
        assert!(!snapshot.theme_panel_open);
        assert_eq!(
            controller.themes.active_slug().as_deref(),
            Some(expected_slug.as_str())
        );
        assert_eq!(
            saved_config.active_theme.as_deref(),
            Some(expected_slug.as_str())
        );
        assert!(snapshot.theme_items[apply_index].is_active);
        assert!(snapshot.theme_items.iter().all(|item| !item.is_preview));
    }

    #[test]
    fn settings_panel_snapshot_reflects_config_values() {
        let (_root, mut controller) = test_controller();

        controller.open_settings_panel();

        let snapshot = controller.snapshot();
        assert!(snapshot.settings_panel_open);
        assert_eq!(snapshot.settings.font_size, AppConfig::default().font_size);
        assert_eq!(
            snapshot.settings.restore_last_session,
            AppConfig::default().restore_last_session
        );
        assert_eq!(
            snapshot.settings.show_launcher_on_startup,
            AppConfig::default().show_launcher_on_startup
        );
        assert_eq!(
            snapshot.settings.sync_clipboard,
            AppConfig::default().sync_clipboard
        );
    }

    #[test]
    fn settings_updates_persist_config_values() {
        let (_root, mut controller) = test_controller();
        let paths = controller.paths.clone();

        controller.open_settings_panel();
        controller.adjust_font_size(3);
        controller.adjust_line_height(2);
        controller.adjust_tab_size(2);
        controller.toggle_restore_last_session();
        controller.toggle_show_launcher_on_startup();
        controller.toggle_word_wrap();
        controller.toggle_sync_clipboard();

        let saved_config = AppConfig::load_or_default(&paths);
        assert_eq!(saved_config.font_size, 17.0);
        assert_eq!(saved_config.line_height, 1.6);
        assert_eq!(saved_config.tab_size, 4);
        assert!(!saved_config.restore_last_session);
        assert!(!saved_config.show_launcher_on_startup);
        assert!(saved_config.word_wrap);
        assert!(!saved_config.sync_clipboard);
    }

    #[test]
    fn settings_numeric_updates_clamp_to_supported_ranges() {
        let (_root, mut controller) = test_controller();

        controller.adjust_font_size(-100);
        controller.adjust_line_height(-100);
        controller.adjust_tab_size(-100);
        controller.adjust_window_opacity(-100);
        assert_eq!(controller.snapshot().settings.font_size, 8.0);
        assert_eq!(controller.snapshot().settings.line_height, 1.0);
        assert_eq!(controller.snapshot().settings.tab_size, 2);
        assert_eq!(controller.snapshot().settings.window_opacity, 40);

        controller.adjust_font_size(100);
        controller.adjust_line_height(100);
        controller.adjust_tab_size(100);
        controller.adjust_window_opacity(100);
        assert_eq!(controller.snapshot().settings.font_size, 32.0);
        assert_eq!(controller.snapshot().settings.line_height, 2.4);
        assert_eq!(controller.snapshot().settings.tab_size, 8);
        assert_eq!(controller.snapshot().settings.window_opacity, 100);
    }

    fn test_controller() -> (PathBuf, AppController) {
        let root = std::env::temp_dir().join(format!(
            "neonote-controller-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default()
        ));
        let paths = AppDataPaths::from_root(root.clone());
        (root, AppController::with_paths(paths))
    }

    fn first_inactive_theme_index(controller: &AppController) -> usize {
        controller
            .themes
            .all()
            .iter()
            .enumerate()
            .find_map(|(index, _)| (!controller.themes.is_active(index)).then_some(index))
            .expect("built-in themes should include an inactive theme")
    }

    #[test]
    fn test_app_yy() {
        let mut controller = AppController::new();
        controller.new_file();
        controller.handle_editor_key("i");
        for key in ["l", "i", "n", "e", "1", "return", "l", "i", "n", "e", "2", "escape"] {
            controller.handle_editor_key(key);
        }
        // Move to the first line
        controller.handle_editor_key("k");
        assert_eq!(controller.active_note().cursor_line(), 0);

        // Press yy
        controller.handle_editor_key("y");
        controller.handle_editor_key("y");

        assert_eq!(controller.active_note().unnamed_register_text(), "line1\n");
        if controller.config.sync_clipboard {
            assert_eq!(clipboard::read_text().unwrap().replace("\r\n", "\n"), "line1\n");
        }
    }
}
