# `Option::map` — 为什么返回值是 `Option<String>` 却还要 `.map(|s| s.to_string())`？

> 场景：你写了一个函数，链式调用里最后跟了一个 `.map(|s| s.to_string())`，想知道为什么不能直接返回 `String`。

---

## 具体例子

以教程里的 `read_cargo_name()` 为例：

```rust
fn read_cargo_name() -> Option<String> {
    let content = std::fs::read_to_string("Cargo.toml").ok()?;
    content
        .lines()
        .find(|line| line.trim_start().starts_with("name"))
        .and_then(|line| line.split('=').nth(1))
        .map(|s| s.trim().trim_matches('"').to_string())
}
```

拆成三步看，注意每步的类型变化：

```
content.lines()                           → Lines<'_>（迭代器）
    .find(...)                            → Option<&str>    ← 匹配到的行，或 None
    .and_then(|line| line.split('=').nth(1))
                                           → Option<&str>    ← "= " 后面的值，或 None
    .map(|s| s.trim().trim_matches('"').to_string())
                                           → Option<String>  ← 清理后的 String，或 None
```

---

## 关键：这里有两个不同的 `map`

| `map` | 属于 | 输入 | 输出 | 作用 |
|-------|------|------|------|------|
| `Iterator::map` | 迭代器 | `Iterator<Item = T>` | `Iterator<Item = U>` | 逐个转换元素 |
| **`Option::map`** | **Option** | **`Option<T>`** | **`Option<U>`** | **转换 `Some` 里的值，保留 `None`** |

这里用的是 **`Option::map`**，不是迭代器的 `map`。

```rust
let opt: Option<&str> = Some("hello");

// Option::map: 把 Some("hello") 变成 Some("HELLO")
let upper: Option<String> = opt.map(|s| s.to_uppercase());
// → Some("HELLO")

// 如果原来是 None，map 不动它
let none: Option<&str> = None;
let still_none: Option<String> = none.map(|s| s.to_uppercase());
// → None
```

---

## 为什么不能省略 `.map(...)`？

因为中间结果是一个**借用的字符串切片**（`&str`），不是**拥有的 `String`**：

```rust
fn read_cargo_name() -> Option<String> {
    // ...
    .and_then(|line| line.split('=').nth(1))
    // → Option<&str>  ← 这是从 line 里切出来的一段，不是独立拥有的
```

`line` 是 `content`（局部 `String`）的一个子串视图。如果直接返回 `&str`，它会悬垂——`content` 在函数结束时就 drop 了。

所以必须用 `.to_string()` 或 `.into()` **克隆**一份独立拥有的 `String`，才能作为返回值传出去。

```rust
.map(|s| s.trim().trim_matches('"').to_string())
//      ↑  &str          ↑  &str          ↑  String
//      原始切片      清理后的切片    克隆出的拥有值
```

---

## `read_package_json_name()` 同理

```rust
fn read_package_json_name() -> Option<String> {
    let content = std::fs::read_to_string("package.json").ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    v.get("name")?.as_str().map(String::from)
}
```

类型流：

```
v.get("name")       → Option<&Value>    ← 从 v 里借出来的
?.as_str()          → Option<&str>       ← 从 Value 里借出来的字符串切片
.map(String::from)  → Option<String>     ← 克隆成拥有的 String
```

`as_str()` 返回 `&str`，因为 `serde_json::Value` 内部存的是字符串数据，它不会随便把所有权交出去。你必须显式 `.map(String::from)` 来克隆一份。

---

## 记忆口诀

> **`Option::map` 只管 `Some` 里的值，`None` 原封不动传下去。**
>
> 如果 `Some` 里是 `&str`，想变成 `String`，就必须 `.map(|s| s.to_string())`——因为借来的不能当返回值。
