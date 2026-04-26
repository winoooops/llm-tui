# `async move { }` 里的 `move` 是干什么的？

## 场景

```rust
Action::SendMessage(prompt) => {
    let tx = self.action_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = call_llm(prompt, tx).await {
            tracing::error!("LLM error: {}", e);
        }
    });
}
```

**问题：** `tokio::spawn` 里为什么必须写 `async move`？不写 `move` 会怎样？

---

## 先理解闭包（Closure）的捕获方式

Rust 闭包捕获外部变量有两种方式：

| 方式 | 关键字 | 效果 |
|------|--------|------|
| **借用**（默认） | 无 | 闭包内部借用外部变量，闭包不能活得比变量久 |
| **转移所有权** | `move` | 把外部变量**所有权移进**闭包，闭包自己拥有它们 |

### 默认情况（借用）

```rust
let s = String::from("hello");
let f = || {
    println!("{}", s);   // 闭包内部借用 s
};
f();                      // ✅ 没问题
println!("{}", s);         // ✅ s 还在，因为只是被借用
```

### 加 move（转移所有权）

```rust
let s = String::from("hello");
let f = move || {
    println!("{}", s);   // s 的所有权被 move 进闭包
};
f();                      // ✅ 闭包内部使用 s
// println!("{}", s);      // ❌ 编译错误！s 已经被 move 进闭包了
```

---

## `tokio::spawn` 为什么必须用 `move`？

`tokio::spawn` 创建了一个**新任务**，这个新任务可能在当前函数结束后还在运行。

### 不用 `move` 会怎样？

```rust
Action::SendMessage(prompt) => {
    let tx = self.action_tx.clone();
    tokio::spawn(async {           // ← 没有 move！
        call_llm(prompt, tx).await; // prompt 和 tx 只是被借用
    });
}
```

编译器会报错：

```
error: lifetime may not live long enough
  --> prompt` is borrowed here
  --> but the spawn task may outlive the current function
```

**原因：**

- `prompt` 和 `tx` 是 `handle_actions` 函数里的局部变量
- `tokio::spawn` 创建的任务是**异步后台任务**，可能活得比 `handle_actions` 还久
- 默认情况下，闭包只是**借用** `prompt` 和 `tx`
- 如果 `handle_actions` 结束了，局部变量被销毁，后台任务还在读它们 → **悬垂引用** → 内存不安全

### 用了 `move` 就安全了

```rust
Action::SendMessage(prompt) => {
    let tx = self.action_tx.clone();
    tokio::spawn(async move {      // ← 加 move！
        call_llm(prompt, tx).await; // prompt 和 tx 的所有权被移进闭包
    });
}                              // handle_actions 结束，但闭包自己拥有 prompt 和 tx
```

**原理：**

- `move` 把 `prompt` 和 `tx` 的**所有权转移**进 `async` 块
- 闭包自己拥有这些值，不依赖外部作用域
- `handle_actions` 结束后，`prompt` 和 `tx` 不会消失，因为闭包持有它们

---

## 生命周期图

### 不用 `move`（错误）

```
handle_actions 开始
    │
    ▼
创建 prompt, tx ─────┐
    │                │
    ▼                │
tokio::spawn(async { │   ← 闭包借用 prompt, tx
    call_llm(...)    │       │
})                   │       │
    │                │       │
    ▼                │       ▼
handle_actions 结束  │   prompt, tx 被销毁！
    │                │
    ▼                ▼
后台任务还在跑     悬垂引用！💥
```

### 用 `move`（正确）

```
handle_actions 开始
    │
    ▼
创建 prompt, tx
    │
    ▼
tokio::spawn(async move {
    call_llm(...)       ← 闭包拥有 prompt, tx（所有权转移）
})
    │
    ▼
handle_actions 结束   ← prompt, tx 不需要了，闭包自己持有副本
    │
    ▼
后台任务继续跑        ← 安全 ✅
```

---

## 哪些类型会被 move？

`move` 会转移闭包用到的**所有外部变量**的所有权：

```rust
let a = String::from("a");     // String: 没有 Copy，会被 move
let b = 42;                     // i32: 有 Copy，会被复制（不是 move）
let c = vec![1, 2, 3];         // Vec: 没有 Copy，会被 move

let closure = move || {
    println!("{}", a);         // a 被 move 进闭包
    println!("{}", b);         // b 被 Copy 进闭包（外部还能用 b）
    println!("{:?}", c);       // c 被 move 进闭包
};

// a 和 c 不能再用了！它们的所有权已经转移给闭包
// b 还能用，因为 i32 实现了 Copy trait
```

| 类型 | `move` 时的行为 |
|------|----------------|
| `String`, `Vec`, `Box` | 所有权转移（外部不能再使用） |
| `i32`, `bool`, `f64`, `char` | `Copy` 自动复制（外部还能用） |
| `UnboundedSender` | 所有权转移（但通常先 `clone()` 一份） |

---

## 常见模式

### 模式 1：先 clone，再 move

```rust
let tx = self.action_tx.clone();   // 克隆一份
let prompt = prompt.clone();        // 如果需要的话

tokio::spawn(async move {
    // tx 和 prompt 的所有权移进闭包
    // 原始的 self.action_tx 还在，可以继续用
});
```

### 模式 2：只 move 部分变量

```rust
let a = String::from("a");
let b = String::from("b");
let c = String::from("c");

// 如果闭包只用到 a 和 b，c 不会被 move
let f = move || {
    println!("{} {}", a, b);   // 只有 a 和 b 被 move
};

// c 还能用！
println!("{}", c);
```

---

## 一句话总结

> **`move` 把闭包用到的外部变量"搬进"闭包内部，让闭包自己拥有它们。这样闭包就能脱离原始作用域独立存活 —— 这对于 `tokio::spawn` 创建的后台任务来说是必须的。**
