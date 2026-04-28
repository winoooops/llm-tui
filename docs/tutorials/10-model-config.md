# Tutorial 10：Model Config —— 让 TUI 知道它在跟哪个模型说话

> **目标**：抽出一个独立的 `model_config` 模块，把当前硬编码在 `llm.rs` 里的模型信息变成可配置、可扩展的结构。  
> **前置要求**：已完成 [Tutorial 07](07-app-vs-chat-system-prompt.md) 和 [Tutorial 09](09-testing-existing-codebase.md)。  
> **参考规格**：`~/projects/learning/llm-learning/llama-server.md`

---

## 目录

1. [为什么要抽 Model Config？](#为什么要抽-model-config)
2. [Step 1：创建 `src/model_config.rs`](#step-1创建-srcmodel_configrs)
3. [Step 2：注册模块](#step-2注册模块)
4. [Step 3：把硬编码模型名从 `llm.rs` 里拔掉](#step-3把硬编码模型名从-llmrs-里拔掉)
5. [Step 4（可选）：接入 `Config` 让配置可热加载](#step-4可选接入-config-让配置可热加载)
6. [验证](#验证)

---

## 为什么要抽 Model Config？

当前 `llm.rs` 里直接写死了：

```rust
let body = serde_json::json!({
    "model": "gemma-4-31b",
    // ...
});
```

这带来几个问题：

1. **换模型要改代码**。`llama-server` 通过 `--alias` 暴露模型名， `--alias gemma-4-31b` 只是当前配置；哪天切到 Qwen 或 Llama，得重新编译。
2. **上下文窗口不可见**。`llama-server` 跑的是 `-c 65536`，但 TUI 层完全不知道这个数。后续做 token 预算、截断策略、进度条时，需要知道 "64k" 这个上限。
3. **模型 spec 散落**。量化方式 (Q4_K_M)、KV cache 类型 (q8_0)、原生最大上下文 (256k) 这些信息在 `llama-server.md` 里有，但代码里完全没有。做错误提示或状态栏展示时，需要一份结构化数据。
4. **特殊行为开关**。`stfuu`（Shut The Fragment Up Ultra？或者你自己定义）——一个布尔开关，控制模型是否进入"极简回复模式"。先占坑，后续在 prompt 层或 UI 层消费。

---

## 规格速查

来自 `~/projects/learning/llm-learning/llama-server.md`：

| 字段 | 当前值 | 说明 |
|------|--------|------|
| `name` | `gemma-4-31b` | `--alias` 参数，API 请求里的 `model` 字段 |
| `context_window` | `65536` | `-c 65536`，当前实际可用的上下文长度 |
| `native_max_context` | `262144` | 模型原生支持 256k，但受限于 VRAM 只开到 64k |
| `quantization` | `Q4_K_M` | 权重量化级别 |
| `kv_cache_type` | `q8_0` | KV cache 量化，让 64k 能塞进 32GB VRAM |
| `gpu_layers` | `99` | 全部层 offload 到 5090 |

---

## Step 1：创建 `src/model_config.rs`

新建文件：

```rust
use serde::{Deserialize, Serialize};

/// 当前对接的 LLM 模型配置。
/// 来源：llama-server 的启动参数 + 模型元数据。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ModelConfig {
    /// API 请求时使用的模型标识，对应 llama-server 的 `--alias`。
    /// 例："gemma-4-31b"
    pub name: String,

    /// 当前实际启用的上下文窗口长度（token 数）。
    /// 例：65536
    pub context_window: usize,

    /// 模型硬件/量化规格。
    pub spec: ModelSpec,

    /// "Shut The Fragment Up Ultra" — 极简回复模式开关。
    /// true 时可在 system prompt 或 UI 层抑制模型废话。
    /// 先占坑，默认 false。
    #[serde(default)]
    pub stfuu: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ModelSpec {
    /// 模型权重文件名称
    pub weights_file: String,
    /// 权重量化级别，如 "Q4_K_M"
    pub quantization: String,
    /// KV cache 量化类型，如 "q8_0"
    pub kv_cache_type: String,
    /// 模型原生支持的最大上下文（不是当前实际开的）
    pub native_max_context: usize,
    /// GPU offload 层数
    pub gpu_layers: usize,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            name: "gemma-4-31b".into(),
            context_window: 65536,
            spec: ModelSpec::default(),
            stfuu: false,
        }
    }
}

impl Default for ModelSpec {
    fn default() -> Self {
        Self {
            weights_file: "gemma-4-31B-it-Q4_K_M.gguf".into(),
            quantization: "Q4_K_M".into(),
            kv_cache_type: "q8_0".into(),
            native_max_context: 262_144,
            gpu_layers: 99,
        }
    }
}

impl ModelConfig {
    /// 构造一个用于 API 请求体的 `model` 字段值。
    pub fn api_model_name(&self) -> &str {
        &self.name
    }

    /// 计算剩余可用 token 数（简单版）。
    /// 后续可接入 tokenizer 做精确计数。
    pub fn remaining_tokens(&self, used: usize) -> usize {
        self.context_window.saturating_sub(used)
    }
}
```

要点：
- `Serialize` 也 derive 上，方便后续打日志或导出状态。
- `Default` 硬编码当前环境的数据，保证「零配置也能跑」。
- `stfuu` 用 `#[serde(default)]`，这样配置文件里不写它也不会报错。

---

## Step 2：注册模块

文件：`src/main.rs`

在模块声明区加一行：

```rust
mod action;
mod app;
mod cli;
mod components;
mod config;
mod errors;
mod llm;
mod logging;
mod message;
mod model_config;   // ← 新增
mod prompt;
mod tui;
mod utils;
```

---

## Step 3：把硬编码模型名从 `llm.rs` 里拔掉

当前 `llm.rs` 把 `"gemma-4-31b"` 写死在请求体里。我们要让 `stream_chat` 接收一个 `&ModelConfig`，从中取 `model` 名。

### Step 3a：修改 `stream_chat` 签名

文件：`src/llm.rs`

```rust
use crate::{action::Action, message::Message, model_config::ModelConfig};

pub async fn stream_chat(
    model_config: &ModelConfig,   // ← 新增
    system: &Message,
    messages: &[Message],
    tx: UnboundedSender<Action>,
) -> color_eyre::Result<()> {
    let client = reqwest::Client::new();

    let mut api_messages: Vec<&Message> = vec![system];
    api_messages.extend(messages.iter());

    let body = serde_json::json!({
        "model": model_config.api_model_name(),   // ← 不再是硬编码
        "messages": api_messages,
        "stream": true
    });

    // ... 以下不变
```

### Step 3b：App 层持有 ModelConfig 并传入

文件：`src/app.rs`

在 `App` 结构体里加字段：

```rust
pub struct App {
    // ...
    pub model_config: ModelConfig,
}
```

在 `App::new()` 里初始化：

```rust
impl App {
    pub fn new(tick_rate: f64, frame_rate: f64) -> Result<Self> {
        let model_config = ModelConfig::default();
        // ...
        Ok(Self {
            // ...
            model_config,
        })
    }
}
```

在 `handle_actions` 里把 `model_config` 传给 `llm::stream_chat`：

```rust
Action::SendMessage(ref history) => {
    let system = self.system_prompt.clone();
    let history = history.clone();
    let tx = self.action_tx.clone();
    let model_config = self.model_config.clone();   // ← 克隆一份 move 进 async
    tokio::spawn(async move {
        if let Err(e) = llm::stream_chat(&model_config, &system, &history, tx).await {
            tracing::error!("LLM error: {}", e);
        }
    });
}
```

> 为什么 `clone()`？因为 `model_config` 很小（几个 String + usize），克隆成本极低。如果介意，可以用 `Arc<ModelConfig>`。

---

## Step 4（可选）：接入 `Config` 让配置可热加载

如果你希望模型信息从 `config.json5` 里读，而不是写死在 `Default` 里：

### Step 4a：在 `Config` 结构体里加字段

文件：`src/config.rs`

```rust
use crate::model_config::ModelConfig;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default, flatten)]
    pub config: AppConfig,
    #[serde(default)]
    pub keybindings: KeyBindings,
    #[serde(default)]
    pub styles: Styles,
    #[serde(default)]
    pub model: ModelConfig,   // ← 新增
}
```

### Step 4b：在 `App::new()` 里用配置值覆盖默认值

```rust
let config = Config::new()?;
let model_config = config.model;   // 如果配置里没有，走 ModelConfig::default()
```

### Step 4c：在 `.config/config.json5` 里加示例

```json5
{
  model: {
    name: "gemma-4-31b",
    context_window: 65536,
    spec: {
      weights_file: "gemma-4-31B-it-Q4_K_M.gguf",
      quantization: "Q4_K_M",
      kv_cache_type: "q8_0",
      native_max_context: 262144,
      gpu_layers: 99
    },
    stfuu: false
  }
}
```

> 因为 `model_config.rs` derive 了 `Deserialize`，`config` crate 会自动把 JSON5/JSON/TOML 映射到结构体。字段名对上即可。

---

## 验证

### 编译通过

```bash
cargo check
```

### 运行测试（如果有的话）

```bash
cargo test
```

### 手动验证模型名确实走了配置

在 `llm.rs` 的 `stream_chat` 里临时加一行日志：

```rust
tracing::info!("Using model: {}", model_config.api_model_name());
```

启动 TUI，发一条消息，看日志里输出的是不是 `"gemma-4-31b"`。然后改 `.config/config.json5` 里的 `model.name` 为 `"fake-model-test"`，重启 TUI，确认日志跟着变。

### 验证 `stfuu` 占坑成功

在 `model_config.rs` 里给 `stfuu` 写个极小单元测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_stfuu_is_false() {
        let cfg = ModelConfig::default();
        assert!(!cfg.stfuu);
    }

    #[test]
    fn api_model_name_matches() {
        let cfg = ModelConfig::default();
        assert_eq!(cfg.api_model_name(), "gemma-4-31b");
    }
}
```

---

## 下一步可做的事（不现在做，先记着）

| 想法 | 时机 |
|------|------|
| 在状态栏显示 `[gemma-4-31b | 64k | Q4_K_M]` | 等你有状态栏组件 |
| 用 `context_window` 做对话历史截断（超过 60k 就弹警告） | 做 memory manager 时 |
| `stfuu: true` 时往 system prompt 追加 "Be extremely concise." | 做 prompt 开关时 |
| 从 `/v1/models` 动态拉取模型列表，取代本地配置 | 做多模型切换时 |

---

> **原则**：`ModelConfig` 是「事实来源」（source of truth）。
> 
> 凡是跟「当前跑的模型是什么、能处理多长、什么规格」有关的问题，都问 `ModelConfig`，不要散落在各处的字符串字面量里。
