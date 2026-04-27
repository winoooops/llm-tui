# `Into<T>` 与 `From<T>`：Rust 的类型转换哲学

> 场景：你看到函数签名里写了 `impl Into<String>`，想知道这是什么意思、该怎么用、什么时候该实现它。

---

## 一句话定义

`Into<T>` 表示**"我可以把自己变成 T"**。任何实现了 `Into<String>` 的类型，都可以调用 `.into()` 得到一个 `String`。

```rust
// 这个函数接受"任何能变成 String 的东西"
pub fn greet(name: impl Into<String>) {
    let s: String = name.into();
    println!("Hello, {s}!");
}

greet("world");        // ✅ &str → String
greet(String::from("x")); // ✅ String → String（自己转自己）
```

---

## `Into` 的核心特性

### 1. 值到值的转换（move）

```rust
let s = "hello";              // s 是 &str
let owned: String = s.into(); // s 被 move（消费）了，产出 String
// 之后不能再使用 s
```

`into()` 会**吃掉**输入值。如果输入是借用的（`&str`），它会克隆数据然后返回 owned 值；如果输入已经是 owned（`String`），它可能直接返回原值或做少量包装。

### 2. 保证成功

`Into` 转换**不能失败**。如果可能失败（比如 `"abc".into::<i32>()`），要用 `TryInto`，它返回 `Result`：

```rust
let s: String = "hello".into();    // ✅ Into，保证成功
let n: i32 = "42".parse()?;        // ✅ TryInto / FromStr，可能失败，返回 Result
```

### 3. `From` 和 `Into` 是双向关系

| 写法 | 含义 | 主语 |
|------|------|------|
| `String::from("hello")` | 从 `&str` **构造出** `String` | 目标类型 |
| `"hello".into::<String>()` | 把 `&str` **变成** `String` | 源类型 |

结果完全一样，只是语法角度不同。

---

## 实现原则：写 `From`，不写 `Into`

标准库有一条**空实现**（blanket impl）：

```rust
impl<T, U> Into<U> for T where U: From<T> { ... }
```

翻译：**只要 `U` 能 `From<T>`，`T` 自动就能 `Into<U>`**。

所以：

```rust
// ✅ 正确：实现 From，Into 会免费送
impl From<&str> for MyType { ... }

// ❌ 没必要：标准库已经帮你做了
impl Into<MyType> for &str { ... }
```

---

## 泛型约束：用 `Into`，不用 `From`

写函数签名时，`Into` 的方向更自然：

```rust
// ✅ 推荐：调用方手里有什么，往目标类型转
fn foo(content: impl Into<String>) { ... }

// ❌ 别扭：约束的是"目标类型能从什么构造"，方向反了
fn foo<T: From<&str>>(content: T) { ... }
```

因为 `Into` 描述的是**调用方的输入能力**，`From` 描述的是**目标类型的构造能力**。前者更直观。

---

## 常见实现者

| 源类型 | 目标类型 | 场景 |
|--------|---------|------|
| `&str` | `String` | 字符串字面量变 owned |
| `String` | `Vec<u8>` | 字符串转字节 |
| `&[T]` | `Vec<T>` | 切片变向量（需要 `T: Clone`）|
| `i32` | `i64` | 小整数转大整数 |
| `MyError` | `Box<dyn std::error::Error>` | 自定义错误转通用错误 |

---

## 在项目中的应用

你的 `Message::system()` 用了 `impl Into<String>`：

```rust
impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: content.into(),  // 这里调用 Into
        }
    }
}
```

好处：调用方可以传 `&str`、`String`、`Cow<str>`，甚至以后自定义的类型——只要它实现了 `Into<String>`。

---

## 记忆口诀

> **`From` 是工厂（"我能从 X 造出来"），`Into` 是通道（"我能变成 Y"）。**
> 
> **实现时写 `From`，用别人时写 `Into`**。
