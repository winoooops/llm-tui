# Tutorial 04：输入框光标移动与多行输入

> **目标**：让输入框支持左右光标移动、在行内任意位置插入/删除字符，以及 `Ctrl+J` / `Shift+Enter` 换行。
> **前置要求**：已完成 [Tutorial 03](03-memory-context.md)。

---

## 问题：现在的输入框为什么只能"尾部操作"？

打开 `src/components/chat.rs`，看当前的按键处理：

```rust
KeyCode::Char(c) => {
    self.input.push(c);      // ← 永远追加到末尾
    Ok(None)
}
KeyCode::Backspace => {
    self.input.pop();        // ← 永远删除最后一个字符
    Ok(None)
}
```

`String::push` 和 `String::pop` 只操作**字符串尾部**。用户按左箭头、想在中间改一个字，完全做不到。

要解决这个问题，我们需要引入一个**光标位置（cursor position）**概念：

```
H e l l o
0 1 2 3 4 5  ← cursor_position 是一个 usize，表示"光标在字符串的哪个字节位置"
```

---

## 核心设计

### 光标位置用什么类型？

`usize` —— 但注意：**它是字节偏移量，不是字符个数**。

Rust 的 `String` 是 UTF-8 编码。一个英文字母占 1 字节，一个中文汉字占 3 字节，一个 emoji 占 4 字节。

```rust
let s = "你好";        // 6 个字节
let c = "🎉";          // 4 个字节
```

`String::insert(idx, c)` 的 `idx` 是**字节位置**。所以我们让 `cursor_position` 也跟踪字节位置，这样插入/删除可以直接用，不需要在"字节"和"字符索引"之间来回转换。

### 辅助方法设计

| 方法 | 行为 |
|------|------|
| `move_cursor_left()` | 光标向左跳一个字符 |
| `move_cursor_right()` | 光标向右跳一个字符 |
| `enter_char(c)` | 在光标处插入字符，光标前进 |
| `delete_char()` | 删除光标前一个字符，光标后退 |
| `enter_newline()` | 在光标处插入 `\n`，光标前进 1 |

---

## Step A：添加 cursor_position 和辅助方法

### A1. 修改 Chat 结构体

```rust
pub struct Chat {
    command_tx: Option<UnboundedSender<Action>>,
    messages: Vec<String>,
    conversation: Vec<Message>,
    current_ai_response: String,
    input: String,
    cursor_position: usize,  // ← 新增：光标在 input 中的字节位置
    focused: bool,
    waiting_for_response: bool,
    tick_count: u8,
}
```

### A2. 修改构造函数

```rust
pub fn new() -> Self {
    Self {
        command_tx: None,
        messages: Vec::new(),
        conversation: Vec::new(),
        current_ai_response: String::new(),
        input: String::new(),
        cursor_position: 0,       // ← 新增
        focused: true,
        waiting_for_response: false,
        tick_count: 0,
    }
}
```

### A3. 先理解核心问题：为什么光标移动不能用 `+1` / `-1`？

在写辅助方法之前，必须先理解一个 Rust 字符串的底层事实：

**`String` 是 `Vec<u8>`，不是 `Vec<char>`。**

```
字符串 "Hi你" 在内存里不是 ['H', 'i', '你']，而是：
字节: [72, 105, 228, 189, 160]
       H    i    [你-----------]
索引:  0    1    2    3    4
```

`'你'` 占 3 个字节。如果光标在 `'你'` 前面（`cursor_position = 2`），你写 `cursor_position += 1`，光标会跑到字节 3——**落在汉字的中间**。Rust 会立刻 panic：

```
byte index 3 is not a char boundary; it is inside '你' (bytes 2..4)
```

所以光标移动的本质问题是：**给定一个字节位置，如何安全地找到"前一个字符的开头"或"后一个字符的开头"？**

**错误做法（只在纯英文下能跑）：**

```rust
// ❌ 不要这样做
fn move_cursor_left(&mut self) {
    self.cursor_position = self.cursor_position.saturating_sub(1);
}
fn move_cursor_right(&mut self) {
    self.cursor_position = (self.cursor_position + 1).min(self.input.len());
}
```

**正确做法：用 `char_indices()`**

`char_indices()` 返回一个迭代器，迭代器里的每个元素是 `(usize, char)` —— 一个**元组（tuple）**：

```rust
let s = "Hi你a";

for (byte_idx, ch) in s.char_indices() {
    println!("({}, '{}')", byte_idx, ch);
}
// 输出：
// (0, 'H')
// (1, 'i')
// (2, '你')   ← '你' 起始于字节 2，占 3 个字节（2,3,4）
// (5, 'a')    ← 'a' 起始于字节 5，占 1 个字节
```

注意：**`byte_idx` 是字节位置，不是字符序号。** `'你'` 是第 3 个字符，但它起始于字节 2。

配合 `.next()` 和 `.last()` 使用：

```rust
let s = "Hi你a";

// .next() —— 取第一个
s.char_indices().next();   // → Some((0, 'H'))

// .last() —— 取最后一个（不管它是 ASCII 还是中文）
s.char_indices().last();   // → Some((5, 'a'))
```

返回值包在 `Option` 里（`Some(...)`），因为字符串可能是空的。拆开看：

```rust
if let Some((idx, c)) = s.char_indices().last() {
    // idx: usize —— 这个字符在字符串中的起始字节位置
    // c:   char  —— 字符本身
}
```

`char_indices()` 返回的 `idx` 永远是**合法的起始字节位置**，永远不会落在某个多字节字符的中间。它替你处理了所有 UTF-8 边界计算，你不需要知道每个字符占多少字节。

> **学习要点**：在 Rust 里，"字符串的第 N 个字符"和"字符串的第 N 个字节"是两个完全不同的概念。`char_indices()` 是你在它们之间做安全转换的桥梁。

---

### A4. 添加辅助方法

在 `impl Chat` 块里（`impl Component for Chat` 之前）添加：

```rust
fn move_cursor_left(&mut self) {
    let before = &self.input[..self.cursor_position];
    // char_indices() 返回 (字节位置, 字符)
    // .last() 取光标前一个字符
    if let Some((idx, _)) = before.char_indices().last() {
        self.cursor_position = idx;
    }
}

fn move_cursor_right(&mut self) {
    let after = &self.input[self.cursor_position..];
    // .next() 取光标后一个字符
    if let Some((idx, c)) = after.char_indices().next() {
        // idx 是相对于 after 的偏移，在这里总是 0
        self.cursor_position += idx + c.len_utf8();
    }
}

fn enter_char(&mut self, c: char) {
    let idx = self.cursor_position;
    self.input.insert(idx, c);
    self.cursor_position += c.len_utf8();
}

fn delete_char(&mut self) {
    let before = &self.input[..self.cursor_position];
    if let Some((idx, c)) = before.char_indices().last() {
        self.input.remove(idx);
        self.cursor_position -= c.len_utf8();
    }
}

fn enter_newline(&mut self) {
    let idx = self.cursor_position;
    self.input.insert(idx, '\n');
    self.cursor_position += 1;
}
```

**逐行拆解：**

**`move_cursor_left`**

```rust
let before = &self.input[..self.cursor_position];
if let Some((idx, _)) = before.char_indices().last() {
    self.cursor_position = idx;
}
```

- `&self.input[..n]` 创建一个从开头到光标位置的子字符串切片
- `char_indices()` 遍历字符串，返回 `(字节位置, 字符)` 的对
- `.last()` 取最后一个，也就是光标前面的那个字符
- 把 `cursor_position` 设为那个字符的起始字节位置

**`move_cursor_right`**

```rust
let after = &self.input[self.cursor_position..];
if let Some((idx, c)) = after.char_indices().next() {
    self.cursor_position += idx + c.len_utf8();
}
```

- `&self.input[n..]` 创建一个从光标位置到末尾的子字符串切片
- `.next()` 取第一个，也就是光标后面的那个字符
- `idx` 是目标字符相对于 `after` 切片的偏移（总是 0）
- `c.len_utf8()` 是这个字符占多少字节
- 光标向前跳 `idx + c.len_utf8()` 个字节

**`enter_char`**

```rust
self.input.insert(idx, c);
self.cursor_position += c.len_utf8();
```

- `String::insert(idx, c)` 在字节位置 `idx` 插入字符 `c`
- `c.len_utf8()` 返回这个字符占多少字节（`char` 类型的方法）

| 字符 | `len_utf8()` |
|------|-------------|
| `'a'` | 1 |
| `'你'` | 3 |
| `'🎉'` | 4 |

**`delete_char`**

```rust
let before = &self.input[..self.cursor_position];
if let Some((idx, c)) = before.char_indices().last() {
    self.input.remove(idx);
    self.cursor_position -= c.len_utf8();
}
```

- 找到光标前一个字符的字节位置 `idx`
- `String::remove(idx)` 删除该字节位置的字符
- 光标回退这个字符的字节长度

---

## Step B：修改按键处理

### B1. 更新导入

在 `src/components/chat.rs` 顶部，把 crossterm 的导入改成：

```rust
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
```

`KeyModifiers` 用来检测是否按了 Ctrl / Shift。

### B2. 重写 handle_key_event

```rust
fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
    match key.code {
        KeyCode::Enter => {
            // Shift+Enter 换行，普通 Enter 发送
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                self.enter_newline();
                return Ok(None);
            }

            if !self.input.is_empty() {
                let text = self.input.clone();
                self.messages.push(format!("You: {}", text));
                self.conversation.push(Message::user(&text));
                self.input.clear();
                self.cursor_position = 0;   // ← 清空后光标归零
                self.start_waiting();

                if let Some(ref tx) = self.command_tx {
                    let _ = tx.send(Action::SendMessage(self.conversation.clone()));
                }
            }
            Ok(None)
        }
        KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            self.enter_newline();
            Ok(None)
        }
        KeyCode::Backspace => {
            self.delete_char();
            Ok(None)
        }
        KeyCode::Left => {
            self.move_cursor_left();
            Ok(None)
        }
        KeyCode::Right => {
            self.move_cursor_right();
            Ok(None)
        }
        KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            self.move_cursor_left();
            Ok(None)
        }
        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            self.move_cursor_right();
            Ok(None)
        }
        KeyCode::Char(c) => {
            self.enter_char(c);
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

**`KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL)` 是什么语法？**

这是 Rust `match` 的**守卫（guard）**。它的意思是：
- 先匹配 `KeyCode::Char('j')`
- 然后检查 `if` 条件是否成立
- 只有 `if` 为真时，才走这个分支

**`Ctrl+H` / `Ctrl+L`：不用离开主键区的光标移动**

除了方向键，我们还绑定了 `Ctrl+H`（左）和 `Ctrl+L`（右）。这借鉴了 Vim 的 `hjkl` 习惯——双手不用离开打字区就能移动光标。`Ctrl+H` 在大多数终端里发送的是 ASCII BS（退格），但由于 `crossterm` 能区分方向键和 `Ctrl+Char`，这里不会和 `Backspace` 冲突。

**注意：`Ctrl+J` 的终端兼容性**

在某些终端里，`Ctrl+J` 和 `Enter` 发送的是同一个控制字符（ASCII LF）。如果 `Ctrl+J` 没反应，可以换成 `Shift+Enter`，它的兼容性更好。代码里我们已经同时支持了两者。

---

## Step C：渲染视觉光标

现在的输入框用 `Paragraph::new(self.input.as_str())` 渲染，没有光标指示。用户按了左右箭头，**看不到光标在哪**。

### C1. 更新导入

在 `src/components/chat.rs` 顶部的 ratatui 导入里加上 `Span`：

```rust
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},   // ← 加了 Span
    widgets::{Block, Borders, Paragraph, Wrap},
};
```

### C2. 添加光标文本构建方法

在 `impl Chat` 里添加：

```rust
fn build_input_text(&self) -> Text<'static> {
    if !self.focused {
        return Text::from(self.input.clone());
    }

    let cursor_style = Style::default().bg(Color::Yellow).fg(Color::Black);
    let block_style = Style::default().fg(Color::Yellow);

    // 1. 计算光标在第几行、第几列（按字符计）
    let text_before = &self.input[..self.cursor_position];
    let cursor_line = text_before.chars().filter(|&c| c == '\n').count();
    let line_start = text_before.rfind('\n').map(|n| n + 1).unwrap_or(0);
    let cursor_col = self.input[line_start..self.cursor_position].chars().count();

    // 2. 按 \n 分割成多行（保留空行）
    let raw_lines: Vec<&str> = self.input.split('\n').collect();
    let mut lines = Vec::new();

    for (i, line) in raw_lines.iter().enumerate() {
        if i == cursor_line {
            let chars: Vec<char> = line.chars().collect();
            if cursor_col < chars.len() {
                // 光标在某个字符上：把它高亮
                let before: String = chars[..cursor_col].iter().collect();
                let c = chars[cursor_col];
                let after: String = chars[cursor_col + 1..].iter().collect();
                lines.push(Line::from(vec![
                    Span::raw(before),
                    Span::styled(c.to_string(), cursor_style),
                    Span::raw(after),
                ]));
            } else {
                // 光标在行尾：追加一个闪烁块
                lines.push(Line::from(vec![
                    Span::raw(line.to_string()),
                    Span::styled("▋", block_style),
                ]));
            }
        } else {
            lines.push(Line::from(line.to_string()));
        }
    }

    Text::from(lines)
}
```

**这段代码的逻辑可以画成一张图：**

```
输入: "Hel|lo"        cursor_position = 3
              ↑
        text_before = "Hel"
        cursor_line = 0（没有换行）
        cursor_col  = 3（3 个字符）

渲染:
    Span::raw("Hel") + Span::styled("l", 黄底黑字) + Span::raw("o")

输入: "Hi\nWor|ld"     cursor_position = 6（"Hi\nWor" 占 6 字节）
                 ↑
        text_before = "Hi\nWor"
        cursor_line = 1（有 1 个换行）
        line_start  = 3（最后一个 \n 在索引 2，+1 = 3）
        cursor_col  = 3（"Wor" 3 个字符）

渲染第 1 行:
    Span::raw("Wor") + Span::styled("l", 黄底黑字) + Span::raw("d")
```

**为什么用 `split('\n')` 而不是 `lines()`？**

`String::lines()` 会**丢弃末尾的空行**。比如 `"hello\n".lines()` 只返回 `["hello"]`，不返回末尾的 `""`。但我们刚按了 `Ctrl+J` 时，光标恰恰是在那个末尾空行上。`split('\n')` 能保留它。

**`Span` 是什么？**

`Text` → `Line` → `Span` 是 Ratatui 的文本层次：

| 层级 | 作用 |
|------|------|
| `Span` | 一段**连续、同样式**的文字 |
| `Line` | 一行，由多个 `Span` 拼接 |
| `Text` | 多行 `Line` 的集合 |

`Span::raw("hello")` = 默认样式的文字。
`Span::styled("l", cursor_style)` = 带自定义样式（黄底黑字）的文字。

把这三层像乐高一样拼起来，就能得到"大部分正常显示，只有一个字符高亮"的效果。

> **🐛 修复（2026-04-24）**：早期版本在循环后额外加了段代码处理"光标在末尾空行"的情况，但实际上循环本身已经能正确处理空字符串和末尾换行——`split('\n')` 会保留末尾空行，`cursor_line` 会定位到它，然后走 `else` 分支（光标在行尾）追加 `▋`。那段额外代码反而会导致空输入或末尾换行时**重复渲染两个光标块**。已删除。
>
> **💡 后续改进（见 Tutorial 05）**：用 `Span::styled` 伪造光标的做法在跨行、空行时容易出现视觉位置与数据位置不同步的 bug。更可靠的做法是只渲染纯文本，然后用 `frame.set_cursor_position()` 放置**真实的终端光标**。

### C3. 修改 draw 方法

把原来的：

```rust
let input_widget = Paragraph::new(self.input.as_str())
```

改成：

```rust
let input_widget = Paragraph::new(self.build_input_text())
    .block(
        Block::default()
            .title("Input (Enter=send, Shift+Enter/Ctrl+J=newline, Ctrl+H=left, Ctrl+L=right, Esc=quit)")
            .borders(Borders::ALL)
            .border_style(if self.focused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            }),
    )
    .wrap(Wrap { trim: true });
```

注意标题更新了，提示用户新的快捷键。

---

## Step D：编译并测试

```bash
cargo build
cargo run
```

测试清单：
1. 打字，观察字符是否正确追加
2. 按 `←` 左箭头，光标应该向左移动（注意看高亮字符或末尾的 `▋`）
3. 在光标不在末尾时打字，新字符应该插入到光标位置，而不是末尾
4. 按 `Backspace`，删除的是光标**前面**的字符，不是末尾字符
5. 按 `Shift+Enter` 或 `Ctrl+J`，应该插入换行，输入框变高
6. 按 `Enter`（不带 Shift），应该发送消息
7. 输入 `"你好"`，按左箭头两次，光标应该在 `"你"` 前面（验证 UTF-8 多字节处理正确）

---

## 概念检查清单

1. **为什么 `cursor_position` 必须跟踪字节位置，而不是字符个数？**
2. **`char_indices()` 返回什么？和 `chars()` 有什么区别？**
3. **为什么删除/移动光标时，我们用 `char_indices()` 而不是简单的 `+1` / `-1`？这是一个防御性设计决策，它防御的是什么？**
4. **`split('\n')` 和 `lines()` 有什么不同？为什么这里必须用前者？**
5. **`Span::styled` 和 `Span::raw` 的区别是什么？**

---

## Bonus：用 `SetCursorStyle` 做真正的终端光标

本教程用"高亮字符 / `▋` 块"来模拟光标，这是纯 Ratatui 的做法，不依赖终端光标状态。

如果你想把**真正的终端光标**（那个闪烁的竖线）移动到输入框里，需要：

1. 在 `draw` 里用 `crossterm::cursor` 命令控制光标位置
2. 计算光标在屏幕上的绝对坐标（要考虑 widget 边框、滚动、换行等）
3. 在 `Tui` 层管理光标显示/隐藏状态

这会复杂得多，因为 Ratatui 的 `Paragraph` 换行逻辑不暴露，你无法精确知道某个字符被渲染到了屏幕的 (x, y) 哪个坐标。需要引入 `tui-textarea` 这样的第三方库，或者自己实现一个 text area widget。

作为学习项目，"高亮字符"方案已经足够好用。
