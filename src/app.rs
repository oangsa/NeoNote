use std::path::PathBuf;

use crate::{
    notes::{NoteDocument, VimMode},
    persistence::{AppConfig, AppDataPaths, RecentFiles, SessionState},
    theme::ThemeStore,
};

pub struct AppController {
    paths: AppDataPaths,
    config: AppConfig,
    session: SessionState,
    recent_files: RecentFiles,
    themes: ThemeStore,
    documents: Vec<NoteDocument>,
    active_document: usize,
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
    pub theme: ThemeSnapshot,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EditorLineSnapshot {
    pub number: i32,
    pub text: String,
    pub is_cursor_line: bool,
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
        let mut config = AppConfig::load_or_default(&paths);
        migrate_old_default_theme(&paths, &mut config);
        let session = SessionState::load_or_default(&paths);
        let recent_files = RecentFiles::load_or_default(&paths);
        let themes = ThemeStore::load(&paths, config.active_theme.as_deref());

        Self {
            paths,
            config,
            session,
            recent_files,
            themes,
            documents: vec![NoteDocument::default()],
            active_document: 0,
            last_message: String::new(),
        }
    }

    pub fn new_file(&mut self) {
        let mut note = NoteDocument::default();
        note.new_blank();
        if self.documents.len() == 1 && !self.active_note().is_open() {
            self.documents[0] = note;
            self.active_document = 0;
        } else {
            self.documents.push(note);
            self.active_document = self.documents.len().saturating_sub(1);
        }
        self.last_message = "New note ready.".to_string();
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
        self.last_message = "Theme panel migration to Slint is next.".to_string();
    }

    pub fn open_settings_panel(&mut self) {
        self.last_message = "Settings panel migration to Slint is next.".to_string();
    }

    pub fn handle_editor_key(&mut self, key: &str) {
        if !self.active_note().is_open() {
            return;
        }

        match self.active_note().mode() {
            VimMode::Normal => self.handle_normal_editor_key(key),
            VimMode::Insert => self.handle_insert_editor_key(key),
        }
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
    }

    pub fn next_document(&mut self) {
        if self.documents.is_empty() {
            return;
        }

        self.active_document = (self.active_document + 1) % self.documents.len();
        self.last_message = format!("Switched to {}.", self.active_note().title());
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
            editor_lines: editor_lines(note),
            document_tabs: self.document_tabs(),
            status_text: if note.is_open() {
                format!(
                    " {}{}",
                    note.title(),
                    if note.dirty() { " [+]" } else { "" },
                )
            } else {
                " [No Name]".to_string()
            },
            status_right: if note.is_open() {
                format!(
                    "Doc {}/{}  |  Ln {}, Col {}  |  {} lines, {} words, {} chars  ",
                    self.active_document + 1,
                    self.open_document_count(),
                    note.cursor_line() + 1,
                    note.display_cursor_col() + 1,
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
            cursor_block: note.mode() == VimMode::Normal,
            mode_text,
            message: self.last_message.clone(),
            has_document: note.is_open(),
            editor_font_family: crate::platform::fonts::select_editor_font(&self.config.font_family),
            theme,
        }
    }

    fn handle_normal_editor_key(&mut self, key: &str) {
        let normal_key = match key {
            "escape" => {
                self.active_note_mut().enter_normal();
                return;
            }
            "left" => "h",
            "right" => "l",
            "up" => "k",
            "down" => "j",
            "return" | "backspace" | "delete" => return,
            value => value,
        };

        if self.active_note_mut().handle_normal_input(normal_key) {
            self.last_message.clear();
        }
    }

    fn handle_insert_editor_key(&mut self, key: &str) {
        match key {
            "escape" => self.active_note_mut().enter_normal(),
            "return" => self.active_note_mut().insert_newline(),
            "backspace" => self.active_note_mut().backspace(),
            "delete" => self.active_note_mut().delete_at_cursor(),
            "left" | "right" | "up" | "down" => {}
            text if is_printable_editor_text(text) => {
                self.active_note_mut().handle_insert_text(text)
            }
            _ => {}
        }
    }

    fn open_path(&mut self, path: PathBuf) {
        if let Some(index) = self.document_index_for_path(&path) {
            self.active_document = index;
            self.last_message = "File already open.".to_string();
            return;
        }

        let mut note = NoteDocument::default();
        match note.open(&path) {
            Ok(()) => {
                if self.documents.len() == 1 && !self.active_note().is_open() {
                    self.documents[0] = note;
                    self.active_document = 0;
                } else {
                    self.documents.push(note);
                    self.active_document = self.documents.len().saturating_sub(1);
                }
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
            "VISUAL" => theme.vim_modes.visual.clone(),
            "COMMAND" => theme.vim_modes.command.clone(),
            "REPLACE" => theme.vim_modes.replace.clone(),
            _ => theme.colors.accent_primary.clone(),
        }
    }

    fn active_note(&self) -> &NoteDocument {
        &self.documents[self
            .active_document
            .min(self.documents.len().saturating_sub(1))]
    }

    fn active_note_mut(&mut self) -> &mut NoteDocument {
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

fn is_printable_editor_text(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|ch| !ch.is_control())
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

fn editor_lines(note: &NoteDocument) -> Vec<EditorLineSnapshot> {
    let cursor_line = note.cursor_line();
    let cursor_column = note.display_cursor_col() as i32;
    let cursor_block = note.mode() == VimMode::Normal;
    let cursor = cursor_snapshot(note);

    content_lines(note)
        .into_iter()
        .enumerate()
        .map(|(index, text)| EditorLineSnapshot {
            number: (index + 1) as i32,
            is_cursor_line: index == cursor_line,
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
        for key in [
            "a", "b", "c", "return", "return", "d", "e", "f", "escape",
        ] {
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
}
