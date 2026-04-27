use ratatui::text::{Line, Text};

use crate::{message::Message, utils};

pub struct Conversation {
    display: Vec<String>,
    conversation: Vec<Message>,
    current_response: String,
    waiting: bool,
    tick: u8,
}

impl Conversation {
    pub fn new() -> Self {
        Self {
            display: Vec::new(),
            conversation: Vec::new(),
            current_response: String::new(),
            waiting: false,
            tick: 0,
        }
    }

    pub fn push_user(&mut self, text: &str) -> Message {
        self.display.push(format!("You: {}", text));
        let msg = Message::user(text);
        self.conversation.push(msg.clone());
        msg
    }

    pub fn start_response(&mut self) {
        self.waiting = true
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1)
    }

    pub fn messages(&self) -> &[Message] {
        &self.conversation
    }

    pub fn append_chunk(&mut self, chunk: &str) {
        self.waiting = false;
        self.current_response.push_str(chunk);

        if let Some(last) = self.display.last_mut()
            && last.starts_with("AI: ")
        {
            last.push_str(chunk);
        } else {
            self.display.push(format!("AI: {}", chunk))
        }
    }

    pub fn finish_response(&mut self) {
        if !self.current_response.is_empty() {
            self.conversation
                .push(Message::assistant(&self.current_response));
            self.current_response.clear();
        }
    }

    pub fn render(&self) -> Text<'static> {
        let mut lines: Vec<Line> = self.display.iter().map(|m| Line::from(m.clone())).collect();

        if self.waiting {
            lines.push(Line::from(format!(
                "AI: {} thinking",
                utils::spinner_frame(self.tick as usize)
            )))
        }

        Text::from(lines)
    }
}
