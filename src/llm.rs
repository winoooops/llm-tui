use crate::{action::Action, message::Message};
use futures::StreamExt;
use tokio::sync::mpsc::UnboundedSender;

const LLM_API_URL: &str = "http://127.0.0.1:8080/v1/chat/completions";

pub async fn stream_chat(
    messages: Vec<Message>,
    tx: UnboundedSender<Action>,
) -> color_eyre::Result<()> {
    let client = reqwest::Client::new();

    let api_messages: Vec<_> = messages
        .iter()
        .map(|m| serde_json::json!({"role": &m.role, "content": &m.content}))
        .collect();

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

        while let Some(pos) = buffer.find("\n") {
            let line = buffer.drain(..=pos).collect::<String>();

            if let Some(data) = line.strip_prefix("data: ") {
                let data = data.trim();
                if data == "[DONE]" {
                    let _ = tx.send(Action::StreamEnd);
                    return Ok(());
                }

                if let Ok(v) = serde_json::from_str::<serde_json::Value>(data)
                    && let Some(content) = v["choices"][0]["delta"]["content"].as_str()
                {
                    let _ = tx.send(Action::ReceiveChunk(content.to_string()));
                }
            }
        }
    }

    let _ = tx.send(Action::StreamEnd);
    Ok(())
}
