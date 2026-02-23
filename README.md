# OpenClaw — Rust Edition

> **[中文文档](./README.zh.md)** | English

A high-performance, multi-channel AI gateway rewritten in Rust. Connect Telegram, Discord, Slack, WhatsApp, and more to any LLM (GPT-4, Claude, Gemini, Ollama) — with an agent loop, tool execution, memory/RAG, and a 5 MB single binary.

---

## Why Rust?

| Metric | Node.js (original) | Rust | Gain |
|--------|-------------------|------|------|
| Cold start | ~800 ms | ~10 ms | **80×** |
| Idle memory | ~150 MB | ~8 MB | **15×** |
| WebSocket concurrency | ~10 K | ~100 K+ | **10×** |
| Deploy size | ~300 MB | ~5 MB | **60×** |

Zero GC pauses. No `node_modules`. Compile-time memory safety.

---

## Features

| Module | What it does |
|--------|-------------|
| **Gateway** | HTTP + WebSocket server, OpenAI-compatible `/v1/chat/completions`, Webhook routing |
| **Agent** | Multi-turn LLM loop (max 25 iterations), tool execution, session history, skills |
| **Auto-Reply** | Inbound pipeline: directive parsing (`/model:`, `/think:`), debounce queue, streaming dispatch |
| **Memory / RAG** | SQLite + `sqlite-vec` vector search + FTS5 full-text, Reciprocal Rank Fusion |
| **Channels** | Telegram, Discord, Slack, WhatsApp, Signal, Matrix, IRC, LINE, Lark … |
| **Config** | JSON5 with comments, `${ENV_VAR:-default}` substitution, hot-reload |
| **CLI + TUI** | 40+ sub-commands, interactive terminal UI (`ratatui`) |
| **Plugin SDK** | `ChannelPlugin` trait — add any platform without touching core |

---

## Quick Start

```bash
# 1. Build release binary (~5 MB)
cargo build --release

# 2. Copy example config
mkdir -p ~/.openclaw
cp config/config.example.json5 ~/.openclaw/config.json5
# Edit config.json5 and fill in your API keys

# 3. Start the gateway
./target/release/openclaw gateway

# Other useful commands
./target/release/openclaw status      # system status
./target/release/openclaw tui         # interactive TUI
./target/release/openclaw doctor      # diagnostics
```

### Docker

```bash
docker compose up -d
# or
docker build -t openclaw-rs .
docker run -p 18789:18789 -e OPENAI_API_KEY=sk-... openclaw-rs
```

---

## Architecture

```
openclaw-cli  (binary entry)
    ├── openclaw-gateway    HTTP / WebSocket server (axum)
    │       └── openclaw-agent      LLM loop + tool execution
    │               └── openclaw-reply      message pipeline
    │                       └── openclaw-memory     RAG / search
    │                               └── openclaw-config     JSON5 loader
    │                                       └── openclaw-core   types / traits
    ├── openclaw-channels   Telegram, Discord, Slack, …
    │       └── openclaw-plugin-sdk  ChannelPlugin trait
    └── openclaw-infra      process mgmt, port utils
```

### API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check (no auth) |
| GET | `/ready` | Readiness probe |
| POST | `/v1/chat/completions` | OpenAI-compatible chat |
| GET | `/v1/models` | List available models |
| POST | `/hooks/:name` | Platform webhook receiver |
| GET | `/api/channels` | List active channels |
| WS | `/ws` | WebSocket (JSON-RPC) |

---

## Configuration

`~/.openclaw/config.json5` supports comments and env-var interpolation:

```json5
{
  gateway: { port: 18789 },
  models: {
    default: "anthropic/claude-3-5-sonnet",
    aliases: { fast: "openai/gpt-4o-mini" },
  },
  agents: {
    default: {
      systemPrompt: "You are a helpful assistant.",
      tools: ["web_search", "web_fetch"],
    }
  },
  channels: {
    telegram_main: { type: "telegram", token: "${TELEGRAM_BOT_TOKEN}" },
  },
}
```

---

## Development

```bash
cargo build                          # debug build
cargo test                           # run all 15 tests
RUST_LOG=debug cargo run -- gateway  # verbose logging
cargo fmt && cargo clippy            # format + lint
```

---

## Documentation

| Doc | Description |
|-----|-------------|
| [Project Overview](./docs/01-PROJECT-OVERVIEW.md) | Architecture, crate map, data flow |
| [Agent System](./docs/02-AGENT-SYSTEM.md) | LLM loop, tool registry, session store |
| [Gateway & WebSocket](./docs/03-GATEWAY-PROTOCOL.md) | Protocol spec, connection lifecycle |
| [Auto-Reply Pipeline](./docs/04-AUTO-REPLY-PIPELINE.md) | Directive parsing, queue, streaming |
| [Learning Guide](./docs/05-LEARNING-GUIDE.md) | Step-by-step guide for newcomers |
| [Technical Spec](./docs/REWRITE_TECHNICAL_SPEC.md) | Full rewrite spec (Chinese) |

---

## License

MIT
