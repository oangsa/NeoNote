use std::{
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
            VimMode::Normal => self.cursor_col(),
            VimMode::Insert => self.cursor_col.min(self.current_line_char_count()),
        }
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

    pub fn enter_insert(&mut self) {
        self.mode = VimMode::Insert;
        self.pending = None;
        self.count = None;
    }

    pub fn enter_normal(&mut self) {
        self.mode = VimMode::Normal;
        self.pending = None;
        self.count = None;
        self.clamp_cursor_normal();
    }

    pub fn handle_normal_input(&mut self, input: &str) -> bool {
        if self.mode != VimMode::Normal {
            return false;
        }

        let mut changed = false;
        for ch in input.chars() {
            changed |= self.handle_normal_char(ch);
        }
        changed
    }

    pub fn handle_insert_text(&mut self, text: &str) {
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

        if let Some(pending) = self.pending {
            return self.handle_pending_normal_char(pending, ch);
        }

        let explicit_count = self.count.take();
        let count = explicit_count.unwrap_or(1).max(1);
        match ch {
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
            'x' => {
                for _ in 0..count {
                    self.delete_char_on_current_line();
                }
                true
            }
            _ => false,
        }
    }

    fn handle_pending_normal_char(&mut self, pending: PendingCommand, ch: char) -> bool {
        match (pending, ch) {
            (
                PendingCommand::Operator {
                    operator: Operator::Delete,
                    count,
                },
                'd',
            ) => {
                self.pending = None;
                self.count = None;
                self.delete_current_lines(count);
                true
            }
            (
                PendingCommand::Operator {
                    operator: Operator::Change,
                    count,
                },
                'c',
            ) => {
                self.pending = None;
                self.count = None;
                self.change_current_lines(count);
                true
            }
            (
                PendingCommand::Operator {
                    operator: Operator::Yank,
                    count,
                },
                'y',
            ) => {
                self.pending = None;
                self.count = None;
                self.yank_current_lines(count);
                false
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
                self.pending = Some(PendingCommand::OperatorGoto { operator, count });
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
                    object == 'W',
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
            (PendingCommand::OperatorGoto { operator, count: _ }, 'g') => {
                let target_line = self.count.take().unwrap_or(1).saturating_sub(1);
                self.pending = None;
                self.apply_operator_range(
                    operator,
                    self.linewise_range(self.cursor_line(), target_line),
                )
            }
            _ => {
                self.pending = None;
                self.count = None;
                false
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
        self.content = lines.join("\n");
        self.dirty = true;
        self.clamp_cursor_line();
    }

    fn replace_lines_keep_insert(&mut self, lines: Vec<String>) {
        self.content = lines.join("\n");
        self.dirty = true;
        self.cursor_line = self.cursor_line.min(self.line_count().saturating_sub(1));
        self.cursor_col = self.cursor_col.min(self.current_line_char_count());
    }

    fn insert_blank_line(&mut self, index: usize) {
        let mut lines = self.lines_vec();
        let index = index.min(lines.len());
        lines.insert(index, String::new());
        self.cursor_line = index;
        self.replace_lines(lines);
    }

    fn delete_current_line(&mut self) {
        self.delete_current_lines(1);
    }

    fn delete_current_lines(&mut self, count: usize) {
        self.capture_line_register(count, false);
        let mut lines = self.lines_vec();
        if lines.len() <= 1 {
            lines[0].clear();
        } else {
            let index = self.cursor_line();
            let remove_count = count.max(1).min(lines.len().saturating_sub(index));
            for _ in 0..remove_count {
                lines.remove(index);
                if lines.is_empty() {
                    lines.push(String::new());
                    break;
                }
            }
        }
        self.replace_lines(lines);
    }

    fn change_current_lines(&mut self, count: usize) {
        self.capture_line_register(count, false);
        let mut lines = self.lines_vec();
        let index = self.cursor_line();
        if lines.len() <= 1 {
            lines[0].clear();
        } else {
            let remove_count = count.max(1).min(lines.len().saturating_sub(index));
            for _ in 0..remove_count {
                lines.remove(index);
                if lines.is_empty() {
                    break;
                }
            }
            lines.insert(index.min(lines.len()), String::new());
        }
        self.content = lines.join("\n");
        self.dirty = true;
        self.cursor_line = index.min(self.line_count().saturating_sub(1));
        self.cursor_col = 0;
        self.enter_insert();
    }

    fn yank_current_lines(&mut self, count: usize) {
        self.capture_line_register(count, true);
        self.clamp_cursor_normal();
    }

    fn delete_char_on_current_line(&mut self) {
        let mut lines = self.lines_vec();
        if let Some(line) = lines.get_mut(self.cursor_line()) {
            if !line.is_empty() {
                let col = self.cursor_col().min(char_count(line).saturating_sub(1));
                remove_char_at(line, col);
                self.replace_lines(lines);
            }
        }
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
        big_word: bool,
    ) -> bool {
        let Some(range) = self.word_text_object_range(count.max(1), around, big_word) else {
            return false;
        };

        self.apply_operator_range(operator, range)
    }

    fn apply_operator_range(&mut self, operator: Operator, range: TextRange) -> bool {
        let range = range.normalized().clamped(self.content_char_len());
        if range.start >= range.end {
            return false;
        }

        match operator {
            Operator::Delete => {
                let text = self.text_for_range(range);
                self.registers.store_deleted(text, range.linewise);
                self.delete_flat_range(range);
                self.set_insert_cursor_from_flat(range.start);
                self.enter_normal();
                true
            }
            Operator::Change => {
                let text = self.text_for_range(range);
                self.registers.store_deleted(text, range.linewise);
                self.delete_flat_range(range);
                self.set_insert_cursor_from_flat(range.start);
                self.enter_insert();
                true
            }
            Operator::Yank => {
                let text = self.text_for_range(range);
                self.registers.store_yank(text, range.linewise);
                self.clamp_cursor_normal();
                false
            }
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

    fn delete_flat_range(&mut self, range: TextRange) {
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

    fn capture_line_register(&mut self, count: usize, yank: bool) {
        let range = self.linewise_range(
            self.cursor_line(),
            self.cursor_line()
                .saturating_add(count.max(1).saturating_sub(1)),
        );
        let text = self.text_for_range(range);
        if yank {
            self.registers.store_yank(text, true);
        } else {
            self.registers.store_deleted(text, true);
        }
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
}

impl VimMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "NORMAL",
            Self::Insert => "INSERT",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PendingCommand {
    Operator {
        operator: Operator,
        count: usize,
    },
    TextObject {
        operator: Operator,
        count: usize,
        around: bool,
    },
    OperatorGoto {
        operator: Operator,
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
}

impl Registers {
    fn store_yank(&mut self, text: String, linewise: bool) {
        let value = RegisterValue { text, linewise };
        self.unnamed = value.clone();
        self.yank = value;
    }

    fn store_deleted(&mut self, text: String, linewise: bool) {
        self.unnamed = RegisterValue { text, linewise };
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NoteStats {
    pub line_count: usize,
    pub word_count: usize,
    pub char_count: usize,
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
    fn normal_mode_operator_gg_uses_linewise_range() {
        let mut yank_to_top = NoteDocument::default();
        yank_to_top.content = "one\ntwo\nthree\nfour".to_string();
        yank_to_top.enter_normal();
        yank_to_top.handle_normal_input("G");

        assert!(!yank_to_top.handle_normal_input("ygg"));

        assert_eq!(yank_to_top.content(), "one\ntwo\nthree\nfour");
        assert_eq!(yank_to_top.yank_register_text(), "one\ntwo\nthree\nfour");
        assert!(yank_to_top.yank_register_is_linewise());

        let mut yank_to_counted_line = NoteDocument::default();
        yank_to_counted_line.content = "one\ntwo\nthree\nfour".to_string();
        yank_to_counted_line.enter_normal();
        yank_to_counted_line.handle_normal_input("G");

        assert!(!yank_to_counted_line.handle_normal_input("y2gg"));

        assert_eq!(
            yank_to_counted_line.yank_register_text(),
            "two\nthree\nfour"
        );

        let mut delete_to_top = NoteDocument::default();
        delete_to_top.content = "one\ntwo\nthree\nfour".to_string();
        delete_to_top.enter_normal();
        delete_to_top.handle_normal_input("G");

        assert!(delete_to_top.handle_normal_input("dgg"));

        assert_eq!(delete_to_top.content(), "");
        assert_eq!(
            delete_to_top.unnamed_register_text(),
            "one\ntwo\nthree\nfour"
        );
        assert!(delete_to_top.unnamed_register_is_linewise());
    }
}
