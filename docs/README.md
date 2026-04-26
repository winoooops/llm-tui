# Coding Agent TUI 文档

用 Rust + Ratatui 构建 Coding Agent 终端界面的学习记录。

---

## 教程

按顺序阅读，每一步都依赖前一步。

| 编号 | 内容 | 产出 |
|------|------|------|
| [Tutorial 00](tutorials/00-local-llm-preparation.md) | 本地 LLM 环境准备 | 安装并运行 llama.cpp server |
| [Tutorial 01](tutorials/01-chat-component.md) | 第一个 Chat 组件 | 本地输入 + 显示的聊天界面 |
| [Tutorial 02a](tutorials/02a-llm-preparation.md) | 准备 LLM 集成 | 添加 reqwest，扩展 Action 枚举 |
| [Tutorial 02b](tutorials/02b-send-message.md) | Chat 发送消息 | 按 Enter 发送 Action::SendMessage |
| [Tutorial 02c](tutorials/02c-streaming-llm.md) | App 调用 LLM | 异步 HTTP + SSE 流式解析 |
| [Tutorial 02d](tutorials/02d-display-response.md) | 显示回复并测试 | Chat 实时渲染 LLM 流式输出 |
| [Tutorial 03](tutorials/03-memory-context.md) | 给 Chat 添加记忆 | LLM 记住多轮对话上下文 |

## 速记卡与概念笔记

- [笔记索引](notes/README.md) — 所有权、trait、`self` vs `this`、impl 块拆分等


