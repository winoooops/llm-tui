# Tutorial 06：System Prompt 组装 —— 让 LLM 知道它是谁、在哪

> **目标**：实现一个最小版的 System Prompt 组装器，让 LLM 在收到用户消息之前，先拿到"身份定义 + 项目上下文"。
> **前置要求**：已完成 [Tutorial 05](05-refactoring-chat.md)。
> **参考设计**：Claude Code 的 `buildSystemPrompt()`（静态区 + 动态区 + 分界标记）。

---

## 目录

1. [为什么需要 System Prompt？](#为什么需要-system-prompt)
2. [架构设计：静态区 + 动态区 + 组装器](#架构设计静态区--动态区--组装器)
3. [Step 1：让 `Message` 支持 `system` 角色](#step-1让-message-支持-system-角色)
4. [Step 2：创建静态 Prompt 文件](#step-2创建静态-prompt-文件)
5. [Step 3：实现动态上下文收集](#step-3实现动态上下文收集)
6. [Step 4：实现组装器](#step-4实现组装器)
7. [Step 5：注入 API 调用](#step-5注入-api-调用)
8. [Step 6（可选）：TUI 里热加载](#step-6可选tui-里热加载)
9. [验证](#验证)

---

## 为什么需要 System Prompt？

当前 `llm.rs` 直接把用户对话历史丢给 API：

```
[{"role": "user", "content": "You: hello"}]
```

LLM 收到的第一条消息就是 `"You: hello"`。它不知道：
- 自己是 Coding Agent 还是客服机器人
- 当前项目用的是什么语言、什么框架
- 用户偏好的代码风格是什么
- 项目里有没有 `AGENTS.md` 这类约束文件

**System Prompt 的作用就是在用户消息之前，先给 LLM 一套"初始设定"**。这相当于游戏开场时的角色创建界面——LLM 先读一遍"我是谁、我在哪、我要遵守什么规则"，然后再开始对话。

Claude Code 的做法是把 System Prompt 拆成两部分：

```
[Static Content]          ← 核心身份 + 行为约束（对所有用户都一样）
__SYSTEM_PROMPT_DYNAMIC_BOUNDARY__
[Dynamic Content]         ← 项目信息 + 环境快照（每次会话不同）
```

我们不搞缓存策略，只学这个**分层思想**。

---

## 架构设计：静态区 + 动态区 + 组装器

| 层级 | 内容 | 存储位置 | 会变吗？ |
|------|------|----------|---------|
| 静态区 | 核心身份、行为规则、工具使用哲学 | `.prompts/system.md` | 很少 |
| 动态区 | 项目名、工作目录、README 摘要、AGENTS.md | 运行时收集 | 每次启动 |
| 组装器 | 把两层拼成 `Message::system(...)` | `src/prompt.rs` | — |

最终发给 API 的消息队列：

```
[0] role: system, content: "静态 + 分界 + 动态"
[1] role: user,   content: "You: hello"
[2] role: assistant, content: "AI: Hi!"
...
```

---

## Step 1：让 `Message` 支持 `system` 角色

文件：`src/message.rs`

在 `impl Message` 里加一条构造器：

```rust
pub fn system(content: impl Into<String>) -> Self {
    Self {
        role: "system".into(),
        content: content.into(),
    }
}
```

OpenAI/llama.cpp 兼容 API 都认 `"role": "system"`。这条消息会被模型当作最高优先级的指令处理。

---

## Step 2：创建静态 Prompt 文件

在项目根目录创建：

```
.prompts/
└── system.md
```

内容先写最小版本（你可以后续自己调）：

```markdown
You are a helpful coding assistant embedded in a terminal UI.
You help the user with software engineering tasks in the current project.

Rules:
- Prefer editing existing files over creating new ones.
- Run tests or type-checking to verify changes when possible.
- Keep changes minimal; don't refactor unrelated code.
- Answer inline for explanations; create files only when asked to write/save/generate.
- Before reporting a task complete, verify it actually works.
```

**为什么放文件里而不是硬编码到 Rust 源码？**

1. **不用 recompile 就能调 Prompt** —— 改 `.prompts/system.md`，重启程序就生效
2. **多语言/多角色可扩展** —— 以后可以做 `.prompts/system-zh.md`、`.prompts/reviewer.md`
3. **版本控制友好** —— Prompt 的演进历史用 git 追踪，和代码变更解耦

---

## Step 3：实现动态上下文收集

新建文件：`src/prompt.rs`

定义一个结构体，负责收集"每次启动会变"的信息：

```rust
use std::path::Path;

pub struct PromptContext {
    pub cwd: String,              // 当前工作目录
    pub project_name: String,     // 从 Cargo.toml 读
    pub project_summary: String,  // 从 README.md 前 500 字读
    pub agents_md: Option<String>, // 项目级 AGENTS.md 内容（如果有）
}
```

实现两个构造路径：

```rust
const README_SUMMARY_MAX_CHARS: usize = 500; // ~125 tokens，控制 system prompt 长度

impl PromptContext {
    /// 从当前环境（文件系统）收集上下文。这是生产路径。
    pub fn from_environment() -> Self {
        let cwd = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| ".".into());

        let project_name = read_cargo_name()
            .or_else(|| Path::new(&cwd).file_name().and_then(|s| s.to_str()).map(String::from))
            .unwrap_or_else(|| "unknown".into());
        let project_summary = read_readme_summary(README_SUMMARY_MAX_CHARS);
        let agents_md = read_agents_md();

        Self {
            cwd,
            project_name,
            project_summary,
            agents_md,
        }
    }

    /// 直接构造（测试路径）。不需要 touching disk。
    pub fn new(cwd: &str, project_name: &str, project_summary: &str, agents_md: Option<&str>) -> Self {
        Self {
            cwd: cwd.into(),
            project_name: project_name.into(),
            project_summary: project_summary.into(),
            agents_md: agents_md.map(|s| s.into()),
        }
    }
}
```

三个辅助函数：

```rust
fn read_cargo_name() -> Option<String> {
    let content = std::fs::read_to_string("Cargo.toml").ok()?;
    content
        .lines()
        .find(|line| line.trim_start().starts_with("name"))
        .and_then(|line| line.split('=').nth(1))
        .map(|s| s.trim().trim_matches('"').to_string())
}

fn read_readme_summary(max_chars: usize) -> String {
    let content = match std::fs::read_to_string("README.md") {
        Ok(s) => s,
        Err(_) => return String::new(),
    };
    content.chars().take(max_chars).collect()
}

fn read_agents_md() -> Option<String> {
    std::fs::read_to_string("AGENTS.md").ok()
}
```

**设计意图**：
- `from_environment()` 把 I/O 命名在 API 里，诚实地告诉调用者"这个函数会读盘"
- `new()` 让测试可以直接构造假上下文，不用 mock 文件系统
- 用 `Option` / 空字符串做 fallback，防止某个文件不存在导致整个 Prompt 崩掉
- `README_SUMMARY_MAX_CHARS` 限制 README 长度，避免把几万字的文档全塞进 System Prompt（上下文窗口很贵的）
- `read_cargo_name` 是 **Rust 专用** 的——它只认识 `Cargo.toml`。真正的 Coding Agent 应该按检测到的项目类型分发（`Cargo.toml` / `package.json` / `pyproject.toml`），这里先保持最小实现。

---

## Step 4：实现组装器

在 `src/prompt.rs` 里加 `build_system_message()`：

```rust
use crate::message::Message;

const DEFAULT_SYSTEM_PROMPT: &str = "You are a helpful coding assistant.";

/// 从磁盘加载静态 prompt。I/O 和格式化分离，方便测试。
fn load_static_prompt() -> String {
    std::fs::read_to_string(".prompts/system.md")
        .unwrap_or_else(|_| DEFAULT_SYSTEM_PROMPT.into())
}

/// 纯函数：把静态 prompt 和动态上下文组装成 system message。
/// 不包含任何 I/O，可以直接单元测试。
pub fn assemble_system_message(static_prompt: &str, ctx: &PromptContext) -> Message {
    const BOUNDARY: &str = "__SYSTEM_PROMPT_DYNAMIC_BOUNDARY__"; // 分界标记，详见下方说明

    let mut dynamic = format!(
        "# Environment\n\
         - Working directory: {}\n\
         - Project: {}\n",
        ctx.cwd, ctx.project_name
    );

    if !ctx.project_summary.is_empty() {
        dynamic.push_str(&format!(
            "\n# Project Summary\n{}\n",
            &ctx.project_summary
        ));
    }

    if let Some(ref agents) = ctx.agents_md {
        dynamic.push_str(&format!(
            "\n# Project Instructions (AGENTS.md)\n{}\n",
            agents
        ));
    }

    let full = format!(
        "{}\n\n{}\n\n{}",
        static_prompt.trim(),
        BOUNDARY,
        dynamic.trim()
    );

    Message::system(full)
}
```

**关于 `BOUNDARY`**：

这个标记本身对 LLM 没有语义，但它有三个作用：
1. **调试时一眼分清**哪里是静态、哪里是动态
2. **后续扩展**——如果以后你想做 Prompt 缓存，可以按这个标记切分
3. **教学价值**——让人类读者理解 Claude Code 的设计哲学

---

## Step 5：注入 API 调用

System Prompt 应该**只组装一次**（程序启动时），而不是每次调用 LLM 都重新读盘。否则：
- 每轮对话触发 4 次文件读取（cwd + Cargo.toml + README.md + AGENTS.md）
- 如果中途改了 `AGENTS.md`，同一对话的 system prompt 会漂移，破坏上下文一致性
- `llm.rs`（HTTP 层）被迫依赖文件系统布局，违反依赖倒置

**正确做法**：在 `App::new()` 或 `Chat::new()` 里组装好，把成品 `Message` 存起来，传给 `stream_chat`。

### 改 `llm.rs`：接收现成的 system message

```rust
pub async fn stream_chat(
    system: &Message,            // ← 由调用方传入，不是在这里组装
    messages: &[Message],        // ← 借用来避免克隆
    tx: UnboundedSender<Action>,
) -> color_eyre::Result<()> {
    let client = reqwest::Client::new();

    // 用 Message 自己的 Serialize 派生，不要手写 JSON
    let mut api_messages = vec![system];
    api_messages.extend(messages.iter());

    let body = serde_json::json!({
        "model": "gemma-4-31b",
        "messages": api_messages,
        "stream": true
    });

    // ... 下面的 HTTP 调用和 SSE 解析完全不变
}
```

### 改 `Chat::new()`：组装一次，存起来

```rust
use crate::prompt::{PromptContext, assemble_system_message, load_static_prompt};

pub struct Chat {
    command_tx: Option<UnboundedSender<Action>>,
    conversation: Conversation,
    input: Input,
    focused: bool,
    system_prompt: Message,  // ← 新增：启动时组装，之后不变
}

impl Chat {
    pub fn new() -> Self {
        let ctx = PromptContext::from_environment();
        let static_prompt = load_static_prompt();
        let system_prompt = assemble_system_message(&static_prompt, &ctx);

        Self {
            command_tx: None,
            conversation: Conversation::new(),
            input: Input::new(),
            focused: true,
            system_prompt,
        }
    }
}
```

### 改发送逻辑：把存好的 system prompt 传下去

在 `Chat::handle_key_event` 的 `Enter` 分支里：

```rust
if let Some(ref tx) = self.command_tx {
    let _ = tx.send(Action::SendMessage(
        self.system_prompt.clone(),           // ← 一起发过去
        self.conversation.messages().to_vec(),
    ));
}
```

`Action::SendMessage` 需要改成携带两个字段：

```rust
pub enum Action {
    // ...
    SendMessage(Message, Vec<Message>),  // (system, conversation_history)
    // ...
}
```

`App` 收到后拆开传给 `llm::stream_chat`：

```rust
Action::SendMessage(system, history) => {
    let tx = self.action_tx.clone();
    tokio::spawn(async move {
        let _ = llm::stream_chat(&system, &history, tx).await;
    });
}
```

**关键点**：
- `system` 消息放在 `messages` 数组的**第一位**
- 用 `Message` 的 `Serialize` 派生序列化，不要手写 `json!`——后者会和 struct 定义漂移
- `llm.rs` 只负责 HTTP 传输，**不知道** system prompt 从哪来、怎么组装的

---

## Step 6（可选）：TUI 里热加载

如果你不想每次改 Prompt 都重启程序，可以在 `Chat` 里加一条命令：

```rust
KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
    // Ctrl+R 重新加载 system prompt（调试用）
    let ctx = PromptContext::from_environment();
    let static_prompt = load_static_prompt();
    self.system_prompt = assemble_system_message(&static_prompt, &ctx);
    Ok(None)
}
```

最小版本可以先不做这个，**改 `.prompts/system.md` → 重启程序** 就足够了。

---

## 验证

改完后，启动程序，发一条消息，然后在 `Chat::new()` 里打印确认：

```rust
pub fn new() -> Self {
    let ctx = PromptContext::from_environment();
    let static_prompt = load_static_prompt();
    let system_prompt = assemble_system_message(&static_prompt, &ctx);

    tracing::info!("system prompt loaded:\n{}", system_prompt.content);

    Self {
        command_tx: None,
        conversation: Conversation::new(),
        input: Input::new(),
        focused: true,
        system_prompt,
    }
}
```

你应该能在日志里看到类似这样的结构：

```
You are a helpful coding assistant embedded in a terminal UI.
...

__SYSTEM_PROMPT_DYNAMIC_BOUNDARY__

# Environment
- Working directory: /home/you/projects/llm-tui
- Project: llm-tui

# Project Summary
# llm-tui

A terminal UI chat application...
```

如果看到了，说明 System Prompt 已经成功注入到 API 请求里了。

---

## 进阶方向（课后作业）

1. **多角色切换**：`.prompts/` 目录下放多个 md 文件，用户用 `/persona reviewer` 切换
2. **会话记忆**：把用户高频指令写入 `~/.llm-tui/memory.md`，动态区自动加载
3. **Token 预算**：给静态区和动态区分别设 `max_tokens`，超长的 README 自动截断或摘要
4. **A/B 测试**：同时维护 `system-v1.md` 和 `system-v2.md`，用环境变量切换对比效果
5. **Trait-based 上下文源（OCP 修复）**：

   当前每加一种上下文（比如"git branch"），要改 `PromptContext` 字段、`from_environment()`、`assemble_system_message()` 三处。更干净的做法：

   ```rust
   trait ContextSource {
       fn section_title(&self) -> &str;
       fn collect(&self) -> Option<String>;
   }

   struct GitContext;
   impl ContextSource for GitContext {
       fn section_title(&self) -> &str { "Git Status" }
       fn collect(&self) -> Option<String> {
           // 读 git branch / status
       }
   }

   // gather() 变成遍历 sources
   let sections: Vec<_> = sources
       .iter()
       .filter_map(|s| s.collect().map(|c| (s.section_title(), c)))
       .collect();
   ```

   新增一种上下文 = 新增一个 struct，**零处**修改现有代码。

---

*本教程只描述设计思路和代码结构，具体实现由读者自行完成。*
