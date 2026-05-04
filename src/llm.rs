use crate::{action::Action, message::Message};
use futures::StreamExt;
use tokio::sync::mpsc::UnboundedSender;

const LLM_API_URL: &str = "http://127.0.0.1:8080/v1/chat/completions";

pub async fn stream_chat(
    system: &Message,
    messages: &[Message],
    tx: UnboundedSender<Action>,
) -> color_eyre::Result<()> {
    let client = reqwest::Client::new();

    let mut api_messages: Vec<&Message> = vec![system];
    api_messages.extend(messages.iter());

    let body = serde_json::json!({
        "model": "gemma-4-31b",
        "messages": api_messages,
        "stream": true
    });

    let response = client.post(LLM_API_URL).json(&body).send().await?;

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&bytes));

        while let Some(pos) = buffer.find('\n') {
            let line = buffer.drain(..=pos).collect::<String>();

            match parse_sse_event(&line) {
                SseEvent::Done => {
                    let _ = tx.send(Action::StreamEnd);
                    return Ok(());
                }
                SseEvent::Chunk(content) => {
                    let _ = tx.send(Action::ReceiveChunk(content));
                }
                SseEvent::Skip => {}
            }
        }
    }

    let _ = tx.send(Action::StreamEnd);
    Ok(())
}

#[derive(Debug, PartialEq)]
enum SseEvent {
    Chunk(String),
    Done,
    Skip,
}

fn parse_sse_event(line: &str) -> SseEvent {
    let line = line.trim();
    if line.is_empty() {
        return SseEvent::Skip;
    }

    let data = match line.strip_prefix("data: ") {
        Some(d) => d,
        None => return SseEvent::Skip,
    };

    if data == "[DONE]" {
        return SseEvent::Done
    }

    match serde_json::from_str::<serde_json::Value>(data) {
        Ok(v) => match v["choices"][0]["delta"]["content"].as_str() {
            Some(s) => SseEvent::Chunk(s.into()),
            None => SseEvent::Skip,
        },
        _ => SseEvent::Skip,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parse_valid_delta() {
        let line = r#"data: {"choices":[{"delta":{"content":"hello"}}]}"#;
        assert_eq!(parse_sse_event(line), SseEvent::Chunk("hello".into()));
    }

    #[test]
    fn parse_done() {
        let line = "data: [DONE]";
        assert_eq!(parse_sse_event(line), SseEvent::Done);
    }

    #[test]
    fn parse_empty_line() {
        assert_eq!(parse_sse_event(""), SseEvent::Skip);
        assert_eq!(parse_sse_event("  "), SseEvent::Skip);
    }

    #[test]
    fn parse_no_data_prefix() {
        assert_eq!(parse_sse_event("random stuff"), SseEvent::Skip);
    }

    #[test]
    fn parse_invalid_json() {
        assert_eq!(parse_sse_event("data: not json"), SseEvent::Skip);
    }

    #[test]
    fn parse_no_content_field() {
        let line = r#"data: {"choices":[{"delta":{}}]}"#;
        assert_eq!(parse_sse_event(line), SseEvent::Skip);
    }
}
