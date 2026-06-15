use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentSettings {
    pub number: bool,
    pub relativenumber: bool,
    pub wrap: bool,
    pub tabstop: usize,
    pub shiftwidth: usize,
    pub ignorecase: bool,
    pub smartcase: bool,
    pub hlsearch: bool,
    pub incsearch: bool,
}

impl Default for DocumentSettings {
    fn default() -> Self {
        Self {
            number: false,
            relativenumber: false,
            wrap: false,
            tabstop: 4,
            shiftwidth: 4,
            ignorecase: false,
            smartcase: false,
            hlsearch: true,
            incsearch: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BufferId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PaneId(pub usize);

pub struct TextBuffer {
    pub content: String,
    pub path: Option<PathBuf>,
    pub encoding: TextEncoding,
    pub dirty: bool,
    pub open: bool,
    pub registers: Registers,
    pub undo_stack: Vec<DocumentSnapshot>,
    pub redo_stack: Vec<DocumentSnapshot>,
    pub changelist: Vec<CursorPosition>,
    pub changelist_index: Option<usize>,
    pub settings: DocumentSettings,
    pub search: SearchState,
    pub search_history: Vec<String>,
    pub marks: BTreeMap<char, CursorPosition>,
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            path: None,
            encoding: TextEncoding::Utf8,
            dirty: false,
            open: true,
            registers: Registers::default(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            changelist: Vec::new(),
            changelist_index: None,
            settings: DocumentSettings::default(),
            search: SearchState::default(),
            search_history: Vec::new(),
            marks: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod unicode_tests {
    use super::*;

    #[test]
    fn utf16_files_round_trip_unicode_content() {
        let root = std::env::temp_dir().join(format!(
            "neonote-unicode-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("unicode.txt");
        let content = "こんにちは\nПривет\nمرحبا";
        let mut bytes = vec![0xFF, 0xFE];
        for unit in content.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        std::fs::write(&path, bytes).unwrap();

        let mut pane = Pane::new(PaneId(1), BufferId(1));
        let mut buffer = TextBuffer::new();
        pane.open(&mut buffer, &path).unwrap();

        assert_eq!(buffer.content, content);
        assert_eq!(buffer.encoding, TextEncoding::Utf16Le);

        pane.save(&mut buffer).unwrap();
        let saved = std::fs::read(&path).unwrap();
        assert_eq!(&saved[..2], &[0xFF, 0xFE]);
    }

    #[test]
    fn ignorecase_search_keeps_unicode_match_ranges_stable() {
        let mut buffer = TextBuffer::new();
        buffer.content = "AİB".to_string();
        buffer.settings.ignorecase = true;
        buffer.search.pattern = "İ".to_string();

        let mut pane = Pane::new(PaneId(1), BufferId(1));
        pane.refresh_search_matches(&mut buffer);

        assert_eq!(buffer.search.matches, vec![TextRange::new(1, 2)]);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextEncoding {
    Utf8,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
}

#[derive(Clone)]
pub struct Pane {
    pub id: PaneId,
    pub buffer_id: BufferId,
    pub vim_state: VimState,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub count: Option<usize>,
    pub selected_register: Option<RegisterTarget>,
    pub last_change: Option<Vec<String>>,
    pub replaying_change: bool,
    pub visual_anchor_flat: Option<usize>,
    pub last_visual_selection: Option<(TextRange, VisualKind)>,
    pub jump_list: Vec<CursorPosition>,
    pub jump_index: Option<usize>,
    pub yank_highlight: Option<TextRange>,
    pub deferred_action: Option<DeferredAction>,
    pub(crate) defer_enabled: bool,
    pub pending_ex_action: Option<ExCommandAction>,
    pub viewport_state: ViewportState,
    pub command_history: Vec<String>,
    pub last_substitute: Option<LastSubstitute>,
    pub macro_depth: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ViewportState {
    pub top_line: usize,
    pub visible_lines: usize,
}

impl Pane {
    pub fn new(id: PaneId, buffer_id: BufferId) -> Self {
        Self {
            id,
            buffer_id,
            vim_state: VimState::default(),
            cursor_line: 0,
            cursor_col: 0,
            count: None,
            selected_register: None,
            last_change: None,
            replaying_change: false,
            visual_anchor_flat: None,
            last_visual_selection: None,
            jump_list: Vec::new(),
            jump_index: None,
            yank_highlight: None,
            deferred_action: None,
            defer_enabled: false,
            pending_ex_action: None,
            viewport_state: ViewportState::default(),
            command_history: Vec::new(),
            last_substitute: None,
            macro_depth: 0,
        }
    }
}

impl Pane {
    pub fn new_blank(&mut self, buffer: &mut TextBuffer) {
        buffer.content.clear();
        buffer.path = None;
        buffer.encoding = TextEncoding::Utf8;
        buffer.dirty = false;
        buffer.open = true;
        self.vim_state = VimState::default();
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.count = None;
        buffer.registers = Registers::default();
        self.selected_register = None;
        buffer.undo_stack.clear();
        buffer.redo_stack.clear();
        self.last_change = None;
        self.replaying_change = false;
        self.visual_anchor_flat = None;
        self.last_visual_selection = None;
        buffer.search = SearchState::default();
        buffer.marks.clear();
        self.jump_list.clear();
        self.jump_index = None;
        self.yank_highlight = None;
        self.defer_enabled = false;
        self.pending_ex_action = None;
        buffer.changelist.clear();
        buffer.changelist_index = None;
        self.viewport_state = ViewportState::default();
    }

    pub fn open(&mut self, buffer: &mut TextBuffer, path: impl AsRef<Path>) -> std::io::Result<()> {
        let path = path.as_ref();
        let (content, encoding) = decode_text_file(&fs::read(path)?)?;
        buffer.content = content.replace("\r\n", "\n");
        buffer.path = Some(path.to_path_buf());
        buffer.encoding = encoding;
        buffer.dirty = false;
        buffer.open = true;
        self.vim_state = VimState::default();
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.count = None;
        buffer.registers = Registers::default();
        self.selected_register = None;
        buffer.undo_stack.clear();
        buffer.redo_stack.clear();
        self.last_change = None;
        self.replaying_change = false;
        self.visual_anchor_flat = None;
        self.last_visual_selection = None;
        buffer.search = SearchState::default();
        buffer.marks.clear();
        self.jump_list.clear();
        self.jump_index = None;
        self.yank_highlight = None;
        self.deferred_action = None;
        self.pending_ex_action = None;
        Ok(())
    }

    pub fn save(&mut self, buffer: &mut TextBuffer) -> std::io::Result<()> {
        let Some(path) = buffer.path.clone() else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "No file path selected",
            ));
        };
        self.save_as(buffer, path)
    }

    pub fn save_as(&mut self, buffer: &mut TextBuffer, path: impl AsRef<Path>) -> std::io::Result<()> {
        let path = path.as_ref();
        fs::write(path, encode_text_file(&buffer.content, buffer.encoding))?;
        buffer.path = Some(path.to_path_buf());
        buffer.dirty = false;
        Ok(())
    }

    pub fn content_mut<'a>(&mut self, buffer: &'a mut TextBuffer) -> &'a mut String {
        &mut buffer.content
    }

    pub fn content<'a>(&self, buffer: &'a TextBuffer) -> &'a str {
        &buffer.content
    }

    pub fn content_with_cursor_marker(&self, buffer: &TextBuffer) -> String {
        self.lines_vec(buffer, )
            .iter()
            .enumerate()
            .map(|(line_index, line)| self.line_with_cursor_marker(buffer, line, line_index))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn is_empty(&self, buffer: &TextBuffer) -> bool {
        buffer.content.is_empty()
    }

    pub fn is_open(&self, buffer: &TextBuffer) -> bool {
        buffer.open
    }

    pub fn mark_dirty(&mut self, buffer: &mut TextBuffer) {
        buffer.dirty = true;
    }

    pub fn dirty(&self, buffer: &mut TextBuffer) -> bool {
        buffer.dirty
    }

    pub fn mode(&self, _buffer: &TextBuffer) -> VimMode {
        self.vim_state.mode
    }

    pub fn cursor_line(&self, buffer: &TextBuffer) -> usize {
        self.cursor_line.min(self.line_count(buffer, ).saturating_sub(1))
    }

    pub fn set_cursor_line(&mut self, _buffer: &mut TextBuffer, line: usize) {
        self.cursor_line = line;
    }

    pub fn cursor_col(&self, buffer: &TextBuffer) -> usize {
        self.cursor_col.min(self.current_line_max_col(buffer, ))
    }

    pub fn set_cursor_col(&mut self, _buffer: &mut TextBuffer, col: usize) {
        self.cursor_col = col;
    }

    pub fn viewport_top_line(&self, _buffer: &TextBuffer) -> usize {
        self.viewport_state.top_line
    }

    pub fn set_viewport_top_line(&mut self, _buffer: &mut TextBuffer, top_line: usize) {
        self.viewport_state.top_line = top_line;
    }

    pub fn display_cursor_col(&self, buffer: &TextBuffer) -> usize {
        match self.vim_state.mode {
            VimMode::Normal | VimMode::Visual | VimMode::VisualLine => self.cursor_col(buffer, ),
            VimMode::Insert => self.cursor_col.min(self.current_line_char_count(buffer, )),
            _ => self.cursor_col(buffer, ),
        }
    }

    pub fn line_selection_cols(&self, buffer: &TextBuffer, line: usize) -> Option<(usize, usize)> {
        let check_range = |buffer: &TextBuffer, range: TextRange| -> Option<(usize, usize)> {
            let start_line = self.line_for_flat(buffer, range.start);
            let end_line = self.line_for_flat(buffer, range.end.saturating_sub(1));
            
            let min_line = start_line.min(end_line);
            let max_line = start_line.max(end_line);

            if (min_line..=max_line).contains(&line) {
                let start_col = if line == min_line {
                    let min_flat = range.start.min(range.end.saturating_sub(1));
                    min_flat - self.flat_index_for_line(buffer, line)
                } else {
                    0
                };

                let end_col = if line == max_line {
                    let max_flat = range.start.max(range.end);
                    max_flat - self.flat_index_for_line(buffer, line)
                } else {
                    usize::MAX
                };

                Some((start_col, end_col))
            } else {
                None
            }
        };

        if let Some(range) = self.yank_highlight {
            if let Some(cols) = check_range(buffer, range) {
                return Some(cols);
            }
        }
        
        let vsr = self.visual_selection_range(buffer, );
        vsr.and_then(|r| check_range(buffer, r))
    }

    pub fn has_yank_highlight(&self, _buffer: &TextBuffer) -> bool {
        self.yank_highlight.is_some()
    }

    pub fn clear_yank_highlight(&mut self, _buffer: &mut TextBuffer) {
        self.yank_highlight = None;
    }

    pub fn has_deferred_action(&self, _buffer: &TextBuffer) -> bool {
        self.deferred_action.is_some()
    }

    pub fn flush_deferred_action(&mut self, buffer: &mut TextBuffer) -> bool {
        if let Some(action) = self.deferred_action.take() {
            self.yank_highlight = None;
            self.apply_operator_range_direct(buffer, action.operator, action.range)
        } else {
            false
        }
    }

    pub fn line_has_search_match(&self, buffer: &TextBuffer, line: usize) -> bool {
        if !buffer.search.highlights_active {
            return false;
        }
        buffer.search
            .matches
            .iter()
            .any(|range| self.line_for_flat(buffer, range.start) == line)
    }

    pub fn search_pattern<'a>(&self, buffer: &'a TextBuffer) -> Option<&'a str> {
        (!buffer.search.pattern.is_empty()).then_some(buffer.search.pattern.as_str())
    }

    pub fn unnamed_register_text<'a>(&self, buffer: &'a TextBuffer) -> &'a str {
        &buffer.registers.unnamed.text
    }

    pub fn yank_register_text<'a>(&self, buffer: &'a TextBuffer) -> &'a str {
        &buffer.registers.yank.text
    }

    pub fn unnamed_register_is_linewise(&self, buffer: &mut TextBuffer) -> bool {
        buffer.registers.unnamed.linewise
    }

    pub fn yank_register_is_linewise(&self, buffer: &mut TextBuffer) -> bool {
        buffer.registers.yank.linewise
    }

    pub fn unnamed_register_snapshot(&self, buffer: &TextBuffer) -> RegisterSnapshot {
        RegisterSnapshot {
            text: buffer.registers.unnamed.text.clone(),
            linewise: buffer.registers.unnamed.linewise,
            blockwise: buffer.registers.unnamed.blockwise,
            version: buffer.registers.version,
        }
    }

    pub fn set_unnamed_register(&mut self, buffer: &mut TextBuffer, text: String, linewise: bool) {
        buffer.registers.unnamed = RegisterValue { text, linewise, blockwise: false };
    }

    pub fn set_clipboard_register(&mut self, buffer: &mut TextBuffer, text: String, linewise: bool) {
        let value = RegisterValue { text, linewise, blockwise: false };
        buffer.registers.clipboard = value.clone();
        buffer.registers.unnamed = value;
    }

    pub fn clipboard_register_text<'a>(&self, buffer: &'a TextBuffer) -> &'a str {
        &buffer.registers.clipboard.text
    }

    pub fn named_register_text<'a>(&self, buffer: &'a TextBuffer, name: char) -> Option<&'a str> {
        buffer.registers
            .named
            .get(&name.to_ascii_lowercase())
            .map(move |value| value.text.as_str())
    }

    pub fn set_cursor_from_pointer(&mut self, buffer: &mut TextBuffer, line: usize, column: usize) {
        self.flush_deferred_action(buffer, );
        self.clear_yank_highlight(buffer, );
        self.vim_state.pending_command = None;
        self.count = None;
        self.visual_anchor_flat = None;
        if matches!(self.vim_state.mode, VimMode::Visual | VimMode::VisualLine) {
            self.vim_state.mode = VimMode::Normal;
        }
        self.cursor_line = line.min(self.line_count(buffer, ).saturating_sub(1));
        self.cursor_col = match self.vim_state.mode {
            VimMode::Normal | VimMode::Visual | VimMode::VisualLine => {
                column.min(self.current_line_max_col(buffer, ))
            }
            VimMode::Insert => column.min(self.current_line_char_count(buffer, )),
            _ => column.min(self.current_line_char_count(buffer, )),
        };
    }

    pub fn enter_insert(&mut self, buffer: &mut TextBuffer) {
        self.record_undo(buffer);
        self.vim_state.mode = VimMode::Insert;
        self.vim_state.pending_command = None;
        self.count = None;
        self.visual_anchor_flat = None;
        self.vim_state.insert_start_pos = Some(self.flattened_cursor(buffer, ));
        self.vim_state.macro_insert_start_index = Some(self.vim_state.current_macro.len());
        self.vim_state.insert_pending = None;
    }

    fn enter_replace_mode(&mut self, buffer: &mut TextBuffer) {
        self.record_undo(buffer);
        self.vim_state.mode = VimMode::Replace;
        self.vim_state.pending_command = None;
        self.count = None;
        self.visual_anchor_flat = None;
        self.vim_state.insert_start_pos = Some(self.flattened_cursor(buffer, ));
        self.vim_state.macro_insert_start_index = Some(self.vim_state.current_macro.len());
        self.vim_state.insert_pending = None;
    }

    pub fn enter_normal(&mut self, buffer: &mut TextBuffer) {
        self.flush_deferred_action(buffer, );
        self.clear_yank_highlight(buffer, );
        if self.vim_state.mode == VimMode::Insert || self.vim_state.mode == VimMode::Replace {
            self.vim_state.last_insert_pos = Some(self.flattened_cursor(buffer, ));
            if !self.replaying_change {
                if let (Some(start), Some(macro_idx)) = (self.vim_state.insert_start_pos, self.vim_state.macro_insert_start_index) {
                    let end = self.flattened_cursor(buffer, );
                    let mut net_sequence = Vec::new();
                    if end < start {
                        for _ in 0..(start - end) {
                            net_sequence.push("backspace".to_string());
                        }
                    } else if end > start {
                        let text = self.text_for_range(buffer, TextRange::new(start, end));
                        if !text.is_empty() {
                            net_sequence.push(text);
                        }
                    }
                    net_sequence.push("escape".to_string());
                    
                    self.vim_state.current_macro.truncate(macro_idx);
                    self.vim_state.current_macro.extend(net_sequence);
                } else {
                    self.vim_state.current_macro.push("escape".to_string());
                }

                if is_repeatable_change(&self.vim_state.current_macro) {
                    self.last_change = Some(self.vim_state.current_macro.clone());
                }
                self.vim_state.current_macro.clear();
            }
        }
        self.vim_state.mode = VimMode::Normal;
        self.vim_state.pending_command = None;
        self.clear_command_line(buffer, );
        self.count = None;
        self.visual_anchor_flat = None;
        self.vim_state.insert_pending = None;
        self.clamp_cursor_normal(buffer);
    }

    pub fn cancel_insert(&mut self, buffer: &mut TextBuffer) {
        self.flush_deferred_action(buffer, );
        self.clear_yank_highlight(buffer, );
        if self.vim_state.mode == VimMode::Insert {
            self.vim_state.last_insert_pos = Some(self.flattened_cursor(buffer, ));
            if !self.replaying_change {
                self.vim_state.current_macro.push("ctrl+c".to_string());
                if is_repeatable_change(&self.vim_state.current_macro) {
                    self.last_change = Some(self.vim_state.current_macro.clone());
                }
                self.vim_state.current_macro.clear();
            }
        }
        self.vim_state.mode = VimMode::Normal;
        self.vim_state.pending_command = None;
        self.clear_command_line(buffer, );
        self.count = None;
        self.visual_anchor_flat = None;
        self.vim_state.insert_pending = None;
        // Deliberately no clamp_cursor_normal here (cursor stays after text)
    }

    pub fn begin_insert_register_paste(&mut self, _buffer: &mut TextBuffer) {
        if matches!(self.vim_state.mode, VimMode::Insert | VimMode::Replace) {
            self.vim_state.insert_pending = Some(InsertPending::RegisterPaste);
        }
    }

    pub fn resolve_insert_register_paste(&mut self, buffer: &mut TextBuffer, key: &str) -> bool {
        if self.vim_state.insert_pending != Some(InsertPending::RegisterPaste) {
            return false;
        }
        let Some(register_name) = normalize_register_name(key) else {
            self.vim_state.insert_pending = None;
            return false;
        };
        self.vim_state.insert_pending = None;
        let register = buffer.registers.register(register_name).clone();
        if register.text.is_empty() {
            return false;
        }
        self.handle_insert_text(buffer, &register.text);
        true
    }

    fn clear_command_line(&mut self, _buffer: &mut TextBuffer) {
        self.vim_state.command_line.input.clear();
        self.vim_state.command_line.cursor = 0;
        self.vim_state.command_line.history_index = None;
        self.vim_state.command_line.saved_current = None;
        self.vim_state.command_line.is_search = false;
    }

    fn begin_command_line(&mut self, _buffer: &mut TextBuffer, mode: VimMode, initial: &str, is_search: bool) {
        self.vim_state.mode = mode;
        self.vim_state.command_line.input.clear();
        self.vim_state.command_line.input.push_str(initial);
        self.vim_state.command_line.cursor = self.vim_state.command_line.input.chars().count();
        self.vim_state.command_line.history_index = None;
        self.vim_state.command_line.saved_current = None;
        self.vim_state.command_line.is_search = is_search;
        self.vim_state.pending_command = None;
        self.count = None;
    }

    pub fn handle_editor_key(&mut self, buffer: &mut TextBuffer, key: crate::vim::key::EditorKey) -> bool {
        self.flush_deferred_action(buffer, );
        self.clear_yank_highlight(buffer, );
        
        let is_insert = self.vim_state.mode == VimMode::Insert || self.vim_state.mode == VimMode::Replace;
        
        if let Some(reg) = self.vim_state.recording_macro {
            if self.macro_depth == 0 {
                let is_stop_key = key == crate::vim::key::EditorKey::Input("q".to_string()) 
                    && !is_insert 
                    && self.vim_state.pending_command.is_none();
                if !is_stop_key {
                    self.vim_state.macros.entry(reg).or_default().push(key.clone());
                }
            }
        }
        
        if is_insert {
            match key {
                crate::vim::key::EditorKey::EnterNormal => {
                    self.enter_normal(buffer, );
                    true
                }
                crate::vim::key::EditorKey::Cancel => {
                    self.cancel_insert(buffer, );
                    true
                }
                crate::vim::key::EditorKey::RegisterPaste => {
                    self.begin_insert_register_paste(buffer, );
                    true
                }
                crate::vim::key::EditorKey::Newline => {
                    self.insert_newline(buffer, );
                    true
                }
                crate::vim::key::EditorKey::Backspace => {
                    self.backspace(buffer, );
                    true
                }
                crate::vim::key::EditorKey::Delete => {
                    self.delete_at_cursor(buffer, );
                    true
                }
                crate::vim::key::EditorKey::DeleteWord => {
                    self.delete_word_insert(buffer, );
                    true
                }
                crate::vim::key::EditorKey::DeleteLine => {
                    self.delete_line_insert(buffer, );
                    true
                }
                crate::vim::key::EditorKey::Indent => {
                    let line = self.cursor_line(buffer, );
                    self.indent_lines(buffer, line, line, 1);
                    true
                }
                crate::vim::key::EditorKey::Dedent => {
                    let line = self.cursor_line(buffer, );
                    self.indent_lines(buffer, line, line, -1);
                    true
                }
                crate::vim::key::EditorKey::SingleNormalCommand => {
                    self.vim_state.mode = VimMode::Normal;
                    self.vim_state.return_to_insert = true;
                    true
                }
                crate::vim::key::EditorKey::DigraphPrefix => {
                    self.vim_state.insert_pending = Some(InsertPending::DigraphPrefix);
                    true
                }
                crate::vim::key::EditorKey::LiteralPrefix => {
                    self.vim_state.insert_pending = Some(InsertPending::LiteralPrefix);
                    true
                }
                crate::vim::key::EditorKey::Input(text) => {
                    if self.resolve_insert_register_paste(buffer, &text) {
                        return true;
                    }
                    if let Some(pending) = self.vim_state.insert_pending {
                        match pending {
                            InsertPending::LiteralPrefix => {
                                self.handle_insert_text(buffer, &text);
                                self.vim_state.insert_pending = None;
                                return true;
                            }
                            InsertPending::DigraphPrefix => {
                                if let Some(ch) = text.chars().next() {
                                    self.vim_state.insert_pending = Some(InsertPending::DigraphFirstChar(ch));
                                } else {
                                    self.vim_state.insert_pending = None;
                                }
                                return true;
                            }
                            InsertPending::DigraphFirstChar(first) => {
                                if let Some(second) = text.chars().next() {
                                    self.handle_insert_text(buffer, &format!("{}{}", first, second));
                                }
                                self.vim_state.insert_pending = None;
                                return true;
                            }
                            _ => {}
                        }
                    }
                    self.handle_insert_text(buffer, &text);
                    true
                }
                crate::vim::key::EditorKey::Ignore => false,
            }
        } else {
            match key {
                crate::vim::key::EditorKey::EnterNormal => {
                    self.enter_normal(buffer, );
                    true
                }
                crate::vim::key::EditorKey::Input(text) => {
                    self.handle_normal_input(buffer, &text)
                }
                _ => false,
            }
        }
    }

    pub fn handle_normal_input(&mut self, buffer: &mut TextBuffer, input: &str) -> bool {
        self.flush_deferred_action(buffer, );
        self.clear_yank_highlight(buffer, );
        if self.vim_state.mode == VimMode::Insert {
            return false;
        }

        if let Some(PendingCommand::SubstituteConfirm { .. }) = &self.vim_state.pending_command {
            return self.handle_substitute_confirm_input(buffer, input);
        }

        if matches!(self.vim_state.mode, VimMode::Command | VimMode::Search(_)) {
            return self.handle_single_key_event(buffer, input);
        }

        let is_command_like = matches!(self.vim_state.mode, VimMode::Command | VimMode::Search(_));
        let is_ignored = !is_command_like
            && (matches!(
                input,
                "shift"
                    | "control"
                    | "alt"
                    | "meta"
                    | "capslock"
                    | "tab"
                    | "insert"
                    | "delete"
                    | "home"
                    | "end"
                    | "pageup"
                    | "pagedown"
                    | "left"
                    | "right"
                    | "up"
                    | "down"
            ) || (input.starts_with('f')
                && input.len() > 1
                && input[1..].chars().all(|c| c.is_ascii_digit())));

        if is_ignored {
            return false;
        }

        if input == "escape" || input == "backspace" || input == "return"
            || input.starts_with("ctrl+") || input.starts_with("alt+") || input.starts_with("shift+")
        {
            return self.handle_single_key_event(buffer, input);
        }

        let mut changed = false;
        
        if input == "escape" || input == "backspace" || input == "return"
            || input.starts_with("ctrl+") || input.starts_with("alt+") || input.starts_with("shift+")
        {
            if !self.replaying_change {
                self.vim_state.current_macro.push(input.to_string());
            }
            changed = self.handle_single_key_event(buffer, input);
        } else {
            for ch in input.chars() {
                if !self.replaying_change {
                    self.vim_state.current_macro.push(ch.to_string());
                }
                changed |= self.handle_single_key_event(buffer, &ch.to_string());
            }
        }

        if !self.replaying_change {
            if changed {
                if is_repeatable_change(&self.vim_state.current_macro) {
                    self.last_change = Some(self.vim_state.current_macro.clone());
                }
            }
            if self.vim_state.pending_command.is_none() && self.count.is_none() && self.vim_state.mode == VimMode::Normal {
                self.vim_state.current_macro.clear();
            }
        }
        changed
    }

    fn handle_substitute_confirm_input(&mut self, buffer: &mut TextBuffer, input: &str) -> bool {
        if let Some(PendingCommand::SubstituteConfirm {
            pattern,
            replacement,
            mut matches,
            match_index,
            flags,
        }) = self.vim_state.pending_command.take() {
            match input {
                "y" => {
                    let m = matches[match_index];
                    let match_len = m.end - m.start;
                    let chars: Vec<char> = buffer.content.chars().collect();
                    let prefix: String = chars[..m.start].iter().collect();
                    let suffix: String = chars[m.end..].iter().collect();
                    buffer.content = format!("{}{}{}", prefix, replacement, suffix);
                    buffer.dirty = true;

                    let shift = replacement.chars().count() as isize - match_len as isize;
                    for rem in matches.iter_mut().skip(match_index + 1) {
                        rem.start = (rem.start as isize + shift) as usize;
                        rem.end = (rem.end as isize + shift) as usize;
                    }

                    let next_index = match_index + 1;
                    if next_index < matches.len() {
                        let next_match = matches[next_index];
                        self.yank_highlight = Some(next_match);
                        self.set_cursor_from_flat(buffer, next_match.start);
                        self.vim_state.pending_command = Some(PendingCommand::SubstituteConfirm {
                            pattern,
                            replacement,
                            matches,
                            match_index: next_index,
                            flags,
                        });
                    } else {
                        self.yank_highlight = None;
                        buffer.search.pattern = pattern;
                        self.refresh_search_matches(buffer, );
                    }
                    true
                }
                "n" => {
                    let next_index = match_index + 1;
                    if next_index < matches.len() {
                        let next_match = matches[next_index];
                        self.yank_highlight = Some(next_match);
                        self.set_cursor_from_flat(buffer, next_match.start);
                        self.vim_state.pending_command = Some(PendingCommand::SubstituteConfirm {
                            pattern,
                            replacement,
                            matches,
                            match_index: next_index,
                            flags,
                        });
                    } else {
                        self.yank_highlight = None;
                        buffer.search.pattern = pattern;
                        self.refresh_search_matches(buffer, );
                    }
                    false
                }
                "a" => {
                    let mut content_chars: Vec<char> = buffer.content.chars().collect();
                    for idx in (match_index..matches.len()).rev() {
                        let m = matches[idx];
                        let prefix: Vec<char> = content_chars[..m.start].to_vec();
                        let suffix: Vec<char> = content_chars[m.end..].to_vec();
                        let replacement_chars: Vec<char> = replacement.chars().collect();
                        content_chars = [prefix, replacement_chars, suffix].concat();
                    }
                    buffer.content = content_chars.into_iter().collect();
                    buffer.dirty = true;
                    self.yank_highlight = None;
                    buffer.search.pattern = pattern;
                    self.refresh_search_matches(buffer, );
                    true
                }
                "l" => {
                    let m = matches[match_index];
                    let chars: Vec<char> = buffer.content.chars().collect();
                    let prefix: String = chars[..m.start].iter().collect();
                    let suffix: String = chars[m.end..].iter().collect();
                    buffer.content = format!("{}{}{}", prefix, replacement, suffix);
                    buffer.dirty = true;
                    self.yank_highlight = None;
                    buffer.search.pattern = pattern;
                    self.refresh_search_matches(buffer, );
                    true
                }
                "q" | "escape" | "ctrl+[" => {
                    self.yank_highlight = None;
                    buffer.search.pattern = pattern;
                    self.refresh_search_matches(buffer, );
                    false
                }
                _ => {
                    self.vim_state.pending_command = Some(PendingCommand::SubstituteConfirm {
                        pattern,
                        replacement,
                        matches,
                        match_index,
                        flags,
                    });
                    false
                }
            }
        } else {
            false
        }
    }

    fn handle_single_key_event(&mut self, buffer: &mut TextBuffer, key: &str) -> bool {
        match self.vim_state.mode {
            VimMode::Command => {
                self.handle_command_mode_input(buffer, key)
            }
            VimMode::Search(dir) => {
                self.handle_search_mode_input(buffer, dir, key)
            }
            VimMode::Visual | VimMode::VisualLine | VimMode::VisualBlock => {
                if key == "ctrl+v" {
                    if self.vim_state.mode == VimMode::VisualBlock {
                        self.vim_state.mode = VimMode::Normal;
                        self.visual_anchor_flat = None;
                    } else {
                        self.vim_state.mode = VimMode::VisualBlock;
                    }
                    return false;
                }
                let mut changed = false;
                for ch in key.chars() {
                    changed |= self.handle_visual_char(buffer, ch);
                }
                changed
            }
            VimMode::Normal => {
                if key == "ctrl+r" { return self.redo(buffer, ); }
                if key == "ctrl+o" { return self.jump_history(buffer, -1); }
                if key == "ctrl+i" { return self.jump_history(buffer, 1); }
                if key == "ctrl+d" { return self.scroll_viewport_half_page(buffer, true); }
                if key == "ctrl+u" { return self.scroll_viewport_half_page(buffer, false); }
                if key == "ctrl+f" { return self.scroll_viewport_full_page(buffer, true); }
                if key == "ctrl+b" { return self.scroll_viewport_full_page(buffer, false); }
                if key == "ctrl+e" { return self.scroll_viewport_line(buffer, true); }
                if key == "ctrl+y" { return self.scroll_viewport_line(buffer, false); }
                if key == "ctrl+v" { self.enter_visual(buffer, VisualKind::Block); return false; }
                if key == "." { return self.repeat_last_change(buffer, ); }
                let mut changed = false;
                for ch in key.chars() {
                    changed |= self.handle_normal_char(buffer, ch);
                }
                if self.vim_state.return_to_insert && self.vim_state.pending_command.is_none() && self.vim_state.mode == VimMode::Normal {
                    self.vim_state.return_to_insert = false;
                    self.vim_state.mode = VimMode::Insert;
                }
                changed
            }
            VimMode::Insert | VimMode::Replace => false,
        }
    }

    pub fn handle_insert_text(&mut self, buffer: &mut TextBuffer, text: &str) {
        self.flush_deferred_action(buffer, );
        self.clear_yank_highlight(buffer, );
        let mode = self.vim_state.mode;
        if (mode != VimMode::Insert && mode != VimMode::Replace) || text.is_empty() {
            return;
        }

        if !self.replaying_change {
            self.vim_state.current_macro.push(text.to_string());
        }

        let insert_str = if text == "tab" {
            " ".repeat(buffer.settings.tabstop)
        } else {
            text.to_string()
        };

        let mut lines = self.lines_vec(buffer, );
        let line_index = self.cursor_line(buffer, ).min(lines.len().saturating_sub(1));
        let col = self.cursor_col.min(char_count(&lines[line_index]));
        
        if mode == VimMode::Replace {
            let chars: Vec<char> = lines[line_index].chars().collect();
            let text_chars: Vec<char> = insert_str.chars().collect();
            let mut new_chars = chars.clone();
            for i in 0..text_chars.len() {
                if col + i < new_chars.len() {
                    new_chars[col + i] = text_chars[i];
                } else {
                    new_chars.push(text_chars[i]);
                }
            }
            lines[line_index] = new_chars.into_iter().collect();
            self.cursor_line = line_index;
            self.cursor_col += insert_str.chars().count();
        } else {
            insert_str_at_char(&mut lines[line_index], col, &insert_str);
            self.cursor_line = line_index;
            self.cursor_col += insert_str.chars().count();
        }
        self.replace_lines_keep_insert(buffer, lines);
    }

    pub fn insert_newline(&mut self, buffer: &mut TextBuffer) {
        self.flush_deferred_action(buffer, );
        self.clear_yank_highlight(buffer, );
        if self.vim_state.mode != VimMode::Insert {
            return;
        }

        let mut lines = self.lines_vec(buffer, );
        let line_index = self.cursor_line(buffer, ).min(lines.len().saturating_sub(1));
        let col = self.cursor_col.min(char_count(&lines[line_index]));
        let tail = split_off_at_char(&mut lines[line_index], col);
        lines.insert(line_index + 1, tail);
        self.cursor_line = line_index + 1;
        self.cursor_col = 0;
        self.replace_lines_keep_insert(buffer, lines);
    }

    pub fn delete_word_insert(&mut self, buffer: &mut TextBuffer) {
        self.flush_deferred_action(buffer, );
        self.clear_yank_highlight(buffer, );
        if self.vim_state.mode != VimMode::Insert { return; }
        
        self.record_undo(buffer);
        if !self.replaying_change {
            self.vim_state.current_macro.push("ctrl+w".to_string());
        }
        
        let mut lines = self.lines_vec(buffer, );
        let line_index = self.cursor_line(buffer, ).min(lines.len().saturating_sub(1));
        let col = self.cursor_col.min(char_count(&lines[line_index]));
        
        if col == 0 {
            if line_index > 0 {
                let prev_len = char_count(&lines[line_index - 1]);
                let current_line = lines.remove(line_index);
                lines[line_index - 1].push_str(&current_line);
                self.cursor_line -= 1;
                self.cursor_col = prev_len;
                self.replace_lines_keep_insert(buffer, lines);
            }
            return;
        }

        let chars: Vec<char> = lines[line_index].chars().collect();
        let mut target_col = col - 1;
        while target_col > 0 && chars[target_col].is_whitespace() {
            target_col -= 1;
        }
        let is_word = chars[target_col].is_alphanumeric() || chars[target_col] == '_';
        while target_col > 0 {
            let ch = chars[target_col - 1];
            let ch_is_word = ch.is_alphanumeric() || ch == '_';
            if ch.is_whitespace() || ch_is_word != is_word {
                break;
            }
            target_col -= 1;
        }
        
        let new_line: String = chars[..target_col].iter().chain(chars[col..].iter()).collect();
        lines[line_index] = new_line;
        self.cursor_col = target_col;
        self.replace_lines_keep_insert(buffer, lines);
    }

    pub fn delete_line_insert(&mut self, buffer: &mut TextBuffer) {
        self.flush_deferred_action(buffer, );
        self.clear_yank_highlight(buffer, );
        if self.vim_state.mode != VimMode::Insert { return; }
        
        self.record_undo(buffer);
        if !self.replaying_change {
            self.vim_state.current_macro.push("ctrl+u".to_string());
        }
        
        let mut lines = self.lines_vec(buffer, );
        let line_index = self.cursor_line(buffer, ).min(lines.len().saturating_sub(1));
        let col = self.cursor_col.min(char_count(&lines[line_index]));
        
        if col == 0 {
            if line_index > 0 {
                let prev_len = char_count(&lines[line_index - 1]);
                let current_line = lines.remove(line_index);
                lines[line_index - 1].push_str(&current_line);
                self.cursor_line -= 1;
                self.cursor_col = prev_len;
                self.replace_lines_keep_insert(buffer, lines);
            }
            return;
        }

        let chars: Vec<char> = lines[line_index].chars().collect();
        let target_col = 0;
        let new_line: String = chars[..target_col].iter().chain(chars[col..].iter()).collect();
        lines[line_index] = new_line;
        self.cursor_col = target_col;
        self.replace_lines_keep_insert(buffer, lines);
    }

    pub fn backspace(&mut self, buffer: &mut TextBuffer) {
        self.flush_deferred_action(buffer, );
        self.clear_yank_highlight(buffer, );
        if self.vim_state.mode != VimMode::Insert {
            return;
        }

        let mut lines = self.lines_vec(buffer, );
        let line_index = self.cursor_line(buffer, ).min(lines.len().saturating_sub(1));
        if self.cursor_col > 0 {
            let col = self.cursor_col.min(char_count(&lines[line_index]));
            remove_char_at(&mut lines[line_index], col.saturating_sub(1));
            self.cursor_col = col.saturating_sub(1);
        } else if line_index > 0 {
            let removed = lines.remove(line_index);
            self.cursor_line = line_index - 1;
            self.cursor_col = char_count(&lines[self.cursor_line]);
            lines[self.cursor_line].push_str(&removed);
        }
        self.replace_lines_keep_insert(buffer, lines);
    }

    pub fn delete_at_cursor(&mut self, buffer: &mut TextBuffer) {
        self.flush_deferred_action(buffer, );
        self.clear_yank_highlight(buffer, );
        if self.vim_state.mode != VimMode::Insert {
            return;
        }

        let mut lines = self.lines_vec(buffer, );
        let line_index = self.cursor_line(buffer, ).min(lines.len().saturating_sub(1));
        let col = self.cursor_col.min(char_count(&lines[line_index]));
        if col < char_count(&lines[line_index]) {
            remove_char_at(&mut lines[line_index], col);
        } else if line_index + 1 < lines.len() {
            let next = lines.remove(line_index + 1);
            lines[line_index].push_str(&next);
        }
        self.replace_lines_keep_insert(buffer, lines);
    }

    pub fn title(&self, buffer: &TextBuffer) -> String { buffer.path.as_ref().map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string()).unwrap_or_else(|| String::from("Untitled")) }

    pub fn path_string(&self, buffer: &TextBuffer) -> Option<String> {
        buffer.path.as_ref().map(|path| path.display().to_string())
    }

    pub fn stats(&self, buffer: &TextBuffer) -> NoteStats {
        let line_count = buffer.content.split('\n').count().max(1);
        let word_count = buffer.content.split_whitespace().count();
        let char_count = buffer.content.chars().count();
        NoteStats {
            line_count,
            word_count,
            char_count,
        }
    }

    fn handle_normal_char(&mut self, buffer: &mut TextBuffer, ch: char) -> bool {
        if ch.is_ascii_digit() && !(ch == '0' && self.count.is_none()) {
            let digit = ch.to_digit(10).unwrap_or_default() as usize;
            self.count = Some(
                self.count
                    .unwrap_or(0)
                    .saturating_mul(10)
                    .saturating_add(digit),
            );
            return false;
        }

        if let Some(pending) = self.vim_state.pending_command.take() {
            if pending == PendingCommand::ZPrefix {
                match ch {
                    'Z' => self.pending_ex_action = Some(ExCommandAction::SaveAndQuit),
                    'Q' => self.pending_ex_action = Some(ExCommandAction::Quit { force: true }),
                    _ => {}
                }
                return false;
            }            if pending == PendingCommand::SmallZPrefix {
                match ch {
                    'z' => return self.cursor_to_center(buffer, ),
                    't' => return self.cursor_to_top(buffer, ),
                    'b' => return self.cursor_to_bottom(buffer, ),
                    'a' | 'c' | 'o' | 'r' | 'm' => {
                        // TODO: Implement folds
                    }
                    _ => {}
                }
                self.vim_state.pending_command = None;
                return false;
            }
            return self.handle_pending_normal_char(buffer, pending, ch);
        }
 
        let explicit_count = self.count.take();
        let count = explicit_count.unwrap_or(1).max(1);
        match ch {
            '"' => {
                self.vim_state.pending_command = Some(PendingCommand::RegisterPrefix);
                false
            }
            ':' => {
                self.begin_command_line(buffer, VimMode::Command, "", false);
                false
            }
            '/' | '?' => {
                self.begin_command_line(buffer, VimMode::Search(if ch == '/' {
                        SearchDirection::Forward
                    } else {
                        SearchDirection::Backward
                    }),
                    "",
                    true,
                );
                false
            }
            'Z' => {
                self.vim_state.pending_command = Some(PendingCommand::ZPrefix);
                false
            }
            'z' => {
                self.vim_state.pending_command = Some(PendingCommand::SmallZPrefix);
                false
            }
            'n' => self.repeat_search(buffer, false),
            'N' => self.repeat_search(buffer, true),
            '*' => self.search_word_under_cursor(buffer, false),
            '#' => self.search_word_under_cursor(buffer, true),
            'm' => {
                self.vim_state.pending_command = Some(PendingCommand::MarkSet);
                false
            }
            '\'' => {
                self.vim_state.pending_command = Some(PendingCommand::MarkJump);
                false
            }
            '`' => {
                self.vim_state.pending_command = Some(PendingCommand::MarkJumpExact);
                false
            }
            'v' => {
                self.enter_visual(buffer, VisualKind::Character);
                false
            }
            'V' => {
                self.enter_visual(buffer, VisualKind::Line);
                false
            }
            'g' => {
                self.vim_state.pending_command = Some(PendingCommand::Goto {
                    count: explicit_count,
                });
                false
            }
            '[' | ']' => {
                self.vim_state.pending_command = Some(PendingCommand::BracketPrefix {
                    is_right: ch == ']',
                    count: explicit_count,
                });
                false
            }
            'u' => self.undo(buffer, ),
            'R' => {
                self.enter_replace_mode(buffer, );
                false
            }
            'i' => {
                self.enter_insert(buffer, );
                false
            }
            'I' => {
                self.cursor_col = self.first_non_blank_col(buffer, );
                self.enter_insert(buffer, );
                false
            }
            'a' => {
                self.cursor_col = (self.cursor_col(buffer, ) + 1).min(self.current_line_char_count(buffer, ));
                self.enter_insert(buffer, );
                false
            }
            'A' => {
                self.cursor_col = self.current_line_char_count(buffer, );
                self.enter_insert(buffer, );
                false
            }
            'o' => {
                self.insert_blank_line(buffer, self.cursor_line(buffer, ) + 1);
                self.enter_insert(buffer, );
                true
            }
            'O' => {
                self.insert_blank_line(buffer, self.cursor_line(buffer, ));
                self.enter_insert(buffer, );
                true
            }
            'j' => {
                self.move_cursor_line(buffer, count as isize);
                false
            }
            'k' => {
                self.move_cursor_line(buffer, -(count as isize));
                false
            }
            '+' => {
                self.move_first_nonblank_on_relative_line(buffer, count as isize);
                false
            }
            '-' => {
                self.move_first_nonblank_on_relative_line(buffer, -(count as isize));
                false
            }
            '_' => {
                self.move_first_nonblank_on_relative_line(buffer, count.saturating_sub(1) as isize);
                false
            }
            'f' | 'F' | 't' | 'T' => {
                self.vim_state.pending_command = Some(PendingCommand::FindChar {
                    is_t: ch == 't' || ch == 'T',
                    is_forward: ch == 'f' || ch == 't',
                    count,
                });
                false
            }
            ';' | ',' => {
                self.repeat_find_char(buffer, ch == ';', count)
            }
            '%' | '{' | '}' | '(' | ')' => {
                self.push_jump(buffer, );
                if let Some(target) = self.extended_motion_flat(buffer, ch, count) {
                    self.set_cursor_from_flat(buffer, target);
                }
                false
            }
            'H' | 'M' | 'L' => {
                self.push_jump(buffer, );
                if let Some(target_line) = self.extended_motion_line(buffer, ch, count) {
                    self.cursor_line = target_line;
                    self.clamp_cursor_normal(buffer);
                }
                false
            }
            'h' => {
                self.move_cursor_col(buffer, -(count as isize));
                false
            }
            'l' => {
                self.move_cursor_col(buffer, count as isize);
                false
            }
            'w' | 'W' => {
                self.move_word_forward(buffer, count, ch == 'W');
                false
            }
            'b' | 'B' => {
                self.move_word_backward(buffer, count, ch == 'B');
                false
            }
            'e' | 'E' => {
                self.move_word_end(buffer, count, ch == 'E');
                false
            }
            '|' => {
                self.cursor_col = explicit_count.unwrap_or(1).saturating_sub(1).min(self.current_line_max_col(buffer, ));
                false
            }
            '0' => {
                self.cursor_col = 0;
                false
            }
            '^' => {
                self.cursor_col = self.first_non_blank_col(buffer, );
                false
            }
            '$' => {
                self.cursor_col = self.current_line_max_col(buffer, );
                false
            }
            'G' => {
                self.push_jump(buffer, );
                self.cursor_line = if let Some(count) = explicit_count {
                    count.saturating_sub(1)
                } else {
                    self.line_count(buffer, ).saturating_sub(1)
                };
                self.clamp_cursor_normal(buffer);
                false
            }
            'd' => {
                self.vim_state.pending_command = Some(PendingCommand::Operator {
                    operator: Operator::Delete,
                    count,
                });
                false
            }
            '!' => {
                self.vim_state.pending_command = Some(PendingCommand::Operator {
                    operator: Operator::Filter,
                    count,
                });
                false
            }
            '\x01' => {
                self.increment_number(buffer, count as isize);
                false
            }
            '\x18' => {
                self.increment_number(buffer, -(count as isize));
                false
            }
            'c' => {
                self.vim_state.pending_command = Some(PendingCommand::Operator {
                    operator: Operator::Change,
                    count,
                });
                false
            }
            'y' => {
                self.vim_state.pending_command = Some(PendingCommand::Operator {
                    operator: Operator::Yank,
                    count,
                });
                false
            }
            'Y' => self.apply_current_lines_operator(buffer, Operator::Yank, count),
            '>' => {
                self.vim_state.pending_command = Some(PendingCommand::Operator {
                    operator: Operator::Indent,
                    count,
                });
                false
            }
            '<' => {
                self.vim_state.pending_command = Some(PendingCommand::Operator {
                    operator: Operator::Outdent,
                    count,
                });
                false
            }
            '=' => {
                self.vim_state.pending_command = Some(PendingCommand::Operator {
                    operator: Operator::Format,
                    count,
                });
                false
            }
            'p' => self.paste_unnamed(buffer, count, PastePlacement::After),
            'P' => self.paste_unnamed(buffer, count, PastePlacement::Before),
            'x' => self.delete_chars_on_current_line(buffer, count),
            'X' => self.delete_chars_before_cursor(buffer, count),
            'r' => {
                self.vim_state.pending_command = Some(PendingCommand::ReplaceChar { count });
                false
            }
            's' => self.substitute_chars(buffer, count),
            'S' => self.apply_current_lines_operator(buffer, Operator::Change, count),
            'D' => self.apply_operator_motion(buffer, Operator::Delete, count, '$'),
            'C' => self.apply_operator_motion(buffer, Operator::Change, count, '$'),
            'J' => self.join_lines(buffer, count, true),
            'q' => {
                if self.vim_state.recording_macro.is_some() {
                    self.stop_macro_recording(buffer, );
                } else {
                    self.vim_state.pending_command = Some(PendingCommand::MacroRecordPrefix);
                }
                false
            }
            '@' => {
                self.vim_state.pending_command = Some(PendingCommand::MacroReplayPrefix { count });
                false
            }
            '~' => self.toggle_case_chars(buffer, count),
            _ => false,
        }
    }

    fn handle_pending_normal_char(&mut self, buffer: &mut TextBuffer, pending: PendingCommand, ch: char) -> bool {
        match (pending, ch) {
            (PendingCommand::RegisterPrefix, register) => {
                self.vim_state.pending_command = None;
                self.selected_register = RegisterTarget::from_prefix(register);
                false
            }
            (PendingCommand::MarkSet, mark) if mark.is_ascii_alphabetic() => {
                let pos = self.cursor_position(buffer);
                buffer.marks.insert(mark.to_ascii_lowercase(), pos);
                false
            }
            (PendingCommand::MarkJump, mark) if mark.is_ascii_alphabetic() => {
                let Some(position) = buffer.marks.get(&mark.to_ascii_lowercase()).copied() else {
                    return false;
                };
                self.push_jump(buffer, );
                self.cursor_line = position.line;
                self.cursor_col = self.first_non_blank_col_for_line(buffer, position.line);
                self.clamp_cursor_normal(buffer);
                false
            }
            (PendingCommand::MarkJumpExact, mark) if mark.is_ascii_alphabetic() => {
                let Some(position) = buffer.marks.get(&mark.to_ascii_lowercase()).copied() else {
                    return false;
                };
                self.push_jump(buffer, );
                self.cursor_line = position.line;
                self.cursor_col = position.col;
                self.clamp_cursor_normal(buffer);
                false
            }
            (PendingCommand::ReplaceChar { count }, replacement) => {
                if matches!(self.vim_state.mode, VimMode::Visual | VimMode::VisualLine | VimMode::VisualBlock) {
                    self.replace_visual_selection(buffer, replacement)
                } else {
                    self.replace_chars(buffer, replacement, count)
                }
            }
            (
                PendingCommand::Operator {
                    operator: Operator::Delete,
                    count,
                },
                'd',
            ) => {
                self.vim_state.pending_command = None;
                let multiplier = self.count.take().unwrap_or(1).max(1);
                self.apply_current_lines_operator(buffer, Operator::Delete, count.saturating_mul(multiplier))
            }
            (
                PendingCommand::Operator {
                    operator: Operator::Delete,
                    count: _,
                },
                's',
            ) => {
                self.vim_state.pending_command = Some(PendingCommand::SurroundWaitDelete {
                    buffer: String::new(),
                });
                false
            }
            (
                PendingCommand::Operator {
                    operator: Operator::Change,
                    count,
                },
                'c',
            ) => {
                self.vim_state.pending_command = None;
                let multiplier = self.count.take().unwrap_or(1).max(1);
                self.apply_current_lines_operator(buffer, Operator::Change, count.saturating_mul(multiplier))
            }
            (
                PendingCommand::Operator {
                    operator: Operator::Change,
                    count: _,
                },
                's',
            ) => {
                self.vim_state.pending_command = Some(PendingCommand::SurroundWaitChange {
                    buffer: String::new(),
                });
                false
            }
            (
                PendingCommand::Operator {
                    operator: Operator::Yank,
                    count,
                },
                'y',
            ) => {
                self.vim_state.pending_command = None;
                let multiplier = self.count.take().unwrap_or(1).max(1);
                self.apply_current_lines_operator(buffer, Operator::Yank, count.saturating_mul(multiplier))
            }
            (
                PendingCommand::Operator {
                    operator: Operator::Yank,
                    count,
                },
                's',
            ) => {
                self.vim_state.pending_command = Some(PendingCommand::Operator {
                    operator: Operator::SurroundAdd,
                    count,
                });
                false
            }
            (
                PendingCommand::Operator {
                    operator: Operator::Indent,
                    count,
                },
                '>',
            ) => {
                self.vim_state.pending_command = None;
                let multiplier = self.count.take().unwrap_or(1).max(1);
                self.apply_current_lines_operator(buffer, Operator::Indent, count.saturating_mul(multiplier))
            }
            (
                PendingCommand::Operator {
                    operator: Operator::Outdent,
                    count,
                },
                '<',
            ) => {
                self.vim_state.pending_command = None;
                let multiplier = self.count.take().unwrap_or(1).max(1);
                self.apply_current_lines_operator(buffer, Operator::Outdent, count.saturating_mul(multiplier))
            }
            (
                PendingCommand::Operator {
                    operator: Operator::Format,
                    count,
                },
                '=',
            ) => {
                self.vim_state.pending_command = None;
                let multiplier = self.count.take().unwrap_or(1).max(1);
                self.apply_current_lines_operator(buffer, Operator::Format, count.saturating_mul(multiplier))
            }
            (
                PendingCommand::Operator {
                    operator: Operator::Change,
                    count,
                },
                'w',
            ) => {
                self.vim_state.pending_command = None;
                self.count = None;
                self.apply_operator_motion(buffer, Operator::Change, count, 'w')
            }
            (PendingCommand::Operator { operator, count }, 'i') => {
                self.vim_state.pending_command = Some(PendingCommand::TextObject {
                    operator,
                    count,
                    around: false,
                });
                self.count = None;
                false
            }
            (PendingCommand::Operator { operator, count }, 'a') => {
                self.vim_state.pending_command = Some(PendingCommand::TextObject {
                    operator,
                    count,
                    around: true,
                });
                self.count = None;
                false
            }
            (PendingCommand::Operator { operator, count }, 'g') => {
                self.vim_state.pending_command = Some(PendingCommand::OperatorOrCaseGoto { operator, count });
                false
            }
            (PendingCommand::Goto { count }, 'u') => {
                self.vim_state.pending_command = Some(PendingCommand::CaseOperator {
                    kind: CaseKind::Lower,
                    count: count.unwrap_or(1),
                });
                false
            }
            (PendingCommand::Goto { count }, 'U') => {
                self.vim_state.pending_command = Some(PendingCommand::CaseOperator {
                    kind: CaseKind::Upper,
                    count: count.unwrap_or(1),
                });
                false
            }
            (PendingCommand::Goto { count }, '~') => {
                self.vim_state.pending_command = Some(PendingCommand::CaseOperator {
                    kind: CaseKind::Toggle,
                    count: count.unwrap_or(1),
                });
                false
            }
            (
                PendingCommand::Operator { operator, count },
                motion @ ('h' | 'j' | 'k' | 'l' | 'w' | 'W' | 'b' | 'B' | 'e' | 'E' | '0' | '^'
                | '$' | 'G' | ';' | ',' | '%' | '{' | '}' | '(' | ')' | 'H' | 'M' | 'L'),
            ) => {
                let motion_count = self.count.take().unwrap_or(1).max(1);
                self.vim_state.pending_command = None;
                self.apply_operator_motion(buffer, operator, count.saturating_mul(motion_count), motion)
            }
            (
                PendingCommand::Operator { operator, count },
                motion @ ('f' | 'F' | 't' | 'T'),
            ) => {
                let find_count = self.count.take().unwrap_or(1).max(1);
                self.vim_state.pending_command = Some(PendingCommand::OperatorThenFindChar {
                    operator,
                    operator_count: count,
                    is_t: motion == 't' || motion == 'T',
                    is_forward: motion == 'f' || motion == 't',
                    find_count,
                });
                false
            }
            (PendingCommand::FindChar { is_t, is_forward, count }, find_ch) => {
                self.vim_state.pending_command = None;
                self.execute_find_char(buffer, find_ch, is_t, is_forward, count)
            }
            (PendingCommand::OperatorThenFindChar { operator, operator_count, is_t, is_forward, find_count }, find_ch) => {
                self.vim_state.pending_command = None;
                self.execute_operator_find_char(buffer, operator, operator_count, find_ch, is_t, is_forward, find_count)
            }
            (PendingCommand::SurroundWaitAdd { range, buffer: mut cmd_buffer }, ch) => {
                cmd_buffer.push(ch);
                match crate::vim::surround::SurroundSpec::parse(&cmd_buffer) {
                    crate::vim::surround::ParseResult::Complete(spec) => {
                        self.vim_state.pending_command = None;
                        self.apply_surround_add(buffer, range, spec);
                        true
                    }
                    crate::vim::surround::ParseResult::Incomplete => {
                        self.vim_state.pending_command = Some(PendingCommand::SurroundWaitAdd { range, buffer: cmd_buffer });
                        false
                    }
                    crate::vim::surround::ParseResult::Invalid => {
                        self.vim_state.pending_command = None;
                        false
                    }
                }
            }
            (PendingCommand::SurroundWaitDelete { buffer: mut cmd_buffer }, ch) => {
                cmd_buffer.push(ch);
                match crate::vim::surround::SurroundSpec::parse(&cmd_buffer) {
                    crate::vim::surround::ParseResult::Complete(spec) => {
                        self.vim_state.pending_command = None;
                        self.apply_surround_delete(buffer, spec);
                        true
                    }
                    crate::vim::surround::ParseResult::Incomplete => {
                        self.vim_state.pending_command = Some(PendingCommand::SurroundWaitDelete { buffer: cmd_buffer });
                        false
                    }
                    crate::vim::surround::ParseResult::Invalid => {
                        self.vim_state.pending_command = None;
                        false
                    }
                }
            }
            (PendingCommand::SurroundWaitChange { buffer: mut cmd_buffer }, ch) => {
                cmd_buffer.push(ch);
                
                // For change, we need TWO specs: the old one to delete, and the new one to add!
                // Let's implement a simple parser for two specs, or just wait.
                // Wait! The user types `cs"'` which means `cmd_buffer` will contain `"'`.
                // We could parse it by attempting to parse a prefix, then if complete, parse the rest!
                // To keep it simple, `cs` is always two single characters, or `cst<span>` (char then tag).
                // Let's do this: we first parse the `old` spec. If incomplete, we wait.
                // If complete, we check if there's more text. If there is, we parse the `new` spec.
                // If the new spec is complete, we apply it. If incomplete, we wait.
                
                let mut parsed_old = None;
                let mut old_len = 0;
                
                // Find the boundary
                let boundaries = cmd_buffer
                    .char_indices()
                    .map(|(index, _)| index)
                    .skip(1)
                    .chain(std::iter::once(cmd_buffer.len()));
                for i in boundaries {
                    let old_part = &cmd_buffer[0..i];
                    match crate::vim::surround::SurroundSpec::parse(old_part) {
                        crate::vim::surround::ParseResult::Complete(spec) => {
                            parsed_old = Some(spec);
                            old_len = i;
                            break;
                        }
                        crate::vim::surround::ParseResult::Invalid => break,
                        crate::vim::surround::ParseResult::Incomplete => continue,
                    }
                }
                
                if let Some(old_spec) = parsed_old {
                    let new_part = &cmd_buffer[old_len..];
                    match crate::vim::surround::SurroundSpec::parse(new_part) {
                        crate::vim::surround::ParseResult::Complete(new_spec) => {
                            self.vim_state.pending_command = None;
                            self.apply_surround_change(buffer, old_spec, new_spec);
                            return true;
                        }
                        crate::vim::surround::ParseResult::Incomplete => {
                            self.vim_state.pending_command = Some(PendingCommand::SurroundWaitChange { buffer: cmd_buffer });
                            return false;
                        }
                        crate::vim::surround::ParseResult::Invalid => {
                            self.vim_state.pending_command = None;
                            return false;
                        }
                    }
                } else {
                    // Still parsing old spec
                    match crate::vim::surround::SurroundSpec::parse(&cmd_buffer) {
                        crate::vim::surround::ParseResult::Complete(_) => unreachable!(),
                        crate::vim::surround::ParseResult::Incomplete => {
                            self.vim_state.pending_command = Some(PendingCommand::SurroundWaitChange { buffer: cmd_buffer });
                            return false;
                        }
                        crate::vim::surround::ParseResult::Invalid => {
                            self.vim_state.pending_command = None;
                            return false;
                        }
                    }
                }
            }
            (
                PendingCommand::TextObject {
                    operator,
                    count,
                    around,
                },
                object @ ('w' | 'W'),
            ) => {
                let motion_count = self.count.take().unwrap_or(1).max(1);
                self.vim_state.pending_command = None;
                self.apply_text_object_operator(buffer, operator,
                    count.saturating_mul(motion_count),
                    around,
                    TextObject::Word {
                        big_word: object == 'W',
                    },
                )
            }
            (
                PendingCommand::TextObject {
                    operator,
                    count,
                    around,
                },
                object @ ('\'' | '"' | '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>'),
            ) => {
                let motion_count = self.count.take().unwrap_or(1).max(1);
                self.vim_state.pending_command = None;
                self.apply_text_object_operator(buffer, operator,
                    count.saturating_mul(motion_count),
                    around,
                    TextObject::Delimited(object),
                )
            }
            (
                PendingCommand::TextObject {
                    operator,
                    count,
                    around,
                },
                object @ ('p' | 'l'),
            ) => {
                let motion_count = self.count.take().unwrap_or(1).max(1);
                self.vim_state.pending_command = None;
                let object = if object == 'p' {
                    TextObject::Paragraph
                } else {
                    TextObject::Line
                };
                self.apply_text_object_operator(buffer, operator,
                    count.saturating_mul(motion_count),
                    around,
                    object,
                )
            }
            (
                PendingCommand::VisualTextObject { around },
                object @ ('w' | 'W'),
            ) => {
                let motion_count = self.count.take().unwrap_or(1).max(1);
                self.vim_state.pending_command = None;
                self.apply_visual_text_object(buffer, motion_count,
                    around,
                    TextObject::Word {
                        big_word: object == 'W',
                    },
                )
            }
            (
                PendingCommand::VisualTextObject { around },
                object @ ('\'' | '"' | '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>'),
            ) => {
                let motion_count = self.count.take().unwrap_or(1).max(1);
                self.vim_state.pending_command = None;
                self.apply_visual_text_object(buffer, motion_count,
                    around,
                    TextObject::Delimited(object),
                )
            }
            (
                PendingCommand::VisualTextObject { around },
                object @ ('p' | 'l'),
            ) => {
                let motion_count = self.count.take().unwrap_or(1).max(1);
                self.vim_state.pending_command = None;
                let object = if object == 'p' {
                    TextObject::Paragraph
                } else {
                    TextObject::Line
                };
                self.apply_visual_text_object(buffer, motion_count,
                    around,
                    object,
                )
            }

            (PendingCommand::Goto { count }, 'w') => {
                self.vim_state.pending_command = Some(PendingCommand::Operator {
                    operator: Operator::FormatLeaveCursor,
                    count: count.unwrap_or(1).max(1),
                });
                false
            }
            (PendingCommand::Goto { count }, 'q') => {
                self.vim_state.pending_command = Some(PendingCommand::Operator {
                    operator: Operator::Format,
                    count: count.unwrap_or(1).max(1),
                });
                false
            }
            (PendingCommand::Goto { count }, 'g') => {
                self.push_jump(buffer, );
                self.vim_state.pending_command = None;
                self.cursor_line = count
                    .or_else(|| self.count.take())
                    .unwrap_or(1)
                    .saturating_sub(1);
                self.clamp_cursor_normal(buffer);
                false
            }
            (PendingCommand::Goto { count }, 'j') => {
                self.vim_state.pending_command = None;
                let steps = count.or(self.count.take()).unwrap_or(1) as isize;
                self.move_cursor_line(buffer, steps);
                false
            }
            (PendingCommand::Goto { count }, 'k') => {
                self.vim_state.pending_command = None;
                let steps = count.or(self.count.take()).unwrap_or(1) as isize;
                self.move_cursor_line(buffer, -steps);
                false
            }
            (PendingCommand::Goto { count }, '_') => {
                self.vim_state.pending_command = None;
                let steps = count.or(self.count.take()).unwrap_or(1).saturating_sub(1) as isize;
                self.move_cursor_line(buffer, steps);
                let lines = self.lines_vec(buffer);
                if self.cursor_line < lines.len() {
                    let chars: Vec<char> = lines[self.cursor_line].chars().collect();
                    let mut col = chars.len().saturating_sub(1);
                    while col > 0 && chars[col].is_whitespace() {
                        col -= 1;
                    }
                    self.cursor_col = col;
                }
                self.clamp_cursor_normal(buffer);
                false
            }
            (PendingCommand::BracketPrefix { is_right, count }, ch) => {
                self.vim_state.pending_command = None;
                let steps = count.or(self.count.take()).unwrap_or(1).max(1);
                
                if (is_right && ch == ']') || (!is_right && ch == '[') {
                    self.jump_section(buffer, is_right, '{', steps);
                } else if (is_right && ch == '[') || (!is_right && ch == ']') {
                    self.jump_section(buffer, is_right, '}', steps);
                } else if is_right && ch == 'p' {
                    self.paste_unnamed(buffer, steps, PastePlacement::After);
                } else if !is_right && ch == 'p' {
                    self.paste_unnamed(buffer, steps, PastePlacement::Before);
                }
                false
            }
            (PendingCommand::Goto { count }, 'p') => {
                self.vim_state.pending_command = None;
                let steps = count.or(self.count.take()).unwrap_or(1).max(1);
                self.paste_unnamed(buffer, steps, PastePlacement::AfterLeaveCursor);
                false
            }
            (PendingCommand::Goto { count }, 'P') => {
                self.vim_state.pending_command = None;
                let steps = count.or(self.count.take()).unwrap_or(1).max(1);
                self.paste_unnamed(buffer, steps, PastePlacement::BeforeLeaveCursor);
                false
            }
            (PendingCommand::Goto { count: _ }, 'v') => {
                self.vim_state.pending_command = None;
                self.restore_last_visual_selection(buffer, )
            }
            (PendingCommand::Goto { count }, ';') => {
                self.vim_state.pending_command = None;
                let steps = count.or(self.count.take()).unwrap_or(1) as isize;
                self.navigate_changelist(buffer, -steps)
            }
            (PendingCommand::Goto { count }, ',') => {
                self.vim_state.pending_command = None;
                let steps = count.or(self.count.take()).unwrap_or(1) as isize;
                self.navigate_changelist(buffer, steps)
            }
            (PendingCommand::Goto { count: _ }, 'i') => {
                self.vim_state.pending_command = None;
                if let Some(pos) = self.vim_state.last_insert_pos {
                    self.set_cursor_from_flat(buffer, pos);
                    self.vim_state.mode = VimMode::Insert;
                }
                false
            }
            (PendingCommand::Goto { count }, 'e') => {
                self.vim_state.pending_command = None;
                let steps = count.or(self.count.take()).unwrap_or(1).max(1);
                self.move_word_backward_end(buffer, steps, false);
                false
            }
            (PendingCommand::Goto { count }, 'E') => {
                self.vim_state.pending_command = None;
                let steps = count.or(self.count.take()).unwrap_or(1).max(1);
                self.move_word_backward_end(buffer, steps, true);
                false
            }
            (PendingCommand::Goto { count }, 'J') => {
                self.vim_state.pending_command = None;
                let steps = count.or(self.count.take()).unwrap_or(1).max(1);
                self.join_lines(buffer, steps, false)
            }
            (PendingCommand::MacroRecordPrefix, register) if register.is_ascii_alphabetic() => {
                self.vim_state.pending_command = None;
                self.start_macro_recording(buffer, register.to_ascii_lowercase());
                false
            }
            (PendingCommand::MacroRecordPrefix, 'q') => {
                self.vim_state.pending_command = None;
                self.stop_macro_recording(buffer, );
                false
            }
            (PendingCommand::MacroReplayPrefix { count }, register) if register.is_ascii_alphabetic() => {
                self.vim_state.pending_command = None;
                self.play_macro(buffer, register.to_ascii_lowercase(), count)
            }
            (PendingCommand::MacroReplayPrefix { count }, '@') => {
                self.vim_state.pending_command = None;
                if let Some(last) = self.last_played_macro(buffer, ) {
                    self.play_macro(buffer, last, count)
                } else {
                    false
                }
            }
            (PendingCommand::OperatorOrCaseGoto { operator, count: _ }, 'g') => {
                let target_line = self.count.take().unwrap_or(1).saturating_sub(1);
                self.vim_state.pending_command = None;
                self.apply_operator_range(buffer, operator,
                    self.linewise_range(buffer, self.cursor_line(buffer, ), target_line),
                )
            }
            (
                PendingCommand::CaseOperator { kind, count },
                motion @ ('h' | 'j' | 'k' | 'l' | 'w' | 'W' | 'b' | 'B' | 'e' | 'E' | '0' | '^'
                | '$' | 'G'),
            ) => {
                let motion_count = self.count.take().unwrap_or(1).max(1);
                self.vim_state.pending_command = None;
                let Some(range) =
                    self.operator_motion_range(buffer, count.saturating_mul(motion_count), motion)
                else {
                    return false;
                };
                self.apply_case_range_kind(buffer, range, kind)
            }
            (PendingCommand::CaseOperator { kind, count }, ch)
                if (kind == CaseKind::Lower && ch == 'u')
                    || (kind == CaseKind::Upper && ch == 'U')
                    || (kind == CaseKind::Toggle && ch == '~') => {
                self.vim_state.pending_command = None;
                let multiplier = self.count.take().unwrap_or(1).max(1);
                let total_count = count.saturating_mul(multiplier);
                let target_line = self.cursor_line(buffer, ).saturating_add(total_count.saturating_sub(1));
                let range = self.linewise_range(buffer, self.cursor_line(buffer, ), target_line);
                self.apply_case_range_kind(buffer, range, kind)
            }
            _ => {
                self.vim_state.pending_command = None;
                self.count = None;
                self.selected_register = None;
                false
            }
        }
    }

    fn handle_visual_char(&mut self, buffer: &mut TextBuffer, ch: char) -> bool {
        if ch.is_ascii_digit() && !(ch == '0' && self.count.is_none()) {
            let digit = ch.to_digit(10).unwrap_or_default() as usize;
            self.count = Some(
                self.count
                    .unwrap_or(0)
                    .saturating_mul(10)
                    .saturating_add(digit),
            );
            return false;
        }

        if let Some(pending) = self.vim_state.pending_command.take() {
            return self.handle_pending_normal_char(buffer, pending, ch);
        }

        let explicit_count = self.count.take();
        let count = explicit_count.unwrap_or(1).max(1);
        match ch {
            'v' if self.vim_state.mode == VimMode::Visual => {
                self.enter_normal(buffer, );
                false
            }
            'V' if self.vim_state.mode == VimMode::VisualLine => {
                self.enter_normal(buffer, );
                false
            }
            'f' | 'F' | 't' | 'T' => {
                self.vim_state.pending_command = Some(PendingCommand::FindChar {
                    is_t: ch == 't' || ch == 'T',
                    is_forward: ch == 'f' || ch == 't',
                    count,
                });
                false
            }
            ';' | ',' => {
                self.repeat_find_char(buffer, ch == ';', count)
            }
            'r' => {
                self.vim_state.pending_command = Some(PendingCommand::ReplaceChar { count });
                false
            }
            'R' => {
                self.vim_state.mode = VimMode::Replace;
                self.vim_state.insert_start_pos = Some(self.flattened_cursor(buffer, ));
                self.vim_state.macro_insert_start_index = Some(self.vim_state.current_macro.len());
                false
            }
            'v' => {
                self.vim_state.mode = VimMode::Visual;
                false
            }
            'V' => {
                self.vim_state.mode = VimMode::VisualLine;
                false
            }
            'i' => {
                self.vim_state.pending_command = Some(PendingCommand::VisualTextObject { around: false });
                false
            }
            'a' => {
                self.vim_state.pending_command = Some(PendingCommand::VisualTextObject { around: true });
                false
            }
            'o' => {
                let Some(anchor) = self.visual_anchor_flat else {
                    return false;
                };
                let cursor = self.flattened_cursor(buffer, );
                self.visual_anchor_flat = Some(cursor);
                self.set_cursor_from_flat(buffer, anchor);
                false
            }
            'h' => {
                self.move_cursor_col(buffer, -(count as isize));
                false
            }
            'j' => {
                self.move_cursor_line(buffer, count as isize);
                false
            }
            'k' => {
                self.move_cursor_line(buffer, -(count as isize));
                false
            }
            'l' => {
                self.move_cursor_col(buffer, count as isize);
                false
            }
            'w' | 'W' => {
                self.move_word_forward(buffer, count, ch == 'W');
                false
            }
            'b' | 'B' => {
                self.move_word_backward(buffer, count, ch == 'B');
                false
            }
            'e' | 'E' => {
                self.move_word_end(buffer, count, ch == 'E');
                false
            }
            '0' => {
                self.cursor_col = 0;
                false
            }
            '+' => {
                self.move_first_nonblank_on_relative_line(buffer, count as isize);
                false
            }
            '-' => {
                self.move_first_nonblank_on_relative_line(buffer, -(count as isize));
                false
            }
            '_' => {
                self.move_first_nonblank_on_relative_line(buffer, count.saturating_sub(1) as isize);
                false
            }
            '|' => {
                self.cursor_col = count.saturating_sub(1).min(self.current_line_max_col(buffer, ));
                false
            }
            '^' => {
                self.cursor_col = self.first_non_blank_col(buffer, );
                false
            }
            '$' => {
                self.cursor_col = self.current_line_max_col(buffer, );
                false
            }
            'G' => {
                self.push_jump(buffer, );
                self.cursor_line = if let Some(explicit) = explicit_count {
                    explicit.saturating_sub(1)
                } else {
                    self.line_count(buffer, ).saturating_sub(1)
                };
                self.clamp_cursor_normal(buffer);
                false
            }
            'g' => {
                self.vim_state.pending_command = Some(PendingCommand::Goto { count: explicit_count });
                false
            }
            '%' | '{' | '}' | '(' | ')' => {
                self.push_jump(buffer, );
                if let Some(target) = self.extended_motion_flat(buffer, ch, count) {
                    self.set_cursor_from_flat(buffer, target);
                }
                false
            }
            'H' | 'M' | 'L' => {
                self.push_jump(buffer, );
                if let Some(target_line) = self.extended_motion_line(buffer, ch, count) {
                    self.cursor_line = target_line;
                    self.clamp_cursor_normal(buffer);
                }
                false
            }
            'I' if self.vim_state.mode == VimMode::VisualBlock => {
                let Some(range) = self.visual_selection_range(buffer, ) else { return false; };
                self.visual_anchor_flat = None;
                self.vim_state.mode = VimMode::Normal;
                
                let start_line = self.line_for_flat(buffer, range.start);
                let end_line = self.line_for_flat(buffer, range.end.saturating_sub(1));
                let top_line = start_line.min(end_line);
                let start_col = {
                    let line_start = self.line_start_flat(buffer, self.line_for_flat(buffer, range.start));
                    range.start.saturating_sub(line_start)
                };
                let end_col = {
                    let line_start = self.line_start_flat(buffer, self.line_for_flat(buffer, range.end.saturating_sub(1)));
                    range.end.saturating_sub(1).saturating_sub(line_start)
                };
                let left_col = start_col.min(end_col);
                
                self.cursor_line = top_line;
                self.cursor_col = left_col;
                self.deferred_action = Some(DeferredAction { operator: Operator::BlockInsert, range });
                self.enter_insert(buffer, );
                false
            }
            'A' if self.vim_state.mode == VimMode::VisualBlock => {
                let Some(range) = self.visual_selection_range(buffer, ) else { return false; };
                self.visual_anchor_flat = None;
                self.vim_state.mode = VimMode::Normal;
                
                let start_line = self.line_for_flat(buffer, range.start);
                let end_line = self.line_for_flat(buffer, range.end.saturating_sub(1));
                let top_line = start_line.min(end_line);
                let start_col = {
                    let line_start = self.line_start_flat(buffer, self.line_for_flat(buffer, range.start));
                    range.start.saturating_sub(line_start)
                };
                let end_col = {
                    let line_start = self.line_start_flat(buffer, self.line_for_flat(buffer, range.end.saturating_sub(1)));
                    range.end.saturating_sub(1).saturating_sub(line_start)
                };
                let right_col = start_col.max(end_col);
                
                self.cursor_line = top_line;
                self.cursor_col = right_col; // wait, cursor_col needs to be one after?
                self.cursor_col = (right_col + 1).min(self.current_line_char_count(buffer, ));
                self.deferred_action = Some(DeferredAction { operator: Operator::BlockAppend, range });
                self.enter_insert(buffer, );
                false
            }
            'd' | 'x' => self.apply_visual_operator(buffer, Operator::Delete),
            'c' => self.apply_visual_operator(buffer, Operator::Change),
            's' | 'S' => self.apply_visual_operator(buffer, Operator::SurroundAdd),
            'y' => self.apply_visual_operator(buffer, Operator::Yank),
            '>' => self.apply_visual_operator(buffer, Operator::Indent),
            '<' => self.apply_visual_operator(buffer, Operator::Outdent),
            '=' => self.apply_visual_operator(buffer, Operator::Format),
            'J' => {
                if let Some(range) = self.visual_selection_range(buffer, ) {
                    self.enter_normal(buffer, );
                    let count = self.line_for_flat(buffer, range.end.saturating_sub(1)) - self.line_for_flat(buffer, range.start);
                    self.set_cursor_from_flat(buffer, range.start);
                    self.join_lines(buffer, count, true)
                } else {
                    false
                }
            }
            '~' => {
                let Some(range) = self.visual_selection_range(buffer, ) else {
                    return false;
                };
                self.last_visual_selection = Some((range, self.visual_kind(buffer, )));
                self.visual_anchor_flat = None;
                self.vim_state.mode = VimMode::Normal;
                self.toggle_case_range(buffer, range)
            }
            'u' => {
                let Some(range) = self.visual_selection_range(buffer, ) else {
                    return false;
                };
                self.last_visual_selection = Some((range, self.visual_kind(buffer, )));
                self.visual_anchor_flat = None;
                self.vim_state.mode = VimMode::Normal;
                self.apply_case_range(buffer, range, false)
            }
            'U' => {
                let Some(range) = self.visual_selection_range(buffer, ) else {
                    return false;
                };
                self.last_visual_selection = Some((range, self.visual_kind(buffer, )));
                self.visual_anchor_flat = None;
                self.vim_state.mode = VimMode::Normal;
                self.apply_case_range(buffer, range, true)
            }
            ':' => {
                let range = self.visual_selection_range(buffer, );
                let kind = self.visual_kind(buffer, );
                if let Some(r) = range {
                    self.last_visual_selection = Some((r, kind));
                }
                self.begin_command_line(buffer, VimMode::Command, "'<,'>", false);
                false
            }
            '/' | '?' => {
                self.begin_command_line(buffer, VimMode::Search(if ch == '/' {
                        SearchDirection::Forward
                    } else {
                        SearchDirection::Backward
                    }),
                    "",
                    true,
                );
                false
            }
            _ => false,
        }
    }

    fn line_count(&self, buffer: &TextBuffer) -> usize {
        buffer.content.split('\n').count().max(1)
    }

    fn clamp_cursor_line(&mut self, buffer: &mut TextBuffer) {
        self.cursor_line = self.cursor_line(buffer, );
        self.cursor_col = self.cursor_col(buffer, );
    }

    pub fn clamp_cursor_normal(&mut self, buffer: &mut TextBuffer) {
        self.cursor_line = self.cursor_line(buffer, );
        self.cursor_col = self.cursor_col.min(self.current_line_max_col(buffer, ));
    }

    fn move_cursor_line(&mut self, buffer: &mut TextBuffer, delta: isize) {
        let current = self.cursor_line(buffer) as isize;
        let max = self.line_count(buffer, ).saturating_sub(1) as isize;
        self.cursor_line = (current + delta).clamp(0, max) as usize;
        self.cursor_col = self.cursor_col.min(self.current_line_max_col(buffer, ));
    }

    fn move_cursor_col(&mut self, buffer: &mut TextBuffer, delta: isize) {
        let current = self.cursor_col(buffer) as isize;
        let max = self.current_line_max_col(buffer, ) as isize;
        self.cursor_col = (current + delta).clamp(0, max) as usize;
    }

    fn current_line_max_col(&self, buffer: &TextBuffer) -> usize {
        self.current_line_char_count(buffer, ).saturating_sub(1)
    }

    fn current_line_char_count(&self, buffer: &TextBuffer) -> usize {
        self.lines_vec(buffer, )
            .get(self.cursor_line(buffer, ))
            .map(|line| line.chars().count())
            .unwrap_or(0)
    }

    fn lines_vec(&self, buffer: &TextBuffer) -> Vec<String> {
        if buffer.content.is_empty() {
            vec![String::new()]
        } else {
            buffer.content.split('\n').map(ToOwned::to_owned).collect()
        }
    }

    fn replace_lines(&mut self, buffer: &mut TextBuffer, lines: Vec<String>) {
        self.record_undo(buffer);
        self.push_change_location(buffer);
        buffer.content = lines.join("\n");
        buffer.dirty = true;
        self.clamp_cursor_line(buffer, );
    }

    fn replace_lines_keep_insert(&mut self, buffer: &mut TextBuffer, lines: Vec<String>) {
        self.push_change_location(buffer);
        buffer.content = lines.join("\n");
        buffer.dirty = true;
        self.cursor_line = self.cursor_line.min(self.line_count(buffer, ).saturating_sub(1));
        self.cursor_col = self.cursor_col.min(self.current_line_char_count(buffer, ));
    }

    fn snapshot_document(&self, buffer: &mut TextBuffer) -> DocumentSnapshot {
        DocumentSnapshot {
            content: buffer.content.clone(),
            mode: self.vim_state.mode,
            cursor_line: self.cursor_line,
            cursor_col: self.cursor_col,
        }
    }

    fn restore_snapshot(&mut self, buffer: &mut TextBuffer, snapshot: DocumentSnapshot) {
        buffer.content = snapshot.content;
        self.vim_state.mode = snapshot.mode;
        self.cursor_line = snapshot.cursor_line;
        self.cursor_col = snapshot.cursor_col;
        self.vim_state.pending_command = None;
        self.count = None;
        self.selected_register = None;
        buffer.dirty = true;
        match self.vim_state.mode {
            VimMode::Normal | VimMode::Visual | VimMode::VisualLine => self.clamp_cursor_normal(buffer, ),
            VimMode::Insert => {
                self.cursor_line = self.cursor_line.min(self.line_count(buffer, ).saturating_sub(1));
                self.cursor_col = self.cursor_col.min(self.current_line_char_count(buffer, ));
            }
            _ => self.clamp_cursor_normal(buffer, ),
        }
    }

    fn record_undo(&mut self, buffer: &mut TextBuffer) {
        let snapshot = self.snapshot_document(buffer, );
        if buffer.undo_stack.last() != Some(&snapshot) {
            buffer.undo_stack.push(snapshot);
        }
        buffer.redo_stack.clear();
    }

    fn undo(&mut self, buffer: &mut TextBuffer) -> bool {
        let Some(snapshot) = buffer.undo_stack.pop() else {
            return false;
        };
        { let snap = self.snapshot_document(buffer); buffer.redo_stack.push(snap); }
        self.restore_snapshot(buffer, snapshot);
        true
    }

    fn redo(&mut self, buffer: &mut TextBuffer) -> bool {
        let Some(snapshot) = buffer.redo_stack.pop() else {
            return false;
        };
        { let snap = self.snapshot_document(buffer); buffer.undo_stack.push(snap); }
        self.restore_snapshot(buffer, snapshot);
        true
    }

    fn repeat_last_change(&mut self, buffer: &mut TextBuffer) -> bool {
        let Some(last_change) = self.last_change.clone() else {
            return false;
        };
        self.replaying_change = true;
        
        self.vim_state.insert_start_pos = Some(self.flattened_cursor(buffer, ));
        self.vim_state.macro_insert_start_index = Some(self.vim_state.current_macro.len());

        let mut changed = false;
        for step in last_change {
            if step == "." {
                continue;
            }
            if self.vim_state.mode == VimMode::Insert || self.vim_state.mode == VimMode::Replace {
                match step.as_str() {
                    "escape" | "ctrl+[" => self.enter_normal(buffer, ),
                    "ctrl+c" => self.cancel_insert(buffer, ),
                    "ctrl+w" => self.delete_word_insert(buffer, ),
                    "ctrl+u" => self.delete_line_insert(buffer, ),
                    "return" => self.insert_newline(buffer, ),
                    "backspace" => self.backspace(buffer, ),
                    "delete" => self.delete_at_cursor(buffer, ),
                    text => {
                        self.handle_insert_text(buffer, text);
                    }
                }
                changed = true;
            } else {
                changed |= self.handle_normal_input(buffer, &step);
            }
        }
        self.replaying_change = false;
        changed
    }

    fn take_register_target(&mut self, _buffer: &mut TextBuffer) -> RegisterTarget {
        self.selected_register
            .take()
            .unwrap_or(RegisterTarget::Unnamed)
    }

    fn cursor_position(&self, buffer: &mut TextBuffer) -> CursorPosition {
        CursorPosition {
            line: self.cursor_line(buffer, ),
            col: self.cursor_col(buffer, ),
        }
    }

    fn push_jump(&mut self, buffer: &mut TextBuffer) {
        let position = self.cursor_position(buffer, );
        if self.jump_list.last().copied() != Some(position) {
            self.jump_list.push(position);
            self.jump_index = Some(self.jump_list.len().saturating_sub(1));
        }
    }

    fn jump_section(&mut self, buffer: &mut TextBuffer, forward: bool, section_char: char, count: usize) {
        self.push_jump(buffer);
        let lines = self.lines_vec(buffer);
        let start_line = self.cursor_line.min(lines.len().saturating_sub(1));
        let mut target_line = start_line;

        for _ in 0..count {
            if forward {
                if target_line + 1 >= lines.len() { break; }
                let mut found = false;
                for i in (target_line + 1)..lines.len() {
                    if lines[i].starts_with(section_char) {
                        target_line = i;
                        found = true;
                        break;
                    }
                }
                if !found { target_line = lines.len().saturating_sub(1); }
            } else {
                if target_line == 0 { break; }
                let mut found = false;
                for i in (0..target_line).rev() {
                    if lines[i].starts_with(section_char) {
                        target_line = i;
                        found = true;
                        break;
                    }
                }
                if !found { target_line = 0; }
            }
        }
        self.cursor_line = target_line;
        self.cursor_col = 0;
        self.clamp_cursor_normal(buffer);
    }

    fn jump_history(&mut self, buffer: &mut TextBuffer, delta: isize) -> bool {
        if self.jump_list.is_empty() {
            return false;
        }

        let current = self.jump_index.unwrap_or_else(|| self.jump_list.len().saturating_sub(1)) as isize;
        let next = (current + delta).clamp(0, self.jump_list.len().saturating_sub(1) as isize);
        if Some(next as usize) == self.jump_index {
            return false;
        }

        let position = self.jump_list[next as usize];
        self.jump_index = Some(next as usize);
        self.cursor_line = position.line;
        self.cursor_col = position.col;
        self.clamp_cursor_normal(buffer);
        true
    }

    fn enter_visual(&mut self, buffer: &mut TextBuffer, kind: VisualKind) {
        self.visual_anchor_flat = Some(self.flattened_cursor(buffer, ));
        self.vim_state.mode = match kind {
            VisualKind::Character => VimMode::Visual,
            VisualKind::Line => VimMode::VisualLine,
            VisualKind::Block => VimMode::VisualBlock,
        };
        self.vim_state.pending_command = None;
        self.count = None;
    }

    fn visual_kind(&self, _buffer: &mut TextBuffer) -> VisualKind {
        match self.vim_state.mode {
            VimMode::Visual => VisualKind::Character,
            VimMode::VisualLine => VisualKind::Line,
            VimMode::VisualBlock => VisualKind::Block,
            _ => VisualKind::Character,
        }
    }

    fn visual_selection_range(&self, buffer: &TextBuffer) -> Option<TextRange> {
        let anchor = self.visual_anchor_flat?;
        let cursor = self.flattened_cursor(buffer, );
        match self.vim_state.mode {
            VimMode::Visual => {
                let start = anchor.min(cursor);
                let end = anchor.max(cursor).saturating_add(1);
                Some(TextRange::new(start, end))
            }
            VimMode::VisualBlock => {
                let start = anchor.min(cursor);
                let end = anchor.max(cursor).saturating_add(1);
                Some(TextRange::blockwise(start, end))
            }
            VimMode::VisualLine => {
                let anchor_line = self.line_for_flat(buffer, anchor);
                let cursor_line = self.cursor_line(buffer, );
                Some(self.linewise_range(buffer, anchor_line, cursor_line))
            }
            _ => None,
        }
    }

    fn apply_visual_operator(&mut self, buffer: &mut TextBuffer, operator: Operator) -> bool {
        let Some(range) = self.visual_selection_range(buffer, ) else {
            return false;
        };
        self.last_visual_selection = Some((range, self.visual_kind(buffer, )));
        self.visual_anchor_flat = None;
        self.vim_state.mode = VimMode::Normal;
        self.apply_operator_range(buffer, operator, range)
    }

    fn restore_last_visual_selection(&mut self, buffer: &mut TextBuffer) -> bool {
        let Some((range, kind)) = self.last_visual_selection else {
            return false;
        };
        self.visual_anchor_flat = Some(range.start);
        match kind {
            VisualKind::Character => {
                self.vim_state.mode = VimMode::Visual;
                self.set_cursor_from_flat(buffer, range.end.saturating_sub(1));
            }
            VisualKind::Line => {
                self.vim_state.mode = VimMode::VisualLine;
                self.set_cursor_from_flat(buffer, range.end.saturating_sub(1));
            }
            VisualKind::Block => {
                self.vim_state.mode = VimMode::VisualBlock;
                self.set_cursor_from_flat(buffer, range.end.saturating_sub(1));
            }
        }
        true
    }

    fn refresh_search_matches(&mut self, buffer: &mut TextBuffer) {
        buffer.search.highlights_active = buffer.settings.hlsearch;
        buffer.search.matches.clear();
        if buffer.search.pattern.is_empty() {
            return;
        }

        let chars: Vec<char> = buffer.content.chars().collect();
        let ignore_case = self.should_ignore_case(buffer, &buffer.search.pattern.clone());
        let pattern: Vec<char> = buffer.search.pattern.chars().collect();
        if chars.is_empty() || pattern.is_empty() || pattern.len() > chars.len() {
            return;
        }

        for start in 0..=chars.len() - pattern.len() {
            if chars_equal_at(&chars[start..start + pattern.len()], &pattern, ignore_case) {
                buffer.search
                    .matches
                    .push(TextRange::new(start, start + pattern.len()));
            }
        }
    }

    fn repeat_search(&mut self, buffer: &mut TextBuffer, opposite: bool) -> bool {
        if buffer.search.pattern.is_empty() {
            return false;
        }
        self.refresh_search_matches(buffer, );
        if buffer.search.matches.is_empty() {
            return false;
        }

        let reverse = buffer.search.reverse ^ opposite;
        let cursor = self.flattened_cursor(buffer, );
        let target = if reverse {
            buffer.search
                .matches
                .iter()
                .rev()
                .find(|range| range.start < cursor)
                .or_else(|| buffer.search.matches.last())
        } else {
            buffer.search
                .matches
                .iter()
                .find(|range| range.start > cursor)
                .or_else(|| buffer.search.matches.first())
        };
        let Some(target) = target.copied() else {
            return false;
        };

        self.push_jump(buffer, );
        self.set_cursor_from_flat(buffer, target.start);
        true
    }

    fn search_word_under_cursor(&mut self, buffer: &mut TextBuffer, reverse: bool) -> bool {
        let Some(range) = self.word_text_object_range(buffer, 1, false, false) else {
            return false;
        };
        let pattern = self.text_for_range(buffer, range);
        if pattern.is_empty() {
            return false;
        }
        buffer.search.pattern = pattern;
        buffer.search.reverse = reverse;
        self.refresh_search_matches(buffer, );
        self.repeat_search(buffer, false)
    }

    fn should_ignore_case(&self, buffer: &mut TextBuffer, pattern: &str) -> bool {
        if !buffer.settings.ignorecase {
            return false;
        }
        if buffer.settings.smartcase && pattern.chars().any(|ch| ch.is_uppercase()) {
            return false;
        }
        true
    }

    fn insert_blank_line(&mut self, buffer: &mut TextBuffer, index: usize) {
        let mut lines = self.lines_vec(buffer, );
        let index = index.min(lines.len());
        lines.insert(index, String::new());
        self.cursor_line = index;
        self.replace_lines(buffer, lines);
    }

    fn delete_current_line(&mut self, buffer: &mut TextBuffer) {
        self.apply_current_lines_operator(buffer, Operator::Delete, 1);
    }

    fn apply_current_lines_operator(&mut self, buffer: &mut TextBuffer, operator: Operator, count: usize) -> bool {
        let original_cursor = self.flattened_cursor(buffer, );
        let original_col = self.cursor_col(buffer, );
        self.enter_visual(buffer, VisualKind::Line);
        self.cursor_line = self.cursor_line(buffer, ).saturating_add(count.max(1).saturating_sub(1)).min(self.line_count(buffer, ).saturating_sub(1));
        let result = self.apply_visual_operator(buffer, operator);
        if operator == Operator::Yank {
            self.set_cursor_from_flat(buffer, original_cursor);
            self.cursor_col = original_col;
            self.clamp_cursor_normal(buffer);
        }
        result
    }


    fn delete_chars_on_current_line(&mut self, buffer: &mut TextBuffer, count: usize) -> bool {
        let mut lines = self.lines_vec(buffer, );
        if let Some(line) = lines.get_mut(self.cursor_line(buffer, )) {
            if line.is_empty() {
                return false;
            }

            let col = self.cursor_col(buffer, ).min(char_count(line).saturating_sub(1));
            let end = col.saturating_add(count.max(1)).min(char_count(line));
            let deleted = line.chars().skip(col).take(end - col).collect::<String>();
            if deleted.is_empty() {
                return false;
            }

            let target = self.take_register_target(buffer, );
            buffer.registers.store_deleted(target, deleted, false);
            self.record_undo(buffer);
            remove_char_range(line, col, end);
            buffer.content = lines.join("\n");
            buffer.dirty = true;
            self.clamp_cursor_line(buffer, );
            return true;
        }

        false
    }

    fn delete_chars_before_cursor(&mut self, buffer: &mut TextBuffer, count: usize) -> bool {
        let mut lines = self.lines_vec(buffer, );
        let line_index = self.cursor_line(buffer, );
        let Some(line) = lines.get_mut(line_index) else {
            return false;
        };
        if line.is_empty() || self.cursor_col(buffer, ) == 0 {
            return false;
        }

        let end = self.cursor_col(buffer, ).min(char_count(line));
        let start = end.saturating_sub(count.max(1));
        let deleted = line
            .chars()
            .skip(start)
            .take(end.saturating_sub(start))
            .collect::<String>();
        if deleted.is_empty() {
            return false;
        }

        let target = self.take_register_target(buffer, );
        buffer.registers.store_deleted(target, deleted, false);
        self.record_undo(buffer);
        self.push_change_location(buffer);
        remove_char_range(line, start, end);
        buffer.content = lines.join("\n");
        buffer.dirty = true;
        self.cursor_col = start;
        self.clamp_cursor_line(buffer, );
        true
    }

    fn replace_visual_selection(&mut self, buffer: &mut TextBuffer, replacement: char) -> bool {
        let Some(range) = self.visual_selection_range(buffer) else {
            return false;
        };

        if range.blockwise {
            let start_line = self.line_for_flat(buffer, range.start);
            let end_line = self.line_for_flat(buffer, range.end.saturating_sub(1));
            let top_line = start_line.min(end_line);
            let bottom_line = start_line.max(end_line);

            let start_col = {
                let line_start = self.line_start_flat(buffer, start_line);
                range.start.saturating_sub(line_start)
            };
            let end_col = {
                let line_start = self.line_start_flat(buffer, end_line);
                range.end.saturating_sub(1).saturating_sub(line_start)
            };
            let left_col = start_col.min(end_col);
            let right_col = start_col.max(end_col);

            let mut lines = self.lines_vec(buffer);
            for i in top_line..=bottom_line {
                if i < lines.len() {
                    let line_len = char_count(&lines[i]);
                    let c_left = left_col.min(line_len);
                    let c_right = (right_col + 1).min(line_len);
                    if c_left <= c_right {
                        let replaced: String = vec![replacement; c_right - c_left].into_iter().collect();
                        let before = lines[i].chars().take(c_left).collect::<String>();
                        let after = lines[i].chars().skip(c_right).collect::<String>();
                        lines[i] = format!("{}{}{}", before, replaced, after);
                    }
                }
            }
            self.replace_lines(buffer, lines);
        } else {
            let start = range.start.min(self.content_char_len(buffer));
            let end = range.end.min(self.content_char_len(buffer));
            
            let text = buffer.content.chars().skip(start).take(end - start).collect::<String>();
            
            let new_text = text.chars()
                .map(|c| if c == '\n' { '\n' } else { replacement })
                .collect::<String>();
            
            self.replace_flat_range(buffer, TextRange::new(start, end), &new_text);
        }

        self.visual_anchor_flat = None;
        self.vim_state.mode = VimMode::Normal;
        self.set_cursor_from_flat(buffer, range.start);
        self.clamp_cursor_normal(buffer);

        true
    }

    fn replace_chars(&mut self, buffer: &mut TextBuffer, replacement: char, count: usize) -> bool {
        let mut lines = self.lines_vec(buffer, );
        let line_index = self.cursor_line(buffer, );
        let Some(line) = lines.get_mut(line_index) else {
            return false;
        };
        let col = self.cursor_col(buffer, ).min(char_count(line));
        if col >= char_count(line) {
            return false;
        }
        let end = (col + count.max(1)).min(char_count(line));
        let deleted = line
            .chars()
            .skip(col)
            .take(end.saturating_sub(col))
            .collect::<String>();
        let target = self.take_register_target(buffer, );
        buffer.registers.store_deleted(target, deleted, false);
        let f_cursor = self.flattened_cursor(buffer);
        self.replace_flat_range(buffer, TextRange::new(f_cursor, f_cursor + end.saturating_sub(col)), &replacement.to_string().repeat(end.saturating_sub(col)));
        self.clamp_cursor_normal(buffer);
        true
    }

    fn push_change_location(&mut self, buffer: &mut TextBuffer) {
        let position = self.cursor_position(buffer, );
        if buffer.changelist.last().copied() == Some(position) {
            buffer.changelist_index = Some(buffer.changelist.len().saturating_sub(1));
            return;
        }
        if let Some(index) = buffer.changelist_index {
            if index + 1 < buffer.changelist.len() {
                buffer.changelist.truncate(index + 1);
            }
        }
        buffer.changelist.push(position);
        buffer.changelist_index = Some(buffer.changelist.len().saturating_sub(1));
    }

    fn navigate_changelist(&mut self, buffer: &mut TextBuffer, delta: isize) -> bool {
        if buffer.changelist.is_empty() {
            return false;
        }
        let current = buffer.changelist_index.unwrap_or_else(|| buffer.changelist.len().saturating_sub(1)) as isize;
        let next = (current + delta).clamp(0, buffer.changelist.len().saturating_sub(1) as isize);
        if Some(next as usize) == buffer.changelist_index {
            return false;
        }
        let position = buffer.changelist[next as usize];
        buffer.changelist_index = Some(next as usize);
        self.cursor_line = position.line;
        self.cursor_col = position.col;
        self.clamp_cursor_normal(buffer);
        true
    }

    fn substitute_chars(&mut self, buffer: &mut TextBuffer, count: usize) -> bool {
        let start = self.flattened_cursor(buffer, );
        let end = (start + count.max(1)).min(self.current_line_end_flat_exclusive(buffer, ));
        if start >= end {
            return false;
        }
        self.apply_operator_range(buffer, Operator::Change, TextRange::new(start, end))
    }

    fn join_lines(&mut self, buffer: &mut TextBuffer, count: usize, with_space: bool) -> bool {
        let mut lines = self.lines_vec(buffer, );
        if lines.len() < 2 || self.cursor_line(buffer, ) >= lines.len().saturating_sub(1) {
            return false;
        }

        let start_line = self.cursor_line(buffer, );
        let join_line_count = count.max(1).saturating_add(1);
        let max_join = join_line_count.min(lines.len().saturating_sub(start_line));
        if max_join <= 1 {
            return false;
        }

        self.record_undo(buffer);
        self.push_change_location(buffer);

        let mut joined = lines[start_line].clone();
        for _ in 1..max_join {
            let next = lines.remove(start_line + 1);
            if with_space {
                let needs_space = !joined.is_empty()
                    && !joined
                        .chars()
                        .last()
                        .map(|ch| ch.is_whitespace())
                        .unwrap_or(false)
                    && !next
                        .chars()
                        .next()
                        .map(|ch| ch.is_whitespace())
                        .unwrap_or(false);
                if needs_space {
                    joined.push(' ');
                }
                joined.push_str(next.trim_start());
            } else {
                joined.push_str(&next);
            }
        }
        lines[start_line] = joined;
        buffer.content = lines.join("\n");
        buffer.dirty = true;
        self.clamp_cursor_normal(buffer);
        true
    }

    fn first_non_blank_col(&self, buffer: &mut TextBuffer) -> usize {
        self.first_non_blank_col_for_line(buffer, self.cursor_line(buffer, ))
    }

    fn first_non_blank_col_for_line(&self, buffer: &mut TextBuffer, line_index: usize) -> usize {
        self.lines_vec(buffer, )
            .get(line_index)
            .and_then(|line| line.chars().position(|ch| !ch.is_whitespace()))
            .unwrap_or(0)
    }

    fn flat_index_for_line(&self, buffer: &TextBuffer, target_line: usize) -> usize {
        let lines = self.lines_vec(buffer, );
        let mut offset = 0;
        for line_index in 0..target_line.min(lines.len()) {
            offset += char_count(&lines[line_index]) + 1;
        }
        offset
    }

    pub fn flattened_cursor(&self, buffer: &TextBuffer) -> usize {
        let lines = self.lines_vec(buffer, );
        let mut offset = 0;
        for line_index in 0..self.cursor_line(buffer, ).min(lines.len()) {
            offset += char_count(&lines[line_index]) + 1;
        }
        offset + self.cursor_col(buffer, ).min(self.current_line_char_count(buffer, ))
    }

    fn set_cursor_from_flat(&mut self, buffer: &mut TextBuffer, mut offset: usize) {
        let lines = self.lines_vec(buffer, );
        for (line_index, line) in lines.iter().enumerate() {
            let len = char_count(line);
            if offset <= len {
                self.cursor_line = line_index;
                self.cursor_col = offset.min(len.saturating_sub(1));
                self.clamp_cursor_normal(buffer);
                return;
            }
            offset = offset.saturating_sub(len + 1);
        }
        self.cursor_line = lines.len().saturating_sub(1);
        self.cursor_col = self.current_line_max_col(buffer, );
    }

    fn move_word_forward(&mut self, buffer: &mut TextBuffer, count: usize, big_word: bool) {
        for _ in 0..count {
            let text = self.content_with_virtual_empty_line(buffer, );
            let chars: Vec<char> = text.chars().collect();
            let mut index = self.flattened_cursor(buffer, ).min(chars.len().saturating_sub(1));
            while index < chars.len() && is_word_char(chars[index], big_word) {
                index += 1;
            }
            while index < chars.len() && !is_word_char(chars[index], big_word) {
                index += 1;
            }
            self.set_cursor_from_flat(buffer, index.min(chars.len().saturating_sub(1)));
        }
    }

    fn move_word_backward(&mut self, buffer: &mut TextBuffer, count: usize, big_word: bool) {
        for _ in 0..count {
            let text = self.content_with_virtual_empty_line(buffer, );
            let chars: Vec<char> = text.chars().collect();
            let mut index = self.flattened_cursor(buffer, ).saturating_sub(1).min(chars.len());
            while index > 0 && !is_word_char(chars[index], big_word) {
                index -= 1;
            }
            while index > 0 && is_word_char(chars[index - 1], big_word) {
                index -= 1;
            }
            self.set_cursor_from_flat(buffer, index);
        }
    }

    fn move_word_end(&mut self, buffer: &mut TextBuffer, count: usize, big_word: bool) {
        for _ in 0..count {
            let text = self.content_with_virtual_empty_line(buffer, );
            let chars: Vec<char> = text.chars().collect();
            let mut index = self.flattened_cursor(buffer, ).saturating_add(1).min(chars.len());
            while index < chars.len() && !is_word_char(chars[index], big_word) {
                index += 1;
            }
            while index + 1 < chars.len() && is_word_char(chars[index + 1], big_word) {
                index += 1;
            }
            self.set_cursor_from_flat(buffer, index.min(chars.len().saturating_sub(1)));
        }
    }

    fn move_word_backward_end(&mut self, buffer: &mut TextBuffer, count: usize, big_word: bool) {
        for _ in 0..count {
            let text = self.content_with_virtual_empty_line(buffer, );
            let chars: Vec<char> = text.chars().collect();
            if chars.is_empty() {
                return;
            }
            let mut index = self.flattened_cursor(buffer, ).min(chars.len().saturating_sub(1));
            if index > 0 {
                index -= 1;
            }
            while index > 0 && !is_word_char(chars[index], big_word) {
                index -= 1;
            }
            while index + 1 < chars.len() && is_word_char(chars[index + 1], big_word) {
                index += 1;
            }
            self.set_cursor_from_flat(buffer, index);
        }
    }

    fn move_first_nonblank_on_relative_line(&mut self, buffer: &mut TextBuffer, delta: isize) {
        let current = self.cursor_line(buffer) as isize;
        let max = self.line_count(buffer, ).saturating_sub(1) as isize;
        let target = (current + delta).clamp(0, max) as usize;
        self.cursor_line = target;
        self.cursor_col = self.first_non_blank_col_for_line(buffer, target);
        self.clamp_cursor_normal(buffer);
    }

    fn content_with_virtual_empty_line(&self, buffer: &mut TextBuffer) -> String {
        if buffer.content.is_empty() {
            " ".to_string()
        } else {
            buffer.content.clone()
        }
    }

    fn apply_operator_motion(&mut self, buffer: &mut TextBuffer, operator: Operator, count: usize, motion: char) -> bool {
        let Some(range) = self.operator_motion_range(buffer, count.max(1), motion) else {
            return false;
        };

        self.apply_operator_range(buffer, operator, range)
    }

    fn apply_text_object_operator(
        &mut self, buffer: &mut TextBuffer,
        operator: Operator,
        count: usize,
        around: bool,
        object: TextObject,
    ) -> bool {
        let range = match object {
            TextObject::Word { big_word } => {
                self.word_text_object_range(buffer, count.max(1), around, big_word)
            }
            TextObject::Delimited(delimiter) => self.delimited_text_object_range(buffer, delimiter, around),
            TextObject::Paragraph => self.paragraph_text_object_range(buffer, count.max(1), around),
            TextObject::Line => self.line_text_object_range(buffer, count.max(1)),
        };
        let Some(range) = range else {
            return false;
        };

        self.apply_operator_range(buffer, operator, range)
    }

    fn apply_visual_text_object(
        &mut self, buffer: &mut TextBuffer,
        count: usize,
        around: bool,
        object: TextObject,
    ) -> bool {
        let range = match object {
            TextObject::Word { big_word } => {
                self.word_text_object_range(buffer, count.max(1), around, big_word)
            }
            TextObject::Delimited(delimiter) => self.delimited_text_object_range(buffer, delimiter, around),
            TextObject::Paragraph => self.paragraph_text_object_range(buffer, count.max(1), around),
            TextObject::Line => self.line_text_object_range(buffer, count.max(1)),
        };
        let Some(range) = range else {
            return false;
        };

        if range.linewise && self.vim_state.mode == VimMode::Visual {
            self.vim_state.mode = VimMode::VisualLine;
        } else if !range.linewise && self.vim_state.mode == VimMode::VisualLine {
            self.vim_state.mode = VimMode::Visual;
        }

        let current_anchor = self.visual_anchor_flat.unwrap_or(range.start);
        let cursor_flat = self.flattened_cursor(buffer, );
        if current_anchor == cursor_flat {
            self.visual_anchor_flat = Some(range.start);
            self.set_cursor_from_flat(buffer, range.end.saturating_sub(1));
        } else if cursor_flat >= current_anchor {
            self.visual_anchor_flat = Some(current_anchor.min(range.start));
            self.set_cursor_from_flat(buffer, range.end.saturating_sub(1).max(cursor_flat));
        } else {
            self.visual_anchor_flat = Some(current_anchor.max(range.end.saturating_sub(1)));
            self.set_cursor_from_flat(buffer, range.start.min(cursor_flat));
        }

        true
    }

    fn apply_operator_range(&mut self, buffer: &mut TextBuffer, operator: Operator, range: TextRange) -> bool {
        let range = range.normalized().clamped(self.content_char_len(buffer));
        if !range.linewise && range.start >= range.end && !range.blockwise {
            return false;
        }

        if self.defer_enabled && (operator == Operator::Delete || operator == Operator::Change) && range.linewise {
            self.yank_highlight = Some(range);
            self.deferred_action = Some(DeferredAction { operator, range });
            return false;
        }

        self.apply_operator_range_direct(buffer, operator, range)
    }

    fn apply_blockwise_operator(&mut self, buffer: &mut TextBuffer, operator: Operator, range: TextRange) -> bool {
        let start_line = self.line_for_flat(buffer, range.start);
        let end_line = self.line_for_flat(buffer, range.end.saturating_sub(1));
        let top_line = start_line.min(end_line);
        let bottom_line = start_line.max(end_line);

        let start_col = {
            let line = self.line_for_flat(buffer, range.start);
            let line_start = self.line_start_flat(buffer, line);
            range.start.saturating_sub(line_start)
        };
        let end_col = {
            let line = self.line_for_flat(buffer, range.end.saturating_sub(1));
            let line_start = self.line_start_flat(buffer, line);
            range.end.saturating_sub(1).saturating_sub(line_start)
        };
        let left_col = start_col.min(end_col);
        let right_col = start_col.max(end_col);

        let mut lines = self.lines_vec(buffer, );

        match operator {
            Operator::Yank => {
                let mut yanked = Vec::new();
                for i in top_line..=bottom_line {
                    if i < lines.len() {
                        let line_len = char_count(&lines[i]);
                        let c_left = left_col.min(line_len);
                        let c_right = (right_col + 1).min(line_len);
                        if c_left <= c_right {
                            yanked.push(lines[i].chars().skip(c_left).take(c_right - c_left).collect::<String>());
                        } else {
                            yanked.push(String::new());
                        }
                    }
                }
                let target = self.take_register_target(buffer, );
                let register = RegisterValue { text: yanked.join("\n"), linewise: false, blockwise: true };
                buffer.registers.store_target(target, register.clone());
                if target == RegisterTarget::Unnamed {
                    buffer.registers.named.insert('0', register.clone());
                }
                buffer.registers.yank = register;
                self.set_cursor_from_flat(buffer, range.start);
                true
            }
            Operator::ToggleCase => {
                self.toggle_case_range(buffer, range);
                self.enter_normal(buffer, );
                true
            }
            Operator::Format | Operator::FormatLeaveCursor => {
                // TODO: Implement actual formatting
                self.enter_normal(buffer, );
                true
            }
            Operator::Filter => {
                // TODO: Implement external filtering
                self.enter_normal(buffer, );
                true
            }
            Operator::Delete => {
                let mut deleted = Vec::new();
                for i in top_line..=bottom_line {
                    if i < lines.len() {
                        let line_len = char_count(&lines[i]);
                        let c_left = left_col.min(line_len);
                        let c_right = (right_col + 1).min(line_len);
                        if c_left <= c_right {
                            deleted.push(lines[i].chars().skip(c_left).take(c_right - c_left).collect::<String>());
                            remove_char_range(&mut lines[i], c_left, c_right);
                        } else {
                            deleted.push(String::new());
                        }
                    }
                }
                let target = self.take_register_target(buffer, );
                let register = RegisterValue { text: deleted.join("\n"), linewise: false, blockwise: true };
                buffer.registers.store_target(target, register.clone());
                if target == RegisterTarget::Unnamed {
                    buffer.registers.named.insert('1', register.clone());
                }
                
                self.replace_lines(buffer, lines);
                self.set_cursor_from_flat(buffer, range.start);
                self.cursor_col = left_col;
                true
            }
            Operator::Change => {
                let mut deleted = Vec::new();
                for i in top_line..=bottom_line {
                    if i < lines.len() {
                        let line_len = char_count(&lines[i]);
                        let c_left = left_col.min(line_len);
                        let c_right = (right_col + 1).min(line_len);
                        if c_left <= c_right {
                            deleted.push(lines[i].chars().skip(c_left).take(c_right - c_left).collect::<String>());
                            remove_char_range(&mut lines[i], c_left, c_right);
                        } else {
                            deleted.push(String::new());
                        }
                    }
                }
                let target = self.take_register_target(buffer, );
                let register = RegisterValue { text: deleted.join("\n"), linewise: false, blockwise: true };
                buffer.registers.store_target(target, register.clone());
                if target == RegisterTarget::Unnamed {
                    buffer.registers.named.insert('1', register.clone());
                }
                
                self.replace_lines(buffer, lines);
                self.set_cursor_from_flat(buffer, range.start);
                self.cursor_col = left_col;
                
                self.deferred_action = Some(DeferredAction { operator: Operator::BlockInsert, range });
                self.enter_insert(buffer, );
                true
            }
            Operator::BlockInsert | Operator::BlockAppend => {
                let inserted = if let Some(start) = self.vim_state.insert_start_pos {
                    let end = self.flattened_cursor(buffer, );
                    if end > start {
                        self.text_for_range(buffer, TextRange::new(start, end))
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };

                if !inserted.is_empty() {
                    let insert_col = if operator == Operator::BlockInsert { left_col } else { right_col + 1 };
                    for i in top_line..=bottom_line {
                        if i == self.cursor_line(buffer, ) { continue; } // Already inserted on this line by user
                        if i < lines.len() {
                            let line_len = char_count(&lines[i]);
                            let col = insert_col.min(line_len);
                            insert_str_at_char(&mut lines[i], col, &inserted);
                        }
                    }
                    self.replace_lines(buffer, lines);
                }
                true
            }
            _ => false,
        }
    }

    fn apply_operator_range_direct(&mut self, buffer: &mut TextBuffer, operator: Operator, range: TextRange) -> bool {
        if range.blockwise {
            return self.apply_blockwise_operator(buffer, operator, range);
        }
        match operator {
            Operator::SurroundAdd => {
                self.vim_state.pending_command = Some(PendingCommand::SurroundWaitAdd {
                    range,
                    buffer: String::new(),
                });
                false
            }
            Operator::ToggleCase => {
                self.toggle_case_range(buffer, range);
                self.enter_normal(buffer, );
                true
            }
            Operator::Format | Operator::FormatLeaveCursor => {
                self.format_range(buffer, range);
                self.enter_normal(buffer, );
                true
            }
            Operator::Filter => {
                // TODO: Implement external filtering
                self.enter_normal(buffer, );
                true
            }
            Operator::Delete => {
                let text = self.text_for_range(buffer, range);
                let target = self.take_register_target(buffer, );
                buffer.registers.store_deleted(target, text, range.linewise);

                let mut actual_delete_range = range;
                if range.linewise && range.end == self.content_char_len(buffer) && range.start > 0 {
                    if buffer.content.chars().nth(range.start - 1) == Some('\n') {
                        actual_delete_range.start -= 1;
                    }
                }

                self.delete_flat_range(buffer, actual_delete_range);
                self.set_insert_cursor_from_flat(buffer, actual_delete_range.start);
                self.enter_normal(buffer, );
                true
            }
            Operator::Change => {
                let text = self.text_for_range(buffer, range);
                let target = self.take_register_target(buffer, );
                buffer.registers.store_deleted(target, text, range.linewise);

                let mut actual_delete_range = range;
                if range.linewise && range.end == self.content_char_len(buffer) && range.start > 0 {
                    if buffer.content.chars().nth(range.start - 1) == Some('\n') {
                        actual_delete_range.start -= 1;
                    }
                }

                self.delete_flat_range(buffer, actual_delete_range);

                if range.linewise {
                    insert_str_at_char(&mut buffer.content, actual_delete_range.start, "\n");
                    buffer.dirty = true;
                    self.set_insert_cursor_from_flat(buffer, actual_delete_range.start);
                } else {
                    self.set_insert_cursor_from_flat(buffer, actual_delete_range.start);
                }

                self.enter_insert(buffer, );
                true
            }
            Operator::Yank => {
                let text = self.text_for_range(buffer, range);
                let target = self.take_register_target(buffer, );
                buffer.registers.store_yank(target, text, range.linewise);
                if range.linewise {
                    self.set_cursor_from_flat(buffer, range.start);
                    self.cursor_col = self.first_non_blank_col(buffer, );
                } else {
                    self.set_cursor_from_flat(buffer, range.start);
                }
                self.clamp_cursor_normal(buffer);
                self.yank_highlight = Some(range);
                false
            }
            Operator::Indent => self.indent_range(buffer, range, 1),
            Operator::Outdent => self.indent_range(buffer, range, -1),

            Operator::BlockInsert | Operator::BlockAppend => false,
        }
    }

    fn apply_surround_add(&mut self, buffer: &mut TextBuffer, range: TextRange, spec: crate::vim::surround::SurroundSpec) {
        let (left, right) = spec.strings();
        let mut actual_range = range;
        if !actual_range.linewise && actual_range.end == self.content_char_len(buffer) && actual_range.start > 0 {
            if buffer.content.chars().nth(actual_range.start - 1) == Some('\n') {
                actual_range.start -= 1;
            }
        }
        
        self.record_undo(buffer);
        self.push_change_location(buffer);
        
        // Insert right first so it doesn't mess up the start index
        insert_str_at_char(&mut buffer.content, actual_range.end, &right);
        insert_str_at_char(&mut buffer.content, actual_range.start, &left);
        
        buffer.dirty = true;
        self.set_cursor_from_flat(buffer, actual_range.start);
        self.clamp_cursor_normal(buffer);
    }

    fn find_surround_target(&self, buffer: &TextBuffer, spec: &crate::vim::surround::SurroundSpec) -> Option<(TextRange, TextRange)> {
        match spec {
            crate::vim::surround::SurroundSpec::Pair(open, close) => {
                let chars: Vec<char> = buffer.content.chars().collect();
                if chars.is_empty() {
                    return None;
                }
                let cursor = self.flattened_cursor(buffer, ).min(chars.len().saturating_sub(1));
                
                // Find nearest open backwards
                let mut start = None;
                let mut depth = 0;
                for i in (0..=cursor).rev() {
                    if chars[i] == *close && open != close {
                        depth += 1;
                    } else if chars[i] == *open {
                        if depth == 0 {
                            start = Some(i);
                            break;
                        } else {
                            depth -= 1;
                        }
                    }
                }
                
                // Find nearest close forwards
                let mut end = None;
                depth = 0;
                for i in cursor..chars.len() {
                    if chars[i] == *open && open != close {
                        depth += 1;
                    } else if chars[i] == *close {
                        if depth == 0 {
                            end = Some(i);
                            break;
                        } else {
                            depth -= 1;
                        }
                    }
                }
                
                let start = start?;
                let end = end?;
                if start >= end {
                    return None;
                }
                Some((TextRange::new(start, start + 1), TextRange::new(end, end + 1)))
            }
            crate::vim::surround::SurroundSpec::Tag { .. } | crate::vim::surround::SurroundSpec::AnyTag => {
                let chars: Vec<char> = buffer.content.chars().collect();
                if chars.is_empty() {
                    return None;
                }
                let cursor = self.flattened_cursor(buffer, ).min(chars.len().saturating_sub(1));
                
                // Find backwards for `<`
                let mut start = None;
                for i in (0..=cursor).rev() {
                    if chars[i] == '<' {
                        // Check if it's an opening tag
                        if i + 1 < chars.len() && chars[i + 1] != '/' {
                            start = Some(i);
                            break;
                        }
                    }
                }
                
                let mut end = None;
                for i in cursor..chars.len() {
                    if chars[i] == '<' {
                        // Check if it's a closing tag
                        if i + 1 < chars.len() && chars[i + 1] == '/' {
                            end = Some(i);
                            break;
                        }
                    }
                }
                
                let start = start?;
                let end = end?;
                
                // Find the > for start
                let start_end = (start..chars.len()).find(|&i| chars[i] == '>')?;
                // Find the > for end
                let end_end = (end..chars.len()).find(|&i| chars[i] == '>')?;
                
                Some((TextRange::new(start, start_end + 1), TextRange::new(end, end_end + 1)))
            }
        }
    }

    fn apply_surround_delete(&mut self, buffer: &mut TextBuffer, spec: crate::vim::surround::SurroundSpec) {
        if let Some((left, right)) = self.find_surround_target(buffer, &spec) {
            self.record_undo(buffer);
            self.push_change_location(buffer);
            
            self.delete_flat_range(buffer, right.clone());
            self.delete_flat_range(buffer, left.clone());
            
            buffer.dirty = true;
            self.set_cursor_from_flat(buffer, left.start);
            self.clamp_cursor_normal(buffer);
        }
    }

    fn apply_surround_change(&mut self, buffer: &mut TextBuffer, old_spec: crate::vim::surround::SurroundSpec, new_spec: crate::vim::surround::SurroundSpec) {
        if let Some((left, right)) = self.find_surround_target(buffer, &old_spec) {
            let (new_left, new_right) = new_spec.strings();
            self.record_undo(buffer);
            self.push_change_location(buffer);
            
            // Right side
            self.delete_flat_range(buffer, right.clone());
            insert_str_at_char(&mut buffer.content, right.start, &new_right);
            
            // Left side
            self.delete_flat_range(buffer, left.clone());
            insert_str_at_char(&mut buffer.content, left.start, &new_left);
            
            buffer.dirty = true;
            self.set_cursor_from_flat(buffer, left.start);
            self.clamp_cursor_normal(buffer);
        }
    }

    fn operator_motion_range(&self, buffer: &mut TextBuffer, count: usize, motion: char) -> Option<TextRange> {
        let start = self.flattened_cursor(buffer, );
        match motion {
            'h' => Some(TextRange::new(
                start.saturating_sub(count),
                start.min(self.content_char_len(buffer)),
            )),
            'l' => Some(TextRange::new(
                start,
                (start + count).min(self.current_line_end_flat_exclusive(buffer, )),
            )),
            'w' | 'W' => {
                let mut target = self.clone();
                target.move_word_forward(buffer, count, motion == 'W');
                let mut end = target.flattened_cursor(buffer, ).min(self.content_char_len(buffer));
                if end == self.content_char_len(buffer).saturating_sub(1) && start < end {
                    end = self.content_char_len(buffer);
                }
                Some(TextRange::new(start, end))
            }
            'b' | 'B' => {
                let mut target = self.clone();
                target.move_word_backward(buffer, count, motion == 'B');
                Some(TextRange::new(target.flattened_cursor(buffer, ), start))
            }
            'e' | 'E' => {
                let mut target = self.clone();
                target.move_word_end(buffer, count, motion == 'E');
                Some(TextRange::new(
                    start,
                    target.flattened_cursor(buffer, ).saturating_add(1),
                ))
            }
            '0' => Some(TextRange::new(self.current_line_start_flat(buffer, ), start)),
            '^' => Some(TextRange::new(
                self.current_line_start_flat(buffer, ) + self.first_non_blank_col(buffer, ),
                start,
            )),
            '$' => Some(TextRange::new(
                start,
                self.current_line_end_flat_exclusive(buffer, ),
            )),
            'j' | 'k' => {
                let mut target = self.clone();
                let delta = if motion == 'j' {
                    count as isize
                } else {
                    -(count as isize)
                };
                target.move_cursor_line(buffer, delta);
                Some(self.linewise_range(buffer, self.cursor_line(buffer, ), target.cursor_line(buffer)))
            }
            'G' => {
                let target_line = count.saturating_sub(1);
                Some(self.linewise_range(buffer, self.cursor_line(buffer, ), target_line))
            }
            ';' | ',' => self.find_char_range(buffer, motion == ';', count),
            '%' | '{' | '}' | '(' | ')' => {
                let target = self.extended_motion_flat(buffer, motion, count)?;
                let start = self.flattened_cursor(buffer, );
                if start <= target {
                    Some(TextRange::new(start, target))
                } else {
                    Some(TextRange::new(target, start))
                }
            }
            'H' | 'M' | 'L' => {
                let target_line = self.extended_motion_line(buffer, motion, count)?;
                Some(self.linewise_range(buffer, self.cursor_line(buffer, ), target_line))
            }
            _ => None,
        }
    }

    fn find_char_flat(&self, buffer: &mut TextBuffer, ch: char, is_t: bool, is_forward: bool, count: usize) -> Option<usize> {
        let lines = self.lines_vec(buffer, );
        let line = lines.get(self.cursor_line(buffer, )).map(|s| s.as_str()).unwrap_or("");
        let col = self.cursor_col(buffer, );
        let chars: Vec<char> = line.chars().collect();
        
        let mut found = 0;
        if is_forward {
            for i in (col + 1)..chars.len() {
                if chars[i] == ch {
                    found += 1;
                    if found == count {
                        return Some(self.current_line_start_flat(buffer, ) + if is_t { i - 1 } else { i });
                    }
                }
            }
        } else {
            if col == 0 {
                return None;
            }
            for i in (0..col).rev() {
                if chars[i] == ch {
                    found += 1;
                    if found == count {
                        return Some(self.current_line_start_flat(buffer, ) + if is_t { i + 1 } else { i });
                    }
                }
            }
        }
        None
    }

    fn execute_find_char(&mut self, buffer: &mut TextBuffer, ch: char, is_t: bool, is_forward: bool, count: usize) -> bool {
        self.vim_state.last_find = Some((ch, is_t, is_forward));
        if let Some(target) = self.find_char_flat(buffer, ch, is_t, is_forward, count) {
            self.set_cursor_from_flat(buffer, target);
            return true;
        }
        false
    }

    fn repeat_find_char(&mut self, buffer: &mut TextBuffer, is_semi: bool, count: usize) -> bool {
        let Some((ch, is_t, mut is_forward)) = self.vim_state.last_find else {
            return false;
        };
        if !is_semi {
            is_forward = !is_forward;
        }
        if let Some(target) = self.find_char_flat(buffer, ch, is_t, is_forward, count) {
            self.set_cursor_from_flat(buffer, target);
            return true;
        }
        false
    }

    fn execute_operator_find_char(
        &mut self, buffer: &mut TextBuffer,
        operator: Operator,
        _operator_count: usize,
        ch: char,
        is_t: bool,
        is_forward: bool,
        find_count: usize,
    ) -> bool {
        self.vim_state.last_find = Some((ch, is_t, is_forward));
        let start = self.flattened_cursor(buffer, );
        let Some(target) = self.find_char_flat(buffer, ch, is_t, is_forward, find_count) else {
            return false;
        };
        
        let mut end = target;
        if is_forward {
            end = target.saturating_add(1);
        }
        let range = if start <= end {
            TextRange::new(start, end)
        } else {
            TextRange::new(end, start.saturating_add(1))
        };
        
        self.apply_operator_range(buffer, operator, range)
    }

    fn find_char_range(&self, buffer: &mut TextBuffer, is_semi: bool, count: usize) -> Option<TextRange> {
        let Some((ch, is_t, mut is_forward)) = self.vim_state.last_find else {
            return None;
        };
        if !is_semi {
            is_forward = !is_forward;
        }
        let start = self.flattened_cursor(buffer, );
        let target = self.find_char_flat(buffer, ch, is_t, is_forward, count)?;
        
        let mut end = target;
        if is_forward {
            end = target.saturating_add(1);
        }
        if start <= end {
            Some(TextRange::new(start, end))
        } else {
            Some(TextRange::new(end, start.saturating_add(1)))
        }
    }

    fn scroll_viewport_half_page(&mut self, buffer: &mut TextBuffer, down: bool) -> bool {
        let lines = self.viewport_state.visible_lines.max(2) / 2;
        self.push_jump(buffer, );
        if down {
            self.cursor_line = self.cursor_line.saturating_add(lines).min(self.line_count(buffer, ).saturating_sub(1));
            self.viewport_state.top_line = self.viewport_state.top_line.saturating_add(lines);
        } else {
            self.cursor_line = self.cursor_line.saturating_sub(lines);
            self.viewport_state.top_line = self.viewport_state.top_line.saturating_sub(lines);
        }
        self.clamp_cursor_normal(buffer);
        true
    }

    fn scroll_viewport_full_page(&mut self, buffer: &mut TextBuffer, down: bool) -> bool {
        let lines = self.viewport_state.visible_lines.saturating_sub(2).max(1);
        self.push_jump(buffer, );
        if down {
            self.cursor_line = self.cursor_line.saturating_add(lines).min(self.line_count(buffer, ).saturating_sub(1));
            self.viewport_state.top_line = self.viewport_state.top_line.saturating_add(lines);
        } else {
            self.cursor_line = self.cursor_line.saturating_sub(lines);
            self.viewport_state.top_line = self.viewport_state.top_line.saturating_sub(lines);
        }
        self.clamp_cursor_normal(buffer);
        true
    }

    fn scroll_viewport_line(&mut self, _buffer: &mut TextBuffer, down: bool) -> bool {
        if down {
            self.viewport_state.top_line = self.viewport_state.top_line.saturating_add(1);
        } else {
            self.viewport_state.top_line = self.viewport_state.top_line.saturating_sub(1);
        }
        true
    }

    fn cursor_to_center(&mut self, _buffer: &mut TextBuffer) -> bool {
        let half = self.viewport_state.visible_lines / 2;
        self.viewport_state.top_line = self.cursor_line.saturating_sub(half);
        true
    }

    fn cursor_to_top(&mut self, _buffer: &mut TextBuffer) -> bool {
        self.viewport_state.top_line = self.cursor_line;
        true
    }

    fn cursor_to_bottom(&mut self, _buffer: &mut TextBuffer) -> bool {
        self.viewport_state.top_line = self.cursor_line.saturating_sub(self.viewport_state.visible_lines.saturating_sub(1));
        true
    }

    fn find_matching_bracket(&self, buffer: &mut TextBuffer, start: usize) -> Option<usize> {
        let chars: Vec<char> = buffer.content.chars().collect();
        if start >= chars.len() {
            return None;
        }
        let ch = chars[start];
        let (open, close, forward) = match ch {
            '(' => ('(', ')', true),
            '[' => ('[', ']', true),
            '{' => ('{', '}', true),
            ')' => ('(', ')', false),
            ']' => ('[', ']', false),
            '}' => ('{', '}', false),
            _ => return None,
        };

        let mut depth = 1;
        if forward {
            for i in (start + 1)..chars.len() {
                if chars[i] == open {
                    depth += 1;
                } else if chars[i] == close {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
            }
        } else {
            if start == 0 {
                return None;
            }
            for i in (0..start).rev() {
                if chars[i] == close {
                    depth += 1;
                } else if chars[i] == open {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
            }
        }
        None
    }

    fn find_paragraph_forward(&self, buffer: &mut TextBuffer, count: usize) -> Option<usize> {
        let lines = self.lines_vec(buffer, );
        let mut curr = self.cursor_line(buffer, );
        for _ in 0..count {
            if curr >= lines.len() {
                break;
            }
            while curr < lines.len() && lines[curr].trim().is_empty() {
                curr += 1;
            }
            while curr < lines.len() && !lines[curr].trim().is_empty() {
                curr += 1;
            }
        }
        Some(self.line_start_flat(buffer, curr.min(lines.len().saturating_sub(1))))
    }

    fn find_paragraph_backward(&self, buffer: &mut TextBuffer, count: usize) -> Option<usize> {
        let lines = self.lines_vec(buffer, );
        let mut curr = self.cursor_line(buffer, );
        for _ in 0..count {
            if curr == 0 {
                break;
            }
            curr -= 1;
            while curr > 0 && lines[curr].trim().is_empty() {
                curr -= 1;
            }
            while curr > 0 && !lines[curr].trim().is_empty() {
                curr -= 1;
            }
        }
        Some(self.line_start_flat(buffer, curr))
    }

    fn find_sentence_forward(&self, buffer: &mut TextBuffer, count: usize) -> Option<usize> {
        let chars: Vec<char> = buffer.content.chars().collect();
        let mut i = self.flattened_cursor(buffer, );
        for _ in 0..count {
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            while i < chars.len() {
                let ch = chars[i];
                if (ch == '.' || ch == '!' || ch == '?') && (i + 1 == chars.len() || chars[i+1].is_whitespace()) {
                    i += 1;
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }
                    break;
                }
                i += 1;
            }
        }
        if chars.is_empty() {
            return None;
        }
        Some(i.min(chars.len().saturating_sub(1)))
    }

    fn find_sentence_backward(&self, buffer: &mut TextBuffer, count: usize) -> Option<usize> {
        let chars: Vec<char> = buffer.content.chars().collect();
        if chars.is_empty() {
            return None;
        }
        let mut i = self.flattened_cursor(buffer, );
        for _ in 0..count {
            if i == 0 { break; }
            i -= 1;
            while i > 0 && chars[i].is_whitespace() {
                i -= 1;
            }
            while i > 0 {
                let ch = chars[i - 1];
                if (ch == '.' || ch == '!' || ch == '?') && chars[i].is_whitespace() {
                    break;
                }
                i -= 1;
            }
        }
        Some(i)
    }

    fn extended_motion_flat(&self, buffer: &mut TextBuffer, motion: char, count: usize) -> Option<usize> {
        match motion {
            '%' => self.find_matching_bracket(buffer, self.flattened_cursor(buffer, )),
            '{' => self.find_paragraph_backward(buffer, count),
            '}' => self.find_paragraph_forward(buffer, count),
            '(' => self.find_sentence_backward(buffer, count),
            ')' => self.find_sentence_forward(buffer, count),
            _ => None,
        }
    }

    fn extended_motion_line(&self, _buffer: &mut TextBuffer, motion: char, count: usize) -> Option<usize> {
        match motion {
            'H' => Some(self.viewport_state.top_line.saturating_add(count.saturating_sub(1))),
            'M' => Some(self.viewport_state.top_line.saturating_add(self.viewport_state.visible_lines / 2)),
            'L' => Some((self.viewport_state.top_line + self.viewport_state.visible_lines).saturating_sub(count)),
            _ => None,
        }
    }

    fn word_text_object_range(
        &self, buffer: &mut TextBuffer,
        count: usize,
        around: bool,
        big_word: bool,
    ) -> Option<TextRange> {
        let chars: Vec<char> = buffer.content.chars().collect();
        if chars.is_empty() {
            return None;
        }

        let mut start = self.flattened_cursor(buffer, ).min(chars.len().saturating_sub(1));
        if !is_word_char(chars[start], big_word) {
            while start < chars.len() && !is_word_char(chars[start], big_word) {
                start += 1;
            }
            if start >= chars.len() {
                return None;
            }
        }

        while start > 0 && is_word_char(chars[start - 1], big_word) {
            start -= 1;
        }

        let mut end = start;
        for index in 0..count {
            if index > 0 {
                while end < chars.len() && !is_word_char(chars[end], big_word) {
                    end += 1;
                }
            }
            while end < chars.len() && is_word_char(chars[end], big_word) {
                end += 1;
            }
            if end >= chars.len() {
                break;
            }
        }

        if around {
            if end < chars.len() {
                while end < chars.len() && chars[end].is_whitespace() && chars[end] != '\n' {
                    end += 1;
                }
            } else {
                while start > 0 && chars[start - 1].is_whitespace() && chars[start - 1] != '\n' {
                    start -= 1;
                }
            }
        }

        Some(TextRange::new(start, end))
    }

    fn delimited_text_object_range(&self, buffer: &mut TextBuffer, delimiter: char, around: bool) -> Option<TextRange> {
        let (open, close) = delimiter_pair(delimiter)?;
        let chars: Vec<char> = buffer.content.chars().collect();
        if chars.is_empty() {
            return None;
        }

        let cursor = self.flattened_cursor(buffer, ).min(chars.len().saturating_sub(1));
        
        let find_pair = |start_scan: usize| -> Option<(usize, usize)> {
            // If open == close (e.g. quotes), we just find the nearest pair around or ahead.
            if open == close {
                // Quotes don't nest.
                let line_start = chars[..start_scan].iter().rposition(|&c| c == '\n').map(|p| p + 1).unwrap_or(0);
                let line_end = chars[start_scan..].iter().position(|&c| c == '\n').map(|p| start_scan + p).unwrap_or(chars.len());
                
                // For quotes, we just scan the whole line and find which pair the cursor is in,
                // or if it's before a pair, pick that pair.
                let mut quote_indices = Vec::new();
                for i in line_start..line_end {
                    if chars[i] == open {
                        quote_indices.push(i);
                    }
                }
                
                for i in (0..quote_indices.len()).step_by(2) {
                    if i + 1 < quote_indices.len() {
                        let s = quote_indices[i];
                        let e = quote_indices[i+1];
                        if start_scan <= e {
                            return Some((s, e));
                        }
                    }
                }
                return None;
            }

            // Nesting pair logic (e.g. < >, ( ), { }, [ ])
            let mut depth = 0;
            let mut start_idx = None;
            for i in (0..=start_scan).rev() {
                if chars[i] == close && i != start_scan {
                    depth += 1;
                } else if chars[i] == open {
                    if depth == 0 {
                        start_idx = Some(i);
                        break;
                    } else {
                        depth -= 1;
                    }
                }
            }

            let start = start_idx?;
            depth = 0;
            for i in (start + 1)..chars.len() {
                if chars[i] == open {
                    depth += 1;
                } else if chars[i] == close {
                    if depth == 0 {
                        return Some((start, i));
                    } else {
                        depth -= 1;
                    }
                }
            }
            None
        };

        let mut pair = find_pair(cursor);
        if pair.is_none() {
            // Seek forward on current line for the open char.
            let line_end = chars[cursor..].iter().position(|&c| c == '\n').map(|p| cursor + p).unwrap_or(chars.len());
            let next_open = (cursor..line_end).find(|&i| chars[i] == open);
            if let Some(next) = next_open {
                pair = find_pair(next);
            }
        }

        let (start, end) = pair?;

        if around {
            Some(TextRange::new(start, end + 1))
        } else {
            Some(TextRange::new(start + 1, end))
        }
    }

    fn paragraph_text_object_range(&self, buffer: &mut TextBuffer, count: usize, around: bool) -> Option<TextRange> {
        let lines = self.lines_vec(buffer, );
        if lines.is_empty() {
            return None;
        }

        let mut start_line = self.cursor_line(buffer, ).min(lines.len().saturating_sub(1));
        while start_line > 0 && !lines[start_line - 1].trim().is_empty() {
            start_line -= 1;
        }

        let mut end_line = self.cursor_line(buffer, ).min(lines.len().saturating_sub(1));
        for paragraph_index in 0..count.max(1) {
            while end_line + 1 < lines.len() && !lines[end_line + 1].trim().is_empty() {
                end_line += 1;
            }
            if paragraph_index + 1 < count {
                while end_line + 1 < lines.len() && lines[end_line + 1].trim().is_empty() {
                    end_line += 1;
                }
                if end_line + 1 < lines.len() {
                    end_line += 1;
                }
            }
        }

        let mut start = self.line_start_flat(buffer, start_line);
        let mut end = self.line_end_flat_including_newline(buffer, end_line);
        if around {
            while end_line + 1 < lines.len() && lines[end_line + 1].trim().is_empty() {
                end_line += 1;
                end = self.line_end_flat_including_newline(buffer, end_line);
                break;
            }
            if end == self.content_char_len(buffer) {
                while start_line > 0 && lines[start_line - 1].trim().is_empty() {
                    start_line -= 1;
                    start = self.line_start_flat(buffer, start_line);
                    break;
                }
            }
        }

        Some(TextRange::linewise(start, end))
    }

    fn line_text_object_range(&self, buffer: &mut TextBuffer, count: usize) -> Option<TextRange> {
        if self.lines_vec(buffer, ).is_empty() {
            return None;
        }
        Some(
            self.linewise_range(buffer, self.cursor_line(buffer, ),
                self.cursor_line(buffer, )
                    .saturating_add(count.max(1).saturating_sub(1)),
            ),
        )
    }


    fn indent_range(&mut self, buffer: &mut TextBuffer, range: TextRange, delta: isize) -> bool {
        let start_line = self.line_for_flat(buffer, range.start);
        let end_offset = range.end.saturating_sub(1);
        let end_line = self.line_for_flat(buffer, end_offset);
        self.indent_lines(buffer, start_line, end_line, delta)
    }

    fn indent_lines(&mut self, buffer: &mut TextBuffer, start_line: usize, end_line: usize, delta: isize) -> bool {
        let mut lines = self.lines_vec(buffer, );
        if lines.is_empty() {
            return false;
        }

        let start_line = start_line.min(lines.len().saturating_sub(1));
        let end_line = end_line.min(lines.len().saturating_sub(1));
        let (start_line, end_line) = if start_line <= end_line {
            (start_line, end_line)
        } else {
            (end_line, start_line)
        };

        let mut changed = false;
        for line in &mut lines[start_line..=end_line] {
            if delta > 0 {
                line.insert_str(0, "  ");
                changed = true;
            } else if line.starts_with("  ") {
                line.drain(..2);
                changed = true;
            } else if line.starts_with('\t') || line.starts_with(' ') {
                line.drain(..1);
                changed = true;
            }
        }

        if !changed {
            return false;
        }

        self.replace_lines(buffer, lines);
        self.cursor_line = start_line;
        self.cursor_col = self.first_non_blank_col(buffer, );
        self.clamp_cursor_normal(buffer);
        true
    }

    fn format_range(&mut self, buffer: &mut TextBuffer, range: TextRange) -> bool {
        let start_line = self.line_for_flat(buffer, range.start);
        let end_line = self.line_for_flat(buffer, range.end.saturating_sub(1));
        let mut lines = self.lines_vec(buffer, );
        if lines.is_empty() {
            return false;
        }

        let start_line = start_line.min(lines.len().saturating_sub(1));
        let end_line = end_line.min(lines.len().saturating_sub(1));
        let mut changed = false;
        for line in &mut lines[start_line..=end_line] {
            let trimmed_len = line.trim_end_matches(|ch| ch == ' ' || ch == '\t').len();
            if trimmed_len != line.len() {
                line.truncate(trimmed_len);
                changed = true;
            }
        }

        if !changed {
            return false;
        }

        self.replace_lines(buffer, lines);
        true
    }

    fn toggle_case_chars(&mut self, buffer: &mut TextBuffer, count: usize) -> bool {
        let start = self.flattened_cursor(buffer, );
        let end = (start + count.max(1)).min(self.current_line_end_flat_exclusive(buffer, ));
        if start >= end {
            return false;
        }

        let range = TextRange::new(start, end);
        let replacement = self
            .text_for_range(buffer, range)
            .chars()
            .flat_map(|ch| {
                if ch.is_lowercase() {
                    ch.to_uppercase().collect::<Vec<_>>()
                } else if ch.is_uppercase() {
                    ch.to_lowercase().collect::<Vec<_>>()
                } else {
                    vec![ch]
                }
            })
            .collect::<String>();
        self.replace_flat_range(buffer, range, &replacement);
        self.set_cursor_from_flat(buffer, end.saturating_sub(1));
        true
    }
    fn toggle_case_range(&mut self, buffer: &mut TextBuffer, range: TextRange) -> bool {
        let range = range.normalized().clamped(self.content_char_len(buffer));
        if range.start >= range.end {
            return false;
        }

        let replacement = self
            .text_for_range(buffer, range)
            .chars()
            .flat_map(|ch| {
                if ch.is_lowercase() {
                    ch.to_uppercase().collect::<Vec<_>>()
                } else if ch.is_uppercase() {
                    ch.to_lowercase().collect::<Vec<_>>()
                } else {
                    vec![ch]
                }
            })
            .collect::<String>();
        self.replace_flat_range(buffer, range, &replacement);
        self.set_cursor_from_flat(buffer, range.start);
        true
    }

    fn increment_number(&mut self, _buffer: &mut TextBuffer, _delta: isize) -> bool {
        // Dummy implementation for now
        false
    }


    fn apply_case_range(&mut self, buffer: &mut TextBuffer, range: TextRange, upper: bool) -> bool {
        self.apply_case_range_kind(buffer, range, if upper { CaseKind::Upper } else { CaseKind::Lower })
    }

    fn apply_case_range_kind(&mut self, buffer: &mut TextBuffer, range: TextRange, kind: CaseKind) -> bool {
        let range = range.normalized().clamped(self.content_char_len(buffer));
        if range.start >= range.end {
            return false;
        }

        let replacement = self
            .text_for_range(buffer, range)
            .chars()
            .flat_map(|ch| match kind {
                CaseKind::Upper => ch.to_uppercase().collect::<Vec<_>>(),
                CaseKind::Lower => ch.to_lowercase().collect::<Vec<_>>(),
                CaseKind::Toggle => {
                    if ch.is_lowercase() {
                        ch.to_uppercase().collect::<Vec<_>>()
                    } else if ch.is_uppercase() {
                        ch.to_lowercase().collect::<Vec<_>>()
                    } else {
                        vec![ch]
                    }
                }
            })
            .collect::<String>();
        self.replace_flat_range(buffer, range, &replacement);
        self.set_cursor_from_flat(buffer, range.start);
        true
    }

    fn replace_flat_range(&mut self, buffer: &mut TextBuffer, range: TextRange, replacement: &str) {
        self.record_undo(buffer);
        self.push_change_location(buffer);
        let start_byte = byte_index_for_char(&buffer.content, range.start);
        let end_byte = byte_index_for_char(&buffer.content, range.end);
        buffer.content
            .replace_range(start_byte..end_byte, replacement);
        buffer.dirty = true;
    }

    fn delete_flat_range(&mut self, buffer: &mut TextBuffer, range: TextRange) {
        self.record_undo(buffer);
        self.push_change_location(buffer);
        remove_char_range(&mut buffer.content, range.start, range.end);
        buffer.dirty = true;
    }

    fn text_for_range(&self, buffer: &TextBuffer, range: TextRange) -> String {
        let range = range.normalized().clamped(self.content_char_len(buffer));
        buffer.content
            .chars()
            .skip(range.start)
            .take(range.end.saturating_sub(range.start))
            .collect()
    }


    fn paste_unnamed(&mut self, buffer: &mut TextBuffer, count: usize, placement: PastePlacement) -> bool {
        let target = self.take_register_target(buffer, );
        let register = buffer.registers.register(target).clone();
        if register.text.is_empty() {
            return false;
        }

        if register.blockwise {
            self.paste_blockwise(buffer, count.max(1), placement, register)
        } else if register.linewise {
            self.paste_linewise(buffer, count.max(1), placement, register)
        } else {
            self.paste_charwise(buffer, count.max(1), placement, register)
        }
    }

    fn paste_blockwise(
        &mut self, buffer: &mut TextBuffer,
        count: usize,
        placement: PastePlacement,
        register: RegisterValue,
    ) -> bool {
        let block_rows = register
            .text
            .split('\n')
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        if block_rows.is_empty() {
            return false;
        }

        let block_width = block_rows
            .iter()
            .map(|row| char_count(row))
            .max()
            .unwrap_or(0)
            .max(1);
        let start_line = self.cursor_line(buffer, );
        let start_col = match placement {
            PastePlacement::After | PastePlacement::AfterLeaveCursor => self.cursor_col(buffer, ).saturating_add(1),
            PastePlacement::Before | PastePlacement::BeforeLeaveCursor => self.cursor_col(buffer, ),
        };

        let mut lines = self.lines_vec(buffer, );
        self.record_undo(buffer);
        self.push_change_location(buffer);
        for repeat_index in 0..count {
            let col_offset = repeat_index * block_width;
            for (row_offset, row) in block_rows.iter().enumerate() {
                let target_line = start_line + row_offset;
                while lines.len() <= target_line {
                    lines.push(String::new());
                }
                let target_col = start_col + col_offset;
                pad_line_to_col(&mut lines[target_line], target_col);
                insert_str_at_char(&mut lines[target_line], target_col, row);
            }
        }

        buffer.content = lines.join("\n");
        buffer.dirty = true;
        self.cursor_line = start_line;
        self.cursor_col = start_col.min(self.current_line_max_col(buffer, ));
        self.clamp_cursor_normal(buffer);
        true
    }

    fn paste_linewise(
        &mut self, buffer: &mut TextBuffer,
        count: usize,
        placement: PastePlacement,
        register: RegisterValue,
    ) -> bool {
        let register_text = register.text;
        let paste_lines = register_text
            .trim_end_matches('\n')
            .split('\n')
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        if paste_lines.is_empty() {
            return false;
        }

        let mut lines = self.lines_vec(buffer, );
        let insert_at = match placement {
            PastePlacement::After | PastePlacement::AfterLeaveCursor => self.cursor_line(buffer, ).saturating_add(1).min(lines.len()),
            PastePlacement::Before | PastePlacement::BeforeLeaveCursor => self.cursor_line(buffer, ).min(lines.len()),
        };
        let mut offset = 0;
        for _ in 0..count {
            for line in &paste_lines {
                lines.insert(insert_at + offset, line.clone());
                offset += 1;
            }
        }

        self.replace_lines(buffer, lines);
        self.cursor_line = if matches!(placement, PastePlacement::AfterLeaveCursor | PastePlacement::BeforeLeaveCursor) {
            (insert_at + offset).min(self.line_count(buffer, ).saturating_sub(1))
        } else {
            insert_at.min(self.line_count(buffer, ).saturating_sub(1))
        };
        self.cursor_col = self.first_non_blank_col(buffer, );
        self.clamp_cursor_normal(buffer);
        true
    }

    fn paste_charwise(
        &mut self, buffer: &mut TextBuffer,
        count: usize,
        placement: PastePlacement,
        register: RegisterValue,
    ) -> bool {
        let text = register.text.repeat(count);
        let insert_at = match placement {
            PastePlacement::After | PastePlacement::AfterLeaveCursor => {
                (self.flattened_cursor(buffer, ) + 1).min(self.current_line_end_flat_exclusive(buffer, ))
            }
            PastePlacement::Before | PastePlacement::BeforeLeaveCursor => self.flattened_cursor(buffer, ),
        };
        let inserted_chars = text.chars().count();
        let byte_index = byte_index_for_char(&buffer.content, insert_at);
        self.record_undo(buffer);
        buffer.content.insert_str(byte_index, &text);
        buffer.dirty = true;
        
        let target_cursor = if matches!(placement, PastePlacement::AfterLeaveCursor | PastePlacement::BeforeLeaveCursor) {
            insert_at + inserted_chars
        } else {
            insert_at + inserted_chars.saturating_sub(1)
        };
        self.set_insert_cursor_from_flat(buffer, target_cursor);
        self.enter_normal(buffer, );
        true
    }

    fn content_char_len(&self, buffer: &TextBuffer) -> usize {
        buffer.content.chars().count()
    }

    fn current_line_start_flat(&self, buffer: &mut TextBuffer) -> usize {
        self.line_start_flat(buffer, self.cursor_line(buffer, ))
    }

    fn current_line_end_flat_exclusive(&self, buffer: &mut TextBuffer) -> usize {
        self.line_start_flat(buffer, self.cursor_line(buffer, )) + self.current_line_char_count(buffer, )
    }

    fn line_start_flat(&self, buffer: &TextBuffer, line: usize) -> usize {
        self.lines_vec(buffer, )
            .iter()
            .take(line)
            .map(|line| char_count(line) + 1)
            .sum()
    }

    fn line_for_flat(&self, buffer: &TextBuffer, mut offset: usize) -> usize {
        let lines = self.lines_vec(buffer, );
        for (line_index, line) in lines.iter().enumerate() {
            let len = char_count(line);
            if offset <= len {
                return line_index;
            }
            offset = offset.saturating_sub(len + 1);
        }
        lines.len().saturating_sub(1)
    }

    fn line_end_flat_including_newline(&self, buffer: &TextBuffer, line: usize) -> usize {
        let lines = self.lines_vec(buffer, );
        let line = line.min(lines.len().saturating_sub(1));
        let start = self.line_start_flat(buffer, line);
        let line_len = char_count(&lines[line]);
        if line + 1 < lines.len() {
            start + line_len + 1
        } else {
            start + line_len
        }
    }

    fn linewise_range(&self, buffer: &TextBuffer, first_line: usize, second_line: usize) -> TextRange {
        let start_line = first_line.min(second_line);
        let end_line = first_line.max(second_line);
        TextRange::linewise(
            self.line_start_flat(buffer, start_line),
            self.line_end_flat_including_newline(buffer, end_line),
        )
    }

    fn set_insert_cursor_from_flat(&mut self, buffer: &mut TextBuffer, mut offset: usize) {
        let lines = self.lines_vec(buffer, );
        for (line_index, line) in lines.iter().enumerate() {
            let len = char_count(line);
            if offset <= len {
                self.cursor_line = line_index;
                self.cursor_col = offset.min(len);
                return;
            }
            offset = offset.saturating_sub(len + 1);
        }
        self.cursor_line = lines.len().saturating_sub(1);
        self.cursor_col = self.current_line_char_count(buffer, );
    }

    pub fn normal_mode_line_with_cursor(&self, buffer: &TextBuffer, line: &str, line_index: usize) -> String {
        if self.cursor_line != line_index {
            return line.to_string();
        }

        if line.is_empty() {
            return "|".to_string();
        }

        let col = self.cursor_col(buffer, );
        let mut output = String::new();
        for (index, ch) in line.chars().enumerate() {
            if index == col {
                output.push('|');
            }
            output.push(ch);
        }
        if col >= line.chars().count() {
            output.push('|');
        }
        output
    }

    fn line_with_cursor_marker(&self, buffer: &TextBuffer, line: &str, line_index: usize) -> String {
        if self.vim_state.mode == VimMode::Normal {
            return self.normal_mode_line_with_cursor(buffer, line, line_index);
        }

        if line_index != self.cursor_line {
            return line.to_string();
        }

        let col = self.cursor_col.min(char_count(line));
        let mut output = String::new();
        for (index, ch) in line.chars().enumerate() {
            if index == col {
                output.push('|');
            }
            output.push(ch);
        }
        if col >= char_count(line) {
            output.push('|');
        }
        output
    }

    fn command_line_insert(&mut self, _buffer: &mut TextBuffer, text: &str) {
        let cursor = self.vim_state.command_line.cursor;
        insert_str_at_char(&mut self.vim_state.command_line.input, cursor, text);
        self.vim_state.command_line.cursor += text.chars().count();
    }

    fn command_line_backspace(&mut self, _buffer: &mut TextBuffer) {
        let cursor = self.vim_state.command_line.cursor;
        if cursor == 0 {
            return;
        }
        remove_char_at(&mut self.vim_state.command_line.input, cursor - 1);
        self.vim_state.command_line.cursor = cursor - 1;
    }

    fn command_line_delete(&mut self, _buffer: &mut TextBuffer) {
        let cursor = self.vim_state.command_line.cursor;
        remove_char_at(&mut self.vim_state.command_line.input, cursor);
    }

    fn command_line_delete_word(&mut self, _buffer: &mut TextBuffer) {
        let cursor = self.vim_state.command_line.cursor;
        if cursor == 0 {
            return;
        }
        let chars: Vec<char> = self.vim_state.command_line.input.chars().collect();
        let mut start = cursor;
        while start > 0 && chars[start - 1].is_whitespace() {
            start -= 1;
        }
        while start > 0 && !chars[start - 1].is_whitespace() {
            start -= 1;
        }
        remove_char_range(&mut self.vim_state.command_line.input, start, cursor);
        self.vim_state.command_line.cursor = start;
    }

    fn push_command_history(&mut self, buffer: &mut TextBuffer, value: String, is_search: bool) {
        if value.is_empty() {
            return;
        }
        let history = if is_search {
            &mut buffer.search_history
        } else {
            &mut self.command_history
        };
        if history.last() != Some(&value) {
            history.push(value);
        }
    }

    fn command_line_history_move(&mut self, buffer: &mut TextBuffer, delta: isize) {
        let is_search = self.vim_state.command_line.is_search;
        let history = if is_search {
            &buffer.search_history
        } else {
            &self.command_history
        };
        if history.is_empty() {
            return;
        }

        let current = self
            .vim_state
            .command_line
            .history_index
            .unwrap_or(history.len()) as isize;
        if self.vim_state.command_line.saved_current.is_none() {
            self.vim_state.command_line.saved_current =
                Some(self.vim_state.command_line.input.clone());
        }
        let next = (current + delta).clamp(0, history.len() as isize);
        if next == history.len() as isize {
            if let Some(saved) = self.vim_state.command_line.saved_current.clone() {
                self.vim_state.command_line.input = saved;
            }
            self.vim_state.command_line.history_index = None;
        } else {
            self.vim_state.command_line.input = history[next as usize].clone();
            self.vim_state.command_line.history_index = Some(next as usize);
        }
        self.vim_state.command_line.cursor = self.vim_state.command_line.input.chars().count();
    }

    fn update_incremental_search(&mut self, buffer: &mut TextBuffer, dir: SearchDirection) {
        if !buffer.settings.incsearch {
            return;
        }
        buffer.search.pattern = self.vim_state.command_line.input.clone();
        buffer.search.reverse = dir == SearchDirection::Backward;
        self.refresh_search_matches(buffer, );
    }

    fn handle_command_mode_input(&mut self, buffer: &mut TextBuffer, input: &str) -> bool {
        match input {
            "escape" | "ctrl+[" => {
                self.enter_normal(buffer, );
                false
            }
            "backspace" => {
                if self.vim_state.command_line.input.is_empty() {
                    self.enter_normal(buffer, );
                } else {
                    self.command_line_backspace(buffer, );
                }
                false
            }
            "delete" => {
                self.command_line_delete(buffer, );
                false
            }
            "left" => {
                self.vim_state.command_line.cursor =
                    self.vim_state.command_line.cursor.saturating_sub(1);
                false
            }
            "right" => {
                self.vim_state.command_line.cursor = (self.vim_state.command_line.cursor + 1)
                    .min(self.vim_state.command_line.input.chars().count());
                false
            }
            "home" | "ctrl+b" => {
                self.vim_state.command_line.cursor = 0;
                false
            }
            "end" | "ctrl+e" => {
                self.vim_state.command_line.cursor = self.vim_state.command_line.input.chars().count();
                false
            }
            "ctrl+u" => {
                self.vim_state.command_line.input.clear();
                self.vim_state.command_line.cursor = 0;
                false
            }
            "ctrl+w" => {
                self.command_line_delete_word(buffer, );
                false
            }
            "up" => {
                self.command_line_history_move(buffer, -1);
                false
            }
            "down" => {
                self.command_line_history_move(buffer, 1);
                false
            }
            "return" => {
                let cmd = self.vim_state.command_line.input.clone();
                self.push_command_history(buffer, cmd.clone(), false);
                self.enter_normal(buffer, );
                self.execute_ex_command(buffer, &cmd)
            }
            other => {
                let is_ignored = matches!(
                    other,
                    "shift"
                        | "control"
                        | "alt"
                        | "meta"
                        | "capslock"
                        | "tab"
                        | "insert"
                        | "pageup"
                        | "pagedown"
                ) || (other.starts_with('f')
                    && other.len() > 1
                    && other[1..].chars().all(|c| c.is_ascii_digit()));

                if !is_ignored {
                    self.command_line_insert(buffer, other);
                    if self.vim_state.command_line.input == "noh" || self.vim_state.command_line.input == "nohlsearch" {
                        let cmd = self.vim_state.command_line.input.clone();
                        self.push_command_history(buffer, cmd.clone(), false);
                        self.enter_normal(buffer, );
                        return self.execute_ex_command(buffer, &cmd);
                    }
                }
                false
            }
        }
    }

    fn handle_search_mode_input(&mut self, buffer: &mut TextBuffer, dir: SearchDirection, input: &str) -> bool {
        match input {
            "escape" | "ctrl+[" => {
                self.enter_normal(buffer, );
                false
            }
            "backspace" => {
                if self.vim_state.command_line.input.is_empty() {
                    self.enter_normal(buffer, );
                } else {
                    self.command_line_backspace(buffer, );
                }
                self.update_incremental_search(buffer, dir);
                false
            }
            "delete" => {
                self.command_line_delete(buffer, );
                self.update_incremental_search(buffer, dir);
                false
            }
            "left" => {
                self.vim_state.command_line.cursor =
                    self.vim_state.command_line.cursor.saturating_sub(1);
                false
            }
            "right" => {
                self.vim_state.command_line.cursor = (self.vim_state.command_line.cursor + 1)
                    .min(self.vim_state.command_line.input.chars().count());
                false
            }
            "home" | "ctrl+b" => {
                self.vim_state.command_line.cursor = 0;
                false
            }
            "end" | "ctrl+e" => {
                self.vim_state.command_line.cursor = self.vim_state.command_line.input.chars().count();
                false
            }
            "ctrl+u" => {
                self.vim_state.command_line.input.clear();
                self.vim_state.command_line.cursor = 0;
                self.update_incremental_search(buffer, dir);
                false
            }
            "ctrl+w" => {
                self.command_line_delete_word(buffer, );
                self.update_incremental_search(buffer, dir);
                false
            }
            "up" => {
                self.command_line_history_move(buffer, -1);
                false
            }
            "down" => {
                self.command_line_history_move(buffer, 1);
                false
            }
            "return" => {
                let query = self.vim_state.command_line.input.clone();
                self.push_command_history(buffer, query.clone(), true);
                self.enter_normal(buffer, );
                if !query.is_empty() {
                    buffer.search.pattern = query;
                    buffer.search.reverse = dir == SearchDirection::Backward;
                    self.refresh_search_matches(buffer, );
                    self.repeat_search(buffer, false)
                } else {
                    false
                }
            }
            other => {
                let is_ignored = matches!(
                    other,
                    "shift"
                        | "control"
                        | "alt"
                        | "meta"
                        | "capslock"
                        | "tab"
                        | "insert"
                        | "pageup"
                        | "pagedown"
                ) || (other.starts_with('f')
                    && other.len() > 1
                    && other[1..].chars().all(|c| c.is_ascii_digit()));

                if !is_ignored {
                    self.command_line_insert(buffer, other);
                    self.update_incremental_search(buffer, dir);
                }
                false
            }
        }
    }

    fn find_substitution_matches(
        &self, buffer: &mut TextBuffer,
        start_line: usize,
        end_line: usize,
        pattern: &str,
        flags: &str,
    ) -> Vec<TextRange> {
        let mut matches = Vec::new();
        if pattern.is_empty() {
            return matches;
        }

        let is_case_insensitive = flags.contains('i') || (flags.is_empty() && self.should_ignore_case(buffer, pattern));
        let is_global_on_line = flags.contains('g');

        let lines = self.lines_vec(buffer, );
        let pat_len = pattern.chars().count();
        let pat_chars: Vec<char> = pattern.chars().collect();

        let mut current_flat_offset = 0;
        for line_idx in 0..lines.len() {
            let line = &lines[line_idx];
            let line_len = line.chars().count();

            if line_idx >= start_line && line_idx <= end_line {
                let line_chars: Vec<char> = line.chars().collect();

                let mut col = 0;
                while col + pat_len <= line_len {
                    if chars_equal_at(&line_chars[col..col + pat_len], &pat_chars, is_case_insensitive) {
                        matches.push(TextRange::new(
                            current_flat_offset + col,
                            current_flat_offset + col + pat_len,
                        ));
                        if !is_global_on_line {
                            break;
                        }
                        col += pat_len;
                    } else {
                        col += 1;
                    }
                }
            }

            current_flat_offset += line_len + 1; // +1 for the '\n'
        }

        matches
    }

    fn execute_substitution_direct(
        &mut self, buffer: &mut TextBuffer,
        start_line: usize,
        end_line: usize,
        pattern: &str,
        replacement: &str,
        flags: &str,
    ) -> bool {
        let matches = self.find_substitution_matches(buffer, start_line, end_line, pattern, flags);
        if matches.is_empty() {
            return false;
        }

        self.record_undo(buffer);
        self.push_change_location(buffer);
        let mut content_chars: Vec<char> = buffer.content.chars().collect();
        for m in matches.iter().rev() {
            let prefix: Vec<char> = content_chars[..m.start].to_vec();
            let suffix: Vec<char> = content_chars[m.end..].to_vec();
            let replacement_chars: Vec<char> = replacement.chars().collect();
            content_chars = [prefix, replacement_chars, suffix].concat();
        }

        buffer.content = content_chars.into_iter().collect();
        buffer.dirty = true;

        buffer.search.pattern = pattern.to_string();
        self.refresh_search_matches(buffer, );
        self.last_substitute = Some(LastSubstitute {
            range: format!("{},{}", start_line + 1, end_line + 1),
            pattern: pattern.to_string(),
            replacement: replacement.to_string(),
            flags: flags.to_string(),
        });

        true
    }

    fn execute_substitution(
        &mut self, buffer: &mut TextBuffer,
        start_line: usize,
        end_line: usize,
        pattern: &str,
        replacement: &str,
        flags: &str,
    ) -> bool {
        let matches = self.find_substitution_matches(buffer, start_line, end_line, pattern, flags);
        if matches.is_empty() {
            return false;
        }

        if flags.contains('c') {
            let first_match = matches[0];
            self.yank_highlight = Some(first_match);
            self.set_cursor_from_flat(buffer, first_match.start);
            self.vim_state.pending_command = Some(PendingCommand::SubstituteConfirm {
                pattern: pattern.to_string(),
                replacement: replacement.to_string(),
                matches,
                match_index: 0,
                flags: flags.to_string(),
            });
            self.last_substitute = Some(LastSubstitute {
                range: format!("{},{}", start_line + 1, end_line + 1),
                pattern: pattern.to_string(),
                replacement: replacement.to_string(),
                flags: flags.to_string(),
            });
            false
        } else {
            self.execute_substitution_direct(buffer, start_line, end_line, pattern, replacement, flags)
        }
    }

    pub fn take_ex_action(&mut self, _buffer: &mut TextBuffer) -> Option<ExCommandAction> {
        self.pending_ex_action.take()
    }

    fn resolve_ex_line_atom(&self, buffer: &TextBuffer, atom: &str) -> Option<usize> {
        let atom = atom.trim();
        if atom.is_empty() {
            return Some(self.cursor_line(buffer, ));
        }
        if atom == "." {
            return Some(self.cursor_line(buffer, ));
        }
        if atom == "$" {
            return Some(self.line_count(buffer, ).saturating_sub(1));
        }
        if atom == "'<" {
            return self.last_visual_selection.map(|(r, _)| self.line_for_flat(buffer, r.start));
        }
        if atom == "'>" {
            return self
                .last_visual_selection
                .map(|(r, _)| self.line_for_flat(buffer, r.end.saturating_sub(1)));
        }
        if let Some((base, offset)) = atom.split_once('+') {
            let base_line = self.resolve_ex_line_atom(buffer, base)?;
            let delta = offset.parse::<usize>().ok()?;
            return Some((base_line + delta).min(self.line_count(buffer, ).saturating_sub(1)));
        }
        atom.parse::<usize>().ok().map(|n| n.saturating_sub(1))
    }

    fn resolve_ex_range(&self, buffer: &mut TextBuffer, range: &str) -> (usize, usize) {
        match range.trim() {
            "" => (self.cursor_line(buffer, ), self.cursor_line(buffer, )),
            "%" => (0, self.line_count(buffer, ).saturating_sub(1)),
            "'<,'>" => {
                if let Some((r, _)) = self.last_visual_selection {
                    let start = self.line_for_flat(buffer, r.start);
                    let end = self.line_for_flat(buffer, r.end.saturating_sub(1));
                    (start.min(end), start.max(end))
                } else {
                    (self.cursor_line(buffer, ), self.cursor_line(buffer, ))
                }
            }
            other => {
                if let Some((start_str, end_str)) = other.split_once(',') {
                    let start = self
                        .resolve_ex_line_atom(buffer, start_str)
                        .unwrap_or(self.cursor_line(buffer, ));
                    let end = self.resolve_ex_line_atom(buffer, end_str).unwrap_or(start);
                    (start.min(end), start.max(end))
                } else {
                    let line = self.resolve_ex_line_atom(buffer, other).unwrap_or(self.cursor_line(buffer, ));
                    (line, line)
                }
            }
        }
    }

    fn execute_ex_command(&mut self, buffer: &mut TextBuffer, command: &str) -> bool {
        self.vim_state.pending_command = None;
        match parse_ex_command(command) {
            Ok(ExCommand::None) => false,
            Ok(ExCommand::Save(path)) => {
                self.pending_ex_action = Some(ExCommandAction::Save(path));
                false
            }
            Ok(ExCommand::Quit { force }) => {
                self.pending_ex_action = Some(ExCommandAction::Quit { force });
                false
            }
            Ok(ExCommand::SaveAndQuit) => {
                self.pending_ex_action = Some(ExCommandAction::SaveAndQuit);
                false
            }
            Ok(ExCommand::Edit(path)) => {
                self.pending_ex_action = Some(ExCommandAction::Edit(path));
                false
            }
            Ok(ExCommand::EditNew) => {
                self.pending_ex_action = Some(ExCommandAction::EditNew);
                false
            }
            Ok(ExCommand::LineJump(line_num)) => {
                let target = line_num.clamp(1, self.line_count(buffer, ));
                self.cursor_line = target - 1;
                self.cursor_col = 0;
                self.clamp_cursor_normal(buffer);
                false
            }
            Ok(ExCommand::RepeatLastSubstitute { keep_flags }) => {
                let Some(last) = self.last_substitute.clone() else {
                    return false;
                };
                let flags = if keep_flags {
                    last.flags
                } else {
                    String::new()
                };
                let (start_line, end_line) = self.resolve_ex_range(buffer, "");
                self.execute_substitution(buffer, start_line, end_line, &last.pattern, &last.replacement, &flags)
            }
            Ok(ExCommand::Substitute { range, pattern, replacement, flags }) => {
                let (start_line, end_line) = self.resolve_ex_range(buffer, &range);
                let pattern = if pattern.is_empty() {
                    self.last_substitute
                        .as_ref()
                        .map(|last| last.pattern.clone())
                        .or_else(|| self.search_pattern(buffer, ).map(ToOwned::to_owned))
                        .unwrap_or_default()
                } else {
                    pattern
                };
                self.execute_substitution(buffer, start_line, end_line, &pattern, &replacement, &flags)
            }
            Ok(ExCommand::SetOption(key, val)) => {
                match key.as_str() {
                    "number" | "nu" => buffer.settings.number = val == "true",
                    "relativenumber" | "rnu" => buffer.settings.relativenumber = val == "true",
                    "wrap" => buffer.settings.wrap = val == "true",
                    "ignorecase" | "ic" => buffer.settings.ignorecase = val == "true",
                    "smartcase" | "scs" => buffer.settings.smartcase = val == "true",
                    "hlsearch" | "hls" => buffer.settings.hlsearch = val == "true",
                    "incsearch" | "is" => buffer.settings.incsearch = val == "true",
                    "tabstop" | "ts" => {
                        if let Ok(n) = val.parse::<usize>() {
                            buffer.settings.tabstop = n.max(1);
                        }
                    }
                    "shiftwidth" | "sw" => {
                        if let Ok(n) = val.parse::<usize>() {
                            buffer.settings.shiftwidth = n.max(1);
                        }
                    }
                    _ => {}
                }
                true // requires UI update
            }
            Ok(ExCommand::SaveAll) => {
                self.pending_ex_action = Some(ExCommandAction::SaveAll);
                false
            }
            Ok(ExCommand::NoHLSearch) => {
                buffer.search.highlights_active = false;
                false
            }
            Ok(ExCommand::Registers(filter)) => {
                self.pending_ex_action =
                    Some(ExCommandAction::ShowMessage(self.format_registers(buffer, filter)));
                false
            }
            Ok(ExCommand::Marks) => {
                self.pending_ex_action = Some(ExCommandAction::ShowMessage(self.format_marks(buffer, )));
                false
            }
            Ok(ExCommand::Jumps) => {
                self.pending_ex_action = Some(ExCommandAction::ShowMessage(self.format_jumps(buffer, )));
                false
            }
            Ok(ExCommand::Changes) => {
                self.pending_ex_action = Some(ExCommandAction::ShowMessage(self.format_changes(buffer, )));
                false
            }
            Err(_) => false,
        }
    }

    fn format_registers(&self, buffer: &mut TextBuffer, filter: Option<Vec<char>>) -> String {
        let mut rows = Vec::new();
        let mut push_row = |name: char, value: &RegisterValue| {
            if value.text.is_empty() {
                return;
            }
            let kind = if value.blockwise {
                'b'
            } else if value.linewise {
                'l'
            } else {
                'c'
            };
            rows.push(format!("{kind} {name} {}", value.text.replace('\n', "\\n")));
        };

        let allow = |name: char, filter: &Option<Vec<char>>| match filter {
            Some(entries) => entries.contains(&name),
            None => true,
        };

        if allow('"', &filter) {
            push_row('"', &buffer.registers.unnamed);
        }
        if allow('0', &filter) {
            if let Some(value) = buffer.registers.named.get(&'0') {
                push_row('0', value);
            }
        }
        if allow('-', &filter) {
            if let Some(value) = buffer.registers.named.get(&'-') {
                push_row('-', value);
            }
        }
        for (name, value) in &buffer.registers.named {
            if *name == '0' || *name == '-' || !allow(*name, &filter) {
                continue;
            }
            push_row(*name, value);
        }
        if allow('+', &filter) {
            push_row('+', &buffer.registers.clipboard);
        }
        if rows.is_empty() {
            "No registers".to_string()
        } else {
            rows.join(" | ")
        }
    }

    fn format_marks(&self, buffer: &mut TextBuffer) -> String {
        if buffer.marks.is_empty() {
            return "No marks".to_string();
        }
        buffer.marks
            .iter()
            .map(|(name, pos)| format!("{name}:{}:{}", pos.line + 1, pos.col + 1))
            .collect::<Vec<_>>()
            .join(" | ")
    }

    fn format_jumps(&self, _buffer: &mut TextBuffer) -> String {
        if self.jump_list.is_empty() {
            return "No jumps".to_string();
        }
        self.jump_list
            .iter()
            .enumerate()
            .map(|(index, pos)| format!("{index}:{}:{}", pos.line + 1, pos.col + 1))
            .collect::<Vec<_>>()
            .join(" | ")
    }

    fn format_changes(&self, buffer: &mut TextBuffer) -> String {
        if buffer.changelist.is_empty() {
            return "No changes".to_string();
        }
        buffer.changelist
            .iter()
            .enumerate()
            .map(|(index, pos)| format!("{index}:{}:{}", pos.line + 1, pos.col + 1))
            .collect::<Vec<_>>()
            .join(" | ")
    }

    fn start_macro_recording(&mut self, _buffer: &mut TextBuffer, register: char) {
        self.vim_state.recording_macro = Some(register);
        self.vim_state.macros.entry(register).or_default().clear();
    }

    fn stop_macro_recording(&mut self, _buffer: &mut TextBuffer) {
        self.vim_state.recording_macro = None;
    }

    fn play_macro(&mut self, buffer: &mut TextBuffer, register: char, count: usize) -> bool {
        if self.macro_depth > 100 {
            return false; // prevent infinite recursion
        }
        self.macro_depth += 1;
        self.vim_state.last_played_macro = Some(register);
        
        let mut changed = false;
        let keys = self.vim_state.macros.get(&register).cloned().unwrap_or_default();
        let actual_count = count.max(1);
        
        for _ in 0..actual_count {
            for key in &keys {
                changed |= self.handle_editor_key(buffer, key.clone());
            }
        }
        
        self.macro_depth -= 1;
        changed
    }

    fn last_played_macro(&self, _buffer: &TextBuffer) -> Option<char> {
        self.vim_state.last_played_macro
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SearchDirection {
    Forward,
    Backward,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum VimMode {
    #[default]
    Normal,
    Insert,
    Visual,
    VisualLine,
    VisualBlock,
    Command,
    Search(SearchDirection),
    Replace,
}

impl VimMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "NORMAL",
            Self::Insert => "INSERT",
            Self::Visual => "VISUAL",
            Self::VisualLine => "V-LINE",
            Self::VisualBlock => "V-BLOCK",
            Self::Command => "COMMAND",
            Self::Search(SearchDirection::Forward) => "/",
            Self::Search(SearchDirection::Backward) => "?",
            Self::Replace => "REPLACE",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CommandLineState {
    pub input: String,
    pub cursor: usize,
    pub history_index: Option<usize>,
    pub saved_current: Option<String>,
    pub is_search: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct VimState {
    pub mode: VimMode,
    pub command_line: CommandLineState,
    pub pending_command: Option<PendingCommand>,
    pub last_insert_pos: Option<usize>,
    pub insert_start_pos: Option<usize>,
    pub macro_insert_start_index: Option<usize>,
    pub current_macro: Vec<String>,
    pub macros: std::collections::HashMap<char, Vec<crate::vim::key::EditorKey>>,
    pub recording_macro: Option<char>,
    pub last_played_macro: Option<char>,
    pub last_find: Option<(char, bool, bool)>, // (char, is_f_or_t, is_forward)
    pub insert_pending: Option<InsertPending>,
    pub return_to_insert: bool,
}

impl VimState {
    pub fn pending_command_line(&self) -> String {
        self.command_line.input.clone()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaseKind {
    Lower,
    Upper,
    Toggle,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PendingCommand {
    RegisterPrefix,
    MarkSet,
    MarkJump,
    MarkJumpExact,
    Operator {
        operator: Operator,
        count: usize,
    },
    TextObject {
        operator: Operator,
        count: usize,
        around: bool,
    },
    VisualTextObject {
        around: bool,
    },
    OperatorOrCaseGoto {
        operator: Operator,
        count: usize,
    },
    FindChar {
        is_t: bool,
        is_forward: bool,
        count: usize,
    },
    OperatorThenFindChar {
        operator: Operator,
        operator_count: usize,
        is_t: bool,
        is_forward: bool,
        find_count: usize,
    },
    CaseOperator {
        kind: CaseKind,
        count: usize,
    },
    Goto {
        count: Option<usize>,
    },
    BracketPrefix {
        is_right: bool,
        count: Option<usize>,
    },
    ZPrefix,
    SmallZPrefix,
    ReplaceChar {
        count: usize,
    },
    SubstituteConfirm {
        pattern: String,
        replacement: String,
        matches: Vec<TextRange>,
        match_index: usize,
        flags: String,
    },
    MacroRecordPrefix,
    MacroReplayPrefix {
        count: usize,
    },
    SurroundWaitAdd {
        range: TextRange,
        buffer: String,
    },
    SurroundWaitDelete {
        buffer: String,
    },
    SurroundWaitChange {
        buffer: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Operator {
    Delete,
    Change,
    Yank,
    Indent,
    Outdent,
    Format,
    FormatLeaveCursor,
    ToggleCase,
    Filter,
    BlockInsert,
    BlockAppend,
    SurroundAdd,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExCommandAction {
    Save(Option<String>),
    SaveAll,
    Quit { force: bool },
    SaveAndQuit,
    Edit(String),
    EditNew,
    ShowMessage(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ExCommand {
    None,
    Save(Option<String>),
    SaveAll,
    Quit { force: bool },
    SaveAndQuit,
    Edit(String),
    EditNew,
    LineJump(usize),
    RepeatLastSubstitute {
        keep_flags: bool,
    },
    Substitute {
        range: String,
        pattern: String,
        replacement: String,
        flags: String,
    },
    SetOption(String, String),
    NoHLSearch,
    Registers(Option<Vec<char>>),
    Marks,
    Jumps,
    Changes,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InsertPending {
    RegisterPaste,
    DigraphPrefix,
    LiteralPrefix,
    DigraphFirstChar(char),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LastSubstitute {
    range: String,
    pattern: String,
    replacement: String,
    flags: String,
}

fn split_unescaped(s: &str, delim: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(&next_ch) = chars.peek() {
                if next_ch == delim {
                    current.push('\\');
                    current.push(delim);
                    chars.next();
                    continue;
                }
            }
        }
        if ch == delim {
            parts.push(current);
            current = String::new();
        } else {
            current.push(ch);
        }
    }
    parts.push(current);
    parts
}

fn parse_ex_command(input: &str) -> Result<ExCommand, String> {
    let input = input.trim();
    if input.is_empty() {
        return Ok(ExCommand::None);
    }

    if let Ok(line_num) = input.parse::<usize>() {
        return Ok(ExCommand::LineJump(line_num));
    }

    let mut s_idx = None;
    for (i, ch) in input.char_indices() {
        if ch == 's' && input[i..].starts_with("s/") {
            s_idx = Some(i);
            break;
        }
        if !ch.is_ascii_digit() && !matches!(ch, '%' | '\'' | '<' | '>' | ',' | '.' | '$' | ' ') {
            break;
        }
    }

    if input == "&" {
        return Ok(ExCommand::RepeatLastSubstitute { keep_flags: false });
    } else if input == "&&" {
        return Ok(ExCommand::RepeatLastSubstitute { keep_flags: true });
    }

    if let Some(idx) = s_idx {
        let range_str = input[..idx].trim().to_string();
        let rest = &input[idx + 1..];
        if rest.starts_with('/') {
            let parts = split_unescaped(rest, '/');
            if parts.len() >= 3 {
                let pattern = parts[1].replace("\\/", "/");
                let replacement = parts[2].replace("\\/", "/");
                let flags = if parts.len() > 3 { parts[3].clone() } else { String::new() };
                return Ok(ExCommand::Substitute {
                    range: range_str,
                    pattern,
                    replacement,
                    flags,
                });
            }
        }
    }

    if input == "w" {
        return Ok(ExCommand::Save(None));
    } else if input.starts_with("w ") {
        let path = input[2..].trim().to_string();
        return Ok(ExCommand::Save(Some(path)));
    } else if input == "wa" || input == "wall" {
        return Ok(ExCommand::SaveAll);
    } else if input == "q" {
        return Ok(ExCommand::Quit { force: false });
    } else if input == "q!" {
        return Ok(ExCommand::Quit { force: true });
    } else if input == "wq" {
        return Ok(ExCommand::SaveAndQuit);
    } else if input.starts_with("e ") {
        let path = input[2..].trim().to_string();
        return Ok(ExCommand::Edit(path));
    } else if input == "enew" {
        return Ok(ExCommand::EditNew);
    } else if input == "noh" || input == "nohlsearch" {
        return Ok(ExCommand::NoHLSearch);
    } else if input == "reg" || input == "registers" {
        return Ok(ExCommand::Registers(None));
    } else if input.starts_with("reg ") || input.starts_with("registers ") {
        let names = input
            .split_once(' ')
            .map(|(_, rest)| rest.split_whitespace().filter_map(|part| part.chars().next()).collect::<Vec<_>>())
            .unwrap_or_default();
        return Ok(ExCommand::Registers(Some(names)));
    } else if input == "marks" {
        return Ok(ExCommand::Marks);
    } else if input == "jumps" {
        return Ok(ExCommand::Jumps);
    } else if input == "changes" {
        return Ok(ExCommand::Changes);
    } else if input.starts_with("set ") {
        let arg = input[4..].trim();
        let parts: Vec<&str> = arg.split('=').collect();
        if parts.len() == 2 {
            return Ok(ExCommand::SetOption(parts[0].trim().to_string(), parts[1].trim().to_string()));
        } else if parts.len() == 1 {
            let option = parts[0].trim();
            if option.starts_with("no") {
                return Ok(ExCommand::SetOption(option[2..].to_string(), "false".to_string()));
            } else {
                return Ok(ExCommand::SetOption(option.to_string(), "true".to_string()));
            }
        }
    }

    Err(format!("Not an editor command: {}", input))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TextObject {
    Word { big_word: bool },
    Delimited(char),
    Paragraph,
    Line,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RegisterTarget {
    Unnamed,
    Named(char),
    NamedAppend(char),
    Clipboard,
    BlackHole,
}

impl RegisterTarget {
    fn from_prefix(ch: char) -> Option<Self> {
        match ch {
            '+' => Some(Self::Clipboard),
            '_' => Some(Self::BlackHole),
            '"' => Some(Self::Unnamed),
            name if name.is_ascii_uppercase() => Some(Self::NamedAppend(name.to_ascii_lowercase())),
            name if name.is_ascii_alphabetic() => Some(Self::Named(name.to_ascii_lowercase())),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DocumentSnapshot {
    content: String,
    mode: VimMode,
    cursor_line: usize,
    cursor_col: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CursorPosition {
    line: usize,
    col: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VisualKind {
    Character,
    Line,
    Block,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct SearchState {
    pattern: String,
    reverse: bool,
    matches: Vec<TextRange>,
    highlights_active: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PastePlacement {
    After,
    Before,
    AfterLeaveCursor,
    BeforeLeaveCursor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DeferredAction {
    operator: Operator,
    range: TextRange,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
    pub linewise: bool,
    pub blockwise: bool,
}

impl TextRange {
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start,
            end,
            linewise: false,
            blockwise: false,
        }
    }

    pub fn linewise(start: usize, end: usize) -> Self {
        Self {
            start,
            end,
            linewise: true,
            blockwise: false,
        }
    }

    pub fn blockwise(start: usize, end: usize) -> Self {
        Self {
            start,
            end,
            linewise: false,
            blockwise: true,
        }
    }

    fn normalized(self) -> Self {
        if self.start <= self.end {
            self
        } else {
            Self {
                start: self.end,
                end: self.start,
                linewise: self.linewise,
                blockwise: self.blockwise,
            }
        }
    }

    fn clamped(self, len: usize) -> Self {
        Self {
            start: self.start.min(len),
            end: self.end.min(len),
            linewise: self.linewise,
            blockwise: self.blockwise,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Registers {
    unnamed: RegisterValue,
    yank: RegisterValue,
    clipboard: RegisterValue,
    named: BTreeMap<char, RegisterValue>,
    version: usize,
}

impl Registers {
    fn register(&self, target: RegisterTarget) -> &RegisterValue {
        match target {
            RegisterTarget::Unnamed | RegisterTarget::BlackHole => &self.unnamed,
            RegisterTarget::Clipboard => &self.clipboard,
            RegisterTarget::Named(name) | RegisterTarget::NamedAppend(name) => {
                self.named.get(&name).unwrap_or(&self.unnamed)
            }
        }
    }

    fn store_yank(&mut self, target: RegisterTarget, text: String, linewise: bool) {
        if target == RegisterTarget::BlackHole {
            return;
        }
        let mut text = text;
        if linewise && !text.ends_with('\n') {
            text.push('\n');
        }
        let value = RegisterValue { text, linewise, blockwise: false };
        
        if target == RegisterTarget::Unnamed {
            self.named.insert('0', value.clone());
        }
        
        self.store_target(target, value.clone());
        self.yank = value;
    }

    fn store_deleted(&mut self, target: RegisterTarget, text: String, linewise: bool) {
        if target == RegisterTarget::BlackHole {
            return;
        }
        let mut text = text;
        if linewise && !text.ends_with('\n') {
            text.push('\n');
        }
        let value = RegisterValue { text: text.clone(), linewise, blockwise: false };

        if target == RegisterTarget::Unnamed {
            let is_small = !linewise && !text.contains('\n');
            if is_small {
                self.named.insert('-', value.clone());
            } else {
                for i in (1..9).rev() {
                    let from_key = char::from_digit(i as u32, 10).unwrap();
                    let to_key = char::from_digit((i + 1) as u32, 10).unwrap();
                    if let Some(val) = self.named.get(&from_key).cloned() {
                        self.named.insert(to_key, val);
                    }
                }
                self.named.insert('1', value.clone());
            }
        }

        self.store_target(target, value);
    }

    fn store_target(&mut self, target: RegisterTarget, value: RegisterValue) {
        self.version = self.version.wrapping_add(1);
        match target {
            RegisterTarget::Unnamed => self.unnamed = value,
            RegisterTarget::Clipboard => {
                self.unnamed = value.clone();
                self.clipboard = value;
            }
            RegisterTarget::Named(name) => {
                self.unnamed = value.clone();
                self.named.insert(name, value);
            }
            RegisterTarget::NamedAppend(name) => {
                let combined = if let Some(existing) = self.named.get(&name) {
                    let mut text = existing.text.clone();
                    text.push_str(&value.text);
                    RegisterValue {
                        text,
                        linewise: existing.linewise || value.linewise,
                        blockwise: existing.blockwise || value.blockwise,
                    }
                } else {
                    value
                };
                self.unnamed = combined.clone();
                self.named.insert(name, combined);
            }
            RegisterTarget::BlackHole => {}
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct RegisterValue {
    text: String,
    linewise: bool,
    blockwise: bool,
}

fn decode_text_file(bytes: &[u8]) -> std::io::Result<(String, TextEncoding)> {
    if let Some(utf8) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        let text = String::from_utf8(utf8.to_vec())
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        return Ok((text, TextEncoding::Utf8Bom));
    }

    if let Some(utf16) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return decode_utf16_bytes(utf16, true).map(|text| (text, TextEncoding::Utf16Le));
    }

    if let Some(utf16) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return decode_utf16_bytes(utf16, false).map(|text| (text, TextEncoding::Utf16Be));
    }

    let text = String::from_utf8(bytes.to_vec())
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    Ok((text, TextEncoding::Utf8))
}

fn decode_utf16_bytes(bytes: &[u8], little_endian: bool) -> std::io::Result<String> {
    if bytes.len() % 2 != 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "UTF-16 text had an odd number of bytes",
        ));
    }

    let units = bytes
        .chunks_exact(2)
        .map(|chunk| {
            if little_endian {
                u16::from_le_bytes([chunk[0], chunk[1]])
            } else {
                u16::from_be_bytes([chunk[0], chunk[1]])
            }
        })
        .collect::<Vec<_>>();

    String::from_utf16(&units)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

fn encode_text_file(content: &str, encoding: TextEncoding) -> Vec<u8> {
    match encoding {
        TextEncoding::Utf8 => content.as_bytes().to_vec(),
        TextEncoding::Utf8Bom => {
            let mut bytes = vec![0xEF, 0xBB, 0xBF];
            bytes.extend_from_slice(content.as_bytes());
            bytes
        }
        TextEncoding::Utf16Le => {
            let mut bytes = vec![0xFF, 0xFE];
            for unit in content.encode_utf16() {
                bytes.extend_from_slice(&unit.to_le_bytes());
            }
            bytes
        }
        TextEncoding::Utf16Be => {
            let mut bytes = vec![0xFE, 0xFF];
            for unit in content.encode_utf16() {
                bytes.extend_from_slice(&unit.to_be_bytes());
            }
            bytes
        }
    }
}

fn chars_equal_at(window: &[char], pattern: &[char], ignore_case: bool) -> bool {
    if !ignore_case {
        return window == pattern;
    }

    window
        .iter()
        .flat_map(|ch| ch.to_lowercase())
        .eq(pattern.iter().flat_map(|ch| ch.to_lowercase()))
}

fn char_count(line: &str) -> usize {
    line.chars().count()
}

fn byte_index_for_char(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(text.len())
}

fn insert_str_at_char(text: &mut String, char_index: usize, value: &str) {
    let byte_index = byte_index_for_char(text, char_index);
    text.insert_str(byte_index, value);
}

fn split_off_at_char(text: &mut String, char_index: usize) -> String {
    let byte_index = byte_index_for_char(text, char_index);
    text.split_off(byte_index)
}

fn remove_char_at(text: &mut String, char_index: usize) {
    let byte_index = byte_index_for_char(text, char_index);
    if byte_index < text.len() {
        text.remove(byte_index);
    }
}

fn remove_char_range(text: &mut String, start: usize, end: usize) {
    let start_byte = byte_index_for_char(text, start);
    let end_byte = byte_index_for_char(text, end);
    if start_byte < end_byte && start_byte <= text.len() && end_byte <= text.len() {
        text.replace_range(start_byte..end_byte, "");
    }
}

fn pad_line_to_col(text: &mut String, target_col: usize) {
    let len = char_count(text);
    if len < target_col {
        text.push_str(&" ".repeat(target_col - len));
    }
}

fn normalize_register_name(key: &str) -> Option<RegisterTarget> {
    match key {
        "\"" => Some(RegisterTarget::Unnamed),
        "+" => Some(RegisterTarget::Clipboard),
        "_" => Some(RegisterTarget::BlackHole),
        "0" => Some(RegisterTarget::Named('0')),
        "-" => Some(RegisterTarget::Named('-')),
        value if value.chars().count() == 1 => value.chars().next().and_then(RegisterTarget::from_prefix),
        _ => None,
    }
}

fn is_word_char(ch: char, big_word: bool) -> bool {
    if ch == '\n' || ch.is_whitespace() {
        return false;
    }
    big_word || ch.is_alphanumeric() || ch == '_'
}

fn is_repeatable_change(macro_seq: &[String]) -> bool {
    if macro_seq.is_empty() {
        return false;
    }
    let first = &macro_seq[0];
    if first == "u" || first == "ctrl+r" || first == "." {
        return false;
    }
    // Block pure yanks
    if first.starts_with('y') {
        return false;
    }
    // Block command/search mode triggers
    if first.starts_with(':') || first.starts_with('/') || first.starts_with('?') {
        return false;
    }
    true
}

fn delimiter_pair(delimiter: char) -> Option<(char, char)> {
    match delimiter {
        '\'' => Some(('\'', '\'')),
        '"' => Some(('"', '"')),
        '(' | ')' => Some(('(', ')')),
        '[' | ']' => Some(('[', ']')),
        '{' | '}' => Some(('{', '}')),
        '<' | '>' => Some(('<', '>')),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NoteStats {
    pub line_count: usize,
    pub word_count: usize,
    pub char_count: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RegisterSnapshot {
    pub text: String,
    pub linewise: bool,
    pub blockwise: bool,
    pub version: usize,
}

#[cfg(any())]
mod tests {
    struct NoteDocument {
        pane: Pane,
        buffer: TextBuffer,
    }

    impl std::ops::Deref for NoteDocument {
        type Target = TextBuffer;
        fn deref(&self) -> &Self::Target { &self.buffer }
    }
    impl std::ops::DerefMut for NoteDocument {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.buffer }
    }

    impl NoteDocument {
        fn default() -> Self {
            Self { pane: Pane::new(PaneId(0), BufferId(0)), buffer: TextBuffer::new() }
        }
        fn title(&self) -> String { self.pane.title(&self.buffer) }
        fn stats(&self) -> NoteStats { self.pane.stats(&self.buffer) }
        fn is_open(&self) -> bool { self.pane.is_open(&self.buffer) }
        fn mode(&self) -> VimMode { self.pane.mode(&self.buffer) }
        fn enter_normal(&mut self) { self.pane.enter_normal(); }
        fn handle_editor_key(&mut self, key: EditorKey) { self.pane.handle_editor_key(&mut self.buffer, key); }
        fn cursor_line(&self) -> usize { self.pane.cursor_line }
        fn cursor_col(&self) -> usize { self.pane.cursor_col }
        fn content(&self) -> &str { self.pane.content(&self.buffer) }
        fn unnamed_register_text(&self) -> &str { self.pane.unnamed_register_text(&self.buffer) }
        fn content_with_cursor_marker(&self) -> String { self.pane.content_with_cursor_marker(&self.buffer) }
        fn display_cursor_col(&self) -> usize { self.pane.display_cursor_col(&self.buffer) }
        fn yank_register_text(&self) -> &str { self.pane.yank_register_text(&self.buffer) }
        fn named_register_text(&self, mark: char) -> &str { self.pane.named_register_text(&self.buffer, mark) }
        fn execute_command(&mut self, cmd: &str) { self.pane.execute_command(&mut self.buffer, cmd); }
        fn line_selection_cols(&self, line: usize) -> Option<(usize, usize)> { self.pane.line_selection_cols(&self.buffer, line) }

        fn new_blank(&mut self) { self.pane.new_blank(&mut self.buffer); }
        fn handle_insert_text(&mut self, text: &str) { self.pane.handle_insert_text(&mut self.buffer, text); }
        fn insert_newline(&mut self) { self.pane.insert_newline(&mut self.buffer); }
        fn backspace(&mut self) { self.pane.backspace(&mut self.buffer); }

        fn handle_normal_input(&mut self, text: &str) -> bool { self.pane.handle_normal_input(&mut self.buffer, text); true }
        fn handle_insert_input(&mut self, text: &str) { self.pane.handle_insert_text(&mut self.buffer, text); }
    }


    use super::*;
    use crate::vim::key::EditorKey;

    fn mk_doc(text: &str) -> NoteDocument {
        let mut doc = NoteDocument::default();
        doc.content = text.to_string();
        doc.open = true;
        doc.enter_normal();
        doc
    }

    fn feed(doc: &mut NoteDocument, keys: &[&str]) {
        for &key in keys {
            let editor_key = match doc.mode() {
                VimMode::Insert | VimMode::Replace => {
                    crate::vim::key::normalize_insert_key(key)
                }
                VimMode::Command | VimMode::Search(_) => {
                    crate::vim::key::normalize_command_key(key)
                }
                _ => {
                    crate::vim::key::normalize_normal_key(key)
                }
            };
            doc.handle_editor_key(editor_key);
        }
    }

    #[test]
    fn blank_document_has_untitled_name() {
        let doc = NoteDocument::default();
        assert_eq!(doc.title(), "Untitled");
        assert_eq!(doc.stats().line_count, 1);
        assert!(!doc.is_open());
    }

    #[test]
    fn stats_count_words_and_chars() {
        let mut doc = NoteDocument::default();
        doc.content = "hello world\nsecond line".to_string();
        assert_eq!(
            doc.stats(),
            NoteStats {
                line_count: 2,
                word_count: 4,
                char_count: 23,
            }
        );
    }

    #[test]
    fn normal_mode_dd_deletes_current_line() {
        let mut doc = NoteDocument::default();
        doc.content = "one\ntwo\nthree".to_string();
        doc.enter_normal();
        doc.handle_normal_input("jdd");
        assert_eq!(doc.content(), "one\nthree");
    }

    #[test]
    fn normal_mode_o_opens_line_below_and_enters_insert() {
        let mut doc = NoteDocument::default();
        doc.content = "one".to_string();
        doc.enter_normal();
        doc.handle_normal_input("o");
        assert_eq!(doc.content(), "one\n");
        assert_eq!(doc.mode(), VimMode::Insert);
    }

    #[test]
    fn normal_mode_hjkl_moves_cursor_line_and_column() {
        let mut doc = NoteDocument::default();
        doc.content = "abc\ndef".to_string();
        doc.enter_normal();
        doc.handle_normal_input("lljh");
        assert_eq!(doc.cursor_line(), 1);
        assert_eq!(doc.cursor_col(), 1);
    }

    #[test]
    fn normal_mode_counts_apply_to_basic_motions() {
        let mut doc = NoteDocument::default();
        doc.content = "abc\ndef\nghi\njkl".to_string();
        doc.enter_normal();
        doc.handle_normal_input("2j2l");
        assert_eq!(doc.cursor_line(), 2);
        assert_eq!(doc.cursor_col(), 2);
    }

    #[test]
    fn normal_mode_line_motions_work() {
        let mut doc = NoteDocument::default();
        doc.content = "  one\ntwo\nthree".to_string();
        doc.enter_normal();
        doc.handle_normal_input("G");
        assert_eq!(doc.cursor_line(), 2);
        doc.handle_normal_input("1G");
        assert_eq!(doc.cursor_line(), 0);
        doc.handle_normal_input("$0^");
        assert_eq!(doc.cursor_col(), 2);
    }

    #[test]
    fn insert_mode_mutates_buffer_without_text_edit() {
        let mut doc = NoteDocument::default();
        doc.new_blank();
        doc.handle_normal_input("i");
        doc.handle_insert_text("hello");
        doc.insert_newline();
        doc.handle_insert_text("world");
        assert_eq!(doc.content(), "hello\nworld");
        doc.backspace();
        assert_eq!(doc.content(), "hello\nworl");
    }

    #[test]
    fn cursor_marker_allows_insert_caret_after_line_end() {
        let mut doc = NoteDocument::default();
        doc.new_blank();
        doc.handle_normal_input("i");
        doc.handle_insert_text("hello");
        assert_eq!(doc.content_with_cursor_marker(), "hello|");

        doc.enter_normal();
        assert_eq!(doc.content_with_cursor_marker(), "hell|o");
    }

    #[test]
    fn test_dd_twice() {
        let mut doc = NoteDocument::default();
        doc.content = "line1\nline2\nline3".to_string();
        doc.enter_normal();
        doc.handle_normal_input("d");
        doc.handle_normal_input("d");
        assert_eq!(doc.content(), "line2\nline3");
    }

    #[test]
    fn normal_mode_word_motions_move_cursor() {
        let mut doc = NoteDocument::default();
        doc.content = "alpha beta gamma".to_string();
        doc.enter_normal();
        doc.handle_normal_input("w");
        assert_eq!(doc.cursor_col(), 6);
        doc.handle_normal_input("e");
        assert_eq!(doc.cursor_col(), 9);
        doc.handle_normal_input("b");
        assert_eq!(doc.cursor_col(), 6);
    }

    #[test]
    fn normal_mode_delete_operator_composes_with_word_motion() {
        let mut doc = NoteDocument::default();
        doc.content = "alpha beta gamma".to_string();
        doc.enter_normal();

        doc.handle_normal_input("dw");

        assert_eq!(doc.content(), "beta gamma");
        assert_eq!(doc.cursor_line(), 0);
        assert_eq!(doc.cursor_col(), 0);
        assert_eq!(doc.mode(), VimMode::Normal);
    }

    #[test]
    fn normal_mode_operator_counts_work_before_or_after_operator() {
        let mut doc = NoteDocument::default();
        doc.content = "alpha beta gamma delta".to_string();
        doc.enter_normal();

        doc.handle_normal_input("3dw");

        assert_eq!(doc.content(), "delta");
        assert_eq!(doc.cursor_col(), 0);

        let mut doc = NoteDocument::default();
        doc.content = "alpha beta gamma delta".to_string();
        doc.enter_normal();

        doc.handle_normal_input("d3w");

        assert_eq!(doc.content(), "delta");
        assert_eq!(doc.cursor_col(), 0);
    }

    #[test]
    fn normal_mode_change_to_line_end_enters_insert() {
        let mut doc = NoteDocument::default();
        doc.content = "alpha beta".to_string();
        doc.enter_normal();
        doc.handle_normal_input("w");

        doc.handle_normal_input("c$");

        assert_eq!(doc.content(), "alpha ");
        assert_eq!(doc.display_cursor_col(), 6);
        assert_eq!(doc.mode(), VimMode::Insert);
    }

    #[test]
    fn normal_mode_change_line_replaces_line_with_empty_insert_line() {
        let mut doc = NoteDocument::default();
        doc.content = "one\ntwo\nthree".to_string();
        doc.enter_normal();
        doc.handle_normal_input("j");

        doc.handle_normal_input("cc");

        assert_eq!(doc.content(), "one\n\nthree");
        assert_eq!(doc.cursor_line(), 1);
        assert_eq!(doc.cursor_col(), 0);
        assert_eq!(doc.mode(), VimMode::Insert);
    }

    #[test]
    fn normal_mode_text_objects_support_inner_and_around_word() {
        let mut inner = NoteDocument::default();
        inner.content = "hello world".to_string();
        inner.enter_normal();
        inner.handle_normal_input("l");

        inner.handle_normal_input("ciw");

        assert_eq!(inner.content(), " world");
        assert_eq!(inner.cursor_col(), 0);
        assert_eq!(inner.mode(), VimMode::Insert);

        let mut around = NoteDocument::default();
        around.content = "hello world".to_string();
        around.enter_normal();
        around.handle_normal_input("l");

        around.handle_normal_input("daw");

        assert_eq!(around.content(), "world");
        assert_eq!(around.cursor_col(), 0);
        assert_eq!(around.mode(), VimMode::Normal);
    }

    #[test]
    fn normal_mode_gg_preserves_prefix_count() {
        let mut doc = NoteDocument::default();
        doc.content = "one\ntwo\nthree".to_string();
        doc.enter_normal();
        doc.handle_normal_input("G");

        doc.handle_normal_input("2gg");

        assert_eq!(doc.cursor_line(), 1);
    }

    #[test]
    fn normal_mode_yank_line_updates_unnamed_and_yank_registers() {
        let mut doc = NoteDocument::default();
        doc.content = "one\ntwo\nthree".to_string();
        doc.enter_normal();
        doc.handle_normal_input("j");

        doc.handle_normal_input("yy");

        assert_eq!(doc.content(), "one\ntwo\nthree");
        assert_eq!(doc.unnamed_register_text(), "two\n");
        assert_eq!(doc.yank_register_text(), "two\n");
        assert!(doc.unnamed_register_is_linewise());
        assert!(doc.yank_register_is_linewise());
    }

    #[test]
    fn unnamed_register_can_be_replaced_from_app_boundary() {
        let mut doc = NoteDocument::default();

        doc.set_unnamed_register("clip\n".to_string(), true);

        assert_eq!(
            doc.unnamed_register_snapshot(),
            RegisterSnapshot {
                text: "clip\n".to_string(),
                linewise: true,
                blockwise: false,
                version: 0,
            }
        );
        assert_eq!(doc.unnamed_register_text(), "clip\n");
        assert!(doc.unnamed_register_is_linewise());
        assert_eq!(doc.yank_register_text(), "");
    }

    #[test]
    fn normal_mode_yank_operator_composes_with_motions_and_text_objects() {
        let mut motion = NoteDocument::default();
        motion.content = "alpha beta gamma".to_string();
        motion.enter_normal();

        assert!(!motion.handle_normal_input("yw"));

        assert_eq!(motion.content(), "alpha beta gamma");
        assert_eq!(motion.unnamed_register_text(), "alpha ");
        assert_eq!(motion.yank_register_text(), "alpha ");
        assert!(!motion.yank_register_is_linewise());

        let mut inner = NoteDocument::default();
        inner.content = "hello world".to_string();
        inner.enter_normal();
        inner.handle_normal_input("w");

        assert!(!inner.handle_normal_input("yiw"));

        assert_eq!(inner.yank_register_text(), "world");
        assert_eq!(inner.content(), "hello world");

        let mut around = NoteDocument::default();
        around.content = "hello world".to_string();
        around.enter_normal();

        assert!(!around.handle_normal_input("yaw"));

        assert_eq!(around.yank_register_text(), "hello ");
        assert_eq!(around.content(), "hello world");
    }

    #[test]
    fn delete_and_change_update_unnamed_without_replacing_yank_register() {
        let mut doc = NoteDocument::default();
        doc.content = "alpha beta gamma".to_string();
        doc.enter_normal();

        doc.handle_normal_input("yiw");
        assert_eq!(doc.yank_register_text(), "alpha");

        doc.handle_normal_input("dw");

        assert_eq!(doc.unnamed_register_text(), "alpha ");
        assert_eq!(doc.yank_register_text(), "alpha");

        doc.handle_normal_input("cw");
        assert_eq!(doc.unnamed_register_text(), "beta ");
        assert_eq!(doc.yank_register_text(), "alpha");
        assert_eq!(doc.mode(), VimMode::Insert);
    }

    #[test]
    fn normal_mode_x_updates_unnamed_without_replacing_yank_register() {
        let mut doc = NoteDocument::default();
        doc.content = "abcdef".to_string();
        doc.enter_normal();

        doc.handle_normal_input("ywl");
        assert_eq!(doc.yank_register_text(), "abcdef");

        doc.handle_normal_input("2x");

        assert_eq!(doc.content(), "adef");
        assert_eq!(doc.unnamed_register_text(), "bc");
        assert!(!doc.unnamed_register_is_linewise());
        assert_eq!(doc.yank_register_text(), "abcdef");
    }

    #[test]
    fn pointer_cursor_placement_clamps_to_line_and_mode_bounds() {
        let mut normal = NoteDocument::default();
        normal.content = "abc\nde".to_string();
        normal.enter_normal();

        normal.set_cursor_from_pointer(9, 99);

        assert_eq!(normal.cursor_line(), 1);
        assert_eq!(normal.cursor_col(), 1);

        let mut insert = NoteDocument::default();
        insert.content = "abc".to_string();
        insert.enter_insert();

        insert.set_cursor_from_pointer(0, 99);

        assert_eq!(insert.cursor_line(), 0);
        assert_eq!(insert.display_cursor_col(), 3);
    }

    #[test]
    fn normal_mode_operator_gg_uses_linewise_range() {
        let mut yank_to_top = NoteDocument::default();
        yank_to_top.content = "one\ntwo\nthree\nfour".to_string();
        yank_to_top.enter_normal();
        yank_to_top.handle_normal_input("G");

        assert!(!yank_to_top.handle_normal_input("ygg"));

        assert_eq!(yank_to_top.content(), "one\ntwo\nthree\nfour");
        assert_eq!(yank_to_top.yank_register_text(), "one\ntwo\nthree\nfour\n");
        assert!(yank_to_top.yank_register_is_linewise());

        let mut yank_to_counted_line = NoteDocument::default();
        yank_to_counted_line.content = "one\ntwo\nthree\nfour".to_string();
        yank_to_counted_line.enter_normal();
        yank_to_counted_line.handle_normal_input("G");

        assert!(!yank_to_counted_line.handle_normal_input("y2gg"));

        assert_eq!(
            yank_to_counted_line.yank_register_text(),
            "two\nthree\nfour\n"
        );

        let mut delete_to_top = NoteDocument::default();
        delete_to_top.content = "one\ntwo\nthree\nfour".to_string();
        delete_to_top.enter_normal();
        delete_to_top.handle_normal_input("G");

        assert!(delete_to_top.handle_normal_input("dgg"));

        assert_eq!(delete_to_top.content(), "");
        assert_eq!(
            delete_to_top.unnamed_register_text(),
            "one\ntwo\nthree\nfour\n"
        );
        assert!(delete_to_top.unnamed_register_is_linewise());
    }

    #[test]
    fn normal_mode_paste_linewise_unnamed_register_after_and_before() {
        let mut after = NoteDocument::default();
        after.content = "one\ntwo".to_string();
        after.enter_normal();

        after.handle_normal_input("yyp");

        assert_eq!(after.content(), "one\none\ntwo");
        assert_eq!(after.cursor_line(), 1);
        assert_eq!(after.cursor_col(), 0);

        let mut before = NoteDocument::default();
        before.content = "one\ntwo".to_string();
        before.enter_normal();

        before.handle_normal_input("yyjP");

        assert_eq!(before.content(), "one\none\ntwo");
        assert_eq!(before.cursor_line(), 1);
        assert_eq!(before.cursor_col(), 0);
    }

    #[test]
    fn normal_mode_paste_charwise_unnamed_register_after_and_before() {
        let mut after = NoteDocument::default();
        after.content = "alpha beta".to_string();
        after.enter_normal();
        after.handle_normal_input("yiw$p");

        assert_eq!(after.content(), "alpha betaalpha");
        assert_eq!(after.cursor_col(), 14);

        let mut before = NoteDocument::default();
        before.content = "alpha beta".to_string();
        before.enter_normal();
        before.handle_normal_input("yiwwP");

        assert_eq!(before.content(), "alpha alphabeta");
        assert_eq!(before.cursor_col(), 10);
    }

    #[test]
    fn normal_mode_paste_respects_count() {
        let mut doc = NoteDocument::default();
        doc.content = "one\ntwo".to_string();
        doc.enter_normal();

        doc.handle_normal_input("yy2p");

        assert_eq!(doc.content(), "one\none\none\ntwo");
        assert_eq!(doc.cursor_line(), 1);
    }

    #[test]
    fn normal_mode_undo_redo_restore_common_edits() {
        let mut doc = NoteDocument::default();
        doc.content = "one\ntwo\nthree".to_string();
        doc.enter_normal();

        doc.handle_normal_input("dd");
        assert_eq!(doc.content(), "two\nthree");

        doc.handle_normal_input("u");
        assert_eq!(doc.content(), "one\ntwo\nthree");

        doc.handle_normal_input("ctrl+r");
        assert_eq!(doc.content(), "two\nthree");
    }

    #[test]
    fn normal_mode_dot_repeats_last_change_without_repeating_undo() {
        let mut doc = NoteDocument::default();
        doc.content = "abcdef".to_string();
        doc.enter_normal();

        doc.handle_normal_input("x");
        doc.handle_normal_input(".");
        assert_eq!(doc.content(), "cdef");

        doc.handle_normal_input("u");
        assert_eq!(doc.content(), "bcdef");
        doc.handle_normal_input(".");
        assert_eq!(doc.content(), "cdef");
    }

    #[test]
    fn normal_mode_named_clipboard_and_black_hole_registers_work() {
        let mut named = NoteDocument::default();
        named.content = "one\ntwo".to_string();
        named.enter_normal();

        named.handle_normal_input("\"ayyj\"ap");

        assert_eq!(named.named_register_text('a'), Some("one\n"));
        assert_eq!(named.content(), "one\ntwo\none");

        let mut clipboard = NoteDocument::default();
        clipboard.content = "alpha beta".to_string();
        clipboard.enter_normal();

        clipboard.handle_normal_input("\"+yiw");

        assert_eq!(clipboard.clipboard_register_text(), "alpha");
        assert_eq!(clipboard.unnamed_register_text(), "alpha");
        assert_eq!(clipboard.yank_register_text(), "alpha");

        let mut black_hole = NoteDocument::default();
        black_hole.content = "one\ntwo".to_string();
        black_hole.enter_normal();
        black_hole.handle_normal_input("yyj\"_dd");

        assert_eq!(black_hole.content(), "one");
        assert_eq!(black_hole.unnamed_register_text(), "one\n");
        assert_eq!(black_hole.yank_register_text(), "one\n");
    }

    #[test]
    fn normal_mode_indent_outdent_and_format_operators_work() {
        let mut doc = NoteDocument::default();
        doc.content = "one  \ntwo\t\nthree".to_string();
        doc.enter_normal();

        doc.handle_normal_input(">>");
        assert_eq!(doc.content(), "  one  \ntwo\t\nthree");

        doc.handle_normal_input("<<");
        assert_eq!(doc.content(), "one  \ntwo\t\nthree");

        doc.handle_normal_input("=j");
        assert_eq!(doc.content(), "one\ntwo\nthree");
    }

    #[test]
    fn normal_mode_case_operators_work() {
        let mut toggle = NoteDocument::default();
        toggle.content = "aBc".to_string();
        toggle.enter_normal();

        assert!(toggle.handle_normal_input("3~"));
        assert_eq!(toggle.content(), "AbC");

        let mut lower = NoteDocument::default();
        lower.content = "ALPHA beta".to_string();
        lower.enter_normal();

        assert!(lower.handle_normal_input("guw"));
        assert_eq!(lower.content(), "alpha beta");

        let mut upper = NoteDocument::default();
        upper.content = "alpha beta".to_string();
        upper.enter_normal();

        assert!(upper.handle_normal_input("gUw"));
        assert_eq!(upper.content(), "ALPHA beta");
    }

    #[test]
    fn normal_mode_quote_bracket_paragraph_and_line_text_objects_work() {
        let mut quote = NoteDocument::default();
        quote.content = "say \"hello\" now".to_string();
        quote.enter_normal();
        quote.handle_normal_input("5l");

        assert!(quote.handle_normal_input("ci\""));
        assert_eq!(quote.content(), "say \"\" now");
        assert_eq!(quote.mode(), VimMode::Insert);

        let mut bracket = NoteDocument::default();
        bracket.content = "call(one, two)".to_string();
        bracket.enter_normal();
        bracket.handle_normal_input("5l");

        assert!(!bracket.handle_normal_input("ya("));
        assert_eq!(bracket.yank_register_text(), "(one, two)");

        let mut paragraph = NoteDocument::default();
        paragraph.content = "one\ntwo\n\nthree\nfour".to_string();
        paragraph.enter_normal();

        assert!(paragraph.handle_normal_input("dap"));
        assert_eq!(paragraph.content(), "three\nfour");

        let mut line = NoteDocument::default();
        line.content = "one\ntwo\nthree".to_string();
        line.enter_normal();
        line.handle_normal_input("j");

        assert!(line.handle_normal_input("dil"));
        assert_eq!(line.content(), "one\nthree");
    }

    #[test]
    fn visual_char_mode_yanks_and_deletes_selection() {
        let mut yank = NoteDocument::default();
        yank.content = "abcdef".to_string();
        yank.enter_normal();

        yank.handle_normal_input("vlly");

        assert_eq!(yank.mode(), VimMode::Normal);
        assert_eq!(yank.content(), "abcdef");
        assert_eq!(yank.yank_register_text(), "abc");

        let mut delete = NoteDocument::default();
        delete.content = "abcdef".to_string();
        delete.enter_normal();

        assert!(delete.handle_normal_input("vlld"));

        assert_eq!(delete.content(), "def");
        assert_eq!(delete.unnamed_register_text(), "abc");
    }

    #[test]
    fn visual_line_mode_operators_gv_and_o_work() {
        let mut doc = NoteDocument::default();
        doc.content = "one\ntwo\nthree".to_string();
        doc.enter_normal();

        doc.handle_normal_input("Vjy");

        assert_eq!(doc.mode(), VimMode::Normal);
        assert_eq!(doc.yank_register_text(), "one\ntwo\n");
        doc.handle_normal_input("gv");
        assert_eq!(doc.mode(), VimMode::VisualLine);
        assert!(doc.line_selection_cols(0).is_some());
        assert!(doc.line_selection_cols(1).is_some());

        doc.handle_normal_input("o");
        assert_eq!(doc.cursor_line(), 0);

        doc.handle_normal_input("d");
        assert_eq!(doc.content(), "three");
    }

    #[test]
    fn search_navigation_highlights_noh_and_word_search_work() {
        let mut doc = NoteDocument::default();
        doc.content = "one two\nthree two\nfour".to_string();
        doc.enter_normal();

        doc.handle_normal_input("/");
        doc.handle_normal_input("two");
        doc.handle_normal_input("return");

        assert_eq!(doc.cursor_line(), 0);
        assert_eq!(doc.cursor_col(), 4);
        assert!(doc.line_has_search_match(0));
        assert!(doc.line_has_search_match(1));

        doc.handle_normal_input("n");
        assert_eq!(doc.cursor_line(), 1);
        assert_eq!(doc.cursor_col(), 6);

        doc.handle_normal_input("N");
        assert_eq!(doc.cursor_line(), 0);

        doc.handle_normal_input(":noh");
        assert!(!doc.line_has_search_match(0));
        assert_eq!(doc.search_pattern(), Some("two"));

        doc.handle_normal_input("0*");
        assert_eq!(doc.search_pattern(), Some("one"));
        assert!(doc.line_has_search_match(0));
    }

    #[test]
    fn marks_jump_to_saved_positions() {
        let mut doc = NoteDocument::default();
        doc.content = "one\ntwo\nthree".to_string();
        doc.enter_normal();

        doc.handle_normal_input("jmaG");
        assert_eq!(doc.cursor_line(), 2);

        doc.handle_normal_input("'a");

        assert_eq!(doc.cursor_line(), 1);
        assert_eq!(doc.cursor_col(), 0);
    }

    #[test]
    fn test_visual_feedback_yank_and_delete() {
        let mut doc = NoteDocument::default();
        doc.defer_enabled = true;
        doc.content = "line one\nline two\nline three".to_string();
        doc.enter_normal();

        // 1. Test linewise yank (yy)
        doc.handle_normal_input("yy");
        assert!(doc.has_yank_highlight());
        assert!(doc.line_selection_cols(0).is_some());
        assert!(doc.line_selection_cols(1).is_none());
        assert_eq!(doc.cursor_line(), 0);

        // 2. Clear highlight
        doc.clear_yank_highlight();
        assert!(!doc.has_yank_highlight());

        // 3. Test linewise delete (dd)
        doc.handle_normal_input("dd");
        assert!(doc.has_deferred_action());
        assert!(doc.has_yank_highlight());
        assert!(doc.line_selection_cols(0).is_some());
        assert_eq!(doc.content(), "line one\nline two\nline three");

        // 4. Flush deferred action
        assert!(doc.flush_deferred_action());
        assert!(!doc.has_deferred_action());
        assert!(!doc.has_yank_highlight());
        assert_eq!(doc.content(), "line two\nline three");

        // 5. Test another key input automatically flushes deferred action
        let mut doc2 = NoteDocument::default();
        doc2.defer_enabled = true;
        doc2.content = "line one\nline two\nline three".to_string();
        doc2.enter_normal();
        
        doc2.handle_normal_input("dd");
        assert!(doc2.has_deferred_action());
        
        doc2.handle_normal_input("p");
        assert!(!doc2.has_deferred_action());
        assert_eq!(doc2.content(), "line two\nline one\nline three");
    }

    #[test]
    fn test_yy_twice() {
        let mut doc = NoteDocument::default();
        doc.content = "line1\nline2\nline3".to_string();
        doc.enter_normal();
        doc.handle_normal_input("y");
        doc.handle_normal_input("y");
        assert_eq!(doc.unnamed_register_text(), "line1\n");
    }

    #[test]
    fn test_yy_last_empty_line() {
        let mut doc = NoteDocument::default();
        doc.content = "line1\n".to_string();
        doc.enter_normal();
        doc.handle_normal_input("G"); // Go to last line (which is empty)
        assert_eq!(doc.cursor_line(), 1);
        doc.handle_normal_input("yy");
        assert_eq!(doc.unnamed_register_text(), "\n");
    }

    #[test]
    fn test_dd_last_empty_line() {
        let mut doc = NoteDocument::default();
        doc.content = "line1\n".to_string();
        doc.enter_normal();
        doc.handle_normal_input("G"); // Go to last line (which is empty)
        assert_eq!(doc.cursor_line(), 1);
        doc.handle_normal_input("dd");
        doc.flush_deferred_action();
        assert_eq!(doc.content(), "line1");
        assert_eq!(doc.cursor_line(), 0);
    }
    #[test]
    fn test_zz_command_maps_to_save_and_quit() {
        let mut doc = NoteDocument::default();
        doc.content = "line1\nline2".to_string();
        doc.enter_normal();

        // Z prefix sets pending_command to ZPrefix
        doc.handle_normal_input("Z");
        assert_eq!(doc.vim_state.pending_command, Some(PendingCommand::ZPrefix));

        // Another Z maps to SaveAndQuit
        doc.handle_normal_input("Z");
        assert_eq!(doc.pending_ex_action, Some(ExCommandAction::SaveAndQuit));
    }

    #[test]
    fn test_substitution_confirm_prompt() {
        let mut doc = NoteDocument::default();
        doc.content = "one two three two five".to_string();
        doc.enter_normal();

        doc.handle_normal_input(":%s/two/new/gc");
        doc.handle_normal_input("return");

        match &doc.vim_state.pending_command {
            Some(PendingCommand::SubstituteConfirm { pattern, replacement, matches, match_index, .. }) => {
                assert_eq!(pattern, "two");
                assert_eq!(replacement, "new");
                assert_eq!(matches.len(), 2);
                assert_eq!(*match_index, 0);
            }
            _ => panic!("Expected SubstituteConfirm"),
        }

        doc.handle_normal_input("y");
        assert_eq!(doc.content(), "one new three two five");

        match &doc.vim_state.pending_command {
            Some(PendingCommand::SubstituteConfirm { match_index, .. }) => {
                assert_eq!(*match_index, 1);
            }
            _ => panic!("Expected SubstituteConfirm"),
        }

        doc.handle_normal_input("n");
        assert_eq!(doc.content(), "one new three two five");
        assert_eq!(doc.vim_state.mode, VimMode::Normal);
        assert!(doc.vim_state.pending_command.is_none());
        assert_eq!(doc.search_pattern(), Some("two"));
    }

    #[test]
    fn test_substitution_confirm_all() {
        let mut doc = NoteDocument::default();
        doc.content = "one two three two five".to_string();
        doc.enter_normal();

        doc.handle_normal_input(":%s/two/new/gc");
        doc.handle_normal_input("return");

        doc.handle_normal_input("a");
        assert_eq!(doc.content(), "one new three new five");
        assert_eq!(doc.vim_state.mode, VimMode::Normal);
        assert_eq!(doc.search_pattern(), Some("two"));
    }

    #[test]
    fn test_substitution_confirm_last() {
        let mut doc = NoteDocument::default();
        doc.content = "one two three two five".to_string();
        doc.enter_normal();

        doc.handle_normal_input(":%s/two/new/gc");
        doc.handle_normal_input("return");

        doc.handle_normal_input("l");
        assert_eq!(doc.content(), "one new three two five");
        assert_eq!(doc.vim_state.mode, VimMode::Normal);
        assert_eq!(doc.search_pattern(), Some("two"));
    }

    #[test]
    fn test_substitution_confirm_quit() {
        let mut doc = NoteDocument::default();
        doc.content = "one two three two five".to_string();
        doc.enter_normal();

        doc.handle_normal_input(":%s/two/new/gc");
        doc.handle_normal_input("return");

        doc.handle_normal_input("q");
        assert_eq!(doc.content(), "one two three two five");
        assert_eq!(doc.vim_state.mode, VimMode::Normal);
        assert_eq!(doc.search_pattern(), Some("two"));
    }

    #[test]
    fn test_insert_mode_undo_breakpoints() {
        let mut doc = NoteDocument::default();
        doc.enter_normal();
        doc.handle_normal_input("i");
        doc.handle_insert_text("Text"); // simulate inserting text
        doc.handle_insert_text("abc ");
        doc.delete_word_insert();
        doc.handle_insert_text("Text");
        doc.handle_insert_text("def");
        doc.cancel_insert();

        doc.undo(); 
        assert_eq!(doc.content(), "Textabc ");
        doc.undo(); 
        assert_eq!(doc.content(), "");
    }

    #[test]
    fn test_small_delete_register() {
        let mut doc = NoteDocument::default();
        doc.content = "one two three".to_string();
        doc.enter_normal();
        
        doc.handle_normal_input("d");
        doc.handle_normal_input("w");
        
        assert_eq!(doc.content(), "two three");
        assert_eq!(doc.registers.register(RegisterTarget::Named('-')).text, "one ");
    }

    #[test]
    fn test_numbered_registers_rotation() {
        let mut doc = NoteDocument::default();
        doc.content = "line 1\nline 2\nline 3".to_string();
        doc.enter_normal();
        
        doc.handle_normal_input("d");
        doc.handle_normal_input("d");
        assert_eq!(doc.registers.register(RegisterTarget::Named('1')).text, "line 1\n");
        
        doc.handle_normal_input("d");
        doc.handle_normal_input("d");
        assert_eq!(doc.registers.register(RegisterTarget::Named('1')).text, "line 2\n");
        assert_eq!(doc.registers.register(RegisterTarget::Named('2')).text, "line 1\n");
    }

    #[test]
    fn test_visual_mode_kind_switching() {
        let mut doc = NoteDocument::default();
        doc.enter_normal();
        
        doc.handle_normal_input("v");
        assert_eq!(doc.visual_kind(), VisualKind::Character);
        
        doc.handle_normal_input("V");
        assert_eq!(doc.visual_kind(), VisualKind::Line);
        
        doc.handle_normal_input("V");
        assert_eq!(doc.vim_state.mode, VimMode::Normal);
    }

    #[test]
    fn test_case_operator_linewise_doubling() {
        let mut doc = NoteDocument::default();
        doc.content = "hello\nworld".to_string();
        doc.enter_normal();
        
        doc.handle_normal_input("g");
        doc.handle_normal_input("U");
        doc.handle_normal_input("U");
        
        assert_eq!(doc.content(), "HELLO\nworld");
    }

    #[test]
    fn test_search_highlight_toggle() {
        let mut doc = NoteDocument::default();
        doc.content = "hello world".to_string();
        doc.enter_normal();
        
        doc.handle_normal_input("/");
        doc.handle_normal_input("hello");
        doc.handle_normal_input("return");
        
        assert!(doc.line_has_search_match(0));
        
        doc.handle_normal_input(":noh");
        
        assert!(!doc.line_has_search_match(0));
        
        doc.handle_normal_input("n");
        assert!(doc.line_has_search_match(0));
    }

    #[test]
    fn normal_mode_r_s_d_c_x_and_join_commands_work() {
        let mut doc = mk_doc("hello\nworld\nagain");
        feed(&mut doc, &["l", "3", "r", "X"]);
        assert_eq!(doc.content(), "hXXXo\nworld\nagain");

        let mut doc = mk_doc("hello");
        feed(&mut doc, &["l", "s", "a", "b", "c", "escape"]);
        assert_eq!(doc.content(), "habcllo");

        let mut doc = mk_doc("hello world");
        feed(&mut doc, &["7", "|", "D"]);
        assert_eq!(doc.content(), "hello ");

        let mut doc = mk_doc("hello world");
        feed(&mut doc, &["7", "|", "C", "R", "u", "s", "t", "escape"]);
        assert_eq!(doc.content(), "hello Rust");

        let mut doc = mk_doc("hello");
        feed(&mut doc, &["$", "X"]);
        assert_eq!(doc.content(), "helo");

        let mut doc = mk_doc("hello\nworld");
        feed(&mut doc, &["J"]);
        assert_eq!(doc.content(), "hello world");

        let mut doc = mk_doc("hello\nworld");
        feed(&mut doc, &["g", "J"]);
        assert_eq!(doc.content(), "helloworld");
    }

    #[test]
    fn normal_mode_replace_and_block_paste_work() {
        let mut doc = mk_doc("hello world");
        feed(&mut doc, &["7", "|", "R", "a", "b", "c", "escape"]);
        assert_eq!(doc.content(), "hello abcld");

        let mut doc = mk_doc("abc\ndef\nghi");
        feed(&mut doc, &["ctrl+v", "j", "y", "1", "|", "p"]);
        assert_eq!(doc.content(), "aabc\nddef\nghi");
    }

    #[test]
    fn visual_counts_and_mark_jumps_work() {
        let mut doc = mk_doc("one\ntwo\nthree\nfour");
        feed(&mut doc, &["V", "2", "j", "d"]);
        assert_eq!(doc.content(), "four");

        let mut doc = mk_doc("hello world");
        feed(&mut doc, &["7", "|", "m", "a", "0", "`", "a"]);
        assert_eq!(doc.cursor_col(), 6);
        feed(&mut doc, &["'", "a"]);
        assert_eq!(doc.cursor_col(), 0);
    }

    #[test]
    fn changelist_command_line_history_and_insert_ctrl_r_work() {
        let mut doc = mk_doc("");
        feed(&mut doc, &["i", "a", "b", "c", "escape", "G", "i", "d", "e", "f", "escape", "g", ";"]);
        assert_eq!(doc.cursor_col(), 3);
        feed(&mut doc, &["g", ","]);
        assert_eq!(doc.cursor_col(), 4);

        let mut doc = mk_doc("");
        feed(&mut doc, &[":", "s", "e", "t", " ", "n", "u", "m", "b", "e", "r", "return"]);
        feed(&mut doc, &[":", "up"]);
        assert_eq!(doc.vim_state.command_line.input, "set number");

        let mut doc = mk_doc("line");
        feed(&mut doc, &["y", "y", "G", "o", "ctrl+r", "0", "escape"]);
        assert_eq!(doc.content(), "line\nline\n");
    }

    #[test]
    fn search_options_uppercase_register_append_and_register_listing_work() {
        let mut doc = mk_doc("Foo foo FOO");
        feed(&mut doc, &[":", "s", "e", "t", " ", "i", "g", "n", "o", "r", "e", "c", "a", "s", "e", "return"]);
        feed(&mut doc, &["/", "f", "o", "o", "return"]);
        assert_eq!(doc.search.matches.len(), 3);

        let mut doc = mk_doc("one\ntwo");
        feed(&mut doc, &["\"", "a", "y", "y", "j", "\"", "A", "y", "y"]);
        assert_eq!(doc.named_register_text('a').unwrap_or(""), "one\ntwo\n");
        feed(&mut doc, &[":", "r", "e", "g", "return"]);
        assert!(matches!(doc.take_ex_action(), Some(ExCommandAction::ShowMessage(message)) if message.contains("a")));
    }

    #[test]
    fn test_macro_recording() {
        let mut doc = NoteDocument::default();
        doc.content = "hello\n".to_string();
        doc.open = true;
        
        // Start recording macro 'a'
        feed(&mut doc, &["q", "a", "A", " ", "w", "o", "r", "l", "d", "escape", "q"]);
        
        assert_eq!(doc.content, "hello world\n");
        
        // Add a line "greeting"
        feed(&mut doc, &["o", "g", "r", "e", "e", "t", "i", "n", "g", "escape"]);
        
        assert_eq!(doc.content, "hello world\ngreeting\n");
        
        // Move to start of second line and play macro
        feed(&mut doc, &["0", "@", "a"]);
        
        assert_eq!(doc.content, "hello world\ngreeting world\n");
        
        // Add third line and test @@
        feed(&mut doc, &["o", "h", "i", "escape", "0", "@", "@"]);
        
        assert_eq!(doc.content, "hello world\ngreeting world\nhi world\n");
    }

    #[test]
    fn test_surround_add_quotes() {
        let mut doc = NoteDocument::default();
        doc.content = "hello world\n".to_string();
        doc.open = true;
        
        // Move to 'world', apply ysiw"
        feed(&mut doc, &["w", "y", "s", "i", "w", "\""]);
        
        assert_eq!(doc.content, "hello \"world\"\n");
    }

    #[test]
    fn test_surround_delete_tag() {
        let mut doc = NoteDocument::default();
        doc.content = "hello <div>world</div>\n".to_string();
        doc.open = true;
        
        // Move to 'world', apply dst
        feed(&mut doc, &["w", "d", "s", "t"]);
        
        assert_eq!(doc.content, "hello world\n");
    }

    #[test]
    fn test_surround_change_tag() {
        let mut doc = NoteDocument::default();
        doc.content = "hello <div>world</div>\n".to_string();
        doc.open = true;
        
        // Move to 'world', apply cst<span>
        feed(&mut doc, &["w", "c", "s", "t", "<", "s", "p", "a", "n", ">"]);
        
        assert_eq!(doc.content, "hello <span>world</span>\n");
    }

    #[test]
    fn test_surround_visual() {
        let mut doc = NoteDocument::default();
        doc.content = "hello world\n".to_string();
        doc.open = true;
        
        // Move to 'world', visual select 'world', apply S(
        feed(&mut doc, &["w", "v", "e", "S", "("]);
        
        assert_eq!(doc.content, "hello (world)\n");
    }

    #[test]
    fn test_ci_angle_brackets() {
        let mut buffer = TextBuffer::new();
        buffer.content = "<div>hello world</div>".to_string();
        buffer.lines = vec!["<div>hello world</div>".to_string()];
        let mut note = super::Pane::new();
        note.cursor_line = 0;
        note.cursor_col = 2; // Inside the first <div> tag (on 'i')

        // type ci<
        note.handle_editor_key(&mut buffer, EditorKey::Input("c".to_string()));
        note.handle_editor_key(&mut buffer, EditorKey::Input("i".to_string()));
        note.handle_editor_key(&mut buffer, EditorKey::Input("<".to_string()));

        assert_eq!(buffer.content, "<>hello world</div>");
        assert_eq!(note.mode(&buffer), VimMode::Insert);
        assert_eq!(note.cursor_col, 1);
        
        // Let's also test da>
        note.handle_editor_key(&mut buffer, EditorKey::EnterNormal);
        buffer.content = "<div>hello world</div>".to_string();
        buffer.lines = vec!["<div>hello world</div>".to_string()];
        note.cursor_line = 0;
        note.cursor_col = 2; // Inside the first <div> tag (on 'i')
        
        note.handle_editor_key(&mut buffer, EditorKey::Input("d".to_string()));
        note.handle_editor_key(&mut buffer, EditorKey::Input("a".to_string()));
        note.handle_editor_key(&mut buffer, EditorKey::Input(">".to_string()));
        
        assert_eq!(buffer.content, "hello world</div>");
        assert_eq!(note.mode(&buffer), VimMode::Normal);
        assert_eq!(note.cursor_col, 0);
        
        // Test seeking forward!
        note.handle_editor_key(&mut buffer, EditorKey::EnterNormal);
        buffer.content = "   <tag>".to_string();
        buffer.lines = vec!["   <tag>".to_string()];
        note.cursor_line = 0;
        note.cursor_col = 0; // On the first space
        
        note.handle_editor_key(&mut buffer, EditorKey::Input("c".to_string()));
        note.handle_editor_key(&mut buffer, EditorKey::Input("i".to_string()));
        note.handle_editor_key(&mut buffer, EditorKey::Input("<".to_string()));
        
        // If it seeks forward, it should become `   <>`
        assert_eq!(buffer.content, "   <>");
    }

    #[test]
    fn utf16_files_round_trip_unicode_content() {
        let root = std::env::temp_dir().join(format!(
            "neonote-unicode-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("unicode.txt");
        let content = "こんにちは\nПривет\nمرحبا";
        let mut bytes = vec![0xFF, 0xFE];
        for unit in content.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        std::fs::write(&path, bytes).unwrap();

        let mut pane = Pane::new(PaneId(1), BufferId(1));
        let mut buffer = TextBuffer::new();
        pane.open(&mut buffer, &path).unwrap();

        assert_eq!(buffer.content, content);
        assert_eq!(buffer.encoding, TextEncoding::Utf16Le);

        pane.save(&mut buffer).unwrap();
        let saved = std::fs::read(&path).unwrap();
        assert_eq!(&saved[..2], &[0xFF, 0xFE]);
    }

    #[test]
    fn ignorecase_search_keeps_unicode_match_ranges_stable() {
        let mut buffer = TextBuffer::new();
        buffer.content = "AİB".to_string();
        buffer.settings.ignorecase = true;
        buffer.search.pattern = "İ".to_string();

        let mut pane = Pane::new(PaneId(1), BufferId(1));
        pane.refresh_search_matches(&mut buffer);

        assert_eq!(buffer.search.matches, vec![TextRange::new(1, 2)]);
    }
}
