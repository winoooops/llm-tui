# Tutorial 02a：准备 LLM 集成

> **目标**：添加 HTTP 客户端依赖，扩展 Action 枚举。  
> **前置要求**：已完成 [Tutorial 01](01-chat-component.md)。

---

你的 llama.cpp server 跑在 `http://127.0.0.1:8080/v1`，用的是 **OpenAI-compatible API**。这意味着我们可以用标准的 Chat Completions 格式跟它对话。

你的 llama.cpp server 跑在 `http://127.0.0.1:8080/v1`，用的是 **OpenAI-compatible API**。这意味着我们可以用标准的 Chat Completions 格式跟它对话。

### B1. 添加 HTTP 客户端依赖

打开 `Cargo.toml`，在 `[dependencies]` 里加一行：

```toml
reqwest = { version = "0.12", features = ["json", "stream"] }
```

**为什么需要 `reqwest`？**

Rust 标准库没有 HTTP 客户端。`reqwest` 是最流行的异步 HTTP 库，基于 `hyper` 封装，API 很友好。

**为什么需要这些 features？**

- `json` — 自动把 Rust 数据结构序列化成 JSON，也自动把 JSON 响应反序列化
- `stream` — 支持流式读取响应体（LLM 是一边生成一边吐字的，我们需要实时接收）

加完依赖后运行一次 `cargo build`，让 cargo 下载并编译新依赖（这会花一点时间）。

---

### B2. 扩展 Action 枚举

打开 `src/action.rs`，加两个新变体：

```rust
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
    SendMessage(String),     // ← 新增：用户发送了一条消息
    ReceiveChunk(String),    // ← 新增：LLM 回复了一块文字
}
```

**为什么用两个 Action？**

- `SendMessage` = 用户按了 Enter，告诉系统"该去调用 LLM 了"
- `ReceiveChunk` = LLM 生成了一部分文字，告诉系统"把这段字显示出来"

分开的原因是：**发送和接收发生在不同时间、不同任务里**。

---

