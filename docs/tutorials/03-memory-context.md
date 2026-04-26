# Tutorial 03：给 Chat 添加记忆（上下文）

> **目标**：让 LLM 能记住之前的对话内容，实现真正的多轮聊天。
> **前置要求**：已完成 [Tutorial 02d](02d-display-response.md)。

---

## 问题：为什么现在的 Chat 没有记忆？

打开 `src/llm.rs`，看 `stream_chat` 函数构造的 HTTP 请求体：

```rust
let body = serde_json::json!({
    "model": "gemma-4-31b",
    "messages": [
        {"role": "user", "content": prompt}
    ],
    "stream": true
});
```

注意 `"messages"` 数组里**永远只有一条消息**——就是用户刚才输入的那句。无论之前聊了多少轮，LLM 每次收到的都是一张"白纸"。

这就像你跟一个人聊天，但对方每句话说完就失忆，完全不记得你们之前说过什么。

### 真正的多轮对话应该长什么样？

OpenAI-compatible API 的 `messages` 字段其实支持一个**数组**：

```json
{
  "messages": [
    {"role": "user", "content": "你好"},
    {"role": "assistant", "content": "你好！有什么可以帮你的？"},
    {"role": "user", "content": "刚才我说了什么？"}
  ]
}
```

LLM 读到这个数组，就能理解对话的完整上下文，回答"你刚才说了'你好'"。

**所以我们的任务很清晰：**
1. 在 `Chat` 组件里维护一个**结构化的对话历史**（不只是 `Vec<String>` 显示文本）。
2. 每次调用 LLM 时，把**整个历史**塞进去，而不只是最后一条。
3. 当 AI 流式回复结束时，把完整的回复也**追加进历史**，为下一轮做准备。

---

## 核心设计：显示数据 vs 业务数据分离

在动手之前，先想清楚一个架构问题：

当前 `Chat` 里的 `messages: Vec<String>` 存的是带前缀的显示文本，比如 `"You: 你好"`、`"AI: 你好！"`。这种格式是给**人看的**，不是给**API 用的**。

如果直接把 `"You: 你好"` 发给 LLM，LLM 会困惑："为什么用户自称 You？"

所以我们需要**两套数据并行存在**：

| 字段 | 用途 | 内容示例 |
|------|------|---------|
| `messages: Vec<String>` | **显示**在屏幕上 | `"You: 你好"`, `"AI: 你好！"` |
| `conversation: Vec<Message>` | **发送**给 LLM API | `{role: "user", content: "你好"}` |

这叫做**关注点分离（Separation of Concerns）**。UI 管 UI，业务管业务，互不干扰。

> **类比**：前端 React 里，`messages` 像是你直接渲染在 DOM 里的 JSX；`conversation` 像是你要 POST 给后端的标准化 JSON。

---

## Step A：定义 Message 结构体

### A1. 创建 `src/message.rs`

我们要定义一个代表"单条对话消息"的类型。OpenAI API 要求每条消息有 `role` 和 `content`：

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}
```

**为什么 `role` 是 `String` 而不是枚举？**

OpenAI API 的 role 有 `"system"`、`"user"`、`"assistant"`、`"tool"` 等，而且各家本地 LLM 还可能支持自定义 role。用 `String` 更灵活，不需要每次新增 role 就改代码。

### A2. 给 Message 加上构造函数

在 `impl Message` 里提供两个工厂方法，让调用方代码更干净：

```rust
impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
        }
    }
}
```

**`impl Into<String>` 是什么意思？**

这是 Rust 的**泛型参数 + trait bound**。它表示：传入任何能转换成 `String` 的类型都行——`&str`、`String`、`Box<str>` 都可以。

| 调用方式 | 实际效果 |
|---------|---------|
| `Message::user("hello")` | `&str` → `String` |
| `Message::user(some_string)` | 直接拿走 `String` |

这比直接写 `content: String` 更友好，调用方不需要手动 `.to_string()`。

### A3. 注册模块

在 `src/main.rs` 的 `mod` 列表里加上：

```rust
mod message;
```

Rust 编译器才能找到这个文件。

---

## Step B：扩展 Action 枚举

### B1. 修改 `src/action.rs`

当前 `Action` 只有两个和聊天相关的变体：

```rust
SendMessage(String),     // 用户发送了一条消息
ReceiveChunk(String),    // LLM 回复了一块文字
```

我们需要做两件事：
1. **让 `SendMessage` 携带完整历史**，否则 `App` 不知道之前聊了什么。
2. **新增 `StreamEnd`**，标记 LLM 流式输出已结束。

**为什么需要 `StreamEnd`？**

流式回复是一块一块来的。`ReceiveChunk("你")`、`ReceiveChunk("好")`…… `Chat` 收到每一块都会实时追加到屏幕上。但**什么时候把这条完整的 AI 回复保存进历史**？

你不能每收到一个 chunk 就 push 一条 `Message::assistant`，那样历史里会碎成几百条。你必须等**整条回复结束**，再一次性 push。

所以 `llm.rs` 要在流结束时发送一个特殊信号：`Action::StreamEnd`。

修改后的 `Action`：

```rust
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
    #[strum(to_string = "SendMessage")]
    SendMessage(Vec<Message>), // ← 现在携带完整对话历史
    ReceiveChunk(String),
    StreamEnd,                 // ← 新增：流结束标记
}
```

**`#[strum(to_string = "SendMessage")]` 是干什么的？**

`strum::Display` 自动给枚举实现 `to_string()`。对于带 payload 的变体，strum 默认行为是**只输出变体名字本身**（如 `SendMessage`），不会展开 `Vec<Message>` 里的内容。

那我们为什么还要写 `#[strum(to_string = "SendMessage")]`？

1. **显式声明，消除隐式依赖**：默认行为虽然是"只输出名字"，但依赖"默认刚好是我想要的"是一种隐式假设。加上这个属性就是告诉读代码的人："我明确指定了这个变体的显示格式。"

2. **防止未来意外展开**：strum 支持用 `"{0}"` 这样的语法把 payload 展开进字符串（如 `SendMessage([Message { ... }])`）。显式写 `to_string = "SendMessage"` 相当于**锁死**：只显示这个名字，不要展开任何字段。

| 写法 | 输出示例 |
|------|---------|
| 不写属性（默认） | `SendMessage` |
| `#[strum(to_string = "SendMessage")]` | `SendMessage` |
| `#[strum(to_string = "SendMessage({0})")]` | `SendMessage([Message { ... }])` ← 内容被展开 |

---

## Step C：重构 Chat 组件

### C1. 添加两个新字段

打开 `src/components/chat.rs`，在 `Chat` 结构体里加：

```rust
pub struct Chat {
    command_tx: Option<UnboundedSender<Action>>,
    messages: Vec<String>,           // 显示用（带 "You:" / "AI:" 前缀）
    conversation: Vec<Message>,      // ← 新增：API 用的结构化历史
    current_ai_response: String,     // ← 新增：当前正在流式接收的 AI 回复缓冲区
    input: String,
    focused: bool,
    waiting_for_response: bool,
    tick_count: u8,
}
```

**`current_ai_response` 是做什么的？**

流式输出期间，`ReceiveChunk` 是一块一块来的。我们需要一个地方把这些碎片**拼成完整句子**。等收到 `StreamEnd` 时，再把完整的句子包装成 `Message::assistant(...)` 存进 `conversation`。

如果不做这个缓冲，你就只能在流结束时从 `messages` 里解析 `"AI: xxx"` 来提取内容——那太脏了。

### C2. 修改构造函数

```rust
impl Chat {
    pub fn new() -> Self {
        Self {
            command_tx: None,
            messages: Vec::new(),
            conversation: Vec::new(),          // ← 新增
            current_ai_response: String::new(), // ← 新增
            input: String::new(),
            focused: true,
            waiting_for_response: false,
            tick_count: 0,
        }
    }
}
```

### C3. 修改 Enter 按键处理

找到 `handle_key_event` 里的 `KeyCode::Enter` 分支。以前只发一条 `Action::SendMessage(text)`，现在要：

1. 先把用户输入保存进 `conversation`
2. 再发送携带完整历史的 `Action`

```rust
KeyCode::Enter => {
    if !self.input.is_empty() {
        let text = self.input.clone();

        // 1. 显示到屏幕上
        self.messages.push(format!("You: {}", text));

        // 2. 保存进结构化历史
        self.conversation.push(Message::user(&text));

        self.input.clear();
        self.start_waiting();

        // 3. 发送完整历史给 App
        if let Some(ref tx) = self.command_tx {
            let _ = tx.send(Action::SendMessage(self.conversation.clone()));
        }
    }
    Ok(None)
}
```

**为什么要 `self.conversation.clone()`？**

`Vec<Message>` 没有实现 `Copy` trait（它内部数据在堆上），所以发送时会发生**所有权转移（move）**。但 `self.conversation` 是 `Chat` 的字段，你不能把它的所有权交出去（否则 `Chat` 自己就没法用了）。

`clone()` 创建一份深拷贝，把拷贝发走，原数据留在 `Chat` 里。详见 [所有权笔记](../notes/ownership.md)。

### C4. 修改 `update` 方法

以前 `update` 只处理 `ReceiveChunk`：

```rust
Action::ReceiveChunk(chunk) => {
    self.stop_waiting();
    self.append_ai_text(&chunk);
}
```

现在要加上两件事：
- 收到 `ReceiveChunk` 时，**同时追加到 `current_ai_response`**
- 收到 `StreamEnd` 时，**把完整的 `current_ai_response` 存进 `conversation`**

```rust
fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
    match action {
        Action::Tick => {
            self.tick_count = self.tick_count.wrapping_add(1);
        }
        Action::ReceiveChunk(chunk) => {
            self.stop_waiting();
            self.current_ai_response.push_str(&chunk); // ← 累积到缓冲区
            self.append_ai_text(&chunk);
        }
        Action::StreamEnd => {
            // ← 新增：流结束，把完整回复归档进历史
            if !self.current_ai_response.is_empty() {
                self.conversation.push(Message::assistant(&self.current_ai_response));
                self.current_ai_response.clear();
            }
        }
        _ => {}
    }
    Ok(None)
}
```

**为什么用 `push_str` 而不是 `push`？**

- `String::push(c)` 追加单个字符
- `String::push_str(s)` 追加整个字符串切片

`chunk` 是一段文字（可能是一个词、一个字，取决于 LLM），所以用 `push_str`。

**`clear()` 后内存会立刻还给系统吗？**

不会。`String::clear()` 只是把长度设为 0，已分配的**容量（capacity）** 还保留着。下一次再 push 时不需要重新分配内存。这是高性能 Rust 代码的常见模式。

---

## Step D：修改 App 和 llm.rs

### D1. 修改 `src/app.rs` 的 `handle_actions`

找到 `Action::SendMessage(ref prompt)` 分支，改成接收历史：

```rust
Action::SendMessage(ref history) => {
    let tx = self.action_tx.clone();
    let history = history.clone();
    tokio::spawn(async move {
        if let Err(e) = llm::stream_chat(history, tx).await {
            tracing::error!("LLM error: {}", e);
        }
    });
}
```

**为什么这里有两个 `clone()`？**

- `self.action_tx.clone()`：`UnboundedSender` 实现了 `Clone`，但 clone 的不是通道本身，而是一个**新的发送端句柄**。多个句柄可以并发往同一个通道发消息。
- `history.clone()`：和前面一样，因为 `history` 要被 `move` 进 `async move` 闭包。

### D2. 修改 `src/llm.rs`

#### 改函数签名

```rust
pub async fn stream_chat(
    messages: Vec<crate::message::Message>,
    tx: UnboundedSender<Action>,
) -> color_eyre::Result<()> {
```

#### 改请求体构造

以前硬编码单条消息：

```rust
let body = serde_json::json!({
    "model": "gemma-4-31b",
    "messages": [
        {"role": "user", "content": prompt}
    ],
    "stream": true
});
```

现在要动态转换整个历史：

```rust
let api_messages: Vec<_> = messages
    .iter()
    .map(|m| serde_json::json!({"role": &m.role, "content": &m.content}))
    .collect();

let body = serde_json::json!({
    "model": "gemma-4-31b",
    "messages": api_messages,
    "stream": true
});
```

**`iter()` + `map()` + `collect()` 的三件套**

这是 Rust 处理集合的**最常用模式**：

| 方法 | 作用 |
|------|------|
| `.iter()` | 借用集合里的每个元素，不拿走所有权 |
| `.map(|m| ...)` | 对每个元素做转换 |
| `.collect::<Vec<_>>()` | 把转换后的迭代器收集成新的集合 |

`_` 表示"让编译器推断元素类型"。这里编译器能推断出是 `serde_json::Value`。

#### 发送 StreamEnd

在 `llm.rs` 里找到两处地方发送结束信号：

1. 收到 SSE `[DONE]` 时：

```rust
if data == "[DONE]" {
    let _ = tx.send(Action::StreamEnd); // ← 新增
    return Ok(());
}
```

2. 流自然结束时（函数末尾，作为保险）：

```rust
// 循环结束，流已耗尽
let _ = tx.send(Action::StreamEnd); // ← 新增
Ok(())
```

**为什么要在两处都发？**

有些 LLM server（或代理）可能不会发送 `[DONE]`，而是直接关闭连接。如果在 `while` 循环结束后补发一次，可以确保 UI 层一定能收到 `StreamEnd`，不会漏掉归档。

---

## Step E：编译并测试

```bash
cargo build
cargo run
```

测试步骤：
1. 输入"你好"，按 Enter
2. 等 AI 回复结束
3. 输入"刚才我说了什么？"，按 Enter
4. **观察**：AI 应该能正确回答"你刚才说了'你好'"。

如果 AI 还是回答"我不知道"或"你什么都没说"，说明历史没有正确传递。检查：
- `Chat::conversation` 是否在 Enter 时 push 了 user message
- `llm.rs` 的 `api_messages` 是否正确构造
- `StreamEnd` 是否被发送和接收
- `Chat::update` 的 `StreamEnd` 分支是否正确把 `current_ai_response` push 进 `conversation`

---

## 概念检查清单

确认你能回答这些问题：

1. **为什么 `messages: Vec<String>` 不能直接传给 LLM API？**
2. **`clone()` 在这里的作用是什么？没有它会怎样？**
3. **为什么需要 `StreamEnd`？只用 `ReceiveChunk` 有什么问题？**
4. **`current_ai_response` 和 `conversation` 的职责分别是什么？**
5. **如果 LLM server 不发送 `[DONE]`，我们的代码还能正常工作吗？为什么？**

---

## Bonus：上下文窗口的隐忧

做到这里，你的 Chat 已经有了记忆。但还有一个现实问题：**上下文窗口（Context Window）**。

LLM 的 `messages` 数组不能无限增长。每个模型都有最大上下文长度（比如 4096 tokens、32768 tokens）。如果对话太长，API 会报错或自动截断。

### 思考方向（不实现，只作为学习思考题）：

- **截断策略**：当历史超过一定长度时，删除最旧的消息。但删除后 LLM 会"忘记"早期内容。
- **摘要策略**：让 LLM 自己生成一段摘要，替换掉早期的详细对话。
- **持久化**：把 `conversation` 保存到本地文件，下次启动时恢复。

这些都是在真实产品中必须面对的问题。我们这个教程故意不做那么复杂，因为**核心目标是理解"记忆是如何工作的"**，而不是做一个完美的产品。

---

## 参考阅读

- [Rust 所有权笔记](../notes/ownership.md) — `clone()`、`move`、借用的核心规则
- [async move 笔记](../notes/async-move.md) — 为什么 `tokio::spawn` 里必须用 `move`
- [impl 块拆分笔记](../notes/impl-blocks.md) — `impl Chat` 和 `impl Component for Chat` 的分工
