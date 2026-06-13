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
            self.count = Some(self.count.unwrap_or(0).saturating_mul(10).saturating_add(digit));
            return false;
        }

        match (self.pending, ch) {
            (Some(PendingCommand::Delete), 'd') => {
                self.pending = None;
                self.count = None;
                self.delete_current_line();
                true
            }
            (Some(PendingCommand::Goto), 'g') => {
                self.pending = None;
                self.cursor_line = self.count.take().unwrap_or(1).saturating_sub(1);
                self.clamp_cursor_normal();
                false
            }
            _ => {
                self.pending = None;
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
                        self.pending = Some(PendingCommand::Goto);
                        false
                    }
                    'd' => {
                        self.pending = Some(PendingCommand::Delete);
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
        let mut lines = self.lines_vec();
        if lines.len() <= 1 {
            lines[0].clear();
        } else {
            lines.remove(self.cursor_line());
        }
        self.replace_lines(lines);
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
    Delete,
    Goto,
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
}
