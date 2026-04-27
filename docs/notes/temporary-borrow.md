# 临时值与悬垂引用：为什么 clone 了还不能返回 `&T`？

> 场景：你想返回一个引用，于是先 clone 了一份数据，再对 clone 的结果取引用。编译器仍然报错。为什么？

---

## 典型错误代码

```rust
pub struct Input {
    text: String,
}

impl Input {
    // ❌ 编译错误！
    pub fn text(&self) -> &String {
        let text = &self.text.clone();  // clone 出一个临时 String
        text                              // 返回它的引用
    }
}
```

编译器报错：

```
error[E0515]: cannot return value referencing temporary value
 --> src/main.rs:8:9
  |
8 |         let text = &self.text.clone();
  |                     ----------------- temporary value created here
9 |         text
  |         ^^^^ returns a value referencing data owned by the current function
```

---

## 时间线：临时值什么时候生、什么时候死

```
函数开始
    │
    ▼
self.text.clone() ──→ 在堆上分配新内存，复制 "hello"
    │                     ↑
    ▼                     │
&self.text.clone() ───→ 对新内存取引用 &String
    │                     │
    ▼                     │
return text ──────────→ 试图把引用传回调用方
    │                     │
    ▼                     │
函数结束 ←──────────────┘
    │
    ▼
临时 String 被 drop（内存释放）
    │
    ▼
调用方手里的引用 → 指向已释放内存 💥 悬垂引用！
```

**关键事实**：

- `clone()` 创建的是**新的独立值**，不是给原值续命
- 这个新值是**临时值（temporary）**，生存期只到当前语句/表达式结束
- `&` 引用**不拥有数据**，也不延长数据的生存期——它只是"借用"
- 函数返回后，临时值被销毁，引用变成**悬垂（dangling）**

---

## 为什么不能"让编译器特殊处理"？

有些语言（比如 C++）有 **RVO（返回值优化）** 或 **延长临时值生命周期** 的规则。Rust 不做这种事，因为：

1. **规则必须简单可预测**：如果编译器有时延长、有时不延长，程序员就无法可靠地推理内存安全
2. **所有权是核心机制**：Rust 用所有权代替 GC，如果允许悬垂引用，就违背了整个语言的设计基石
3. **错误必须编译期捕获**：悬垂引用是内存安全的经典 bug（use-after-free），Rust 宁可误报也不漏报

---

## 三种正确的修复方式

根据你的真实需求选择：

### 方式 1：返回 `&str`（只读借用）

```rust
pub fn text(&self) -> &str {
    &self.text
}
```

| 要点 | 说明 |
|------|------|
| 返回类型 | `&str`（对 `String` 的不可变借用） |
| 数据归属 | 仍然属于 `Input` 里的 `self.text` |
| 调用方得到 | 一个只读视图，不能修改，不拥有数据 |
| 生命周期约束 | 返回的 `&str` 不能活得比 `Input` 实例久 |

**适用场景**：调用方只需要读数据，不需要修改或长期保存。

### 方式 2：返回 `String`（转移所有权）

```rust
pub fn text_owned(&self) -> String {
    self.text.clone()
}
```

| 要点 | 说明 |
|------|------|
| 返回类型 | `String`（owned） |
| 数据归属 | clone 出来的新 `String` 归调用方所有 |
| 调用方得到 | 完整的所有权，可以随意修改、传给别的函数 |
| 代价 | 一次堆内存分配 + 数据复制 |

**适用场景**：调用方需要长期保存数据，或者需要修改它。

### 方式 3：返回 `&String`（直接借用原字段）

```rust
pub fn text_ref(&self) -> &String {
    &self.text
}
```

| 要点 | 说明 |
|------|------|
| 返回类型 | `&String`（对原字段的借用） |
| 数据归属 | 仍然属于 `Input` |
| 调用方得到 | 借用原 `String` 的引用 |
| 为什么能行 | `self.text` 不是临时值——它的生命周期和 `Input` 一样长 |

**适用场景**：调用方需要 `&String` 类型的参数（比如某些 API 要求）。

> **为什么 `&self.text` 可以，`&self.text.clone()` 不行？**  
> `self.text` 是结构体字段，生命周期 = 整个结构体。  
> `self.text.clone()` 是函数里创建的临时值，生命周期 = 当前语句。

---

## 回到项目：我们之前的错误

重构前，在 `chat/mod.rs` 里你可能写过：

```rust
// ❌ 多余的 clone
let text = self.input.text().to_string();
self.conversation.push_user(&text);
self.input.clear();
```

如果 `text()` 返回 `&str`，这段代码根本不需要 `.to_string()`：

```rust
// ✅ 零拷贝
self.conversation.push_user(self.input.text());
self.input.clear();
```

因为 `push_user(text: &str)` 只要求 `&str`，而 `text()` 返回的 `&str` 在 `clear()` 之前一直有效。

---

## 常见变体错误

### 变体 1：`format!` 后取引用

```rust
// ❌ 错误
pub fn label(&self) -> &str {
    let s = &format!("User: {}", self.name);
    s
}

// ✅ 正确：返回 owned String
pub fn label(&self) -> String {
    format!("User: {}", self.name)
}
```

`format!` 返回临时 `String`，取引用再返回同样会悬垂。

### 变体 2：map 里构造临时值

```rust
// ❌ 错误
fn get_names(users: &[User]) -> Vec<&str> {
    users.iter().map(|u| &u.name.clone()).collect()
}

// ✅ 正确：直接借用原数据
fn get_names(users: &[User]) -> Vec<&str> {
    users.iter().map(|u| u.name.as_str()).collect()
}
```

### 变体 3：match 臂里返回引用

```rust
// ❌ 错误
fn find(ids: &[u64], target: u64) -> &str {
    match ids.iter().find(|&&id| id == target) {
        Some(_) => &format!("found: {}", target),
        None => "not found",
    }
}
```

`"not found"` 是字符串字面量（`'static`），可以返回。但 `format!()` 结果是临时的，不行。

---

## 一句话记忆法

> **引用不续命。clone 创造了新生命，但如果你只给它一个引用而不给所有权，这个新生命还是会随函数结束而终结。**

| 操作 | 数据活了多久？ |
|------|-------------|
| `&self.field` | 和 `self` 一样久 |
| `self.field.clone()` | 到当前语句结束（临时值） |
| `return self.field.clone()` | 转移给调用方，和调用方一样久 |
| `return &self.field.clone()` | ❌ 调用方拿到的是悬垂引用 |
