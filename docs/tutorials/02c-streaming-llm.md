# Tutorial 02c：App 调用 LLM API（流式）

> **目标**：在 App 里写异步 HTTP 请求，流式读取 LLM 回复。  
> **前置要求**：已完成 [Tutorial 02b](02b-send-message.md)。

---

打开 `src/app.rs`。这是最难的一步，因为你要写**异步 HTTP 请求 + 流式解析**。

### 4a. 添加导入

在 `app.rs` 顶部，在现有 `use` 语句后面加：

```rust
use futures::StreamExt;
```

`futures::StreamExt` 提供了 `.next()` 方法，用来从异步流里读取下一块数据。

### 4b. 修改 `handle_actions`

在 `handle_actions` 的 `match action { ... }` 里，加一个新分支（放在 `_ => {}` 前面）：

```rust
Action::SendMessage(prompt) => {
    let tx = self.action_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = call_llm(prompt, tx).await {
            tracing::error!("LLM error: {}", e);
        }
    });
}
```

**这一行一行拆解：**

| 代码 | 含义 |
|------|------|
| `let tx = self.action_tx.clone()` | 克隆一个发送端。因为等下要传给新任务，而原始 `action_tx` 还要继续留在 App 里用 |
| `tokio::spawn(async move { ... })` | 启动一个新的 Tokio 异步任务。`async move` 表示这个任务有自己的生命周期，并且**把 `tx` 和 `prompt` 的所有权移进去** |
| `call_llm(prompt, tx).await` | 调用异步函数 `call_llm`，等待它完成 |
| `tracing::error!(...)` | 如果出错了，记录一条错误日志 |

**为什么必须用 `tokio::spawn`？**

`handle_actions` 跑在主事件循环里。如果你在这里直接 `await` HTTP 请求，整个 UI 会**冻结**——按键没反应、画面不刷新——直到 LLM 返回完整回复（可能要几十秒）。

`tokio::spawn` 创建了一个**后台任务**。HTTP 请求在后台跑，主事件循环继续处理用户输入和渲染。LLM 每吐一块字，后台任务就 `tx.send(Action::ReceiveChunk(...))` 通知主循环更新画面。

**`async move { }` 里的 `move` 是什么意思？**

Rust 默认会尝试**借用**外部变量。但 `tokio::spawn` 创建的任务可能比当前函数活得更久，所以借用会失效。`move` 关键字强制把 `tx` 和 `prompt` **所有权转移**进闭包，这样闭包自己拥有它们，不依赖外部。

### 4c. 添加 `call_llm` 函数

在 `app.rs` 的最底部（`impl App` 块外面），加这个异步函数：

```rust
async fn call_llm(
    prompt: String,
    tx: tokio::sync::mpsc::UnboundedSender<Action>,
) -> color_eyre::Result<()> {
    let client = reqwest::Client::new();
    let url = "http://127.0.0.1:8080/v1/chat/completions";

    let body = serde_json::json!({
        "model": "gemma-4-31b",
        "messages": [
            {"role": "user", "content": prompt}
        ],
        "stream": true
    });

    let response = client
        .post(url)
        .json(&body)
        .send()
        .await?;

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&bytes));

        // SSE 格式：每行以 "data: " 开头
        while let Some(pos) = buffer.find('\n') {
            let line = buffer.drain(..=pos).collect::<String>();
            
            if let Some(data) = line.strip_prefix("data: ") {
                let data = data.trim();
                if data == "[DONE]" {
                    return Ok(());
                }

                // 解析 JSON，提取 delta.content
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(content) = v["choices"][0]["delta"]["content"].as_str() {
                        let _ = tx.send(Action::ReceiveChunk(content.to_string()));
                    }
                }
            }
        }
    }

    Ok(())
}
```

**这段代码很复杂，逐段解释：**

**`serde_json::json!` 宏**

```rust
let body = serde_json::json!({
    "model": "gemma-4-31b",
    "messages": [{"role": "user", "content": prompt}],
    "stream": true
});
```

`json!` 是 serde_json 提供的宏。你在 `{}` 里直接写 JSON 格式，它会生成一个 `serde_json::Value`。`prompt` 变量会自动插入进去。

**发送请求**

```rust
let response = client.post(url).json(&body).send().await?;
```

- `.post(url)` — 构造 POST 请求
- `.json(&body)` — 把 body 序列化成 JSON，并加 `Content-Type: application/json` 头
- `.send()` — 真正发送请求，返回一个 Future
- `.await?` — 等待请求完成。如果出错，`?` 会提前返回错误

**流式读取**

```rust
let mut stream = response.bytes_stream();
```

不是等整个响应回来再处理，而是得到一个**字节流**。LLM 每生成一小块文字，网络上就会传过来一小块数据，流里就会多出一个 chunk。

```rust
while let Some(chunk) = stream.next().await {
    let bytes = chunk?;
    buffer.push_str(&String::from_utf8_lossy(&bytes));
```

- `stream.next().await` — 异步等待下一块数据
- `chunk?` — 如果网络出错，提前返回
- `String::from_utf8_lossy(&bytes)` — 把原始字节转成字符串。`lossy` 表示如果遇到非法 UTF-8 字节，会用 `�` 替换（不会崩溃）
- 追加到 `buffer` — 因为网络 chunk 可能只包含半行 SSE 数据，需要缓冲

**SSE 解析**

OpenAI / llama.cpp 的流式响应用的是 **SSE（Server-Sent Events）** 格式。它本质上是一段一段的文本，每段以 `data: ` 开头：

```
data: {"choices":[{"delta":{"content":"Hello"}}]}

data: {"choices":[{"delta":{"content":" world"}}]}

data: [DONE]
```

```rust
while let Some(pos) = buffer.find('\n') {
    let line = buffer.drain(..=pos).collect::<String>();
```

- 从缓冲区的开头找到第一个换行符 `\n`
- `buffer.drain(..=pos)` — 把从开头到换行符的这段文字**取出来**（同时从 buffer 里删掉）
- `.collect::<String>()` — 把取出来的字符收集成一条完整的 `String`

```rust
if let Some(data) = line.strip_prefix("data: ") {
```

- `strip_prefix("data: ")` — 如果这行以 `data: ` 开头，返回后面的内容；否则返回 `None`
- `if let Some(data) = ...` — 只处理真正的 SSE 数据行

```rust
let data = data.trim();
if data == "[DONE]" {
    return Ok(());
}
```

- `[DONE]` 是流结束的标志。收到它就返回。

```rust
if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
    if let Some(content) = v["choices"][0]["delta"]["content"].as_str() {
        let _ = tx.send(Action::ReceiveChunk(content.to_string()));
    }
}
```

- `serde_json::from_str::<serde_json::Value>(data)` — 把 JSON 字符串解析成动态 JSON 值
- `v["choices"][0]["delta"]["content"]` — 像查字典一样取嵌套字段
- `.as_str()` — 如果该字段是字符串，返回 `Some(&str)`，否则 `None`
- `tx.send(...)` — 把这块文字发给 App 的事件循环

**`serde_json::Value` 是什么？**

它是 serde_json 提供的"动态 JSON"类型。你不需要预先定义结构体，可以直接用 `[]` 索引字段。适合字段很多、结构复杂、或者你只需要其中一两个字段的场景。

---

*下一步：[Tutorial 02d](02d-display-response.md) — 显示 LLM 回复并测试。*
