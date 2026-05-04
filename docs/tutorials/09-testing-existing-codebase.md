# Tutorial 09：给现有代码补测试 —— 逐文件攻略

> **目标**：给每个值得测的 Rust 文件补上单元测试。  
> **前置要求**：已完成 [Tutorial 08](08-rust-testing-basics.md)。  
> **原则**：不改业务逻辑，只加 `#[cfg(test)]` 模块。

---

## 目录

1. [先跑一遍现有测试](#先跑一遍现有测试)
2. [Dev Dependency：加 `tempfile`](#dev-dependency加-tempfile)
3. [先重构 `utils.rs`](#先重构-utilsrs)
4. [逐文件攻略](#逐文件攻略)
   - [`src/message.rs`](#srcmessagers)
   - [`src/action.rs`](#srcactionrs)
   - [`src/utils.rs`](#srcutilsrs)
   - [`src/prompt.rs`](#srcpromptrs)
   - [`src/components/chat/input.rs`](#srccomponentschatinputrs)
   - [`src/components/chat/conversation.rs`](#srccomponentschatconversationrs)
   - [`src/components/fps.rs`](#srccomponentsfpsrs)
   - [`src/config.rs`（补漏）](#srcconfigrs补漏)
   - [`src/cli.rs`](#srcclirs)
   - [`src/tui.rs`](#srctuirs)
   - [`src/llm.rs`](#srcllmrs)
5. [不测的文件清单](#不测的文件清单)
6. [验收：一次全绿](#验收一次全绿)

---

## 先跑一遍现有测试

```bash
cargo test
```

你应该已经看见 `config.rs` 里有一批测试在跑。确认 baseline 是绿的，然后再加新测试。

---

## Dev Dependency：加 `tempfile`

`utils.rs` 和 `prompt.rs` 的测试需要临时文件。在 `Cargo.toml` 末尾加：

```toml
[dev-dependencies]
tempfile = "3"
```

然后 `cargo check` 拉取依赖。

---

## 先重构 `utils.rs`

在写 `utils.rs` 的测试之前，先解决一个设计问题：当前 `utils.rs` 的函数（`detect_project_type`、`read_cargo_name`、`read_readme_summary` 等）都**隐式依赖全局 cwd**。测试时为了控制"当前目录在哪"，不得不 `set_current_dir`——这是全局副作用，在并行测试里会互相踩踏。

**正确的做法**：把核心逻辑抽到带 `_at(base: &Path)` 后缀的函数里，原函数变成薄包装。测试直接调用 `_at` 版本，传临时目录路径，**不再需要 `CwdGuard`**。

这不是为了测试而测试的技巧，而是一个设计边界：

- `*_at(base)` 是**核心逻辑**：输入明确、无全局副作用、容易组合、容易测试。
- `*()` 是**环境适配器**：只负责从 `std::env::current_dir()` 取默认路径，然后转调 `*_at(base)`。

从 SOLID 看，这是把“项目探测逻辑”和“从哪里取当前目录”分开，符合**单一职责原则**；核心逻辑依赖 `&Path` 这个抽象输入，而不是直接依赖全局进程状态，符合**依赖倒置原则**的精神。

从《A Philosophy of Software Design》看，这是在消除一个“深层隐藏依赖”：cwd 原本藏在函数内部，调用者和测试都看不见，却会影响结果。把 `base` 显式传入后，接口更深、更清楚，复杂度被隔离在很薄的一层 wrapper 里。

> 关键结论：不要给 `detect_project_type()`、`read_readme_summary()` 这类无参 wrapper 写一堆改变 cwd 的单元测试。真正需要覆盖的是 `detect_project_type_at(base)` 这些核心函数。wrapper 足够薄时，它的正确性主要来自代码结构，而不是重复测一遍全局状态。

### 重构内容

文件：`src/utils.rs`

在顶部加 `use std::path::Path;`，然后修改以下函数：

```rust
pub fn detect_project_type_at(base: &Path) -> String {
    if base.join("Cargo.toml").exists() {
        "rust".into()
    } else if base.join("package.json").exists() {
        "node".into()
    } else if base.join("pyproject.toml").exists() {
        "python".into()
    } else {
        "unknown".into()
    }
}

pub fn detect_project_type() -> String {
    detect_project_type_at(&std::env::current_dir().unwrap_or_else(|_| ".".into()))
}

fn read_cargo_name_at(base: &Path) -> Option<String> {
    let content = std::fs::read_to_string(base.join("Cargo.toml")).ok()?;
    content.lines()
        .find(|line| line.trim_start().starts_with("name"))
        .and_then(|line| line.split('=').nth(1))
        .map(|s| s.trim().trim_matches('"').to_string())
}

fn read_cargo_name() -> Option<String> {
    read_cargo_name_at(&std::env::current_dir().unwrap_or_else(|_| ".".into()))
}

pub fn read_readme_summary_at(base: &Path, max_chars: usize) -> String {
    let content = match std::fs::read_to_string(base.join("README.md")) {
        Ok(s) => s,
        Err(_) => return String::new(),
    };
    content.chars().take(max_chars).collect()
}

pub fn read_readme_summary(max_chars: usize) -> String {
    read_readme_summary_at(
        &std::env::current_dir().unwrap_or_else(|_| ".".into()),
        max_chars,
    )
}

pub fn read_agents_md_at(base: &Path) -> Option<String> {
    std::fs::read_to_string(base.join("AGENTS.md")).ok()
}

pub fn read_agents_md() -> Option<String> {
    read_agents_md_at(&std::env::current_dir().unwrap_or_else(|_| ".".into()))
}
```

同样给 `read_package_json_name` 和 `read_pyproject_name` 也抽 `_at` 版本（过程略），然后给 `detect_project_name` 也抽一个：

```rust
pub fn detect_project_name_at(base: &Path) -> String {
    read_cargo_name_at(base)
        .or_else(|| read_package_json_name_at(base))
        .or_else(|| read_pyproject_name_at(base))
        .or_else(|| base.file_name().and_then(|n| n.to_str()).map(String::from))
        .unwrap_or_else(|| "unknown".into())
}

pub fn detect_project_name() -> String {
    detect_project_name_at(&std::env::current_dir().unwrap_or_else(|_| ".".into()))
}
```

**生产代码的调用方完全不需要改**，原函数签名和行为不变。只有测试会调用 `_at` 版本。

### 那无参函数还要不要测？

一般不要单独测每一个无参函数。原因是：为了测它们，你必须再次 `set_current_dir`，这会把已经移除的全局副作用带回测试套件。

更优雅的策略是分三层：

1. **大量单元测试测 `_at` 函数**  
   例如 `detect_project_type_at(dir.path())`、`read_cargo_name_at(dir.path())`。这些测试并行安全，覆盖真正的业务规则。

2. **无参函数保持极薄**  
   无参函数只允许做两件事：取 `current_dir`，调用对应的 `_at`。不要在 wrapper 里塞判断、解析、fallback 逻辑。

3. **如果你非常想测 wrapper，只写一个 smoke test**  
   这个测试只证明“wrapper 会转调 cwd”，不要为每个场景都测一遍。并且要用互斥锁保护 cwd，避免并行测试互相干扰。

例如：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::{
        path::PathBuf,
        sync::{Mutex, OnceLock},
    };

    fn cwd_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct CwdGuard {
        original: PathBuf,
    }

    impl CwdGuard {
        fn enter(path: &Path) -> Self {
            let original = std::env::current_dir().unwrap();
            std::env::set_current_dir(path).unwrap();
            Self { original }
        }
    }

    impl Drop for CwdGuard {
        fn drop(&mut self) {
            std::env::set_current_dir(&self.original).unwrap();
        }
    }

    #[test]
    fn detect_project_type_uses_current_dir_as_default_base() {
        let _lock = cwd_lock().lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"foo\"\n").unwrap();

        let _cwd = CwdGuard::enter(dir.path());

        assert_eq!(detect_project_type(), "rust");
    }
}
```

但注意：这只是 smoke test，不是主测试策略。如果项目里完全没有 wrapper 测试，也可以接受，因为 wrapper 的实现应该简单到一眼能看完：

```rust
pub fn detect_project_type() -> String {
    detect_project_type_at(&std::env::current_dir().unwrap_or_else(|_| ".".into()))
}
```

这就是“让代码容易理解，而不是让测试绕复杂度打转”。

---

## 逐文件攻略

---

### `src/message.rs`

**测什么**：三个构造器 + serde 往返。

在文件底部追加：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn user_message_fields() {
        let msg = Message::user("hello");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "hello");
    }

    #[test]
    fn assistant_message_fields() {
        let msg = Message::assistant("world");
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.content, "world");
    }

    #[test]
    fn system_message_fields() {
        let msg = Message::system("be concise");
        assert_eq!(msg.role, "system");
        assert_eq!(msg.content, "be concise");
    }

    #[test]
    fn serde_roundtrip() {
        let original = Message::user("test content");
        let json = serde_json::to_string(&original).unwrap();
        let restored: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }
}
```

---

### `src/action.rs`

**测什么**：`Display` 输出、`Serialize` 格式、变体判别。

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn display_variants() {
        assert_eq!(Action::Tick.to_string(), "Tick");
        assert_eq!(Action::Quit.to_string(), "Quit");
        assert_eq!(Action::StreamEnd.to_string(), "StreamEnd");
    }

    #[test]
    fn send_message_display_and_payload() {
        let action = Action::SendMessage(vec![Message::user("hi")]);
        assert_eq!(action.to_string(), "SendMessage");

        // verify the variant carries the payload (Message's own tests cover content/role)
        if let Action::SendMessage(msgs) = action {
            assert_eq!(msgs.len(), 1);
        } else {
            panic!("expected SendMessage variant");
        }
    }

    #[test]
    fn receive_chunk_display_and_payload() {
        let action = Action::ReceiveChunk("hello".into());
        assert_eq!(action.to_string(), "ReceiveChunk");

        if let Action::ReceiveChunk(chunk) = action {
            assert_eq!(chunk, "hello");
        } else {
            panic!("expected ReceiveChunk variant");
        }
    }

    #[test]
    fn serde_serialize_tick() {
        let json = serde_json::to_string(&Action::Tick).unwrap();
        assert_eq!(json, r#""Tick""#);
    }

    #[test]
    fn action_equality() {
        assert_eq!(Action::Quit, Action::Quit);
        assert_ne!(Action::Tick, Action::Quit);
    }
}
```

---

### `src/utils.rs`

**测什么**：`spinner_frame`、纯逻辑部分。文件 IO 函数用临时目录测。

这里的原则是：**测试 `_at`，不要测试 cwd wrapper**。

`detect_project_type_at(base)`、`read_cargo_name_at(base)`、`read_readme_summary_at(base, max_chars)` 才是有业务规则的函数。`detect_project_type()` 这种无参函数只是“取当前目录 + 转调”，不要为了覆盖它而在每个测试里 `set_current_dir`。

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn spinner_frame_cycles() {
        let len = SPINNER_BRAILLE.len();
        assert_eq!(spinner_frame(0), SPINNER_BRAILLE[0]);
        assert_eq!(spinner_frame(len), SPINNER_BRAILLE[0]);   // wrap around
        assert_eq!(spinner_frame(len + 1), SPINNER_BRAILLE[1]);
    }

    #[test]
    fn detect_project_type_rust() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"foo\"\n").unwrap();
        assert_eq!(detect_project_type_at(dir.path()), "rust");
    }

    #[test]
    fn detect_project_type_unknown() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(detect_project_type_at(dir.path()), "unknown");
    }

    #[test]
    fn read_cargo_name_parses_name() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"my-crate\"\nversion = \"0.1.0\"\n"
        ).unwrap();
        assert_eq!(read_cargo_name_at(dir.path()), Some("my-crate".into()));
    }

    #[test]
    fn read_readme_summary_truncates() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("README.md"), "a".repeat(1000)).unwrap();
        assert_eq!(read_readme_summary_at(dir.path(), 100).len(), 100);
    }

    #[test]
    fn read_readme_summary_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_readme_summary_at(dir.path(), 100).is_empty());
    }
}
```

> 测试直接调用 `_at` 版本并传 `dir.path()`，**不需要 `set_current_dir`，不需要 `CwdGuard`**。并行跑任意线程数都是安全的，因为不再触碰全局状态。

如果你发现自己想为 `detect_project_type()`、`read_readme_summary()`、`read_agents_md()` 分别写一套 cwd 测试，先停下来：这说明测试正在穿过 wrapper 去测核心逻辑。应该把断言移到 `_at` 函数上，让 wrapper 保持极薄。

---

### `src/prompt.rs`

**测什么**：`PromptContext::new()` 直接构造 + `assemble_system_message()` 的输出格式。

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn system_prompt_contains_boundary() {
        let ctx = PromptContext::new(
            "/home/will/projects/foo",
            "foo",
            "A test project",
            "rust",
            None,
        );
        let msg = ctx.system_prompt();
        assert_eq!(msg.role, "system");
        assert!(
            msg.content.contains("__SYSTEM_PROMPT_DYNAMIC_BOUNDARY__"),
            "boundary marker missing"
        );
    }

    #[test]
    fn system_prompt_includes_project_info() {
        let ctx = PromptContext::new(
            "/tmp",
            "my-app",
            "Does cool things",
            "node",
            None,
        );
        let msg = ctx.system_prompt();
        assert!(msg.content.contains("my-app"));
        assert!(msg.content.contains("node"));
        assert!(msg.content.contains("Does cool things"));
    }

    #[test]
    fn system_prompt_omits_empty_summary() {
        let ctx = PromptContext::new("/tmp", "x", "", "rust", None);
        let msg = ctx.system_prompt();
        assert!(!msg.content.contains("# Project Summary"));
    }

    #[test]
    fn system_prompt_includes_agents_md() {
        let ctx = PromptContext::new("/tmp", "x", "summary", "rust", Some("Be aggressive"));
        let msg = ctx.system_prompt();
        assert!(msg.content.contains("Be aggressive"));
        assert!(msg.content.contains("Project Instructions(Agents.md)"));
    }

    #[test]
    fn new_constructor_maps_fields() {
        let ctx = PromptContext::new("/a", "b", "c", "d", Some("e"));
        assert_eq!(ctx.cwd, "/a");
        assert_eq!(ctx.project_name, "b");
        assert_eq!(ctx.project_summary, "c");
        assert_eq!(ctx.project_type, "d");
        assert_eq!(ctx.agents_md, Some("e".into()));
    }
}
```

---

### `src/components/chat/input.rs`

**测什么**：光标移动、字符插入删除、多行、行列计算。这是最容易出 Unicode bug 的地方。

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn new_input_is_empty() {
        let input = Input::new();
        assert!(input.is_empty());
        assert_eq!(input.text(), "");
    }

    #[test]
    fn enter_char_appends() {
        let mut input = Input::new();
        input.enter_char('a');
        assert_eq!(input.text(), "a");
    }

    #[test]
    fn enter_char_moves_cursor() {
        let mut input = Input::new();
        input.enter_char('a');
        input.enter_char('b');
        assert_eq!(input.text(), "ab");
    }

    #[test]
    fn delete_char_removes_last() {
        let mut input = Input::new();
        input.enter_char('a');
        input.enter_char('b');
        input.delete_char();
        assert_eq!(input.text(), "a");
    }

    #[test]
    fn delete_char_on_empty_does_nothing() {
        let mut input = Input::new();
        input.delete_char();
        assert!(input.is_empty());
    }

    #[test]
    fn move_cursor_left_and_right() {
        let mut input = Input::new();
        input.enter_char('a');
        input.enter_char('b');
        input.move_cursor_left();
        input.enter_char('x');
        assert_eq!(input.text(), "axb");
    }

    #[test]
    fn unicode_char_handling() {
        let mut input = Input::new();
        input.enter_char('中');
        input.enter_char('文');
        assert_eq!(input.text(), "中文");
        input.move_cursor_left();
        input.delete_char();
        assert_eq!(input.text(), "文");
    }

    #[test]
    fn new_line_and_cursor_position() {
        let mut input = Input::new();
        input.enter_char('a');
        input.enter_new_line();
        input.enter_char('b');
        assert_eq!(input.text(), "a\nb");
        assert_eq!(input.cursor_position(), (1, 1));
    }

    #[test]
    fn cursor_position_at_start() {
        let input = Input::new();
        assert_eq!(input.cursor_position(), (0, 0));
    }

    #[test]
    fn clear_resets_everything() {
        let mut input = Input::new();
        input.enter_char('a');
        input.clear();
        assert!(input.is_empty());
        assert_eq!(input.cursor_position(), (0, 0));
    }
}
```

---

### `src/components/chat/conversation.rs`

**测什么**：状态机转换。这是最核心的业务逻辑之一。

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn new_conversation_is_empty() {
        let conv = Conversation::new();
        assert!(conv.messages().is_empty());
    }

    #[test]
    fn push_user_adds_message() {
        let mut conv = Conversation::new();
        conv.push_user("hello");
        assert_eq!(conv.messages().len(), 1);
        assert_eq!(conv.messages()[0].role, "user");
        assert_eq!(conv.messages()[0].content, "hello");
    }

    #[test]
    fn start_response_sets_waiting() {
        let mut conv = Conversation::new();
        conv.start_response();
        // tests 模块和 Conversation 在同一个文件里，可以直接做白盒断言。
        assert!(conv.waiting);
    }

    #[test]
    fn append_chunk_creates_ai_line() {
        let mut conv = Conversation::new();
        conv.append_chunk("Hi");
        let text = conv.render();
        assert!(text.to_string().contains("AI: Hi"));
    }

    #[test]
    fn append_chunk_appends_to_existing_ai_line() {
        let mut conv = Conversation::new();
        conv.append_chunk("Hi");
        conv.append_chunk(" there");
        let text = conv.render();
        assert!(text.to_string().contains("AI: Hi there"));
    }

    #[test]
    fn finish_response_moves_to_conversation() {
        let mut conv = Conversation::new();
        conv.append_chunk("Done");
        conv.finish_response();
        assert_eq!(conv.messages().len(), 1);
        assert_eq!(conv.messages()[0].role, "assistant");
        assert_eq!(conv.messages()[0].content, "Done");
    }

    #[test]
    fn finish_response_on_empty_does_nothing() {
        let mut conv = Conversation::new();
        conv.finish_response();
        assert!(conv.messages().is_empty());
    }

    #[test]
    fn full_conversation_flow() {
        let mut conv = Conversation::new();

        conv.push_user("Hello");
        conv.start_response();
        conv.append_chunk("W");
        conv.append_chunk("orld");
        conv.finish_response();

        let msgs = conv.messages();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "user");
        assert_eq!(msgs[1].role, "assistant");
        assert_eq!(msgs[1].content, "World");
    }
}
```

> 注意：`waiting` 字段仍然不是 `pub`。这里能访问它，是因为 `#[cfg(test)] mod tests` 写在 `conversation.rs` 同一个文件里，属于当前模块的子模块，可以访问父模块的私有字段。如果把测试放到 `tests/` 目录做集成测试，就不能直接读 `conv.waiting`，需要改用 `render()` 或公开方法来观察行为。

---

### `src/components/fps.rs`

**测什么**：数学公式。`Instant` 难以直接操控，但可以测内部的计数逻辑——如果把它拆成纯函数的话。

当前的 `app_tick` 和 `render_tick` 依赖 `Instant::now()`，不好测。**不改业务代码的前提下**，我们能测的是 `Default` 和初始化值：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_fps_counter_defaults_to_zero() {
        let fps = FpsCounter::new();
        assert_eq!(fps.tick_count, 0);
        assert_eq!(fps.frame_count, 0);
        assert_eq!(fps.ticks_per_second, 0.0);
        assert_eq!(fps.frames_per_second, 0.0);
    }

    #[test]
    fn default_equals_new() {
        let a = FpsCounter::new();
        let b = FpsCounter::default();
        assert_eq!(a.tick_count, b.tick_count);
        assert_eq!(a.frame_count, b.frame_count);
    }
}
```

> 如果要深度测 FPS 计算，需要把 `app_tick` 里的 `Instant::now()` 抽象成可注入的时钟。这是重构话题，不是测试话题。先放一放。

---

### `src/config.rs`（补漏）

这个文件已经有大量测试了。运行 `cargo test config::tests` 看看覆盖率。

值得补的缺口：

```rust
#[cfg(test)]
mod extra_tests {
    use super::*;

    #[test]
    fn get_data_dir_returns_absolute() {
        let dir = get_data_dir();
        assert!(dir.is_absolute());
    }

    #[test]
    fn get_config_dir_returns_absolute() {
        let dir = get_config_dir();
        assert!(dir.is_absolute());
    }

    #[test]
    fn parse_key_sequence_single_key() {
        let keys = parse_key_sequence("q").unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].code, KeyCode::Char('q'));
    }

    #[test]
    fn parse_key_sequence_with_brackets() {
        let keys = parse_key_sequence("<ctrl-a>").unwrap();
        assert_eq!(keys.len(), 1);
        assert!(keys[0].modifiers.contains(KeyModifiers::CONTROL));
    }

    #[test]
    fn key_event_to_string_roundtrip() {
        let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL);
        let s = key_event_to_string(&key);
        let parsed = parse_key_event(&s).unwrap();
        assert_eq!(key, parsed);
    }
}
```

> 把这些补进已有的 `mod tests` 里即可，不用新建 `mod extra_tests`。

---

### `src/cli.rs`

**测什么**：`version()` 输出里包含预期字段。

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_contains_authors() {
        let v = version();
        assert!(v.contains("Authors:"));
    }

    #[test]
    fn version_contains_config_dir() {
        let v = version();
        assert!(v.contains("Config directory:"));
    }

    #[test]
    fn version_contains_data_dir() {
        let v = version();
        assert!(v.contains("Data directory:"));
    }

    #[test]
    fn cli_default_tick_rate() {
        let cli = Cli::try_parse_from(["llm-tui"]).unwrap();
        assert_eq!(cli.tick_rate, 4.0);
        assert_eq!(cli.frame_rate, 60.0);
    }
}
```

> `Cli::try_parse_from` 来自 `clap::Parser`，用来在测试里模拟命令行参数。

---

### `src/tui.rs`

**测什么**：`Event` 枚举的 serde 往返（这是纯数据，好测）。终端操作不测。

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_serde_roundtrip() {
        let events = vec![
            Event::Tick,
            Event::Quit,
            Event::Resize(80, 24),
            Event::Paste("hello".into()),
        ];
        for original in events {
            let json = serde_json::to_string(&original).unwrap();
            let restored: Event = serde_json::from_str(&json).unwrap();
            assert_eq!(format!("{:?}", original), format!("{:?}", restored));
        }
    }
}
```

> `Event` 里有 `KeyEvent` 和 `MouseEvent`，它们不一定实现 `PartialEq`，所以用 `format!("{:?}")` 比较 Debug 输出。实际运行看看是否通过；如果 `KeyEvent` 的某个字段（如 `kind`）序列化后丢失，再调整。

---

### `src/llm.rs`

**现状**：`stream_chat` 直接发 HTTP，URL 和模型名都硬编码。这是**最难测**的函数。

**策略**：先不测 HTTP 层，把 SSE 解析逻辑抽成纯函数再测。

#### Step A：把 SSE buffer 处理抽成函数（不改外部接口）

在 `llm.rs` 里，把 stream 循环中的解析逻辑拆出来：

```rust
// 私有函数，供测试
fn extract_content_from_sse_line(line: &str) -> Option<String> {
    let data = line.strip_prefix("data: ")?;
    if data == "[DONE]" {
        return Some("[DONE]".into());
    }
    let v: serde_json::Value = serde_json::from_str(data).ok()?;
    v["choices"][0]["delta"]["content"]
        .as_str()
        .map(String::from)
}
```

然后原函数改成调用它：

```rust
while let Some(pos) = buffer.find('\n') {
    let line = buffer.drain(..=pos).collect::<String>();
    match extract_content_from_sse_line(&line) {
        Some(ref s) if s == "[DONE]" => {
            let _ = tx.send(Action::StreamEnd);
            return Ok(());
        }
        Some(content) => {
            let _ = tx.send(Action::ReceiveChunk(content));
        }
        None => {}
    }
}
```

#### Step B：给提取函数写测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_content_valid_delta() {
        let line = r#"data: {"choices":[{"delta":{"content":"hello"}}]}"#;
        assert_eq!(
            extract_content_from_sse_line(line),
            Some("hello".into())
        );
    }

    #[test]
    fn extract_content_done_marker() {
        let line = "data: [DONE]";
        assert_eq!(
            extract_content_from_sse_line(line),
            Some("[DONE]".into())
        );
    }

    #[test]
    fn extract_content_missing_prefix() {
        assert_eq!(
            extract_content_from_sse_line(r#"{"choices":[]}"#),
            None
        );
    }

    #[test]
    fn extract_content_invalid_json() {
        assert_eq!(
            extract_content_from_sse_line("data: not json"),
            None
        );
    }

    #[test]
    fn extract_content_no_content_field() {
        let line = r#"data: {"choices":[{"delta":{}}]}"#;
        assert_eq!(extract_content_from_sse_line(line), None);
    }
}
```

#### 关于 HTTP 层测试

如果你后续想测完整的 HTTP 请求，有两种路线：

1. **加 `wiremock` 或 `mockito` dev-dependency**，在测试里起一个假服务器。
2. **重构 `llm.rs` 接收 `base_url` 参数**，测试时指向 `http://127.0.0.1:<random_port>`。

这属于进阶话题，不在本章覆盖。先把 SSE 解析逻辑测稳就已经很有价值了。

---

## 不测的文件清单

以下文件**本章不加单元测试**。原因都写在注释里：

| 文件 | 原因 |
|---|---|
| `src/app.rs` | 组合根 + 事件循环，集成测试 territory |
| `src/errors.rs` | 全局 panic hook，`better_panic`/`human_panic` 是外部行为 |
| `src/logging.rs` | 全局 tracing subscriber 初始化，副作用不可撤销 |
| `src/components/home.rs` | 纯 UI 绘制，无逻辑 |
| `src/components.rs` | trait 定义，默认实现无状态 |

这些可以在项目成熟后用 `tests/integration_tests.rs` 做端到端测试。

---

## 验收：一次全绿

全部加完后，运行：

```bash
cargo test
```

预期输出：

```
running XXX tests
test result: ok. XXX passed; 0 failed; 0 ignored; 0 measured
```

如果某个测试因为临时目录切换失败导致后续测试崩（工作目录被改到 `/tmp` 没恢复），用单线程跑定位问题：

```bash
cargo test -- --test-threads=1
```

---

> **测试不是一次性任务**。以后每改一个函数，顺手补一个测试；每发现一个 bug，先写一个**会失败的测试**，再修代码。这叫 TDD 的半价版——不严格要求"测试先于实现"，但要求"测试先于关闭 issue"。
