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

#[derive(Clone, Debug, Default)]
pub struct NoteDocument {
    content: String,
    path: Option<PathBuf>,
    dirty: bool,
    open: bool,
    pub vim_state: VimState,
    cursor_line: usize,
    cursor_col: usize,
    count: Option<usize>,
    registers: Registers,
    selected_register: Option<RegisterTarget>,
    undo_stack: Vec<DocumentSnapshot>,
    redo_stack: Vec<DocumentSnapshot>,
    last_change: Option<Vec<String>>,
    replaying_change: bool,
    visual_anchor_flat: Option<usize>,
    last_visual_selection: Option<(TextRange, VisualKind)>,
    search: SearchState,
    marks: BTreeMap<char, CursorPosition>,
    jump_list: Vec<CursorPosition>,
    jump_index: Option<usize>,
    yank_highlight: Option<TextRange>,
    deferred_action: Option<DeferredAction>,
    pub(crate) defer_enabled: bool,
    pub pending_ex_action: Option<ExCommandAction>,
    changelist: Vec<CursorPosition>,
    changelist_index: Option<usize>,
    pub viewport_state: ViewportState,
    pub settings: DocumentSettings,
    command_history: Vec<String>,
    search_history: Vec<String>,
    last_substitute: Option<LastSubstitute>,
    macro_depth: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ViewportState {
    pub top_line: usize,
    pub visible_lines: usize,
}

impl NoteDocument {
    pub fn new_blank(&mut self) {
        self.content.clear();
        self.path = None;
        self.dirty = false;
        self.open = true;
        self.vim_state = VimState::default();
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.count = None;
        self.registers = Registers::default();
        self.selected_register = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.last_change = None;
        self.replaying_change = false;
        self.visual_anchor_flat = None;
        self.last_visual_selection = None;
        self.search = SearchState::default();
        self.marks.clear();
        self.jump_list.clear();
        self.jump_index = None;
        self.yank_highlight = None;
        self.defer_enabled = false;
        self.pending_ex_action = None;
        self.changelist.clear();
        self.changelist_index = None;
        self.viewport_state = ViewportState::default();
    }

    pub fn open(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let path = path.as_ref();
        self.content = fs::read_to_string(path)?;
        self.path = Some(path.to_path_buf());
        self.dirty = false;
        self.open = true;
        self.vim_state = VimState::default();
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.count = None;
        self.registers = Registers::default();
        self.selected_register = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.last_change = None;
        self.replaying_change = false;
        self.visual_anchor_flat = None;
        self.last_visual_selection = None;
        self.search = SearchState::default();
        self.marks.clear();
        self.jump_list.clear();
        self.jump_index = None;
        self.yank_highlight = None;
        self.deferred_action = None;
        self.pending_ex_action = None;
        Ok(())
    }

    pub fn save(&mut self) -> std::io::Result<()> {
        let Some(path) = self.path.clone() else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "No file path selected",
            ));
        };
        self.save_as(path)
    }

    pub fn save_as(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let path = path.as_ref();
        fs::write(path, &self.content)?;
        self.path = Some(path.to_path_buf());
        self.dirty = false;
        Ok(())
    }

    pub fn content_mut(&mut self) -> &mut String {
        &mut self.content
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn content_with_cursor_marker(&self) -> String {
        self.lines_vec()
            .iter()
            .enumerate()
            .map(|(line_index, line)| self.line_with_cursor_marker(line, line_index))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn mode(&self) -> VimMode {
        self.vim_state.mode
    }

    pub fn cursor_line(&self) -> usize {
        self.cursor_line.min(self.line_count().saturating_sub(1))
    }

    pub fn set_cursor_line(&mut self, line: usize) {
        self.cursor_line = line;
    }

    pub fn cursor_col(&self) -> usize {
        self.cursor_col.min(self.current_line_max_col())
    }

    pub fn set_cursor_col(&mut self, col: usize) {
        self.cursor_col = col;
    }

    pub fn viewport_top_line(&self) -> usize {
        self.viewport_state.top_line
    }

    pub fn set_viewport_top_line(&mut self, top_line: usize) {
        self.viewport_state.top_line = top_line;
    }

    pub fn display_cursor_col(&self) -> usize {
        match self.vim_state.mode {
            VimMode::Normal | VimMode::Visual | VimMode::VisualLine => self.cursor_col(),
            VimMode::Insert => self.cursor_col.min(self.current_line_char_count()),
            _ => self.cursor_col(),
        }
    }

    pub fn line_selection_cols(&self, line: usize) -> Option<(usize, usize)> {
        let check_range = |range: TextRange| -> Option<(usize, usize)> {
            let start_line = self.line_for_flat(range.start);
            let end_line = self.line_for_flat(range.end.saturating_sub(1));
            
            let min_line = start_line.min(end_line);
            let max_line = start_line.max(end_line);

            if (min_line..=max_line).contains(&line) {
                let start_col = if line == min_line {
                    let min_flat = range.start.min(range.end.saturating_sub(1));
                    min_flat - self.flat_index_for_line(line)
                } else {
                    0
                };

                let end_col = if line == max_line {
                    let max_flat = range.start.max(range.end);
                    max_flat - self.flat_index_for_line(line)
                } else {
                    usize::MAX
                };

                Some((start_col, end_col))
            } else {
                None
            }
        };

        if let Some(range) = self.yank_highlight {
            if let Some(cols) = check_range(range) {
                return Some(cols);
            }
        }
        
        self.visual_selection_range().and_then(check_range)
    }

    pub fn has_yank_highlight(&self) -> bool {
        self.yank_highlight.is_some()
    }

    pub fn clear_yank_highlight(&mut self) {
        self.yank_highlight = None;
    }

    pub fn has_deferred_action(&self) -> bool {
        self.deferred_action.is_some()
    }

    pub fn flush_deferred_action(&mut self) -> bool {
        if let Some(action) = self.deferred_action.take() {
            self.yank_highlight = None;
            self.apply_operator_range_direct(action.operator, action.range)
        } else {
            false
        }
    }

    pub fn line_has_search_match(&self, line: usize) -> bool {
        if !self.search.highlights_active {
            return false;
        }
        self.search
            .matches
            .iter()
            .any(|range| self.line_for_flat(range.start) == line)
    }

    pub fn search_pattern(&self) -> Option<&str> {
        (!self.search.pattern.is_empty()).then_some(self.search.pattern.as_str())
    }

    pub fn unnamed_register_text(&self) -> &str {
        &self.registers.unnamed.text
    }

    pub fn yank_register_text(&self) -> &str {
        &self.registers.yank.text
    }

    pub fn unnamed_register_is_linewise(&self) -> bool {
        self.registers.unnamed.linewise
    }

    pub fn yank_register_is_linewise(&self) -> bool {
        self.registers.yank.linewise
    }

    pub fn unnamed_register_snapshot(&self) -> RegisterSnapshot {
        RegisterSnapshot {
            text: self.registers.unnamed.text.clone(),
            linewise: self.registers.unnamed.linewise,
            blockwise: self.registers.unnamed.blockwise,
            version: self.registers.version,
        }
    }

    pub fn set_unnamed_register(&mut self, text: String, linewise: bool) {
        self.registers.unnamed = RegisterValue { text, linewise, blockwise: false };
    }

    pub fn set_clipboard_register(&mut self, text: String, linewise: bool) {
        let value = RegisterValue { text, linewise, blockwise: false };
        self.registers.clipboard = value.clone();
        self.registers.unnamed = value;
    }

    pub fn clipboard_register_text(&self) -> &str {
        &self.registers.clipboard.text
    }

    pub fn named_register_text(&self, name: char) -> Option<&str> {
        self.registers
            .named
            .get(&name.to_ascii_lowercase())
            .map(|value| value.text.as_str())
    }

    pub fn set_cursor_from_pointer(&mut self, line: usize, column: usize) {
        self.flush_deferred_action();
        self.clear_yank_highlight();
        self.vim_state.pending_command = None;
        self.count = None;
        self.visual_anchor_flat = None;
        if matches!(self.vim_state.mode, VimMode::Visual | VimMode::VisualLine) {
            self.vim_state.mode = VimMode::Normal;
        }
        self.cursor_line = line.min(self.line_count().saturating_sub(1));
        self.cursor_col = match self.vim_state.mode {
            VimMode::Normal | VimMode::Visual | VimMode::VisualLine => {
                column.min(self.current_line_max_col())
            }
            VimMode::Insert => column.min(self.current_line_char_count()),
            _ => column.min(self.current_line_char_count()),
        };
    }

    pub fn enter_insert(&mut self) {
        self.record_undo();
        self.vim_state.mode = VimMode::Insert;
        self.vim_state.pending_command = None;
        self.count = None;
        self.visual_anchor_flat = None;
        self.vim_state.insert_start_pos = Some(self.flattened_cursor());
        self.vim_state.macro_insert_start_index = Some(self.vim_state.current_macro.len());
        self.vim_state.insert_pending = None;
    }

    fn enter_replace_mode(&mut self) {
        self.record_undo();
        self.vim_state.mode = VimMode::Replace;
        self.vim_state.pending_command = None;
        self.count = None;
        self.visual_anchor_flat = None;
        self.vim_state.insert_start_pos = Some(self.flattened_cursor());
        self.vim_state.macro_insert_start_index = Some(self.vim_state.current_macro.len());
        self.vim_state.insert_pending = None;
    }

    pub fn enter_normal(&mut self) {
        self.flush_deferred_action();
        self.clear_yank_highlight();
        if self.vim_state.mode == VimMode::Insert || self.vim_state.mode == VimMode::Replace {
            self.vim_state.last_insert_pos = Some(self.flattened_cursor());
            if !self.replaying_change {
                if let (Some(start), Some(macro_idx)) = (self.vim_state.insert_start_pos, self.vim_state.macro_insert_start_index) {
                    let end = self.flattened_cursor();
                    let mut net_sequence = Vec::new();
                    if end < start {
                        for _ in 0..(start - end) {
                            net_sequence.push("backspace".to_string());
                        }
                    } else if end > start {
                        let text = self.text_for_range(TextRange::new(start, end));
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
        self.clear_command_line();
        self.count = None;
        self.visual_anchor_flat = None;
        self.vim_state.insert_pending = None;
        self.clamp_cursor_normal();
    }

    pub fn cancel_insert(&mut self) {
        self.flush_deferred_action();
        self.clear_yank_highlight();
        if self.vim_state.mode == VimMode::Insert {
            self.vim_state.last_insert_pos = Some(self.flattened_cursor());
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
        self.clear_command_line();
        self.count = None;
        self.visual_anchor_flat = None;
        self.vim_state.insert_pending = None;
        // Deliberately no clamp_cursor_normal here (cursor stays after text)
    }

    pub fn begin_insert_register_paste(&mut self) {
        if matches!(self.vim_state.mode, VimMode::Insert | VimMode::Replace) {
            self.vim_state.insert_pending = Some(InsertPending::RegisterPaste);
        }
    }

    pub fn resolve_insert_register_paste(&mut self, key: &str) -> bool {
        if self.vim_state.insert_pending != Some(InsertPending::RegisterPaste) {
            return false;
        }
        let Some(register_name) = normalize_register_name(key) else {
            self.vim_state.insert_pending = None;
            return false;
        };
        self.vim_state.insert_pending = None;
        let register = self.registers.register(register_name).clone();
        if register.text.is_empty() {
            return false;
        }
        self.handle_insert_text(&register.text);
        true
    }

    fn clear_command_line(&mut self) {
        self.vim_state.command_line.input.clear();
        self.vim_state.command_line.cursor = 0;
        self.vim_state.command_line.history_index = None;
        self.vim_state.command_line.saved_current = None;
        self.vim_state.command_line.is_search = false;
    }

    fn begin_command_line(&mut self, mode: VimMode, initial: &str, is_search: bool) {
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

    pub fn handle_editor_key(&mut self, key: crate::vim::key::EditorKey) -> bool {
        self.flush_deferred_action();
        self.clear_yank_highlight();
        
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
                    self.enter_normal();
                    true
                }
                crate::vim::key::EditorKey::Cancel => {
                    self.cancel_insert();
                    true
                }
                crate::vim::key::EditorKey::RegisterPaste => {
                    self.begin_insert_register_paste();
                    true
                }
                crate::vim::key::EditorKey::Newline => {
                    self.insert_newline();
                    true
                }
                crate::vim::key::EditorKey::Backspace => {
                    self.backspace();
                    true
                }
                crate::vim::key::EditorKey::Delete => {
                    self.delete_at_cursor();
                    true
                }
                crate::vim::key::EditorKey::DeleteWord => {
                    self.delete_word_insert();
                    true
                }
                crate::vim::key::EditorKey::DeleteLine => {
                    self.delete_line_insert();
                    true
                }
                crate::vim::key::EditorKey::Input(text) => {
                    if self.resolve_insert_register_paste(&text) {
                        return true;
                    }
                    self.handle_insert_text(&text);
                    true
                }
                crate::vim::key::EditorKey::Ignore => false,
            }
        } else {
            match key {
                crate::vim::key::EditorKey::EnterNormal => {
                    self.enter_normal();
                    true
                }
                crate::vim::key::EditorKey::Input(text) => {
                    self.handle_normal_input(&text)
                }
                _ => false,
            }
        }
    }

    pub fn handle_normal_input(&mut self, input: &str) -> bool {
        self.flush_deferred_action();
        self.clear_yank_highlight();
        if self.vim_state.mode == VimMode::Insert {
            return false;
        }

        if let Some(PendingCommand::SubstituteConfirm { .. }) = &self.vim_state.pending_command {
            return self.handle_substitute_confirm_input(input);
        }

        if matches!(self.vim_state.mode, VimMode::Command | VimMode::Search(_)) {
            return self.handle_single_key_event(input);
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
            return self.handle_single_key_event(input);
        }

        let mut changed = false;
        
        if input == "escape" || input == "backspace" || input == "return"
            || input.starts_with("ctrl+") || input.starts_with("alt+") || input.starts_with("shift+")
        {
            if !self.replaying_change {
                self.vim_state.current_macro.push(input.to_string());
            }
            changed = self.handle_single_key_event(input);
        } else {
            for ch in input.chars() {
                if !self.replaying_change {
                    self.vim_state.current_macro.push(ch.to_string());
                }
                changed |= self.handle_single_key_event(&ch.to_string());
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

    fn handle_substitute_confirm_input(&mut self, input: &str) -> bool {
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
                    let chars: Vec<char> = self.content.chars().collect();
                    let prefix: String = chars[..m.start].iter().collect();
                    let suffix: String = chars[m.end..].iter().collect();
                    self.content = format!("{}{}{}", prefix, replacement, suffix);
                    self.dirty = true;

                    let shift = replacement.chars().count() as isize - match_len as isize;
                    for rem in matches.iter_mut().skip(match_index + 1) {
                        rem.start = (rem.start as isize + shift) as usize;
                        rem.end = (rem.end as isize + shift) as usize;
                    }

                    let next_index = match_index + 1;
                    if next_index < matches.len() {
                        let next_match = matches[next_index];
                        self.yank_highlight = Some(next_match);
                        self.set_cursor_from_flat(next_match.start);
                        self.vim_state.pending_command = Some(PendingCommand::SubstituteConfirm {
                            pattern,
                            replacement,
                            matches,
                            match_index: next_index,
                            flags,
                        });
                    } else {
                        self.yank_highlight = None;
                        self.search.pattern = pattern;
                        self.refresh_search_matches();
                    }
                    true
                }
                "n" => {
                    let next_index = match_index + 1;
                    if next_index < matches.len() {
                        let next_match = matches[next_index];
                        self.yank_highlight = Some(next_match);
                        self.set_cursor_from_flat(next_match.start);
                        self.vim_state.pending_command = Some(PendingCommand::SubstituteConfirm {
                            pattern,
                            replacement,
                            matches,
                            match_index: next_index,
                            flags,
                        });
                    } else {
                        self.yank_highlight = None;
                        self.search.pattern = pattern;
                        self.refresh_search_matches();
                    }
                    false
                }
                "a" => {
                    let mut content_chars: Vec<char> = self.content.chars().collect();
                    for idx in (match_index..matches.len()).rev() {
                        let m = matches[idx];
                        let prefix: Vec<char> = content_chars[..m.start].to_vec();
                        let suffix: Vec<char> = content_chars[m.end..].to_vec();
                        let replacement_chars: Vec<char> = replacement.chars().collect();
                        content_chars = [prefix, replacement_chars, suffix].concat();
                    }
                    self.content = content_chars.into_iter().collect();
                    self.dirty = true;
                    self.yank_highlight = None;
                    self.search.pattern = pattern;
                    self.refresh_search_matches();
                    true
                }
                "l" => {
                    let m = matches[match_index];
                    let chars: Vec<char> = self.content.chars().collect();
                    let prefix: String = chars[..m.start].iter().collect();
                    let suffix: String = chars[m.end..].iter().collect();
                    self.content = format!("{}{}{}", prefix, replacement, suffix);
                    self.dirty = true;
                    self.yank_highlight = None;
                    self.search.pattern = pattern;
                    self.refresh_search_matches();
                    true
                }
                "q" | "escape" | "ctrl+[" => {
                    self.yank_highlight = None;
                    self.search.pattern = pattern;
                    self.refresh_search_matches();
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

    fn handle_single_key_event(&mut self, key: &str) -> bool {
        match self.vim_state.mode {
            VimMode::Command => {
                self.handle_command_mode_input(key)
            }
            VimMode::Search(dir) => {
                self.handle_search_mode_input(dir, key)
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
                    changed |= self.handle_visual_char(ch);
                }
                changed
            }
            VimMode::Normal => {
                if key == "ctrl+r" { return self.redo(); }
                if key == "ctrl+o" { return self.jump_history(-1); }
                if key == "ctrl+i" { return self.jump_history(1); }
                if key == "ctrl+d" { return self.scroll_viewport_half_page(true); }
                if key == "ctrl+u" { return self.scroll_viewport_half_page(false); }
                if key == "ctrl+f" { return self.scroll_viewport_full_page(true); }
                if key == "ctrl+b" { return self.scroll_viewport_full_page(false); }
                if key == "ctrl+e" { return self.scroll_viewport_line(true); }
                if key == "ctrl+y" { return self.scroll_viewport_line(false); }
                if key == "ctrl+v" { self.enter_visual(VisualKind::Block); return false; }
                if key == "." { return self.repeat_last_change(); }
                let mut changed = false;
                for ch in key.chars() {
                    changed |= self.handle_normal_char(ch);
                }
                changed
            }
            VimMode::Insert | VimMode::Replace => false,
        }
    }

    pub fn handle_insert_text(&mut self, text: &str) {
        self.flush_deferred_action();
        self.clear_yank_highlight();
        let mode = self.vim_state.mode;
        if (mode != VimMode::Insert && mode != VimMode::Replace) || text.is_empty() {
            return;
        }

        if !self.replaying_change {
            self.vim_state.current_macro.push(text.to_string());
        }

        let mut lines = self.lines_vec();
        let line_index = self.cursor_line().min(lines.len().saturating_sub(1));
        let col = self.cursor_col.min(char_count(&lines[line_index]));
        
        if mode == VimMode::Replace {
            let chars: Vec<char> = lines[line_index].chars().collect();
            let text_chars: Vec<char> = text.chars().collect();
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
            self.cursor_col += text.chars().count();
        } else {
            insert_str_at_char(&mut lines[line_index], col, text);
            self.cursor_line = line_index;
            self.cursor_col += text.chars().count();
        }
        self.replace_lines_keep_insert(lines);
    }

    pub fn insert_newline(&mut self) {
        self.flush_deferred_action();
        self.clear_yank_highlight();
        if self.vim_state.mode != VimMode::Insert {
            return;
        }

        let mut lines = self.lines_vec();
        let line_index = self.cursor_line().min(lines.len().saturating_sub(1));
        let col = self.cursor_col.min(char_count(&lines[line_index]));
        let tail = split_off_at_char(&mut lines[line_index], col);
        lines.insert(line_index + 1, tail);
        self.cursor_line = line_index + 1;
        self.cursor_col = 0;
        self.replace_lines_keep_insert(lines);
    }

    pub fn delete_word_insert(&mut self) {
        self.flush_deferred_action();
        self.clear_yank_highlight();
        if self.vim_state.mode != VimMode::Insert { return; }
        
        self.record_undo();
        if !self.replaying_change {
            self.vim_state.current_macro.push("ctrl+w".to_string());
        }
        
        let mut lines = self.lines_vec();
        let line_index = self.cursor_line().min(lines.len().saturating_sub(1));
        let col = self.cursor_col.min(char_count(&lines[line_index]));
        
        if col == 0 {
            if line_index > 0 {
                let prev_len = char_count(&lines[line_index - 1]);
                let current_line = lines.remove(line_index);
                lines[line_index - 1].push_str(&current_line);
                self.cursor_line -= 1;
                self.cursor_col = prev_len;
                self.replace_lines_keep_insert(lines);
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
        self.replace_lines_keep_insert(lines);
    }

    pub fn delete_line_insert(&mut self) {
        self.flush_deferred_action();
        self.clear_yank_highlight();
        if self.vim_state.mode != VimMode::Insert { return; }
        
        self.record_undo();
        if !self.replaying_change {
            self.vim_state.current_macro.push("ctrl+u".to_string());
        }
        
        let mut lines = self.lines_vec();
        let line_index = self.cursor_line().min(lines.len().saturating_sub(1));
        let col = self.cursor_col.min(char_count(&lines[line_index]));
        
        if col == 0 {
            if line_index > 0 {
                let prev_len = char_count(&lines[line_index - 1]);
                let current_line = lines.remove(line_index);
                lines[line_index - 1].push_str(&current_line);
                self.cursor_line -= 1;
                self.cursor_col = prev_len;
                self.replace_lines_keep_insert(lines);
            }
            return;
        }

        let chars: Vec<char> = lines[line_index].chars().collect();
        let target_col = 0;
        let new_line: String = chars[..target_col].iter().chain(chars[col..].iter()).collect();
        lines[line_index] = new_line;
        self.cursor_col = target_col;
        self.replace_lines_keep_insert(lines);
    }

    pub fn backspace(&mut self) {
        self.flush_deferred_action();
        self.clear_yank_highlight();
        if self.vim_state.mode != VimMode::Insert {
            return;
        }

        let mut lines = self.lines_vec();
        let line_index = self.cursor_line().min(lines.len().saturating_sub(1));
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
        self.replace_lines_keep_insert(lines);
    }

    pub fn delete_at_cursor(&mut self) {
        self.flush_deferred_action();
        self.clear_yank_highlight();
        if self.vim_state.mode != VimMode::Insert {
            return;
        }

        let mut lines = self.lines_vec();
        let line_index = self.cursor_line().min(lines.len().saturating_sub(1));
        let col = self.cursor_col.min(char_count(&lines[line_index]));
        if col < char_count(&lines[line_index]) {
            remove_char_at(&mut lines[line_index], col);
        } else if line_index + 1 < lines.len() {
            let next = lines.remove(line_index + 1);
            lines[line_index].push_str(&next);
        }
        self.replace_lines_keep_insert(lines);
    }

    pub fn title(&self) -> String {
        let title = self
            .path
            .as_deref()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled");

        title.to_string()
    }

    pub fn path_string(&self) -> Option<String> {
        self.path.as_ref().map(|path| path.display().to_string())
    }

    pub fn stats(&self) -> NoteStats {
        let line_count = self.content.split('\n').count().max(1);
        let word_count = self.content.split_whitespace().count();
        let char_count = self.content.chars().count();
        NoteStats {
            line_count,
            word_count,
            char_count,
        }
    }

    fn handle_normal_char(&mut self, ch: char) -> bool {
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
            }
            if pending == PendingCommand::SmallZPrefix {
                match ch {
                    'z' => return self.cursor_to_center(),
                    't' => return self.cursor_to_top(),
                    'b' => return self.cursor_to_bottom(),
                    _ => {}
                }
                return false;
            }
            return self.handle_pending_normal_char(pending, ch);
        }
 
        let explicit_count = self.count.take();
        let count = explicit_count.unwrap_or(1).max(1);
        match ch {
            '"' => {
                self.vim_state.pending_command = Some(PendingCommand::RegisterPrefix);
                false
            }
            ':' => {
                self.begin_command_line(VimMode::Command, "", false);
                false
            }
            '/' | '?' => {
                self.begin_command_line(
                    VimMode::Search(if ch == '/' {
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
            'n' => self.repeat_search(false),
            'N' => self.repeat_search(true),
            '*' => self.search_word_under_cursor(false),
            '#' => self.search_word_under_cursor(true),
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
                self.enter_visual(VisualKind::Character);
                false
            }
            'V' => {
                self.enter_visual(VisualKind::Line);
                false
            }
            'g' if explicit_count.is_none() => {
                self.vim_state.pending_command = Some(PendingCommand::Goto {
                    count: explicit_count,
                });
                false
            }
            'u' => self.undo(),
            'R' => {
                self.enter_replace_mode();
                false
            }
            'i' => {
                self.enter_insert();
                false
            }
            'I' => {
                self.cursor_col = self.first_non_blank_col();
                self.enter_insert();
                false
            }
            'a' => {
                self.cursor_col = (self.cursor_col() + 1).min(self.current_line_char_count());
                self.enter_insert();
                false
            }
            'A' => {
                self.cursor_col = self.current_line_char_count();
                self.enter_insert();
                false
            }
            'o' => {
                self.insert_blank_line(self.cursor_line() + 1);
                self.enter_insert();
                true
            }
            'O' => {
                self.insert_blank_line(self.cursor_line());
                self.enter_insert();
                true
            }
            'j' => {
                self.move_cursor_line(count as isize);
                false
            }
            'k' => {
                self.move_cursor_line(-(count as isize));
                false
            }
            '+' => {
                self.move_first_nonblank_on_relative_line(count as isize);
                false
            }
            '-' => {
                self.move_first_nonblank_on_relative_line(-(count as isize));
                false
            }
            '_' => {
                self.move_first_nonblank_on_relative_line(count.saturating_sub(1) as isize);
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
                self.repeat_find_char(ch == ';', count)
            }
            '%' | '{' | '}' | '(' | ')' => {
                self.push_jump();
                if let Some(target) = self.extended_motion_flat(ch, count) {
                    self.set_cursor_from_flat(target);
                }
                false
            }
            'H' | 'M' | 'L' => {
                self.push_jump();
                if let Some(target_line) = self.extended_motion_line(ch, count) {
                    self.cursor_line = target_line;
                    self.clamp_cursor_normal();
                }
                false
            }
            'h' => {
                self.move_cursor_col(-(count as isize));
                false
            }
            'l' => {
                self.move_cursor_col(count as isize);
                false
            }
            'w' | 'W' => {
                self.move_word_forward(count, ch == 'W');
                false
            }
            'b' | 'B' => {
                self.move_word_backward(count, ch == 'B');
                false
            }
            'e' | 'E' => {
                self.move_word_end(count, ch == 'E');
                false
            }
            '|' => {
                self.cursor_col = explicit_count.unwrap_or(1).saturating_sub(1).min(self.current_line_max_col());
                false
            }
            '0' => {
                self.cursor_col = 0;
                false
            }
            '^' => {
                self.cursor_col = self.first_non_blank_col();
                false
            }
            '$' => {
                self.cursor_col = self.current_line_max_col();
                false
            }
            'G' => {
                self.push_jump();
                self.cursor_line = if let Some(count) = explicit_count {
                    count.saturating_sub(1)
                } else {
                    self.line_count().saturating_sub(1)
                };
                self.clamp_cursor_normal();
                false
            }
            'g' => {
                self.vim_state.pending_command = Some(PendingCommand::Goto {
                    count: explicit_count,
                });
                false
            }
            'd' => {
                self.vim_state.pending_command = Some(PendingCommand::Operator {
                    operator: Operator::Delete,
                    count,
                });
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
            'Y' => self.apply_current_lines_operator(Operator::Yank, count),
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
            'p' => self.paste_unnamed(count, PastePlacement::After),
            'P' => self.paste_unnamed(count, PastePlacement::Before),
            'x' => self.delete_chars_on_current_line(count),
            'X' => self.delete_chars_before_cursor(count),
            'r' => {
                self.vim_state.pending_command = Some(PendingCommand::ReplaceChar { count });
                false
            }
            's' => self.substitute_chars(count),
            'S' => self.apply_current_lines_operator(Operator::Change, count),
            'D' => self.apply_operator_motion(Operator::Delete, count, '$'),
            'C' => self.apply_operator_motion(Operator::Change, count, '$'),
            'J' => self.join_lines(count, true),
            'q' => {
                if self.vim_state.recording_macro.is_some() {
                    self.stop_macro_recording();
                } else {
                    self.vim_state.pending_command = Some(PendingCommand::MacroRecordPrefix);
                }
                false
            }
            '@' => {
                self.vim_state.pending_command = Some(PendingCommand::MacroReplayPrefix { count });
                false
            }
            '~' => self.toggle_case_chars(count),
            _ => false,
        }
    }

    fn handle_pending_normal_char(&mut self, pending: PendingCommand, ch: char) -> bool {
        match (pending, ch) {
            (PendingCommand::RegisterPrefix, register) => {
                self.vim_state.pending_command = None;
                self.selected_register = RegisterTarget::from_prefix(register);
                false
            }
            (PendingCommand::MarkSet, mark) if mark.is_ascii_alphabetic() => {
                self.marks
                    .insert(mark.to_ascii_lowercase(), self.cursor_position());
                false
            }
            (PendingCommand::MarkJump, mark) if mark.is_ascii_alphabetic() => {
                let Some(position) = self.marks.get(&mark.to_ascii_lowercase()).copied() else {
                    return false;
                };
                self.push_jump();
                self.cursor_line = position.line;
                self.cursor_col = self.first_non_blank_col_for_line(position.line);
                self.clamp_cursor_normal();
                false
            }
            (PendingCommand::MarkJumpExact, mark) if mark.is_ascii_alphabetic() => {
                let Some(position) = self.marks.get(&mark.to_ascii_lowercase()).copied() else {
                    return false;
                };
                self.push_jump();
                self.cursor_line = position.line;
                self.cursor_col = position.col;
                self.clamp_cursor_normal();
                false
            }
            (PendingCommand::ReplaceChar { count }, replacement) => self.replace_chars(replacement, count),
            (
                PendingCommand::Operator {
                    operator: Operator::Delete,
                    count,
                },
                'd',
            ) => {
                self.vim_state.pending_command = None;
                let multiplier = self.count.take().unwrap_or(1).max(1);
                self.apply_current_lines_operator(Operator::Delete, count.saturating_mul(multiplier))
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
                self.apply_current_lines_operator(Operator::Change, count.saturating_mul(multiplier))
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
                self.apply_current_lines_operator(Operator::Yank, count.saturating_mul(multiplier))
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
                self.apply_current_lines_operator(Operator::Indent, count.saturating_mul(multiplier))
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
                self.apply_current_lines_operator(Operator::Outdent, count.saturating_mul(multiplier))
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
                self.apply_current_lines_operator(Operator::Format, count.saturating_mul(multiplier))
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
                self.apply_operator_motion(Operator::Change, count, 'w')
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
                self.apply_operator_motion(operator, count.saturating_mul(motion_count), motion)
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
                self.execute_find_char(find_ch, is_t, is_forward, count)
            }
            (PendingCommand::OperatorThenFindChar { operator, operator_count, is_t, is_forward, find_count }, find_ch) => {
                self.vim_state.pending_command = None;
                self.execute_operator_find_char(operator, operator_count, find_ch, is_t, is_forward, find_count)
            }
            (PendingCommand::SurroundWaitAdd { range, mut buffer }, ch) => {
                buffer.push(ch);
                match crate::vim::surround::SurroundSpec::parse(&buffer) {
                    crate::vim::surround::ParseResult::Complete(spec) => {
                        self.vim_state.pending_command = None;
                        self.apply_surround_add(range, spec);
                        true
                    }
                    crate::vim::surround::ParseResult::Incomplete => {
                        self.vim_state.pending_command = Some(PendingCommand::SurroundWaitAdd { range, buffer });
                        false
                    }
                    crate::vim::surround::ParseResult::Invalid => {
                        self.vim_state.pending_command = None;
                        false
                    }
                }
            }
            (PendingCommand::SurroundWaitDelete { mut buffer }, ch) => {
                buffer.push(ch);
                match crate::vim::surround::SurroundSpec::parse(&buffer) {
                    crate::vim::surround::ParseResult::Complete(spec) => {
                        self.vim_state.pending_command = None;
                        self.apply_surround_delete(spec);
                        true
                    }
                    crate::vim::surround::ParseResult::Incomplete => {
                        self.vim_state.pending_command = Some(PendingCommand::SurroundWaitDelete { buffer });
                        false
                    }
                    crate::vim::surround::ParseResult::Invalid => {
                        self.vim_state.pending_command = None;
                        false
                    }
                }
            }
            (PendingCommand::SurroundWaitChange { mut buffer }, ch) => {
                buffer.push(ch);
                
                // For change, we need TWO specs: the old one to delete, and the new one to add!
                // Let's implement a simple parser for two specs, or just wait.
                // Wait! The user types `cs"'` which means `buffer` will contain `"'`.
                // We could parse it by attempting to parse a prefix, then if complete, parse the rest!
                // To keep it simple, `cs` is always two single characters, or `cst<span>` (char then tag).
                // Let's do this: we first parse the `old` spec. If incomplete, we wait.
                // If complete, we check if there's more text. If there is, we parse the `new` spec.
                // If the new spec is complete, we apply it. If incomplete, we wait.
                
                let mut parsed_old = None;
                let mut old_len = 0;
                
                // Find the boundary
                for i in 1..=buffer.len() {
                    let old_part = &buffer[0..i];
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
                    let new_part = &buffer[old_len..];
                    match crate::vim::surround::SurroundSpec::parse(new_part) {
                        crate::vim::surround::ParseResult::Complete(new_spec) => {
                            self.vim_state.pending_command = None;
                            self.apply_surround_change(old_spec, new_spec);
                            return true;
                        }
                        crate::vim::surround::ParseResult::Incomplete => {
                            self.vim_state.pending_command = Some(PendingCommand::SurroundWaitChange { buffer });
                            return false;
                        }
                        crate::vim::surround::ParseResult::Invalid => {
                            self.vim_state.pending_command = None;
                            return false;
                        }
                    }
                } else {
                    // Still parsing old spec
                    match crate::vim::surround::SurroundSpec::parse(&buffer) {
                        crate::vim::surround::ParseResult::Incomplete => {
                            self.vim_state.pending_command = Some(PendingCommand::SurroundWaitChange { buffer });
                            return false;
                        }
                        _ => {
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
                self.apply_text_object_operator(
                    operator,
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
                object @ ('\'' | '"' | '(' | ')' | '[' | ']' | '{' | '}'),
            ) => {
                let motion_count = self.count.take().unwrap_or(1).max(1);
                self.vim_state.pending_command = None;
                self.apply_text_object_operator(
                    operator,
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
                self.apply_text_object_operator(
                    operator,
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
                self.apply_visual_text_object(
                    motion_count,
                    around,
                    TextObject::Word {
                        big_word: object == 'W',
                    },
                )
            }
            (
                PendingCommand::VisualTextObject { around },
                object @ ('\'' | '"' | '(' | ')' | '[' | ']' | '{' | '}'),
            ) => {
                let motion_count = self.count.take().unwrap_or(1).max(1);
                self.vim_state.pending_command = None;
                self.apply_visual_text_object(
                    motion_count,
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
                self.apply_visual_text_object(
                    motion_count,
                    around,
                    object,
                )
            }
            (PendingCommand::Goto { count }, 'g') => {
                self.push_jump();
                self.vim_state.pending_command = None;
                self.cursor_line = count
                    .or_else(|| self.count.take())
                    .unwrap_or(1)
                    .saturating_sub(1);
                self.clamp_cursor_normal();
                false
            }
            (PendingCommand::Goto { count: _ }, 'v') => {
                self.vim_state.pending_command = None;
                self.restore_last_visual_selection()
            }
            (PendingCommand::Goto { count }, ';') => {
                self.vim_state.pending_command = None;
                let steps = count.or(self.count.take()).unwrap_or(1) as isize;
                self.navigate_changelist(-steps)
            }
            (PendingCommand::Goto { count }, ',') => {
                self.vim_state.pending_command = None;
                let steps = count.or(self.count.take()).unwrap_or(1) as isize;
                self.navigate_changelist(steps)
            }
            (PendingCommand::Goto { count: _ }, 'i') => {
                self.vim_state.pending_command = None;
                if let Some(pos) = self.vim_state.last_insert_pos {
                    self.set_cursor_from_flat(pos);
                    self.vim_state.mode = VimMode::Insert;
                }
                false
            }
            (PendingCommand::Goto { count }, 'e') => {
                self.vim_state.pending_command = None;
                let steps = count.or(self.count.take()).unwrap_or(1).max(1);
                self.move_word_backward_end(steps, false);
                false
            }
            (PendingCommand::Goto { count }, 'E') => {
                self.vim_state.pending_command = None;
                let steps = count.or(self.count.take()).unwrap_or(1).max(1);
                self.move_word_backward_end(steps, true);
                false
            }
            (PendingCommand::Goto { count }, 'J') => {
                self.vim_state.pending_command = None;
                let steps = count.or(self.count.take()).unwrap_or(1).max(1);
                self.join_lines(steps, false)
            }
            (PendingCommand::MacroRecordPrefix, register) if register.is_ascii_alphabetic() => {
                self.vim_state.pending_command = None;
                self.start_macro_recording(register.to_ascii_lowercase());
                false
            }
            (PendingCommand::MacroRecordPrefix, 'q') => {
                self.vim_state.pending_command = None;
                self.stop_macro_recording();
                false
            }
            (PendingCommand::MacroReplayPrefix { count }, register) if register.is_ascii_alphabetic() => {
                self.vim_state.pending_command = None;
                self.play_macro(register.to_ascii_lowercase(), count)
            }
            (PendingCommand::MacroReplayPrefix { count }, '@') => {
                self.vim_state.pending_command = None;
                if let Some(last) = self.last_played_macro() {
                    self.play_macro(last, count)
                } else {
                    false
                }
            }
            (PendingCommand::OperatorOrCaseGoto { operator, count: _ }, 'g') => {
                let target_line = self.count.take().unwrap_or(1).saturating_sub(1);
                self.vim_state.pending_command = None;
                self.apply_operator_range(
                    operator,
                    self.linewise_range(self.cursor_line(), target_line),
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
                    self.operator_motion_range(count.saturating_mul(motion_count), motion)
                else {
                    return false;
                };
                self.apply_case_range_kind(range, kind)
            }
            (PendingCommand::CaseOperator { kind, count }, ch)
                if (kind == CaseKind::Lower && ch == 'u')
                    || (kind == CaseKind::Upper && ch == 'U')
                    || (kind == CaseKind::Toggle && ch == '~') => {
                self.vim_state.pending_command = None;
                let multiplier = self.count.take().unwrap_or(1).max(1);
                let total_count = count.saturating_mul(multiplier);
                let target_line = self.cursor_line().saturating_add(total_count.saturating_sub(1));
                let range = self.linewise_range(self.cursor_line(), target_line);
                self.apply_case_range_kind(range, kind)
            }
            _ => {
                self.vim_state.pending_command = None;
                self.count = None;
                self.selected_register = None;
                false
            }
        }
    }

    fn handle_visual_char(&mut self, ch: char) -> bool {
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
            return self.handle_pending_normal_char(pending, ch);
        }

        let count = self.count.take().unwrap_or(1).max(1);
        match ch {
            'v' if self.vim_state.mode == VimMode::Visual => {
                self.enter_normal();
                false
            }
            'V' if self.vim_state.mode == VimMode::VisualLine => {
                self.enter_normal();
                false
            }
            'R' => {
                self.vim_state.mode = VimMode::Replace;
                self.vim_state.insert_start_pos = Some(self.flattened_cursor());
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
                let cursor = self.flattened_cursor();
                self.visual_anchor_flat = Some(cursor);
                self.set_cursor_from_flat(anchor);
                false
            }
            'h' => {
                self.move_cursor_col(-(count as isize));
                false
            }
            'j' => {
                self.move_cursor_line(count as isize);
                false
            }
            'k' => {
                self.move_cursor_line(-(count as isize));
                false
            }
            'l' => {
                self.move_cursor_col(count as isize);
                false
            }
            'w' | 'W' => {
                self.move_word_forward(count, ch == 'W');
                false
            }
            'b' | 'B' => {
                self.move_word_backward(count, ch == 'B');
                false
            }
            'e' | 'E' => {
                self.move_word_end(count, ch == 'E');
                false
            }
            '0' => {
                self.cursor_col = 0;
                false
            }
            '+' => {
                self.move_first_nonblank_on_relative_line(count as isize);
                false
            }
            '-' => {
                self.move_first_nonblank_on_relative_line(-(count as isize));
                false
            }
            '_' => {
                self.move_first_nonblank_on_relative_line(count.saturating_sub(1) as isize);
                false
            }
            '|' => {
                self.cursor_col = count.saturating_sub(1).min(self.current_line_max_col());
                false
            }
            '^' => {
                self.cursor_col = self.first_non_blank_col();
                false
            }
            '$' => {
                self.cursor_col = self.current_line_max_col();
                false
            }
            'G' => {
                self.cursor_line = self
                    .cursor_line()
                    .saturating_add(count.saturating_sub(1))
                    .min(self.line_count().saturating_sub(1));
                self.clamp_cursor_normal();
                false
            }
            'I' if self.vim_state.mode == VimMode::VisualBlock => {
                let Some(range) = self.visual_selection_range() else { return false; };
                self.visual_anchor_flat = None;
                self.vim_state.mode = VimMode::Normal;
                
                let start_line = self.line_for_flat(range.start);
                let end_line = self.line_for_flat(range.end.saturating_sub(1));
                let top_line = start_line.min(end_line);
                let start_col = {
                    let line_start = self.line_start_flat(self.line_for_flat(range.start));
                    range.start.saturating_sub(line_start)
                };
                let end_col = {
                    let line_start = self.line_start_flat(self.line_for_flat(range.end.saturating_sub(1)));
                    range.end.saturating_sub(1).saturating_sub(line_start)
                };
                let left_col = start_col.min(end_col);
                
                self.cursor_line = top_line;
                self.cursor_col = left_col;
                self.deferred_action = Some(DeferredAction { operator: Operator::BlockInsert, range });
                self.enter_insert();
                false
            }
            'A' if self.vim_state.mode == VimMode::VisualBlock => {
                let Some(range) = self.visual_selection_range() else { return false; };
                self.visual_anchor_flat = None;
                self.vim_state.mode = VimMode::Normal;
                
                let start_line = self.line_for_flat(range.start);
                let end_line = self.line_for_flat(range.end.saturating_sub(1));
                let top_line = start_line.min(end_line);
                let start_col = {
                    let line_start = self.line_start_flat(self.line_for_flat(range.start));
                    range.start.saturating_sub(line_start)
                };
                let end_col = {
                    let line_start = self.line_start_flat(self.line_for_flat(range.end.saturating_sub(1)));
                    range.end.saturating_sub(1).saturating_sub(line_start)
                };
                let right_col = start_col.max(end_col);
                
                self.cursor_line = top_line;
                self.cursor_col = right_col; // wait, cursor_col needs to be one after?
                self.cursor_col = (right_col + 1).min(self.current_line_char_count());
                self.deferred_action = Some(DeferredAction { operator: Operator::BlockAppend, range });
                self.enter_insert();
                false
            }
            'd' | 'x' => self.apply_visual_operator(Operator::Delete),
            'c' => self.apply_visual_operator(Operator::Change),
            's' | 'S' => self.apply_visual_operator(Operator::SurroundAdd),
            'y' => self.apply_visual_operator(Operator::Yank),
            '>' => self.apply_visual_operator(Operator::Indent),
            '<' => self.apply_visual_operator(Operator::Outdent),
            '=' => self.apply_visual_operator(Operator::Format),
            'J' => {
                if let Some(range) = self.visual_selection_range() {
                    self.enter_normal();
                    let count = self.line_for_flat(range.end.saturating_sub(1)) - self.line_for_flat(range.start);
                    self.set_cursor_from_flat(range.start);
                    self.join_lines(count, true)
                } else {
                    false
                }
            }
            '~' => {
                let Some(range) = self.visual_selection_range() else {
                    return false;
                };
                self.last_visual_selection = Some((range, self.visual_kind()));
                self.visual_anchor_flat = None;
                self.vim_state.mode = VimMode::Normal;
                self.toggle_case_range(range)
            }
            'u' => {
                let Some(range) = self.visual_selection_range() else {
                    return false;
                };
                self.last_visual_selection = Some((range, self.visual_kind()));
                self.visual_anchor_flat = None;
                self.vim_state.mode = VimMode::Normal;
                self.apply_case_range(range, false)
            }
            'U' => {
                let Some(range) = self.visual_selection_range() else {
                    return false;
                };
                self.last_visual_selection = Some((range, self.visual_kind()));
                self.visual_anchor_flat = None;
                self.vim_state.mode = VimMode::Normal;
                self.apply_case_range(range, true)
            }
            ':' => {
                let range = self.visual_selection_range();
                let kind = self.visual_kind();
                if let Some(r) = range {
                    self.last_visual_selection = Some((r, kind));
                }
                self.begin_command_line(VimMode::Command, "'<,'>", false);
                false
            }
            '/' | '?' => {
                self.begin_command_line(
                    VimMode::Search(if ch == '/' {
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

    fn line_count(&self) -> usize {
        self.content.split('\n').count().max(1)
    }

    fn clamp_cursor_line(&mut self) {
        self.cursor_line = self.cursor_line();
        self.cursor_col = self.cursor_col();
    }

    fn clamp_cursor_normal(&mut self) {
        self.cursor_line = self.cursor_line();
        self.cursor_col = self.cursor_col.min(self.current_line_max_col());
    }

    fn move_cursor_line(&mut self, delta: isize) {
        let current = self.cursor_line() as isize;
        let max = self.line_count().saturating_sub(1) as isize;
        self.cursor_line = (current + delta).clamp(0, max) as usize;
        self.cursor_col = self.cursor_col.min(self.current_line_max_col());
    }

    fn move_cursor_col(&mut self, delta: isize) {
        let current = self.cursor_col() as isize;
        let max = self.current_line_max_col() as isize;
        self.cursor_col = (current + delta).clamp(0, max) as usize;
    }

    fn current_line_max_col(&self) -> usize {
        self.current_line_char_count().saturating_sub(1)
    }

    fn current_line_char_count(&self) -> usize {
        self.lines_vec()
            .get(self.cursor_line())
            .map(|line| line.chars().count())
            .unwrap_or(0)
    }

    fn lines_vec(&self) -> Vec<String> {
        if self.content.is_empty() {
            vec![String::new()]
        } else {
            self.content.split('\n').map(ToOwned::to_owned).collect()
        }
    }

    fn replace_lines(&mut self, lines: Vec<String>) {
        self.record_undo();
        self.push_change_location();
        self.content = lines.join("\n");
        self.dirty = true;
        self.clamp_cursor_line();
    }

    fn replace_lines_keep_insert(&mut self, lines: Vec<String>) {
        self.push_change_location();
        self.content = lines.join("\n");
        self.dirty = true;
        self.cursor_line = self.cursor_line.min(self.line_count().saturating_sub(1));
        self.cursor_col = self.cursor_col.min(self.current_line_char_count());
    }

    fn snapshot_document(&self) -> DocumentSnapshot {
        DocumentSnapshot {
            content: self.content.clone(),
            mode: self.vim_state.mode,
            cursor_line: self.cursor_line,
            cursor_col: self.cursor_col,
        }
    }

    fn restore_snapshot(&mut self, snapshot: DocumentSnapshot) {
        self.content = snapshot.content;
        self.vim_state.mode = snapshot.mode;
        self.cursor_line = snapshot.cursor_line;
        self.cursor_col = snapshot.cursor_col;
        self.vim_state.pending_command = None;
        self.count = None;
        self.selected_register = None;
        self.dirty = true;
        match self.vim_state.mode {
            VimMode::Normal | VimMode::Visual | VimMode::VisualLine => self.clamp_cursor_normal(),
            VimMode::Insert => {
                self.cursor_line = self.cursor_line.min(self.line_count().saturating_sub(1));
                self.cursor_col = self.cursor_col.min(self.current_line_char_count());
            }
            _ => self.clamp_cursor_normal(),
        }
    }

    fn record_undo(&mut self) {
        let snapshot = self.snapshot_document();
        if self.undo_stack.last() != Some(&snapshot) {
            self.undo_stack.push(snapshot);
        }
        self.redo_stack.clear();
    }

    fn undo(&mut self) -> bool {
        let Some(snapshot) = self.undo_stack.pop() else {
            return false;
        };
        self.redo_stack.push(self.snapshot_document());
        self.restore_snapshot(snapshot);
        true
    }

    fn redo(&mut self) -> bool {
        let Some(snapshot) = self.redo_stack.pop() else {
            return false;
        };
        self.undo_stack.push(self.snapshot_document());
        self.restore_snapshot(snapshot);
        true
    }

    fn repeat_last_change(&mut self) -> bool {
        let Some(last_change) = self.last_change.clone() else {
            return false;
        };
        self.replaying_change = true;
        
        self.vim_state.insert_start_pos = Some(self.flattened_cursor());
        self.vim_state.macro_insert_start_index = Some(self.vim_state.current_macro.len());

        let mut changed = false;
        for step in last_change {
            if step == "." {
                continue;
            }
            if self.vim_state.mode == VimMode::Insert || self.vim_state.mode == VimMode::Replace {
                match step.as_str() {
                    "escape" | "ctrl+[" => self.enter_normal(),
                    "ctrl+c" => self.cancel_insert(),
                    "ctrl+w" => self.delete_word_insert(),
                    "ctrl+u" => self.delete_line_insert(),
                    "return" => self.insert_newline(),
                    "backspace" => self.backspace(),
                    "delete" => self.delete_at_cursor(),
                    text => {
                        self.handle_insert_text(text);
                    }
                }
                changed = true;
            } else {
                changed |= self.handle_normal_input(&step);
            }
        }
        self.replaying_change = false;
        changed
    }

    fn take_register_target(&mut self) -> RegisterTarget {
        self.selected_register
            .take()
            .unwrap_or(RegisterTarget::Unnamed)
    }

    fn cursor_position(&self) -> CursorPosition {
        CursorPosition {
            line: self.cursor_line(),
            col: self.cursor_col(),
        }
    }

    fn push_jump(&mut self) {
        let position = self.cursor_position();
        if self.jump_list.last().copied() != Some(position) {
            self.jump_list.push(position);
            self.jump_index = Some(self.jump_list.len().saturating_sub(1));
        }
    }

    fn jump_history(&mut self, delta: isize) -> bool {
        if self.jump_list.is_empty() {
            return false;
        }

        let current =
            self.jump_index
                .unwrap_or_else(|| self.jump_list.len().saturating_sub(1)) as isize;
        let next = (current + delta).clamp(0, self.jump_list.len().saturating_sub(1) as isize);
        if next == current {
            return false;
        }

        let position = self.jump_list[next as usize];
        self.jump_index = Some(next as usize);
        self.cursor_line = position.line;
        self.cursor_col = position.col;
        self.clamp_cursor_normal();
        true
    }

    fn enter_visual(&mut self, kind: VisualKind) {
        self.visual_anchor_flat = Some(self.flattened_cursor());
        self.vim_state.mode = match kind {
            VisualKind::Character => VimMode::Visual,
            VisualKind::Line => VimMode::VisualLine,
            VisualKind::Block => VimMode::VisualBlock,
        };
        self.vim_state.pending_command = None;
        self.count = None;
    }

    fn visual_kind(&self) -> VisualKind {
        match self.vim_state.mode {
            VimMode::Visual => VisualKind::Character,
            VimMode::VisualLine => VisualKind::Line,
            VimMode::VisualBlock => VisualKind::Block,
            _ => VisualKind::Character,
        }
    }

    fn visual_selection_range(&self) -> Option<TextRange> {
        let anchor = self.visual_anchor_flat?;
        let cursor = self.flattened_cursor();
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
                let anchor_line = self.line_for_flat(anchor);
                let cursor_line = self.cursor_line();
                Some(self.linewise_range(anchor_line, cursor_line))
            }
            _ => None,
        }
    }

    fn apply_visual_operator(&mut self, operator: Operator) -> bool {
        let Some(range) = self.visual_selection_range() else {
            return false;
        };
        self.last_visual_selection = Some((range, self.visual_kind()));
        self.visual_anchor_flat = None;
        self.vim_state.mode = VimMode::Normal;
        self.apply_operator_range(operator, range)
    }

    fn restore_last_visual_selection(&mut self) -> bool {
        let Some((range, kind)) = self.last_visual_selection else {
            return false;
        };
        self.visual_anchor_flat = Some(range.start);
        match kind {
            VisualKind::Character => {
                self.vim_state.mode = VimMode::Visual;
                self.set_cursor_from_flat(range.end.saturating_sub(1));
            }
            VisualKind::Line => {
                self.vim_state.mode = VimMode::VisualLine;
                self.set_cursor_from_flat(range.end.saturating_sub(1));
            }
            VisualKind::Block => {
                self.vim_state.mode = VimMode::VisualBlock;
                self.set_cursor_from_flat(range.end.saturating_sub(1));
            }
        }
        true
    }

    fn refresh_search_matches(&mut self) {
        self.search.highlights_active = self.settings.hlsearch;
        self.search.matches.clear();
        if self.search.pattern.is_empty() {
            return;
        }

        let chars: Vec<char> = self.content.chars().collect();
        let ignore_case = self.should_ignore_case(&self.search.pattern);
        let pattern_source = if ignore_case {
            self.search.pattern.to_lowercase()
        } else {
            self.search.pattern.clone()
        };
        let pattern: Vec<char> = pattern_source.chars().collect();
        if chars.is_empty() || pattern.is_empty() || pattern.len() > chars.len() {
            return;
        }

        let haystack: Vec<char> = if ignore_case {
            self.content.to_lowercase().chars().collect()
        } else {
            chars.clone()
        };
        for start in 0..=chars.len() - pattern.len() {
            if haystack[start..start + pattern.len()] == pattern[..] {
                self.search
                    .matches
                    .push(TextRange::new(start, start + pattern.len()));
            }
        }
    }

    fn repeat_search(&mut self, opposite: bool) -> bool {
        if self.search.pattern.is_empty() {
            return false;
        }
        self.refresh_search_matches();
        if self.search.matches.is_empty() {
            return false;
        }

        let reverse = self.search.reverse ^ opposite;
        let cursor = self.flattened_cursor();
        let target = if reverse {
            self.search
                .matches
                .iter()
                .rev()
                .find(|range| range.start < cursor)
                .or_else(|| self.search.matches.last())
        } else {
            self.search
                .matches
                .iter()
                .find(|range| range.start > cursor)
                .or_else(|| self.search.matches.first())
        };
        let Some(target) = target.copied() else {
            return false;
        };

        self.push_jump();
        self.set_cursor_from_flat(target.start);
        true
    }

    fn search_word_under_cursor(&mut self, reverse: bool) -> bool {
        let Some(range) = self.word_text_object_range(1, false, false) else {
            return false;
        };
        let pattern = self.text_for_range(range);
        if pattern.is_empty() {
            return false;
        }
        self.search.pattern = pattern;
        self.search.reverse = reverse;
        self.refresh_search_matches();
        self.repeat_search(false)
    }

    fn should_ignore_case(&self, pattern: &str) -> bool {
        if !self.settings.ignorecase {
            return false;
        }
        if self.settings.smartcase && pattern.chars().any(|ch| ch.is_uppercase()) {
            return false;
        }
        true
    }

    fn insert_blank_line(&mut self, index: usize) {
        let mut lines = self.lines_vec();
        let index = index.min(lines.len());
        lines.insert(index, String::new());
        self.cursor_line = index;
        self.replace_lines(lines);
    }

    fn delete_current_line(&mut self) {
        self.apply_current_lines_operator(Operator::Delete, 1);
    }

    fn apply_current_lines_operator(&mut self, operator: Operator, count: usize) -> bool {
        let original_cursor = self.flattened_cursor();
        let original_col = self.cursor_col();
        self.enter_visual(VisualKind::Line);
        self.cursor_line = self.cursor_line().saturating_add(count.max(1).saturating_sub(1)).min(self.line_count().saturating_sub(1));
        let result = self.apply_visual_operator(operator);
        if operator == Operator::Yank {
            self.set_cursor_from_flat(original_cursor);
            self.cursor_col = original_col;
            self.clamp_cursor_normal();
        }
        result
    }


    fn delete_chars_on_current_line(&mut self, count: usize) -> bool {
        let mut lines = self.lines_vec();
        if let Some(line) = lines.get_mut(self.cursor_line()) {
            if line.is_empty() {
                return false;
            }

            let col = self.cursor_col().min(char_count(line).saturating_sub(1));
            let end = col.saturating_add(count.max(1)).min(char_count(line));
            let deleted = line.chars().skip(col).take(end - col).collect::<String>();
            if deleted.is_empty() {
                return false;
            }

            let target = self.take_register_target();
            self.registers.store_deleted(target, deleted, false);
            self.record_undo();
            remove_char_range(line, col, end);
            self.content = lines.join("\n");
            self.dirty = true;
            self.clamp_cursor_line();
            return true;
        }

        false
    }

    fn delete_chars_before_cursor(&mut self, count: usize) -> bool {
        let mut lines = self.lines_vec();
        let line_index = self.cursor_line();
        let Some(line) = lines.get_mut(line_index) else {
            return false;
        };
        if line.is_empty() || self.cursor_col() == 0 {
            return false;
        }

        let end = self.cursor_col().min(char_count(line));
        let start = end.saturating_sub(count.max(1));
        let deleted = line
            .chars()
            .skip(start)
            .take(end.saturating_sub(start))
            .collect::<String>();
        if deleted.is_empty() {
            return false;
        }

        let target = self.take_register_target();
        self.registers.store_deleted(target, deleted, false);
        self.record_undo();
        self.push_change_location();
        remove_char_range(line, start, end);
        self.content = lines.join("\n");
        self.dirty = true;
        self.cursor_col = start;
        self.clamp_cursor_line();
        true
    }

    fn replace_chars(&mut self, replacement: char, count: usize) -> bool {
        let mut lines = self.lines_vec();
        let line_index = self.cursor_line();
        let Some(line) = lines.get_mut(line_index) else {
            return false;
        };
        let col = self.cursor_col().min(char_count(line));
        if col >= char_count(line) {
            return false;
        }
        let end = (col + count.max(1)).min(char_count(line));
        let deleted = line
            .chars()
            .skip(col)
            .take(end.saturating_sub(col))
            .collect::<String>();
        let target = self.take_register_target();
        self.registers.store_deleted(target, deleted, false);
        self.replace_flat_range(TextRange::new(self.flattened_cursor(), self.flattened_cursor() + end.saturating_sub(col)), &replacement.to_string().repeat(end.saturating_sub(col)));
        self.clamp_cursor_normal();
        true
    }

    fn push_change_location(&mut self) {
        let position = self.cursor_position();
        if self.changelist.last().copied() == Some(position) {
            self.changelist_index = Some(self.changelist.len().saturating_sub(1));
            return;
        }
        if let Some(index) = self.changelist_index {
            if index + 1 < self.changelist.len() {
                self.changelist.truncate(index + 1);
            }
        }
        self.changelist.push(position);
        self.changelist_index = Some(self.changelist.len().saturating_sub(1));
    }

    fn navigate_changelist(&mut self, delta: isize) -> bool {
        if self.changelist.is_empty() {
            return false;
        }
        let current = self
            .changelist_index
            .unwrap_or_else(|| self.changelist.len().saturating_sub(1)) as isize;
        let next = (current + delta).clamp(0, self.changelist.len().saturating_sub(1) as isize);
        if next == current {
            return false;
        }
        let position = self.changelist[next as usize];
        self.changelist_index = Some(next as usize);
        self.cursor_line = position.line;
        self.cursor_col = position.col;
        self.clamp_cursor_normal();
        true
    }

    fn substitute_chars(&mut self, count: usize) -> bool {
        let start = self.flattened_cursor();
        let end = (start + count.max(1)).min(self.current_line_end_flat_exclusive());
        if start >= end {
            return false;
        }
        self.apply_operator_range(Operator::Change, TextRange::new(start, end))
    }

    fn join_lines(&mut self, count: usize, with_space: bool) -> bool {
        let mut lines = self.lines_vec();
        if lines.len() < 2 || self.cursor_line() >= lines.len().saturating_sub(1) {
            return false;
        }

        let start_line = self.cursor_line();
        let join_line_count = count.max(1).saturating_add(1);
        let max_join = join_line_count.min(lines.len().saturating_sub(start_line));
        if max_join <= 1 {
            return false;
        }

        self.record_undo();
        self.push_change_location();

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
        self.content = lines.join("\n");
        self.dirty = true;
        self.clamp_cursor_normal();
        true
    }

    fn first_non_blank_col(&self) -> usize {
        self.first_non_blank_col_for_line(self.cursor_line())
    }

    fn first_non_blank_col_for_line(&self, line_index: usize) -> usize {
        self.lines_vec()
            .get(line_index)
            .and_then(|line| line.chars().position(|ch| !ch.is_whitespace()))
            .unwrap_or(0)
    }

    fn flat_index_for_line(&self, target_line: usize) -> usize {
        let lines = self.lines_vec();
        let mut offset = 0;
        for line_index in 0..target_line.min(lines.len()) {
            offset += char_count(&lines[line_index]) + 1;
        }
        offset
    }

    fn flattened_cursor(&self) -> usize {
        let lines = self.lines_vec();
        let mut offset = 0;
        for line_index in 0..self.cursor_line().min(lines.len()) {
            offset += char_count(&lines[line_index]) + 1;
        }
        offset + self.cursor_col().min(self.current_line_char_count())
    }

    fn set_cursor_from_flat(&mut self, mut offset: usize) {
        let lines = self.lines_vec();
        for (line_index, line) in lines.iter().enumerate() {
            let len = char_count(line);
            if offset <= len {
                self.cursor_line = line_index;
                self.cursor_col = offset.min(len.saturating_sub(1));
                self.clamp_cursor_normal();
                return;
            }
            offset = offset.saturating_sub(len + 1);
        }
        self.cursor_line = lines.len().saturating_sub(1);
        self.cursor_col = self.current_line_max_col();
    }

    fn move_word_forward(&mut self, count: usize, big_word: bool) {
        for _ in 0..count {
            let text = self.content_with_virtual_empty_line();
            let chars: Vec<char> = text.chars().collect();
            let mut index = self.flattened_cursor().min(chars.len().saturating_sub(1));
            while index < chars.len() && is_word_char(chars[index], big_word) {
                index += 1;
            }
            while index < chars.len() && !is_word_char(chars[index], big_word) {
                index += 1;
            }
            self.set_cursor_from_flat(index.min(chars.len().saturating_sub(1)));
        }
    }

    fn move_word_backward(&mut self, count: usize, big_word: bool) {
        for _ in 0..count {
            let text = self.content_with_virtual_empty_line();
            let chars: Vec<char> = text.chars().collect();
            let mut index = self.flattened_cursor().saturating_sub(1).min(chars.len());
            while index > 0 && !is_word_char(chars[index], big_word) {
                index -= 1;
            }
            while index > 0 && is_word_char(chars[index - 1], big_word) {
                index -= 1;
            }
            self.set_cursor_from_flat(index);
        }
    }

    fn move_word_end(&mut self, count: usize, big_word: bool) {
        for _ in 0..count {
            let text = self.content_with_virtual_empty_line();
            let chars: Vec<char> = text.chars().collect();
            let mut index = self.flattened_cursor().saturating_add(1).min(chars.len());
            while index < chars.len() && !is_word_char(chars[index], big_word) {
                index += 1;
            }
            while index + 1 < chars.len() && is_word_char(chars[index + 1], big_word) {
                index += 1;
            }
            self.set_cursor_from_flat(index.min(chars.len().saturating_sub(1)));
        }
    }

    fn move_word_backward_end(&mut self, count: usize, big_word: bool) {
        for _ in 0..count {
            let text = self.content_with_virtual_empty_line();
            let chars: Vec<char> = text.chars().collect();
            if chars.is_empty() {
                return;
            }
            let mut index = self.flattened_cursor().min(chars.len().saturating_sub(1));
            if index > 0 {
                index -= 1;
            }
            while index > 0 && !is_word_char(chars[index], big_word) {
                index -= 1;
            }
            while index + 1 < chars.len() && is_word_char(chars[index + 1], big_word) {
                index += 1;
            }
            self.set_cursor_from_flat(index);
        }
    }

    fn move_first_nonblank_on_relative_line(&mut self, delta: isize) {
        let current = self.cursor_line() as isize;
        let max = self.line_count().saturating_sub(1) as isize;
        let target = (current + delta).clamp(0, max) as usize;
        self.cursor_line = target;
        self.cursor_col = self.first_non_blank_col_for_line(target);
        self.clamp_cursor_normal();
    }

    fn content_with_virtual_empty_line(&self) -> String {
        if self.content.is_empty() {
            " ".to_string()
        } else {
            self.content.clone()
        }
    }

    fn apply_operator_motion(&mut self, operator: Operator, count: usize, motion: char) -> bool {
        let Some(range) = self.operator_motion_range(count.max(1), motion) else {
            return false;
        };

        self.apply_operator_range(operator, range)
    }

    fn apply_text_object_operator(
        &mut self,
        operator: Operator,
        count: usize,
        around: bool,
        object: TextObject,
    ) -> bool {
        let range = match object {
            TextObject::Word { big_word } => {
                self.word_text_object_range(count.max(1), around, big_word)
            }
            TextObject::Delimited(delimiter) => self.delimited_text_object_range(delimiter, around),
            TextObject::Paragraph => self.paragraph_text_object_range(count.max(1), around),
            TextObject::Line => self.line_text_object_range(count.max(1)),
        };
        let Some(range) = range else {
            return false;
        };

        self.apply_operator_range(operator, range)
    }

    fn apply_visual_text_object(
        &mut self,
        count: usize,
        around: bool,
        object: TextObject,
    ) -> bool {
        let range = match object {
            TextObject::Word { big_word } => {
                self.word_text_object_range(count.max(1), around, big_word)
            }
            TextObject::Delimited(delimiter) => self.delimited_text_object_range(delimiter, around),
            TextObject::Paragraph => self.paragraph_text_object_range(count.max(1), around),
            TextObject::Line => self.line_text_object_range(count.max(1)),
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
        let cursor_flat = self.flattened_cursor();
        if current_anchor == cursor_flat {
            self.visual_anchor_flat = Some(range.start);
            self.set_cursor_from_flat(range.end.saturating_sub(1));
        } else if cursor_flat >= current_anchor {
            self.visual_anchor_flat = Some(current_anchor.min(range.start));
            self.set_cursor_from_flat(range.end.saturating_sub(1).max(cursor_flat));
        } else {
            self.visual_anchor_flat = Some(current_anchor.max(range.end.saturating_sub(1)));
            self.set_cursor_from_flat(range.start.min(cursor_flat));
        }

        true
    }

    fn apply_operator_range(&mut self, operator: Operator, range: TextRange) -> bool {
        let range = range.normalized().clamped(self.content_char_len());
        if !range.linewise && range.start >= range.end && !range.blockwise {
            return false;
        }

        if self.defer_enabled && (operator == Operator::Delete || operator == Operator::Change) && range.linewise {
            self.yank_highlight = Some(range);
            self.deferred_action = Some(DeferredAction { operator, range });
            return false;
        }

        self.apply_operator_range_direct(operator, range)
    }

    fn apply_blockwise_operator(&mut self, operator: Operator, range: TextRange) -> bool {
        let start_line = self.line_for_flat(range.start);
        let end_line = self.line_for_flat(range.end.saturating_sub(1));
        let top_line = start_line.min(end_line);
        let bottom_line = start_line.max(end_line);

        let start_col = {
            let line = self.line_for_flat(range.start);
            let line_start = self.line_start_flat(line);
            range.start.saturating_sub(line_start)
        };
        let end_col = {
            let line = self.line_for_flat(range.end.saturating_sub(1));
            let line_start = self.line_start_flat(line);
            range.end.saturating_sub(1).saturating_sub(line_start)
        };
        let left_col = start_col.min(end_col);
        let right_col = start_col.max(end_col);

        let mut lines = self.lines_vec();

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
                let target = self.take_register_target();
                let register = RegisterValue { text: yanked.join("\n"), linewise: false, blockwise: true };
                self.registers.store_target(target, register.clone());
                if target == RegisterTarget::Unnamed {
                    self.registers.named.insert('0', register.clone());
                }
                self.registers.yank = register;
                self.set_cursor_from_flat(range.start);
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
                let target = self.take_register_target();
                let register = RegisterValue { text: deleted.join("\n"), linewise: false, blockwise: true };
                self.registers.store_target(target, register.clone());
                if target == RegisterTarget::Unnamed {
                    self.registers.named.insert('1', register.clone());
                }
                
                self.replace_lines(lines);
                self.set_cursor_from_flat(range.start);
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
                let target = self.take_register_target();
                let register = RegisterValue { text: deleted.join("\n"), linewise: false, blockwise: true };
                self.registers.store_target(target, register.clone());
                if target == RegisterTarget::Unnamed {
                    self.registers.named.insert('1', register.clone());
                }
                
                self.replace_lines(lines);
                self.set_cursor_from_flat(range.start);
                self.cursor_col = left_col;
                
                self.deferred_action = Some(DeferredAction { operator: Operator::BlockInsert, range });
                self.enter_insert();
                true
            }
            Operator::BlockInsert | Operator::BlockAppend => {
                let inserted = if let Some(start) = self.vim_state.insert_start_pos {
                    let end = self.flattened_cursor();
                    if end > start {
                        self.text_for_range(TextRange::new(start, end))
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };

                if !inserted.is_empty() {
                    let insert_col = if operator == Operator::BlockInsert { left_col } else { right_col + 1 };
                    for i in top_line..=bottom_line {
                        if i == self.cursor_line() { continue; } // Already inserted on this line by user
                        if i < lines.len() {
                            let line_len = char_count(&lines[i]);
                            let col = insert_col.min(line_len);
                            insert_str_at_char(&mut lines[i], col, &inserted);
                        }
                    }
                    self.replace_lines(lines);
                }
                true
            }
            _ => false,
        }
    }

    fn apply_operator_range_direct(&mut self, operator: Operator, range: TextRange) -> bool {
        if range.blockwise {
            return self.apply_blockwise_operator(operator, range);
        }
        match operator {
            Operator::SurroundAdd => {
                self.vim_state.pending_command = Some(PendingCommand::SurroundWaitAdd {
                    range,
                    buffer: String::new(),
                });
                false
            }
            Operator::Delete => {
                let text = self.text_for_range(range);
                let target = self.take_register_target();
                self.registers.store_deleted(target, text, range.linewise);

                let mut actual_delete_range = range;
                if range.linewise && range.end == self.content_char_len() && range.start > 0 {
                    if self.content.chars().nth(range.start - 1) == Some('\n') {
                        actual_delete_range.start -= 1;
                    }
                }

                self.delete_flat_range(actual_delete_range);
                self.set_insert_cursor_from_flat(actual_delete_range.start);
                self.enter_normal();
                true
            }
            Operator::Change => {
                let text = self.text_for_range(range);
                let target = self.take_register_target();
                self.registers.store_deleted(target, text, range.linewise);

                let mut actual_delete_range = range;
                if range.linewise && range.end == self.content_char_len() && range.start > 0 {
                    if self.content.chars().nth(range.start - 1) == Some('\n') {
                        actual_delete_range.start -= 1;
                    }
                }

                self.delete_flat_range(actual_delete_range);

                if range.linewise {
                    insert_str_at_char(&mut self.content, actual_delete_range.start, "\n");
                    self.dirty = true;
                    self.set_insert_cursor_from_flat(actual_delete_range.start);
                } else {
                    self.set_insert_cursor_from_flat(actual_delete_range.start);
                }

                self.enter_insert();
                true
            }
            Operator::Yank => {
                let text = self.text_for_range(range);
                let target = self.take_register_target();
                self.registers.store_yank(target, text, range.linewise);
                if range.linewise {
                    self.set_cursor_from_flat(range.start);
                    self.cursor_col = self.first_non_blank_col();
                } else {
                    self.set_cursor_from_flat(range.start);
                }
                self.clamp_cursor_normal();
                self.yank_highlight = Some(range);
                false
            }
            Operator::Indent => self.indent_range(range, 1),
            Operator::Outdent => self.indent_range(range, -1),
            Operator::Format => self.format_range(range),
            Operator::BlockInsert | Operator::BlockAppend => false,
        }
    }

    fn apply_surround_add(&mut self, range: TextRange, spec: crate::vim::surround::SurroundSpec) {
        let (left, right) = spec.strings();
        let mut actual_range = range;
        if !actual_range.linewise && actual_range.end == self.content_char_len() && actual_range.start > 0 {
            if self.content.chars().nth(actual_range.start - 1) == Some('\n') {
                actual_range.start -= 1;
            }
        }
        
        self.record_undo();
        self.push_change_location();
        
        // Insert right first so it doesn't mess up the start index
        insert_str_at_char(&mut self.content, actual_range.end, &right);
        insert_str_at_char(&mut self.content, actual_range.start, &left);
        
        self.dirty = true;
        self.set_cursor_from_flat(actual_range.start);
        self.clamp_cursor_normal();
    }

    fn find_surround_target(&self, spec: &crate::vim::surround::SurroundSpec) -> Option<(TextRange, TextRange)> {
        match spec {
            crate::vim::surround::SurroundSpec::Pair(open, close) => {
                let chars: Vec<char> = self.content.chars().collect();
                if chars.is_empty() {
                    return None;
                }
                let cursor = self.flattened_cursor().min(chars.len().saturating_sub(1));
                
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
                let chars: Vec<char> = self.content.chars().collect();
                if chars.is_empty() {
                    return None;
                }
                let cursor = self.flattened_cursor().min(chars.len().saturating_sub(1));
                
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

    fn apply_surround_delete(&mut self, spec: crate::vim::surround::SurroundSpec) {
        if let Some((left, right)) = self.find_surround_target(&spec) {
            self.record_undo();
            self.push_change_location();
            
            self.delete_flat_range(right.clone());
            self.delete_flat_range(left.clone());
            
            self.dirty = true;
            self.set_cursor_from_flat(left.start);
            self.clamp_cursor_normal();
        }
    }

    fn apply_surround_change(&mut self, old_spec: crate::vim::surround::SurroundSpec, new_spec: crate::vim::surround::SurroundSpec) {
        if let Some((left, right)) = self.find_surround_target(&old_spec) {
            let (new_left, new_right) = new_spec.strings();
            self.record_undo();
            self.push_change_location();
            
            // Right side
            self.delete_flat_range(right.clone());
            insert_str_at_char(&mut self.content, right.start, &new_right);
            
            // Left side
            self.delete_flat_range(left.clone());
            insert_str_at_char(&mut self.content, left.start, &new_left);
            
            self.dirty = true;
            self.set_cursor_from_flat(left.start);
            self.clamp_cursor_normal();
        }
    }

    fn operator_motion_range(&self, count: usize, motion: char) -> Option<TextRange> {
        let start = self.flattened_cursor();
        match motion {
            'h' => Some(TextRange::new(
                start.saturating_sub(count),
                start.min(self.content_char_len()),
            )),
            'l' => Some(TextRange::new(
                start,
                (start + count).min(self.current_line_end_flat_exclusive()),
            )),
            'w' | 'W' => {
                let mut target = self.clone();
                target.move_word_forward(count, motion == 'W');
                let mut end = target.flattened_cursor().min(self.content_char_len());
                if end == self.content_char_len().saturating_sub(1) && start < end {
                    end = self.content_char_len();
                }
                Some(TextRange::new(start, end))
            }
            'b' | 'B' => {
                let mut target = self.clone();
                target.move_word_backward(count, motion == 'B');
                Some(TextRange::new(target.flattened_cursor(), start))
            }
            'e' | 'E' => {
                let mut target = self.clone();
                target.move_word_end(count, motion == 'E');
                Some(TextRange::new(
                    start,
                    target.flattened_cursor().saturating_add(1),
                ))
            }
            '0' => Some(TextRange::new(self.current_line_start_flat(), start)),
            '^' => Some(TextRange::new(
                self.current_line_start_flat() + self.first_non_blank_col(),
                start,
            )),
            '$' => Some(TextRange::new(
                start,
                self.current_line_end_flat_exclusive(),
            )),
            'j' | 'k' => {
                let mut target = self.clone();
                let delta = if motion == 'j' {
                    count as isize
                } else {
                    -(count as isize)
                };
                target.move_cursor_line(delta);
                Some(self.linewise_range(self.cursor_line(), target.cursor_line()))
            }
            'G' => {
                let target_line = count.saturating_sub(1);
                Some(self.linewise_range(self.cursor_line(), target_line))
            }
            ';' | ',' => self.find_char_range(motion == ';', count),
            '%' | '{' | '}' | '(' | ')' => {
                let target = self.extended_motion_flat(motion, count)?;
                let start = self.flattened_cursor();
                if start <= target {
                    Some(TextRange::new(start, target))
                } else {
                    Some(TextRange::new(target, start))
                }
            }
            'H' | 'M' | 'L' => {
                let target_line = self.extended_motion_line(motion, count)?;
                Some(self.linewise_range(self.cursor_line(), target_line))
            }
            _ => None,
        }
    }

    fn find_char_flat(&self, ch: char, is_t: bool, is_forward: bool, count: usize) -> Option<usize> {
        let lines = self.lines_vec();
        let line = lines.get(self.cursor_line()).map(|s| s.as_str()).unwrap_or("");
        let col = self.cursor_col();
        let chars: Vec<char> = line.chars().collect();
        
        let mut found = 0;
        if is_forward {
            for i in (col + 1)..chars.len() {
                if chars[i] == ch {
                    found += 1;
                    if found == count {
                        return Some(self.current_line_start_flat() + if is_t { i - 1 } else { i });
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
                        return Some(self.current_line_start_flat() + if is_t { i + 1 } else { i });
                    }
                }
            }
        }
        None
    }

    fn execute_find_char(&mut self, ch: char, is_t: bool, is_forward: bool, count: usize) -> bool {
        self.vim_state.last_find = Some((ch, is_t, is_forward));
        if let Some(target) = self.find_char_flat(ch, is_t, is_forward, count) {
            self.set_cursor_from_flat(target);
            return true;
        }
        false
    }

    fn repeat_find_char(&mut self, is_semi: bool, count: usize) -> bool {
        let Some((ch, is_t, mut is_forward)) = self.vim_state.last_find else {
            return false;
        };
        if !is_semi {
            is_forward = !is_forward;
        }
        if let Some(target) = self.find_char_flat(ch, is_t, is_forward, count) {
            self.set_cursor_from_flat(target);
            return true;
        }
        false
    }

    fn execute_operator_find_char(
        &mut self,
        operator: Operator,
        _operator_count: usize,
        ch: char,
        is_t: bool,
        is_forward: bool,
        find_count: usize,
    ) -> bool {
        self.vim_state.last_find = Some((ch, is_t, is_forward));
        let start = self.flattened_cursor();
        let Some(target) = self.find_char_flat(ch, is_t, is_forward, find_count) else {
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
        
        self.apply_operator_range(operator, range)
    }

    fn find_char_range(&self, is_semi: bool, count: usize) -> Option<TextRange> {
        let Some((ch, is_t, mut is_forward)) = self.vim_state.last_find else {
            return None;
        };
        if !is_semi {
            is_forward = !is_forward;
        }
        let start = self.flattened_cursor();
        let target = self.find_char_flat(ch, is_t, is_forward, count)?;
        
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

    fn scroll_viewport_half_page(&mut self, down: bool) -> bool {
        let lines = self.viewport_state.visible_lines.max(2) / 2;
        self.push_jump();
        if down {
            self.cursor_line = self.cursor_line.saturating_add(lines).min(self.line_count().saturating_sub(1));
            self.viewport_state.top_line = self.viewport_state.top_line.saturating_add(lines);
        } else {
            self.cursor_line = self.cursor_line.saturating_sub(lines);
            self.viewport_state.top_line = self.viewport_state.top_line.saturating_sub(lines);
        }
        self.clamp_cursor_normal();
        true
    }

    fn scroll_viewport_full_page(&mut self, down: bool) -> bool {
        let lines = self.viewport_state.visible_lines.saturating_sub(2).max(1);
        self.push_jump();
        if down {
            self.cursor_line = self.cursor_line.saturating_add(lines).min(self.line_count().saturating_sub(1));
            self.viewport_state.top_line = self.viewport_state.top_line.saturating_add(lines);
        } else {
            self.cursor_line = self.cursor_line.saturating_sub(lines);
            self.viewport_state.top_line = self.viewport_state.top_line.saturating_sub(lines);
        }
        self.clamp_cursor_normal();
        true
    }

    fn scroll_viewport_line(&mut self, down: bool) -> bool {
        if down {
            self.viewport_state.top_line = self.viewport_state.top_line.saturating_add(1);
        } else {
            self.viewport_state.top_line = self.viewport_state.top_line.saturating_sub(1);
        }
        true
    }

    fn cursor_to_center(&mut self) -> bool {
        let half = self.viewport_state.visible_lines / 2;
        self.viewport_state.top_line = self.cursor_line.saturating_sub(half);
        true
    }

    fn cursor_to_top(&mut self) -> bool {
        self.viewport_state.top_line = self.cursor_line;
        true
    }

    fn cursor_to_bottom(&mut self) -> bool {
        self.viewport_state.top_line = self.cursor_line.saturating_sub(self.viewport_state.visible_lines.saturating_sub(1));
        true
    }

    fn find_matching_bracket(&self, start: usize) -> Option<usize> {
        let chars: Vec<char> = self.content.chars().collect();
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

    fn find_paragraph_forward(&self, count: usize) -> Option<usize> {
        let lines = self.lines_vec();
        let mut curr = self.cursor_line();
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
        Some(self.line_start_flat(curr.min(lines.len().saturating_sub(1))))
    }

    fn find_paragraph_backward(&self, count: usize) -> Option<usize> {
        let lines = self.lines_vec();
        let mut curr = self.cursor_line();
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
        Some(self.line_start_flat(curr))
    }

    fn find_sentence_forward(&self, count: usize) -> Option<usize> {
        let chars: Vec<char> = self.content.chars().collect();
        let mut i = self.flattened_cursor();
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

    fn find_sentence_backward(&self, count: usize) -> Option<usize> {
        let chars: Vec<char> = self.content.chars().collect();
        if chars.is_empty() {
            return None;
        }
        let mut i = self.flattened_cursor();
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

    fn extended_motion_flat(&self, motion: char, count: usize) -> Option<usize> {
        match motion {
            '%' => self.find_matching_bracket(self.flattened_cursor()),
            '{' => self.find_paragraph_backward(count),
            '}' => self.find_paragraph_forward(count),
            '(' => self.find_sentence_backward(count),
            ')' => self.find_sentence_forward(count),
            _ => None,
        }
    }

    fn extended_motion_line(&self, motion: char, count: usize) -> Option<usize> {
        match motion {
            'H' => Some(self.viewport_state.top_line.saturating_add(count.saturating_sub(1))),
            'M' => Some(self.viewport_state.top_line.saturating_add(self.viewport_state.visible_lines / 2)),
            'L' => Some((self.viewport_state.top_line + self.viewport_state.visible_lines).saturating_sub(count)),
            _ => None,
        }
    }

    fn word_text_object_range(
        &self,
        count: usize,
        around: bool,
        big_word: bool,
    ) -> Option<TextRange> {
        let chars: Vec<char> = self.content.chars().collect();
        if chars.is_empty() {
            return None;
        }

        let mut start = self.flattened_cursor().min(chars.len().saturating_sub(1));
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

    fn delimited_text_object_range(&self, delimiter: char, around: bool) -> Option<TextRange> {
        let (open, close) = delimiter_pair(delimiter)?;
        let chars: Vec<char> = self.content.chars().collect();
        if chars.is_empty() {
            return None;
        }

        let cursor = self.flattened_cursor().min(chars.len().saturating_sub(1));
        let start = (0..=cursor).rev().find(|&index| chars[index] == open)?;
        let end = (cursor..chars.len()).find(|&index| chars[index] == close)?;
        if start >= end {
            return None;
        }

        if around {
            Some(TextRange::new(start, end + 1))
        } else {
            Some(TextRange::new(start + 1, end))
        }
    }

    fn paragraph_text_object_range(&self, count: usize, around: bool) -> Option<TextRange> {
        let lines = self.lines_vec();
        if lines.is_empty() {
            return None;
        }

        let mut start_line = self.cursor_line().min(lines.len().saturating_sub(1));
        while start_line > 0 && !lines[start_line - 1].trim().is_empty() {
            start_line -= 1;
        }

        let mut end_line = self.cursor_line().min(lines.len().saturating_sub(1));
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

        let mut start = self.line_start_flat(start_line);
        let mut end = self.line_end_flat_including_newline(end_line);
        if around {
            while end_line + 1 < lines.len() && lines[end_line + 1].trim().is_empty() {
                end_line += 1;
                end = self.line_end_flat_including_newline(end_line);
                break;
            }
            if end == self.content_char_len() {
                while start_line > 0 && lines[start_line - 1].trim().is_empty() {
                    start_line -= 1;
                    start = self.line_start_flat(start_line);
                    break;
                }
            }
        }

        Some(TextRange::linewise(start, end))
    }

    fn line_text_object_range(&self, count: usize) -> Option<TextRange> {
        if self.lines_vec().is_empty() {
            return None;
        }
        Some(
            self.linewise_range(
                self.cursor_line(),
                self.cursor_line()
                    .saturating_add(count.max(1).saturating_sub(1)),
            ),
        )
    }


    fn indent_range(&mut self, range: TextRange, delta: isize) -> bool {
        let start_line = self.line_for_flat(range.start);
        let end_offset = range.end.saturating_sub(1);
        let end_line = self.line_for_flat(end_offset);
        self.indent_lines(start_line, end_line, delta)
    }

    fn indent_lines(&mut self, start_line: usize, end_line: usize, delta: isize) -> bool {
        let mut lines = self.lines_vec();
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

        self.replace_lines(lines);
        self.cursor_line = start_line;
        self.cursor_col = self.first_non_blank_col();
        self.clamp_cursor_normal();
        true
    }

    fn format_range(&mut self, range: TextRange) -> bool {
        let start_line = self.line_for_flat(range.start);
        let end_line = self.line_for_flat(range.end.saturating_sub(1));
        let mut lines = self.lines_vec();
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

        self.replace_lines(lines);
        true
    }

    fn toggle_case_chars(&mut self, count: usize) -> bool {
        let start = self.flattened_cursor();
        let end = (start + count.max(1)).min(self.current_line_end_flat_exclusive());
        if start >= end {
            return false;
        }

        let range = TextRange::new(start, end);
        let replacement = self
            .text_for_range(range)
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
        self.replace_flat_range(range, &replacement);
        self.set_cursor_from_flat(end.saturating_sub(1));
        true
    }

    fn toggle_case_range(&mut self, range: TextRange) -> bool {
        let range = range.normalized().clamped(self.content_char_len());
        if range.start >= range.end {
            return false;
        }

        let replacement = self
            .text_for_range(range)
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
        self.replace_flat_range(range, &replacement);
        self.set_cursor_from_flat(range.start);
        true
    }

    fn apply_case_range(&mut self, range: TextRange, upper: bool) -> bool {
        self.apply_case_range_kind(range, if upper { CaseKind::Upper } else { CaseKind::Lower })
    }

    fn apply_case_range_kind(&mut self, range: TextRange, kind: CaseKind) -> bool {
        let range = range.normalized().clamped(self.content_char_len());
        if range.start >= range.end {
            return false;
        }

        let replacement = self
            .text_for_range(range)
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
        self.replace_flat_range(range, &replacement);
        self.set_cursor_from_flat(range.start);
        true
    }

    fn replace_flat_range(&mut self, range: TextRange, replacement: &str) {
        self.record_undo();
        self.push_change_location();
        let start_byte = byte_index_for_char(&self.content, range.start);
        let end_byte = byte_index_for_char(&self.content, range.end);
        self.content
            .replace_range(start_byte..end_byte, replacement);
        self.dirty = true;
    }

    fn delete_flat_range(&mut self, range: TextRange) {
        self.record_undo();
        self.push_change_location();
        remove_char_range(&mut self.content, range.start, range.end);
        self.dirty = true;
    }

    fn text_for_range(&self, range: TextRange) -> String {
        let range = range.normalized().clamped(self.content_char_len());
        self.content
            .chars()
            .skip(range.start)
            .take(range.end.saturating_sub(range.start))
            .collect()
    }


    fn paste_unnamed(&mut self, count: usize, placement: PastePlacement) -> bool {
        let target = self.take_register_target();
        let register = self.registers.register(target).clone();
        if register.text.is_empty() {
            return false;
        }

        if register.blockwise {
            self.paste_blockwise(count.max(1), placement, register)
        } else if register.linewise {
            self.paste_linewise(count.max(1), placement, register)
        } else {
            self.paste_charwise(count.max(1), placement, register)
        }
    }

    fn paste_blockwise(
        &mut self,
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
        let start_line = self.cursor_line();
        let start_col = match placement {
            PastePlacement::After => self.cursor_col().saturating_add(1),
            PastePlacement::Before => self.cursor_col(),
        };

        let mut lines = self.lines_vec();
        self.record_undo();
        self.push_change_location();
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

        self.content = lines.join("\n");
        self.dirty = true;
        self.cursor_line = start_line;
        self.cursor_col = start_col.min(self.current_line_max_col());
        self.clamp_cursor_normal();
        true
    }

    fn paste_linewise(
        &mut self,
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

        let mut lines = self.lines_vec();
        let insert_at = match placement {
            PastePlacement::After => self.cursor_line().saturating_add(1).min(lines.len()),
            PastePlacement::Before => self.cursor_line().min(lines.len()),
        };
        let mut offset = 0;
        for _ in 0..count {
            for line in &paste_lines {
                lines.insert(insert_at + offset, line.clone());
                offset += 1;
            }
        }

        self.replace_lines(lines);
        self.cursor_line = insert_at.min(self.line_count().saturating_sub(1));
        self.cursor_col = self.first_non_blank_col();
        self.clamp_cursor_normal();
        true
    }

    fn paste_charwise(
        &mut self,
        count: usize,
        placement: PastePlacement,
        register: RegisterValue,
    ) -> bool {
        let text = register.text.repeat(count);
        let insert_at = match placement {
            PastePlacement::After => {
                (self.flattened_cursor() + 1).min(self.current_line_end_flat_exclusive())
            }
            PastePlacement::Before => self.flattened_cursor(),
        };
        let inserted_chars = text.chars().count();
        let byte_index = byte_index_for_char(&self.content, insert_at);
        self.record_undo();
        self.content.insert_str(byte_index, &text);
        self.dirty = true;
        self.set_insert_cursor_from_flat(insert_at + inserted_chars.saturating_sub(1));
        self.enter_normal();
        true
    }

    fn content_char_len(&self) -> usize {
        self.content.chars().count()
    }

    fn current_line_start_flat(&self) -> usize {
        self.line_start_flat(self.cursor_line())
    }

    fn current_line_end_flat_exclusive(&self) -> usize {
        self.line_start_flat(self.cursor_line()) + self.current_line_char_count()
    }

    fn line_start_flat(&self, line: usize) -> usize {
        self.lines_vec()
            .iter()
            .take(line)
            .map(|line| char_count(line) + 1)
            .sum()
    }

    fn line_for_flat(&self, mut offset: usize) -> usize {
        let lines = self.lines_vec();
        for (line_index, line) in lines.iter().enumerate() {
            let len = char_count(line);
            if offset <= len {
                return line_index;
            }
            offset = offset.saturating_sub(len + 1);
        }
        lines.len().saturating_sub(1)
    }

    fn line_end_flat_including_newline(&self, line: usize) -> usize {
        let lines = self.lines_vec();
        let line = line.min(lines.len().saturating_sub(1));
        let start = self.line_start_flat(line);
        let line_len = char_count(&lines[line]);
        if line + 1 < lines.len() {
            start + line_len + 1
        } else {
            start + line_len
        }
    }

    fn linewise_range(&self, first_line: usize, second_line: usize) -> TextRange {
        let start_line = first_line.min(second_line);
        let end_line = first_line.max(second_line);
        TextRange::linewise(
            self.line_start_flat(start_line),
            self.line_end_flat_including_newline(end_line),
        )
    }

    fn set_insert_cursor_from_flat(&mut self, mut offset: usize) {
        let lines = self.lines_vec();
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
        self.cursor_col = self.current_line_char_count();
    }

    pub fn normal_mode_line_with_cursor(&self, line: &str, line_index: usize) -> String {
        if line_index != self.cursor_line() {
            return line.to_string();
        }

        if line.is_empty() {
            return "|".to_string();
        }

        let col = self.cursor_col();
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

    fn line_with_cursor_marker(&self, line: &str, line_index: usize) -> String {
        if self.vim_state.mode == VimMode::Normal {
            return self.normal_mode_line_with_cursor(line, line_index);
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

    fn command_line_insert(&mut self, text: &str) {
        let cursor = self.vim_state.command_line.cursor;
        insert_str_at_char(&mut self.vim_state.command_line.input, cursor, text);
        self.vim_state.command_line.cursor += text.chars().count();
    }

    fn command_line_backspace(&mut self) {
        let cursor = self.vim_state.command_line.cursor;
        if cursor == 0 {
            return;
        }
        remove_char_at(&mut self.vim_state.command_line.input, cursor - 1);
        self.vim_state.command_line.cursor = cursor - 1;
    }

    fn command_line_delete(&mut self) {
        let cursor = self.vim_state.command_line.cursor;
        remove_char_at(&mut self.vim_state.command_line.input, cursor);
    }

    fn command_line_delete_word(&mut self) {
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

    fn push_command_history(&mut self, value: String, is_search: bool) {
        if value.is_empty() {
            return;
        }
        let history = if is_search {
            &mut self.search_history
        } else {
            &mut self.command_history
        };
        if history.last() != Some(&value) {
            history.push(value);
        }
    }

    fn command_line_history_move(&mut self, delta: isize) {
        let is_search = self.vim_state.command_line.is_search;
        let history = if is_search {
            &self.search_history
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

    fn update_incremental_search(&mut self, dir: SearchDirection) {
        if !self.settings.incsearch {
            return;
        }
        self.search.pattern = self.vim_state.command_line.input.clone();
        self.search.reverse = dir == SearchDirection::Backward;
        self.refresh_search_matches();
    }

    fn handle_command_mode_input(&mut self, input: &str) -> bool {
        match input {
            "escape" | "ctrl+[" => {
                self.enter_normal();
                false
            }
            "backspace" => {
                if self.vim_state.command_line.input.is_empty() {
                    self.enter_normal();
                } else {
                    self.command_line_backspace();
                }
                false
            }
            "delete" => {
                self.command_line_delete();
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
                self.command_line_delete_word();
                false
            }
            "up" => {
                self.command_line_history_move(-1);
                false
            }
            "down" => {
                self.command_line_history_move(1);
                false
            }
            "return" => {
                let cmd = self.vim_state.command_line.input.clone();
                self.push_command_history(cmd.clone(), false);
                self.enter_normal();
                self.execute_ex_command(&cmd)
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
                    self.command_line_insert(other);
                    if self.vim_state.command_line.input == "noh" || self.vim_state.command_line.input == "nohlsearch" {
                        let cmd = self.vim_state.command_line.input.clone();
                        self.push_command_history(cmd.clone(), false);
                        self.enter_normal();
                        return self.execute_ex_command(&cmd);
                    }
                }
                false
            }
        }
    }

    fn handle_search_mode_input(&mut self, dir: SearchDirection, input: &str) -> bool {
        match input {
            "escape" | "ctrl+[" => {
                self.enter_normal();
                false
            }
            "backspace" => {
                if self.vim_state.command_line.input.is_empty() {
                    self.enter_normal();
                } else {
                    self.command_line_backspace();
                }
                self.update_incremental_search(dir);
                false
            }
            "delete" => {
                self.command_line_delete();
                self.update_incremental_search(dir);
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
                self.update_incremental_search(dir);
                false
            }
            "ctrl+w" => {
                self.command_line_delete_word();
                self.update_incremental_search(dir);
                false
            }
            "up" => {
                self.command_line_history_move(-1);
                false
            }
            "down" => {
                self.command_line_history_move(1);
                false
            }
            "return" => {
                let query = self.vim_state.command_line.input.clone();
                self.push_command_history(query.clone(), true);
                self.enter_normal();
                if !query.is_empty() {
                    self.search.pattern = query;
                    self.search.reverse = dir == SearchDirection::Backward;
                    self.refresh_search_matches();
                    self.repeat_search(false)
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
                    self.command_line_insert(other);
                    self.update_incremental_search(dir);
                }
                false
            }
        }
    }

    fn find_substitution_matches(
        &self,
        start_line: usize,
        end_line: usize,
        pattern: &str,
        flags: &str,
    ) -> Vec<TextRange> {
        let mut matches = Vec::new();
        if pattern.is_empty() {
            return matches;
        }

        let is_case_insensitive = flags.contains('i') || (flags.is_empty() && self.should_ignore_case(pattern));
        let is_global_on_line = flags.contains('g');

        let lines = self.lines_vec();
        let pat_len = pattern.chars().count();
        let pat_chars: Vec<char> = if is_case_insensitive {
            pattern.to_lowercase().chars().collect()
        } else {
            pattern.chars().collect()
        };

        let mut current_flat_offset = 0;
        for line_idx in 0..lines.len() {
            let line = &lines[line_idx];
            let line_len = line.chars().count();

            if line_idx >= start_line && line_idx <= end_line {
                let line_chars: Vec<char> = if is_case_insensitive {
                    line.to_lowercase().chars().collect()
                } else {
                    line.chars().collect()
                };

                let mut col = 0;
                while col + pat_len <= line_len {
                    if line_chars[col..col + pat_len] == pat_chars[..] {
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
        &mut self,
        start_line: usize,
        end_line: usize,
        pattern: &str,
        replacement: &str,
        flags: &str,
    ) -> bool {
        let matches = self.find_substitution_matches(start_line, end_line, pattern, flags);
        if matches.is_empty() {
            return false;
        }

        self.record_undo();
        self.push_change_location();
        let mut content_chars: Vec<char> = self.content.chars().collect();
        for m in matches.iter().rev() {
            let prefix: Vec<char> = content_chars[..m.start].to_vec();
            let suffix: Vec<char> = content_chars[m.end..].to_vec();
            let replacement_chars: Vec<char> = replacement.chars().collect();
            content_chars = [prefix, replacement_chars, suffix].concat();
        }

        self.content = content_chars.into_iter().collect();
        self.dirty = true;

        self.search.pattern = pattern.to_string();
        self.refresh_search_matches();
        self.last_substitute = Some(LastSubstitute {
            range: format!("{},{}", start_line + 1, end_line + 1),
            pattern: pattern.to_string(),
            replacement: replacement.to_string(),
            flags: flags.to_string(),
        });

        true
    }

    fn execute_substitution(
        &mut self,
        start_line: usize,
        end_line: usize,
        pattern: &str,
        replacement: &str,
        flags: &str,
    ) -> bool {
        let matches = self.find_substitution_matches(start_line, end_line, pattern, flags);
        if matches.is_empty() {
            return false;
        }

        if flags.contains('c') {
            let first_match = matches[0];
            self.yank_highlight = Some(first_match);
            self.set_cursor_from_flat(first_match.start);
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
            self.execute_substitution_direct(start_line, end_line, pattern, replacement, flags)
        }
    }

    pub fn take_ex_action(&mut self) -> Option<ExCommandAction> {
        self.pending_ex_action.take()
    }

    fn resolve_ex_line_atom(&self, atom: &str) -> Option<usize> {
        let atom = atom.trim();
        if atom.is_empty() {
            return Some(self.cursor_line());
        }
        if atom == "." {
            return Some(self.cursor_line());
        }
        if atom == "$" {
            return Some(self.line_count().saturating_sub(1));
        }
        if atom == "'<" {
            return self.last_visual_selection.map(|(r, _)| self.line_for_flat(r.start));
        }
        if atom == "'>" {
            return self
                .last_visual_selection
                .map(|(r, _)| self.line_for_flat(r.end.saturating_sub(1)));
        }
        if let Some((base, offset)) = atom.split_once('+') {
            let base_line = self.resolve_ex_line_atom(base)?;
            let delta = offset.parse::<usize>().ok()?;
            return Some((base_line + delta).min(self.line_count().saturating_sub(1)));
        }
        atom.parse::<usize>().ok().map(|n| n.saturating_sub(1))
    }

    fn resolve_ex_range(&self, range: &str) -> (usize, usize) {
        match range.trim() {
            "" => (self.cursor_line(), self.cursor_line()),
            "%" => (0, self.line_count().saturating_sub(1)),
            "'<,'>" => {
                if let Some((r, _)) = self.last_visual_selection {
                    let start = self.line_for_flat(r.start);
                    let end = self.line_for_flat(r.end.saturating_sub(1));
                    (start.min(end), start.max(end))
                } else {
                    (self.cursor_line(), self.cursor_line())
                }
            }
            other => {
                if let Some((start_str, end_str)) = other.split_once(',') {
                    let start = self
                        .resolve_ex_line_atom(start_str)
                        .unwrap_or(self.cursor_line());
                    let end = self.resolve_ex_line_atom(end_str).unwrap_or(start);
                    (start.min(end), start.max(end))
                } else {
                    let line = self.resolve_ex_line_atom(other).unwrap_or(self.cursor_line());
                    (line, line)
                }
            }
        }
    }

    fn execute_ex_command(&mut self, command: &str) -> bool {
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
                let target = line_num.clamp(1, self.line_count());
                self.cursor_line = target - 1;
                self.cursor_col = 0;
                self.clamp_cursor_normal();
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
                let (start_line, end_line) = self.resolve_ex_range("");
                self.execute_substitution(start_line, end_line, &last.pattern, &last.replacement, &flags)
            }
            Ok(ExCommand::Substitute { range, pattern, replacement, flags }) => {
                let (start_line, end_line) = self.resolve_ex_range(&range);
                let pattern = if pattern.is_empty() {
                    self.last_substitute
                        .as_ref()
                        .map(|last| last.pattern.clone())
                        .or_else(|| self.search_pattern().map(ToOwned::to_owned))
                        .unwrap_or_default()
                } else {
                    pattern
                };
                self.execute_substitution(start_line, end_line, &pattern, &replacement, &flags)
            }
            Ok(ExCommand::SetOption(key, val)) => {
                match key.as_str() {
                    "number" | "nu" => self.settings.number = val == "true",
                    "relativenumber" | "rnu" => self.settings.relativenumber = val == "true",
                    "wrap" => self.settings.wrap = val == "true",
                    "ignorecase" | "ic" => self.settings.ignorecase = val == "true",
                    "smartcase" | "scs" => self.settings.smartcase = val == "true",
                    "hlsearch" | "hls" => self.settings.hlsearch = val == "true",
                    "incsearch" | "is" => self.settings.incsearch = val == "true",
                    "tabstop" | "ts" => {
                        if let Ok(n) = val.parse::<usize>() {
                            self.settings.tabstop = n.max(1);
                        }
                    }
                    "shiftwidth" | "sw" => {
                        if let Ok(n) = val.parse::<usize>() {
                            self.settings.shiftwidth = n.max(1);
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
                self.search.highlights_active = false;
                false
            }
            Ok(ExCommand::Registers(filter)) => {
                self.pending_ex_action =
                    Some(ExCommandAction::ShowMessage(self.format_registers(filter)));
                false
            }
            Ok(ExCommand::Marks) => {
                self.pending_ex_action = Some(ExCommandAction::ShowMessage(self.format_marks()));
                false
            }
            Ok(ExCommand::Jumps) => {
                self.pending_ex_action = Some(ExCommandAction::ShowMessage(self.format_jumps()));
                false
            }
            Ok(ExCommand::Changes) => {
                self.pending_ex_action = Some(ExCommandAction::ShowMessage(self.format_changes()));
                false
            }
            Err(_) => false,
        }
    }

    fn format_registers(&self, filter: Option<Vec<char>>) -> String {
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
            push_row('"', &self.registers.unnamed);
        }
        if allow('0', &filter) {
            if let Some(value) = self.registers.named.get(&'0') {
                push_row('0', value);
            }
        }
        if allow('-', &filter) {
            if let Some(value) = self.registers.named.get(&'-') {
                push_row('-', value);
            }
        }
        for (name, value) in &self.registers.named {
            if *name == '0' || *name == '-' || !allow(*name, &filter) {
                continue;
            }
            push_row(*name, value);
        }
        if allow('+', &filter) {
            push_row('+', &self.registers.clipboard);
        }
        if rows.is_empty() {
            "No registers".to_string()
        } else {
            rows.join(" | ")
        }
    }

    fn format_marks(&self) -> String {
        if self.marks.is_empty() {
            return "No marks".to_string();
        }
        self.marks
            .iter()
            .map(|(name, pos)| format!("{name}:{}:{}", pos.line + 1, pos.col + 1))
            .collect::<Vec<_>>()
            .join(" | ")
    }

    fn format_jumps(&self) -> String {
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

    fn format_changes(&self) -> String {
        if self.changelist.is_empty() {
            return "No changes".to_string();
        }
        self.changelist
            .iter()
            .enumerate()
            .map(|(index, pos)| format!("{index}:{}:{}", pos.line + 1, pos.col + 1))
            .collect::<Vec<_>>()
            .join(" | ")
    }

    fn start_macro_recording(&mut self, register: char) {
        self.vim_state.recording_macro = Some(register);
        self.vim_state.macros.entry(register).or_default().clear();
    }

    fn stop_macro_recording(&mut self) {
        self.vim_state.recording_macro = None;
    }

    fn play_macro(&mut self, register: char, count: usize) -> bool {
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
                changed |= self.handle_editor_key(key.clone());
            }
        }
        
        self.macro_depth -= 1;
        changed
    }

    fn last_played_macro(&self) -> Option<char> {
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LastSubstitute {
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
enum RegisterTarget {
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
struct DocumentSnapshot {
    content: String,
    mode: VimMode,
    cursor_line: usize,
    cursor_col: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CursorPosition {
    line: usize,
    col: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VisualKind {
    Character,
    Line,
    Block,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct SearchState {
    pattern: String,
    reverse: bool,
    matches: Vec<TextRange>,
    highlights_active: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PastePlacement {
    After,
    Before,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DeferredAction {
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
struct Registers {
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

#[cfg(test)]
mod tests {
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

        assert!(doc.handle_normal_input("dw"));

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

        assert!(doc.handle_normal_input("3dw"));

        assert_eq!(doc.content(), "delta");
        assert_eq!(doc.cursor_col(), 0);

        let mut doc = NoteDocument::default();
        doc.content = "alpha beta gamma delta".to_string();
        doc.enter_normal();

        assert!(doc.handle_normal_input("d3w"));

        assert_eq!(doc.content(), "delta");
        assert_eq!(doc.cursor_col(), 0);
    }

    #[test]
    fn normal_mode_change_to_line_end_enters_insert() {
        let mut doc = NoteDocument::default();
        doc.content = "alpha beta".to_string();
        doc.enter_normal();
        doc.handle_normal_input("w");

        assert!(doc.handle_normal_input("c$"));

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

        assert!(doc.handle_normal_input("cc"));

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

        assert!(inner.handle_normal_input("ciw"));

        assert_eq!(inner.content(), " world");
        assert_eq!(inner.cursor_col(), 0);
        assert_eq!(inner.mode(), VimMode::Insert);

        let mut around = NoteDocument::default();
        around.content = "hello world".to_string();
        around.enter_normal();
        around.handle_normal_input("l");

        assert!(around.handle_normal_input("daw"));

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

        assert!(!doc.handle_normal_input("yy"));

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

        assert!(doc.handle_normal_input("dw"));

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

        assert!(doc.handle_normal_input("2x"));

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

        assert!(doc.handle_normal_input("dd"));
        assert_eq!(doc.content(), "two\nthree");

        assert!(doc.handle_normal_input("u"));
        assert_eq!(doc.content(), "one\ntwo\nthree");

        assert!(doc.handle_normal_input("ctrl+r"));
        assert_eq!(doc.content(), "two\nthree");
    }

    #[test]
    fn normal_mode_dot_repeats_last_change_without_repeating_undo() {
        let mut doc = NoteDocument::default();
        doc.content = "abcdef".to_string();
        doc.enter_normal();

        assert!(doc.handle_normal_input("x"));
        assert!(doc.handle_normal_input("."));
        assert_eq!(doc.content(), "cdef");

        assert!(doc.handle_normal_input("u"));
        assert_eq!(doc.content(), "bcdef");
        assert!(doc.handle_normal_input("."));
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

        assert!(doc.handle_normal_input(">>"));
        assert_eq!(doc.content(), "  one  \ntwo\t\nthree");

        assert!(doc.handle_normal_input("<<"));
        assert_eq!(doc.content(), "one  \ntwo\t\nthree");

        assert!(doc.handle_normal_input("=j"));
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
        assert!(doc.handle_normal_input("gv"));
        assert_eq!(doc.mode(), VimMode::VisualLine);
        assert!(doc.line_selection_cols(0).is_some());
        assert!(doc.line_selection_cols(1).is_some());

        doc.handle_normal_input("o");
        assert_eq!(doc.cursor_line(), 0);

        assert!(doc.handle_normal_input("d"));
        assert_eq!(doc.content(), "three");
    }

    #[test]
    fn search_navigation_highlights_noh_and_word_search_work() {
        let mut doc = NoteDocument::default();
        doc.content = "one two\nthree two\nfour".to_string();
        doc.enter_normal();

        doc.handle_normal_input("/");
        doc.handle_normal_input("two");
        assert!(doc.handle_normal_input("return"));

        assert_eq!(doc.cursor_line(), 0);
        assert_eq!(doc.cursor_col(), 4);
        assert!(doc.line_has_search_match(0));
        assert!(doc.line_has_search_match(1));

        assert!(doc.handle_normal_input("n"));
        assert_eq!(doc.cursor_line(), 1);
        assert_eq!(doc.cursor_col(), 6);

        assert!(doc.handle_normal_input("N"));
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
}
