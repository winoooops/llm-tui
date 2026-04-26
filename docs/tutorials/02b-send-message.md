# Tutorial 02b：Chat 组件发送消息

> **目标**：修改 Chat，让按 Enter 时发送 `Action::SendMessage` 给 App。  
> **前置要求**：已完成 [Tutorial 02a](02a-llm-preparation.md)（添加了 reqwest 和新 Action）。

---

### B3. 修改 Chat 的按键处理

打开 `src/components/chat.rs`，修改 `KeyCode::Enter` 分支：

```rust
KeyCode::Enter => {
    if !self.input.is_empty() {
        let text = self.input.clone();
        self.messages.push(format!("You: {}", text));
        self.input.clear();
        
        if let Some(ref tx) = self.command_tx {
            let _ = tx.send(Action::SendMessage(text));
        }
    }
    Ok(None)
}
```

**发生了什么变化？**

1. 先把输入内容 `clone()` 一份（因为等下要 `clear()`，而发送需要原始内容）
2. 立即把 `"You: xxx"` 显示在聊天记录里（用户立刻看到自己的消息）
3. 清空输入框
4. 通过 `command_tx` 发送 `Action::SendMessage(text)` 给 App

**为什么要 `clone()`？**

Rust 的所有权规则：`self.input.clear()` 会修改 `self.input`，但 `tx.send(...)` 需要拿走 ownership。如果先 `clear()`，内容就没了。所以先用 `clone()` 复制一份发给 App。

**`String` 的 `clone()` 做了什么？**

`String` 存储在堆上。`clone()` 会分配一块新内存，把原来的字符串内容完整复制过去。原来的 `self.input` 不受影响。

---

