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
pub fn system(content: impl Into<String>) -> Self {  // → 见笔记 [`Into<T>`](../notes/into-trait.md)
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
    pub project_name: String,     // 按优先级探测：Cargo.toml → package.json → pyproject.toml → 目录名
    pub project_type: String,     // "rust" | "node" | "python" | "unknown"
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
            .unwrap_or_else(|_| ".".into());  // → 见笔记 [`Result::map + unwrap_or_else`](../notes/result-map-unwrap.md)

        let project_name = detect_project_name();
        let project_type = detect_project_type();
        let project_summary = read_readme_summary(README_SUMMARY_MAX_CHARS);
        let agents_md = read_agents_md();

        Self {
            cwd,
            project_name,
            project_type,
            project_summary,
            agents_md,
        }
    }

    /// 直接构造（测试路径）。不需要 touching disk。
    pub fn new(cwd: &str, project_name: &str, project_type: &str, project_summary: &str, agents_md: Option<&str>) -> Self {
        Self {
            cwd: cwd.into(),
            project_name: project_name.into(),
            project_type: project_type.into(),
            project_summary: project_summary.into(),
            agents_md: agents_md.map(|s| s.into()),
        }
    }
}
```

项目名探测函数（按优先级 fallback）：

```rust
fn read_cargo_name() -> Option<String> {
    let content = std::fs::read_to_string("Cargo.toml").ok()?;
    content
        .lines()
        .find(|line| line.trim_start().starts_with("name"))
        .and_then(|line| line.split('=').nth(1))
        .map(|s| s.trim().trim_matches('"').to_string())
}

fn read_package_json_name() -> Option<String> {
    let content = std::fs::read_to_string("package.json").ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    v.get("name")?.as_str().map(String::from)
}

fn read_pyproject_name() -> Option<String> {
    let content = std::fs::read_to_string("pyproject.toml").ok()?;
    content
        .lines()
        .find(|line| line.trim_start().starts_with("name"))
        .and_then(|line| line.split('=').nth(1))
        .map(|s| s.trim().trim_matches('"').to_string())
}

fn read_dir_basename() -> Option<String> {
    std::env::current_dir().ok()?
        .file_name()?
        .to_str()
        .map(String::from)
}

/// 按优先级探测项目名：Rust → Node/TS → Python → 目录名
fn detect_project_name() -> String {
    read_cargo_name()
        .or_else(read_package_json_name)
        .or_else(read_pyproject_name)
        .or_else(read_dir_basename)
        .unwrap_or_else(|| "unknown".into())
}

/// 根据哪个 manifest 存在来判断项目类型
fn detect_project_type() -> String {
    if std::path::Path::new("Cargo.toml").exists() {
        "rust".into()
    } else if std::path::Path::new("package.json").exists() {
        "node".into()
    } else if std::path::Path::new("pyproject.toml").exists() {
        "python".into()
    } else {
        "unknown".into()
    }
}
```

其他辅助函数：

```rust
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
- `detect_project_name()` 的 `or_else` 链是**策略模式的最小形式**——不引入 trait 抽象，但能处理 Rust / Node / Python 三种项目。后续如果要支持 Go、Java 等，继续往链后面加即可。

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
         - Project: {}\n\
         - Project type: {}\n",
        ctx.cwd, ctx.project_name, ctx.project_type
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

## Step 5：注入 API 调用 —— 以及为什么 `llm.rs` 不该做这件事

### 先回答一个架构问题：System Prompt 该归谁管？

你可能在想：既然 `PromptContext` 能产出 `Message`，为什么不直接在 `llm.rs` 里链式调用？

```rust
// ❌ 不要这样做
let system = PromptContext::from_environment().system_prompt();
```

用 **单一职责原则（SRP）** 来看，答案很清楚。**SRP 问的是：这个模块有几个"变更理由"？**

假设 `llm.rs` 负责组装 prompt：

| 谁要求变更 | 变更内容 | 是否 touch `llm.rs` |
|-----------|---------|-------------------|
| 后端升级 | llama.cpp 响应格式变了 | ✅ |
| 换网络库 | reqwest 换成 hyper | ✅ |
| 产品经理 | 用户想自定义 prompt 模板 | ✅ |
| 新功能 | 支持 Python 项目（加 pyproject.toml） | ✅ |
| 新功能 | 动态区加入 git branch | ✅ |
| 体验优化 | Prompt 太长，要截断 | ✅ |

**6 个不同的理由**，来自 6 个不同的人，都会修改同一个文件。这是 SRP violation。

更严重的是**依赖方向**：

```
llm.rs ──→ prompt.rs ──→ 文件系统（Cargo.toml、README.md）
```

HTTP 传输层（最底层的基础设施）被迫耦合文件系统。测试 `stream_chat` 时，要么真的读盘，要么 mock 整个文件系统——而测试 HTTP 本来只需要 mock 一个 TCP 连接。

**正确做法**：把 prompt 组装从 `llm.rs` 抽出来，放到 `Chat::new()`（或 `App::new()`）。`llm.rs` 只收成品 `Message`，不关心它从哪来。

| 模块 | 职责 | 变更理由 |
|------|------|---------|
| `llm.rs` | HTTP 传输：序列化、POST、SSE 解析 | API 格式、网络库 |
| `prompt.rs` / `Chat` | Prompt 组装：读文件、拼接、格式化 | 模板内容、项目类型、上下文源 |

判断口诀： **`llm.rs` 是管道，不是厨子。** 厨子决定做什么菜，管道只负责把菜端到餐桌。

---

### 为什么 System Prompt 只该组装一次？

- 每轮对话都重新读盘 = 4 次文件 IO（cwd + Cargo.toml + README.md + AGENTS.md）
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

改完后，在 `Chat::new()` 里加一行日志确认：

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

### 日志写在哪里？

TUI 程序会接管整个终端（alternate screen），所以 `tracing::info!()` 不能输出到 stdout——你会看不到。项目的 `src/logging.rs` 已经配置好把日志写到文件：

```rust
let directory = config::get_data_dir();  // → ~/.local/share/llm-tui/
let log_path = directory.join("llm-tui.log");
```

**查看日志的方法**：

```bash
# 方法 1：TUI 运行中，另开一个终端实时 tail
tail -f ~/.local/share/llm-tui/llm-tui.log

# 方法 2：TUI 退出后一次性查看
cat ~/.local/share/llm-tui/llm-tui.log
```

### 为什么可能看不到 INFO 日志？

如果日志文件里只有 ERROR，没有 INFO，检查 `src/logging.rs` 的 `EnvFilter` 配置。正确的写法：

```rust
let env_filter = EnvFilter::builder()
    .with_default_directive(tracing::Level::INFO.into())
    .from_env_lossy();
```

`from_env_lossy()` 会读 `RUST_LOG` 环境变量，如果没设置就默认用 `INFO`。如果用了复杂的 `try_from_env().or_else(...)` 组合，容易因为 filter 为空而回退到 `ERROR` 级别。

### 预期输出

你应该能在日志里看到类似这样的结构：

```
2026-04-28T04:22:45.640825Z  INFO src/components/chat/mod.rs:32: system prompt loaded: You are a helpful coding assistant...

__SYSTEM_PROMPT_DYNAMIC_BOUNDARY__

# Environment
- Working directory: /home/you/projects/llm-tui
- Project: llm-tui
- Project type: rust

# Project Summary
# llm-tui
...
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
