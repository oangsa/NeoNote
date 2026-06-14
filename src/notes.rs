use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Default)]
pub struct NoteDocument {
    content: String,
    path: Option<PathBuf>,
    dirty: bool,
    open: bool,
    mode: VimMode,
    cursor_line: usize,
    cursor_col: usize,
    pending: Option<PendingCommand>,
    count: Option<usize>,
    registers: Registers,
    selected_register: Option<RegisterTarget>,
    undo_stack: Vec<DocumentSnapshot>,
    redo_stack: Vec<DocumentSnapshot>,
    last_change: Option<String>,
    replaying_change: bool,
    visual_anchor: Option<usize>,
    last_visual_selection: Option<(TextRange, VisualKind)>,
    search: SearchState,
    marks: BTreeMap<char, CursorPosition>,
    jump_list: Vec<CursorPosition>,
    jump_index: Option<usize>,
    yank_highlight: Option<TextRange>,
    deferred_action: Option<DeferredAction>,
    pub(crate) defer_enabled: bool,
}

impl NoteDocument {
    pub fn new_blank(&mut self) {
        self.content.clear();
        self.path = None;
        self.dirty = false;
        self.open = true;
        self.mode = VimMode::Normal;
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.pending = None;
        self.count = None;
        self.registers = Registers::default();
        self.selected_register = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.last_change = None;
        self.replaying_change = false;
        self.visual_anchor = None;
        self.last_visual_selection = None;
        self.search = SearchState::default();
        self.marks.clear();
        self.jump_list.clear();
        self.jump_index = None;
        self.yank_highlight = None;
        self.deferred_action = None;
    }

    pub fn open(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let path = path.as_ref();
        self.content = fs::read_to_string(path)?;
        self.path = Some(path.to_path_buf());
        self.dirty = false;
        self.open = true;
        self.mode = VimMode::Normal;
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.pending = None;
        self.count = None;
        self.registers = Registers::default();
        self.selected_register = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.last_change = None;
        self.replaying_change = false;
        self.visual_anchor = None;
        self.last_visual_selection = None;
        self.search = SearchState::default();
        self.marks.clear();
        self.jump_list.clear();
        self.jump_index = None;
        self.yank_highlight = None;
        self.deferred_action = None;
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
        self.mode
    }

    pub fn cursor_line(&self) -> usize {
        self.cursor_line.min(self.line_count().saturating_sub(1))
    }

    pub fn cursor_col(&self) -> usize {
        self.cursor_col.min(self.current_line_max_col())
    }

    pub fn display_cursor_col(&self) -> usize {
        match self.mode {
            VimMode::Normal | VimMode::Visual | VimMode::VisualLine => self.cursor_col(),
            VimMode::Insert => self.cursor_col.min(self.current_line_char_count()),
        }
    }

    pub fn line_is_visually_selected(&self, line: usize) -> bool {
        if let Some(range) = self.yank_highlight {
            let start_line = self.line_for_flat(range.start);
            let end_line = self.line_for_flat(range.end.saturating_sub(1));
            if (start_line.min(end_line)..=start_line.max(end_line)).contains(&line) {
                return true;
            }
        }
        self.visual_selection_range()
            .map(|range| {
                let start_line = self.line_for_flat(range.start);
                let end_line = self.line_for_flat(range.end.saturating_sub(1));
                (start_line.min(end_line)..=start_line.max(end_line)).contains(&line)
            })
            .unwrap_or(false)
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
            version: self.registers.version,
        }
    }

    pub fn set_unnamed_register(&mut self, text: String, linewise: bool) {
        self.registers.unnamed = RegisterValue { text, linewise };
    }

    pub fn set_clipboard_register(&mut self, text: String, linewise: bool) {
        let value = RegisterValue { text, linewise };
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
        self.pending = None;
        self.count = None;
        self.visual_anchor = None;
        if matches!(self.mode, VimMode::Visual | VimMode::VisualLine) {
            self.mode = VimMode::Normal;
        }
        self.cursor_line = line.min(self.line_count().saturating_sub(1));
        self.cursor_col = match self.mode {
            VimMode::Normal | VimMode::Visual | VimMode::VisualLine => {
                column.min(self.current_line_max_col())
            }
            VimMode::Insert => column.min(self.current_line_char_count()),
        };
    }

    pub fn enter_insert(&mut self) {
        self.mode = VimMode::Insert;
        self.pending = None;
        self.count = None;
        self.visual_anchor = None;
    }

    pub fn enter_normal(&mut self) {
        self.flush_deferred_action();
        self.clear_yank_highlight();
        self.mode = VimMode::Normal;
        self.pending = None;
        self.count = None;
        self.visual_anchor = None;
        self.clamp_cursor_normal();
    }

    pub fn handle_normal_input(&mut self, input: &str) -> bool {
        self.flush_deferred_action();
        self.clear_yank_highlight();
        if self.mode == VimMode::Insert {
            return false;
        }

        if let Some(PendingCommand::Search { .. }) = &self.pending {
            if let Some(PendingCommand::Search { reverse, query }) = self.pending.take() {
                return self.handle_pending_search(reverse, query, input);
            }
        }

        if let Some(PendingCommand::ExCommand { .. }) = &self.pending {
            if let Some(PendingCommand::ExCommand { command }) = self.pending.take() {
                return self.handle_pending_ex_command(command, input);
            }
        }

        if input == "ctrl+r" {
            return self.redo();
        }

        if input == "ctrl+o" {
            return self.jump_history(-1);
        }

        if input == "ctrl+i" {
            return self.jump_history(1);
        }

        if input == "." {
            return self.repeat_last_change();
        }

        let mut changed = false;
        for ch in input.chars() {
            changed |= match self.mode {
                VimMode::Visual | VimMode::VisualLine => self.handle_visual_char(ch),
                VimMode::Normal => self.handle_normal_char(ch),
                VimMode::Insert => false,
            };
        }
        if changed && !self.replaying_change {
            if is_repeatable_change(input) {
                self.last_change = Some(input.to_string());
            }
        }
        changed
    }

    pub fn handle_insert_text(&mut self, text: &str) {
        self.flush_deferred_action();
        self.clear_yank_highlight();
        if self.mode != VimMode::Insert || text.is_empty() {
            return;
        }

        let mut lines = self.lines_vec();
        let line_index = self.cursor_line().min(lines.len().saturating_sub(1));
        let col = self.cursor_col.min(char_count(&lines[line_index]));
        insert_str_at_char(&mut lines[line_index], col, text);
        self.cursor_line = line_index;
        self.cursor_col = col + text.chars().count();
        self.replace_lines_keep_insert(lines);
    }

    pub fn insert_newline(&mut self) {
        self.flush_deferred_action();
        self.clear_yank_highlight();
        if self.mode != VimMode::Insert {
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

    pub fn backspace(&mut self) {
        self.flush_deferred_action();
        self.clear_yank_highlight();
        if self.mode != VimMode::Insert {
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
        if self.mode != VimMode::Insert {
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

        if let Some(pending) = self.pending.take() {
            return match pending {
                PendingCommand::Search { reverse, query } => {
                    self.handle_pending_search(reverse, query, &ch.to_string())
                }
                PendingCommand::ExCommand { command } => {
                    self.handle_pending_ex_command(command, &ch.to_string())
                }
                other => self.handle_pending_normal_char(other, ch),
            };
        }

        let explicit_count = self.count.take();
        let count = explicit_count.unwrap_or(1).max(1);
        match ch {
            '"' => {
                self.pending = Some(PendingCommand::RegisterPrefix);
                false
            }
            ':' => {
                self.pending = Some(PendingCommand::ExCommand {
                    command: String::new(),
                });
                false
            }
            '/' | '?' => {
                self.pending = Some(PendingCommand::Search {
                    reverse: ch == '?',
                    query: String::new(),
                });
                false
            }
            'n' => self.repeat_search(false),
            'N' => self.repeat_search(true),
            '*' => self.search_word_under_cursor(false),
            '#' => self.search_word_under_cursor(true),
            'm' => {
                self.pending = Some(PendingCommand::MarkSet);
                false
            }
            '\'' => {
                self.pending = Some(PendingCommand::MarkJump);
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
                self.pending = Some(PendingCommand::Goto {
                    count: explicit_count,
                });
                false
            }
            'u' => self.undo(),
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
                self.cursor_line = if let Some(count) = explicit_count {
                    count.saturating_sub(1)
                } else {
                    self.line_count().saturating_sub(1)
                };
                self.clamp_cursor_normal();
                false
            }
            'g' => {
                self.pending = Some(PendingCommand::Goto {
                    count: explicit_count,
                });
                false
            }
            'd' => {
                self.pending = Some(PendingCommand::Operator {
                    operator: Operator::Delete,
                    count,
                });
                false
            }
            'c' => {
                self.pending = Some(PendingCommand::Operator {
                    operator: Operator::Change,
                    count,
                });
                false
            }
            'y' => {
                self.pending = Some(PendingCommand::Operator {
                    operator: Operator::Yank,
                    count,
                });
                false
            }
            '>' => {
                self.pending = Some(PendingCommand::Operator {
                    operator: Operator::Indent,
                    count,
                });
                false
            }
            '<' => {
                self.pending = Some(PendingCommand::Operator {
                    operator: Operator::Outdent,
                    count,
                });
                false
            }
            '=' => {
                self.pending = Some(PendingCommand::Operator {
                    operator: Operator::Format,
                    count,
                });
                false
            }
            'p' => self.paste_unnamed(count, PastePlacement::After),
            'P' => self.paste_unnamed(count, PastePlacement::Before),
            'x' => self.delete_chars_on_current_line(count),
            '~' => self.toggle_case_chars(count),
            _ => false,
        }
    }

    fn handle_pending_normal_char(&mut self, pending: PendingCommand, ch: char) -> bool {
        match (pending, ch) {
            (PendingCommand::RegisterPrefix, register) => {
                self.pending = None;
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
                self.cursor_col = position.col;
                self.clamp_cursor_normal();
                false
            }
            (
                PendingCommand::Operator {
                    operator: Operator::Delete,
                    count,
                },
                'd',
            ) => {
                self.pending = None;
                let multiplier = self.count.take().unwrap_or(1).max(1);
                self.apply_current_lines_operator(Operator::Delete, count.saturating_mul(multiplier))
            }
            (
                PendingCommand::Operator {
                    operator: Operator::Change,
                    count,
                },
                'c',
            ) => {
                self.pending = None;
                let multiplier = self.count.take().unwrap_or(1).max(1);
                self.apply_current_lines_operator(Operator::Change, count.saturating_mul(multiplier))
            }
            (
                PendingCommand::Operator {
                    operator: Operator::Yank,
                    count,
                },
                'y',
            ) => {
                self.pending = None;
                let multiplier = self.count.take().unwrap_or(1).max(1);
                self.apply_current_lines_operator(Operator::Yank, count.saturating_mul(multiplier))
            }
            (
                PendingCommand::Operator {
                    operator: Operator::Indent,
                    count,
                },
                '>',
            ) => {
                self.pending = None;
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
                self.pending = None;
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
                self.pending = None;
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
                self.pending = None;
                self.count = None;
                self.apply_operator_motion(Operator::Change, count, 'w')
            }
            (PendingCommand::Operator { operator, count }, 'i') => {
                self.pending = Some(PendingCommand::TextObject {
                    operator,
                    count,
                    around: false,
                });
                self.count = None;
                false
            }
            (PendingCommand::Operator { operator, count }, 'a') => {
                self.pending = Some(PendingCommand::TextObject {
                    operator,
                    count,
                    around: true,
                });
                self.count = None;
                false
            }
            (PendingCommand::Operator { operator, count }, 'g') => {
                self.pending = Some(PendingCommand::OperatorOrCaseGoto { operator, count });
                false
            }
            (PendingCommand::Goto { count }, 'u') => {
                self.pending = Some(PendingCommand::CaseOperator {
                    upper: false,
                    count: count.unwrap_or(1),
                });
                false
            }
            (PendingCommand::Goto { count }, 'U') => {
                self.pending = Some(PendingCommand::CaseOperator {
                    upper: true,
                    count: count.unwrap_or(1),
                });
                false
            }
            (
                PendingCommand::Operator { operator, count },
                motion @ ('h' | 'j' | 'k' | 'l' | 'w' | 'W' | 'b' | 'B' | 'e' | 'E' | '0' | '^'
                | '$' | 'G'),
            ) => {
                let motion_count = self.count.take().unwrap_or(1).max(1);
                self.pending = None;
                self.apply_operator_motion(operator, count.saturating_mul(motion_count), motion)
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
                self.pending = None;
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
                self.pending = None;
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
                self.pending = None;
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
            (PendingCommand::Goto { count }, 'g') => {
                self.pending = None;
                self.cursor_line = count
                    .or_else(|| self.count.take())
                    .unwrap_or(1)
                    .saturating_sub(1);
                self.clamp_cursor_normal();
                false
            }
            (PendingCommand::Goto { count: _ }, 'v') => {
                self.pending = None;
                self.restore_last_visual_selection()
            }
            (PendingCommand::OperatorOrCaseGoto { operator, count: _ }, 'g') => {
                let target_line = self.count.take().unwrap_or(1).saturating_sub(1);
                self.pending = None;
                self.apply_operator_range(
                    operator,
                    self.linewise_range(self.cursor_line(), target_line),
                )
            }
            (
                PendingCommand::CaseOperator { upper, count },
                motion @ ('h' | 'j' | 'k' | 'l' | 'w' | 'W' | 'b' | 'B' | 'e' | 'E' | '0' | '^'
                | '$' | 'G'),
            ) => {
                let motion_count = self.count.take().unwrap_or(1).max(1);
                self.pending = None;
                let Some(range) =
                    self.operator_motion_range(count.saturating_mul(motion_count), motion)
                else {
                    return false;
                };
                self.apply_case_range(range, upper)
            }
            _ => {
                self.pending = None;
                self.count = None;
                self.selected_register = None;
                false
            }
        }
    }

    fn handle_visual_char(&mut self, ch: char) -> bool {
        match ch {
            'v' if self.mode == VimMode::Visual => {
                self.enter_normal();
                false
            }
            'V' if self.mode == VimMode::VisualLine => {
                self.enter_normal();
                false
            }
            'v' => {
                self.mode = VimMode::Visual;
                false
            }
            'V' => {
                self.mode = VimMode::VisualLine;
                false
            }
            'o' => {
                let Some(anchor) = self.visual_anchor else {
                    return false;
                };
                let cursor = self.flattened_cursor();
                self.visual_anchor = Some(cursor);
                self.set_cursor_from_flat(anchor);
                false
            }
            'h' => {
                self.move_cursor_col(-1);
                false
            }
            'j' => {
                self.move_cursor_line(1);
                false
            }
            'k' => {
                self.move_cursor_line(-1);
                false
            }
            'l' => {
                self.move_cursor_col(1);
                false
            }
            'w' | 'W' => {
                self.move_word_forward(1, ch == 'W');
                false
            }
            'b' | 'B' => {
                self.move_word_backward(1, ch == 'B');
                false
            }
            'e' | 'E' => {
                self.move_word_end(1, ch == 'E');
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
                self.cursor_line = self.line_count().saturating_sub(1);
                self.clamp_cursor_normal();
                false
            }
            'd' | 'x' => self.apply_visual_operator(Operator::Delete),
            'c' => self.apply_visual_operator(Operator::Change),
            'y' => self.apply_visual_operator(Operator::Yank),
            '>' => self.apply_visual_operator(Operator::Indent),
            '<' => self.apply_visual_operator(Operator::Outdent),
            '=' => self.apply_visual_operator(Operator::Format),
            '~' => {
                let Some(range) = self.visual_selection_range() else {
                    return false;
                };
                self.last_visual_selection = Some((range, self.visual_kind()));
                self.visual_anchor = None;
                self.mode = VimMode::Normal;
                self.toggle_case_range(range)
            }
            'u' => {
                let Some(range) = self.visual_selection_range() else {
                    return false;
                };
                self.last_visual_selection = Some((range, self.visual_kind()));
                self.visual_anchor = None;
                self.mode = VimMode::Normal;
                self.apply_case_range(range, false)
            }
            'U' => {
                let Some(range) = self.visual_selection_range() else {
                    return false;
                };
                self.last_visual_selection = Some((range, self.visual_kind()));
                self.visual_anchor = None;
                self.mode = VimMode::Normal;
                self.apply_case_range(range, true)
            }
            ':' => {
                self.pending = Some(PendingCommand::ExCommand {
                    command: String::new(),
                });
                false
            }
            '/' | '?' => {
                self.pending = Some(PendingCommand::Search {
                    reverse: ch == '?',
                    query: String::new(),
                });
                false
            }
            _ => false,
        }
    }

    fn handle_pending_search(&mut self, reverse: bool, mut query: String, input: &str) -> bool {
        match input {
            "escape" => {
                self.pending = None;
                false
            }
            "backspace" => {
                query.pop();
                self.pending = Some(PendingCommand::Search { reverse, query });
                false
            }
            "return" => {
                if query.is_empty() {
                    return false;
                }
                self.search.pattern = query;
                self.search.reverse = reverse;
                self.refresh_search_matches();
                self.repeat_search(false)
            }
            text => {
                query.push_str(text);
                self.pending = Some(PendingCommand::Search { reverse, query });
                false
            }
        }
    }

    fn handle_pending_ex_command(&mut self, mut command: String, input: &str) -> bool {
        match input {
            "escape" => false,
            "backspace" => {
                command.pop();
                self.pending = Some(PendingCommand::ExCommand { command });
                false
            }
            "return" => self.execute_ex_command(&command),
            text => {
                command.push_str(text);
                if command == "noh" || command == "nohlsearch" {
                    self.execute_ex_command(&command)
                } else {
                    self.pending = Some(PendingCommand::ExCommand { command });
                    false
                }
            }
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
        self.content = lines.join("\n");
        self.dirty = true;
        self.clamp_cursor_line();
    }

    fn replace_lines_keep_insert(&mut self, lines: Vec<String>) {
        self.record_undo();
        self.content = lines.join("\n");
        self.dirty = true;
        self.cursor_line = self.cursor_line.min(self.line_count().saturating_sub(1));
        self.cursor_col = self.cursor_col.min(self.current_line_char_count());
    }

    fn snapshot_document(&self) -> DocumentSnapshot {
        DocumentSnapshot {
            content: self.content.clone(),
            mode: self.mode,
            cursor_line: self.cursor_line,
            cursor_col: self.cursor_col,
        }
    }

    fn restore_snapshot(&mut self, snapshot: DocumentSnapshot) {
        self.content = snapshot.content;
        self.mode = snapshot.mode;
        self.cursor_line = snapshot.cursor_line;
        self.cursor_col = snapshot.cursor_col;
        self.pending = None;
        self.count = None;
        self.selected_register = None;
        self.dirty = true;
        match self.mode {
            VimMode::Normal | VimMode::Visual | VimMode::VisualLine => self.clamp_cursor_normal(),
            VimMode::Insert => {
                self.cursor_line = self.cursor_line.min(self.line_count().saturating_sub(1));
                self.cursor_col = self.cursor_col.min(self.current_line_char_count());
            }
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
        if last_change == "." {
            return false;
        }

        self.replaying_change = true;
        let changed = self.handle_normal_input(&last_change);
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
        self.visual_anchor = Some(self.flattened_cursor());
        self.mode = match kind {
            VisualKind::Character => VimMode::Visual,
            VisualKind::Line => VimMode::VisualLine,
        };
        self.pending = None;
        self.count = None;
    }

    fn visual_kind(&self) -> VisualKind {
        match self.mode {
            VimMode::VisualLine => VisualKind::Line,
            _ => VisualKind::Character,
        }
    }

    fn visual_selection_range(&self) -> Option<TextRange> {
        let anchor = self.visual_anchor?;
        let cursor = self.flattened_cursor();
        match self.mode {
            VimMode::Visual => {
                let start = anchor.min(cursor);
                let end = anchor.max(cursor).saturating_add(1);
                Some(TextRange::new(start, end))
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
        self.visual_anchor = None;
        self.mode = VimMode::Normal;
        self.apply_operator_range(operator, range)
    }

    fn restore_last_visual_selection(&mut self) -> bool {
        let Some((range, kind)) = self.last_visual_selection else {
            return false;
        };
        self.visual_anchor = Some(range.start);
        match kind {
            VisualKind::Character => {
                self.mode = VimMode::Visual;
                self.set_cursor_from_flat(range.end.saturating_sub(1));
            }
            VisualKind::Line => {
                self.mode = VimMode::VisualLine;
                self.set_cursor_from_flat(range.end.saturating_sub(1));
            }
        }
        true
    }

    fn execute_ex_command(&mut self, command: &str) -> bool {
        self.pending = None;
        match command.trim() {
            "noh" | "nohlsearch" => {
                self.search.matches.clear();
                false
            }
            _ => false,
        }
    }

    fn refresh_search_matches(&mut self) {
        self.search.matches.clear();
        if self.search.pattern.is_empty() {
            return;
        }

        let chars: Vec<char> = self.content.chars().collect();
        let pattern: Vec<char> = self.search.pattern.chars().collect();
        if chars.is_empty() || pattern.is_empty() || pattern.len() > chars.len() {
            return;
        }

        for start in 0..=chars.len() - pattern.len() {
            if chars[start..start + pattern.len()] == pattern[..] {
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

    fn first_non_blank_col(&self) -> usize {
        self.lines_vec()
            .get(self.cursor_line())
            .and_then(|line| line.chars().position(|ch| !ch.is_whitespace()))
            .unwrap_or(0)
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

    fn apply_operator_range(&mut self, operator: Operator, range: TextRange) -> bool {
        let range = range.normalized().clamped(self.content_char_len());
        if !range.linewise && range.start >= range.end {
            return false;
        }

        if self.defer_enabled && (operator == Operator::Delete || operator == Operator::Change) && range.linewise {
            self.yank_highlight = Some(range);
            self.deferred_action = Some(DeferredAction { operator, range });
            return false;
        }

        self.apply_operator_range_direct(operator, range)
    }

    fn apply_operator_range_direct(&mut self, operator: Operator, range: TextRange) -> bool {
        match operator {
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
        let range = range.normalized().clamped(self.content_char_len());
        if range.start >= range.end {
            return false;
        }

        let replacement = self
            .text_for_range(range)
            .chars()
            .flat_map(|ch| {
                if upper {
                    ch.to_uppercase().collect::<Vec<_>>()
                } else {
                    ch.to_lowercase().collect::<Vec<_>>()
                }
            })
            .collect::<String>();
        self.replace_flat_range(range, &replacement);
        self.set_cursor_from_flat(range.start);
        true
    }

    fn replace_flat_range(&mut self, range: TextRange, replacement: &str) {
        self.record_undo();
        let start_byte = byte_index_for_char(&self.content, range.start);
        let end_byte = byte_index_for_char(&self.content, range.end);
        self.content
            .replace_range(start_byte..end_byte, replacement);
        self.dirty = true;
    }

    fn delete_flat_range(&mut self, range: TextRange) {
        self.record_undo();
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

        if register.linewise {
            self.paste_linewise(count.max(1), placement, register)
        } else {
            self.paste_charwise(count.max(1), placement, register)
        }
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
        if self.mode == VimMode::Normal {
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
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum VimMode {
    #[default]
    Normal,
    Insert,
    Visual,
    VisualLine,
}

impl VimMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "NORMAL",
            Self::Insert => "INSERT",
            Self::Visual => "VISUAL",
            Self::VisualLine => "V-LINE",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PendingCommand {
    RegisterPrefix,
    Search {
        reverse: bool,
        query: String,
    },
    ExCommand {
        command: String,
    },
    MarkSet,
    MarkJump,
    Operator {
        operator: Operator,
        count: usize,
    },
    TextObject {
        operator: Operator,
        count: usize,
        around: bool,
    },
    OperatorOrCaseGoto {
        operator: Operator,
        count: usize,
    },
    CaseOperator {
        upper: bool,
        count: usize,
    },
    Goto {
        count: Option<usize>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Operator {
    Delete,
    Change,
    Yank,
    Indent,
    Outdent,
    Format,
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
    Clipboard,
    BlackHole,
}

impl RegisterTarget {
    fn from_prefix(ch: char) -> Option<Self> {
        match ch {
            '+' => Some(Self::Clipboard),
            '_' => Some(Self::BlackHole),
            '"' => Some(Self::Unnamed),
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
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct SearchState {
    pattern: String,
    reverse: bool,
    matches: Vec<TextRange>,
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
struct TextRange {
    start: usize,
    end: usize,
    linewise: bool,
}

impl TextRange {
    fn new(start: usize, end: usize) -> Self {
        Self {
            start,
            end,
            linewise: false,
        }
    }

    fn linewise(start: usize, end: usize) -> Self {
        Self {
            start,
            end,
            linewise: true,
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
            }
        }
    }

    fn clamped(self, len: usize) -> Self {
        Self {
            start: self.start.min(len),
            end: self.end.min(len),
            linewise: self.linewise,
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
            RegisterTarget::Named(name) => self.named.get(&name).unwrap_or(&self.unnamed),
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
        let value = RegisterValue { text, linewise };
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
        self.store_target(target, RegisterValue { text, linewise });
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
            RegisterTarget::BlackHole => {}
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct RegisterValue {
    text: String,
    linewise: bool,
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

fn is_word_char(ch: char, big_word: bool) -> bool {
    if ch == '\n' || ch.is_whitespace() {
        return false;
    }
    big_word || ch.is_alphanumeric() || ch == '_'
}

fn is_repeatable_change(input: &str) -> bool {
    input != "u" && input != "ctrl+r" && !input.starts_with('y')
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
    pub version: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(doc.line_is_visually_selected(0));
        assert!(doc.line_is_visually_selected(1));

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
        assert!(doc.line_is_visually_selected(0));
        assert!(!doc.line_is_visually_selected(1));
        assert_eq!(doc.cursor_line(), 0);

        // 2. Clear highlight
        doc.clear_yank_highlight();
        assert!(!doc.has_yank_highlight());

        // 3. Test linewise delete (dd)
        doc.handle_normal_input("dd");
        assert!(doc.has_deferred_action());
        assert!(doc.has_yank_highlight());
        assert!(doc.line_is_visually_selected(0));
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
}
