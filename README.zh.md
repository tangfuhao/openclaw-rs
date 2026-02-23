# OpenClaw — Rust 版

> **[English](./README.md)** | 中文

高性能多通道 AI 网关，用 Rust 从零重写。将 Telegram、Discord、Slack、WhatsApp 等聊天平台接入任意 LLM（GPT-4、Claude、Gemini、Ollama），内置 Agent 循环、工具执行、Memory/RAG，最终产物是一个 **5 MB 的单二进制文件**。

---

## 为什么选择 Rust？

| 指标 | Node.js 原版 | Rust 版 | 提升 |
|------|-------------|---------|------|
| 冷启动时间 | ~800 ms | ~10 ms | **80×** |
| 空闲内存 | ~150 MB | ~8 MB | **15×** |
| WebSocket 并发 | ~1 万 | ~10 万+ | **10×** |
| 部署体积 | ~300 MB | ~5 MB | **60×** |

无 GC 停顿。无 `node_modules`。编译期内存安全。

---

## 核心功能

| 模块 | 功能 |
|------|------|
| **Gateway** | HTTP + WebSocket 服务器，兼容 OpenAI `/v1/chat/completions`，Webhook 路由 |
| **Agent** | 多轮 LLM 循环（最多 25 轮），工具执行，会话历史，Skills 系统 |
| **Auto-Reply** | 消息流水线：指令解析（`/model:`、`/think:`），防抖队列，流式分发 |
| **Memory / RAG** | SQLite + `sqlite-vec` 向量搜索 + FTS5 全文检索，RRF 融合算法 |
| **多通道** | Telegram、Discord、Slack、WhatsApp、Signal、Matrix、IRC、LINE、飞书… |
| **配置** | JSON5 格式（支持注释），`${ENV_VAR:-默认值}` 环境变量替换，热重载 |
| **CLI + TUI** | 40+ 子命令，交互式终端 UI（`ratatui`）|
| **插件 SDK** | `ChannelPlugin` trait，添加任意平台无需修改核心代码 |

---

## 快速开始

```bash
# 1. 构建 Release 版本（约 5 MB）
cargo build --release

# 2. 复制示例配置
mkdir -p ~/.openclaw
cp config/config.example.json5 ~/.openclaw/config.json5
# 编辑 config.json5，填入你的 API Key

# 3. 启动 Gateway
./target/release/openclaw gateway

# 其他常用命令
./target/release/openclaw status      # 查看系统状态
./target/release/openclaw tui         # 交互式 TUI
./target/release/openclaw doctor      # 诊断检查
```

### Docker

```bash
docker compose up -d
# 或手动构建
docker build -t openclaw-rs .
docker run -p 18789:18789 -e OPENAI_API_KEY=sk-... openclaw-rs
```

---

## 架构总览

```
openclaw-cli  （程序入口）
    ├── openclaw-gateway    HTTP / WebSocket 服务器（axum）
    │       └── openclaw-agent      LLM 执行循环 + 工具
    │               └── openclaw-reply      消息流水线
    │                       └── openclaw-memory     RAG / 搜索
    │                               └── openclaw-config     JSON5 加载器
    │                                       └── openclaw-core   核心类型 / Trait
    ├── openclaw-channels   Telegram、Discord、Slack…
    │       └── openclaw-plugin-sdk  ChannelPlugin Trait
    └── openclaw-infra      进程管理、端口检测
```

### 一条消息的完整旅程

```
用户在 Telegram 发消息
    → Channel Plugin（平台格式 → InboundMessage）
    → ReplyPipeline（队列防抖 → 指令解析 → 模型选择）
    → AgentRunner（加载历史 → 调用 LLM → 执行工具 → 循环）
    → StreamDelta（流式输出，打字机效果）
    → OutboundReply → Channel Plugin → 用户收到回复
```

### API 端点

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/health` | 健康检查（无需认证）|
| GET | `/ready` | 就绪探针 |
| POST | `/v1/chat/completions` | OpenAI 兼容聊天接口 |
| GET | `/v1/models` | 列出可用模型 |
| POST | `/hooks/:name` | 平台 Webhook 接收 |
| GET | `/api/channels` | 列出活跃通道 |
| WS | `/ws` | WebSocket（JSON-RPC）|

---

## 配置示例

`~/.openclaw/config.json5` 支持注释和环境变量插值：

```json5
{
  gateway: { port: 18789 },
  models: {
    default: "anthropic/claude-3-5-sonnet",
    aliases: {
      fast: "openai/gpt-4o-mini",    // 快速模型别名
      smart: "anthropic/claude-opus-4",
    },
  },
  agents: {
    default: {
      systemPrompt: "你是一个智能助手。",
      tools: ["web_search", "web_fetch", "memory"],
    }
  },
  channels: {
    telegram_main: {
      type: "telegram",
      token: "${TELEGRAM_BOT_TOKEN}",  // 从环境变量读取
    },
  },
}
```

**热重载**：修改配置文件后自动生效，无需重启。

---

## 开发

```bash
cargo build                          # Debug 构建
cargo test                           # 运行全部测试（15 个）
RUST_LOG=debug cargo run -- gateway  # 详细日志
cargo fmt && cargo clippy            # 格式化 + Lint
```

---

## 学习文档

如果你是第一次接触 OpenClaw 或 AI Agent 系统，推荐按顺序阅读：

| 文档 | 内容 | 建议时间 |
|------|------|---------|
| [01 · 项目全景图与架构总览](./docs/01-PROJECT-OVERVIEW.md) | 整体认知、10 个 Crate 分工、核心词汇表 | 10 min |
| [02 · Agent 系统核心流程](./docs/02-AGENT-SYSTEM.md) | LLM 循环、工具注册表、会话存储、流式输出 | 30 min |
| [03 · Gateway 与 WebSocket 协议](./docs/03-GATEWAY-PROTOCOL.md) | 协议格式、连接生命周期、认证机制 | 20 min |
| [04 · Auto-Reply 消息流水线](./docs/04-AUTO-REPLY-PIPELINE.md) | 指令解析、防抖队列、Producer-Consumer 模式 | 20 min |
| [05 · 从零开始学习路径](./docs/05-LEARNING-GUIDE.md) | 5 阶段路线、动手练习、里程碑检查清单 | 持续参考 |
| [技术规格文档](./docs/REWRITE_TECHNICAL_SPEC.md) | 完整重写规格（含原版深度分析）| 深度阅读 |

### 最核心的一条调用链

```
InboundMessage
  → ReplyPipeline::process()    [openclaw-reply]
      → parse_directives()       指令解析
      → AgentRunner::run()       [openclaw-agent]
          → call_llm()           调用 LLM API（流式）
          → tool_registry.execute()  执行工具
          → session_store.append_turn()  保存历史
      → OutboundReply
```

---

## License

MIT
