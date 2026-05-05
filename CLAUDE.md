# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Authoritative reference

**`AGENTS.md` is the long-form agent guide** — read it for tech stack details, configuration grammar (keybindings, styles), CI/CD, and the component-adding checklist. This file only captures essentials and the parts where the codebase has moved past `AGENTS.md`.

## Commands

```bash
cargo run                                                            # run the TUI (needs a local LLM at 127.0.0.1:8080)
cargo test --locked --all-features --workspace                        # full test suite (CI command)
cargo test <name>                                                     # single test by name; tests live in #[cfg(test)] modules
cargo fmt --all --check                                               # CI fmt check (nightly)
cargo clippy --all-targets --all-features --workspace -- -D warnings  # CI clippy (warnings = errors)
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --all-features --workspace --examples
```

CI uses **nightly**. The `release` profile is size-optimized (`opt-level="s"`, `lto=true`, `strip=true`) — don't expect debug symbols there.

## Local environment

`.envrc` (direnv) redirects config and logs into the repo:

- `LLM_TUI_CONFIG=$PWD/.config` — overrides OS config dir
- `LLM_TUI_DATA=$PWD/.data` — log output goes here (`tail -f .data/llm-tui.log`)
- `LLM_TUI_LOG_LEVEL=debug`

The repo ships compile-time defaults via `.config/config.json5` (embedded into the binary); user files in `LLM_TUI_CONFIG` override.

## Runtime model

Standard ratatui template event loop, but extended with a chat path:

- `Tui` (`src/tui.rs`) multiplexes a tick timer, render timer, and crossterm `EventStream` via `tokio::select!`.
- All state changes flow through the `Action` enum (`src/action.rs`). Components emit actions on an `mpsc::UnboundedSender<Action>`; `App` owns the receiver and dispatches.
- `App` (`src/app.rs`) registers components in order: `Home`, `FpsCounter`, `Chat`. Each frame, `App` calls `update()` then `draw()` on every component.
- `Mode` enum currently has only `Mode::Home`; keybindings/styles are mode-scoped.

### Chat / LLM path (the part not in AGENTS.md)

The `Chat` component (`src/components/chat/`) is split into `Input` and `Conversation` submodules. The flow:

1. User input → `Chat` emits `Action::SendMessage(history)` with the full message history.
2. `App::handle_actions` intercepts `SendMessage`, clones the system prompt + history, and `tokio::spawn`s `llm::stream_chat`.
3. `llm::stream_chat` (`src/llm.rs`) POSTs to **`http://127.0.0.1:8080/v1/chat/completions`** (hardcoded), parses SSE, and emits `Action::ReceiveChunk(String)` per token, then `Action::StreamEnd`.
4. `Chat` receives chunks via its `update()` and appends them to the in-progress assistant message.

The model name is **hardcoded as `"gemma-4-31b"`** in `llm.rs`. Any OpenAI-compatible server (llama.cpp, Ollama, vLLM) on port 8080 should work, but the model string is sent verbatim — change it to match what your server serves.

### System prompt assembly

`App` owns the system prompt and injects it at dispatch time; the `Chat` component is unaware of it. `PromptContext::from_environment()` (`src/prompt.rs`) walks the cwd to detect project name / type, reads `README.md` (capped at 500 chars) and `AGENTS.md`, and produces a `Message`. See `docs/tutorials/06-system-prompt-assembly.md` and `07-app-vs-chat-system-prompt.md` for the design.

## Tests

Tests live next to code in `#[cfg(test)]` modules — no `tests/` directory. Coverage is concentrated in `src/config.rs` (key/style parsing) and `src/llm.rs` (SSE parser). `dev-dependencies` includes `tempfile` for filesystem-touching tests. See `docs/tutorials/08-rust-testing-basics.md` and `09-testing-existing-codebase.md`.

## Tutorials as design docs

`docs/tutorials/00-10` document each major feature step-by-step and effectively serve as design notes. When changing chat, prompt, or testing infrastructure, check the corresponding tutorial — it likely explains the *why*. `docs/notes/` covers Rust concept refreshers (ownership, traits, async-move, etc.).

## Drift to be aware of

`AGENTS.md` predates the chat work and still describes only `Home` + `FpsCounter` as registered components and lists no `src/llm.rs`, `src/message.rs`, `src/prompt.rs`, or `src/utils.rs`. Trust the source for the chat/LLM/prompt subsystem; trust `AGENTS.md` for everything else (config, keybindings, CI, component trait, release profile).
