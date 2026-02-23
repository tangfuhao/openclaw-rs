# Auto-Reply 消息流水线详解

> 适合人群：想理解消息从进入到输出经历了哪些处理步骤

---

## 一、为什么需要流水线？

如果没有流水线，消息处理会是这样：

```
❌ 简单直连（有很多问题）：
消息进来 → 直接调用 AI → 直接发回去

问题：
- 用户连续快速发消息，每条都调用 AI？（浪费钱）
- 用户发"/model:gpt-4 帮我搜索..."，模型切换哪里处理？
- 消息里附带图片，怎么处理？
- 多个用户同时发消息，如何并发不混乱？
```

流水线把这些问题分而治之，每个步骤专注做好一件事：

```
✅ 流水线（分步处理）：
消息进来
  → [步骤1] 队列管理（防抖去重）
  → [步骤2] 解析指令
  → [步骤3] 确定 Agent 和模型
  → [步骤4] 调用 Agent
  → [步骤5] 收集流式输出
  → 发回给用户
```

---

## 二、流水线全景图

```
crates/openclaw-reply/src/
├── pipeline.rs    ← 主流水线（协调者）
├── directive.rs   ← 指令解析（/model: 等）
├── queue.rs       ← 消息队列（防抖去重）
├── dispatcher.rs  ← 回复分发（格式化、发送）
└── media.rs       ← 媒体处理（图片描述等）
```

---

## 三、ReplyPipeline：核心结构体

```rust
// crates/openclaw-reply/src/pipeline.rs

pub struct ReplyPipeline {
    config: ConfigManager,                // 读取配置
    agent_runner: Arc<AgentRunner>,       // 调用 AI
    message_queue: Arc<MessageQueue>,     // 防抖去重队列
}
```

### process() 方法——流水线入口

```rust
pub async fn process(&self, message: InboundMessage) -> anyhow::Result<Option<OutboundReply>> {
    let session_key = message.session_key.clone();
    
    // ═══ 步骤 1：队列管理 ═══
    if !self.message_queue.enqueue(&session_key, &message).await {
        return Ok(None);  // 被去重或限流，跳过处理
    }
    
    // ═══ 步骤 2：确定 Agent ═══
    let agent_id = self.resolve_agent(&message);
    let agent_config = config.agents.get(agent_id.as_str()).cloned().unwrap_or_default();
    
    // ═══ 步骤 3：解析指令 ═══
    let text = message.text_content().to_string();
    let (clean_text, directives) = parse_directives(&text);
    
    // ═══ 步骤 4：确定模型 ═══
    let model = self.resolve_model(&directives, &agent_config, &config);
    
    // ═══ 步骤 5：构建 Agent 请求 ═══
    let run_request = AgentRunRequest {
        session_key: session_key.clone(),
        agent_id,
        model,
        message: clean_text,
        media: message.media.clone(),
        config: agent_config,
    };
    
    // ═══ 步骤 6：并发运行 Agent + 收集流式输出 ═══
    let (delta_tx, mut delta_rx) = mpsc::unbounded_channel::<StreamDelta>();
    let runner = self.agent_runner.clone();
    let run_handle = tokio::spawn(async move { 
        runner.run(run_request, delta_tx).await 
    });
    
    let mut response_text = String::new();
    while let Some(delta) = delta_rx.recv().await {
        match &delta {
            StreamDelta::TextDelta { text } => response_text.push_str(text),
            StreamDelta::MessageEnd { .. } => break,
            _ => {}
        }
    }
    
    // ═══ 步骤 7：返回结果 ═══
    match run_handle.await {
        Ok(Ok(result)) => {
            self.message_queue.dequeue(&session_key).await;
            Ok(Some(OutboundReply { text: Some(result.response_text), ... }))
        }
        Ok(Err(e)) => { ... }  // 处理错误
        Err(e) => { ... }      // 处理 panic
    }
}
```

---

## 四、步骤详解：指令解析

### 什么是指令？

用户可以在消息开头加上特殊指令来控制 AI 行为：

```
/model:gpt-4o 帮我写一首诗
/think:high 分析这个复杂问题
/agent:coding 帮我 debug 这段代码
/clear 清除对话历史
```

### parse_directives() 实现

```rust
// crates/openclaw-reply/src/directive.rs

#[derive(Debug, Clone)]
pub enum Directive {
    Model(String),       // /model:xxx
    Think(String),       // /think:low|medium|high
    Agent(String),       // /agent:xxx
    Clear,               // /clear
    NoReply,             // /noreply（不回复）
    Custom(String, String),  // 其他自定义指令
}

/// 从消息文本中解析指令，返回 (清理后的文本, 指令列表)
pub fn parse_directives(text: &str) -> (String, Vec<Directive>) {
    let mut directives = Vec::new();
    let mut clean_lines = Vec::new();
    
    for line in text.lines() {
        if let Some(directive) = try_parse_directive(line.trim()) {
            directives.push(directive);
        } else {
            clean_lines.push(line);
        }
    }
    
    // 去掉首尾空行
    let clean_text = clean_lines.join("\n").trim().to_string();
    
    (clean_text, directives)
}

fn try_parse_directive(s: &str) -> Option<Directive> {
    if !s.starts_with('/') {
        return None;
    }
    
    // 解析 /key:value 格式
    let rest = &s[1..];
    if let Some((key, value)) = rest.split_once(':') {
        let value = value.trim().to_string();
        return Some(match key.to_lowercase().as_str() {
            "model" | "m"  => Directive::Model(value),
            "think" | "t"  => Directive::Think(value),
            "agent" | "a"  => Directive::Agent(value),
            _              => Directive::Custom(key.to_string(), value),
        });
    }
    
    // 解析无参数指令
    Some(match rest.to_lowercase().as_str() {
        "clear"   => Directive::Clear,
        "noreply" => Directive::NoReply,
        _ => return None,
    })
}
```

### 指令解析示例

```
输入: "/model:claude-3-5-sonnet /think:high\n帮我分析这个问题"

解析结果:
  clean_text: "帮我分析这个问题"
  directives: [
    Directive::Model("claude-3-5-sonnet"),
    Directive::Think("high"),
  ]
```

---

## 五、步骤详解：模型解析优先级

当有多个地方配置了模型时，按以下优先级决定：

```
优先级从高到低：

1. 消息中的指令  /model:gpt-4o
           ↓
2. Agent 配置    agents.default.model = "claude-3-5-sonnet"
           ↓
3. 全局默认      models.default = "openai/gpt-4o"
           ↓
4. 硬编码兜底    "openai/gpt-4o"
```

代码实现：

```rust
fn resolve_model(
    &self,
    directives: &[Directive],
    agent_config: &AgentConfig,
    config: &OpenClawConfig,
) -> ModelId {
    // 优先：消息指令
    for d in directives {
        if let Directive::Model(model_str) = d {
            if let Some(model) = ModelId::parse(model_str) {
                return model;
            }
            // 尝试别名解析
            if let Some(resolved) = config.models.aliases.get(model_str.as_str()) {
                if let Some(model) = ModelId::parse(resolved) {
                    return model;
                }
            }
        }
    }
    
    // 次优先：Agent 配置
    if let Some(model_str) = &agent_config.model {
        if let Some(model) = ModelId::parse(model_str) {
            return model;
        }
    }
    
    // 再次：全局默认
    if let Some(model_str) = &config.models.default_model {
        if let Some(model) = ModelId::parse(model_str) {
            return model;
        }
    }
    
    // 最后兜底
    ModelId::new("openai", "gpt-4o")
}
```

---

## 六、步骤详解：消息队列（防抖去重）

### 为什么需要队列？

```
场景1（去重）：
用户在 1 秒内快速发了 3 条消息：
  "帮我"
  "帮我搜"
  "帮我搜索 AI 新闻"
→ 只有最后一条真正触发 AI，前两条被忽略

场景2（防抖）：
用户发消息后立刻撤回，又发了新的
→ 只处理最终消息

场景3（限流）：
同一用户疯狂发消息
→ 避免 API 被刷爆
```

### MessageQueue 实现

```rust
// crates/openclaw-reply/src/queue.rs

pub struct MessageQueue {
    // 每个 session 一个队列条目
    sessions: DashMap<SessionKey, QueueEntry>,
}

struct QueueEntry {
    latest_message: InboundMessage,
    enqueue_time: Instant,
    is_processing: bool,   // 防止同一会话并发处理
}

impl MessageQueue {
    /// 返回 true 表示应该处理，false 表示被过滤掉
    pub async fn enqueue(&self, key: &SessionKey, message: &InboundMessage) -> bool {
        let mut entry = self.sessions.entry(key.clone()).or_insert(QueueEntry {
            latest_message: message.clone(),
            enqueue_time: Instant::now(),
            is_processing: false,
        });
        
        // 如果正在处理，更新最新消息但不触发新处理
        if entry.is_processing {
            entry.latest_message = message.clone();
            return false;
        }
        
        entry.latest_message = message.clone();
        entry.is_processing = true;
        true
    }
    
    pub async fn dequeue(&self, key: &SessionKey) {
        if let Some(mut entry) = self.sessions.get_mut(key) {
            entry.is_processing = false;
        }
    }
}
```

---

## 七、步骤详解：流式输出处理

### 为什么要流式输出？

AI 生成文字是逐词生成的（token by token），如果等全部生成完再发送，用户需要等 10-30 秒才看到任何内容。

流式输出让用户立刻看到 AI "打字"，体验好很多。

### 并发模型：Producer-Consumer

```
         AgentRunner（生产者）              Pipeline（消费者）
              │                                  │
              │  tokio::spawn 在后台线程运行      │
              │                                  │
              ├──▶ 调用 LLM API（流式）           │
              │                                  │
              │    每生成一段文字                  │
              ├──▶ delta_tx.send(TextDelta{...}) │
              │                                  │
              │    每开始/结束一个工具调用         │
              ├──▶ delta_tx.send(ToolStart{...}) │
              │                                  │
              │    完成时                         │
              └──▶ delta_tx.send(MessageEnd{...})│
                                                 │
                       mpsc::unbounded_channel   │
                                                 │
                   delta_rx.recv().await ◀────────┤
                          │                      │
                     match delta {               │
                       TextDelta → 追加文字       │
                       ToolStart → 可选：通知用户 │
                       MessageEnd → 退出循环      │
                     }                           │
```

### mpsc channel 是什么？

`mpsc` = Multi-Producer Single-Consumer（多生产者单消费者）

```rust
// 创建一个无缓冲通道
let (tx, mut rx) = mpsc::unbounded_channel::<StreamDelta>();

// 生产者（Agent）：发送数据
tx.send(StreamDelta::TextDelta { text: "你好".to_string() })?;
tx.send(StreamDelta::TextDelta { text: "，世界！".to_string() })?;
// tx 被 drop（或 MessageEnd）时，rx 会收到 None，循环结束

// 消费者（Pipeline）：接收数据
while let Some(delta) = rx.recv().await {
    // 处理每个片段
}
```

---

## 八、回复分发器（Dispatcher）

`ReplyDispatcher` 负责把 AI 的输出送达用户，并进行格式化处理：

```rust
// crates/openclaw-reply/src/dispatcher.rs

pub struct ReplyDispatcher {
    channel: Arc<dyn ChannelPlugin>,
}

impl ReplyDispatcher {
    pub async fn dispatch(&self, reply: OutboundReply) -> Result<()> {
        // 1. 处理特殊格式
        let processed_text = self.process_text(reply.text.as_deref().unwrap_or(""));
        
        // 2. 分块发送（避免单条消息过长）
        for chunk in split_into_chunks(&processed_text, MAX_CHUNK_SIZE) {
            self.channel.send_message(&reply.session_key, OutboundReply {
                text: Some(chunk),
                ..reply.clone()
            }).await?;
        }
        
        Ok(())
    }
    
    fn process_text(&self, text: &str) -> String {
        // 剥离 <think>...</think> 思考标签（用户不需要看 AI 的思考过程）
        let text = strip_thinking_tags(text);
        
        // 提取 [[media:url]] 格式的媒体链接
        let text = extract_media_directives(&text);
        
        text
    }
}
```

### 思考标签剥离

部分 LLM（如 Claude extended thinking、QwQ）会在回复中包含思考过程：

```
输入：
<think>
让我思考一下这道数学题...
首先需要找公因数...
计算得到...
</think>
答案是 42。

输出（发给用户）：
答案是 42。
```

---

## 九、媒体处理

### 入站媒体（用户发来图片/文件）

```rust
// crates/openclaw-reply/src/media.rs

pub struct MediaAttachment {
    pub url: Option<String>,
    pub data: Option<Vec<u8>>,  // base64 编码的原始数据
    pub mime_type: String,       // "image/jpeg", "application/pdf" 等
    pub filename: Option<String>,
}

pub async fn process_media(
    attachments: &[MediaAttachment],
    http_client: &reqwest::Client,
) -> Vec<ContentBlock> {
    let mut blocks = Vec::new();
    
    for attachment in attachments {
        match attachment.mime_type.as_str() {
            mime if mime.starts_with("image/") => {
                // 图片：直接以 base64 格式发给 LLM（支持视觉的模型）
                blocks.push(ContentBlock::Image {
                    data: get_image_data(attachment, http_client).await,
                    media_type: attachment.mime_type.clone(),
                });
            }
            "application/pdf" => {
                // PDF：提取文本内容
                let text = extract_pdf_text(attachment).await;
                blocks.push(ContentBlock::Text { text });
            }
            _ => {
                // 其他文件类型：跳过或提示不支持
            }
        }
    }
    
    blocks
}
```

### 出站媒体（AI 回复包含图片）

AI 可以通过特殊语法指示回复中包含图片：

```
AI 回复文本：
"我为你生成了一张图片：[[media:https://example.com/generated.png]]"

Dispatcher 处理后：
1. 发送文本："我为你生成了一张图片："
2. 下载图片 URL
3. 通过平台 API 发送图片附件
```

---

## 十、完整数据流图——汇总

```
┌────────────────────────────────────────────────────────────────────────┐
│                     Auto-Reply 完整流水线                               │
└────────────────────────────────────────────────────────────────────────┘

【入站】
  用户在 Telegram 发："/model:gpt-4o 帮我搜索最新 AI 新闻"

          ↓ (Telegram Webhook → Channel Plugin)

【InboundMessage】{
  session_key: "telegram:user123",
  content: "/model:gpt-4o 帮我搜索最新 AI 新闻",
  media: [],
}

          ↓ (ReplyPipeline::process)

【步骤1：队列管理】
  enqueue("telegram:user123", message) → true（通过，开始处理）

          ↓

【步骤2：指令解析】
  parse_directives("/model:gpt-4o 帮我搜索最新 AI 新闻")
  → clean_text: "帮我搜索最新 AI 新闻"
  → directives: [Directive::Model("gpt-4o")]

          ↓

【步骤3：模型解析】
  resolve_model(directives, ...) → ModelId("openai", "gpt-4o")

          ↓

【步骤4：Agent 调用】（后台 tokio::spawn）
  AgentRunner::run(AgentRunRequest {
    model: "openai/gpt-4o",
    message: "帮我搜索最新 AI 新闻",
    ...
  }, delta_tx)

          ↓ (LLM 决定调用工具)

【工具执行】
  web_search("最新 AI 新闻") → "1. OpenAI... 2. Anthropic..."

          ↓ (LLM 整合结果，生成回复)

【流式输出】（delta_tx 发送，delta_rx 接收）
  TextDelta { "根据" }
  TextDelta { "最新搜索" }
  TextDelta { "，AI 新闻包括：..." }
  MessageEnd { usage: { total_tokens: 456 } }

          ↓

【步骤5：收集回复】
  response_text = "根据最新搜索，AI 新闻包括：..."

          ↓

【步骤6：清理队列】
  dequeue("telegram:user123")

          ↓

【OutboundReply】{
  text: "根据最新搜索，AI 新闻包括：...",
  session_key: "telegram:user123",
}

          ↓ (Channel Plugin)

【出站】用户在 Telegram 收到回复（打字机效果显示）
```

---

## 十一、新手常见疑问

**Q: 如果同一个用户连续发 5 条消息，会并发处理 5 个 Agent 吗？**

A: 不会。`MessageQueue` 的 `is_processing` 标志确保同一个 `session_key` 同时只有一个 Agent 在运行。后续消息会更新队列中的"最新消息"，等上一个处理完后再处理最新的那条。

**Q: 流式输出中途断了怎么办？**

A: `tokio::spawn` 的返回值是 `JoinHandle`，`pipeline.process()` 用 `run_handle.await` 等待结果。如果 task panic 了，`Err(JoinError)` 会被捕获并记录日志，向用户返回友好的错误信息。

**Q: 指令必须在消息开头吗？**

A: 默认按行解析，只要某行以 `/` 开头就会被识别为指令。可以多个指令换行，最后一行是实际消息内容。

**Q: `mpsc::unbounded_channel` 和 `bounded_channel` 有什么区别？**

A: `unbounded`（无界）发送时不阻塞，适合流式输出这种生产速度快于消费速度的场景；`bounded`（有界）当队列满时发送方会阻塞，适合需要背压控制的场景。

---

*文档生成时间: 2026-02-23 | 基于 OpenClaw Rust Edition v2.0*
