# `self` (Rust) vs `this` (JS/Java)

Rust 的 `self` 和 JavaScript/Java 的 `this` 看起来相似，但底层设计哲学完全不同。

---

## JS / Java 的 `this` — 隐式、总是可用

### JavaScript
```javascript
class Chat {
    waiting = false;
    
    startWaiting() {
        this.waiting = true;   // this 自动存在，随时可用
    }
    
    isWaiting() {
        return this.waiting;   // 读也用 this
    }
}
```

### Java
```java
class Chat {
    private boolean waiting = false;
    
    public void startWaiting() {
        this.waiting = true;   // this 隐式传入，默认可读写
    }
    
    public boolean isWaiting() {
        return this.waiting;   // 读也用 this
    }
}
```

**特点：**
- `this` 是**隐式**的，你不用声明它，它自动存在
- `this` **总是指向实例**，且总是可读可写
- 没有"所有权"概念，方法调用完实例还在那里

---

## Rust 的 `self` — 显式、可选三种形式

```rust
impl Chat {
    fn start_waiting(&mut self) {    // ← 必须显式声明 &mut self
        self.waiting_for_response = true;
    }
    
    fn is_waiting(&self) -> bool {   // ← 显式声明 &self
        self.waiting_for_response
    }
}
```

**特点：**

| 特点 | JS/Java | Rust |
|------|---------|------|
| `this`/`self` | 隐式，自动存在 | **必须显式写在参数列表里** |
| 访问方式 | 只有一种（总是引用实例） | 三种选择：`self`、`&self`、`&mut self` |
| 修改权限 | 默认可读写 | 必须显式申请 `&mut` |
| 调用后实例是否可用 | 总是可用 | 取决于你用哪种 `self` |

### `self` 的三种形式

| 写法 | 全称 | 能不能读 | 能不能改 | 调用后实例还在吗？ |
|------|------|---------|---------|------------------|
| `fn foo(self)` | `self: Self` | ✅ | ✅ | ❌ 被 move 走了 |
| `fn foo(&self)` | `self: &Self` | ✅ | ❌ | ✅ 还在 |
| `fn foo(&mut self)` | `self: &mut Self` | ✅ | ✅ | ✅ 还在 |

---

## 为什么要设计得这么"麻烦"？

因为 Rust 的 `self` 参数不只是**语法**，而是**所有权声明**。

### JS/Java 的问题（在 Rust 看来）

```javascript
const chat = new Chat();
chat.startWaiting();   // chat 还在
chat.startWaiting();   // chat 还能用，随便调用多少次
```

这在 Rust 里看起来是**模糊了所有权**：
- `startWaiting` 到底会不会消耗掉 `chat`？
- 能不能同时有两个方法在修改 `chat`？
- 方法内部会不会把 `chat` 的某个字段 move 走？

JS/Java 的运行时帮你处理了这些问题（垃圾回收、引用计数），代价是运行时 bug（race condition、意外 mutation）。

### Rust 的做法：在函数签名里就写清楚

```rust
// 1. 这个会消耗 chat，调用后 chat 没了
fn into_history(self) -> Vec<String> { self.messages }

// 2. 这个只读，可以随便调用无数次
fn is_waiting(&self) -> bool { self.waiting }

// 3. 这个要修改，同时只能有一个调用
fn start_waiting(&mut self) { self.waiting = true; }
```

**调用方一看函数签名就知道规则：**

```rust
let chat = Chat::new();

chat.into_history();    // ❌ 之后 chat 不能用了！编译器会拦
chat.is_waiting();      // ✅ 可以调用无数次
chat.start_waiting();   // ✅ 但如果有另一个 &mut chat 同时存在，编译器会拦
```

---

## 另一个关键区别：`self` 不是关键字，是参数名

在 Rust 里，`self` 本质上是一个**特殊的参数名**，不是全局关键字：

```rust
// 这三行完全等价：
fn foo(self: Self)           // 显式写全
fn foo(self)                 // 简写
fn foo(mut self)             // 甚至可以加 mut！
```

甚至你可以不用 `self`：

```rust
impl Chat {
    // 关联函数（类似 JS 的 static method）
    fn new() -> Self {        // ← 没有 self，不能访问实例字段
        Self { ... }
    }
}
```

这在 JS/Java 里需要 `static` 关键字来区分，Rust 直接看有没有 `self` 参数就行。

---

## 一句话总结

> JS/Java 的 `this` 是**隐式环境变量**，Rust 的 `self` 是**显式参数声明**。Rust 要求你在函数签名里就写清楚"我要怎么用这个实例"，这样编译器能在调用前就保证安全，而不是等到运行时才发现问题。
