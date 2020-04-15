#[repr(C)]
#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum InputKey<'a> {
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    // PageUp,
    // PageDown,
    Enter,
    Backspace,
    Del,
    Char(char),
    Text(&'a str),
}

#[repr(C)]
#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub struct InputModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl InputModifiers {
    pub fn none() -> InputModifiers {
        InputModifiers {
            shift: false,
            ctrl: false,
            alt: false,
        }
    }

    pub fn ctrl() -> InputModifiers {
        InputModifiers {
            shift: false,
            ctrl: true,
            alt: false,
        }
    }

    pub fn shift() -> InputModifiers {
        InputModifiers {
            shift: true,
            ctrl: false,
            alt: false,
        }
    }

    pub fn ctrl_shift() -> InputModifiers {
        InputModifiers {
            shift: true,
            ctrl: true,
            alt: false,
        }
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub struct Pos {
    pub row: usize,
    pub column: usize,
}

impl Pos {
    fn from_row_column(row_index: usize, column_index: usize) -> Pos {
        Pos {
            row: row_index,
            column: column_index,
        }
    }

    fn with_column(&self, col: usize) -> Pos {
        Pos {
            column: col,
            ..*self
        }
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub struct Selection {
    start: Pos,
    end: Option<Pos>,
}

impl Selection {
    pub fn from_pos(pos: Pos) -> Selection {
        Selection {
            start: pos,
            end: None,
        }
    }

    pub fn single(row_index: usize, column_index: usize) -> Selection {
        Selection {
            start: Pos {
                row: row_index,
                column: column_index,
            },
            end: None,
        }
    }

    pub fn range(start: Pos, end: Pos) -> Selection {
        Selection {
            start,
            end: if start == end { None } else { Some(end) },
        }
    }

    pub fn is_range(&self) -> bool {
        return self.end.is_some();
    }

    pub fn get_first(&self) -> Pos {
        if let Some(end) = self.end {
            let end_index = end.row * 1024 + end.column;
            let start_index = self.start.row * 1024 + self.start.column;
            if end_index < start_index {
                end
            } else {
                self.start
            }
        } else {
            self.start
        }
    }

    pub fn get_second(&self) -> Pos {
        if let Some(end) = self.end {
            let end_index = end.row * 1024 + end.column;
            let start_index = self.start.row * 1024 + self.start.column;
            if end_index > start_index {
                end
            } else {
                self.start
            }
        } else {
            self.start
        }
    }

    pub fn extend(&self, new_end: Pos) -> Selection {
        return if self.start == new_end {
            Selection::single(new_end.row, new_end.column)
        } else {
            Selection::range(self.start, new_end)
        };
    }

    pub fn get_cursor_pos(&self) -> Pos {
        self.end.unwrap_or(self.start)
    }
}

pub struct Editor {
    selection: Selection,
    last_column_index: usize,
    next_blink_at: u32,
    pub show_cursor: bool,
    max_line_len: usize,
    line_lens: Vec<usize>,
    canvas: Vec<char>,
}

pub struct FirstModifiedRowIndex(usize);

#[derive(Eq, PartialEq, Copy, Clone)]
enum JumpMode {
    IgnoreWhitespaces,
    ConsiderWhitespaces,
    BlockOnWhitespace,
}

impl Editor {
    pub fn new(max_len: usize) -> Editor {
        let mut ed = Editor {
            canvas: Vec::with_capacity(max_len * 32),
            line_lens: Vec::with_capacity(32),
            max_line_len: max_len,
            selection: Selection::single(0, 0),
            last_column_index: 0,
            next_blink_at: 0,
            show_cursor: false,
        };
        ed.push_line();
        return ed;
    }

    pub fn push_line(&mut self) {
        let line = std::iter::repeat(0 as char).take(self.max_line_len);
        self.canvas.extend(line);
        self.line_lens.push(0);
    }

    pub fn insert_line_at(&mut self, at: usize) {
        let start_pos = self.max_line_len * at;
        let line = std::iter::repeat(0 as char).take(self.max_line_len);
        self.canvas.splice(start_pos..start_pos, line);
        self.line_lens.insert(at, 0);
    }

    pub fn remove_line_at(&mut self, at: usize) {
        let from = self.max_line_len * at;
        let to = from + self.max_line_len;
        self.canvas.splice(from..to, std::iter::empty());
        self.line_lens.remove(at);
    }

    pub fn line_count(&self) -> usize {
        self.line_lens.len()
    }

    pub fn line_len(&self, row_i: usize) -> usize {
        self.line_lens[row_i]
    }

    fn get_char_pos(&self, row_index: usize, column_index: usize) -> usize {
        row_index * self.max_line_len + column_index
    }

    fn get_line_chars(&self, row_index: usize) -> &[char] {
        let from = row_index * self.max_line_len;
        let to = from + self.max_line_len;
        &self.canvas[from..to]
    }

    fn get_mut_line_chars(&mut self, row_index: usize) -> &mut [char] {
        let from = row_index * self.max_line_len;
        let to = from + self.max_line_len;
        &mut self.canvas[from..to]
    }

    pub fn get_char(&mut self, row_index: usize, column_index: usize) -> char {
        return self.canvas[self.get_char_pos(row_index, column_index)];
    }

    pub fn set_char(&mut self, row_index: usize, column_index: usize, ch: char) {
        let current_line_count = self.line_count();
        for _ in current_line_count..=row_index {
            self.push_line();
        }
        let char_pos = self.get_char_pos(row_index, column_index);
        self.canvas[char_pos] = ch;
    }

    pub fn insert_char(&mut self, row_index: usize, column_index: usize, ch: char) -> bool {
        if self.line_lens[row_index] == self.max_line_len {
            return false;
        }
        let from = self.get_char_pos(row_index, column_index);
        let len = self.line_lens[row_index];
        let to = self.get_char_pos(row_index, len);
        self.canvas.copy_within(from..to, from + 1);
        self.canvas[from] = ch;
        self.line_lens[row_index] += 1;
        return true;
    }

    pub fn remove_char(&mut self, row_index: usize, column_index: usize) -> bool {
        let from = self.get_char_pos(row_index, column_index);
        let len = self.line_lens[row_index];
        let to = self.get_char_pos(row_index, len);
        self.canvas.copy_within(from + 1..to, from);
        self.line_lens[row_index] -= 1;
        return true;
    }

    pub fn set_content(&mut self, text: &str) {
        self.clear();
        self.set_cursor_pos(0, 0);
        self.insert_at(text, 0, 0);
    }

    pub fn lines(&self) -> impl Iterator<Item = &[char]> {
        return self
            .canvas
            .chunks(self.max_line_len)
            .zip(self.line_lens.iter())
            .map(|(line, len)| &line[0..*len]);
    }

    pub fn get_content(&self) -> String {
        let mut result = String::with_capacity(self.canvas.len() * self.max_line_len);

        for line in self.lines() {
            result.extend(line);
            result.push('\n');
        }
        return result;
    }

    pub fn clear(&mut self) {
        for len in self.line_lens.iter_mut() {
            *len = 0;
        }
    }

    pub fn get_selection(&self) -> &Selection {
        &self.selection
    }

    pub fn handle_click(&mut self, x: usize, y: usize) {
        let line_count = self.line_count();
        let y = if y >= line_count { line_count - 1 } else { y };

        let col = x.min(self.line_len(y));
        self.selection = Selection::from_pos(Pos::from_row_column(y, col));
    }

    pub fn handle_drag(&mut self, x: usize, y: usize) {
        let y = if y >= self.line_count() {
            self.line_count() - 1
        } else {
            y
        };
        let col = x.min(self.line_len(y));
        self.selection = self.selection.extend(Pos::from_row_column(y, col));
    }

    pub fn get_selected_text(&self) -> Option<String> {
        if self.selection.end.is_none() {
            return None;
        }
        let start = self.selection.get_first();
        let end = self.selection.get_second();
        if end.row > start.row {
            let mut result = String::with_capacity((end.row - start.row) * self.max_line_len);
            // first line
            let from = self.get_char_pos(start.row, start.column);
            let to = self.get_char_pos(start.row, self.line_lens[start.row]);
            result.extend(&self.canvas[from..to]);
            result.push('\n');
            // full lines
            for i in start.row + 1..end.row {
                let from = self.get_char_pos(i, 0);
                let to = self.get_char_pos(i, self.line_lens[i]);
                result.extend(&self.canvas[from..to]);
                result.push('\n');
            }

            let from = self.get_char_pos(end.row, 0);
            let to = self.get_char_pos(end.row, end.column);
            result.extend(&self.canvas[from..to]);
            Some(result)
        } else {
            let from = self.get_char_pos(start.row, start.column);
            let to = self.get_char_pos(start.row, end.column);
            Some((&self.canvas[from..to]).iter().collect::<String>())
        }
    }

    pub fn set_cursor_pos(&mut self, row_index: usize, column_index: usize) {
        self.selection = Selection::single(row_index, column_index);
        self.last_column_index = column_index;
    }

    pub fn set_selection(&mut self, start: Pos, end: Pos) {
        self.selection = Selection::range(start, end);
        self.last_column_index = self.selection.get_cursor_pos().column;
    }

    pub fn handle_tick(&mut self, now: u32) -> bool {
        return if now >= self.next_blink_at {
            self.show_cursor = !self.show_cursor;
            self.next_blink_at = now + 500;
            true
        } else {
            false
        };
    }

    pub fn handle_input(
        &mut self,
        input: InputKey,
        modifiers: InputModifiers,
    ) -> FirstModifiedRowIndex {
        let cur_pos = self.selection.get_cursor_pos();
        match input {
            InputKey::Home => {
                let new_pos = cur_pos.with_column(0);
                self.selection = if modifiers.shift {
                    self.selection.extend(new_pos)
                } else {
                    Selection::from_pos(new_pos)
                };
                self.last_column_index = self.selection.get_cursor_pos().column;
            }
            InputKey::End => {
                let new_pos = cur_pos.with_column(self.line_lens[cur_pos.row]);
                self.selection = if modifiers.shift {
                    self.selection.extend(new_pos)
                } else {
                    Selection::from_pos(new_pos)
                };
                self.last_column_index = self.selection.get_cursor_pos().column;
            }
            InputKey::Right => {
                let new_pos = if cur_pos.column + 1 > self.line_lens[cur_pos.row] {
                    if cur_pos.row + 1 < self.line_count() {
                        Pos::from_row_column(cur_pos.row + 1, 0)
                    } else {
                        cur_pos
                    }
                } else {
                    let col = if modifiers.ctrl {
                        self.jump_word_forward(&cur_pos, JumpMode::IgnoreWhitespaces)
                    } else {
                        cur_pos.column + 1
                    };
                    cur_pos.with_column(col)
                };
                self.selection = if modifiers.shift {
                    self.selection.extend(new_pos)
                } else {
                    if self.selection.end.is_some() {
                        Selection::from_pos(self.selection.get_second())
                    } else {
                        Selection::from_pos(new_pos)
                    }
                };
                self.last_column_index = self.selection.get_cursor_pos().column;
            }
            InputKey::Left => {
                let new_pos = if cur_pos.column == 0 {
                    if cur_pos.row >= 1 {
                        Pos::from_row_column(cur_pos.row - 1, self.line_lens[cur_pos.row - 1])
                    } else {
                        cur_pos
                    }
                } else {
                    let col = if modifiers.ctrl {
                        // check the type of the prev char
                        self.jump_word_backward(&cur_pos, JumpMode::IgnoreWhitespaces)
                    } else {
                        cur_pos.column - 1
                    };
                    cur_pos.with_column(col)
                };

                self.selection = if modifiers.shift {
                    self.selection.extend(new_pos)
                } else {
                    if self.selection.end.is_some() {
                        Selection::from_pos(self.selection.get_first())
                    } else {
                        Selection::from_pos(new_pos)
                    }
                };
                self.last_column_index = self.selection.get_cursor_pos().column;
            }
            InputKey::Up => {
                let new_pos = if cur_pos.row == 0 {
                    cur_pos.with_column(0)
                } else {
                    Pos::from_row_column(
                        cur_pos.row - 1,
                        self.last_column_index.min(self.line_lens[cur_pos.row - 1]),
                    )
                };
                self.selection = if modifiers.shift {
                    self.selection.extend(new_pos)
                } else {
                    Selection::from_pos(new_pos)
                };
            }
            InputKey::Down => {
                let new_pos = if cur_pos.row == self.line_count() - 1 {
                    cur_pos.with_column(self.line_lens[cur_pos.row])
                } else {
                    Pos::from_row_column(
                        cur_pos.row + 1,
                        self.last_column_index.min(self.line_lens[cur_pos.row + 1]),
                    )
                };
                self.selection = if modifiers.shift {
                    self.selection.extend(new_pos)
                } else {
                    Selection::from_pos(new_pos)
                };
            }
            InputKey::Del => {
                if let Some(end) = self.selection.end {
                    let first = self.selection.get_first();
                    let second = self.selection.get_second();

                    self.remove_selection(first, second);
                    self.selection = Selection::from_pos(first);
                } else {
                    if cur_pos.column == self.line_lens[cur_pos.row] {
                        if cur_pos.row < self.line_count() - 1 {
                            self.merge_with_next_row(cur_pos.row, self.line_lens[cur_pos.row], 0);
                        }
                    } else if modifiers.ctrl {
                        let col = self.jump_word_forward(&cur_pos, JumpMode::ConsiderWhitespaces);
                        let new_pos = cur_pos.with_column(col);
                        self.remove_selection(cur_pos, new_pos);
                    } else {
                        self.remove_char(cur_pos.row, cur_pos.column);
                    }
                }
            }
            InputKey::Enter => {
                if let Some(end) = self.selection.end {
                    let first = self.selection.get_first();
                    let second = self.selection.get_second();

                    self.remove_selection(first, second);
                    self.split_line(first.row, first.column);
                    self.selection = Selection::from_pos(Pos::from_row_column(first.row + 1, 0));
                } else {
                    self.split_line(cur_pos.row, cur_pos.column);
                    self.selection = Selection::from_pos(Pos::from_row_column(cur_pos.row + 1, 0));
                }
            }
            InputKey::Backspace => {
                if self.selection.end.is_some() {
                    let first = self.selection.get_first();
                    let second = self.selection.get_second();

                    self.remove_selection(first, second);
                    self.selection = Selection::from_pos(first);
                } else {
                    if cur_pos.column == 0 {
                        if cur_pos.row > 0 {
                            let prev_row_i = cur_pos.row - 1;
                            let prev_len_before_merge = self.line_lens[prev_row_i];
                            if self.merge_with_next_row(prev_row_i, prev_len_before_merge, 0) {
                                self.selection = Selection::from_pos(Pos::from_row_column(
                                    prev_row_i,
                                    prev_len_before_merge,
                                ));
                            }
                        }
                    } else if modifiers.ctrl {
                        let col = self.jump_word_backward(&cur_pos, JumpMode::IgnoreWhitespaces);
                        let new_pos = cur_pos.with_column(col);
                        self.remove_selection(new_pos, cur_pos);
                        self.selection = Selection::from_pos(new_pos);
                    } else if self.remove_char(cur_pos.row, cur_pos.column - 1) {
                        self.selection =
                            Selection::from_pos(cur_pos.with_column(cur_pos.column - 1));
                    }
                }
            }
            InputKey::Char(ch) => {
                if ch == 'e' && modifiers.ctrl {
                    let prev_index = self.jump_word_backward(
                        &self.selection.get_first(),
                        if self.selection.end.is_some() {
                            JumpMode::IgnoreWhitespaces
                        } else {
                            JumpMode::BlockOnWhitespace
                        },
                    );
                    let next_index = self.jump_word_forward(
                        &self.selection.get_second(),
                        if self.selection.end.is_some() {
                            JumpMode::IgnoreWhitespaces
                        } else {
                            JumpMode::BlockOnWhitespace
                        },
                    );
                    self.selection = Selection::range(
                        cur_pos.with_column(prev_index),
                        cur_pos.with_column(next_index),
                    );
                } else if self.selection.end.is_some() {
                    let mut first = self.selection.get_first();
                    let second = self.selection.get_second();

                    // we will insert a char at this pos
                    first.column += 1;
                    if self.remove_selection(first, second) {
                        self.set_char(first.row, first.column - 1, ch);
                    }
                    self.selection = Selection::from_pos(first);
                } else {
                    if self.insert_char(cur_pos.row, cur_pos.column, ch) {
                        self.selection =
                            Selection::from_pos(cur_pos.with_column(cur_pos.column + 1));
                    }
                }
            }
            InputKey::Text(str) => {
                // save the content of first row which will be moved
                let mut text_to_move_buf: [u8; /*MAX_EDITOR_WIDTH * 4*/ 1024] = [0; 1024];
                let mut text_to_move_buf_index = 0;

                for ch in
                    &self.get_line_chars(cur_pos.row)[cur_pos.column..self.line_lens[cur_pos.row]]
                {
                    ch.encode_utf8(&mut text_to_move_buf[text_to_move_buf_index..]);
                    text_to_move_buf_index += ch.len_utf8();
                }

                let new_pos = self.insert_at(str, cur_pos.row, cur_pos.column);
                if text_to_move_buf_index > 0 {
                    let p = self.insert_at(
                        unsafe {
                            std::str::from_utf8_unchecked(
                                &text_to_move_buf[0..text_to_move_buf_index],
                            )
                        },
                        new_pos.row,
                        new_pos.column,
                    );
                    self.line_lens[p.row] = p.column;
                }
                self.selection = Selection::from_pos(new_pos);
            }
        }
        return FirstModifiedRowIndex(0);
    }

    fn jump_word_backward(&mut self, cur_pos: &Pos, mode: JumpMode) -> usize {
        let mut col = cur_pos.column;
        let line = self.get_line_chars(cur_pos.row);
        while col > 0 {
            if line[col - 1].is_alphanumeric() || line[col - 1] == '_' {
                col -= 1;
                while col > 0 && (line[col - 1].is_alphanumeric() || line[col - 1] == '_') {
                    col -= 1;
                }
                break;
            } else if line[col - 1] == '\"' {
                col -= 1;
                break;
            } else if !line[col - 1].is_ascii_whitespace() {
                col -= 1;
                while col > 0
                    && !(line[col - 1].is_alphanumeric()
                        || line[col - 1] == '_'
                        || line[col - 1] == '\"'
                        || line[col - 1].is_ascii_whitespace())
                {
                    col -= 1;
                }
                break;
            } else {
                match mode {
                    JumpMode::IgnoreWhitespaces => {
                        col -= 1;
                    }
                    JumpMode::ConsiderWhitespaces => {
                        col -= 1;
                        while col > 0 && line[col - 1].is_ascii_whitespace() {
                            col -= 1;
                        }
                        break;
                    }
                    JumpMode::BlockOnWhitespace => {
                        break;
                    }
                }
            }
        }
        col
    }

    fn jump_word_forward(&mut self, cur_pos: &Pos, mode: JumpMode) -> usize {
        // check the type of the prev char
        let mut col = cur_pos.column;
        let line = self.get_line_chars(cur_pos.row);
        let len = self.line_lens[cur_pos.row];
        while col < len {
            if line[col].is_alphanumeric() || line[col] == '_' {
                col += 1;
                while col < len && (line[col].is_alphanumeric() || line[col] == '_') {
                    col += 1;
                }
                break;
            } else if line[col] == '\"' {
                col += 1;
                break;
            } else if !line[col].is_ascii_whitespace() {
                col += 1;
                while col < len
                    && !(line[col].is_alphanumeric()
                        || line[col] == '_'
                        || line[col] == '\"'
                        || line[col].is_ascii_whitespace())
                {
                    col += 1;
                }
                break;
            } else {
                match mode {
                    JumpMode::IgnoreWhitespaces => {
                        col += 1;
                    }
                    JumpMode::ConsiderWhitespaces => {
                        col += 1;
                        while col < len && line[col].is_ascii_whitespace() {
                            col += 1;
                        }
                        break;
                    }
                    JumpMode::BlockOnWhitespace => {
                        break;
                    }
                }
            }
        }
        col
    }

    fn insert_at(&mut self, str: &str, row_index: usize, insert_at: usize) -> Pos {
        let mut col = insert_at;
        let mut row = row_index;
        for ch in str.chars() {
            if ch == '\r' {
                // ignore
                continue;
            } else if ch == '\n' {
                self.line_lens[row] = col;
                row += 1;
                self.insert_line_at(row);
                col = 0;
                continue;
            } else if col == self.max_line_len {
                self.line_lens[row] = col;
                row += 1;
                self.insert_line_at(row);
                col = 0;
            }
            self.set_char(row, col, ch);
            col += 1;
        }
        self.line_lens[row] = col;
        return Pos::from_row_column(row, col);
    }

    fn split_line(&mut self, row_index: usize, split_at: usize) {
        // let move_to_next_line = &lines[row_index].chars[split_at..lines[row_index].len];

        self.insert_line_at(row_index + 1);
        let new_line_pos = self.get_char_pos(row_index + 1, 0);

        // let move_to_next_line = {
        //     let from = self.get_char_pos(row_index, split_at);
        //     let to = self.line_lens[row_index];
        //     &self.canvas[from..to]
        // };
        {
            let from = self.get_char_pos(row_index, split_at);
            let to = self.get_char_pos(row_index, self.line_lens[row_index]);
            self.canvas.copy_within(from..to, new_line_pos);
            self.line_lens[row_index + 1] = to - from;
        }
        // self.canvas[new_line_pos..new_line_pos + move_to_next_line.len()]
        //     .copy_from_slice(move_to_next_line);
        // new_line.chars[0..move_to_next_line.len()].copy_from_slice(move_to_next_line);
        // new_line.len = move_to_next_line.len();

        // lines.insert(row_index + 1, new_line);
        self.line_lens[row_index] = split_at;
        // lines[row_index].len = split_at;
    }

    fn merge_with_next_row(
        &mut self,
        row_index: usize,
        first_row_col: usize,
        second_row_col: usize,
    ) -> bool {
        if self.line_lens[row_index] + self.line_lens[row_index + 1] > self.max_line_len {
            return false;
        }

        let dst = self.get_char_pos(row_index, first_row_col);
        let src_from = self.get_char_pos(row_index + 1, second_row_col);
        let src_to = self.get_char_pos(row_index + 1, self.line_lens[row_index + 1]);
        self.canvas.copy_within(src_from..src_to, dst);
        self.line_lens[row_index] = first_row_col + (src_to - src_from);

        self.remove_line_at(row_index + 1);

        // let tmp = lines.remove(row_index + 1);
        // let keep = &tmp.chars[second_row_col..tmp.len];
        // let from = first_row_col;
        // let to = from + keep.len();
        // lines[row_index].chars[from..to].copy_from_slice(keep);
        // lines[row_index].len = first_row_col + keep.len();

        return true;
    }

    fn remove_selection(&mut self, first: Pos, second: Pos) -> bool {
        if second.row > first.row {
            // töröld a közbenső egész sorokat teljesen
            for _ in first.row + 1..second.row {
                self.remove_line_at(first.row + 1);
            }
            self.merge_with_next_row(first.row, first.column, second.column);
        } else {
            self.get_mut_line_chars(first.row)
                .copy_within(second.column.., first.column);
            let selected_char_count = second.column - first.column;
            self.line_lens[first.row] -= selected_char_count;
        }
        return true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CURSOR_MARKER: char = '█';
    // U+2770	❰	e2 9d b0	HEAVY LEFT-POINTING ANGLE BRACKET OR­NA­MENT
    const SELECTION_START_MARK: char = '❱';
    const SELECTION_END_MARK: char = '❰';

    fn test(
        initial_content: &str,
        inputs: &[InputKey],
        modifiers: InputModifiers,
        expected_content: &str,
    ) {
        let mut editor = Editor::new(80);
        test0(
            &mut editor,
            initial_content,
            inputs,
            modifiers,
            expected_content,
        );
    }

    /// the strings in the parameter list are kind of a markup language
    /// '|' marks the cursor's position. If there are two of them, then
    /// it means a selection's begin and end.
    fn test0(
        editor: &mut Editor,
        initial_content: &str,
        inputs: &[InputKey],
        modifiers: InputModifiers,
        expected_content: &str,
    ) {
        // we can assume here that it does not contain illegal or complex input
        // so we can just set it as it is
        let mut selection_found = false;
        let mut selection_start = Pos { row: 0, column: 0 };
        let mut selection_end = Pos { row: 0, column: 0 };
        for (row_index, line) in initial_content.lines().enumerate() {
            let mut row_len = 0;
            for char in line.chars() {
                if char == CURSOR_MARKER {
                    editor.set_cursor_pos(row_index, row_len);
                } else if char == SELECTION_START_MARK {
                    selection_found = true;
                    selection_start = Pos {
                        row: row_index,
                        column: row_len,
                    };
                } else if char == SELECTION_END_MARK {
                    selection_end = Pos {
                        row: row_index,
                        column: row_len,
                    };
                } else {
                    editor.set_char(row_index, row_len, char);
                    row_len += 1;
                }
            }
            editor.line_lens[row_index] = row_len;
        }
        if selection_found {
            editor.set_selection(selection_start, selection_end);
        }

        for input in inputs {
            editor.handle_input(*input, modifiers);
        }

        // assert
        let editor: &Editor = editor;
        let mut expected_cursor = Selection::single(0, 0);
        let mut expected_selection_start = Pos { row: 0, column: 0 };
        let mut expected_selection_end = Pos { row: 0, column: 0 };
        let mut selection_found = false;
        for (row_index, expected_line) in expected_content.lines().enumerate() {
            let mut expected_row_len = 0;
            for char in expected_line.chars() {
                if char == CURSOR_MARKER {
                    expected_cursor = Selection::single(row_index, expected_row_len);
                } else if char == SELECTION_START_MARK {
                    selection_found = true;
                    expected_selection_start = Pos {
                        row: row_index,
                        column: expected_row_len,
                    }
                } else if char == SELECTION_END_MARK {
                    expected_selection_end = Pos {
                        row: row_index,
                        column: expected_row_len,
                    }
                } else {
                    assert_eq!(
                        editor.get_line_chars(row_index)[expected_row_len],
                        char,
                        "row: {}, column: {}, chars: {:?}",
                        row_index,
                        expected_row_len,
                        editor.get_line_chars(row_index)
                    );
                    expected_row_len += 1;
                }
            }

            assert!(
                editor.line_lens[row_index] <= expected_row_len,
                "Line {}, Actual data is longer: {:?}",
                row_index,
                &editor.get_line_chars(row_index)[expected_row_len..editor.line_lens[row_index]]
            );
            assert!(
                editor.line_lens[row_index] >= expected_row_len,
                "Line {}, Actual data is shorter,  actual: {:?} \n, expected: {:?}",
                row_index,
                &editor.get_line_chars(row_index)[0..editor.line_lens[row_index]],
                &expected_line[editor.line_lens[row_index]..expected_row_len]
            );
        }
        if selection_found {
            assert_eq!(
                editor.selection.start, expected_selection_start,
                "Selection start"
            );
            assert!(editor.selection.end.is_some());
            assert_eq!(
                editor.selection.end.unwrap(),
                expected_selection_end,
                "Selection end"
            );
        } else {
            assert_eq!(editor.selection, expected_cursor, "Cursor");
        }
    }

    #[test]
    fn test_the_test() {
        let mut editor = Editor::new(80);
        test0(
            &mut editor,
            "█abcdefghijklmnopqrstuvwxyz",
            &[],
            InputModifiers::none(),
            "█abcdefghijklmnopqrstuvwxyz",
        );
        assert_eq!(editor.selection.start.column, 0);
        assert_eq!(editor.selection.start.row, 0);
        assert_eq!(editor.selection.end, None);

        assert_eq!(editor.line_count(), 1);
        assert_eq!(editor.line_lens[0], 26);
        assert_eq!(editor.canvas[0], 'a');
        assert_eq!(editor.canvas[3], 'd');
        assert_eq!(editor.canvas[25], 'z');

        // single codepoint
        test0(
            &mut editor,
            "█abcdeéfghijklmnopqrstuvwxyz",
            &[],
            InputModifiers::none(),
            "█abcdee\u{301}fghijklmnopqrstuvwxyz",
        );
        assert_eq!(editor.selection.start.column, 0);
        assert_eq!(editor.selection.start.row, 0);
        assert_eq!(editor.selection.end, None);

        assert_eq!(editor.line_count(), 1);
        assert_eq!(editor.line_lens[0], 28);
        assert_eq!(editor.canvas[0], 'a');
        assert_eq!(editor.canvas[3], 'd');
        assert_eq!(editor.canvas[25], 'x');

        let lines = test0(
            &mut editor,
            "abcdefghijklmnopqrstuvwxyz\n\
            ABCD█EFGHIJKLMNOPQRSTUVWXY",
            &[],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            ABCD█EFGHIJKLMNOPQRSTUVWXY",
        );
        assert!(
            matches!(
                editor.selection,
                Selection {
                    start: Pos { row: 1, column: 4 },
                    end: None
                }
            ),
            "selection: {:?}",
            editor.selection
        );
        assert_eq!(editor.line_count(), 2);
        assert_eq!(editor.line_lens[1], 25);
        assert_eq!(editor.get_char(1, 0), 'A');
        assert_eq!(editor.get_char(1, 3), 'D');
        assert_eq!(editor.get_char(1, 24), 'Y');
    }

    #[test]
    #[should_panic(expected = "Cursor")]
    fn test_the_test2() {
        test(
            "█abcdefghijklmnopqrstuvwxyz",
            &[],
            InputModifiers::none(),
            "a█bcdefghijklmnopqrstuvwxyz",
        );
    }

    #[test]
    #[should_panic(expected = "row: 0, column: 1")]
    fn test_the_test3() {
        test(
            "█abcdefghijklmnopqrstuvwxyz",
            &[],
            InputModifiers::none(),
            "█aacdefghijklmnopqrstuvwxyz",
        );
    }

    #[test]
    #[should_panic(expected = "Actual data is longer: ['x', 'y', 'z']")]
    fn test_the_test4() {
        test(
            "█abcdefghijklmnopqrstuvwxyz",
            &[],
            InputModifiers::none(),
            "█abcdefghijklmnopqrstuvw",
        );
    }

    #[test]
    #[should_panic(expected = "row: 0, column: 23")]
    fn test_the_test5() {
        test(
            "█abcdefghijklmnopqrstuvw",
            &[],
            InputModifiers::none(),
            "█abcdefghijklmnopqrstuvwxyz",
        );
    }

    #[test]
    fn test_the_test_selection() {
        let mut editor = Editor::new(80);
        test0(
            &mut editor,
            "a❱bcdefghij❰klmnopqrstuvwxyz",
            &[],
            InputModifiers::none(),
            "a❱bcdefghij❰klmnopqrstuvwxyz",
        );
        assert!(
            matches!(
                editor.selection,
                Selection {
                    start: Pos { row: 0, column: 1 },
                    end: Some(Pos { row: 0, column: 10 })
                }
            ),
            "selection: {:?}",
            editor.selection
        );

        test0(
            &mut editor,
            "a❱bcdefghijklmnopqrstuvwxyz\n\
            abcdefghij❰klmnopqrstuvwxyz",
            &[],
            InputModifiers::none(),
            "a❱bcdefghijklmnopqrstuvwxyz\n\
            abcdefghij❰klmnopqrstuvwxyz",
        );
        assert!(
            matches!(
                editor.selection,
                Selection {
                    start: Pos { row: 0, column: 1 },
                    end: Some(Pos { row: 1, column: 10 })
                }
            ),
            "selection: {:?}",
            editor.selection
        );

        test0(
            &mut editor,
            "a❰bcdefghijklmnopqrstuvwxyz\n\
            abcdefghij❱klmnopqrstuvwxyz",
            &[],
            InputModifiers::none(),
            "a❰bcdefghijklmnopqrstuvwxyz\n\
            abcdefghij❱klmnopqrstuvwxyz",
        );
        assert!(
            matches!(
                editor.selection,
                Selection {
                    start: Pos { row: 1, column: 10 },
                    end: Some(Pos { row: 0, column: 1 })
                }
            ),
            "selection: {:?}",
            editor.selection
        );
    }

    #[test]
    #[should_panic(expected = "Selection start")]
    fn test_the_test_selection2() {
        let mut editor = Editor::new(80);
        test0(
            &mut editor,
            "a❱bcdefghij❰klmnopqrstuvwxyz",
            &[],
            InputModifiers::none(),
            "ab❱cdefghij❰klmnopqrstuvwxyz",
        );
    }

    #[test]
    #[should_panic(expected = "Selection end")]
    fn test_the_test_selection3() {
        let mut editor = Editor::new(80);
        test0(
            &mut editor,
            "a❱bcdefghij❰klmnopqrstuvwxyz",
            &[],
            InputModifiers::none(),
            "a❱bcdefghijk❰lmnopqrstuvwxyz",
        );
    }

    #[test]
    fn test_simple_right_cursor() {
        test(
            "█abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::none(),
            "a█bcdefghijklmnopqrstuvwxyz",
        );

        test(
            "█abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Right, InputKey::Right, InputKey::Right],
            InputModifiers::none(),
            "abc█defghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz█\n\
            ABCDEFGHIJKLMNOPQRSTUVWXY",
            &[InputKey::Right],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            █ABCDEFGHIJKLMNOPQRSTUVWXY",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz█\n\
            ABCDEFGHIJKLMNOPQRSTUVWXY",
            &[InputKey::Right, InputKey::Right, InputKey::Right],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            AB█CDEFGHIJKLMNOPQRSTUVWXY",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            ABCDEFGHIJKLMNOPQRSTUVWXY█",
            &[InputKey::Right],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            ABCDEFGHIJKLMNOPQRSTUVWXY█",
        );
    }

    #[test]
    fn test_simple_left_cursor() {
        let mut editor = Editor::new(80);
        test(
            "abcdefghij█klmnopqrstuvwxyz",
            &[InputKey::Left],
            InputModifiers::none(),
            "abcdefghi█jklmnopqrstuvwxyz",
        );

        test(
            "abcdefghij█klmnopqrstuvwxyz",
            &[InputKey::Left, InputKey::Left, InputKey::Left],
            InputModifiers::none(),
            "abcdefg█hijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            █ABCDEFGHIJKLMNOPQRSTUVWXY",
            &[InputKey::Left],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz█\n\
            ABCDEFGHIJKLMNOPQRSTUVWXY",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            █ABCDEFGHIJKLMNOPQRSTUVWXY",
            &[InputKey::Left, InputKey::Left, InputKey::Left],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwx█yz\n\
            ABCDEFGHIJKLMNOPQRSTUVWXY",
        );

        test(
            "█abcdefghijklmnopqrstuvwxyz\n\
            ABCDEFGHIJKLMNOPQRSTUVWXY",
            &[InputKey::Left],
            InputModifiers::none(),
            "█abcdefghijklmnopqrstuvwxyz\n\
            ABCDEFGHIJKLMNOPQRSTUVWXY",
        );
    }

    #[test]
    fn test_simple_up_cursor() {
        test(
            "abcdefghij█klmnopqrstuvwxyz",
            &[InputKey::Up],
            InputModifiers::none(),
            "█abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "█abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Up],
            InputModifiers::none(),
            "█abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            █ABCDEFGHIJKLMNOPQRSTUVWXY",
            &[InputKey::Up],
            InputModifiers::none(),
            "█abcdefghijklmnopqrstuvwxyz\n\
            ABCDEFGHIJKLMNOPQRSTUVWXY",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            ABCDEFGHI█JKLMNOPQRSTUVWXY",
            &[InputKey::Up],
            InputModifiers::none(),
            "abcdefghi█jklmnopqrstuvwxyz\n\
            ABCDEFGHIJKLMNOPQRSTUVWXY",
        );
    }

    #[test]
    fn test_simple_down_cursor() {
        test(
            "abcdefghij█klmnopqrstuvwxyz",
            &[InputKey::Down],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz█",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz█",
            &[InputKey::Down],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz█",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz█\n\
            ABCDEFGHIJKLMNOPQRSTUVWXY",
            &[InputKey::Down],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            ABCDEFGHIJKLMNOPQRSTUVWXY█",
        );

        test(
            "abcdefghi█jklmnopqrstuvwxyz\n\
            ABCDEFGHIJKLMNOPQRSTUVWXY",
            &[InputKey::Down],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            ABCDEFGHI█JKLMNOPQRSTUVWXY",
        );
    }

    #[test]
    fn test_column_index_keeping_navigation_up() {
        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqr█stuvwxyz",
            &[InputKey::Up],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijkl█\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqr█stuvwxyz",
            &[InputKey::Up, InputKey::Up],
            InputModifiers::none(),
            "abcdefghijklmnopqr█stuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqr█stuvwxyz",
            &[InputKey::Left, InputKey::Up, InputKey::Up],
            InputModifiers::none(),
            "abcdefghijklmnopq█rstuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqr█stuvwxyz",
            &[InputKey::Right, InputKey::Up, InputKey::Up],
            InputModifiers::none(),
            "abcdefghijklmnopqrs█tuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxyz█",
            &[InputKey::Up, InputKey::Up],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz█\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxy\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxyz█",
            &[InputKey::Up, InputKey::Up],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxy█\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxyz",
        );
    }

    #[test]
    fn test_column_index_keeping_navigation_down() {
        test(
            "abcdefghijklmnopqr█stuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Down],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijkl█\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqr█stuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Down, InputKey::Down],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqr█stuvwxyz",
        );

        test(
            "abcdefghijklmnopqr█stuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Left, InputKey::Down, InputKey::Down],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopq█rstuvwxyz",
        );

        test(
            "abcdefghijklmnopqr█stuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Right, InputKey::Down, InputKey::Down],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrs█tuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz█\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Down, InputKey::Down],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxyz█",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz█\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxy",
            &[InputKey::Down, InputKey::Down],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxy█",
        );
    }

    #[test]
    fn test_home_btn() {
        test(
            "abcdefghijklmnopqrstuvwxyz█",
            &[InputKey::Home],
            InputModifiers::none(),
            "█abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnop█qrstuvwxyz",
            &[InputKey::Home],
            InputModifiers::none(),
            "█abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "█abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Home],
            InputModifiers::none(),
            "█abcdefghijklmnopqrstuvwxyz",
        );
    }

    #[test]
    fn test_end_btn() {
        test(
            "█abcdefghijklmnopqrstuvwxyz",
            &[InputKey::End],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz█",
        );

        test(
            "█abcdefghijklmnopqrstuvwxyz",
            &[InputKey::End],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz█",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz█",
            &[InputKey::End],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz█",
        );
    }

    #[test]
    fn test_ctrl_plus_left() {
        test(
            "abcdefghijklmnopqrstuvwxyz█",
            &[InputKey::Left],
            InputModifiers::ctrl(),
            "█abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl mnopqrstuvwxyz█",
            &[InputKey::Left],
            InputModifiers::ctrl(),
            "abcdefghijkl █mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl █mnopqrstuvwxyz",
            &[InputKey::Left],
            InputModifiers::ctrl(),
            "█abcdefghijkl mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█ mnopqrstuvwxyz",
            &[InputKey::Left],
            InputModifiers::ctrl(),
            "█abcdefghijkl mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl    █mnopqrstuvwxyz",
            &[InputKey::Left],
            InputModifiers::ctrl(),
            "█abcdefghijkl    mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl  )  █mnopqrstuvwxyz",
            &[InputKey::Left],
            InputModifiers::ctrl(),
            "abcdefghijkl  █)  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl  |()-+%'^%/=?{}#<>&@[]*  █mnopqrstuvwxyz",
            &[InputKey::Left],
            InputModifiers::ctrl(),
            "abcdefghijkl  █|()-+%'^%/=?{}#<>&@[]*  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl  \"  █mnopqrstuvwxyz",
            &[InputKey::Left],
            InputModifiers::ctrl(),
            "abcdefghijkl  █\"  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl  12  █mnopqrstuvwxyz",
            &[InputKey::Left],
            InputModifiers::ctrl(),
            "abcdefghijkl  █12  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl  12a  █mnopqrstuvwxyz",
            &[InputKey::Left],
            InputModifiers::ctrl(),
            "abcdefghijkl  █12a  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl  a12  █mnopqrstuvwxyz",
            &[InputKey::Left],
            InputModifiers::ctrl(),
            "abcdefghijkl  █a12  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl  _  █mnopqrstuvwxyz",
            &[InputKey::Left],
            InputModifiers::ctrl(),
            "abcdefghijkl  █_  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl  _1a  █mnopqrstuvwxyz",
            &[InputKey::Left],
            InputModifiers::ctrl(),
            "abcdefghijkl  █_1a  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl  \"❤(  █mnopqrstuvwxyz",
            &[InputKey::Left],
            InputModifiers::ctrl(),
            "abcdefghijkl  \"█❤(  mnopqrstuvwxyz",
        );
    }

    #[test]
    fn test_ctrl_plus_right() {
        test(
            "█abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl(),
            "abcdefghijklmnopqrstuvwxyz█",
        );

        test(
            "█abcdefghijkl mnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl(),
            "abcdefghijkl█ mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█ mnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl(),
            "abcdefghijkl mnopqrstuvwxyz█",
        );

        test(
            "abcdefghijkl █mnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl(),
            "abcdefghijkl mnopqrstuvwxyz█",
        );

        test(
            "abcdefghijkl█    mnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl(),
            "abcdefghijkl    mnopqrstuvwxyz█",
        );

        test(
            "abcdefghijkl█  )  mnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl(),
            "abcdefghijkl  )█  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█  |()-+%'^%/=?{}#<>&@[]*  mnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl(),
            "abcdefghijkl  |()-+%'^%/=?{}#<>&@[]*█  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█  \"  mnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl(),
            "abcdefghijkl  \"█  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█  12  mnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl(),
            "abcdefghijkl  12█  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█  12a  mnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl(),
            "abcdefghijkl  12a█  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█  a12  mnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl(),
            "abcdefghijkl  a12█  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█  _  mnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl(),
            "abcdefghijkl  _█  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█  _1a  mnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl(),
            "abcdefghijkl  _1a█  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█  \"❤(  mnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl(),
            "abcdefghijkl  \"█❤(  mnopqrstuvwxyz",
        );
    }

    ///////////////////////////////////////////////////////
    ///////////////////////////////////////////////////////
    ///////////////////////////////////////////////////////
    /// SELECTION
    ///////////////////////////////////////////////////////
    ///////////////////////////////////////////////////////
    ///////////////////////////////////////////////////////
    ///////////////////////////////////////////////////////
    #[test]
    fn test_simple_right_cursor_selection() {
        test(
            "█abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::shift(),
            "❱a❰bcdefghijklmnopqrstuvwxyz",
        );

        test(
            "█abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Right, InputKey::Right, InputKey::Right],
            InputModifiers::shift(),
            "❱abc❰defghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz█\n\
            ABCDEFGHIJKLMNOPQRSTUVWXY",
            &[InputKey::Right],
            InputModifiers::shift(),
            "abcdefghijklmnopqrstuvwxyz❱\n\
            ❰ABCDEFGHIJKLMNOPQRSTUVWXY",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz█\n\
            ABCDEFGHIJKLMNOPQRSTUVWXY",
            &[InputKey::Right, InputKey::Right, InputKey::Right],
            InputModifiers::shift(),
            "abcdefghijklmnopqrstuvwxyz❱\n\
            AB❰CDEFGHIJKLMNOPQRSTUVWXY",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            ABCDEFGHIJKLMNOPQRSTUVWXY█",
            &[InputKey::Right],
            InputModifiers::shift(),
            "abcdefghijklmnopqrstuvwxyz\n\
            ABCDEFGHIJKLMNOPQRSTUVWXY█",
        );
    }

    #[test]
    fn test_simple_left_cursor_selection() {
        let mut editor = Editor::new(80);
        test(
            "abcdefghij█klmnopqrstuvwxyz",
            &[InputKey::Left],
            InputModifiers::shift(),
            "abcdefghi❰j❱klmnopqrstuvwxyz",
        );

        test(
            "abcdefghij█klmnopqrstuvwxyz",
            &[InputKey::Left, InputKey::Left, InputKey::Left],
            InputModifiers::shift(),
            "abcdefg❰hij❱klmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            █ABCDEFGHIJKLMNOPQRSTUVWXY",
            &[InputKey::Left],
            InputModifiers::shift(),
            "abcdefghijklmnopqrstuvwxyz❰\n\
            ❱ABCDEFGHIJKLMNOPQRSTUVWXY",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            █ABCDEFGHIJKLMNOPQRSTUVWXY",
            &[InputKey::Left, InputKey::Left, InputKey::Left],
            InputModifiers::shift(),
            "abcdefghijklmnopqrstuvwx❰yz\n\
            ❱ABCDEFGHIJKLMNOPQRSTUVWXY",
        );

        test(
            "█abcdefghijklmnopqrstuvwxyz\n\
            ABCDEFGHIJKLMNOPQRSTUVWXY",
            &[InputKey::Left],
            InputModifiers::shift(),
            "█abcdefghijklmnopqrstuvwxyz\n\
            ABCDEFGHIJKLMNOPQRSTUVWXY",
        );
    }

    #[test]
    fn test_left_right_cursor_selection() {
        let mut editor = Editor::new(80);
        test(
            "abcdefghij█klmnopqrstuvwxyz",
            &[
                InputKey::Left,
                InputKey::Left,
                InputKey::Left,
                InputKey::Right,
                InputKey::Right,
                InputKey::Right,
            ],
            InputModifiers::shift(),
            "abcdefghij█klmnopqrstuvwxyz",
        );

        test(
            "abcdefghij█klmnopqrstuvwxyz",
            &[
                InputKey::Left,
                InputKey::Left,
                InputKey::Left,
                InputKey::Right,
                InputKey::Right,
                InputKey::Right,
                InputKey::Right,
            ],
            InputModifiers::shift(),
            "abcdefghij❱k❰lmnopqrstuvwxyz",
        );

        test(
            "abcdefghij█klmnopqrstuvwxyz",
            &[
                InputKey::Left,
                InputKey::Left,
                InputKey::Left,
                InputKey::Right,
                InputKey::Right,
                InputKey::Right,
                InputKey::Right,
                InputKey::Right,
                InputKey::Right,
            ],
            InputModifiers::shift(),
            "abcdefghij❱klm❰nopqrstuvwxyz",
        );
    }

    #[test]
    fn test_simple_up_cursor_selection() {
        test(
            "abcdefghij█klmnopqrstuvwxyz",
            &[InputKey::Up],
            InputModifiers::shift(),
            "❰abcdefghij❱klmnopqrstuvwxyz",
        );

        test(
            "█abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Up],
            InputModifiers::shift(),
            "█abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            █ABCDEFGHIJKLMNOPQRSTUVWXY",
            &[InputKey::Up],
            InputModifiers::shift(),
            "❰abcdefghijklmnopqrstuvwxyz\n\
            ❱ABCDEFGHIJKLMNOPQRSTUVWXY",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            ABCDEFGHI█JKLMNOPQRSTUVWXY",
            &[InputKey::Up],
            InputModifiers::shift(),
            "abcdefghi❰jklmnopqrstuvwxyz\n\
            ABCDEFGHI❱JKLMNOPQRSTUVWXY",
        );
    }

    #[test]
    fn test_simple_down_cursor_selection() {
        test(
            "abcdefghij█klmnopqrstuvwxyz",
            &[InputKey::Down],
            InputModifiers::shift(),
            "abcdefghij❱klmnopqrstuvwxyz❰",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz█",
            &[InputKey::Down],
            InputModifiers::shift(),
            "abcdefghijklmnopqrstuvwxyz█",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz█\n\
            ABCDEFGHIJKLMNOPQRSTUVWXY",
            &[InputKey::Down],
            InputModifiers::shift(),
            "abcdefghijklmnopqrstuvwxyz❱\n\
            ABCDEFGHIJKLMNOPQRSTUVWXY❰",
        );

        test(
            "abcdefghi█jklmnopqrstuvwxyz\n\
            ABCDEFGHIJKLMNOPQRSTUVWXY",
            &[InputKey::Down],
            InputModifiers::shift(),
            "abcdefghi❱jklmnopqrstuvwxyz\n\
            ABCDEFGHI❰JKLMNOPQRSTUVWXY",
        );
    }

    #[test]
    fn test_column_index_keeping_navigation_up_selection() {
        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqr█stuvwxyz",
            &[InputKey::Up],
            InputModifiers::shift(),
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijkl❰\n\
            abcdefghijklmnopqr❱stuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqr█stuvwxyz",
            &[InputKey::Up, InputKey::Up],
            InputModifiers::shift(),
            "abcdefghijklmnopqr❰stuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqr❱stuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqr█stuvwxyz",
            &[InputKey::Left, InputKey::Up, InputKey::Up],
            InputModifiers::shift(),
            "abcdefghijklmnopq❰rstuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqr❱stuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqr█stuvwxyz",
            &[InputKey::Right, InputKey::Up, InputKey::Up],
            InputModifiers::shift(),
            "abcdefghijklmnopqrs❰tuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqr❱stuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxyz█",
            &[InputKey::Up, InputKey::Up],
            InputModifiers::shift(),
            "abcdefghijklmnopqrstuvwxyz❰\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxyz❱",
        );

        test(
            "abcdefghijklmnopqrstuvwxy\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxyz█",
            &[InputKey::Up, InputKey::Up],
            InputModifiers::shift(),
            "abcdefghijklmnopqrstuvwxy❰\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxyz❱",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijkl\n\
            █abcdefghijklmnopqrstuvwxyz",
            &[InputKey::End, InputKey::Up],
            InputModifiers::shift(),
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijkl❰\n\
            ❱abcdefghijklmnopqrstuvwxyz",
        );
    }

    #[test]
    fn test_column_index_keeping_navigation_down_selection() {
        test(
            "abcdefghijklmnopqr█stuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Down],
            InputModifiers::shift(),
            "abcdefghijklmnopqr❱stuvwxyz\n\
            abcdefghijkl❰\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqr█stuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Down, InputKey::Down],
            InputModifiers::shift(),
            "abcdefghijklmnopqr❱stuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqr❰stuvwxyz",
        );

        test(
            "abcdefghijklmnopqr█stuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Left, InputKey::Down, InputKey::Down],
            InputModifiers::shift(),
            "abcdefghijklmnopqr❱stuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopq❰rstuvwxyz",
        );

        test(
            "abcdefghijklmnopqr█stuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Right, InputKey::Down, InputKey::Down],
            InputModifiers::shift(),
            "abcdefghijklmnopqr❱stuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrs❰tuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz█\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Down, InputKey::Down],
            InputModifiers::shift(),
            "abcdefghijklmnopqrstuvwxyz❱\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxyz❰",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz█\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxy",
            &[InputKey::Down, InputKey::Down],
            InputModifiers::shift(),
            "abcdefghijklmnopqrstuvwxyz❱\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxy❰",
        );

        test(
            "█abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::End, InputKey::Down],
            InputModifiers::shift(),
            "❱abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijkl❰\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz█\n\
            abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Home, InputKey::Down],
            InputModifiers::shift(),
            "abcdefghijklmnopqrstuvwxyz❱\n\
            ❰abcdefghijkl\n\
            abcdefghijklmnopqrstuvwxyz",
        );
    }

    #[test]
    fn test_home_btn_selection() {
        test(
            "abcdefghijklmnopqrstuvwxyz█",
            &[InputKey::Home],
            InputModifiers::shift(),
            "❰abcdefghijklmnopqrstuvwxyz❱",
        );

        test(
            "abcdefghijklmnop█qrstuvwxyz",
            &[InputKey::Home],
            InputModifiers::shift(),
            "❰abcdefghijklmnop❱qrstuvwxyz",
        );

        test(
            "█abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Home],
            InputModifiers::shift(),
            "█abcdefghijklmnopqrstuvwxyz",
        );
    }

    #[test]
    fn test_end_btn_selection() {
        test(
            "█abcdefghijklmnopqrstuvwxyz",
            &[InputKey::End],
            InputModifiers::shift(),
            "❱abcdefghijklmnopqrstuvwxyz❰",
        );

        test(
            "█abcdefghijklmnopqrstuvwxyz",
            &[InputKey::End],
            InputModifiers::shift(),
            "❱abcdefghijklmnopqrstuvwxyz❰",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz█",
            &[InputKey::End],
            InputModifiers::shift(),
            "abcdefghijklmnopqrstuvwxyz█",
        );
    }

    #[test]
    fn test_home_end_btn_selection() {
        test(
            "abcdefghijklmnopqrstuvwxyz█",
            &[InputKey::Home, InputKey::End],
            InputModifiers::shift(),
            "abcdefghijklmnopqrstuvwxyz█",
        );

        test(
            "abcdefghijklmno█pqrstuvwxyz",
            &[InputKey::Home, InputKey::End],
            InputModifiers::shift(),
            "abcdefghijklmno❱pqrstuvwxyz❰",
        );
    }

    #[test]
    fn test_ctrl_shift_left() {
        test(
            "abcdefghijklmnopqrstuvwxyz█",
            &[InputKey::Left],
            InputModifiers::ctrl_shift(),
            "❰abcdefghijklmnopqrstuvwxyz❱",
        );

        test(
            "abcdefghijkl mnopqrstuvwxyz█",
            &[InputKey::Left],
            InputModifiers::ctrl_shift(),
            "abcdefghijkl ❰mnopqrstuvwxyz❱",
        );

        test(
            "abcdefghijkl █mnopqrstuvwxyz",
            &[InputKey::Left],
            InputModifiers::ctrl_shift(),
            "❰abcdefghijkl ❱mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█ mnopqrstuvwxyz",
            &[InputKey::Left],
            InputModifiers::ctrl_shift(),
            "❰abcdefghijkl❱ mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl    █mnopqrstuvwxyz",
            &[InputKey::Left],
            InputModifiers::ctrl_shift(),
            "❰abcdefghijkl    ❱mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl  )  █mnopqrstuvwxyz",
            &[InputKey::Left],
            InputModifiers::ctrl_shift(),
            "abcdefghijkl  ❰)  ❱mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl  |()-+%'^%/=?{}#<>&@[]*  █mnopqrstuvwxyz",
            &[InputKey::Left],
            InputModifiers::ctrl_shift(),
            "abcdefghijkl  ❰|()-+%'^%/=?{}#<>&@[]*  ❱mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl  \"  █mnopqrstuvwxyz",
            &[InputKey::Left],
            InputModifiers::ctrl_shift(),
            "abcdefghijkl  ❰\"  ❱mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl  12  █mnopqrstuvwxyz",
            &[InputKey::Left],
            InputModifiers::ctrl_shift(),
            "abcdefghijkl  ❰12  ❱mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl  12a  █mnopqrstuvwxyz",
            &[InputKey::Left],
            InputModifiers::ctrl_shift(),
            "abcdefghijkl  ❰12a  ❱mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl  a12  █mnopqrstuvwxyz",
            &[InputKey::Left],
            InputModifiers::ctrl_shift(),
            "abcdefghijkl  ❰a12  ❱mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl  _  █mnopqrstuvwxyz",
            &[InputKey::Left],
            InputModifiers::ctrl_shift(),
            "abcdefghijkl  ❰_  ❱mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl  _1a  █mnopqrstuvwxyz",
            &[InputKey::Left],
            InputModifiers::ctrl_shift(),
            "abcdefghijkl  ❰_1a  ❱mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl  \"❤(  █mnopqrstuvwxyz",
            &[InputKey::Left],
            InputModifiers::ctrl_shift(),
            "abcdefghijkl  \"❰❤(  ❱mnopqrstuvwxyz",
        );
    }

    #[test]
    fn test_ctrl_shift_right() {
        test(
            "█abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl_shift(),
            "❱abcdefghijklmnopqrstuvwxyz❰",
        );

        test(
            "█abcdefghijkl mnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl_shift(),
            "❱abcdefghijkl❰ mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█ mnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl_shift(),
            "abcdefghijkl❱ mnopqrstuvwxyz❰",
        );

        test(
            "abcdefghijkl █mnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl_shift(),
            "abcdefghijkl ❱mnopqrstuvwxyz❰",
        );

        test(
            "abcdefghijkl█    mnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl_shift(),
            "abcdefghijkl❱    mnopqrstuvwxyz❰",
        );

        test(
            "abcdefghijkl█  )  mnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl_shift(),
            "abcdefghijkl❱  )❰  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█  |()-+%'^%/=?{}#<>&@[]*  mnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl_shift(),
            "abcdefghijkl❱  |()-+%'^%/=?{}#<>&@[]*❰  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█  \"  mnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl_shift(),
            "abcdefghijkl❱  \"❰  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█  12  mnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl_shift(),
            "abcdefghijkl❱  12❰  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█  12a  mnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl_shift(),
            "abcdefghijkl❱  12a❰  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█  a12  mnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl_shift(),
            "abcdefghijkl❱  a12❰  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█  _  mnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl_shift(),
            "abcdefghijkl❱  _❰  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█  _1a  mnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl_shift(),
            "abcdefghijkl❱  _1a❰  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█  \"❤(  mnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::ctrl_shift(),
            "abcdefghijkl❱  \"❰❤(  mnopqrstuvwxyz",
        );
    }

    #[test]
    fn test_movement_cancels_selection() {
        test(
            "abcdef❱ghijklmnopqrstuvwxyz\n\
            abcdefghijkl❰mnopqrstuvwxyz",
            &[InputKey::Left],
            InputModifiers::none(),
            "abcdef█ghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdef❱ghijklmnopqrstuvwxyz\n\
            abcdefghijkl❰mnopqrstuvwxyz",
            &[InputKey::Right],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijkl█mnopqrstuvwxyz",
        );

        test(
            "abcdef❱ghijklmnopqrstuvwxyz\n\
            abcdefghijkl❰mnopqrstuvwxyz",
            &[InputKey::Down],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz█",
        );

        test(
            "abcdef❱ghijklmnopqrstuvwxyz\n\
            abcdefghijkl❰mnopqrstuvwxyz",
            &[InputKey::Up],
            InputModifiers::none(),
            "abcdefghijkl█mnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdef❱ghijklmnopqrstuvwxyz\n\
            abcdefghijkl❰mnopqrstuvwxyz",
            &[InputKey::Home],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            █abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdef❱ghijklmnopqrstuvwxyz\n\
            abcdefghijkl❰mnopqrstuvwxyz",
            &[InputKey::End],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz█",
        );

        test(
            "abcd❰efghijklmnopqrstuvwxyz\n\
            abcdefghijkl❱mnopqrstuvwxyz",
            &[InputKey::Home],
            InputModifiers::none(),
            "█abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcd❰efghijklmnopqrstuvwxyz\n\
            abcdefghijkl❱mnopqrstuvwxyz",
            &[InputKey::End],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz█\n\
            abcdefghijklmnopqrstuvwxyz",
        );
    }

    /// //////////////////////////////////////
    /// Edit
    /// //////////////////////////////////////

    #[test]
    fn test_insert_char() {
        test(
            "█abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Char('1')],
            InputModifiers::none(),
            "1█abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdef█ghijklmnopqrstuvwxyz",
            &[InputKey::Char('1')],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdef1█ghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz█\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Char('1')],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz1█\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz█",
            &[InputKey::Char('1')],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz1█",
        );

        test(
            "█abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[
                InputKey::Char('1'),
                InputKey::Char('❤'),
                InputKey::Char('3'),
            ],
            InputModifiers::none(),
            "1❤3█abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        // line is full, no insertion is allowed
        let text_80_len =
            "█abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzab\n\
            abcdefghijklmnopqrstuvwxyz";
        test(
            text_80_len,
            &[
                InputKey::Char('1'),
                InputKey::Char('❤'),
                InputKey::Char('3'),
            ],
            InputModifiers::none(),
            text_80_len,
        );
    }

    #[test]
    fn insert_char_with_selection() {
        test(
            "abcd❰efghijk❱lmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Char('X')],
            InputModifiers::none(),
            "abcdX█lmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcd❰efghijklmnopqrstuvwxyz\n\
            abcdefghijkl❱mnopqrstuvwxyz",
            &[InputKey::Char('X')],
            InputModifiers::none(),
            "abcdX█mnopqrstuvwxyz",
        );

        test(
            "❰abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz❱",
            &[InputKey::Char('X')],
            InputModifiers::none(),
            "X█",
        );

        test(
            "ab❰c❱defghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Char('X')],
            InputModifiers::none(),
            "abX█defghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcd❰efghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijkl❱mnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Char('X')],
            InputModifiers::none(),
            "abcdX█mnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
        );
    }

    #[test]
    fn test_backspace() {
        test("a█", &[InputKey::Backspace], InputModifiers::none(), "█");

        test(
            "█abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Backspace],
            InputModifiers::none(),
            "█abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdef█ghijklmnopqrstuvwxyz",
            &[InputKey::Backspace],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            abcde█ghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz█\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Backspace],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxy█\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz█",
            &[InputKey::Backspace],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxy█",
        );

        test(
            "abcde█fghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[
                InputKey::Backspace,
                InputKey::Backspace,
                InputKey::Backspace,
            ],
            InputModifiers::none(),
            "ab█fghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "█",
            &[
                InputKey::Backspace,
                InputKey::Backspace,
                InputKey::Backspace,
            ],
            InputModifiers::none(),
            "█",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            █abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Backspace],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz█abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrst█uvwxyz",
            &[
                InputKey::Home,
                InputKey::Backspace,
                InputKey::Home,
                InputKey::Backspace,
            ],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz█abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz",
        );

        // the last backspace is not allowed, there is no enough space for it
        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrst█uvwxyz",
            &[
                InputKey::Home,
                InputKey::Backspace,
                InputKey::Home,
                InputKey::Backspace,
                InputKey::Home,
                InputKey::Backspace,
            ],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            █abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz",
        );
    }

    #[test]
    fn test_ctrl_del() {
        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz█",
            &[InputKey::Del],
            InputModifiers::ctrl(),
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz█",
        );

        test(
            "abcde█fghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Del, InputKey::Del, InputKey::Del],
            InputModifiers::ctrl(),
            "abcde█",
        );

        test(
            "█",
            &[InputKey::Del, InputKey::Del, InputKey::Del],
            InputModifiers::ctrl(),
            "█",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz█\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Del],
            InputModifiers::ctrl(),
            "abcdefghijklmnopqrstuvwxyz█abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnop█qrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::End, InputKey::Del, InputKey::End, InputKey::Del],
            InputModifiers::ctrl(),
            "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz█abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "█abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Del],
            InputModifiers::ctrl(),
            "█",
        );

        test(
            "█abcdefghijkl mnopqrstuvwxyz",
            &[InputKey::Del],
            InputModifiers::ctrl(),
            "█ mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█ mnopqrstuvwxyz",
            &[InputKey::Del],
            InputModifiers::ctrl(),
            "abcdefghijkl█mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl █mnopqrstuvwxyz",
            &[InputKey::Del],
            InputModifiers::ctrl(),
            "abcdefghijkl █",
        );

        test(
            "abcdefghijkl█    mnopqrstuvwxyz",
            &[InputKey::Del],
            InputModifiers::ctrl(),
            "abcdefghijkl█mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█  )  mnopqrstuvwxyz",
            &[InputKey::Del],
            InputModifiers::ctrl(),
            "abcdefghijkl█)  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█  |()-+%'^%/=?{}#<>&@[]*  mnopqrstuvwxyz",
            &[InputKey::Del],
            InputModifiers::ctrl(),
            "abcdefghijkl█|()-+%'^%/=?{}#<>&@[]*  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█  \"  mnopqrstuvwxyz",
            &[InputKey::Del],
            InputModifiers::ctrl(),
            "abcdefghijkl█\"  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█  12  mnopqrstuvwxyz",
            &[InputKey::Del],
            InputModifiers::ctrl(),
            "abcdefghijkl█12  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█  12a  mnopqrstuvwxyz",
            &[InputKey::Del],
            InputModifiers::ctrl(),
            "abcdefghijkl█12a  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█  a12  mnopqrstuvwxyz",
            &[InputKey::Del],
            InputModifiers::ctrl(),
            "abcdefghijkl█a12  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█  _  mnopqrstuvwxyz",
            &[InputKey::Del],
            InputModifiers::ctrl(),
            "abcdefghijkl█_  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█  _1a  mnopqrstuvwxyz",
            &[InputKey::Del],
            InputModifiers::ctrl(),
            "abcdefghijkl█_1a  mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█  \"❤(  mnopqrstuvwxyz",
            &[InputKey::Del],
            InputModifiers::ctrl(),
            "abcdefghijkl█\"❤(  mnopqrstuvwxyz",
        );
    }

    #[test]
    fn test_ctrl_w() {
        test("█", &[InputKey::Char('w')], InputModifiers::ctrl(), "█");
        test("a█", &[InputKey::Char('w')], InputModifiers::ctrl(), "❱a❰");
        test("█a", &[InputKey::Char('w')], InputModifiers::ctrl(), "❱a❰");

        test(
            "█asd",
            &[InputKey::Char('w')],
            InputModifiers::ctrl(),
            "❱asd❰",
        );
        test(
            "asd█",
            &[InputKey::Char('w')],
            InputModifiers::ctrl(),
            "❱asd❰",
        );
        test(
            "a█sd",
            &[InputKey::Char('w')],
            InputModifiers::ctrl(),
            "❱asd❰",
        );
        test(
            "as█d",
            &[InputKey::Char('w')],
            InputModifiers::ctrl(),
            "❱asd❰",
        );

        test(
            "as█d 12",
            &[InputKey::Char('w')],
            InputModifiers::ctrl(),
            "❱asd❰ 12",
        );
        test(
            "asd █12",
            &[InputKey::Char('w')],
            InputModifiers::ctrl(),
            "asd ❱12❰",
        );
        test(
            "asd 1█2",
            &[InputKey::Char('w')],
            InputModifiers::ctrl(),
            "asd ❱12❰",
        );
        test(
            "asd 12█",
            &[InputKey::Char('w')],
            InputModifiers::ctrl(),
            "asd ❱12❰",
        );

        test(
            "█asdasdasd\n\
            bbbbbbbbbbb",
            &[InputKey::Char('w')],
            InputModifiers::ctrl(),
            "❱asdasdasd❰\n\
            bbbbbbbbbbb",
        );

        test(
            "asd 12█",
            &[InputKey::Char('w'), InputKey::Char('w')],
            InputModifiers::ctrl(),
            "❱asd 12❰",
        );

        test(
            "█asd 12",
            &[InputKey::Char('w'), InputKey::Char('w')],
            InputModifiers::ctrl(),
            "❱asd 12❰",
        );

        test(
            "asd █12 qwe",
            &[InputKey::Char('w'), InputKey::Char('w')],
            InputModifiers::ctrl(),
            "❱asd 12 qwe❰",
        );

        test(
            "vvv asd █12 qwe ttt",
            &[InputKey::Char('w'), InputKey::Char('w')],
            InputModifiers::ctrl(),
            "vvv ❱asd 12 qwe❰ ttt",
        );

        test(
            "vvv ❱asd 12 qwe❱ ttt",
            &[InputKey::Char('w')],
            InputModifiers::ctrl(),
            "❱vvv asd 12 qwe ttt❰",
        );

        test(
            "vvv asd █12 qwe ttt",
            &[
                InputKey::Char('w'),
                InputKey::Char('w'),
                InputKey::Char('w'),
            ],
            InputModifiers::ctrl(),
            "❱vvv asd 12 qwe ttt❰",
        );

        // asd
        // test(
        //     "(1+█2)*2 / 4",
        //     &[InputKey::Char('w'), InputKey::Char('w'), InputKey::Char('w')],
        //     InputModifiers::ctrl(),
        //     "❱vvv asd 12 qwe ttt❰",
        // );
    }

    #[test]
    fn test_ctrl_backspace() {
        test("a█", &[InputKey::Backspace], InputModifiers::ctrl(), "█");

        test(
            "█abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Backspace],
            InputModifiers::ctrl(),
            "█abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdef█ghijklmnopqrstuvwxyz",
            &[InputKey::Backspace],
            InputModifiers::ctrl(),
            "abcdefghijklmnopqrstuvwxyz\n\
            █ghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz█\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Backspace],
            InputModifiers::ctrl(),
            "█\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz█",
            &[InputKey::Backspace],
            InputModifiers::ctrl(),
            "abcdefghijklmnopqrstuvwxyz\n\
            █",
        );

        test(
            "abcde█fghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[
                InputKey::Backspace,
                InputKey::Backspace,
                InputKey::Backspace,
            ],
            InputModifiers::ctrl(),
            "█fghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "█",
            &[
                InputKey::Backspace,
                InputKey::Backspace,
                InputKey::Backspace,
            ],
            InputModifiers::ctrl(),
            "█",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            █abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Backspace],
            InputModifiers::ctrl(),
            "abcdefghijklmnopqrstuvwxyz█abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrst█uvwxyz",
            &[
                InputKey::Home,
                InputKey::Backspace,
                InputKey::Home,
                InputKey::Backspace,
            ],
            InputModifiers::ctrl(),
            "abcdefghijklmnopqrstuvwxyz█abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz█",
            &[InputKey::Backspace],
            InputModifiers::ctrl(),
            "█",
        );

        test(
            "abcdefghijkl mnopqrstuvwxyz█",
            &[InputKey::Backspace],
            InputModifiers::ctrl(),
            "abcdefghijkl █",
        );

        test(
            "abcdefghijkl █mnopqrstuvwxyz",
            &[InputKey::Backspace],
            InputModifiers::ctrl(),
            "█mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl█ mnopqrstuvwxyz",
            &[InputKey::Backspace],
            InputModifiers::ctrl(),
            "█ mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl    █mnopqrstuvwxyz",
            &[InputKey::Backspace],
            InputModifiers::ctrl(),
            "█mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl  )  █mnopqrstuvwxyz",
            &[InputKey::Backspace],
            InputModifiers::ctrl(),
            "abcdefghijkl  █mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl  |()-+%'^%/=?{}#<>&@[]*  █mnopqrstuvwxyz",
            &[InputKey::Backspace],
            InputModifiers::ctrl(),
            "abcdefghijkl  █mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl  \"  █mnopqrstuvwxyz",
            &[InputKey::Backspace],
            InputModifiers::ctrl(),
            "abcdefghijkl  █mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl  12  █mnopqrstuvwxyz",
            &[InputKey::Backspace],
            InputModifiers::ctrl(),
            "abcdefghijkl  █mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl  12a  █mnopqrstuvwxyz",
            &[InputKey::Backspace],
            InputModifiers::ctrl(),
            "abcdefghijkl  █mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl  a12  █mnopqrstuvwxyz",
            &[InputKey::Backspace],
            InputModifiers::ctrl(),
            "abcdefghijkl  █mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl  _  █mnopqrstuvwxyz",
            &[InputKey::Backspace],
            InputModifiers::ctrl(),
            "abcdefghijkl  █mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl  _1a  █mnopqrstuvwxyz",
            &[InputKey::Backspace],
            InputModifiers::ctrl(),
            "abcdefghijkl  █mnopqrstuvwxyz",
        );

        test(
            "abcdefghijkl  \"❤(  █mnopqrstuvwxyz",
            &[InputKey::Backspace],
            InputModifiers::ctrl(),
            "abcdefghijkl  \"█mnopqrstuvwxyz",
        );
    }

    #[test]
    fn press_backspace_with_selection() {
        test(
            "abcd❰efghijk❱lmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Backspace],
            InputModifiers::none(),
            "abcd█lmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcd❰efghijklmnopqrstuvwxyz\n\
            abcdefghijkl❱mnopqrstuvwxyz",
            &[InputKey::Backspace],
            InputModifiers::none(),
            "abcd█mnopqrstuvwxyz",
        );

        test(
            "❰abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz❱",
            &[InputKey::Backspace],
            InputModifiers::none(),
            "█",
        );

        test(
            "ab❰c❱defghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Backspace],
            InputModifiers::none(),
            "ab█defghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcd❰efghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijkl❱mnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Backspace],
            InputModifiers::none(),
            "abcd█mnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
        );
    }

    #[test]
    fn test_del() {
        test(
            "█abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Del],
            InputModifiers::none(),
            "█bcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdef█ghijklmnopqrstuvwxyz",
            &[InputKey::Del],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdef█hijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz█",
            &[InputKey::Del],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz█",
        );

        test(
            "abcde█fghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Del, InputKey::Del, InputKey::Del],
            InputModifiers::none(),
            "abcde█ijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "█",
            &[InputKey::Del, InputKey::Del, InputKey::Del],
            InputModifiers::none(),
            "█",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz█\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Del],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz█abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnop█qrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::End, InputKey::Del, InputKey::End, InputKey::Del],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz█abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnop█qrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[
                InputKey::End,
                InputKey::Del,
                InputKey::End,
                InputKey::Del,
                InputKey::End,
                InputKey::Del,
            ],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz█\n\
            abcdefghijklmnopqrstuvwxyz",
        );
    }

    #[test]
    fn press_del_with_selection() {
        test(
            "abcd❰efghijk❱lmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Del],
            InputModifiers::none(),
            "abcd█lmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcd❰efghijklmnopqrstuvwxyz\n\
            abcdefghijkl❱mnopqrstuvwxyz",
            &[InputKey::Del],
            InputModifiers::none(),
            "abcd█mnopqrstuvwxyz",
        );

        test(
            "❰abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz❱",
            &[InputKey::Del],
            InputModifiers::none(),
            "█",
        );

        test(
            "ab❰c❱defghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Del],
            InputModifiers::none(),
            "ab█defghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcd❰efghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijkl❱mnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Del],
            InputModifiers::none(),
            "abcd█mnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
        );
    }

    #[test]
    fn test_enter() {
        test(
            "█abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Enter],
            InputModifiers::none(),
            "\n\
            █abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdef█ghijklmnopqrstuvwxyz",
            &[InputKey::Enter],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdef\n\
            █ghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz█\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Enter],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            █\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz█",
            &[InputKey::Enter],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz\n\
            █",
        );

        test(
            "abcde█fghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Enter, InputKey::Enter, InputKey::Enter],
            InputModifiers::none(),
            "abcde\n\
            \n\
            \n\
            █fghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "█",
            &[InputKey::Enter, InputKey::Enter, InputKey::Enter],
            InputModifiers::none(),
            "\n\
            \n\
            \n\
            █",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            █abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Enter],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            \n\
            █abcdefghijklmnopqrstuvwxyz",
        );
    }

    #[test]
    fn press_enter_with_selection() {
        test(
            "abcd❰efghijk❱lmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Enter],
            InputModifiers::none(),
            "abcd\n\
            █lmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcd❰efghijklmnopqrstuvwxyz\n\
            abcdefghijkl❱mnopqrstuvwxyz",
            &[InputKey::Enter],
            InputModifiers::none(),
            "abcd\n\
            █mnopqrstuvwxyz",
        );

        test(
            "❰abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz❱",
            &[InputKey::Enter],
            InputModifiers::none(),
            "\n\
            █",
        );

        test(
            "ab❰c❱defghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Enter],
            InputModifiers::none(),
            "ab\n\
            █defghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcd❰efghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijkl❱mnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Enter],
            InputModifiers::none(),
            "abcd\n\
            █mnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
        );
    }

    #[test]
    fn test_insert_text() {
        test(
            "█abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Text("long text")],
            InputModifiers::none(),
            "long text█abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdef█ghijklmnopqrstuvwxyz",
            &[InputKey::Text("long text")],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdeflong text█ghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz█\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Text("long text")],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyzlong text█\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz█",
            &[InputKey::Text("long text")],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyzlong text█",
        );

        test(
            "█abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Text("long text ❤")],
            InputModifiers::none(),
            "long text ❤█abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        // on insertion, characters are moved to the next line if exceeds line limit
        test(
            "█abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzab\n\
            abcdefghijklmnopqrstuvwxyz",
            &[InputKey::Text("long text ❤")],
            InputModifiers::none(),
            "long text ❤█abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopq\n\
            rstuvwxyzab\n\
            abcdefghijklmnopqrstuvwxyz",
        );

        test(
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijk█lmnopqrstuvwxyz",
            &[InputKey::Text("long text ❤\nwith 3\nlines")],
            InputModifiers::none(),
            "abcdefghijklmnopqrstuvwxyz\n\
            abcdefghijklong text ❤\n\
            with 3\n\
            lines█lmnopqrstuvwxyz",
        );
    }

    #[test]
    fn test_bug1() {
        test(
            "aaaaa❱12s aa\n\
            a\n\
            a\n\
            a\n\
            a❰",
            &[InputKey::Del],
            InputModifiers::none(),
            "aaaaa█",
        );

        test(
            "((0b00101 AND 0xFF) XOR 0xFF00) << 16 >> 16  ❱NOT(0xFF)❰",
            &[InputKey::Del],
            InputModifiers::none(),
            "((0b00101 AND 0xFF) XOR 0xFF00) << 16 >> 16  █",
        );
    }

    #[test]
    fn test_copy() {
        let mut editor = Editor::new(80);
        let lines = test0(
            &mut editor,
            "aaaaa❱12s aa\n\
            a\n\
            a\n\
            a\n\
            a❰",
            &[],
            InputModifiers::none(),
            "aaaaa❱12s aa\n\
            a\n\
            a\n\
            a\n\
            a❰",
        );
        assert_eq!(
            editor.get_selected_text(),
            Some("12s aa\na\na\na\na".to_owned())
        )
    }
}
