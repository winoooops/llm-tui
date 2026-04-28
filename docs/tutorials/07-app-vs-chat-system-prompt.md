# Tutorial 07：App vs Chat —— 谁该拥有 System Prompt？

> **目标**：用 SRP + 信息隐藏分析 System Prompt 的归属，并实现一种比"注入 Chat"更干净的方案。
> **前置要求**：已完成 [Tutorial 06](06-system-prompt-assembly.md)。

---

## 问题

Tutorial 06 完成后，System Prompt 由 `PromptContext` 组装。但它该存在哪里？

- **Chat 拥有**？Chat 是发消息的组件，似乎该它管。
- **App 拥有**？App 是组合根，管全局配置更合理。

---

## System Prompt 到底是谁的上下文？

| 维度 | Chat 的上下文 | App 的上下文 |
|------|--------------|-------------|
| 用户输入历史 | ✅ | ❌ |
| 光标位置 | ✅ | ❌ |
| 当前聚焦状态 | ✅ | ❌ |
| AI 的身份定义（"你是 Coding Agent"） | ❌ | ✅ |
| 项目类型（Rust/Node/Python） | ❌ | ✅ |
| 工作目录、AGENTS.md | ❌ | ✅ |

**关键洞察**：System Prompt 的 90% 是"这个项目是什么样的"、"AI 该怎么 behave"——这是**应用级**信息，不是**聊天窗口级**信息。

---

## 方案对比

### 方案 A：Chat 拥有（Tutorial 06 初始设计）

```
Chat::new()
  └─ PromptContext::from_environment().system_prompt()
```

Chat 存 `system_prompt: Message`，发 `Action::SendMessage(system, history)`。

**问题**：
1. Chat 被迫耦合文件系统（Cargo.toml、README.md）
2. **穿堂风**：Chat 不解析、不使用 prompt，只是存 → clone → 转发
3. Action 变胖：`SendMessage(Message, Vec<Message>)`

### 方案 B：App 拥有，注入 Chat（常见做法）

```
App::new()
  ├─ system_prompt = PromptContext::from_environment().system_prompt()
  └─ Chat::new(system_prompt)   // 通过构造函数注入
```

Chat 仍然存 `system_prompt: Message`，但不再负责组装。

**改进**：消除了文件系统耦合，但 Chat 仍是穿堂风——它只是替 App 保管 prompt。

### 方案 C：App 拥有，不注入 Chat，只在 dispatch 层拼接 ⭐

```
App::new()
  └─ system_prompt = PromptContext::from_environment().system_prompt()

Chat::handle_key_event()
  └─ Action::SendMessage(history)   // 只带对话历史

App::handle_actions()
  └─ SendMessage(history) ──→ llm::stream_chat(&system_prompt, &history, tx)
```

**Chat 完全不知道 system prompt 存在。**

| 指标 | 方案 A | 方案 B | 方案 C |
|------|--------|--------|--------|
| Chat 是否知悉 system prompt | 组装 + 存储 | 只存储 | **完全不知** |
| Action 定义 | `SendMessage(Message, Vec<Message>)` | 同上 | `SendMessage(Vec<Message>)` |
| 穿堂风 | 严重 | 中等 | **无** |

---

## 为什么方案 C 最干净

### 1. Action 只携带"用户数据"

```rust
pub enum Action {
    // ...
    SendMessage(Vec<Message>),  // 只有对话历史
}
```

System prompt 不是用户产生的，不应该出现在用户 action 里。就像 HTTP request body 只传业务数据，auth token 放在 header 里由客户端统一加。

### 2. Chat 彻底纯粹

```rust
pub struct Chat {
    conversation: Conversation,
    input: Input,
    focused: bool,
    // 没有 system_prompt 字段
}

impl Chat {
    pub fn new() -> Self {  // 无参构造
        Self {
            conversation: Conversation::new(),
            input: Input::new(),
            focused: true,
        }
    }
}
```

Chat 只做 UI：收按键、管输入框、显示对话。它甚至不知道有 system prompt 这回事。

### 3. App 在"最后一公里"注入

```rust
// src/app.rs
fn handle_actions(&mut self, tui: &mut Tui) -> Result<()> {
    while let Ok(action) = self.action_rx.try_recv() {
        match action {
            Action::SendMessage(ref history) => {
                let system = self.system_prompt.clone();
                let history = history.clone();
                let tx = self.action_tx.clone();
                tokio::spawn(async move {
                    let _ = llm::stream_chat(&system, &history, tx).await;
                });
            }
            // ...
        }
    }
}
```

App 是**组合根**，也是**调度中心**。它知道：
- system prompt 是什么（自己存的）
- 什么时候该发 LLM 请求（收到 SendMessage action）
- 怎么发（调用 `llm::stream_chat`）

这三件事都在 App 里发生，没有信息泄露给 Chat。

---

## 迁移步骤

### Step 1：App 存 system_prompt

```rust
// src/app.rs
pub struct App {
    // ...
    system_prompt: Message,
}

impl App {
    pub fn new(tick_rate: f64, frame_rate: f64) -> Result<Self> {
        let system_prompt = PromptContext::from_environment().system_prompt();
        Ok(Self {
            // ...
            system_prompt,
        })
    }
}
```

### Step 2：Action 瘦身

```rust
// src/action.rs
pub enum Action {
    // ...
    SendMessage(Vec<Message>),  // 去掉 Message（system prompt）
}
```

### Step 3：Chat 删掉 system_prompt

```rust
// src/components/chat/mod.rs
pub struct Chat {
    conversation: Conversation,
    input: Input,
    focused: bool,
    // 删除 system_prompt 字段
}

impl Chat {
    pub fn new() -> Self {
        Self {
            conversation: Conversation::new(),
            input: Input::new(),
            focused: true,
        }
    }
}
```

### Step 4：App handle_actions 拼接

```rust
// src/app.rs
Action::SendMessage(ref history) => {
    let system = self.system_prompt.clone();
    let history = history.clone();
    let tx = self.action_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = llm::stream_chat(&system, &history, tx).await {
            tracing::error!("LLM error: {}", e);
        }
    });
}
```

---

## 结论

> **System Prompt 是应用级配置，不是聊天窗口级状态。**
>
> **App 组装，App 调度，Chat 完全不知悉。**
>
> 这比"注入 Chat"更彻底——Chat 不是过道，它根本不在那条路上。
