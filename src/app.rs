use std::path::PathBuf;
use std::collections::HashMap;

use crate::{
    notes::{Pane, TextBuffer, PaneId, BufferId, RegisterSnapshot, VimMode, PendingCommand, ExCommandAction},
    persistence::{AppConfig, AppDataPaths, RecentFiles, SessionState},
    platform::clipboard,
    theme::ThemeStore,
};

pub struct AppController {
    paths: AppDataPaths,
    config: AppConfig,
    session: SessionState,
    recent_files: RecentFiles,
    themes: ThemeStore,
    documents: Vec<Pane>,
    buffers: HashMap<BufferId, TextBuffer>,
    active_document: usize,
    theme_panel_open: bool,
    settings_panel_open: bool,
    last_message: String,
    suppress_session_save: bool,
    last_session_save: Option<std::time::Instant>,
    next_pane_id: usize,
    next_buffer_id: usize,
}

#[derive(Clone, Debug, Default)]
pub struct AppSnapshot {
    pub file_title: String,
    pub file_path: String,
    pub editor_lines: Vec<EditorLineSnapshot>,
    pub document_tabs: Vec<DocumentTabSnapshot>,
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
pub struct DocumentTabSnapshot {
    pub index: i32,
    pub title: String,
    pub is_active: bool,
    pub is_modified: bool,
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

        let initial_buffer = TextBuffer::new();
        let buffer_id = BufferId(1);
        let pane_id = PaneId(1);
        let mut initial_pane = Pane::new(pane_id, buffer_id);
        initial_pane.defer_enabled = true;
        
        let mut buffers = HashMap::new();
        buffers.insert(buffer_id, initial_buffer);

        Self {
            paths,
            config,
            session,
            recent_files,
            themes,
            documents: vec![initial_pane],
            buffers,
            active_document: 0,
            theme_panel_open: false,
            settings_panel_open: false,
            last_message: String::new(),
            suppress_session_save: false,
            last_session_save: None,
            next_pane_id: 2,
            next_buffer_id: 2,
        }
    }

    pub fn new_file(&mut self) {
        let buffer_id = BufferId(self.next_buffer_id);
        self.next_buffer_id += 1;
        let pane_id = PaneId(self.next_pane_id);
        self.next_pane_id += 1;

        let mut note = Pane::new(pane_id, buffer_id);
        note.defer_enabled = true;
        
        let buffer = TextBuffer::new();
        self.buffers.insert(buffer_id, buffer);

        let active_buffer = self.active_buffer();
        let is_untitled_empty = self.active_pane().path_string(active_buffer).is_none() && self.active_pane().is_empty(active_buffer);
        if self.documents.len() == 1 && is_untitled_empty {
            self.documents[0] = note;
            self.active_document = 0;
        } else {
            self.documents.push(note);
            self.active_document = self.documents.len().saturating_sub(1);
        }
        self.last_message = "New note ready.".to_string();
        self.flush_deferred_action();
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
        if !self.active_pane().is_open(self.active_buffer()) {
            self.new_file();
        }

        if self.active_pane().path_string(self.active_buffer()).is_some() {
            match { let (p, b) = self.active_pane_and_buffer(); p.save(b) } {
                Ok(()) => {
                    if let Some(path) = self.active_pane().path_string(self.active_buffer()) {
                        self.remember_recent(PathBuf::from(path));
                    }
                    self.last_message = "Saved.".to_string();
                }
                Err(error) => self.last_message = format!("Could not save note: {error}"),
            }
        } else {
            self.save_as();
        }
        self.save_session_state(false);
    }

    pub fn save_as(&mut self) {
        if !self.active_pane().is_open(self.active_buffer()) {
            { let (p, b) = self.active_pane_and_buffer(); p.new_blank(b) };
        }

        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Text", &["txt"])
            .add_filter("Markdown", &["md"])
            .set_file_name("note.txt")
            .save_file()
        {
            match { let (p, b) = self.active_pane_and_buffer(); p.save_as(b, &path) } {
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
            Ok(()) => {
                self.last_message = "Theme applied.".to_string();
            },
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
        if !self.active_pane().is_open(self.active_buffer()) {
            return false;
        }

        let before_register = self
            .config
            .sync_clipboard
            .then(|| self.active_pane().unnamed_register_snapshot(self.active_buffer()));

        let result = { let (p, b) = self.active_pane_and_buffer(); p.flush_deferred_action(b) };
        if result {
            self.export_unnamed_register_if_changed(before_register);
        }
        result
    }

    pub fn handle_editor_key(&mut self, key: &str) {
        match key {
            "ctrl+=" | "ctrl++" | "ctrl+scroll-up" => {
                self.config.font_size = (self.config.font_size + 1.0).min(72.0);
                let _ = self.config.save(&self.paths);
                return;
            }
            "ctrl+-" | "ctrl+scroll-down" => {
                self.config.font_size = (self.config.font_size - 1.0).max(8.0);
                let _ = self.config.save(&self.paths);
                return;
            }
            "ctrl+0" => {
                self.config.font_size = 14.0;
                let _ = self.config.save(&self.paths);
                return;
            }
            _ => {}
        }

        if !self.active_pane().is_open(self.active_buffer()) {
            return;
        }

        self.flush_deferred_action();
        { let (p, b) = self.active_pane_and_buffer(); p.clear_yank_highlight(b) };
        
        let is_insert = self.active_pane().mode(self.active_buffer()) == crate::notes::VimMode::Insert || self.active_pane().mode(self.active_buffer()) == crate::notes::VimMode::Replace;
        
        if is_insert {
            if { let (p, b) = self.active_pane_and_buffer(); p.resolve_insert_register_paste(b, key) } {
                return;
            }
        }
        
        let editor_key = match self.active_pane().mode(self.active_buffer()) {
            crate::notes::VimMode::Insert | crate::notes::VimMode::Replace => {
                crate::vim::key::normalize_insert_key(key)
            }
            crate::notes::VimMode::Command | crate::notes::VimMode::Search(_) => {
                crate::vim::key::normalize_command_key(key)
            }
            _ => {
                crate::vim::key::normalize_normal_key(key)
            }
        };

        if !is_insert {
            if self.config.sync_clipboard {
                if let crate::vim::key::EditorKey::Input(ref s) = editor_key {
                    if crate::app::is_clipboard_paste_key(s) {
                        self.import_clipboard_to_unnamed_register();
                    }
                }
            }
        }

        let before_register = self
            .config
            .sync_clipboard
            .then(|| self.active_pane().unnamed_register_snapshot(self.active_buffer()));

        if { let (p, b) = self.active_pane_and_buffer(); p.handle_editor_key(b, editor_key) } {
            self.last_message.clear();
        }

        self.export_unnamed_register_if_changed(before_register);

        if let Some(action) = { let (p, b) = self.active_pane_and_buffer(); p.take_ex_action(b) } {
            self.handle_ex_action(action);
        }
        self.save_session_state(false);
    }

    pub fn handle_editor_pointer(&mut self, line: i32, x_pixels: f32, event_kind: &str) {
        if !self.active_pane().is_open(self.active_buffer()) || line < 0 {
            return;
        }

        self.flush_deferred_action();
        { let (p, b) = self.active_pane_and_buffer(); p.clear_yank_highlight(b) };
        let line = line as usize;
        let column = pointer_column_from_x(x_pixels, self.config.font_size);


        let pane = self.active_pane();
        let is_visual = matches!(pane.mode(self.active_buffer()), VimMode::Visual | VimMode::VisualLine);

        eprintln!("handle_editor_pointer: kind={}, line={}, column={}, is_visual={}", event_kind, line, column, is_visual);

        let mut trigger_viw = false;

        match event_kind {
            "down" => {
                let (p, b) = self.active_pane_and_buffer();
                if matches!(p.mode(b), VimMode::Visual | VimMode::VisualLine) {
                    p.vim_state.mode = VimMode::Normal;
                    p.visual_anchor_flat = None;
                }
                p.set_cursor_from_pointer(b, line, column);
                eprintln!("down: set cursor to {},{}", p.cursor_line, p.cursor_col);
            }
            "move" => {
                let (p, b) = self.active_pane_and_buffer();
                if !matches!(p.mode(b), VimMode::Visual | VimMode::VisualLine) {
                    // Start visual selection from current cursor
                    let current_flat = p.flattened_cursor(b);
                    p.vim_state.mode = VimMode::Visual;
                    p.visual_anchor_flat = Some(current_flat);
                    eprintln!("move: started visual mode at flat {}", current_flat);
                }
                p.cursor_line = line;
                p.cursor_col = column;
                p.clamp_cursor_normal(b);
                eprintln!("move: updated cursor to {},{}", p.cursor_line, p.cursor_col);
            }
            "double_click" => {
                let (p, _b) = self.active_pane_and_buffer();
                p.vim_state.mode = VimMode::Normal;
                p.visual_anchor_flat = None;
                trigger_viw = true;
            }
            "up" => {}
            _ => {}
        }

        if trigger_viw {
            let (p, b) = self.active_pane_and_buffer();
            p.handle_editor_key(b, crate::vim::key::EditorKey::Input("v".to_string()));
            p.handle_editor_key(b, crate::vim::key::EditorKey::Input("i".to_string()));
            p.handle_editor_key(b, crate::vim::key::EditorKey::Input("w".to_string()));
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
        self.last_message = format!("Switched to {}.", self.active_pane().title(self.active_buffer()));
    }

    pub fn next_document(&mut self) {
        if self.documents.is_empty() {
            return;
        }

        self.active_document = (self.active_document + 1) % self.documents.len();
        self.last_message = format!("Switched to {}.", self.active_pane().title(self.active_buffer()));
    }

    pub fn switch_to_document(&mut self, index: usize) {
        if index < self.documents.len() {
            self.active_document = index;
            self.last_message = format!("Switched to {}.", self.active_pane().title(self.active_buffer()));
        }
    }

    pub fn close_document(&mut self, index: usize) {
        if index >= self.documents.len() {
            return;
        }
        
        let is_dirty = {
            let doc = &self.documents[index];
            let buffer = self.buffers.get(&doc.buffer_id).unwrap();
            buffer.dirty
        };

        if is_dirty {
            let name = {
                let doc = &self.documents[index];
                let buffer = self.buffers.get(&doc.buffer_id).unwrap();
                doc.path_string(buffer).map(|p| PathBuf::from(p).file_name().unwrap_or_default().to_string_lossy().to_string()).unwrap_or_else(|| "Untitled".to_string())
            };
            
            let msg = rfd::MessageDialog::new()
                .set_title("Save changes?")
                .set_description(&format!("Do you want to save the changes you made to {}?", name))
                .set_buttons(rfd::MessageButtons::YesNoCancel)
                .show();
                
            match msg {
                rfd::MessageDialogResult::Yes => {
                    let doc = &mut self.documents[index];
                    let buffer = self.buffers.get_mut(&doc.buffer_id).unwrap();

                    if let Some(path_str) = doc.path_string(buffer) {
                        let path = PathBuf::from(path_str);
                        if let Err(e) = doc.save_as(buffer, &path) {
                            self.last_message = format!("Could not save note: {e}");
                            return;
                        }
                    } else {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Text", &["txt"])
                            .add_filter("Markdown", &["md"])
                            .set_file_name("note.txt")
                            .save_file()
                        {
                            if let Err(e) = doc.save_as(buffer, &path) {
                                self.last_message = format!("Could not save note: {e}");
                                return;
                            }
                        } else {
                            return;
                        }
                    }
                }
                rfd::MessageDialogResult::No => {}
                _ => return,
            }
        }
        
        self.documents.remove(index);
        
        if self.documents.is_empty() {
            self.new_file();
        } else if self.active_document >= self.documents.len() {
            self.active_document = self.documents.len().saturating_sub(1);
        }
        self.save_session_state(false);
    }

    pub fn snapshot(&self) -> AppSnapshot {
        let note = self.active_pane();
        let buffer = self.active_buffer();
        let stats = note.stats(buffer);
        let cursor = cursor_snapshot(note, buffer);
        let mode_text = if note.is_open(self.active_buffer()) {
            note.mode(buffer).label().to_string()
        } else {
            "READY".to_string()
        };
        let theme = self.theme_snapshot();
        AppSnapshot {
            file_title: note.title(self.active_buffer()),
            file_path: note.path_string(self.active_buffer()).unwrap_or_default(),
            editor_lines: editor_lines(note, self.active_buffer()),
            document_tabs: self.document_tabs(),
            status_text: if note.is_open(self.active_buffer()) {
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
                                note.title(self.active_buffer()),
                                if self.active_buffer().dirty { " [+]" } else { "" },
                            )
                        }
                    }
                }
            } else {
                " [No Name]".to_string()
            },
            status_right: if note.is_open(self.active_buffer()) {
                format!(
                    "Doc {}/{}  |  Ln {}, Col {}{}  |  {} lines, {} words, {} chars  ",
                    self.active_document + 1,
                    self.open_document_count(),
                    note.cursor_line(buffer) + 1,
                    note.display_cursor_col(buffer) + 1,
                    note.search_pattern(buffer)
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
            cursor_line: note.cursor_line(buffer) as i32,
            cursor_column: note.display_cursor_col(buffer) as i32,
            cursor_prefix: cursor.prefix,
            cursor_cell: cursor.cell,
            cursor_suffix: cursor.suffix,
            cursor_block: note.mode(buffer) != VimMode::Insert,
            mode_text,
            message: self.last_message.clone(),
            has_document: note.is_open(self.active_buffer()),
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


    fn close_active_document(&mut self, force: bool) -> bool {
        if self.active_buffer().dirty && !force {
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
            self.save_session_state(false);
            return false;
        }
    }

    fn handle_ex_action(&mut self, action: ExCommandAction) {
        match action {
            ExCommandAction::Save(Some(path_str)) => {
                let path = PathBuf::from(path_str);
                match { let (p, b) = self.active_pane_and_buffer(); p.save_as(b, &path) } {
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
                if !self.active_buffer().dirty {
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


    pub fn run_startup_flow(&mut self, startup_args: crate::startup::StartupArgs) {
        if startup_args.register_file_associations {
            let config = self.file_association_config();
            let _ = crate::platform::file_association::register_file_associations(&config);
            crate::platform::file_association::notify_shell_association_changed();
            std::process::exit(0);
        }

        if startup_args.unregister_file_associations {
            let config = self.file_association_config();
            let _ = crate::platform::file_association::unregister_file_associations(&config);
            crate::platform::file_association::notify_shell_association_changed();
            std::process::exit(0);
        }

        if !startup_args.files_to_open.is_empty() {
            self.open_files(startup_args.files_to_open);
            return;
        }

        if self.config.restore_last_session {
            let restored_count = self.restore_last_session();
            if restored_count > 0 {
                return;
            }
        }

        self.open_startup_fallback();
    }

    pub fn open_startup_fallback(&mut self) {
        if self.config.show_launcher_on_startup {
            self.last_message = "Ready.".to_string();
            // Assuming launcher is shown by default if there's no open document or based on some state.
            // In the original app, new_file() might be called.
            // Let's ensure there is at least one blank document if needed, or clear.
            if self.documents.is_empty() {
                self.new_file();
            } else if !self.active_pane().is_open(self.active_buffer()) {
                // blank note
            }
        } else {
            self.new_file();
        }
    }

    fn file_association_config(&self) -> crate::platform::file_association::FileAssociationConfig {
        let app_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("NeoNote.exe"));
        crate::platform::file_association::FileAssociationConfig {
            app_name: "NeoNote".to_string(),
            app_exe_name: "NeoNote.exe".to_string(),
            app_path,
            prog_id: "NeoNote.File".to_string(),
            friendly_file_type_name: "NeoNote Document".to_string(),
            extensions: vec![
                ".txt".to_string(),
                ".md".to_string(),
                ".markdown".to_string(),
                ".log".to_string(),
                ".note".to_string(),
                ".neonote".to_string(),
            ],
        }
    }

    pub fn restore_last_session(&mut self) -> usize {
        self.suppress_session_save = true;
        let mut restored_count = 0;
        let mut to_open = Vec::new();

        // clone to avoid borrow checker issues
        let opened_files = self.session.opened_files.clone();
        let active_document = self.session.active_document.clone();

        for session_file in opened_files {
            if session_file.path.as_ref().map_or(true, |p| p.exists()) {
                to_open.push(session_file);
            }
        }

        for session_file in to_open {
            let mut opened = false;
            if let Some(path) = &session_file.path {
                if self.open_file_or_focus_existing(path.clone()).is_ok() {
                    opened = true;
                }
            } else {
                self.new_file();
                opened = true;
            }

            if opened {
                { let (p, b) = self.active_pane_and_buffer(); p.set_cursor_line(b, session_file.cursor_line); }
                { let (p, b) = self.active_pane_and_buffer(); p.set_cursor_col(b, session_file.cursor_col); }
                { let (p, b) = self.active_pane_and_buffer(); p.set_viewport_top_line(b, session_file.viewport_top_line); }
                
                if let Some(content) = session_file.unsaved_content {
                    let (_, b) = self.active_pane_and_buffer();
                    b.content = content;
                    b.dirty = session_file.is_dirty;
                }
                
                restored_count += 1;
            }
        }

        if let Some(active_path) = active_document {
            let _ = self.open_file_or_focus_existing(active_path);
        }

        self.suppress_session_save = false;
        restored_count
    }

    pub fn open_files(&mut self, paths: Vec<PathBuf>) {
        for path in paths {
            if let Err(error) = self.open_file_or_focus_existing(path) {
                self.last_message = format!("Failed to open file: {}", error);
            }
        }
    }

    pub fn open_file_or_focus_existing(&mut self, path: PathBuf) -> anyhow::Result<()> {
        let canonical_path = std::fs::canonicalize(&path).unwrap_or_else(|_| path.to_path_buf());

        if let Some(index) = self.find_open_document_by_path(&canonical_path) {
            self.active_document = index;
            self.last_message = format!("Focused existing file: {}", canonical_path.display());
            return Ok(());
        }

        self.open_path(canonical_path);
        Ok(())
    }

    fn find_open_document_by_path(&self, canonical_path: &std::path::Path) -> Option<usize> {
        for (i, doc) in self.documents.iter().enumerate() {
            let buffer = self.buffers.get(&doc.buffer_id).unwrap();
            if let Some(doc_path_str) = doc.path_string(buffer) {
                let doc_path = PathBuf::from(doc_path_str);
                let doc_canonical = std::fs::canonicalize(&doc_path).unwrap_or_else(|_| doc_path);
                if doc_canonical == canonical_path {
                    return Some(i);
                }
            }
        }
        None
    }

    pub fn save_session_state(&mut self, force: bool) {
        if self.suppress_session_save {
            return;
        }

        let now = std::time::Instant::now();
        if !force {
            if let Some(last) = self.last_session_save {
                if now.duration_since(last).as_secs() < 2 {
                    return;
                }
            }
        }
        self.last_session_save = Some(now);

        let mut opened_files = Vec::new();
        for doc in &self.documents {
            let buffer = self.buffers.get(&doc.buffer_id).unwrap();
            if doc.is_open(buffer) {
                let path = doc.path_string(buffer).map(|p| {
                    let p_buf = PathBuf::from(&p);
                    std::fs::canonicalize(&p_buf).unwrap_or_else(|_| p_buf)
                });

                let is_dirty = buffer.dirty;
                let unsaved_content = if is_dirty {
                    Some(buffer.content.clone())
                } else {
                    None
                };

                opened_files.push(crate::persistence::session::SessionFile {
                    path,
                    cursor_line: doc.cursor_line(buffer),
                    cursor_col: doc.cursor_col(buffer),
                    viewport_top_line: doc.viewport_top_line(buffer),
                    is_dirty,
                    unsaved_content,
                });
            }
        }

        let active_document = self.active_pane().path_string(self.active_buffer()).map(|p| {
            let p_buf = PathBuf::from(&p);
            std::fs::canonicalize(&p_buf).unwrap_or_else(|_| p_buf)
        });

        self.session.version = 1;
        self.session.opened_files = opened_files;
        self.session.active_document = active_document;

        let _ = self.session.save(&self.paths);
    }

    fn open_path(&mut self, path: PathBuf) {
        if let Some(index) = self.document_index_for_path(&path) {
            self.active_document = index;
            self.last_message = "File already open.".to_string();
            return;
        }

        let buffer_id = BufferId(self.next_buffer_id);
        self.next_buffer_id += 1;
        let pane_id = PaneId(self.next_pane_id);
        self.next_pane_id += 1;

        let mut note = Pane::new(pane_id, buffer_id);
        note.defer_enabled = true;
        
        let mut buffer = TextBuffer::new();
        match note.open(&mut buffer, &path) {
            Ok(()) => {
                self.buffers.insert(buffer_id, buffer);
                let active_buffer = self.active_buffer();
                let is_untitled_empty = self.active_pane().path_string(active_buffer).is_none() && self.active_pane().is_empty(active_buffer);
                if self.documents.len() == 1 && is_untitled_empty {
                    self.documents[0] = note;
                    self.active_document = 0;
                } else {
                    self.documents.push(note);
                    self.active_document = self.documents.len().saturating_sub(1);
                }
                self.remember_recent(path);

                self.last_message = "Opened note.".to_string();
                self.save_session_state(false);
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
                if self.active_pane().clipboard_register_text(self.active_buffer()) == text {
                    return;
                }
                let linewise = text.ends_with('\n');
                { let (p, b) = self.active_pane_and_buffer(); p.set_clipboard_register(b, text, linewise); }
            }
            Err(error) => self.last_message = format!("Could not read clipboard: {error}"),
        }
    }

    fn export_unnamed_register_if_changed(&mut self, before: Option<RegisterSnapshot>) {
        let Some(before) = before else {
            return;
        };

        let after = self.active_pane().unnamed_register_snapshot(self.active_buffer());
        if after == before || after.text.is_empty() {
            return;
        }

        if let Err(error) = clipboard::write_text(&after.text) {
            self.last_message = format!("Could not update clipboard: {error}");
        }
    }

    
    pub(crate) fn active_pane_and_buffer(&mut self) -> (&mut Pane, &mut TextBuffer) {
        let index = self
            .active_document
            .min(self.documents.len().saturating_sub(1));
        let pane = &mut self.documents[index];
        let buffer = self.buffers.get_mut(&pane.buffer_id).unwrap();
        (pane, buffer)
    }

    pub(crate) fn active_pane(&self) -> &Pane {
        &self.documents[self
            .active_document
            .min(self.documents.len().saturating_sub(1))]
    }

    pub(crate) fn active_buffer(&self) -> &TextBuffer {
        self.buffers.get(&self.active_pane().buffer_id).unwrap()
    }

    pub(crate) fn active_note(&self) -> &Pane {
        &self.documents[self
            .active_document
            .min(self.documents.len().saturating_sub(1))]
    }

    pub(crate) fn active_note_mut(&mut self) -> &mut Pane {
        let index = self
            .active_document
            .min(self.documents.len().saturating_sub(1));
        &mut self.documents[index]
    }

    fn document_index_for_path(&self, path: &std::path::Path) -> Option<usize> {
        self.documents.iter().position(|note| {
            note.path_string(self.active_buffer())
                .map(|open_path| PathBuf::from(open_path) == path)
                .unwrap_or(false)
        })
    }

    fn document_tabs(&self) -> Vec<DocumentTabSnapshot> {
        self.documents
            .iter()
            .enumerate()
            .filter_map(|(index, note)| {
                let buffer = self.buffers.get(&note.buffer_id)?;
                if !note.is_open(buffer) {
                    return None;
                }
                
                Some(DocumentTabSnapshot {
                    index: index as i32,
                    title: note.title(buffer),
                    is_active: index == self.active_document,
                    is_modified: buffer.dirty,
                })
            })
            .collect()
    }

    fn open_document_count(&self) -> usize {
        self.documents
            .iter()
            .filter(|note| {
                self.buffers.get(&note.buffer_id).map_or(false, |b| note.is_open(b))
            })
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

fn cursor_snapshot(note: &Pane, buffer: &TextBuffer) -> CursorSnapshot {
    let line = note
        .content(buffer)
        .split('\n')
        .nth(note.cursor_line(buffer))
        .unwrap_or_default();
    let col = note.display_cursor_col(buffer);
    let prefix = line.chars().take(col).collect::<String>();
    let suffix_start = if note.mode(buffer) == VimMode::Normal {
        col.saturating_add(1)
    } else {
        col
    };
    let cell = if note.mode(buffer) == VimMode::Normal {
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

fn editor_lines(note: &Pane, buffer: &TextBuffer) -> Vec<EditorLineSnapshot> {
    let cursor_line = note.cursor_line(buffer);
    let cursor_column = note.display_cursor_col(buffer) as i32;
    let cursor_block = note.mode(buffer) != VimMode::Insert;
    let cursor = cursor_snapshot(note, buffer);

    content_lines(buffer)
        .into_iter()
        .enumerate()
        .map(|(index, text)| {
            let (selected_prefix, selected_text) = {
                let cols = note.line_selection_cols(buffer, index);
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
            is_search_match: note.line_has_search_match(buffer, index),
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

fn content_lines(buffer: &TextBuffer) -> Vec<String> {
    if buffer.content.is_empty() {
        vec![String::new()]
    } else {
        buffer.content.split('\n').map(ToOwned::to_owned).collect()
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

        assert_eq!(controller.active_note().content(controller.active_buffer()), "b");
        controller.previous_document();
        assert_eq!(controller.active_note().content(controller.active_buffer()), "a");
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
        dbg!(controller.documents.len());
        controller.open_path(first);
        dbg!(controller.documents.len());
        controller.open_path(second);
        dbg!(controller.documents.len());

        assert_eq!(controller.open_document_count(), 2);
        assert_eq!(controller.active_note().content(controller.active_buffer()), "two");
        controller.previous_document();
        assert_eq!(controller.active_note().content(controller.active_buffer()), "one");
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

        controller.handle_editor_key("escape");
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
        { let (p, b) = controller.active_pane_and_buffer(); *p.content_mut(b) = "one\ntwo\nthree".to_string(); }
        { let (p, b) = controller.active_pane_and_buffer(); p.enter_normal(b); }

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
        assert_eq!(controller.active_note().cursor_line(controller.active_buffer()), 0);

        // Press yy
        controller.handle_editor_key("y");
        controller.handle_editor_key("y");

        assert_eq!(controller.active_note().unnamed_register_text(controller.active_buffer()), "line1\n");
        if controller.config.sync_clipboard {
            assert_eq!(clipboard::read_text().unwrap().replace("\r\n", "\n"), "line1\n");
        }
    }
}
