# Tutorial 00：本地 LLM 环境准备

> **目标**：安装并运行 llama.cpp server，让它暴露 OpenAI-compatible API，供 `llm-tui` 连接。  
> **前置要求**：一台能跑 LLM 的电脑（GPU 推荐，CPU 也行）。

---

## 什么是 llama.cpp？

[llama.cpp](https://github.com/ggerganov/llama.cpp) 是一个用 C/C++ 写的高性能 LLM 推理引擎。它可以把大模型转换成 **GGUF** 格式，在消费级硬件上本地运行，不需要 CUDA 也能用 Vulkan/Metal/CPU 后端。

**GGUF** 是 llama.cpp 的模型格式，类似这样：

```
llama-3-8b.Q4_K_M.gguf
│           │ │
│           │ └─ 量化方法（Q4_K_M = 4-bit，推荐的质量/体积平衡）
│           └── 参数量（8B = 80 亿参数）
└────────── 模型名称
```

量化数字越小，模型体积越小、速度越快，但质量越低：

| 量化 | 体积 (7B 模型) | 质量 | 用途 |
|------|---------------|------|------|
| Q8_0 | ~7GB | 接近原模型 | 追求质量 |
| Q4_K_M | ~4.5GB | 很好 | **日常使用推荐** |
| Q3_K_S | ~3GB | 可接受 | 显存紧张 |

---

## 安装 llama.cpp

### 从源码编译（推荐，获得最新功能）

```bash
git clone https://github.com/ggerganov/llama.cpp.git
cd llama.cpp

# Vulkan 后端（AMD/Intel/NVIDIA 通用，推荐）
cmake -B build -DGGML_VULKAN=ON
cmake --build build --config Release -j$(nproc)

# 或者 CUDA 后端（仅 NVIDIA，速度最快）
# cmake -B build -DGGML_CUDA=ON
# cmake --build build --config Release -j$(nproc)

# 创建软链接方便调用
ln -s $(pwd)/build/bin/llama-server ~/.local/bin/llama-server
ln -s $(pwd)/build/bin/llama-cli ~/.local/bin/llama-cli
```

### 或者下载预编译二进制

llama.cpp 的 [Release 页面](https://github.com/ggerganov/llama.cpp/releases) 提供各平台预编译版本。

---

## 下载模型

模型会自动下载到 `~/.cache/llama.cpp/`。第一次下载后，后续复用无需再下。

### 方式 1：通过 HuggingFace 直接拉取（推荐）

```bash
# 运行模型（会自动下载到缓存）
llama-cli -hf unsloth/Qwen2.5-7B-Instruct-GGUF:Q4_K_M -ngl 99 -p "Hello!"
```

格式说明：
- `unsloth/Qwen2.5-7B-Instruct-GGUF` — HuggingFace 仓库名
- `Q4_K_M` — 量化版本
- `-ngl 99` — 把 99 层都放到 GPU 上（数字越大，GPU 负担越重）

### 方式 2：手动下载 GGUF

从 [HuggingFace](https://huggingface.co/models?search=gguf) 或 [TheBloke](https://huggingface.co/TheBloke) 下载 `.gguf` 文件，然后直接指定路径运行。

---

## 启动 OpenAI-compatible Server

这是 `llm-tui` 需要的运行模式：

```bash
llama-server -hf unsloth/Qwen2.5-7B-Instruct-GGUF:Q4_K_M -ngl 99 \
    --host 127.0.0.1 --port 8080
```

参数说明：

| 参数 | 含义 |
|------|------|
| `-hf ...` | HuggingFace 模型地址（或直接用 `-m path/to/model.gguf`） |
| `-ngl 99` | GPU  offload 层数。`99` = 全放 GPU；`0` = 纯 CPU |
| `--host 127.0.0.1` | 只接受本机连接 |
| `--port 8080` | 服务端口 |
| `-c 32768` | 上下文长度（默认 4096，可按需加大） |

启动后，API 端点在这里：

```
POST http://127.0.0.1:8080/v1/chat/completions
```

测试一下：

```bash
curl http://127.0.0.1:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "qwen2.5-7b",
    "messages": [{"role": "user", "content": "Hello!"}],
    "stream": true
  }'
```

看到流式输出就是成功了。

---

## 推荐参数调整

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `--temp 0.8` | 0.8 | 温度。`0.0` = 确定性输出；`1.5` = 很有创意/可能胡言 |
| `--top-p 0.95` | 0.95 | Nucleus sampling。`0.1` = 保守；`0.95` = 多样 |
| `-c 4096` | 4096 | 上下文窗口。越大占显存越多 |
| `-ngl 99` | 0 | GPU 层数。能开多少看显存 |

**显存占用估算（Q4_K_M）：**

| 模型 | 显存 | 5090 速度 |
|------|------|----------|
| 7B | ~4.5GB | ~120 tok/s |
| 13B | ~8GB | ~70 tok/s |
| 32B | ~20GB | ~25 tok/s |

---

## 下一步

Server 跑起来后，打开另一个终端：

```bash
cd /path/to/llm-tui
cargo run
```

开始聊天。

---

*下一步：[Tutorial 01](01-chat-component.md) — 构建第一个 Chat 组件。*
