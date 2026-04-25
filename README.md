# llm-tui

[![CI](https://github.com/winoooops/llm-tui/workflows/CI/badge.svg)](https://github.com/winoooops/llm-tui/actions)

A terminal chat interface for local LLMs, built with Rust and [Ratatui](https://github.com/ratatui/ratatui).

> This project started as a learning journey into Rust and terminal UI development. The codebase is intentionally documented with step-by-step tutorials for anyone who wants to build something similar.

---

## Features

- **Streamed responses** — See the LLM reply token-by-token in real time
- **Local-first** — Talks to your own hardware, no API keys or cloud required
- **Async architecture** — Built on Tokio; UI stays responsive while the model thinks
- **Component-based UI** — Easy to extend with new panels and features

## Tech Stack

| Layer | Choice |
|-------|--------|
| Language | Rust (Edition 2024) |
| TUI Framework | [ratatui](https://github.com/ratatui/ratatui) 0.30 + crossterm |
| Async Runtime | Tokio |
| HTTP Client | reqwest |
| Error Handling | color-eyre |
| Configuration | config crate + json5 |

## Requirements

- Rust toolchain (nightly recommended for `cargo fmt` and `cargo clippy`)
- A local LLM server with an **OpenAI-compatible API**
  - Tested with [llama.cpp server](https://github.com/ggerganov/llama.cpp/blob/master/examples/server/README.md)
  - Should also work with Ollama, vLLM, etc.

## Quick Start

### 1. Start your local LLM server

Example with llama.cpp:

```bash
./server -m your-model.gguf --port 8080
```

Make sure it exposes the `/v1/chat/completions` endpoint.

### 2. Build and run

```bash
cargo build --release
cargo run
```

### 3. Chat

- Type your message and press **Enter** to send
- Watch the response stream in live
- Press **Esc** or **Ctrl-C** to quit

### Local development environment

The repo includes a `.envrc` for [direnv](https://direnv.net/) that keeps config and logs inside the project folder:

```bash
export LLM_TUI_CONFIG=`pwd`/.config
export LLM_TUI_DATA=`pwd`/.data
export LLM_TUI_LOG_LEVEL=debug
```

## Learning from This Project

The entire project was built incrementally, and each step is documented as a tutorial:

| Tutorial | What You Build |
|----------|----------------|
| [01 — Chat Component](docs/tutorials/01-chat-component.md) | A local input + display chat UI |
| [02a — LLM Preparation](docs/tutorials/02a-llm-preparation.md) | Add HTTP client and Action types |
| [02b — Send Message](docs/tutorials/02b-send-message.md) | Wire Chat to emit `Action::SendMessage` |
| [02c — Streaming LLM](docs/tutorials/02c-streaming-llm.md) | Async HTTP request + SSE parsing |
| [02d — Display Response](docs/tutorials/02d-display-response.md) | Render streaming LLM output |

There's also a [study notes](docs/notes/rust-ratatui-study-notes.md) doc covering Rust ownership, references, `Option`/`Result`, and Ratatui's immediate-mode rendering model.

## Project Status

This is a **work in progress**. Current capabilities:

- [x] Basic chat UI (input + scrollable history)
- [x] Connect to local LLM via OpenAI-compatible API
- [x] Streamed response rendering
- [x] Async architecture with Tokio channels
- [ ] Chat history / conversation context
- [ ] Multi-panel layout (file tree + chat)
- [ ] Code syntax highlighting in responses
- [ ] Multiple model support

## Architecture

```
┌─────────────────────────────────────────┐
│                  App                    │
│  ┌─────────────┐    ┌───────────────┐  │
│  │  Event Loop │    │ Action Router │  │
│  └──────┬──────┘    └───────┬───────┘  │
│         │                   │           │
│    ┌────▼────┐         ┌────▼────┐      │
│    │   Tui   │         │  Chat   │      │
│    │(crossterm)        │Component│      │
│    └────┬────┘         └────┬────┘      │
│         │                   │           │
│    Keyboard            ┌────▼────┐      │
│    Timer               │  llm.rs │      │
│    Resize              │(reqwest)│      │
│                        └────┬────┘      │
│                             │           │
│                      Local LLM Server   │
└─────────────────────────────────────────┘
```

All state changes flow through the `Action` enum. Components communicate with `App` via async channels (`tokio::sync::mpsc`).

## License

See [LICENSE](LICENSE).

---

*Built while learning Rust. If you spot something odd, open an issue — feedback is welcome.*
