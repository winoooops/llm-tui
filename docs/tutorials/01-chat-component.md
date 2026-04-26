# Tutorial 01：第一个 Chat 组件

> **目标**：创建一个本地输入 + 显示的聊天界面组件。  
> **前置要求**：无（这是第一个教程）。

---

## Step A：做一个聊天组件（本地输入 + 显示）

### 目标
创建一个新组件，实现：
- 屏幕上方 80% 显示聊天记录
- 屏幕下方 20% 显示输入框
- 可以打字、退格、按回车"发送"
- 按 Esc 退出程序

### 开始之前：理解 `Component` trait

打开 `src/components.rs`，阅读里面的 `Component` trait 定义。

**trait（特质）** 是 Rust 的"接口/契约"。它的意思是："任何想成为 Component 的类型，必须实现这些方法"。

注意大部分方法都有**默认实现**（后面带 `{ ... }` 的）。唯一没有默认实现的是 `draw`。这意味着**每个组件必须自己知道怎么画自己**。

你会覆盖的关键方法：

| 方法 | 作用 | 有默认实现？ |
|------|------|-----------|
| `register_action_handler` | 保存给 App 发 Action 的通道 | 有 |
| `handle_key_event` | 响应键盘输入 | 有 |
| `update` | 响应系统中流动的 Action | 有 |
| `draw` | 把自己画到屏幕上 | **没有** |

---

### A1. 创建新文件

创建 `src/components/chat.rs`。这就是你的新组件。

---

### A2. 添加导入（use 语句）

在 `src/components/chat.rs` 最上方添加：

```rust
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::action::Action;
```

**为什么需要这些导入？**

- `crossterm::event` — crossterm 处理所有终端输入输出。`KeyCode` 告诉你按了哪个键（回车、字母 a、退格等）。`KeyEvent` 是完整的按键事件（包含修饰键如 Ctrl）。
- `ratatui::{...}` — UI 工具包。`Layout` 切分屏幕空间，`Block` 画边框，`Paragraph` 渲染文字，`Frame` 是你作画的画布。
- `UnboundedSender` — Tokio 的异步无界通道发送端。你用它来给 App 发 Action。
- `super::Component` — `super` 指"父模块"。`Component` trait 定义在 `src/components.rs` 里。
- `crate::action::Action` — `crate` 指整个项目的根。从 `src/action.rs` 导入 Action 枚举。

---

### A3. 定义结构体

在 `chat.rs` 中添加：

```rust
pub struct Chat {
    command_tx: Option<UnboundedSender<Action>>,
    messages: Vec<String>,
    input: String,
    focused: bool,
}
```

**为什么 `command_tx` 是 `Option<UnboundedSender<Action>>`？**

当你在 `App::new()` 里创建 `Chat` 实例时，App 还没有给你 action 通道。Rust 不允许字段"先不初始化"。所以先用 `None` 占位，之后 `register_action_handler` 会把真正的发送器传进来，存成 `Some(tx)`。

**为什么用 `Vec<String>`？**

`Vec` 是 Rust 的可变长数组。每条消息是一个 `String`。用户聊天时，你用 `push()` 把新消息追加进去。

**为什么有 `focused: bool`？**

现在永远为 `true`，但后面做多组件时（比如左边文件树、右边聊天），只有"获得焦点"的组件才应该接收键盘输入。

---

### A4. 添加构造函数

```rust
impl Chat {
    pub fn new() -> Self {
        Self {
            command_tx: None,
            messages: Vec::new(),
            input: String::new(),
            focused: true,
        }
    }
}
```

`impl Chat` 表示"为 `Chat` 类型实现方法"。`pub fn new() -> Self` 是 Rust 的惯用构造函数。

- `Self`（大写 S）是 `Chat` 的别名
- `Vec::new()` 创建空数组
- `String::new()` 创建空字符串

---

### A5. 实现 Component trait

```rust
impl Component for Chat {
```

这行代码的意思是："Chat 满足 Component 契约"。接下来你必须提供 trait 要求的方法。

#### A5a. `register_action_handler`

```rust
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> color_eyre::Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }
```

App 在启动时会调用这个方法，交给你一个 action 发送通道的克隆。你用 `Some(...)` 包起来存好。以后随时可以给 App 发消息。

- `&mut self` — 可变引用，可以修改 Chat 的内部状态
- `color_eyre::Result<()>` — 要么成功（`Ok(())`），要么返回错误。`()`（unit 类型）表示"成功时没有有意义的返回值"

#### A5b. `handle_key_event` — 键盘输入处理

```rust
    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        match key.code {
            KeyCode::Enter => {
                if !self.input.is_empty() {
                    self.messages.push(format!("You: {}", self.input));
                    self.input.clear();
                }
                Ok(None)
            }
            KeyCode::Backspace => {
                self.input.pop();
                Ok(None)
            }
            KeyCode::Char(c) => {
                self.input.push(c);
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
```

**`match` 是 Rust 最核心的语法**，相当于其他语言的 `switch`，但强大得多。`key.code` 是 `KeyCode` 枚举，我们匹配不同的变体：

| 按键 | 行为 |
|------|------|
| `Enter` | 如果输入不为空，格式化成 `"You: <文字>"`，push 到 `messages`，然后清空 `input` |
| `Backspace` | `String::pop()` 删除最后一个字符 |
| `Char(c)` | 把字符追加到输入末尾 |
| `Esc` | 通过 `command_tx` 发送 `Action::Quit` 给 App，让 App 退出 |
| `_` | 通配符，匹配上面没 catch 到的任何按键 |

注意 Esc 里的这段：
```rust
if let Some(ref tx) = self.command_tx {
    let _ = tx.send(Action::Quit);
}
```

- `if let Some(...)` = 如果 `command_tx` 是 `Some`，就取出来用
- `ref tx` = 只借用，不拿走所有权
- `let _ = ...` = 忽略 send 的返回值（即使发送失败也不处理）

**为什么返回 `Ok(None)`？**

返回类型是 `Result<Option<Action>>`。你可以返回 `Some(Action::Quit)`，App 也会收到。但你也可以自己通过 `command_tx` 发送。两种方式都行。通过通道发送更灵活，可以绕过正常流程。

#### A5c. `update`

```rust
    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        let _ = action;
        Ok(None)
    }
```

现在先留空。App 每收到一个 Action，都会调用所有组件的 `update`。`let _ = action;` 是为了告诉编译器"我知道这个参数存在，但暂时不用"，避免 unused 警告。

后面 Step B 接 LLM 时，我们会在这里处理 `Action::ReceiveChunk`，把流式回复追加到聊天记录。

#### A5d. `draw` — 渲染（Ratatui 的核心）

```rust
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        // 1. 把区域垂直切成两块
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
            .split(area);

        let messages_area = chunks[0];
        let input_area = chunks[1];

        // 2. 构建消息区域的控件
        let messages_text = Text::from(
            self.messages
                .iter()
                .map(|m| Line::from(m.as_str()))
                .collect::<Vec<_>>(),
        );
        let messages_widget = Paragraph::new(messages_text)
            .block(Block::default().title("Chat").borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        frame.render_widget(messages_widget, messages_area);

        // 3. 构建输入区域的控件
        let input_widget = Paragraph::new(self.input.as_str())
            .block(
                Block::default()
                    .title("Input (Enter to send, Esc to quit)")
                    .borders(Borders::ALL)
                    .border_style(if self.focused {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    }),
            )
            .wrap(Wrap { trim: true });
        frame.render_widget(input_widget, input_area);

        Ok(())
    }
```

**理解 Ratatui 的渲染原理：**

Ratatui 是**即时模式（Immediate Mode）** UI 库。每一帧（每秒 60 次），你都要**从头重新画一遍**。没有"更新这个标签"的概念——每次就全部重新画。

**布局系统：**

```rust
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
    .split(area);
```

这相当于 CSS 的 Flexbox：
- `direction: vertical` = 垂直堆叠子元素
- `constraints` = 子元素的尺寸规则
- `split(area)` = 按照规则把 `area` 切成若干块

`chunks` 是一个 `Vec<Rect>`。`chunks[0]` 是上面 80%，`chunks[1]` 是下面 20%。

```
┌─────────────────────┐  ↑
│                     │  │
│    messages_area    │  │ 80%
│                     │  │
├─────────────────────┤  ↓
│                     │  │
│     input_area      │  │ 20%
│                     │  ↓
└─────────────────────┘
```

**Text → Paragraph → render_widget 的层次：**

Ratatui 有一个层次结构：
1. `String` / `&str` — 原始 Rust 字符串
2. `Line` — 一行带样式的文字
3. `Text` — 多行 Line 的集合
4. `Paragraph` — 可渲染的控件，包裹 Text，带边框、样式、自动换行
5. `frame.render_widget(...)` — 真正把它画到终端缓冲区

**`Text::from(...)` 链式转换：**

```rust
self.messages
    .iter()                              // 遍历 Vec<String>
    .map(|m| Line::from(m.as_str()))     // 每个 String 转成 Line
    .collect::<Vec<_>>()                 // 收集成 Vec<Line>
```

`.collect::<Vec<_>>()` 是必要的，因为 `Text::from` 需要 `Vec<Line>`，但 `.map()` 返回的是迭代器。`collect()` 把迭代器变成集合。`::<Vec<_>>` 告诉 Rust 要变成什么集合类型（元素类型让编译器推断）。

**边框样式：**

```rust
.border_style(if self.focused {
    Style::default().fg(Color::Yellow)
} else {
    Style::default()
})
```

在 Rust 中，`if` 是一个**表达式**——它可以返回值。这相当于其他语言的三目运算符。

---

### A6. 注册组件

编译器已经知道 `chat.rs` 存在了，但 App 还不知道加载它。你需要在**两个地方**注册。

**文件 1：`src/components.rs`**

在顶部其他 `pub mod` 旁边添加：

```rust
pub mod chat;
```

这告诉 Rust："有一个叫 `chat` 的模块，对应的文件是 `chat.rs`。

**文件 2：`src/app.rs`**

先加导入。找到这行：

```rust
    components::{Component, fps::FpsCounter, home::Home},
```

改成：

```rust
    components::{Component, chat::Chat, fps::FpsCounter, home::Home},
```

然后找到 `App::new` 里的这行：

```rust
components: vec![Box::new(Home::new()), Box::new(FpsCounter::default())],
```

改成：

```rust
components: vec![Box::new(Home::new()), Box::new(FpsCounter::default()), Box::new(Chat::new())],
```

**为什么用 `Box::new(...)`？**

`components` 字段的类型是 `Vec<Box<dyn Component>>`。`dyn Component` 表示"任何实现了 Component 的类型"。不同组件在内存中占的大小不一样，Rust 无法把它们直接存在 Vec 里。`Box` 把每个组件放到堆上，Vec 里只存指针。这叫做**trait object（特质对象）**。

---

### A7. 编译并运行

```bash
cargo build
cargo run
```

你应该看到：
- 上方标题为 "Chat" 的框（空白，因为还没消息）
- 下方标题为 "Input" 的框（黄色边框）
- 打字会出现在输入框
- 按回车，文字会出现在上面的 Chat 框
- 按 Esc 退出程序

---

### A7b. 加餐：`color_eyre::Result<()>` 和 `Ok(())` 是什么？

你在教程里频繁看到这样的返回类型：

```rust
fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> color_eyre::Result<()> {
    self.command_tx = Some(tx);
    Ok(())
}
```

#### `color_eyre::Result<T>` 是什么？

`color_eyre` 是一个 Rust 错误处理库。它给标准库的 `Result` 做了一层包装：

```rust
// color_eyre 定义的类型别名：
type Result<T> = std::result::Result<T, color_eyre::Report>;
```

也就是说，`color_eyre::Result<()>` **就是** `Result<(), color_eyre::Report>`。

**为什么用它而不是普通的 `Result`？**

| 特性 | `std::result::Result<T, E>` | `color_eyre::Result<T>` |
|------|---------------------------|------------------------|
| 错误类型 | 你必须指定一种 `E` | 统一的 `Report`，任何错误都能装进去 |
| 错误显示 | 默认 `Debug` 输出 | 带颜色、带上下文、带完整 backtrace |
| `?` 自动转换 | 需要实现 `From<E>` | **任何错误都能直接用 `?`**，无需额外转换 |

**简单比喻：**

- 普通 `Result<T, E>` 像是一个"只能装苹果的错误篮子"
- `color_eyre::Result<T>` 像是一个"万能错误篮子"，苹果、橘子、香蕉（任何错误）都能丢进去，而且打印出来时还会自动标红高亮

#### `Ok(())` 是什么意思？

Rust 里 `()` 读作 **unit**（单元类型），表示"什么都没有"。它类似于其他语言的 `void`，但 `()` 是一个**真正的类型和值**。

```rust
fn do_something() -> color_eyre::Result<()> {
    // 做一堆事情...
    Ok(())   // "成功完成，但没有什么要返回的"
}
```

| 写法 | 含义 |
|------|------|
| `Ok(())` | 成功，返回值是 unit（空） |
| `Ok(value)` | 成功，返回值是 `value` |
| `Err(e)` | 失败，返回错误 `e` |

**为什么末尾必须写 `Ok(())`？**

因为函数签名承诺了返回 `color_eyre::Result<()>`。Rust 是表达式语言，函数体最后一个表达式的值就是返回值。如果你不写 `Ok(())`，函数就没有返回值，编译器会报错：

```
error[E0308]: mismatched types
  --> expected enum `Result`, found `()`
```

#### `?` 运算符的魔法

`color_eyre` 最方便的地方是配合 `?` 使用：

```rust
fn some_function() -> color_eyre::Result<()> {
    let file = std::fs::read_to_string("config.json")?;  // 如果失败，自动返回 Err
    let config = serde_json::from_str(&file)?;            // 如果失败，自动返回 Err
    // ...
    Ok(())
}
```

无论 `read_to_string` 返回 `std::io::Error`，还是 `from_str` 返回 `serde_json::Error`，`?` 都会**自动把它们转换成 `color_eyre::Report`** 然后返回。你不用写任何 `.map_err()`。

---

## Step A 概念检查清单

进入 Step B 之前，确认你能回答这些问题：

1. **为什么 `command_tx` 是 `Option`？**
2. **`match key.code` 做了什么？**
3. **`Text`、`Line`、`Paragraph` 三者有什么区别？**
4. **`Layout::split(area)` 返回什么，怎么使用？**
5. **为什么需要 `Box::new(Chat::new())` 而不是直接用 `Chat::new()`？**

---
