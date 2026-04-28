# SRP：System Prompt 该归谁管？

> 场景：你在纠结 `system_prompt` 应该放在 `llm.rs` 里组装，还是放在 `Chat` 里组装。这篇用**单一职责原则**（SRP）给你一个判断标准。

---

## SRP 的核心问题

Robert C. Martin 的定义：

> **A module should have only one reason to change.**
>
> （一个模块只应该有一个变更理由。）

"变更理由"不是"代码变了"，而是"谁要求你变"。如果产品经理让你改 A，测试工程师让你改 B，两者都会 touch 同一个文件，那这个文件就违反了 SRP。

---

## 方案 A：llm.rs 组装 System Prompt

假设你把 `PromptContext::from_environment().system_prompt()` 塞进 `llm.rs`：

```rust
// src/llm.rs
pub async fn stream_chat(messages: Vec<Message>, tx: UnboundedSender<Action>) {
    // ❌ llm.rs 被迫知道 Prompt 怎么组装
    let system = PromptContext::from_environment().system_prompt();
    let api_messages = vec![&system, ...messages];
    // ... HTTP 调用
}
```

现在问：`llm.rs` 会因为什么原因被修改？

| 变更来源 | 变更内容 | 是否 touch llm.rs |
|---------|---------|-----------------|
| 后端 API | llama.cpp 升级，响应格式变了 | ✅ |
| 网络库 | reqwest 换成 hyper | ✅ |
| 产品经理 | 用户想自定义 system prompt 模板 | ✅ |
| 新功能 | 支持 Python 项目（加 pyproject.toml 探测） | ✅ |
| 新功能 | 动态区加入 git branch 信息 | ✅ |
| 用户体验 | Prompt 太长，需要截断策略 | ✅ |

**6 个不同的变更理由**，都来自不同的利益相关方，都会修改同一个文件。这就是 SRP  violation。

更隐蔽的问题：**依赖方向反了**。

```
llm.rs ──→ prompt.rs ──→ 文件系统（Cargo.toml、README.md）
```

HTTP 传输层（最底层的基础设施）被迫依赖文件系统布局。这意味着你测试 `stream_chat` 时，要么真的去读盘，要么 mock 整个文件系统——而测试 HTTP 本来只需要 mock 一个 TCP 连接。

---

## 方案 B：Chat 组装，llm.rs 只收成品

```rust
// src/components/chat/mod.rs
pub struct Chat {
    system_prompt: Message,  // ← 启动时组装好
    // ...
}

impl Chat {
    pub fn new() -> Self {
        let system_prompt = PromptContext::from_environment().system_prompt();
        Self { system_prompt, ... }
    }
}

// src/llm.rs
pub async fn stream_chat(
    system: &Message,      // ← 由调用方传入
    messages: &[Message],
    tx: UnboundedSender<Action>,
) {
    let api_messages = vec![system, ...messages];
    // ... 纯 HTTP，不关心内容从哪来
}
```

现在再看变更理由：

**llm.rs 的变更理由**：

| 变更来源 | 变更内容 | 是否 touch llm.rs |
|---------|---------|-----------------|
| 后端 API | 响应格式变了 | ✅ |
| 网络库 | reqwest 换成 hyper | ✅ |
| 产品经理 | 用户想自定义 system prompt | ❌ |
| 新功能 | 支持 Python 项目 | ❌ |

**只剩 2 个理由，而且都跟 HTTP 有关。**

**Chat / prompt 模块的变更理由**：

| 变更来源 | 变更内容 | 是否 touch prompt 代码 |
|---------|---------|----------------------|
| 产品经理 | 用户想自定义 system prompt | ✅ |
| 新功能 | 支持 Python 项目 | ✅ |
| 新功能 | 动态区加入 git branch | ✅ |
| 用户体验 | Prompt 太长，截断策略 | ✅ |

**4 个理由，全部跟"怎么描述项目上下文"有关。**

---

## 判断标准："如果换一个传输方式，要不要改这段代码？"

假设明天你们不用 llama.cpp 了，改用 Claude API 或 Gemini。问自己：

- **llm.rs 要不要改？** → 当然要改，HTTP endpoint、认证头、响应格式全不同。
- **system prompt 组装逻辑要不要改？** → **不应该改。** 无论后端换成谁，"告诉 LLM 当前是 Rust 项目"这件事是一样的。

如果两段代码**变更理由不同**，它们就不应该待在同一个模块里。

---

## 类比：快递系统

| 角色 | 职责 | 知道的细节 |
|------|------|-----------|
| 你（Chat） | 决定寄什么、写什么纸条 | 收件人是谁、里面是什么 |
| 快递公司（llm.rs） | 把包裹从 A 运到 B | 只认地址和重量，不认内容 |
| 纸箱厂（prompt.rs） | 根据物品大小做合适的箱子 | 物品尺寸、易碎标识 |

你不会让快递小哥自己决定用什么纸箱、写什么祝福语——他的职责是**运输**。同样，你不会让 `llm.rs` 决定 system prompt 写什么——它的职责是**把消息发到 API**。

---

## 进阶：那 prompt.rs 该独立成模块，还是挂在 Chat 下面？

两种都可以，取决于复杂度：

| 规模 | 做法 | 理由 |
|------|------|------|
| 最小版 | `Chat::new()` 里直接调 `PromptContext` | 代码少，没必要多一层 |
| 中等 | `prompt.rs` 独立模块，`Chat` 持有 `Message` | 测试 `assemble_system_message` 不需要启动整个 Chat |
| 大型 | `ContextSource` trait + 多个 detector | 支持插件式扩展（git、MCP、云资源） |

你现在的规模，独立 `prompt.rs` + `Chat` 持有 `system_prompt: Message` 是最平衡的。

---

## 记忆口诀

> **`llm.rs` 是管道，不是厨子。**
>
> 厨子（Chat/prompt）决定做什么菜、放什么料。
> 管道（llm.rs）只负责把菜从厨房端到餐桌。
> 让管道决定放盐还是放糖，就是 SRP violation。
