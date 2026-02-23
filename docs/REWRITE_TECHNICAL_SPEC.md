# OpenClaw Rust 重写技术规格文档

**版本**: 2.0  
**创建日期**: 2026-02-20  
**项目路径**: `/Users/fuhao/Documents/workspace/openclaw_lab/openclaw-rs`

---

## 目录

1. [项目概述](#1-项目概述)
2. [原项目深度分析](#2-原项目深度分析)
3. [Pi SDK 集成架构](#3-pi-sdk-集成架构)
4. [技术栈映射](#4-技术栈映射)
5. [Rust 重写架构设计](#5-rust-重写架构设计)
6. [各子系统详细设计](#6-各子系统详细设计)
7. [关键实现细节](#7-关键实现细节)
8. [性能对比](#8-性能对比)
9. [测试策略](#9-测试策略)
10. [部署指南](#10-部署指南)
11. [从 TypeScript 到 Rust 的转换指南](#11-从-typescript-到-rust-的转换指南)
12. [后续优化方向](#12-后续优化方向)
13. [附录](#13-附录)

---

## 1. 项目概述

### 1.1 什么是 OpenClaw

OpenClaw 是一个**多通道 AI 网关**，核心功能包括：

- **Gateway 服务器**：原生 Node.js HTTP + `ws` WebSocket 服务器，无框架，提供控制平面和事件推送
- **Agent 系统**：基于 `@mariozechner/pi-agent-core` 的 AI 代理执行引擎，支持工具、技能、子代理
- **Auto-Reply 引擎**：消息处理流水线（指令解析、媒体处理、队列管理、流式分发）
- **Memory/RAG 系统**：SQLite + `sqlite-vec` 向量搜索 + FTS5 全文搜索的混合检索
- **Config 系统**：Zod schema 验证的 JSON5 配置，支持环境变量替换和热重载
- **Plugin/Extension 系统**：31+ 消息通道插件（Telegram、Discord、Slack、WhatsApp、Signal、Matrix、iMessage 等）
- **CLI**：Commander.js 实现的 40+ 子命令
- **TUI**：基于 `@mariozechner/pi-tui` 的交互式终端界面

### 1.2 原项目规模

| 指标 | 数值 |
|------|------|
| 语言 | TypeScript (ESM) |
| 运行时 | Node.js >= 22.12 |
| 包管理器 | pnpm 10.23.0 |
| 构建工具 | tsdown, rolldown |
| 源文件数量 | ~2000+ 文件 |
| 依赖体积 | ~300MB+ (node_modules) |
| 版本 | 2026.2.15 |

### 1.3 重写目标

将 OpenClaw 完全重写为 Rust，实现：

| 目标 | 预期效果 |
|------|----------|
| 冷启动时间 | 从 ~800ms 降至 ~10ms (80x 提升) |
| 内存占用 | 从 ~150MB 降至 ~10MB (15x 降低) |
| WebSocket 并发 | 从 ~10K 提升至 ~100K+ (10x+) |
| 部署体积 | 从 ~300MB 降至 ~5MB (60x 缩减) |
| 类型安全 | 编译期保证，无运行时类型错误 |
| 内存安全 | 所有权系统保证，无 GC 停顿 |

---

## 2. 原项目深度分析

### 2.1 核心子系统目录结构

```
openclaw/src/
├── gateway/                    # Gateway HTTP/WS 服务器 (~222 文件)
│   ├── server-methods/        # RPC 方法实现
│   ├── protocol.ts            # WebSocket 协议定义
│   └── ...
├── agents/                     # AI Agent 执行系统 (~539 文件)
│   ├── pi-embedded-runner/    # Pi SDK 嵌入式运行器
│   ├── pi-embedded-subscribe/ # 事件订阅
│   ├── pi-tools/              # 工具定义
│   ├── tools/                 # 具体工具实现
│   ├── skills/                # 技能系统
│   └── ...
├── auto-reply/                 # 消息处理流水线 (~225 文件)
│   ├── pipeline.ts            # 处理管道
│   ├── dispatcher.ts          # 回复分发
│   └── ...
├── memory/                     # Memory/RAG 系统 (~63 文件)
│   ├── search/                # 搜索实现
│   ├── embeddings/            # 嵌入服务
│   └── ...
├── config/                     # 配置系统 (~159 文件)
│   ├── schema.ts              # Zod schema
│   ├── loader.ts              # 加载器
│   └── ...
├── cli/                        # CLI 命令 (~198 文件)
├── channels/                   # 通道抽象 (~29 文件)
├── plugins/                    # 插件系统
├── hooks/                      # Hook 系统 (~25 文件)
├── infra/                      # 基础设施 (~176 文件)
├── process/                    # 进程管理 (~11 文件)
└── tui/                        # 终端 UI (~25 文件)
```

### 2.2 核心依赖分析

#### Pi SDK 套件 (核心 AI 能力)

```json
{
  "@mariozechner/pi-agent-core": "0.52.12",  // Agent 循环、工具执行、消息类型
  "@mariozechner/pi-ai": "0.52.12",          // LLM 抽象、流式传输、提供商 API
  "@mariozechner/pi-coding-agent": "0.52.12", // 高级 SDK、会话管理、内置工具
  "@mariozechner/pi-tui": "0.52.12"          // 终端 UI 组件
}
```

| 包 | 用途 |
|---|------|
| `pi-ai` | 核心 LLM 抽象：`Model`、`streamSimple`、消息类型、提供商 API |
| `pi-agent-core` | Agent 循环、工具执行、`AgentMessage` 类型 |
| `pi-coding-agent` | 高级 SDK：`createAgentSession`、`SessionManager`、`AuthStorage`、`ModelRegistry`、内置工具 |
| `pi-tui` | 终端 UI 组件（用于 OpenClaw 的本地 TUI 模式）|

#### 消息平台依赖

```json
{
  "grammy": "^1.40.0",                    // Telegram Bot API
  "@buape/carbon": "0.14.0",              // Discord
  "@slack/bolt": "^4.6.0",                // Slack
  "@whiskeysockets/baileys": "7.0.0-rc.9", // WhatsApp (unofficial)
  "@line/bot-sdk": "^10.6.0",             // LINE
  "@larksuiteoapi/node-sdk": "^1.59.0"    // 飞书
}
```

#### 其他关键依赖

```json
{
  "better-sqlite3": "...",       // SQLite (通过 N-API)
  "sqlite-vec": "0.1.7-alpha.2", // 向量搜索扩展
  "zod": "^4.3.6",               // Schema 验证
  "commander": "^14.0.3",        // CLI
  "chokidar": "^5.0.0",          // 文件监控
  "croner": "^10.0.1",           // Cron 调度
  "playwright-core": "1.58.2",   // 浏览器自动化
  "sharp": "^0.34.5"             // 图像处理
}
```

### 2.3 Gateway 架构详解

```
┌─────────────────────────────────────────────────────────────────┐
│                        Gateway Server                            │
│                    (127.0.0.1:18789 默认)                        │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │   HTTP 层    │  │ WebSocket 层 │  │  Canvas 主机 │              │
│  │             │  │             │  │  (端口 18793) │              │
│  │ /v1/chat/*  │  │  JSON-RPC   │  │             │              │
│  │ /hooks/*    │  │  协议       │  │ A2UI 服务    │              │
│  │ /tools/*    │  │             │  │             │              │
│  └─────────────┘  └─────────────┘  └─────────────┘              │
├─────────────────────────────────────────────────────────────────┤
│                        核心服务                                   │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │ 认证服务     │  │ 配置热重载  │  │ 事件广播    │              │
│  │ (Token/密码) │  │ (chokidar) │  │ (pub/sub)   │              │
│  └─────────────┘  └─────────────┘  └─────────────┘              │
├─────────────────────────────────────────────────────────────────┤
│                      消息通道层                                   │
│  ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐              │
│  │ WA │ │ TG │ │ DC │ │ SL │ │ SG │ │ MX │ │ ... │              │
│  └────┘ └────┘ └────┘ └────┘ └────┘ └────┘ └────┘              │
└─────────────────────────────────────────────────────────────────┘

WA = WhatsApp, TG = Telegram, DC = Discord, SL = Slack
SG = Signal, MX = Matrix
```

#### WebSocket 协议

```typescript
// 请求-响应模式
{ type: "req", id: string, method: string, params: object }
{ type: "res", id: string, ok: boolean, payload?: object, error?: object }

// 事件推送模式
{ type: "event", event: string, payload: object, seq?: number, stateVersion?: number }
```

#### 主要 RPC 方法

| 方法 | 功能 |
|------|------|
| `connect` | 握手认证（必须是第一帧）|
| `health` | 健康检查快照 |
| `status` | 简短状态摘要 |
| `agent` | 运行 AI Agent 轮次（流式返回）|
| `send` | 通过通道发送消息 |
| `system-presence` | Presence 列表 |
| `node.*` | 节点管理（配对、调用）|

### 2.4 消息处理数据流

```
┌──────────────────────────────────────────────────────────────────────┐
│                          入站消息流                                   │
└──────────────────────────────────────────────────────────────────────┘

    Telegram/Discord/Slack/WhatsApp/Signal/...
                        │
                        ▼
    ┌───────────────────────────────────────┐
    │         Channel Plugin                 │
    │  - 协议适配                            │
    │  - 消息标准化 → InboundMessage         │
    │  - Webhook 处理                        │
    └───────────────────────────────────────┘
                        │
                        ▼
    ┌───────────────────────────────────────┐
    │         Auto-Reply Engine              │
    │  - 指令解析 (/model:, /think:, ...)   │
    │  - 媒体处理 (图片描述, 文档提取)        │
    │  - 消息队列 (防抖, 去重, 限流)          │
    │  - Agent 调用决策                       │
    └───────────────────────────────────────┘
                        │
                        ▼
    ┌───────────────────────────────────────┐
    │         Agent Runner                   │
    │  - 会话加载/创建                       │
    │  - 系统提示词构建                      │
    │  - Pi SDK 调用                         │
    │  - 流式响应处理                        │
    └───────────────────────────────────────┘
           │                    │
           ▼                    ▼
    ┌─────────────┐     ┌─────────────┐
    │ Tool Exec   │     │ LLM API     │
    │ - web_search│     │ - Anthropic │
    │ - web_fetch │     │ - OpenAI    │
    │ - memory    │     │ - Google    │
    │ - browser   │     │ - OpenRouter│
    │ - cron      │     │ - Ollama    │
    │ - image_gen │     │             │
    └─────────────┘     └─────────────┘
           │                    │
           └────────┬───────────┘
                    ▼
    ┌───────────────────────────────────────┐
    │         Reply Dispatcher               │
    │  - 流式分块 (soft chunks)              │
    │  - 思考标签剥离 (<think>...</think>)   │
    │  - 媒体指令提取 ([[media:url]])        │
    │  - 工具摘要注入                        │
    └───────────────────────────────────────┘
                        │
                        ▼
    ┌───────────────────────────────────────┐
    │         Channel Plugin                 │
    │  - 消息格式化                          │
    │  - 平台 API 调用                       │
    │  - 发送确认                            │
    └───────────────────────────────────────┘
                        │
                        ▼
              用户收到回复
```

---

## 3. Pi SDK 集成架构

### 3.1 集成方式

OpenClaw 使用 **嵌入式集成** 方式，不是将 Pi 作为子进程或 RPC 服务，而是直接导入并实例化 `AgentSession`：

```typescript
import {
  createAgentSession,
  DefaultResourceLoader,
  SessionManager,
  SettingsManager,
} from "@mariozechner/pi-coding-agent";

const { session } = await createAgentSession({
  cwd: resolvedWorkspace,
  agentDir,
  authStorage: params.authStorage,
  modelRegistry: params.modelRegistry,
  model: params.model,
  thinkingLevel: mapThinkingLevel(params.thinkLevel),
  tools: builtInTools,
  customTools: allCustomTools,
  sessionManager,
  settingsManager,
  resourceLoader,
});
```

### 3.2 这种集成提供的能力

- 对会话生命周期和事件处理的**完全控制**
- **自定义工具注入**（消息、沙箱、通道特定操作）
- 每个通道/上下文的**系统提示自定义**
- 支持分支/压缩的**会话持久化**
- 带故障转移的**多账户认证配置文件轮换**
- 与提供商无关的**模型切换**

### 3.3 事件订阅桥接

`subscribeEmbeddedPiSession()` 将 Pi 的 `AgentSession` 事件桥接到 OpenClaw：

```typescript
// Pi 事件 → OpenClaw 流
message_start/update/end  →  stream: "assistant"
tool_execution_*          →  stream: "tool"
turn_start/end            →  stream: "lifecycle"
agent_start/end           →  stream: "lifecycle"
auto_compaction_*         →  stream: "compaction"
```

### 3.4 工具架构

工具管道处理流程：

```
1. 基础工具：Pi 的 codingTools（read、bash、edit、write）
       ↓
2. 自定义替换：OpenClaw 将 bash 替换为 exec/process，沙箱化 read/edit/write
       ↓
3. OpenClaw 工具：消息、浏览器、画布、会话、定时任务、Gateway 等
       ↓
4. 通道工具：Discord/Telegram/Slack/WhatsApp 特定的操作工具
       ↓
5. 策略过滤：按配置文件、提供商、Agent、群组、沙箱策略过滤
       ↓
6. Schema 规范化：为 Gemini/OpenAI 的特殊情况清理 Schema
       ↓
7. AbortSignal 包装：工具被包装以尊重中止信号
```

### 3.5 Rust 重写中的 Pi 替代方案

由于 Pi SDK 是 TypeScript 生态的产物，Rust 版本需要**完全重新实现** Agent 核心：

| Pi SDK 组件 | Rust 重写方案 |
|-------------|---------------|
| `AgentSession` | `AgentRunner` struct + async methods |
| `SessionManager` | `SessionStore` (文件 + 内存缓存) |
| `AgentMessage` | `ConversationTurn` enum |
| `AgentTool` | `AgentTool` trait + `ToolRegistry` |
| `AuthStorage` | 配置驱动的 API key 管理 |
| `ModelRegistry` | `ModelId` + provider 路由 |
| Event subscription | `tokio::sync::mpsc` channels |

---

## 4. 技术栈映射

### 4.1 完整技术栈对照表

| 模块 | TypeScript 实现 | Rust 替代方案 | 选择理由 |
|------|-----------------|---------------|----------|
| **异步运行时** | Node.js 事件循环 | `tokio` | Rust 生态标准，高性能多线程异步 |
| **HTTP 服务器** | 原生 `node:http` | `axum` | 基于 tower/hyper，类型安全路由，性能极佳 |
| **WebSocket** | `ws` 库 | `axum` 内置 + `tokio-tungstenite` | 与 axum 无缝集成 |
| **JSON 解析** | V8 内置 | `serde_json` | 零拷贝反序列化，编译期生成 |
| **配置验证** | `zod` schema | `serde` + `validator` | 编译期类型安全 + 运行时验证 |
| **配置格式** | `json5` | `json5` crate | 相同格式，无迁移成本 |
| **CLI** | `commander.js` | `clap` (derive) | Rust CLI 标准，编译期参数解析 |
| **SQLite** | `better-sqlite3` (N-API) | `rusqlite` (直接 FFI) | 零开销，直接 C 绑定 |
| **向量搜索** | `sqlite-vec` | `rusqlite` + `sqlite-vec` | 相同扩展 |
| **全文搜索** | SQLite FTS5 | SQLite FTS5 | 原生支持 |
| **HTTP 客户端** | `undici` | `reqwest` | 基于 hyper，异步 + 连接池 |
| **日志/追踪** | `tslog` | `tracing` + `tracing-subscriber` | 结构化日志，异步友好 |
| **文件监控** | `chokidar` | `notify` | 跨平台文件系统事件 |
| **Cron 调度** | `croner` | `tokio-cron-scheduler` | 异步 cron |
| **模板引擎** | 内置字符串 | `minijinja` | 轻量级模板 |
| **TUI** | `@mariozechner/pi-tui` | `ratatui` + `crossterm` | Rust TUI 标准 |
| **错误处理** | try-catch + Error | `thiserror` + `anyhow` | 类型化错误 + 便捷传播 |
| **正则表达式** | 内置 RegExp | `regex` | 高性能，与 RE2 兼容 |

### 4.2 消息平台 SDK 替代

| 平台 | TypeScript SDK | Rust 替代方案 |
|------|----------------|---------------|
| Telegram | `grammy` | 自实现 Bot API 客户端 (reqwest) |
| Discord | `@buape/carbon` | 自实现 Gateway + REST (reqwest + tokio-tungstenite) |
| Slack | `@slack/bolt` | 自实现 Events API + Web API (reqwest) |
| WhatsApp | `baileys` | 自实现协议客户端 (复杂度高，优先级低) |
| Signal | `signal-cli` | 同样调用 signal-cli |
| Matrix | `matrix-js-sdk` | `matrix-sdk` (官方 Rust SDK) |

---

## 5. Rust 重写架构设计

### 5.1 Crate 结构

```
openclaw-rs/
├── Cargo.toml                    # Workspace 根配置
├── crates/
│   ├── openclaw-core/            # 核心类型、trait、错误定义
│   ├── openclaw-config/          # 配置加载、验证、热重载
│   ├── openclaw-gateway/         # HTTP/WebSocket 服务器
│   ├── openclaw-agent/           # AI Agent 执行引擎
│   ├── openclaw-reply/           # 消息处理流水线
│   ├── openclaw-memory/          # Memory/RAG 系统
│   ├── openclaw-plugin-sdk/      # 插件 SDK
│   ├── openclaw-channels/        # 内置通道实现
│   ├── openclaw-cli/             # CLI 二进制 + TUI
│   └── openclaw-infra/           # 基础设施工具
├── config/                       # 示例配置
├── tests/                        # 集成测试
├── Dockerfile
└── docker-compose.yml
```

### 5.2 依赖关系图

```
                         ┌──────────────────┐
                         │   openclaw-cli   │ (binary)
                         │   main.rs        │
                         └────────┬─────────┘
                                  │
        ┌─────────────────────────┼─────────────────────────┐
        │                         │                         │
        ▼                         ▼                         ▼
┌───────────────┐         ┌───────────────┐         ┌───────────────┐
│  openclaw-    │         │  openclaw-    │         │  openclaw-    │
│  gateway      │         │  channels     │         │  infra        │
│               │         │               │         │               │
│ - HTTP routes │         │ - Telegram    │         │ - dotenv      │
│ - WebSocket   │         │ - Discord     │         │ - ports       │
│ - Auth        │         │ - Slack       │         │ - process     │
│ - State       │         │ - WhatsApp    │         │ - update      │
└───────┬───────┘         │ - Signal      │         └───────────────┘
        │                 │ - Matrix      │
        │                 │ - IRC         │
        ▼                 └───────┬───────┘
┌───────────────┐                 │
│  openclaw-    │                 │
│  agent        │                 │
│               │                 │
│ - Runner      │                 ▼
│ - Session     │         ┌───────────────┐
│ - Tools       │         │  openclaw-    │
│ - Skills      │         │  plugin-sdk   │
│ - Subagent    │         │               │
│ - Prompt      │         │ - Traits      │
└───────┬───────┘         │ - API         │
        │                 │ - Types       │
        ▼                 └───────┬───────┘
┌───────────────┐                 │
│  openclaw-    │                 │
│  reply        │                 │
│               │                 │
│ - Pipeline    │◀────────────────┘
│ - Dispatcher  │
│ - Queue       │
│ - Directive   │
│ - Media       │
└───────┬───────┘
        │
        ▼
┌───────────────┐
│  openclaw-    │
│  memory       │
│               │
│ - SQLite      │
│ - Embeddings  │
│ - Search      │
│ - Chunker     │
└───────┬───────┘
        │
        ▼
┌───────────────┐
│  openclaw-    │
│  config       │
│               │
│ - Schema      │
│ - Loader      │
│ - EnvSubst    │
│ - Migration   │
│ - Watcher     │
└───────┬───────┘
        │
        ▼
┌───────────────┐
│  openclaw-    │
│  core         │ (foundation)
│               │
│ - Types       │
│ - Error       │
│ - Message     │
│ - Channel     │
└───────────────┘
```

### 5.3 各 Crate 职责概述

| Crate | 职责 | 对应原版模块 |
|-------|------|-------------|
| `openclaw-core` | 统一错误、核心类型、消息模型、Channel trait | `src/channels/`, 类型定义 |
| `openclaw-config` | JSON5 解析、环境变量替换、Schema 验证、热重载 | `src/config/` |
| `openclaw-gateway` | HTTP 服务器、WebSocket、RPC 方法、认证 | `src/gateway/` |
| `openclaw-agent` | Agent 执行引擎、会话、工具、技能、子代理 | `src/agents/`, Pi SDK 替代 |
| `openclaw-reply` | 消息流水线、指令解析、回复分发、队列 | `src/auto-reply/` |
| `openclaw-memory` | SQLite、向量搜索、FTS5、嵌入服务 | `src/memory/` |
| `openclaw-plugin-sdk` | 插件 trait、生命周期钩子、API | `src/plugins/`, plugin-sdk |
| `openclaw-channels` | 内置通道实现 | `extensions/*` |
| `openclaw-cli` | CLI 命令、TUI | `src/cli/`, `src/tui/` |
| `openclaw-infra` | 进程管理、端口、dotenv、自更新 | `src/infra/`, `src/process/` |

---

## 6. 各子系统详细设计

### 6.1 openclaw-core

**核心类型定义**

```rust
// 标识符类型 (newtype pattern)
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentId(pub String);

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChannelId(pub String);

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionKey(pub String);

// RBAC 权限
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scope {
    Read,      // 读取配置/状态
    Write,     // 修改配置
    Agent,     // 运行 Agent
    Admin,     // 完全控制
}

// LLM 提供商
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmProvider {
    Anthropic,
    OpenAI,
    Google,
    OpenRouter,
    Ollama,
    Custom { base_url: String },
}
```

**统一错误类型**

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Config error: {0}")]
    Config(String),
    
    #[error("Auth failed: {0}")]
    Auth(String),
    
    #[error("Channel error: {channel}: {message}")]
    Channel { channel: String, message: String },
    
    #[error("Agent error: {0}")]
    Agent(String),
    
    #[error("Tool execution failed: {tool}: {message}")]
    Tool { tool: String, message: String },
    
    #[error("Memory error: {0}")]
    Memory(String),
    
    #[error("Rate limited, retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },
    
    // ... 更多错误类型
}
```

**消息模型**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    pub id: String,
    pub channel_id: ChannelId,
    pub session_key: SessionKey,
    pub sender_id: String,
    pub sender_name: Option<String>,
    pub content: String,
    pub attachments: Vec<MediaAttachment>,
    pub reply_to_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundReply {
    pub text: String,
    pub media_urls: Vec<String>,
    pub reply_to_id: Option<String>,
    pub voice: bool,
    pub metadata: HashMap<String, Value>,
}
```

**Channel Trait**

```rust
#[async_trait]
pub trait ChannelPlugin: Send + Sync {
    fn id(&self) -> &ChannelId;
    fn name(&self) -> &str;
    fn status(&self) -> ChannelStatus;
    
    async fn start(&mut self) -> Result<()>;
    async fn stop(&mut self) -> Result<()>;
    
    async fn send_message(
        &self,
        session_key: &SessionKey,
        reply: OutboundReply,
    ) -> Result<String>;
    
    async fn handle_webhook(
        &self,
        request: Request<Body>,
    ) -> Result<Response<Body>>;
}
```

### 6.2 openclaw-config

**配置 Schema**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct OpenClawConfig {
    #[serde(default)]
    pub gateway: GatewayConfig,
    
    #[serde(default)]
    pub models: ModelsConfig,
    
    #[serde(default)]
    pub agents: HashMap<String, AgentConfig>,
    
    #[serde(default)]
    pub memory: MemoryConfig,
    
    #[serde(default)]
    pub sessions: SessionsConfig,
    
    #[serde(default)]
    pub channels: HashMap<String, Value>,
    
    #[serde(default)]
    pub plugins: PluginsConfig,
    
    #[serde(default)]
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    #[serde(default = "default_gateway_port")]
    pub port: u16,  // 默认 18789
    
    #[serde(default)]
    pub host: String,  // 默认 "127.0.0.1"
    
    #[serde(default)]
    pub auth: GatewayAuthConfig,
    
    #[serde(default)]
    pub tls: Option<TlsConfig>,
    
    #[serde(default)]
    pub reload: ReloadConfig,
}
```

**环境变量替换**

```rust
/// 支持 ${VAR} 和 ${VAR:-default} 语法
pub fn substitute_env_vars(input: &str) -> String {
    let re = Regex::new(r"\$\{([^}]+)\}").unwrap();
    re.replace_all(input, |caps: &regex::Captures| {
        let expr = &caps[1];
        if let Some((var, default)) = expr.split_once(":-") {
            env::var(var).unwrap_or_else(|_| default.to_string())
        } else {
            env::var(expr).unwrap_or_default()
        }
    }).to_string()
}
```

**配置热重载**

```rust
pub struct ConfigWatcher {
    config_manager: Arc<ConfigManager>,
    watcher: RecommendedWatcher,
    debounce_rx: mpsc::Receiver<()>,
}

impl ConfigWatcher {
    pub async fn watch(&mut self) -> Result<()> {
        while let Some(()) = self.debounce_rx.recv().await {
            // 防抖 500ms
            tokio::time::sleep(Duration::from_millis(500)).await;
            
            if let Err(e) = self.config_manager.reload() {
                tracing::error!("Config reload failed: {}", e);
            } else {
                tracing::info!("Config reloaded successfully");
            }
        }
        Ok(())
    }
}
```

### 6.3 openclaw-gateway

**服务器架构**

```rust
pub async fn start_gateway_server(
    config: Arc<ConfigManager>,
    shutdown_signal: CancellationToken,
) -> Result<()> {
    let state = AppState::new(config.clone());
    
    let app = Router::new()
        // HTTP 路由
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/models", get(list_models))
        .route("/hooks/:name", post(handle_webhook))
        .route("/api/channels", get(list_channels))
        .route("/api/channels/:id/webhook", post(channel_webhook))
        // WebSocket 升级
        .route("/ws", get(ws_upgrade_handler))
        // 中间件
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .with_state(state);
    
    let addr = SocketAddr::from(([127, 0, 0, 1], config.gateway().port));
    let listener = TcpListener::bind(addr).await?;
    
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal.cancelled_owned())
        .await?;
    
    Ok(())
}
```

**WebSocket 协议**

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct WsMessage {
    #[serde(rename = "type")]
    pub msg_type: WsMessageType,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<WsError>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WsMessageType {
    Req,
    Res,
    Event,
}
```

### 6.4 openclaw-agent

**Agent 执行引擎**

```rust
pub struct AgentRunner {
    config: Arc<ConfigManager>,
    session_store: SessionStore,
    tool_registry: ToolRegistry,
    skill_manager: SkillManager,
}

impl AgentRunner {
    pub async fn run(
        &self,
        request: AgentRunRequest,
        delta_tx: mpsc::Sender<StreamDelta>,
    ) -> Result<AgentRunResult> {
        // 1. 构建系统提示词
        let system_prompt = build_system_prompt(
            &request.config,
            &request.agent_id,
            &self.skill_manager,
        );
        
        // 2. 加载会话历史
        let history = self.session_store
            .get_history(&request.session_key)
            .await?;
        
        // 3. 构建消息列表
        let mut messages = vec![
            LlmMessage { role: "system", content: system_prompt },
        ];
        messages.extend(history);
        messages.push(LlmMessage { 
            role: "user", 
            content: request.message 
        });
        
        // 4. Agent 循环
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 25;
        
        loop {
            if iterations >= MAX_ITERATIONS {
                return Err(Error::Agent("Max iterations reached".into()));
            }
            iterations += 1;
            
            // 调用 LLM
            let response = self.call_llm(
                &request.model,
                &messages,
                &delta_tx,
            ).await?;
            
            // 检查是否有工具调用
            if response.tool_calls.is_empty() {
                // 保存历史
                self.session_store.append_turn(
                    &request.session_key,
                    ConversationTurn::Assistant { 
                        content: response.text.clone() 
                    },
                ).await?;
                
                return Ok(AgentRunResult {
                    response_text: response.text,
                    tool_calls_count: iterations - 1,
                    tokens_used: response.usage,
                });
            }
            
            // 执行工具
            for tool_call in &response.tool_calls {
                let result = self.tool_registry
                    .execute(&tool_call.name, &tool_call.input)
                    .await;
                
                // 发送工具事件
                delta_tx.send(StreamDelta::ToolResult {
                    tool_call_id: tool_call.id.clone(),
                    result: result.clone(),
                }).await?;
                
                // 添加到消息
                messages.push(LlmMessage {
                    role: "tool",
                    content: result,
                });
            }
        }
    }
}
```

**工具 Trait 和注册表**

```rust
#[async_trait]
pub trait AgentTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;
    
    async fn execute(
        &self,
        params: Value,
        context: &ToolContext,
    ) -> Result<String>;
}

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn AgentTool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self { tools: HashMap::new() };
        
        // 注册内置工具
        registry.register(Arc::new(WebSearchTool::new()));
        registry.register(Arc::new(WebFetchTool::new()));
        registry.register(Arc::new(ImageGenerationTool::new()));
        registry.register(Arc::new(MemorySearchTool::new()));
        registry.register(Arc::new(BrowserTool::new()));
        registry.register(Arc::new(CronTool::new()));
        
        registry
    }
    
    pub async fn execute(&self, name: &str, params: &Value) -> Result<String> {
        let tool = self.tools.get(name)
            .ok_or_else(|| Error::Tool { 
                tool: name.to_string(), 
                message: "Tool not found".to_string() 
            })?;
        
        tool.execute(params.clone(), &ToolContext::default()).await
    }
}
```

### 6.5 openclaw-memory

**混合搜索实现**

```rust
pub struct MemoryIndexManager {
    db: Connection,
    embedding_service: Arc<dyn EmbeddingService>,
}

impl MemoryIndexManager {
    pub async fn hybrid_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        // 1. 生成查询嵌入
        let query_embedding = self.embedding_service
            .embed(query)
            .await?;
        
        // 2. 向量搜索
        let vector_results = self.vector_search(&query_embedding, limit * 2)?;
        
        // 3. 全文搜索
        let fts_results = self.fts_search(query, limit * 2)?;
        
        // 4. RRF 融合
        let merged = hybrid_merge(
            &vector_results,
            &fts_results,
            0.7,  // vector_weight
            0.3,  // text_weight
            limit,
        );
        
        Ok(merged)
    }
}

/// Reciprocal Rank Fusion 融合算法
pub fn hybrid_merge(
    vector_results: &[(String, f64)],
    fts_results: &[(String, f64)],
    vector_weight: f32,
    text_weight: f32,
    limit: usize,
) -> Vec<SearchResult> {
    let k = 60.0;  // RRF 常数
    let mut scores: HashMap<String, f64> = HashMap::new();
    
    for (rank, (id, _)) in vector_results.iter().enumerate() {
        *scores.entry(id.clone()).or_default() += 
            vector_weight as f64 / (k + rank as f64 + 1.0);
    }
    
    for (rank, (id, _)) in fts_results.iter().enumerate() {
        *scores.entry(id.clone()).or_default() += 
            text_weight as f64 / (k + rank as f64 + 1.0);
    }
    
    let mut ranked: Vec<_> = scores.into_iter().collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    ranked.truncate(limit);
    
    ranked.into_iter()
        .map(|(id, score)| SearchResult { id, score })
        .collect()
}
```

**嵌入服务**

```rust
#[async_trait]
pub trait EmbeddingService: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
    fn dimensions(&self) -> usize;
}

// OpenAI 实现
pub struct OpenAIEmbeddingService {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

#[async_trait]
impl EmbeddingService for OpenAIEmbeddingService {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let response = self.client
            .post("https://api.openai.com/v1/embeddings")
            .bearer_auth(&self.api_key)
            .json(&json!({
                "model": self.model,
                "input": text
            }))
            .send()
            .await?;
        
        let data: EmbeddingResponse = response.json().await?;
        Ok(data.data[0].embedding.clone())
    }
    
    fn dimensions(&self) -> usize {
        match self.model.as_str() {
            "text-embedding-3-small" => 1536,
            "text-embedding-3-large" => 3072,
            _ => 1536,
        }
    }
}
```

---

## 7. 关键实现细节

### 7.1 配置系统完整流程

```
加载流程:
config.json5 → 读取文件 → 环境变量替换 → JSON5 解析 
    → 旧格式迁移 → Serde 反序列化 → Validator 验证 → ConfigManager

热重载流程:
文件变化 (notify) → 防抖 (500ms) → 重新加载 → 验证 
    → Arc::swap 原子替换 → 广播变更事件
```

### 7.2 WebSocket 连接生命周期

```
Client                              Gateway
  │                                    │
  │─────── TCP Connect ───────────────▶│
  │◀────── TCP Accept ─────────────────│
  │                                    │
  │─────── WS Upgrade Request ────────▶│
  │◀────── WS Upgrade Response ────────│
  │                                    │
  │─────── req:connect ───────────────▶│  必须是第一帧
  │        {type:"req", id, method:"connect",
  │         params:{auth, client, caps}}
  │                                    │
  │◀────── res:connect ────────────────│  或错误后关闭
  │        {type:"res", id, ok:true,
  │         payload:{snapshot, policy}}
  │                                    │
  │◀────── event:presence ─────────────│  服务端推送
  │◀────── event:tick ─────────────────│  心跳
  │                                    │
  │─────── req:agent ─────────────────▶│  运行 Agent
  │◀────── res:agent (accepted) ───────│  立即确认
  │◀────── event:agent (streaming) ────│  流式输出
  │◀────── event:agent (streaming) ────│
  │◀────── res:agent (final) ──────────│  最终结果
  │                                    │
```

### 7.3 Agent 执行循环详解

```
                    ┌────────────────────────────────┐
                    │       AgentRunRequest          │
                    │  - session_key                 │
                    │  - message                     │
                    │  - model                       │
                    │  - thinking_level              │
                    └───────────────┬────────────────┘
                                    │
                                    ▼
                    ┌────────────────────────────────┐
                    │     构建系统提示词               │
                    │  - 基础提示                     │
                    │  - Skills 提示                 │
                    │  - 工具列表                     │
                    │  - 运行时元数据                 │
                    └───────────────┬────────────────┘
                                    │
                                    ▼
                    ┌────────────────────────────────┐
                    │     加载会话历史                │
                    │  - 从文件/缓存加载              │
                    │  - 历史截断 (DM vs 群组)        │
                    │  - 压缩检查                     │
                    └───────────────┬────────────────┘
                                    │
                                    ▼
              ┌────▶┌────────────────────────────────┐
              │     │       调用 LLM API              │
              │     │  - 构建请求                     │
              │     │  - 流式接收                     │
              │     │  - 解析工具调用                 │
              │     └───────────────┬────────────────┘
              │                     │
              │         ┌───────────┴───────────┐
              │         │                       │
              │    有工具调用              无工具调用
              │         │                       │
              │         ▼                       ▼
              │    ┌─────────────┐       ┌─────────────┐
              │    │ 执行工具     │       │ 返回结果    │
              │    │ - 并发执行   │       │ - 保存历史  │
              │    │ - 超时控制   │       │ - 发送回复  │
              │    │ - 错误处理   │       └─────────────┘
              │    └──────┬──────┘
              │           │
              │           ▼
              │    ┌─────────────┐
              │    │ 添加工具结果 │
              └────│ 到消息列表  │
                   └─────────────┘
```

---

## 8. 性能对比

### 8.1 基准测试预估

| 指标 | TypeScript/Node.js | Rust | 提升倍数 |
|------|-------------------|------|---------|
| 冷启动时间 | ~800ms (V8 JIT 预热) | ~10ms (原生二进制) | **80x** |
| 内存占用 (空闲) | ~120-200MB (V8 heap + GC 开销) | ~5-15MB | **10-20x** |
| WebSocket 并发 | ~10K (事件循环瓶颈) | ~100K+ (tokio 多线程) | **10x+** |
| JSON 解析吞吐 | V8 内置 (~100MB/s) | serde_json 零拷贝 (~500MB/s) | **3-5x** |
| SQLite 查询 | better-sqlite3 (N-API 开销) | rusqlite (直接 FFI) | **2-3x** |
| 向量相似度计算 | JS 数组 | SIMD 优化 | **5-10x** |
| 部署体积 | ~300MB+ (node_modules) | ~5MB (单二进制) | **60x** |

### 8.2 实测结果

```
平台: macOS arm64
Rust: 1.85 (2024 edition)

Release 二进制大小: 4.9 MB
启动时间 (--version): < 50ms
空闲内存占用: ~8 MB
```

### 8.3 内存安全优势

| 问题类型 | Node.js | Rust |
|---------|---------|------|
| 内存泄漏 | 常见 (闭包、事件监听器) | 所有权系统防止 |
| 空指针 | 运行时错误 | 编译期阻止 (Option) |
| 数据竞争 | 可能 (shared mutable state) | 编译期阻止 (borrow checker) |
| GC 停顿 | 有 (可能影响延迟) | 无 GC |

---

## 9. 测试策略

### 9.1 测试层次

| 测试类型 | 位置 | 工具 | 覆盖内容 |
|----------|------|------|----------|
| 单元测试 | `crates/*/src/*.rs` | `#[test]` | 函数级逻辑 |
| 集成测试 | `tests/integration/` | `#[tokio::test]` | 跨 crate 交互 |
| E2E 测试 | `tests/e2e/` | 外部进程 | 完整流程 |

### 9.2 已实现的测试

```
openclaw-config:
  - env_subst::tests::test_simple_substitution
  - env_subst::tests::test_default_value
  - env_subst::tests::test_missing_var_empty
  - migration::tests::test_daemon_to_gateway_migration
  - migration::tests::test_no_migration_needed

openclaw-memory:
  - chunker::tests::test_chunking
  - chunker::tests::test_small_text
  - chunker::tests::test_empty
  - search::tests::test_cosine_similarity
  - search::tests::test_hybrid_merge

openclaw-reply:
  - directive::tests::test_parse_model_directive
  - directive::tests::test_no_directives
  - directive::tests::test_multiple_directives

openclaw-gateway:
  - auth::tests::test_verify_token

tests/integration:
  - config_test::test_default_config_loading

总计: 15 个测试全部通过
```

### 9.3 测试运行

```bash
# 运行所有测试
cargo test

# 运行特定 crate 的测试
cargo test -p openclaw-config

# 运行带输出的测试
cargo test -- --nocapture

# 运行集成测试
cargo test --test '*'
```

---

## 10. 部署指南

### 10.1 本地开发

```bash
# 1. 克隆并进入项目
cd openclaw-rs

# 2. 构建 Debug
cargo build

# 3. 运行 Gateway
cargo run -- gateway --port 18789 --allow-unconfigured

# 4. 运行 TUI
cargo run -- tui
```

### 10.2 Release 构建

```bash
# 构建优化版本
cargo build --release

# 二进制位于
./target/release/openclaw

# 查看版本
./target/release/openclaw --version
# openclaw 0.1.0 (rust)
```

### 10.3 Docker 部署

**Dockerfile**

```dockerfile
# 多阶段构建
FROM rust:1.85-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin openclaw
RUN strip target/release/openclaw

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/openclaw /usr/local/bin/
EXPOSE 18789
ENTRYPOINT ["/usr/local/bin/openclaw"]
CMD ["gateway", "--allow-unconfigured"]
```

**docker-compose.yml**

```yaml
services:
  gateway:
    build: .
    ports:
      - "18789:18789"
    volumes:
      - ./config:/config:ro
      - openclaw-data:/data/.openclaw
    environment:
      - OPENCLAW_CONFIG_PATH=/config/config.json5
      - OPENAI_API_KEY=${OPENAI_API_KEY:-}
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY:-}
    restart: unless-stopped

volumes:
  openclaw-data:
```

### 10.4 systemd 服务

```ini
[Unit]
Description=OpenClaw Gateway
After=network.target

[Service]
Type=simple
User=openclaw
ExecStart=/usr/local/bin/openclaw gateway
Restart=always
RestartSec=5
Environment=OPENCLAW_CONFIG_PATH=/etc/openclaw/config.json5

[Install]
WantedBy=multi-user.target
```

---

## 11. 从 TypeScript 到 Rust 的转换指南

### 11.1 类型系统映射

| TypeScript | Rust | 说明 |
|------------|------|------|
| `interface Foo { ... }` | `struct Foo { ... }` | 数据类型 |
| `interface Bar { fn(): void }` | `trait Bar { fn ...; }` | 行为抽象 |
| `class Baz { ... }` | `struct Baz { ... }` + `impl Baz { ... }` | 实现分离 |
| `T \| null \| undefined` | `Option<T>` | 可选值 |
| `Promise<T>` | `impl Future<Output = T>` / `async fn` | 异步 |
| `throw new Error(...)` | `return Err(...)` | 错误处理 |
| `any` | 避免使用 / `dyn Any` | 动态类型 |
| `unknown` | 泛型 `T` + trait bounds | 约束未知类型 |
| 运行时类型检查 | 编译期类型检查 | 安全保证时机 |
| 垃圾回收 | 所有权 + 借用检查 | 内存管理 |

### 11.2 常见模式转换

**可空处理**

```typescript
// TypeScript
function getName(user: User | null): string {
  return user?.name ?? "Unknown";
}
```

```rust
// Rust
fn get_name(user: Option<&User>) -> &str {
    user.map(|u| u.name.as_str()).unwrap_or("Unknown")
}
```

**错误处理**

```typescript
// TypeScript
async function fetchData(): Promise<Data> {
  try {
    const response = await fetch(url);
    if (!response.ok) throw new Error("Request failed");
    return response.json();
  } catch (e) {
    throw new Error(`Fetch error: ${e.message}`);
  }
}
```

```rust
// Rust
async fn fetch_data() -> Result<Data, Error> {
    let response = reqwest::get(url).await
        .map_err(|e| Error::Http(e.to_string()))?;
    
    if !response.status().is_success() {
        return Err(Error::Http("Request failed".into()));
    }
    
    response.json().await
        .map_err(|e| Error::Json(e.to_string()))
}
```

**异步迭代**

```typescript
// TypeScript
for await (const chunk of stream) {
  process(chunk);
}
```

```rust
// Rust
while let Some(chunk) = stream.next().await {
    process(chunk)?;
}
```

### 11.3 并发模型差异

| 方面 | Node.js | Rust (tokio) |
|------|---------|--------------|
| 模型 | 单线程事件循环 | 多线程工作窃取 |
| CPU 密集 | 阻塞整个进程 | 可用 `spawn_blocking` |
| 共享状态 | 自由访问 (单线程) | 需要 `Arc<Mutex<T>>` |
| 通道 | EventEmitter | `mpsc`, `broadcast` |
| 取消 | AbortController | `CancellationToken` |

---

## 12. 后续优化方向

### 12.1 功能完善

- [ ] 完整的 LLM provider 路由 (OpenAI, Anthropic, Google, OpenRouter)
- [ ] 流式响应 (SSE) 完整实现
- [ ] 工具执行的真实集成
- [ ] sqlite-vec 扩展动态加载
- [ ] 完整的 Telegram/Discord 长轮询/Gateway 连接
- [ ] WhatsApp 协议实现 (复杂度高)

### 12.2 性能优化

- [ ] 连接池复用 (HTTP client)
- [ ] 内存分配器优化 (jemalloc/mimalloc)
- [ ] SIMD 向量计算加速
- [ ] 会话缓存 LRU
- [ ] 编译期优化 (PGO, LTO)

### 12.3 可观测性

- [ ] OpenTelemetry 集成
- [ ] Prometheus metrics 端点
- [ ] 结构化日志导出
- [ ] 分布式追踪

### 12.4 安全加固

- [ ] TLS 支持
- [ ] 证书固定
- [ ] Rate limiting
- [ ] IP 白名单

---

## 13. 附录

### 13.1 项目统计

| 指标 | 数值 |
|------|------|
| Crate 数量 | 10 |
| Rust 源文件 | 86 |
| 代码行数 | ~6,500 |
| Release 二进制 | 4.9 MB |
| 测试通过 | 15/15 |

### 13.2 关键文件路径

```
openclaw-rs/
├── Cargo.toml                           # Workspace 配置
├── crates/
│   ├── openclaw-core/src/
│   │   ├── lib.rs                       # 模块导出
│   │   ├── error.rs                     # 错误定义
│   │   ├── types.rs                     # 核心类型
│   │   ├── message.rs                   # 消息模型
│   │   └── channel.rs                   # Channel trait
│   ├── openclaw-config/src/
│   │   ├── schema.rs                    # 配置 Schema
│   │   ├── loader.rs                    # 配置加载器
│   │   ├── env_subst.rs                 # 环境变量替换
│   │   └── watcher.rs                   # 热重载
│   ├── openclaw-gateway/src/
│   │   ├── server.rs                    # 服务器启动
│   │   ├── routes/                      # HTTP 路由
│   │   ├── ws/                          # WebSocket 处理
│   │   └── auth.rs                      # 认证
│   ├── openclaw-agent/src/
│   │   ├── runner.rs                    # Agent 执行引擎
│   │   ├── session.rs                   # 会话管理
│   │   ├── tools/                       # 工具实现
│   │   └── skills/                      # 技能系统
│   ├── openclaw-memory/src/
│   │   ├── sqlite.rs                    # SQLite 操作
│   │   ├── embeddings/                  # 嵌入服务
│   │   └── search.rs                    # 混合搜索
│   └── openclaw-cli/src/
│       ├── main.rs                      # CLI 入口
│       ├── commands/                    # 子命令
│       └── tui/                         # TUI 界面
├── config/config.example.json5          # 示例配置
├── Dockerfile                           # Docker 构建
├── docker-compose.yml                   # Docker Compose
└── README.md                            # 项目说明
```

### 13.3 参考资料

- [原版 OpenClaw 项目](../openclaw/)
- [Pi SDK 文档](https://github.com/badlogic/pi-mono)
- [Axum 框架](https://github.com/tokio-rs/axum)
- [Tokio 异步运行时](https://tokio.rs/)
- [Serde 序列化](https://serde.rs/)
- [Rusqlite](https://github.com/rusqlite/rusqlite)

---

*文档版本: 2.0*  
*最后更新: 2026-02-20*
