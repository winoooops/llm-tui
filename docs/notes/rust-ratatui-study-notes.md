# Rust + Ratatui 速记卡

## Rust 基础

### 变量与常量
- `let x = 5` — 运行时变量，默认不可变
- `let mut x = 5` — 可变变量
- `const MAX: i32 = 100` — 编译期常量，必须标类型，全大写
- `static NAME: &str = "app"` — 编译期，内存中只有一个地址

### 引用（Borrowing）
- `&x` — 不可变引用，只读，可以同时有多个
- `&mut x` — 可变引用，可以修改，**同一时刻只能有一个**
- 规则：要么一个 `&mut`，要么多个 `&`，不能同时存在

### 常用类型
- `String` — 堆上可增长的字符串，拥有所有权
- `&str` — 字符串切片，借用，不可变
- `Vec<T>` — 可变长数组，`push`/`pop`/`iter`
- `Option<T>` — `Some(T)` 或 `None`，表示"可能有值"
- `Result<T, E>` — `Ok(T)` 或 `Err(E)`，表示"可能失败"
- `Box<T>` — 堆指针，用于 trait object 或递归类型

### 控制流
- `match` — 模式匹配，必须覆盖所有情况，`_` 是通配符
- `if` 是表达式，可以返回值：`let x = if a { 1 } else { 2 };`
- `if let Some(v) = opt { ... }` — 只处理 `Some`，忽略 `None`

### 结构与方法
- `struct Foo { a: i32 }` — 定义结构体
- `impl Foo { fn new() -> Self { ... } }` — 普通方法
- `impl Trait for Foo { ... }` — 实现 trait（接口）
- `Self` — 当前类型的别名

### 迭代器
- `.iter()` — 遍历引用
- `.map(|x| ...)` — 转换每个元素
- `.collect::<Vec<_>>()` — 收集成集合，需标注类型
- `|x| x + 1` — 闭包（匿名函数）

### 所有权转移
- 默认_move_：把值传给函数后，原变量不能再用（除非实现 `Copy` trait）
- `clone()` — 显式深拷贝
- `ref` — 借用而不是拿走：`if let Some(ref x) = opt`

---

## Ratatui 核心

### 即时模式（Immediate Mode）
- 每帧（60fps）**从头重画一切**
- 没有"更新某个标签"，只有"这一帧画什么"

### 渲染流程
```
String / &str
    ↓
Line — 一行带样式的文字
    ↓
Text — 多行 Line
    ↓
Paragraph — 可渲染控件（边框、换行、样式）
    ↓
frame.render_widget(widget, area) — 画到屏幕上
```

### 布局
- `Layout::default().direction(Direction::Vertical)` — 像 Flexbox
- `.constraints([Constraint::Percentage(80), Constraint::Percentage(20)])` — 子元素尺寸
- `.split(area)` — 返回 `Vec<Rect>`，`chunks[0]`、`chunks[1]`
- `Rect` — 有 `x, y, width, height`

### 常用控件
- `Block` — 带边框和标题的容器
- `Paragraph` — 显示多行文字
- `Borders::ALL` — 四边边框
- `Wrap { trim: true }` — 自动换行
- `Style::default().fg(Color::Yellow)` — 前景色黄色

---

## 项目架构

### Component 模型
- `Component` trait = 组件契约，`draw()` 必须实现
- App 持有 `Vec<Box<dyn Component>>` — 所有已注册组件
- `Box::new(...)` — trait object，不同大小组件存指针

### Action 系统
- `Action` 枚举 = 所有可能的状态变更指令
- App 拥有 `action_rx`（接收端），组件持有 `action_tx` 克隆（发送端）
- 组件 `tx.send(Action::Quit)` → App `rx.try_recv()` → 处理

### 事件流
```
用户按键 → crossterm EventStream → Tui::event_loop 
    → Event::Key → App::handle_events 
    → Action → App::handle_actions 
    → Component::update() + Component::draw()
```

### 文件结构
```
src/components.rs          — 定义 Component trait，声明子模块
src/components/chat.rs     — Chat 组件实现
src/components.rs 顶部:    pub mod chat;
src/app.rs 注册:           Box::new(Chat::new())
```

---

## 常见写法速查

| 场景 | 写法 |
|------|------|
| 忽略未使用变量 | `let _ = foo;` |
| 解包 Option | `if let Some(ref x) = opt { use x }` |
| 忽略 Result | `let _ = tx.send(action);` |
| 构造函数 | `pub fn new() -> Self { Self { ... } }` |
| 默认实现覆盖 | 在 `impl Trait for Type` 里重写方法 |
| 字符串拼接 | `format!("Hello {}", name)` |
| 追加到 Vec | `vec.push(item)` |
| 删除 String 末尾 | `s.pop()` |
| 遍历引用 | `for x in vec.iter() { ... }` |

---

## Step B 预告

连接本地 LLM（如 Ollama）：
1. `Cargo.toml` 添加 `reqwest` 依赖
2. `Action` 枚举增加 `SendMessage(String)`、`ReceiveChunk(String)`
3. 按 Enter 时发送 `Action::SendMessage` 而不是直接 push
4. App 或 Chat 启动异步任务，用 `reqwest` POST 到 `http://localhost:11434/api/generate`
5. 流式读取响应，每收到一块就 `tx.send(Action::ReceiveChunk(chunk))`
6. `Chat::update()` 处理 `ReceiveChunk`，追加到最新消息


---

## 知识点：`Option` + `if let` + `ref` 的完整生命周期

### 场景

组件里的 `command_tx` 是 `Option<UnboundedSender<Action>>`，用来给 App 发消息。

### 为什么用 `Option`？

因为创建组件时（`Chat::new()`），App 还没把发送通道给过来。Rust 不允许字段空着，所以用 `None` 占位：

```rust
impl Chat {
    pub fn new() -> Self {
        Self {
            command_tx: None,  // ← 先放 None
            // ...
        }
    }
}
```

### 什么时候变成 `Some`？

App 启动时，在 `App::run()` 里遍历所有组件，调用 `register_action_handler`：

```rust
// src/app.rs
for component in self.components.iter_mut() {
    component.register_action_handler(self.action_tx.clone())?;
}
```

组件收到后存起来：

```rust
// src/components/chat.rs
fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> color_eyre::Result<()> {
    self.command_tx = Some(tx);  // ← None 变成 Some(tx)
    Ok(())
}
```

### `if let Some(ref tx)` 拆解

```rust
if let Some(ref tx) = self.command_tx {
    let _ = tx.send(Action::Quit);
}
```

| 部分 | 含义 |
|------|------|
| `if let Some(...)` | 检查是不是 `Some`，同时解包里面的值 |
| `ref tx` | 只**借用**，不**拿走**所有权。`tx` 的类型是 `&UnboundedSender` |
| `= self.command_tx` | 被检查的对象 |
| `let _ = ...` | 执行 `send`，但忽略它的返回值（避免编译器警告） |

### `ref` vs 不加 `ref`

```rust
if let Some(tx) = self.command_tx {      // tx 拿走值，self.command_tx 变成 None
if let Some(ref tx) = self.command_tx {  // tx 只借用，self.command_tx 不变
```

### 时间线

```
Chat::new()              → command_tx = None
App::run() 遍历组件      → 调用 register_action_handler(tx)
组件存 tx                → command_tx = Some(tx)
用户按键                 → if let Some(ref tx) 匹配成功 → tx.send(...)
```

### 一句话

> `Option` 先用 `None` 占位，等别人传值进来变成 `Some`，用时用 `if let Some(ref x)` 安全检查并借用。


---

## 知识点：流（Stream）为什么返回 `Option`？

### 场景

```rust
while let Some(chunk) = stream.next().await {
    let bytes = chunk?;
    buffer.push_str(&String::from_utf8_lossy(&bytes));
}
```

### `stream.next().await` 返回什么？

```
Option<Result<Bytes, reqwest::Error>>
```

| 外层 | 内层 | 含义 |
|------|------|------|
| `Option` | `Result<Bytes, Error>` | 流还有没有数据？ |
| | `Result` | 这块数据读取成功还是失败？ |

### 为什么流用 `Option`？

流（Stream/Iterator）不知道还有没有下一个元素。

```
数据块 1      数据块 2      数据块 3      （流结束）
   │            │            │              │
   ▼            ▼            ▼              ▼
Some(bytes)  Some(bytes)  Some(bytes)     None
```

- `Some(值)` = 还有数据，值在里面
- `None` = 流结束了，没有更多数据

### `while let Some(chunk)` 的含义

```rust
while let Some(chunk) = stream.next().await {
    // chunk = Result<Bytes, Error>
}
```

| `stream.next()` 返回 | 结果 |
|---------------------|------|
| `Some(Ok(bytes))` | 匹配成功，进入循环体 |
| `Some(Err(e))` | 匹配成功，`chunk?` 会抛出错误 |
| `None` | 匹配失败，退出循环 |

**等价写法：**

```rust
loop {
    match stream.next().await {
        Some(chunk) => { /* 循环体 */ }
        None => break,
    }
}
```

### 常见嵌套类型

| 类型 | 含义 |
|------|------|
| `Option<T>` | 可能有值（`Some`），可能没有（`None`） |
| `Result<T, E>` | 可能成功（`Ok`），可能失败（`Err`） |
| `Option<Result<T, E>>` | 先检查有没有，再检查对不对 |
| `Result<Option<T>, E>` | 先检查对不对，再检查有没有 |

### 一句话

> `Some` = "还有数据，给你一块"。`None` = "流结束了"。`while let Some(x)` 就是"只要有，就一直拿"。
