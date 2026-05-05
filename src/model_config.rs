use serde::{Deserialize, Serialize};

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
    pub fn api_model_name(&self) -> &str {
        &self.name
    }

    #[allow(dead_code)]
    pub fn remaining_tokens(&self, used: usize) -> usize {
        self.context_window.saturating_sub(used)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_stfuu_is_false() {
        let cfg = ModelConfig::default();
        assert!(!cfg.stfuu)
    }

    #[test]
    fn api_model_name_matches() {
        let cfg = ModelConfig::default();
        assert_eq!(cfg.api_model_name(), "gemma-4-31b")
    }
}
