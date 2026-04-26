use crate::utils;
use crate::message::Message;
use crossterm::event::{KeyCode, KeyEvent};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::action::Action;

pub struct Chat {
    command_tx: Option<UnboundedSender<Action>>,
    messages: Vec<String>,                       // the message history user sees
    conversation: Vec<Message>,                  // the message history llm api sees
    current_ai_response: String,                 // the temporary llm response text, will be removed
    input: String,
    focused: bool,
    waiting_for_response: bool,
    tick_count: u8,
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
                    // 1. show in the chat history ui
                    self.messages.push(format!("You: {}", text));

                    // 2. save to conversation
                    self.conversation.push(Message::user(&text));

                    // 3. do the cleanup
                    self.input.clear();
                    self.start_waiting();

                    if let Some(ref tx) = self.command_tx {
                        let _ = tx.send(Action::SendMessage(self.conversation.clone()));
                    }
                }
                Ok(None)
            }
            KeyCode::Backspace => {
                self.input.pop();
                Ok(None)
            }
            KeyCode::Char(c) => {
                self.input.push(c);
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
            Action::StreamEnd => {
                if !self.current_ai_response.is_empty() {
                    self.conversation.push(Message::assistant(&self.current_ai_response));
                    self.current_ai_response.clear();
                }
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
        let input_widget = Paragraph::new(self.input.as_str())
            .block(
                Block::default()
                    .title("Input (Enter to send, Esc to quit)")
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
