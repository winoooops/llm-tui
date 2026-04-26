# Rust 所有权（Ownership）

Rust 最核心的特性，也是和 JS/Java 区别最大的地方。

---

## 一句话定义

> **每个值都有一个所有者。当所有者离开作用域，值就被销毁。所有权可以转移（move），也可以暂时借出（borrow），但永远只有一个活跃的所有者。**

---

## 三种状态

Rust 里的任何数据，在同一时刻只可能处于三种状态之一：

```
┌─────────────┐      move       ┌─────────────┐
│   被拥有     │ ──────────────→ │   被转移     │
│  (owned)    │                 │  (moved)    │
└──────┬──────┘                 └─────────────┘
       │
       │ borrow
       ▼
┌─────────────┐
│   被借用     │
│ (borrowed)  │
│  ├─ &T      │  不可变引用（只读，可多份）
│  └─ &mut T  │  可变引用（可写，唯一）
└─────────────┘
```

---

## 1. 被拥有（Owned）

值属于一个变量。变量就是"所有者"。

```rust
fn main() {
    let s = String::from("hello");   // s 拥有这个 String
    println!("{}", s);                // ✅ 可以用
}                                    // s 离开作用域，String 的内存被自动释放
```

**不需要 `free`，不需要 GC。** Rust 在编译期就插入了销毁代码。

---

## 2. 转移 / Move

把值交给别人，原来的变量**不能再使用**。

```rust
fn main() {
    let s = String::from("hello");   // s 拥有
    let s2 = s;                       // 所有权 move 给 s2
    
    println!("{}", s2);              // ✅ s2 可以用
    println!("{}", s);               // ❌ 编译错误！s 已经被 move
}
```

编译器报错：

```
error[E0382]: borrow of moved value: `s`
```

**这和 JS/Java 完全不同：**

```javascript
// JavaScript
let s = "hello";
let s2 = s;       // 只是复制了引用
console.log(s);   // ✅ 还能用！
```

```java
// Java
String s = "hello";
String s2 = s;    // 引用复制
System.out.println(s);  // ✅ 还能用！
```

在 JS/Java 里，`let s2 = s` 只是复制了引用/指针，两个变量指向同一个对象。  
在 Rust 里，`let s2 = s` 是**所有权转移**，`s` 被"掏空"了。

### 为什么 String 会被 move，但整数不会？

```rust
let x = 5;
let y = x;       // ✅ 没问题，x 还能用
println!("{}", x);
```

因为 `i32` 实现了 `Copy` trait。简单的栈上数据（数字、布尔、char）会被**自动复制**；堆上的复杂数据（`String`、`Vec`）默认是**move**。

| 类型 | 行为 | 原因 |
|------|------|------|
| `i32`, `bool`, `char`, `f64` | Copy | 固定大小，存栈上 |
| `String`, `Vec`, `Box` | Move | 动态大小，存堆上 |

---

## 3. 借用（Borrow）

不想转移所有权，只想暂时用一下？用引用：

### 不可变引用 `&T` — 只读

```rust
fn print_length(s: &String) {
    println!("{}", s.len());   // 只读，不修改
}

fn main() {
    let s = String::from("hello");
    print_length(&s);            // 借给 print_length 看看
    print_length(&s);            // ✅ 还能再借！
    println!("{}", s);           // ✅ 所有权还在 s 手里
}
```

### 可变引用 `&mut T` — 可写

```rust
fn add_exclamation(s: &mut String) {
    s.push('!');                 // 修改借来的值
}

fn main() {
    let mut s = String::from("hello");
    add_exclamation(&mut s);     // 可变借用
    println!("{}", s);           // ✅ 输出 "hello!"
}
```

---

## 借用检查器（Borrow Checker）的铁律

Rust 编译器 enforcing 的规则：

> **对于同一个值，在同一时间：**
> - **要么** 有一个可变引用 `&mut T`
> - **要么** 有任意多个不可变引用 `&T`
> - **不能同时存在**

```rust
let mut s = String::from("hello");

let r1 = &s;      // 不可变引用 1
let r2 = &s;      // 不可变引用 2 ✅
let r3 = &mut s;  // ❌ 编译错误！不能同时有 & 和 &mut

println!("{} {} {}", r1, r2, r3);
```

为什么？防止**数据竞争**（data race）。如果 `r1` 正在读，同时 `r3` 在写，结果不可预测。

```rust
let mut s = String::from("hello");

let r1 = &mut s;   // 可变引用 1
let r2 = &mut s;   // ❌ 编译错误！不能有两个 &mut
```

为什么？防止**同时使用**两个写入者导致状态混乱。

---

## 和 JS/Java 的对比

### 同一个场景，三种写法

**场景：函数接收一个字符串，修改它。**

#### JavaScript
```javascript
function shout(msg) {
    msg += "!!!";       // 修改的是局部副本，原字符串不变
    return msg;
}

let s = "hello";
shout(s);               // s 还是 "hello"（字符串不可变）
```

#### Java
```java
void shout(StringBuilder msg) {
    msg.append("!!!");  // 修改的是同一个对象（引用传递）
}

StringBuilder s = new StringBuilder("hello");
shout(s);               // s 变成了 "hello!!!"
```

#### Rust
```rust
fn shout(msg: &mut String) {
    msg.push_str("!!!");   // 通过可变引用修改，所有权还在调用者
}

let mut s = String::from("hello");
shout(&mut s);              // s 变成了 "hello!!!"
println!("{}", s);          // ✅ 还能用，所有权没丢
```

| 语言 | 传参方式 | 默认是否修改原值 | 安全性 |
|------|---------|----------------|--------|
| JS | 值传递（原始类型）/ 引用传递（对象） | 原始类型不会 | 运行时可能出 bug |
| Java | 引用传递 | 会（如果对象可变） | 运行时可能出 bug |
| Rust | 所有权 / 借用 | 必须显式声明 `&mut` | **编译期就拦住** |

---

## 所有权和 `self` 的关系

现在回头看 Rust 的 `self` 参数，你就能理解为什么有三种形式了：

```rust
impl Chat {
    // self: 拿走所有权，调用后 chat 不能用了
    fn into_messages(self) -> Vec<String> {
        self.messages    // move 走整个 messages，chat 被消耗
    }
    
    // &self: 不可变借用，只读不改
    fn is_waiting(&self) -> bool {
        self.waiting_for_response
    }
    
    // &mut self: 可变借用，可以修改，但调用后 chat 还在
    fn start_waiting(&mut self) {
        self.waiting_for_response = true;
    }
}
```

**`self` 的三种形式，本质上就是所有权规则的直接体现。**

---

## 常见坑

### 坑 1：move 后还想用

```rust
let s = String::from("hello");
let s2 = s;
println!("{}", s);   // ❌ s 已经被 move
```

**修复：** 如果需要两个独立的副本，用 `clone()`：

```rust
let s2 = s.clone();    // 深拷贝，s 和 s2 各自拥有独立数据
println!("{}", s);     // ✅ 没问题
```

### 坑 2：在 `&` 存在时用 `&mut`

```rust
let mut s = String::from("hello");
let r1 = &s;
let r2 = &mut s;       // ❌ 编译错误
println!("{}", r1);    // 如果编译通过，这里可能读到半写完的数据
```

**修复：** 确保引用的生命周期不重叠：

```rust
let mut s = String::from("hello");
{
    let r1 = &s;
    println!("{}", r1);   // r1 在这里用完
}                           // r1 的生命周期结束
let r2 = &mut s;            // ✅ 现在可以创建 &mut
```

### 坑 3：返回局部变量的引用

```rust
fn bad() -> &String {
    let s = String::from("hello");
    &s                      // ❌ s 会在函数结束时被销毁
}                           // 返回悬垂引用！
```

编译器会拒绝，因为 `s` 的生命周期不够长。

---

## 一句话总结

> **所有权让 Rust 不需要垃圾回收器，也不需要手动 free。编译器在编译期就跟踪每个值的生命周期，确保内存安全、线程安全、没有悬垂指针 —— 代价是你要花时间去理解 move、borrow 和生命周期的规则。**
