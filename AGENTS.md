# llm-tui — Agent Guide

This document captures everything an AI coding agent needs to know to work effectively in this codebase.

---

## Project Overview

`llm-tui` is a **Rust TUI coding agent powered by local LLM** built on top of [ratatui](https://github.com/ratatui/ratatui). It provides an opinionated starter structure for building async terminal applications with configurable keybindings, component-based UI, and cross-platform release automation.

- **Repository**: https://github.com/winoooops/llm-tui
- **Language**: Rust (Edition 2024)
- **Runtime**: Tokio (full feature set)
- **TUI Framework**: ratatui 0.30 with crossterm backend
- **License**: See `LICENSE` file

---

## Technology Stack

| Concern              | Library / Tool                                   |
|----------------------|--------------------------------------------------|
| Async runtime        | tokio                                            |
| TUI rendering        | ratatui (with serde + macros features)           |
| Terminal I/O         | crossterm (with serde + event-stream)            |
| CLI parsing          | clap (derive, cargo, wrap_help, unicode, string) |
| Error handling       | color-eyre, anyhow (build only)                  |
| Panic reporting      | human-panic (release), better-panic (debug)      |
| Configuration        | config crate + json5 + serde                     |
| Logging / tracing    | tracing + tracing-subscriber + tracing-error     |
| Build-time metadata  | vergen-gix (git describe, build date, cargo)     |
| Testing assertions   | pretty_assertions                                |
| Cross-compilation    | GitHub Actions with `cross` / `use-cross`        |

---

## Build & Development Commands

```bash
# Build debug binary
cargo build

# Run the app locally
cargo run

# Run tests (used in CI)
cargo test --locked --all-features --workspace

# Format check
cargo fmt --all --check

# Lint (CI treats warnings as errors)
cargo clippy --all-targets --all-features --workspace -- -D warnings

# Build release binary (optimized for size)
cargo build --release

# Generate docs (CI also checks this)
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --all-features --workspace --examples
```

### Local Development Environment

The repository includes a `.envrc` (direnv) that sets three environment variables when entering the directory:

- `LLM_TUI_CONFIG` → `pwd/.config`
- `LLM_TUI_DATA`   → `pwd/.data`
- `LLM_TUI_LOG_LEVEL` → `debug`

This ensures the app reads config from the repo-local `.config/` directory and writes logs to `.data/` instead of OS-specific directories.

---

## Project Structure

```
├── Cargo.toml              # Package manifest, dependencies, release profile
├── build.rs                # vergen-gix metadata injection
├── src/
│   ├── main.rs             # Entry point: init errors/logging, parse CLI, run App
│   ├── app.rs              # App struct: event loop, mode state, component registry
│   ├── action.rs           # Action enum: Tick, Render, Quit, Resize, Suspend, etc.
│   ├── cli.rs              # Clap derive-based CLI arguments (tick_rate, frame_rate)
│   ├── config.rs           # Config loading, keybinding parsing, style parsing
│   ├── errors.rs           # color-eyre + human-panic / better-panic setup
│   ├── logging.rs          # tracing subscriber with file appender
│   ├── tui.rs              # Tui wrapper: raw mode, alternate screen, event loop
│   ├── components.rs       # Component trait definition
│   └── components/
│       ├── home.rs         # Default "hello world" screen
│       └── fps.rs          # FPS / tick-rate overlay component
├── .config/
│   └── config.json5        # Default keybindings shipped as compile-time default
├── .github/workflows/
│   ├── ci.yml              # CI: test, rustfmt, clippy, docs
│   └── cd.yml              # CD: cross-platform binaries + crates.io publish
└── .envrc                  # direnv local dev overrides
```

---

## Runtime Architecture

### Event Loop

1. `main.rs` parses CLI args and constructs `App`.
2. `App::run()` creates a `Tui`, enters raw mode / alternate screen, and starts an async event loop.
3. The `Tui` spawns a Tokio task that multiplexes three sources via `tokio::select!`:
   - **Tick timer** (`tick_rate`, default 4 Hz) → `Event::Tick`
   - **Render timer** (`frame_rate`, default 60 Hz) → `Event::Render`
   - **crossterm event stream** (`EventStream`) → key, mouse, resize, focus, paste
4. `App::handle_events()` maps `Event`s to `Action`s and forwards them to all registered components.
5. `App::handle_actions()` drains the action channel, updates global state, and calls `Component::update()` on every component.
6. `App::render()` calls `Component::draw()` on every component in registration order.

### Action System

All state changes flow through the `Action` enum (defined in `src/action.rs`). Components send actions via an `UnboundedSender<Action>` channel. The `App` owns the receiver and acts as a central dispatcher.

### Component Model

Every UI element implements the `Component` trait (`src/components.rs`):

- `register_action_handler` — receive the action sender
- `register_config_handler` — receive the merged `Config`
- `init` — one-time setup with terminal area
- `handle_events` / `handle_key_event` / `handle_mouse_event` — input handling
- `update` — react to an action and optionally emit another
- `draw` — render to a `ratatui::Frame`

The default app registers two components:

- `Home` — renders "hello world"
- `FpsCounter` — displays current tick rate and FPS in the top-right corner

### Modes

`App` tracks a `Mode` enum (currently only `Mode::Home`). Keybindings and styles are mode-scoped in the configuration file.

---

## Configuration System

### Sources (loaded in order, later overrides earlier)

1. **Compile-time defaults** embedded from `.config/config.json5`
2. **User config files** (searched in OS config dir, overrideable via `LLM_TUI_CONFIG`):
   - `config.json5`
   - `config.json`
   - `config.yaml`
   - `config.toml`
   - `config.ini`

### Supported config sections

- `keybindings` — map of `Mode → { "<key-seq>": "ActionName" }`
- `styles` — map of `Mode → { "identifier": "foreground on background modifiers" }`

### Keybinding syntax

- Single keys: `<q>`, `<esc>`, `<enter>`, `<space>`
- Modifiers: `<ctrl-d>`, `<alt-f4>`, `<shift-tab>`
- Chords: `<g><i>` (parsed from `"<g><i>"`)

See `src/config.rs` for the full parser (`parse_key_sequence`, `parse_key_event`, `key_event_to_string`).

### Style syntax

Examples: `red`, `bold underline green on blue`, `rgb123`, `gray10`

See `src/config.rs` (`parse_style`, `process_color_string`, `parse_color`) for the full grammar.

---

## Testing Strategy

- **Unit tests** live in `src/config.rs` under `#[cfg(test)]`.
- They cover key event parsing, style parsing, color processing, and basic config deserialization.
- **CI command**: `cargo test --locked --all-features --workspace`
- There is no dedicated `tests/` integration test directory yet.

---

## CI / CD

### CI (`ci.yml`)

Triggered on pushes to `main` and on all pull requests. Jobs:

- **Test** — `cargo test --locked --all-features --workspace`
- **Rustfmt** — `cargo fmt --all --check`
- **Clippy** — `cargo clippy --all-targets --all-features --workspace -- -D warnings`
- **Docs** — `cargo doc --no-deps --document-private-items --all-features --workspace --examples` with `RUSTDOCFLAGS=-D warnings`

All jobs run on `ubuntu-latest` using the **nightly** Rust toolchain.

### CD (`cd.yml`)

Triggered on tags matching `[v]?[0-9]+.[0-9]+.[0-9]+`. Jobs:

- **Cross-platform binary builds** for:
  - macOS x86_64 & arm64
  - Linux x86_64, aarch64, i686
  - Windows x86_64
- **GitHub Release** creation with `tar.gz` + SHA256 checksums
- **crates.io publish** (`cargo publish`)

---

## Code Style Guidelines

- Use the **nightly** toolchain for CI formatting and linting.
- Keep `clippy` clean; CI treats warnings as errors.
- Follow standard Rust naming (`PascalCase` for types/traits, `snake_case` for functions/variables, `SCREAMING_SNAKE_CASE` for constants).
- The project uses `color_eyre::Result<()>` as the primary error type in most modules.
- Unused parameters are silenced with `let _ = var;` to appease clippy (this pattern is intentionally used in the `Component` trait defaults).

---

## Security & Error Handling Considerations

- **Terminal restoration on panic**: `errors.rs` installs a custom panic hook that attempts to exit raw mode / alternate screen before printing the panic report, preventing a stuck terminal.
- **Human-friendly panics in release**: `human_panic` generates a crash report file and friendly message for end users.
- **Rich backtraces in debug**: `better_panic` is active only in debug builds.
- **No secrets in source**: The app does not handle authentication, network credentials, or cryptographic material. Standard care applies if you add such features.

---

## Adding a New Component

1. Create `src/components/my_component.rs`.
2. Implement the `Component` trait.
3. Add `pub mod my_component;` to `src/components.rs`.
4. Register the component in `App::new()` inside `src/app.rs` (push onto `self.components`).
5. If you introduce a new `Mode`, extend the `Mode` enum in `src/app.rs` and add corresponding keybindings in `.config/config.json5`.

---

## Notes for Agents

- Do **not** assume the presence of `tests/` or `benches/` directories; they do not exist yet.
- The `Component` trait contains many default no-op implementations; only override what you need.
- When modifying keybindings or styles, update the embedded `.config/config.json5` defaults if the change should apply to all users.
- The `build.rs` file relies on `vergen-gix`; if you change git history or tags, rebuild to refresh the version string shown in `--version`.
- The `release` profile in `Cargo.toml` is aggressively optimized for binary size (`opt-level = "s"`, `lto = true`, `strip = true`). Keep this in mind if you need debug symbols in release builds.
