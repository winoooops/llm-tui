use crate::message::Message;
use crate::utils;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::action::Action;

pub struct Chat {
    command_tx: Option<UnboundedSender<Action>>,
    messages: Vec<String>,       // the message history user sees
    conversation: Vec<Message>,  // the message history llm api sees
    current_ai_response: String, // the temporary llm response text, will be removed
    input: String,
    focused: bool,
    waiting_for_response: bool,
    tick_count: u8,
    cursor_position: usize, // the cursor position
}

impl Chat {
    pub fn new() -> Self {
        Self {
            command_tx: None,
            messages: Vec::new(),
            conversation: Vec::new(),
            current_ai_response: String::new(),
            input: String::new(),
            focused: true,
            waiting_for_response: false,
            tick_count: 0,
            cursor_position: 0,
        }
    }

    fn start_waiting(&mut self) {
        self.waiting_for_response = true;
    }

    fn stop_waiting(&mut self) {
        self.waiting_for_response = false;
    }

    fn is_waiting(&self) -> bool {
        self.waiting_for_response
    }

    fn append_ai_text(&mut self, text: &str) {
        if let Some(last) = self.messages.last_mut()
            && last.starts_with("AI: ")
        {
            last.push_str(text);
            return;
        }
        self.messages.push(format!("AI: {}", text));
    }

    fn move_cursor_left(&mut self) {
        let before = &self.input[..self.cursor_position];
        if let Some((idx, _)) = before.char_indices().last() {
            self.cursor_position = idx;
        }
    }

    fn move_cursor_right(&mut self) {
        let after = &self.input[self.cursor_position..];
        if let Some((idx, c)) = after.char_indices().next() {
            // although idx is alwasy 0,
            // it's better to understand if we do it like idx + c.len_utf8()
            // which means the cursor will jump the end of next character(utf-8 or not)
            self.cursor_position += idx + c.len_utf8();
        }
    }

    fn enter_char(&mut self, c: char) {
        // insert the character
        self.input.insert(self.cursor_position, c);
        // move the cursor to the right by one length of the character
        self.cursor_position += c.len_utf8();
    }

    fn delete_char(&mut self) {
        let before = &self.input[..self.cursor_position];
        if let Some((idx, c)) = before.char_indices().last() {
            // remove the character
            self.input.remove(idx);
            // move the cursor to the right by one character length
            self.cursor_position -= c.len_utf8();
        }
    }

    fn enter_newline(&mut self) {
        self.input.insert(self.cursor_position, '\n');
        self.cursor_position += 1;
    }

    fn build_input_text(&self) -> Text<'static> {
        if !self.focused {
            return Text::from(self.input.clone());
        }

        let cursor_style = Style::default().bg(Color::Yellow).fg(Color::Black);
        let block_style = Style::default().fg(Color::Yellow);

        // 1. 计算光标在第几行、第几列（按字符计）
        let text_before = &self.input[..self.cursor_position];
        let cursor_line = text_before.chars().filter(|&c| c == '\n').count();
        let line_start = text_before.rfind('\n').map(|n| n + 1).unwrap_or(0);
        let cursor_col = self.input[line_start..self.cursor_position].chars().count();

        // 2. split into multiple line by '\n'
        let raw_lines: Vec<&str> = self.input.split('\n').collect();
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
        if self.input.is_empty()
            || (self.input.ends_with('\n') && cursor_line >= raw_lines.len().saturating_sub(1))
        {
            lines.push(Line::from(Span::styled("▋", block_style)));
        }

        Text::from(lines)
    }
}

impl Component for Chat {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> color_eyre::Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        match key.code {
            KeyCode::Enter => {
                if !self.input.is_empty() {
                    let text = self.input.clone();

                    self.messages.push(format!("You: {}", text));
                    self.conversation.push(Message::user(&text));
                    self.input.clear();
                    self.cursor_position = 0; // reset the position
                    self.start_waiting();

                    if let Some(ref tx) = self.command_tx {
                        let _ = tx.send(Action::SendMessage(self.conversation.clone()));
                    }
                }
                Ok(None)
            }
            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.enter_newline();
                Ok(None)
            }
            KeyCode::Left => {
                self.move_cursor_left();
                Ok(None)
            }
            KeyCode::Right => {
                self.move_cursor_right();
                Ok(None)
            }
            KeyCode::Backspace => {
                self.delete_char();
                Ok(None)
            }
            KeyCode::Char(c) => {
                self.enter_char(c);
                Ok(None)
            }
            KeyCode::Esc => {
                if let Some(ref tx) = self.command_tx {
                    let _ = tx.send(Action::Quit);
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        match action {
            Action::Tick => {
                self.tick_count = self.tick_count.wrapping_add(1);
            }
            Action::ReceiveChunk(chunk) => {
                self.stop_waiting();
                self.current_ai_response.push_str(&chunk);
                self.append_ai_text(&chunk);
            }
            Action::StreamEnd if !self.current_ai_response.is_empty() => {
                self.conversation
                    .push(Message::assistant(&self.current_ai_response));
                self.current_ai_response.clear();
            }
            _ => {}
        }

        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        // 1. divied the area into two parts
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
            .split(area);

        let messages_area = chunks[0];
        let input_area = chunks[1];

        // 2. construct the message area controller
        let mut lines: Vec<Line> = self
            .messages
            .iter()
            .map(|m| Line::from(m.as_str()))
            .collect();

        if self.is_waiting() {
            lines.push(Line::from(format!(
                "AI: {} thinking",
                utils::spinner_frame(self.tick_count as usize)
            )));
        }

        let messages_widget = Paragraph::new(Text::from(lines))
            .block(Block::default().title("Chat").borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        frame.render_widget(messages_widget, messages_area);

        // 3. constrcut the input area controller
        let input_widget = Paragraph::new(self.build_input_text())
            .block(
                Block::default()
                    .title("Input (Enter=Send, Ctrl+J=newline, Esc=quit)")
                    .borders(Borders::ALL)
                    .border_style(if self.focused {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    }),
            )
            .wrap(Wrap { trim: true });

        // 4. render the widget to the view
        frame.render_widget(input_widget, input_area);

        Ok(())
    }
}
