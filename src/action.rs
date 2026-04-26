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
