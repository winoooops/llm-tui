use serde::{Deserialize, Serialize};
use strum::Display;

use crate::message::Message;

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    ClearScreen,
    Error(String),
    Help,
    #[strum(to_string = "SendMessage")] // see @docs/notes/03-memory-context.md for more
    SendMessage(Vec<Message>), // user sends prompt
    ReceiveChunk(String), // agents responds in stream
    StreamEnd,            // marks when the stream respond is completed
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn display_variants() {
        assert_eq!(Action::Tick.to_string(), "Tick");
        assert_eq!(Action::Quit.to_string(), "Quit");
        assert_eq!(Action::StreamEnd.to_string(), "StreamEnd");
        assert_eq!(Action::Help.to_string(), "Help");
        assert_eq!(Action::Suspend.to_string(), "Suspend");
        assert_eq!(Action::Render.to_string(), "Render");
        assert_eq!(Action::Resume.to_string(), "Resume");
        assert_eq!(Action::ClearScreen.to_string(), "ClearScreen");
    }

    #[test]
    fn send_message_display_and_payload() {
        let action = Action::SendMessage(vec![Message::user("hi")]);
        assert_eq!(action.to_string(), "SendMessage");

        if let Action::SendMessage(msgs) = action {
            assert_eq!(msgs.len(), 1);
        } else {
            panic!("expted SendMessage variant");
        }
    }

    #[test]
    fn receive_chunk_display_and_paylod() {
        let action = Action::ReceiveChunk("[done]".into());
        assert_eq!(action.to_string(), "ReceiveChunk");

        if let Action::ReceiveChunk(chunk) = action {
            assert_eq!(chunk, "[done]");
        } else {
            panic!("expected ReceiveChunk variant");
        }
    }
}
