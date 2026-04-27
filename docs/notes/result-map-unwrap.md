# `Result::map` + `unwrap_or_else`：优雅的错误处理链

> 场景：你想获取当前目录路径，转成字符串，如果失败了就用 `"."` 兜底。
>
> 这段代码一次性完成了三件事：执行操作、转换成功值、处理失败值。

---

## 拆解代码

```rust
let cwd = std::env::current_dir()
    .map(|p| p.display().to_string())
    .unwrap_or_else(|_| ".".into());
```

拆成三步看：

### 第 1 步：`std::env::current_dir()`

```rust
pub fn current_dir() -> Result<PathBuf, io::Error>
```

获取程序的**当前工作目录**。返回 `Result` 因为可能失败（比如当前目录被删了、权限不足）。

- 成功 → `Ok(PathBuf { "/home/you/project" })`
- 失败 → `Err(io::Error { ... })`

### 第 2 步：`.map(|p| p.display().to_string())`

`Result::map` 只对 **`Ok` 里的值**做转换，**`Err` 原封不动传下去**。

```rust
// 假设 current_dir() 返回 Ok(PathBuf)
Ok(PathBuf("/home/you/project"))
    .map(|p| p.display().to_string())
// → Ok(String::from("/home/you/project"))
```

为什么用 `p.display().to_string()` 而不是 `p.to_string_lossy()`？

| 方法 | 行为 | 返回值 |
|------|------|--------|
| `p.display()` | 把 `PathBuf` 包装成可打印格式 | `Display`（临时） |
| `.to_string()` | 把 `Display` 格式化成 `String` | `String` |

`PathBuf` 不能直接 `.to_string()`，因为它不是 `Display` trait——路径可能包含非 UTF-8 字节（Windows 上常见）。`display()` 是一个安全的展示适配器。

### 第 3 步：`.unwrap_or_else(|_| ".".into())`

从 `Result<T, E>` 里**取出 `T`**，但如果遇到 `Err` 就执行闭包生成默认值。

```rust
// 成功路径
Ok("/home/you/project".into())
    .unwrap_or_else(|_| ".".into())
// → String::from("/home/you/project")

// 失败路径  
Err(io::Error { ... })
    .unwrap_or_else(|_| ".".into())
// → String::from(".")   （当前目录的相对表示）
```

`|_|` 表示"我不关心错误具体是什么"，直接丢弃。

> **为什么用 `unwrap_or_else` 而不是 `unwrap_or`？**
>
> `unwrap_or(".".into())` 会先执行 `".".into()` 创建默认值，**无论是否失败**。
> `unwrap_or_else(|| ...)` 只在失败时才执行闭包，**懒加载**，省一次内存分配。

---

## 完整数据流

```
std::env::current_dir()
    │
    ├─ Ok(PathBuf) ──→ .map(转String) ──→ Ok(String) ──→ .unwrap_or_else ──→ String
    │                                                              ↑
    └─ Err(io::Error) ──→ .map(跳过) ──→ Err(io::Error) ──────────┘
                                                                     闭包返回 "."
```

---

## 等价写法对比

| 写法 | 风格 | 推荐度 |
|------|------|--------|
| `match` 表达式 | 显式、冗长 | ✅ 初学者友好 |
| `?` 运算符 | 简洁，但会提前返回 | 适合函数内部 |
| `map + unwrap_or_else` | 函数式、链式 | ✅ 单表达式场景最佳 |
| `if let Ok(...) = ...` | 命令式 | 需要做额外处理时 |

用 `match` 的等价版本：

```rust
let cwd = match std::env::current_dir() {
    Ok(p) => p.display().to_string(),
    Err(_) => ".".into(),
};
```

功能完全一样，只是 `map + unwrap_or_else` 更紧凑。

---

## 常见变体

### 只想转换，不想兜底（错误往上抛）

```rust
let cwd = std::env::current_dir()
    .map(|p| p.display().to_string())?;  // ? 把 Err 抛给调用方
```

### 需要用到错误信息

```rust
let cwd = std::env::current_dir()
    .map(|p| p.display().to_string())
    .unwrap_or_else(|e| {
        eprintln!("Warning: cannot get cwd: {e}, falling back to .");
        ".".into()
    });
```

### 多级转换

```rust
let dir_name = std::env::current_dir()
    .map(|p| p.display().to_string())   // PathBuf → String
    .map(|s| s.split('/').last().unwrap_or("").to_string())  // 取目录名
    .unwrap_or_else(|_| "unknown".into());
```

---

## 记忆口诀

> **`map` 改造成功值，`unwrap_or_else` 兜底失败值。**
>
> 链式读起来像英语：*"获取当前目录，映射为字符串，否则用点号"*。
