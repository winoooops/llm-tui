# 速记卡与概念笔记

这里收录了学习 Rust + Ratatui 过程中遇到的核心概念。

**阅读建议：**
- **概念笔记** — 逐个阅读，建立正确的 Rust 心智模型
- **语法速查** — 写代码时快速翻阅，不求深入理解

---

## 概念笔记（建议按顺序阅读）

| 笔记 | 回答的问题 | 前置知识 |
|------|-----------|---------|
| [所有权](ownership.md) | 什么是 Owned / Borrowed / Move？`&` 和 `&mut` 的区别？Copy vs Clone？ | 无 |
| [Trait 是什么](what-is-trait.md) | Trait 的定义？为什么需要它？和 Java interface 的区别？ | 无 |
| [impl 块拆分](impl-blocks.md) | 为什么 `impl Type` 和 `impl Trait for Type` 要分开？ | 了解 trait 后 |
| [`self` vs `this`](self-vs-this.md) | Rust `self` 参数和 JS/Java `this` 的根本区别？ | 了解所有权后 |
| [`async move`](async-move.md) | 为什么 `tokio::spawn` 里必须用 `move`？闭包捕获方式？ | 了解所有权后 |

## 语法速查（不需要顺序阅读）

| 笔记 | 用途 |
|------|------|
| [Rust + Ratatui 语法速查](rust-cheatsheet.md) | 变量、引用、类型、控制流、迭代器、Ratatui 布局与控件、项目常用写法 |

---

*概念笔记随代码演进持续更新。如果发现哪里讲错了，欢迎开 issue 指出。*
