use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
};

pub struct Input {
    text: String,
    cursor: usize, // cursor position
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

        if let Some((idx, char)) = after.char_indices().next() {
            self.cursor = idx + char.len_utf8();
        }
    }

    pub fn delete_char(&mut self) {
        let before = &self.text[..self.cursor];

        if let Some((idx, char)) = before.char_indices().last() {
            self.text.remove(idx);
            self.cursor -= char.len_utf8();
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

    pub fn render(&self, focused: bool) -> Text<'static> {
        if !focused {
            return Text::from(self.text.clone());
        }

        let cursor_style = Style::default().bg(Color::Yellow).fg(Color::Black);
        let block_style = Style::default().fg(Color::Yellow);

        // 1. 计算光标在第几行、第几列（按字符计）
        let text_before = &self.text[..self.cursor];
        let cursor_line = text_before.chars().filter(|&c| c == '\n').count();
        let line_start = text_before.rfind('\n').map(|n| n + 1).unwrap_or(0);
        let cursor_col = self.text[line_start..self.cursor].chars().count();

        // 2. split into multiple line by '\n'
        let raw_lines: Vec<&str> = self.text.split('\n').collect();
        let mut lines = Vec::new();

        for (i, line) in raw_lines.iter().enumerate() {
            if i == cursor_line {
                let chars: Vec<char> = line.chars().collect();
                if cursor_col < chars.len() {
                    // 光标在某个字符上：把它高亮
                    let before: String = chars[..cursor_col].iter().collect();
                    let c = chars[cursor_col];
                    let after: String = chars[cursor_col + 1..].iter().collect();
                    lines.push(Line::from(vec![
                        Span::raw(before),
                        Span::styled(c.to_string(), cursor_style),
                        Span::raw(after),
                    ]));
                } else {
                    // 光标在行尾：追加一个闪烁块
                    lines.push(Line::from(vec![
                        Span::raw(line.to_string()),
                        Span::styled("▋", block_style),
                    ]));
                }
            } else {
                lines.push(Line::from(line.to_string()));
            }
        }

        // 3. 处理光标在末尾空行的情况（比如刚按了 Ctrl+J）
        if self.text.is_empty()
            || (self.text.ends_with('\n') && cursor_line >= raw_lines.len().saturating_sub(1))
        {
            lines.push(Line::from(Span::styled("▋", block_style)));
        }

        Text::from(lines)
    }
}
