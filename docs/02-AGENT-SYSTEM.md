# Agent 系统核心流程详解

> 适合人群：已了解项目全貌，想深入理解 AI Agent 是如何工作的

---

## 一、什么是 Agent？为什么需要 Agent 循环？

### 普通 LLM 调用（一问一答）

```
用户："今天天气怎么样？"
LLM："我无法获取实时天气信息。"
```

LLM 不能主动获取信息，只能回答它训练数据里有的东西。

### Agent 模式（可以使用工具）

```
用户："今天北京天气怎么样？"
    ↓
LLM 思考："我需要搜索一下"
    ↓
调用工具: web_search("北京今天天气")
    ↓
工具返回: "北京今日晴天，最高气温25°C"
    ↓
LLM 整合信息: "今天北京天气晴朗，最高气温25°C，适合出行！"
    ↓
用户收到有用的回答
```

**Agent = LLM + 工具调用能力 + 多轮循环**

Agent 会反复循环"思考→调用工具→看结果→再思考"，直到不再需要工具时输出最终答案。

---

## 二、AgentRunner：核心执行引擎

`AgentRunner` 是 `openclaw-agent` crate 的核心结构体，位于 `crates/openclaw-agent/src/runner.rs`。

### 它包含什么

```rust
pub struct AgentRunner {
    session_store: Arc<SessionStore>,    // 会话历史存储
    tool_registry: Arc<ToolRegistry>,   // 工具注册表
    http_client: reqwest::Client,       // HTTP 客户端（调用 LLM API）
}
```

三个组件的分工：
- `session_store`：记住用户说过的话（上下文历史）
- `tool_registry`：知道有哪些工具可以用
- `http_client`：真正去调用 OpenAI/Anthropic/Google 的 API

### 如何创建

```rust
let session_store = Arc::new(SessionStore::new());
let tool_registry = Arc::new(ToolRegistry::with_defaults(http_client.clone()));
let agent_runner = AgentRunner::new(session_store, tool_registry);
```

---

## 三、Agent 执行循环——最关键的部分

这是理解整个 Agent 系统的核心。`AgentRunner::run()` 方法执行以下流程：

```
┌─────────────────────────────────────────────────────────────┐
│                    AgentRunner::run()                        │
└─────────────────────────────────────────────────────────────┘

输入: AgentRunRequest {
    session_key: "telegram_user_123",   ← 哪个会话
    agent_id: "default",               ← 用哪个 Agent
    model: "openai/gpt-4o",            ← 用哪个模型
    message: "帮我搜索 AI 新闻",         ← 用户的消息
    ...
}

第一步：构建系统提示词
━━━━━━━━━━━━━━━━━━━━━━
build_system_prompt(&config, &agent_id)
→ 组合：基础人格提示 + 技能提示 + 工具列表 + 当前时间等

第二步：加载会话历史
━━━━━━━━━━━━━━━━━━━━━━
session_store.get_history(&session_key)
→ 从文件/内存中读取之前的对话记录
→ 防止上下文太长（超出 LLM 的 context window）

第三步：构建 messages 数组
━━━━━━━━━━━━━━━━━━━━━━━━━━
messages = [
    { role: "system",    content: <系统提示词> },
    { role: "user",      content: "上一次用户说的话" },   // 来自历史
    { role: "assistant", content: "上一次 AI 回复的" },   // 来自历史
    { role: "user",      content: "帮我搜索 AI 新闻" },   // 当前消息
]

┌─────────────────────────────────────────────────────────────┐
│                    Agent 主循环（最多 25 轮）                 │
│                                                             │
│  ┌─────────────────────────────────────┐                   │
│  │           调用 LLM API              │                   │
│  │  POST /v1/chat/completions          │                   │
│  │  携带: messages + tools 定义         │                   │
│  │  方式: 流式 (streaming=true)         │                   │
│  └──────────────┬──────────────────────┘                   │
│                 │                                           │
│        LLM 返回什么？                                        │
│         │                                                   │
│    ┌────┴────┐                                              │
│    │         │                                              │
│  纯文本回复  工具调用请求                                     │
│    │         │                                              │
│    ▼         ▼                                              │
│  结束循环   执行工具                                          │
│  保存历史   → 获得工具结果                                   │
│  返回结果   → 添加到 messages                               │
│             → 继续下一轮循环                                 │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 循环为什么最多 25 轮？

防止 Agent 陷入无限循环消耗 Token 和 API 费用。每调用一次 LLM 算一轮。

---

## 四、工具系统——Agent 的双手

### 工具 Trait（接口定义）

每个工具都必须实现 `AgentTool` trait：

```rust
// crates/openclaw-agent/src/tools/mod.rs

#[async_trait]
pub trait AgentTool: Send + Sync {
    fn definition(&self) -> ToolDefinition;           // 告诉 LLM 这个工具是什么
    async fn execute(&self, input: &Value) -> Result<String>;  // 真正执行
}

pub struct ToolDefinition {
    pub name: String,          // 工具名，如 "web_search"
    pub description: String,   // 工具描述（LLM 读这个来决定是否调用）
    pub parameters: Value,     // JSON Schema，描述参数格式
}
```

### 内置工具一览

| 工具名 | 文件 | 功能 |
|--------|------|------|
| `web_search` | `tools/web_search.rs` | 搜索网页 |
| `web_fetch` | `tools/web_fetch.rs` | 获取网页内容 |
| `image` | `tools/image.rs` | 生成图片（调用 DALL-E 等）|
| `memory` | `tools/memory.rs` | 搜索记忆/知识库 |
| `browser` | `tools/browser.rs` | 浏览器自动化 |
| `cron` | `tools/cron.rs` | 定时任务管理 |

### ToolRegistry：工具仓库

```rust
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn AgentTool>>,
}

impl ToolRegistry {
    // 初始化时注册所有默认工具
    pub fn with_defaults(http_client: reqwest::Client) -> Self {
        let mut registry = Self::new();
        registry.register(Arc::new(WebSearchTool::new(http_client.clone())));
        registry.register(Arc::new(WebFetchTool::new(http_client.clone())));
        registry.register(Arc::new(ImageGenerationTool::new(http_client.clone())));
        registry.register(Arc::new(CronTool::new()));
        registry
    }
    
    // 执行工具
    pub async fn execute(&self, name: &str, input: &Value) -> Result<String> {
        let tool = self.tools.get(name)
            .ok_or_else(|| anyhow!("Tool '{}' not found", name))?;
        tool.execute(input).await
    }
}
```

### 工具调用的完整流程（以 web_search 为例）

```
1. LLM 返回一个工具调用请求：
   {
     "tool_calls": [{
       "id": "call_abc123",
       "name": "web_search",
       "input": { "query": "最新 AI 新闻", "num_results": 5 }
     }]
   }

2. AgentRunner 解析工具调用

3. 调用 tool_registry.execute("web_search", input)

4. WebSearchTool 向搜索 API 发起 HTTP 请求

5. 返回搜索结果字符串

6. 构建工具结果消息：
   { role: "tool", content: "搜索结果: 1. OpenAI 发布新模型... 2. ..." }

7. 添加到 messages 数组

8. 开始下一轮 LLM 调用（带着工具结果）
```

---

## 五、会话系统——记住上下文

### SessionStore 的职责

```rust
// crates/openclaw-agent/src/session.rs

pub struct SessionStore {
    // 内存缓存 + 文件持久化
    sessions: DashMap<SessionKey, Vec<ConversationTurn>>,
}

impl SessionStore {
    // 获取历史对话
    pub async fn get_history(&self, key: &SessionKey) -> Result<Vec<ConversationTurn>> { ... }
    
    // 追加新的对话轮次
    pub async fn append_turn(&self, key: &SessionKey, turn: ConversationTurn) -> Result<()> { ... }
    
    // 清空历史（用户说"忘掉之前的话"）
    pub async fn clear(&self, key: &SessionKey) -> Result<()> { ... }
}
```

### SessionKey 是什么？

`SessionKey` 唯一标识一个会话，格式通常是 `{channel_id}:{sender_id}`：

```
telegram_main:user_123456   ← Telegram 用户 123456 在 telegram_main 通道的会话
discord_bot:channel_789     ← Discord 频道 789 的会话
```

同一个用户在同一个通道里的所有对话共享一个 SessionKey，这样 AI 才能记住上下文。

### ConversationTurn：对话记录的最小单位

```rust
pub enum ConversationTurn {
    User {
        content: Vec<ContentBlock>,   // 用户说的话（可含图片等）
        timestamp: DateTime<Utc>,
    },
    Assistant {
        content: String,              // AI 的回复
        tool_calls: Vec<ToolCall>,    // AI 使用的工具
        timestamp: DateTime<Utc>,
    },
    Tool {
        call_id: String,              // 对应哪个工具调用
        name: String,
        result: String,               // 工具返回的结果
    },
}
```

---

## 六、系统提示词构建

`build_system_prompt()` 函数负责组装发给 LLM 的系统提示词：

```rust
// crates/openclaw-agent/src/prompt.rs

pub fn build_system_prompt(config: &AgentConfig, agent_id: &AgentId) -> String {
    let mut parts = Vec::new();
    
    // 1. 基础人格提示
    if let Some(prompt) = &config.system_prompt {
        parts.push(prompt.clone());
    } else {
        parts.push("你是一个智能助手。".to_string());
    }
    
    // 2. 当前时间（让 AI 知道"现在"是什么时候）
    parts.push(format!("当前时间: {}", Utc::now().format("%Y-%m-%d %H:%M UTC")));
    
    // 3. Agent ID（让 AI 知道自己是谁）
    parts.push(format!("Agent: {}", agent_id));
    
    // 4. 技能提示（Skills）
    // 如果配置了技能，追加技能的提示词
    
    parts.join("\n\n")
}
```

**系统提示词的重要性**：它定义了 AI 的"人格"、能力边界和行为规范，每次对话都会带上。

---

## 七、流式输出——边生成边发送

Agent 运行过程中的输出是**流式**的，用 `mpsc` channel 传递：

```rust
// 调用方创建 channel
let (delta_tx, mut delta_rx) = mpsc::unbounded_channel::<StreamDelta>();

// 在后台线程运行 Agent
tokio::spawn(async move {
    agent_runner.run(request, delta_tx).await
});

// 实时接收流式片段
while let Some(delta) = delta_rx.recv().await {
    match delta {
        StreamDelta::TextDelta { text } => {
            // 实时把文字发给用户（打字机效果）
            send_to_user(&text).await;
        }
        StreamDelta::ToolStart { name } => {
            // 显示"正在使用工具..."
            show_tool_indicator(&name).await;
        }
        StreamDelta::MessageEnd { .. } => break,
        _ => {}
    }
}
```

### StreamDelta 的种类

```rust
pub enum StreamDelta {
    TextDelta { text: String },             // AI 生成的文字片段
    ThinkingDelta { thinking: String },     // AI 的思考过程（<think> 标签里的）
    ToolStart { call_id: String, name: String, input: Value },  // 开始调用工具
    ToolResult { call_id: String, result: String },             // 工具返回结果
    MessageEnd { usage: TokenUsage },       // 本次回复结束
}
```

---

## 八、多 LLM 提供商支持

OpenClaw 支持多种 LLM 提供商，通过 `ModelId` 路由到不同的 API：

```rust
// ModelId 格式: "provider/model_name"
ModelId::new("openai", "gpt-4o")
ModelId::new("anthropic", "claude-3-5-sonnet-20241022")
ModelId::new("google", "gemini-2.0-flash")
ModelId::new("openrouter", "meta-llama/llama-3.3-70b-instruct")
```

每个提供商都有各自的 API 格式，但都通过同一套抽象层调用：

```
ModelId("openai/gpt-4o")
    ↓
OpenAI API: POST https://api.openai.com/v1/chat/completions

ModelId("anthropic/claude-3-5-sonnet")
    ↓
Anthropic API: POST https://api.anthropic.com/v1/messages

ModelId("google/gemini-2.0-flash")
    ↓
Google API: POST https://generativelanguage.googleapis.com/...
```

---

## 九、完整调用链代码追踪

从 Pipeline 调用到 Agent 结束，完整的代码路径：

```
ReplyPipeline::process()          [openclaw-reply/src/pipeline.rs]
    │
    ├── parse_directives()         [openclaw-reply/src/directive.rs]
    │   └── 解析 /model:xxx 等指令
    │
    ├── resolve_model()
    │   └── 确定用哪个 LLM
    │
    └── agent_runner.run()         [openclaw-agent/src/runner.rs]
            │
            ├── build_system_prompt()   [openclaw-agent/src/prompt.rs]
            │
            ├── session_store.get_history()  [openclaw-agent/src/session.rs]
            │
            ├── call_llm()              [内部：发 HTTP 请求到 LLM API]
            │   └── 流式解析 SSE 响应
            │
            ├── [如果有工具调用]
            │   └── tool_registry.execute()  [openclaw-agent/src/tools/mod.rs]
            │           └── 具体工具（web_search 等）
            │
            └── session_store.append_turn()  [保存历史]
```

---

## 十、新手常见疑问

**Q: Agent 循环最多 25 轮，那如果任务很复杂怎么办？**
A: 25 轮已经够用于大多数任务（每轮调用一次 LLM）。复杂任务可以通过子 Agent（subagent）分解，让一个 Agent 调用另一个 Agent。

**Q: 多个用户同时发消息，会互相影响吗？**
A: 不会。每个用户有自己的 `SessionKey`，`SessionStore` 用 `DashMap`（并发安全的哈希表）隔离，`AgentRunner` 是无状态的，可以并发执行。

**Q: LLM 调用失败了怎么办？**
A: `anyhow::Result` 会把错误向上传播，`ReplyPipeline` 会捕获错误并记录日志，向用户返回错误提示。

**Q: 工具超时怎么处理？**
A: `reqwest::Client` 构建时设置了 300 秒超时（针对 LLM 长时间思考的场景）。工具层也有各自的超时配置。

---

*文档生成时间: 2026-02-23 | 基于 OpenClaw Rust Edition v2.0*
