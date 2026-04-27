use ratatui::text::Text;

pub struct Input {
    text: String,
    cursor: usize, // byte position
}

impl Input {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    pub fn move_cursor_left(&mut self) {
        let before = &self.text[..self.cursor];
        if let Some((idx, _)) = before.char_indices().last() {
            self.cursor = idx;
        }
    }

    pub fn move_cursor_right(&mut self) {
        let after = &self.text[self.cursor..];
        if let Some((idx, c)) = after.char_indices().next() {
            self.cursor += idx + c.len_utf8();
        }
    }

    pub fn delete_char(&mut self) {
        let before = &self.text[..self.cursor];
        if let Some((idx, c)) = before.char_indices().last() {
            self.text.remove(idx);
            self.cursor -= c.len_utf8();
        }
    }

    pub fn enter_char(&mut self, c: char) {
        self.text.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    pub fn enter_new_line(&mut self) {
        self.text.insert(self.cursor, '\n');
        self.cursor += 1;
    }

    /// 返回纯文本，不带任何光标样式
    pub fn render(&self) -> Text<'static> {
        Text::from(self.text.clone())
    }

    /// 计算光标在文本中的 (列, 行) 位置（按字符计，不考虑换行折行）
    ///
    /// 返回值是相对于文本区域内部的坐标，需要加上边框偏移才是终端绝对坐标。
    pub fn cursor_position(&self) -> (u16, u16) {
        let text_before = &self.text[..self.cursor];
        let line = text_before.chars().filter(|&c| c == '\n').count() as u16;
        let line_start = text_before.rfind('\n').map(|n| n + 1).unwrap_or(0);
        let col = self.text[line_start..self.cursor].chars().count() as u16;
        (col, line)
    }
}
