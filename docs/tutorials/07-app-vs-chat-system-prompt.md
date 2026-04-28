# App vs Chat：谁该拥有 System Prompt？

这是一个架构设计问题，没有绝对正确答案，但可以用 SRP + 信息隐藏来权衡。

---

## 当前设计（Chat 拥有）

```
App::new()
  └─ Chat::new()
       └─ PromptContext::from_environment().system_prompt()
```

`Chat` 存储 `system_prompt: Message`，发送时 `clone()` 给 `Action::SendMessage`。

---

## 挑战：System Prompt 到底是谁的上下文？

| 维度 | Chat 的上下文 | App 的上下文 |
|------|--------------|-------------|
| 用户输入历史 | ✅ | ❌ |
| 光标位置 | ✅ | ❌ |
| 当前聚焦状态 | ✅ | ❌ |
| AI 的身份定义（"你是 Coding Agent"） | ❌ | ✅ |
| 项目类型（Rust/Node/Python） | ❌ | ✅ |
| 工作目录 | ❌ | ✅ |
| AGENTS.md 约束 | ❌ | ✅ |

**关键洞察**：System Prompt 的 90% 内容是"这个项目是什么样的"、"AI 该怎么 behave"——这是**应用级**信息，不是**聊天窗口级**信息。

---

## 如果 Chat 拥有（当前设计）的问题

### 1. Chat 被迫知道文件系统

```
Chat ──→ PromptContext ──→ 读 Cargo.toml、README.md、AGENTS.md
```

Chat 是一个 UI 组件，它的职责是：接收按键、渲染输入框、显示对话气泡。现在它被迫耦合项目探测逻辑。

### 2. "穿堂风"（Pass-through）反模式

Chat 并不**使用** system prompt 的内容——它不基于 prompt 做决策、不解析 prompt、不修改 prompt。它只是：
1. 存起来
2. `clone()` 
3. 转发给 Action

这叫**穿堂风**——数据从一头进、另一头出，中间模块只是过道。《A Philosophy of Software Design》明确说这是 bad smell。

### 3. 无法共享

如果未来加第二个 LLM 消费者（比如侧边栏显示 "AI 建议"），它也需要 system prompt。但 prompt 锁在 Chat 里，其他组件拿不到。

---

## 如果 App 拥有的好处

```
App::new()
  ├─ PromptContext::from_environment().system_prompt()
  │
  └─ Chat::new(system_prompt)   // 注入，不是让 Chat 自己组装
```

### 1. 职责边界清晰

| 模块 | 知道什么 |
|------|---------|
| App | 项目上下文、AI 身份、System Prompt 组装 |
| Chat | 用户输入、对话历史、渲染、事件路由 |
| llm.rs | HTTP 传输、序列化 |

### 2. Chat 变薄，更容易测试

```rust
// 测试 Chat 时，直接注入假 prompt，不用 mock 文件系统
let chat = Chat::new(Message::system("test"));
```

### 3. 多组件共享天然支持

```rust
let system_prompt = PromptContext::from_environment().system_prompt();

App {
    chat: Chat::new(system_prompt.clone()),
    sidebar: Sidebar::new(system_prompt.clone()), // 未来扩展
}
```

---

## 反对意见 & 回应

**"但 Chat 是发消息的，它应该知道发什么"**

Chat 知道的是**用户消息**（"帮我把这个函数重构一下"）。System prompt 不是消息，是**会话配置**——就像 HTTP Header 不是 Body。App 组装 Header，Chat 发 Body。

**"那 App 不是也变厚了吗？"**

App 本来就是**组合根**（Composition Root）——所有高层对象的组装中心。它本来就该负责：
- 创建组件
- 注入依赖
- 配置全局行为

这是 App 的正当职责，不是 fat。

---

## 最小改动迁移方案

**改 `Chat::new` 接收 system_prompt：**

```rust
// src/components/chat/mod.rs
pub struct Chat {
    system_prompt: Message,
    // ...
}

impl Chat {
    pub fn new(system_prompt: Message) -> Self {
        Self {
            system_prompt,
            conversation: Conversation::new(),
            input: Input::new(),
            // ...
        }
    }
}
```

**改 `App::new` 组装并注入：**

```rust
// src/app.rs
use crate::prompt::PromptContext;

impl App {
    pub fn new(tick_rate: f64, frame_rate: f64) -> Result<Self> {
        let system_prompt = PromptContext::from_environment().system_prompt();
        
        Ok(Self {
            components: vec![
                Box::new(Home::new()),
                Box::new(FpsCounter::default()),
                Box::new(Chat::new(system_prompt)),  // ← 注入
            ],
            // ...
        })
    }
}
```

**删除 `Chat` 对 `prompt` 模块的依赖：**

```rust
// 之前
use crate::prompt::PromptContext;  // 可以删掉
```

---

## 结论

> **System Prompt 是应用级配置，不是聊天窗口级状态。**
>
> App 组装，Chat 消费。这符合 SRP，也消除了穿堂风。
