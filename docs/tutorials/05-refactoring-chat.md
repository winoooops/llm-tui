# Tutorial 05：重构 Chat 组件 —— 从浅模块到深模块

> **目标**：把 280+ 行的 `chat.rs` 拆分为职责清晰的深模块（deep modules）。
> **前置要求**：已完成 [Tutorial 04](04-cursor-and-multiline.md)。
> **参考书籍**：《A Philosophy of Software Design》（John Ousterhout）

---

## 当前问题诊断

打开 `src/components/chat.rs`，它目前长达 280+ 行，混合了至少 3 个完全不同的领域：

| 领域 | 涉及的字段和方法 | 行数估算 |
|------|----------------|---------|
| **对话** | `messages`, `conversation`, `current_ai_response`, `waiting_for_response`, `tick_count`, `start_waiting`, `stop_waiting`, `append_ai_text` | ~60 |
| **文本输入** | `input`, `cursor_position`, `move_cursor_left/right`, `enter_char`, `delete_char`, `enter_newline`, `build_input_text` | ~90 |
| **编排/粘合** | `handle_key_event`（分发按键到输入或发送）, `update`（分发 Action 到对话）, `draw`（切分布局并渲染两者）, `register_action_handler` | ~100 |

这违反了 SOLID 的 **单一职责原则（Single Responsibility）**：`Chat` 现在有"对话变更"、"文本编辑行为"、"组件事件编排"三个完全不同的变更理由。

更深层的问题来自 **信息泄露（Information Leakage）**：

```rust
// Chat 的调用方（App）和读者，被迫知道这些内部细节：
pub struct Chat {
    messages: Vec<String>,       // 你知道这是带 "You:" 前缀的显示文本
    conversation: Vec<Message>,  // 你知道这是给 API 用的结构化历史
    current_ai_response: String, // 你知道这是流式缓冲用的临时字段
    input: String,
    cursor_position: usize,      // 你知道这是字节位置不是字符位置
    // ...
}
```

调用方本不该关心 `"You:"` 前缀在哪加、`cursor_position` 是字节还是字符、`current_ai_response` 什么时候归档到 `conversation`。这些实现细节被暴露在 `Chat` 的字段层面，任何人读代码时都要同时理解三个子系统的内部机制。

---

## 书里怎么说：几个帮你做判断的概念

在动手拆代码之前，先建立几个来自《A Philosophy of Software Design》的心智模型。它们是你做重构决策时的"透镜"。

### 1. 战略式编程 vs 战术式编程（Working Code Isn't Enough）

> "Tactical programming is about getting something working quickly. Strategic programming is about producing a great design."

当前 `chat.rs` 的每一次功能叠加（Tutorial 01 加输入框 → 03 加记忆 → 04 加光标）都是**战术式**的：

- "我需要 cursor，加字段"
- "我需要对话历史，加字段"
- "我需要 spinner，加字段"

每次改动都"能跑"，但没有人停下来问：**这个模块的职责边界在哪里？**

战术式编程的隐性成本是**认知负荷的复利**。第 1 个功能你花 10 分钟理解，第 5 个功能你花 2 小时，因为所有细节摊在同一个平面上。战略式编程要求你在第 3 个功能时就停下来重构——不是因为它坏了，而是因为**它还跑得动的时候最容易拆**。

### 2. 信息隐藏是降低复杂度的唯一武器

> "Information hiding is the most important technique for achieving deep modules."

复杂度来自于**依赖关系**的数量。当前 `Chat` 暴露 8 个字段，每个字段都可能被 `handle_key_event`、`update`、`draw` 读取或修改。理论上的状态组合是 `2^8 = 256` 种（实际没那么多，但趋势如此）。

重构后 `Chat` 只剩 3 个字段，其中 2 个是深模块（`conversation`, `input`）。这些模块把内部状态**藏起来了**，`Chat` 无法直接修改 `cursor_position` 或 `current_ai_response`——它只能通过方法调用间接影响。状态组合从 256 降到了个位数。

**信息隐藏的本质不是"不让别人看"，而是"让别人不需要看"。**

### 3. 浅模块的陷阱

> "Shallow modules are ones whose interfaces are complicated relative to the functionality they provide."

判断一个模块是深是浅，不是看它有多少行代码，而是看**"接口宽度 / 内部功能量"**的比值：

| 模块 | 接口方法数 | 内部功能量 | 深度 |
|------|-----------|-----------|------|
| `std::Vec` | `push`, `pop`, `iter`… ~10 个常用 | 内存分配、扩容、realloc、迭代器、边界检查… | **深** ✅ |
| 当前 `Chat` | `new`, `move_cursor_left`, `append_ai_text`, `build_input_text`, `start_waiting`… 15+ 个 | 对话 + 文本编辑 + 编排，但混在一起没有抽象 | **浅** ❌ |

`Chat` 的问题不是"做了太多事"，而是**"做了太多事，却没有任何一道门把细节挡在外面"**。所有方法都摊在同一个 `impl` 块里，调用方和阅读方被迫同时理解全部。

### 4. 不同抽象层不应该混合

> "Each layer in a system should have a different abstraction from the layers above and below it."

当前 `draw` 方法里混着三层抽象：

```rust
fn draw(&mut self, frame: &mut Frame, area: Rect) {
    // 层 1: 空间布局（容器级决策）
    let chunks = Layout::default()...split(area);

    // 层 2: 内容生成（领域级决策）
    let mut lines: Vec<Line> = self.messages.iter()...collect();
    if self.is_waiting() {
        lines.push(Line::from(format!("AI: {} thinking", ...)));
    }

    // 层 3: 视觉渲染（控件级决策）
    let messages_widget = Paragraph::new(...)
        .block(Block::default().title("Chat")...)
        .wrap(Wrap { trim: true });
    frame.render_widget(messages_widget, chunks[0]);
}
```

层 1 和层 3 属于"UI 框架怎么摆"，层 2 属于"对话长什么样"。它们应该在不同的模块里。

重构后：
- `Conversation::render()` 只做层 2（生成内容）
- `Chat::draw()` 只做层 1 和层 3（分配空间、套边框、渲染）

---

## 设计目标：深模块 + 小接口

> "The best modules are deep: they have a lot of functionality hidden behind a simple interface."
> — A Philosophy of Software Design, Chapter 4

我们要把 `Chat` 从一个**浅而宽**的模块：

```
┌─────────────────────────────┐
│           Chat              │   ← 1 个模块，接口很宽（十几个 public 方法/字段）
│  ┌─────────┐ ┌───────────┐ │
│  │Conversation│ │ Input Box │ │   ← 内部职责混在一起，没有边界
│  └─────────┘ └───────────┘ │
└─────────────────────────────┘
```

变成**深而窄**的层级：

```
┌─────────────────────────────┐
│           Chat              │   ← 变薄：只负责"把事件路由给对的人"
│        （thin shell）       │
└─────────────────────────────┘
         ┌───────┴───────┐
    ┌────┐             ┌────┐
    │Conversation│    │InputBox │   ← 两个深模块，各自隐藏复杂实现
    │  （deep）    │    │（deep） │
    └─────────────┘    └────────┘
```

### 深模块的定义

| | 浅模块（Shallow） | 深模块（Deep） |
|--|------------------|---------------|
| **接口** | 宽（很多 public 方法/字段） | 窄（少量 public 方法） |
| **内部** | 简单（没做多少事） | 复杂（做了很多事） |
| **成本** | 调用方要理解很多 | 调用方只需理解一点 |
| **例子** | 当前的 `Chat` | `std::Vec`（几十种内部优化，接口只有 `push`, `pop`, `iter`…） |

---

## 目标架构

### 模块 1：Conversation

负责所有与"对话历史"相关的事：
- 显示消息（带 `"You:"` / `"AI:"` 前缀）
- API 对话历史（`Vec<Message>`）
- 流式回复缓冲与归档
- 等待状态与 spinner 动画

**对外接口（约 6 个方法）：**

```rust
impl Conversation {
    pub fn new() -> Self;
    pub fn push_user(&mut self, text: &str) -> Message;    // 添加用户消息
    pub fn start_response(&mut self);                       // LLM 开始回复
    pub fn append_chunk(&mut self, chunk: &str);            // 收到流式 chunk
    pub fn finish_response(&mut self);                      // 流结束，归档到对话
    pub fn tick(&mut self);                                 // 更新 spinner 帧
    pub fn render(&self) -> Text<'static>;                  // 渲染为 Ratatui Text
}
```

**内部隐藏的细节：**
- `display_messages` 和 `conversation` 的对应关系
- `current_response` 临时缓冲的存在
- `"AI: "` 前缀何时添加、何时追加
- spinner 帧的计算

### 模块 2：InputBox

负责所有与"文本输入"相关的事：
- 多行文本内容
- 光标移动（左/右）
- 字符插入/删除
- 换行插入
- 视觉光标渲染（黄底高亮 / `▋`）

**对外接口（约 8 个方法）：**

```rust
impl InputBox {
    pub fn new() -> Self;
    pub fn text(&self) -> &str;
    pub fn is_empty(&self) -> bool;
    pub fn clear(&mut self);
    pub fn move_cursor_left(&mut self);
    pub fn move_cursor_right(&mut self);
    pub fn insert_char(&mut self, c: char);
    pub fn delete_char(&mut self);
    pub fn insert_newline(&mut self);
    pub fn render(&self, focused: bool) -> Text<'static>;
}
```

**内部隐藏的细节：**
- `cursor_position` 是字节位置
- `char_indices()` 的 UTF-8 边界处理
- `split('\n')` 与空行处理
- `Span::styled` 的光标渲染逻辑

### 为什么这样分是"按功能分"而不是"按流程分"？

> **Red Flag: Temporal Decomposition**
> "Decomposing a system based on the order in which operations occur is usually a bad idea."

一个常见的错误拆法是：

```
❌ handle_key_event.rs   ← 把所有按键处理拿出来
❌ update.rs             ← 把所有 Action 处理拿出来
❌ draw.rs               ← 把所有渲染拿出来
```

这叫**按时间/流程分解（Temporal Decomposition）**。它的坏处是：`handle_key_event` 里既有输入框的逻辑又有对话消息的逻辑，改输入框时你还是要 touch `handle_key_event.rs`。

正确的拆法是**按功能分解（Functional Decomposition）**：

```
✅ input.rs      ← 所有"文本输入"相关的东西：按键处理 + 渲染 + 光标
✅ conversation.rs    ← 所有"对话"相关的东西：流式接收 + 归档 + 渲染
```

输入框的按键处理和输入框的渲染是**同一抽象层**的，它们应该待在一起。流程（event → update → draw）只是调用顺序，不是模块边界。

### 模块 3：Chat（变薄后的外壳）

只负责一件事：**收到事件/Action 时，判断该交给谁处理**。

```rust
pub struct Chat {
    command_tx: Option<UnboundedSender<Action>>,
    conversation: Conversation,   // ← 深模块
    input: InputBox,           // ← 深模块
    focused: bool,
}
```

`Chat` 的 `impl Component for Chat` 变成薄薄一层：

```rust
fn handle_key_event(&mut self, key: KeyEvent) -> ... {
    match key.code {
        KeyCode::Enter if !self.input.is_empty() => {
            let text = self.input.text().to_string();
            self.conversation.push_user(&text);        // ← 委托给对话
            self.input.clear();                    // ← 委托给输入框
            // 发送 Action::SendMessage...
        }
        KeyCode::Char(c) => {
            self.input.insert_char(c);             // ← 委托给输入框
        }
        // ...其他按键同理
    }
}

fn update(&mut self, action: Action) -> ... {
    match action {
        Action::ReceiveChunk(chunk) => {
            self.conversation.append_chunk(&chunk);     // ← 委托给对话
        }
        Action::StreamEnd => {
            self.conversation.finish_response();        // ← 委托给对话
        }
        Action::Tick => {
            self.conversation.tick();                   // ← 委托给对话
        }
        // ...
    }
}

fn draw(&mut self, frame: &mut Frame, area: Rect) -> ... {
    let chunks = Layout::default()...split(area);

    // 对话区域渲染
    let conversation_widget = Paragraph::new(self.conversation.render())...;
    frame.render_widget(conversation_widget, chunks[0]);

    // 输入区域渲染
    let input_widget = Paragraph::new(self.input.render(self.focused))...;
    frame.render_widget(input_widget, chunks[1]);
}
```

---

## Step-by-Step 重构指南

### Step 0：创建模块目录

在 `src/components/` 下创建：

```
src/components/
├── chat/
│   ├── mod.rs          # Chat 组件（变薄后的外壳）
│   ├── conversation.rs      # Conversation 深模块
│   └── input.rs        # InputBox 深模块
├── chat.rs             # 删除（内容拆分到 chat/mod.rs）
├── fps.rs
├── home.rs
└── mod.rs
```

**为什么用目录模块？**

Rust 允许把一个模块拆成目录：`chat/mod.rs` 是入口，`chat/conversation.rs` 和 `chat/input.rs` 是子模块。这遵循**文件即边界**的直觉——你看到 `chat/` 目录就知道 Chat 组件由多个文件协作实现。

在 `src/components/mod.rs` 里修改：

```rust
pub mod chat;  // 之前是 pub mod chat; 指向 chat.rs
// 现在指向 chat/mod.rs
```

### Step 1：迁移 InputBox

新建 `src/components/chat/input.rs`：

```rust
use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
};

pub struct InputBox {
    text: String,
    cursor: usize,  // 字节位置，外部不需要知道
}

impl InputBox {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
        }
    }

    pub fn text(&self) -> &str { &self.text }
    pub fn is_empty(&self) -> bool { self.text.is_empty() }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    pub fn move_cursor_left(&mut self) {
        let before = &self.text[..self.cursor];
        if let Some((idx, _)) = before.char_indices().last() {
            self.cursor = idx;
        }
    }

    pub fn move_cursor_right(&mut self) {
        let after = &self.text[self.cursor..];
        if let Some((idx, c)) = after.char_indices().next() {
            self.cursor += idx + c.len_utf8();
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.text.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    pub fn delete_char(&mut self) {
        let before = &self.text[..self.cursor];
        if let Some((idx, c)) = before.char_indices().last() {
            self.text.remove(idx);
            self.cursor -= c.len_utf8();
        }
    }

    pub fn insert_newline(&mut self) {
        self.text.insert(self.cursor, '\n');
        self.cursor += 1;
    }

    pub fn render(&self, focused: bool) -> Text<'static> {
        if !focused {
            return Text::from(self.text.clone());
        }
        // ...原来的 build_input_text 逻辑搬进来...
    }
}
```

**关键设计决策**：

- `cursor` 字段是 `pub(crate)` 或私有，**不暴露给 Chat**。Chat 只能通过方法操作输入框。
- `render` 接收 `focused: bool` 作为参数，而不是让 InputBox 自己存储 focus 状态。为什么？因为 focus 是**容器级**概念（谁获得焦点），不是输入框自己的属性。这个设计让 InputBox 更纯粹。

### Step 2：迁移 Conversation

新建 `src/components/chat/conversation.rs`：

```rust
use crate::message::Message;
use crate::utils;
use ratatui::{
    style::Style,
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub struct Conversation {
    display: Vec<String>,
    conversation: Vec<Message>,
    current_response: String,
    waiting: bool,
    tick: u8,
}

impl Conversation {
    pub fn new() -> Self {
        Self {
            display: Vec::new(),
            conversation: Vec::new(),
            current_response: String::new(),
            waiting: false,
            tick: 0,
        }
    }

    pub fn push_user(&mut self, text: &str) -> Message {
        self.display.push(format!("You: {}", text));
        let msg = Message::user(text);
        self.conversation.push(msg.clone());
        msg
    }

    pub fn start_response(&mut self) {
        self.waiting = true;
    }

    pub fn append_chunk(&mut self, chunk: &str) {
        self.waiting = false;
        self.current_response.push_str(chunk);

        // 更新显示：追加到同一条 "AI: ..." 消息
        if let Some(last) = self.display.last_mut()
            && last.starts_with("AI: ")
        {
            last.push_str(chunk);
        } else {
            self.display.push(format!("AI: {}", chunk));
        }
    }

    pub fn finish_response(&mut self) {
        if !self.current_response.is_empty() {
            self.conversation.push(Message::assistant(&self.current_response));
            self.current_response.clear();
        }
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    pub fn is_waiting(&self) -> bool {
        self.waiting
    }

    pub fn messages(&self) -> &[Message] {
        &self.conversation
    }

    pub fn render(&self) -> Text<'static> {
        let mut lines: Vec<Line> = self
            .display
            .iter()
            .map(|m| Line::from(m.as_str()))
            .collect();

        if self.waiting {
            lines.push(Line::from(format!(
                "AI: {} thinking",
                utils::spinner_frame(self.tick as usize)
            )));
        }

        Text::from(lines)
    }
}
```

**关键设计决策**：

- `display` 和 `conversation` 的对应关系被完全隐藏。调用方只调用 `push_user()` 和 `append_chunk()`，不需要知道有两套数据在并行维护。
- `current_response` 临时缓冲的存在对外部完全透明。外部调用 `finish_response()` 时，缓冲被归档到 `conversation`，但外部不需要关心"归档"这个词——它只知道"流结束了"。
- `render()` 返回 `Text<'static>` 而不是直接操作 `Frame`。为什么？因为渲染方式（边框、颜色、Wrap 等）是容器（Chat）的决定，不是对话模块的决定。Conversation 只负责"生成内容"，不负责"怎么框起来"。

### Step 3：重写 Chat 外壳

新建 `src/components/chat/mod.rs`：

```rust
pub mod conversation;
pub mod input;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::components::Component;

use conversation::Conversation;
use input::InputBox;

pub struct Chat {
    command_tx: Option<UnboundedSender<Action>>,
    conversation: Conversation,
    input: InputBox,
    focused: bool,
}

impl Chat {
    pub fn new() -> Self {
        Self {
            command_tx: None,
            conversation: Conversation::new(),
            input: InputBox::new(),
            focused: true,
        }
    }
}

impl Component for Chat {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> color_eyre::Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        match key.code {
            KeyCode::Enter if key.modifiers.is_empty() => {
                if !self.input.is_empty() {
                    let text = self.input.text().to_string();
                    self.conversation.push_user(&text);
                    self.input.clear();
                    self.conversation.start_response();

                    if let Some(ref tx) = self.command_tx {
                        let _ = tx.send(Action::SendMessage(self.conversation.messages().to_vec()));
                    }
                }
                Ok(None)
            }
            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.insert_newline();
                Ok(None)
            }
            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.insert_newline();
                Ok(None)
            }
            KeyCode::Left => {
                self.input.move_cursor_left();
                Ok(None)
            }
            KeyCode::Right => {
                self.input.move_cursor_right();
                Ok(None)
            }
            KeyCode::Backspace => {
                self.input.delete_char();
                Ok(None)
            }
            KeyCode::Char(c) => {
                self.input.insert_char(c);
                Ok(None)
            }
            KeyCode::Esc => {
                if let Some(ref tx) = self.command_tx {
                    let _ = tx.send(Action::Quit);
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        match action {
            Action::Tick => {
                self.conversation.tick();
            }
            Action::ReceiveChunk(chunk) => {
                self.conversation.append_chunk(&chunk);
            }
            Action::StreamEnd => {
                self.conversation.finish_response();
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
            .split(area);

        let conversation_widget = Paragraph::new(self.conversation.render())
            .block(Block::default().title("Chat").borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        frame.render_widget(conversation_widget, chunks[0]);

        let input_widget = Paragraph::new(self.input.render(self.focused))
            .block(
                Block::default()
                    .title("Input (Enter=send, Ctrl+J=newline, Esc=quit)")
                    .borders(Borders::ALL)
                    .border_style(if self.focused {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    }),
            )
            .wrap(Wrap { trim: true });
        frame.render_widget(input_widget, chunks[1]);

        Ok(())
    }
}
```

**看看 `Chat` 变多薄了：**

- `handle_key_event`：只做**路由判断**（这是输入按键还是发送按键？），然后把工作委托出去
- `update`：只做**Action 分发**（这是对话相关还是输入相关？），然后委托出去
- `draw`：只做**空间分配**（上面 80% 给对话，下面 20% 给输入），然后委托出去

所有的实现细节都被压进了 `Conversation` 和 `InputBox` 内部。

### Step 4：更新注册

在 `src/components.rs` 里，确认 `chat` 模块的注册方式：

```rust
pub mod chat;
```

因为 `chat` 现在是目录（`chat/mod.rs`），Rust 会自动找到它。不需要改 `app.rs` 里的导入，因为 `chat::Chat` 的公开路径不变。

---

## 重构前后对比

| 维度 | 重构前 | 重构后 |
|------|--------|--------|
| **文件数** | 1 个（chat.rs, 283 行） | 3 个（mod.rs ~90 行, conversation.rs ~90 行, input.rs ~100 行） |
| **Chat 的字段数** | 8 个 | 3 个（`conversation`, `input`, `focused`） |
| **Chat 的方法数** | 15+ 个 | 4 个（Component trait 方法） |
| **暴露的内部状态** | `messages`, `conversation`, `current_ai_response`, `input`, `cursor_position`… | 无。所有状态都是子模块私有 |
| **修改影响范围** | 改输入逻辑可能影响对话显示 | 改 `InputBox` 内部不影响 `Conversation` |

---

## 概念检查清单

1. **当前 `Chat` 混合了哪几个不同领域的职责？**
2. **什么是"信息泄露"？`cursor_position` 作为 `Chat` 的 public 字段，泄露了什么信息？**
3. **为什么说 `InputBox` 不自己存储 `focused` 状态是更好的设计？**
4. **`Conversation::render()` 返回 `Text` 而不是直接操作 `Frame`，这个设计决策的依据是什么？**
5. **重构后，如果要在输入框里加入"撤销（Undo）"功能，你只需要修改哪个文件？**

---

## Bonus：比 SOLID 更重要的 —— 信息隐藏，以及什么时候不该拆

SOLID 的 **S（单一职责）** 告诉你"一个类只做一件事"，但它没说怎么做。

"A Philosophy of Software Design" 的 **信息隐藏（Information Hiding）** 告诉你具体怎么做：

> "Design modules so that their interfaces don't expose internal implementation details."

重构前，`Chat` 暴露了 `cursor_position: usize`。这个字段的含义（字节位置）、约束（必须是 char boundary）、甚至存在理由（为了 `char_indices()`），都暴露在调用方面前。

重构后，`InputBox` 完全隐藏了 `cursor` 字段。调用方只知道：`move_cursor_left()`, `insert_char()`, `render()`。至于里面是字节位置还是字符位置、用的是 `char_indices()` 还是 `grapheme clusters`，都是 `InputBox` 的私有实现。

**这就是深度（Depth）的来源：接口越薄，你能隐藏的实现细节就越多，模块就越深。**

---

### 反模式：为了拆而拆（Over-modularization）

> "If a module's interface is almost as complex as its implementation, then the module isn't providing much abstraction."

拆模块不是目的，**降低复杂度**才是。如果你拆完之后，调用方要通过 5 层间接才能做一件原本 1 行代码的事，那你就拆过头了。

比如，不要把 `InputBox` 再拆成：

```
❌ CursorManager    ← 只存 cursor_position
❌ TextBuffer       ← 只存 String
❌ KeyHandler       ← 只处理按键
```

这三个东西的接口加起来，复杂度跟实现差不多，属于**假抽象**。`InputBox` 作为一个整体才是深模块——它内部复杂（光标 + 文本 + 渲染），但对外只有 8 个方法。

### 反模式：穿堂风方法（Pass-through Methods）

> "A pass-through method is one that does nothing except pass its arguments to another method, usually with the same signature."

重构后的 `Chat` 应该避免出现这种情况：

```rust
// ❌ 不要这样：Chat 只是透传，没有增加任何价值
impl Chat {
    fn move_cursor_left(&mut self) {
        self.input.move_cursor_left();
    }
    fn insert_char(&mut self, c: char) {
        self.input.insert_char(c);
    }
}
```

这些方法是"穿堂风"——它们的存在让 `Chat` 的接口变宽了，却没有增加任何新语义。正确的做法是：**让调用方直接操作子模块**（`self.input.move_cursor_left()`），或者把这些方法限制为私有。

在我们的重构里，`Chat::handle_key_event` 直接调用 `self.input.move_cursor_left()`，不需要在 `Chat` 上再包一层 `move_cursor_left()`。这就是避免穿堂风。

### 什么时候停止？

一个实用的判断标准：**如果拆完之后，你无法用一句话说清楚某个模块是干嘛的，那它可能不该存在。**

- ✅ "`InputBox` 负责文本输入和光标管理" —— 一句话能说清
- ✅ "`Conversation` 负责对话历史和流式回复的归档" —— 一句话能说清
- ❌ "`CursorManager` 负责光标位置的增减" —— 太细了，没有独立存在的意义
