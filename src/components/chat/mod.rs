use color_eyre::eyre::Ok;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Position},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    components::{
        Component,
        chat::{conversation::Conversation, input::Input},
    }, message::Message, prompt::PromptContext,
};

pub mod conversation;
pub mod input;

pub struct Chat {
    command_tx: Option<UnboundedSender<Action>>,
    system_prompt: Message,
    conversation: Conversation,
    input: Input,
    focused: bool,
}

impl Chat {
    pub fn new() -> Self {
        let system_prompt = PromptContext::from_environment().system_prompt();
        tracing::info!("system prompt loaded: {}", system_prompt.content);

        Self {
            command_tx: None,
            system_prompt,
            conversation: Conversation::new(),
            input: Input::new(),
            focused: true,
        }
    }
}

impl Component for Chat {
    fn register_action_handler(
        &mut self,
        tx: tokio::sync::mpsc::UnboundedSender<Action>,
    ) -> color_eyre::Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        match key.code {
            KeyCode::Enter if key.modifiers.is_empty() => {
                if !self.input.is_empty() {
                    self.conversation.push_user(self.input.text());
                    self.input.clear();
                    self.conversation.start_response();

                    if let Some(ref tx) = self.command_tx {
                        let _ = tx.send(Action::SendMessage(
                            self.system_prompt.clone(),
                            self.conversation.messages().to_vec(),
                        ));
                    }
                }
                Ok(None)
            }
            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.enter_new_line();
                Ok(None)
            }
            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.move_cursor_left();
                Ok(None)
            }
            KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.move_cursor_right();
                Ok(None)
            }
            KeyCode::Left => {
                self.input.move_cursor_left();
                Ok(None)
            }
            KeyCode::Right => {
                self.input.move_cursor_right();
                Ok(None)
            }
            KeyCode::Backspace => {
                self.input.delete_char();
                Ok(None)
            }
            KeyCode::Char(c) => {
                self.input.enter_char(c);
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
                self.conversation.tick();
            }
            Action::ReceiveChunk(chunk) => {
                self.conversation.append_chunk(&chunk);
            }
            Action::StreamEnd => {
                self.conversation.finish_response();
            }
            _ => {}
        }

        Ok(None)
    }

    fn draw(
        &mut self,
        frame: &mut ratatui::Frame,
        area: ratatui::prelude::Rect,
    ) -> color_eyre::Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
            .split(area);

        let conversation_widget = Paragraph::new(self.conversation.render())
            .block(Block::default().title("Chat").borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        frame.render_widget(conversation_widget, chunks[0]);

        let input_widget = Paragraph::new(self.input.render())
            .block(
                Block::default()
                    .title("Input (Enter=send, Ctrl+J=newline, Ctrl+H=left, Ctrl+L=right, Esc=quit)")
                    .borders(Borders::ALL)
                    .border_style(if self.focused {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    }),
            );

        frame.render_widget(input_widget, chunks[1]);

        // use native Ratatui Frame for real cursor position
        // 放置真实终端光标（当输入框获得焦点时）
        if self.focused {
            let (col, line) = self.input.cursor_position();
            let x = chunks[1].x + 1 + col;
            let y = chunks[1].y + 1 + line;
            frame.set_cursor_position(Position::new(x, y));
        }

        Ok(())
    }
}
