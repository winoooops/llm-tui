# llm-tui

[![CI](https://github.com/winoooops/llm-tui/workflows/CI/badge.svg)](https://github.com/winoooops/llm-tui/actions)

A **full-functional coding agent TUI** powered by local LLMs. Built with Rust and [Ratatui](https://github.com/ratatui/ratatui).

> **Status:** Early development. Currently a working chat interface with streaming LLM responses. The long-term goal is a terminal-based coding assistant that can read your project, discuss code, and help you write — all running locally on your own hardware.
>
> This project is also a **documented learning journey** into Rust and terminal UI development. Every major feature is accompanied by a step-by-step tutorial.

---

## Features

- **Streamed responses** — See the LLM reply token-by-token in real time
- **Local-first** — Talks to your own hardware; no API keys or cloud required
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

## Quick Start

```bash
# 1. Clone
git clone https://github.com/winoooops/llm-tui.git
cd llm-tui

# 2. Start your local LLM server (OpenAI-compatible API)
#    Example with llama.cpp:
./server -m your-model.gguf --port 8080

# 3. Build and run
cargo build --release
cargo run
```

Then type your message and press **Enter** to chat. Press **Esc** to quit.

> **Note:** Screenshots and demo recordings will be added soon.

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

## Roadmap

| Phase | Goal | Status |
|-------|------|--------|
| **Phase 1: Chat** | Basic chat UI with streaming LLM responses | ✅ Done |
| **Phase 2: Context** | Conversation history, multi-turn dialogue | 🔄 Next |
| **Phase 3: Workspace** | File tree panel, read project files into context | 📋 Planned |
| **Phase 4: Code** | Syntax highlighting, diff view, code block extraction | 📋 Planned |
| **Phase 5: Agent** | Tool use (file read/write, shell commands), agent loop | 📋 Planned |
| **Phase 6: Harness** | Deploy, package, and harness into daily workflow | 📋 Planned |

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

## Requirements

- Rust toolchain (nightly recommended for `cargo fmt` and `cargo clippy`)
- A local LLM server with an **OpenAI-compatible API**
  - Tested with [llama.cpp server](https://github.com/ggerganov/llama.cpp/blob/master/examples/server/README.md)
  - Should also work with Ollama, vLLM, etc.

## License

See [LICENSE](LICENSE).

---

*Built while learning Rust. If you spot something odd, open an issue — feedback is welcome.*
