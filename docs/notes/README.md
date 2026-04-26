# 速记卡与概念笔记

这里收录了学习 Rust + Ratatui 过程中遇到的核心概念解释。每篇笔记都试图回答一个"为什么"。

---

## 笔记列表

| 笔记 | 回答的问题 |
|------|-----------|
| [所有权](ownership.md) | Owned / Borrowed / Move、`&`/`&mut`、借用检查器、Copy vs Clone |
| [`async move`](async-move.md) | 为什么 `tokio::spawn` 里必须用 `move`、闭包捕获方式、生命周期 |
| [Trait 是什么？](what-is-trait.md) | Trait 的定义、为什么需要它、和 Java interface 的区别 |
| [`self` vs `this`](self-vs-this.md) | Rust `self` 参数和 JS/Java `this` 的根本区别 |
| [impl 块拆分](impl-blocks.md) | 为什么 `impl Type` 和 `impl Trait for Type` 要分开写 |

---

*这些笔记随代码演进持续更新。如果发现哪里讲错了，欢迎开 issue 指出。*
