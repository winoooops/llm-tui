use ratatui::text::{Line, Text};

use crate::{message::Message, utils};

pub struct Conversation {
    display: Vec<String>,
    messages: Vec<Message>,
    current_response: String,
    waiting: bool,
    tick: u8,
}

impl Conversation {
    pub fn new() -> Self {
        Self {
            display: Vec::new(),
            messages: Vec::new(),
            current_response: String::new(),
            waiting: false,
            tick: 0,
        }
    }

    pub fn push_user(&mut self, text: &str) -> Message {
        self.display.push(format!("You: {}", text));
        let msg = Message::user(text);
        self.messages.push(msg.clone());
        msg
    }

    pub fn start_response(&mut self) {
        self.waiting = true
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1)
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
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
            self.messages
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

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn new_conversation_is_empty() {
        let conversation = Conversation::new();
        assert!(conversation.messages().is_empty());
    }

    #[test]
    fn push_user_adds_message() {
        let mut conversation = Conversation::new();
        conversation.push_user("hello");
        assert_eq!(conversation.messages().len(), 1);
        assert_eq!(conversation.messages()[0].role, "user");
        assert_eq!(conversation.messages()[0].content, "hello");
    }

    #[test]
    fn start_response_sets_waiting() {
        let mut conversation = Conversation::new();
        conversation.start_response();
        assert_eq!(conversation.waiting, true);
    }

    #[test]
    fn append_chunk_stop_waiting_and_create_line() {
        let mut conversation = Conversation::new();
        conversation.start_response();
        conversation.append_chunk("hi");
        assert_eq!(conversation.waiting, false);
        let text = conversation.render();
        assert!(text.to_string().contains("AI: hi"))
    }

    #[test]
    fn append_chunk_appends_to_existing_ai_line() {
        let mut conversation = Conversation::new();
        conversation.append_chunk("hello");
        assert!(conversation.render().to_string().contains("AI: hello"));
        conversation.append_chunk(" world");
        assert!(
            conversation
                .render()
                .to_string()
                .contains("AI: hello world")
        );
    }

    #[test]
    fn finish_response_moves_to_conversation() {
        let mut conversation = Conversation::new();
        conversation.append_chunk("hello back");
        conversation.finish_response();
        assert_eq!(conversation.messages().len(), 1);
        assert_eq!(conversation.messages()[0].role, "assistant");
        assert_eq!(conversation.messages()[0].content, "hello back");
    }
}
