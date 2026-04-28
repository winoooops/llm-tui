# Tutorial 08：Rust 测试基础 —— 从零到 `cargo test`

> **目标**：掌握 Rust 单元测试的写法、组织方式、常用宏和 async 测试。  
> **前置要求**：会写基础 Rust，有本项目代码在手。  
> **下一步**：[Tutorial 09](09-testing-existing-codebase.md) 会给每个现有文件写测试。

---

## 目录

1. [为什么测试？](#为什么测试)
2. [最小测试：`#[test]`](#最小测试test)
3. [测试的组织：`#[cfg(test)]`](#测试的组织cfgtest)
4. [常用断言宏](#常用断言宏)
5. [测试返回 `Result`](#测试返回-result)
6. [测试 panic：`should_panic`](#测试-panicshould_panic)
7. [美化输出：`pretty_assertions`](#美化输出pretty_assertions)
8. [Async 测试：`tokio::test`](#async-测试tokiotest)
9. [测试私有函数](#测试私有函数)
10. [运行测试：`cargo test` 的常用姿势](#运行测试cargo-test-的常用姿势)
11. [什么值得测、什么跳过](#什么值得测什么跳过)

---

## 为什么测试？

我们这个项目不是玩具。它有：

- 聊天状态机（`Conversation`：user → waiting → chunk → StreamEnd）
- 光标移动（`Input`：Unicode 字符边界、换行、行列计算）
- 文件系统探测（`utils.rs`：读 Cargo.toml、README.md）
- Prompt 组装（字符串拼接、边界标记）
- 配置解析（JSON5、键位映射、颜色字符串）

**任何一处手滑，都是 TUI 里的诡异 bug。** 测试不是"额外工作"，是防止你凌晨 3 点 debug 的保险。

---

## 最小测试：`#[test]`

Rust 的测试就是普通函数，头顶加一个属性：

```rust
#[test]
fn it_works() {
    assert_eq!(2 + 2, 4);
}
```

放哪里都行，但惯例是放在被测代码的同一个文件底部，包在一个 `tests` 模块里：

```rust
// src/message.rs

pub struct Message { /* ... */ }

impl Message {
    pub fn user(content: impl Into<String>) -> Self { /* ... */ }
}

// ========== 以下全是测试 ==========

#[cfg(test)]
mod tests {
    use super::*;   // 把 Message 等导入测试模块

    #[test]
    fn user_message_has_user_role() {
        let msg = Message::user("hello");
        assert_eq!(msg.role, "user");
    }
}
```

> `#[cfg(test)]` 的意思是：**只在 `cargo test` 时编译这段代码**，正常 `cargo build` 会跳过，不增加 release 体积。

---

## 测试的组织：`#[cfg(test)]`

一个文件一个 `mod tests`，这是最轻量的做法：

```rust
// src/foo.rs

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_positive_numbers() {
        assert_eq!(add(1, 2), 3);
    }

    #[test]
    fn add_negative_numbers() {
        assert_eq!(add(-1, -2), -3);
    }
}
```

如果你要测**跨模块的集成行为**，可以建一个顶层 `tests/` 目录：

```
project/
├── src/
│   └── message.rs
└── tests/
    └── integration_test.rs   // 独立编译成单独的 test binary
```

`tests/integration_test.rs` 里要像外部使用者一样 `use llm_tui::message::Message;`，所以只能测 `pub` 接口。

**我们的策略**：
- 纯逻辑、小函数 → 同文件 `mod tests`
- 端到端（App 启动、组件联动）→ `tests/` 目录（后续需要时再加）

---

## 常用断言宏

| 宏 | 用途 | 失败时输出 |
|---|---|---|
| `assert!(expr)` | expr 为 true | `assertion failed: expr` |
| `assert_eq!(a, b)` | a == b | 左右值都打印 |
| `assert_ne!(a, b)` | a != b | 左右值都打印 |
| `assert!(expr, "msg: {}", val)` | 带自定义报错信息 | 你的格式化字符串 |

```rust
#[test]
fn message_content_matches() {
    let msg = Message::user("hello");
    assert_eq!(msg.content, "hello");
    assert_ne!(msg.role, "assistant");
    assert!(!msg.content.is_empty(), "content should not be empty");
}
```

---

## 测试返回 `Result`

如果测试里有很多可能失败的操作，可以让测试本身返回 `Result`，省去每层都 `unwrap()`：

```rust
#[test]
fn parse_config_returns_ok() -> color_eyre::Result<()> {
    let c = Config::new()?;
    assert_eq!(c.keybindings.0.len(), 3);
    Ok(())
}
```

> 注意：测试返回 `Result` 时，**不能用 `should_panic`**。二者互斥。

---

## 测试 panic：`should_panic`

如果你有一个函数"在非法输入时必须 panic"，可以用 `should_panic`：

```rust
pub fn divide(a: f64, b: f64) -> f64 {
    if b == 0.0 {
        panic!("division by zero");
    }
    a / b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "division by zero")]
    fn divide_by_zero_panics() {
        divide(10.0, 0.0);
    }
}
```

`expected = "..."` 是可选的，用来确认 panic 消息里确实包含这段文字，防止"因为别的原因 panic 了却测试通过"。

---

## 美化输出：`pretty_assertions`

本项目 `Cargo.toml` 里已经依赖了 `pretty_assertions`。当 `assert_eq!` 失败时，它会给出一个带颜色、带 diff 的输出：

```rust
use pretty_assertions::assert_eq;

#[test]
fn long_string_comparison() {
    let a = "hello world foo bar";
    let b = "hello world baz bar";
    assert_eq!(a, b);   // diff 高亮 foo vs baz
}
```

> 不用改任何别的代码，只需要 `use pretty_assertions::assert_eq;` 覆盖掉 std 的 `assert_eq!`。对 `assert_ne!` 同样有效。

---

## Async 测试：`tokio::test`

我们的 `llm.rs` 里有 `async fn stream_chat(...)`。要测 async 函数，用 `tokio::test`：

```rust
#[tokio::test]
async fn stream_chat_returns_ok() {
    // ... setup mock server ...
    let result = llm::stream_chat(&system, &messages, tx).await;
    assert!(result.is_ok());
}
```

`#[tokio::test]` 会自动帮你包一个 runtime，不用手动 `tokio::runtime::Runtime::new()`。

---

## 测试私有函数

Rust 的 `mod tests` 如果放在同一个文件里，可以通过 `use super::*;` 访问私有函数：

```rust
// src/utils.rs

fn read_cargo_name() -> Option<String> { /* ... */ }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_name_parsing() {
        // 可以直接调用私有函数 read_cargo_name()
        let name = read_cargo_name();
        assert!(name.is_some());
    }
}
```

这是同文件测试最大的优势——**白盒测试**。

---

## 运行测试：`cargo test` 的常用姿势

```bash
# 运行全部测试
cargo test

# 只运行某个测试函数
cargo test user_message_has_user_role

# 运行某个模块下的所有测试
cargo test message::tests

# 运行匹配前缀的所有测试
cargo test conversation_

# 看见 println! 输出（默认被隐藏）
cargo test -- --nocapture

# 只编译不运行（快速检查语法）
cargo test --no-run

# 单线程运行（避免并行导致输出混乱）
cargo test -- --test-threads=1
```

---

## 什么值得测、什么跳过

| 类型 | 例子 | 建议 |
|------|------|------|
| **纯逻辑/计算** | `Input::cursor_position()`、`spinner_frame()` | ✅ 必测，输入输出确定 |
| **状态机转换** | `Conversation::append_chunk()` → `finish_response()` | ✅ 必测，核心行为 |
| **字符串组装** | `PromptContext::assemble_system_message()` | ✅ 必测，用 assert_eq! |
| **文件解析** | `read_cargo_name()`、`detect_project_type()` | ✅ 测，但需要临时文件 |
| **数据结构构造** | `Message::user()`、`Action` 枚举 | ✅ 轻量测试 |
| **配置反序列化** | `Config::new()`、`parse_key_event()` | ✅ 已有测试，继续补 |
| **HTTP 请求** | `llm::stream_chat()` | ⚠️ 需要 mock，见 Tutorial 09 |
| **UI 绘制** | `Chat::draw()`、`FpsCounter::draw()` | ❌ 不测（测了维护成本极高） |
| **终端操作** | `Tui::enter()`、`Tui::exit()` | ❌ 不测（依赖真实终端状态） |
| **初始化副作用** | `errors::init()`、`logging::init()` | ❌ 不测（全局状态难清理） |
| **App 主循环** | `App::run()` | ❌ 同文件不测，放 `tests/` 集成测 |

---

## 最小可运行示例

新建 `src/example_test.rs`（只是练手，后续删掉）：

```rust
pub fn double(x: i32) -> i32 {
    x * 2
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn double_positive() {
        assert_eq!(double(3), 6);
    }

    #[test]
    fn double_zero() {
        assert_eq!(double(0), 0);
    }

    #[test]
    fn double_negative() {
        assert_eq!(double(-4), -8);
    }
}
```

运行：

```bash
cargo test example_test::tests
```

看见 `test result: ok. 3 passed; 0 failed` 就算过关。

---

> **核心原则**：测试不是为了"覆盖率好看"，是为了**你改代码时敢下手**。
>
> 有了测试，重构 `Input` 的光标算法时你心里有底；没有测试，改一行抖三抖。
