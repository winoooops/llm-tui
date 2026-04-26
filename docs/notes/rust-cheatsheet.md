# Rust + Ratatui 语法速查

快速查阅，不展开解释。需要深入理解请翻看对应的概念笔记。

---

## Rust 语法

### 变量

```rust
let x = 5;           // 不可变
let mut y = 5;       // 可变
const MAX: i32 = 100;// 编译期常量
```

### 引用与借用

```rust
&x           // 不可变引用
&mut x       // 可变引用
*x           // 解引用
```

> 深入理解：[`所有权`](./ownership.md)

### 常用类型

| 类型 | 创建 | 说明 |
|------|------|------|
| `String` | `String::from("s")` / `"s".to_string()` | 堆上，可增长，拥有所有权 |
| `&str` | `"hello"` | 字符串切片，借用 |
| `Vec<T>` | `Vec::new()` / `vec![1, 2]` | 动态数组 |
| `Option<T>` | `Some(v)` / `None` | 可能有值 |
| `Result<T, E>` | `Ok(v)` / `Err(e)` | 可能失败 |
| `Box<T>` | `Box::new(v)` | 堆指针 |

### 控制流

```rust
match x {
    1 => ...,
    2 | 3 => ...,
    _ => ...,
}

if let Some(v) = opt { ... }

while let Some(x) = iter.next() { ... }

let result = if cond { a } else { b };   // if 是表达式
```

### 结构体与方法

```rust
struct Foo { a: i32 }

impl Foo {
    fn new() -> Self { Self { a: 0 } }
}

impl Trait for Foo {
    fn required(&self) { ... }
}
```

> 深入理解：[`impl 块拆分`](./impl-blocks.md)、[`Trait 是什么`](./what-is-trait.md)

### 迭代器

```rust
vec.iter().map(|x| x + 1).collect::<Vec<_>>()
```

### 闭包

```rust
|x| x + 1                    // 单参数
|x, y| x + y                // 多参数
move |x| x + captured       // 捕获环境变量
```

> 深入理解：[`async move`](./async-move.md)

### 错误处理

```rust
fn foo() -> Result<(), Error> {
    let file = read_file()?;   // ? 自动传播错误
    Ok(())
}
```

### 所有权常用操作

```rust
s.clone()        // 深拷贝
let _ = expr;    // 忽略返回值
```

> 深入理解：[`所有权`](./ownership.md)

---

## Ratatui 速查

### 渲染链

```rust
let text = Text::from(vec![Line::from("hello")]);
let para = Paragraph::new(text)
    .block(Block::default().title("Title").borders(Borders::ALL))
    .wrap(Wrap { trim: true });
frame.render_widget(para, area);
```

### 布局

```rust
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
    .split(area);

// chunks[0] = 上 80%, chunks[1] = 下 20%
```

### 常用控件

| 控件 | 用途 |
|------|------|
| `Block` | 带边框和标题的容器 |
| `Paragraph` | 多行文字 |
| `Borders::ALL` | 四边边框 |
| `Wrap { trim: true }` | 自动换行 |
| `Style::default().fg(Color::Yellow)` | 前景色 |

---

## 本项目常用模式

| 场景 | 写法 |
|------|------|
| 忽略未使用变量 | `let _ = foo;` |
| 解包 Option | `if let Some(ref x) = opt { ... }` |
| 忽略 Result | `let _ = tx.send(action);` |
| 构造函数 | `pub fn new() -> Self { Self { ... } }` |
| 字符串拼接 | `format!("Hello {}", name)` |
| 追加到 Vec | `vec.push(item)` |
| 删除 String 末尾 | `s.pop()` |
