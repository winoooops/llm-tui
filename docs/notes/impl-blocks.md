# `impl Type` vs `impl Trait for Type`

Rust 有两种 `impl` 块，职责不同，风格上应该分开写。

---

## 两种 `impl` 块

### 1. `impl Chat` — 固有方法（Inherent Methods）

这是"只属于 Chat 的方法"，不依赖任何 trait：

```rust
impl Chat {
    pub fn new() -> Self { ... }
    fn start_waiting(&mut self) { ... }
    fn append_ai_text(&mut self, text: &str) { ... }
}
```

调用方式：`Chat::new()` 或 `chat.start_waiting()`

这些是 Chat **原生自带**的能力，和 Component trait 无关。即使没有 `Component`，Chat 也有 `new()` 和 `start_waiting()`。

### 2. `impl Component for Chat` — Trait 实现

这是"让 Chat 满足某个契约（trait）"：

```rust
impl Component for Chat {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> color_eyre::Result<()> { ... }
    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> { ... }
    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> { ... }
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> { ... }
}
```

调用方式：`component.draw(frame, area)` — 通过 `Component` trait 的接口调用。

这些方法是 **Component trait 要求你实现的**，不是 Chat 自己发明的。

---

## 能合并吗？

**技术上可以，但不应该。**

你可以写成一个块：

```rust
impl Chat {
    pub fn new() -> Self { ... }
    fn start_waiting(&mut self) { ... }

    // trait 方法也塞进来
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) { ... }
    fn draw(&mut self, frame: &mut Frame, area: Rect) { ... }
}
```

但这样编译器会报错 —— trait 方法**必须**写在 `impl Trait for Type` 块里，否则不算实现了 trait。

所以唯一"合并"的方式是写两个 `impl Chat` 块，其中一个手动声明 trait 方法但没有 `for Component`。这更混乱，而且失去了 trait 的语义。

---

## 为什么要分开？

### 原因 1：一个类型可以实现多个 trait

```rust
impl Component for Chat { ... }
impl Drawable for Chat { ... }
impl Debug for Chat { ... }
```

如果全塞进一个 `impl Chat` 块，代码会爆炸。分开后每个 trait 的代码各自独立，一目了然。

### 原因 2：职责清晰

| 块 | 职责 | 谁决定的 |
|----|------|---------|
| `impl Chat` | Chat 自己特有的行为 | 你（作者） |
| `impl Component for Chat` | 满足 Component 契约的方法 | `Component` trait 定义者 |

`impl Chat` 的方法是你自由设计的；`impl Component for Chat` 的方法名和签名是 trait 定死的，你只是填空。

### 原因 3：阅读者预期

Rust 程序员看到代码时：
- `impl Chat` → "这是 Chat 的构造器和工具方法"
- `impl Component for Chat` → "Chat 可以作为 UI 组件被 App 注册和管理"

混在一起会让阅读者困惑："`draw` 是 Chat 自己的方法，还是 Component 要求的？"

---

## 一句话

> `impl Chat` 是"我是什么"；`impl Component for Chat` 是"我能扮演什么角色"。分开写，别人一眼就能看懂你的设计。
