# Tutorial 02d：显示 LLM 回复并测试

> **目标**：Chat 组件处理 `ReceiveChunk`，实时显示流式回复。  
> **前置要求**：已完成 [Tutorial 02c](02c-streaming-llm.md)。

---

### B5. 在 Chat 里显示 LLM 回复

打开 `src/components/chat.rs`，修改 `update` 方法：

```rust
fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
    match action {
        Action::ReceiveChunk(chunk) => {
            if let Some(last) = self.messages.last_mut() {
                if last.starts_with("AI: ") {
                    last.push_str(&chunk);
                } else {
                    self.messages.push(format!("AI: {}", chunk));
                }
            } else {
                self.messages.push(format!("AI: {}", chunk));
            }
        }
        _ => {}
    }
    Ok(None)
}
```

**逻辑：**

- 收到 `ReceiveChunk`
- 看聊天记录的最后一条：
  - 如果以 `"AI: "` 开头 → 这是同一次回复的后续 chunk，**追加**到末尾
  - 否则 → 这是新回复的第一块，**新建一条消息**

**`.last_mut()` 是什么？**

- `.last()` = 返回 `Option<&T>`，不可变引用
- `.last_mut()` = 返回 `Option<&mut T>`，可变引用
- 因为我们要修改最后一条消息（追加文字），所以必须用 `_mut` 版本

**`.starts_with("AI: ")`**

`String` 的方法，检查字符串是否以某个前缀开头。返回 `true` 或 `false`。

---

### B6. 编译并测试

1. 确保你的 llama.cpp server 正在运行：
   ```bash
   # 你的启动命令，比如：
   ./server -m gemma-4-31b.gguf --port 8080
   ```

2. 编译：
   ```bash
   cargo build
   ```
   第一次会很慢，因为 reqwest 有很多依赖。

3. 运行：
   ```bash
   cargo run
   ```

4. 打字，按 Enter，你应该看到：
   - 你的消息立刻出现在上方
   - 稍等片刻，LLM 的回复开始逐字出现在聊天记录里
   - 按 Esc 退出

---

## 概念检查清单

1. **为什么 `tokio::spawn` 是必须的？**
2. **`async move` 里的 `move` 起什么作用？**
3. **SSE 是什么？为什么 LLM API 要用这种格式？**
4. **`serde_json::Value` 和定义结构体相比有什么优缺点？**
5. **`.last_mut()` 和 `.last()` 有什么区别？**

---

*跑通后告诉我，我们进入 Step C：多面板布局。*
