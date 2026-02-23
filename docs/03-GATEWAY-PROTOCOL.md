# Gateway 与 WebSocket 协议详解

> 适合人群：想理解 OpenClaw 如何对外提供服务、客户端如何连接和交互

---

## 一、Gateway 是什么？

Gateway（网关）是 OpenClaw 对外的"门面"，负责：

1. **接收来自外部的请求**（HTTP API、WebSocket 连接）
2. **认证**（验证 Token）
3. **路由**（把请求分发给对应的处理器）
4. **将通道 Webhook 转发给对应插件**

```
外部世界                         Gateway (port 18789)
─────────                        ─────────────────────
UI 客户端  ──── WebSocket ──────▶  /ws
OpenAI 兼容客户端 ── HTTP ────────▶  /v1/chat/completions
Telegram    ── Webhook ──────────▶  /hooks/telegram_main
管理脚本    ── HTTP ─────────────▶  /api/channels
```

---

## 二、服务器启动流程

文件：`crates/openclaw-gateway/src/server.rs`

```rust
pub async fn start_gateway_server(
    config: Arc<ConfigManager>,
    shutdown_signal: CancellationToken,  // 用于优雅停机
) -> Result<()> {
    // 第一步：创建全局状态（所有路由共享）
    let state = AppState::new(config.clone());
    
    // 第二步：定义路由表
    let app = Router::new()
        // 健康检查
        .route("/health", get(health_check))
        .route("/ready",  get(readiness_check))
        
        // OpenAI 兼容 API
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/models",           get(list_models))
        
        // Webhook 入口（各聊天平台回调到这里）
        .route("/hooks/:name",                post(handle_webhook))
        .route("/api/channels",               get(list_channels))
        .route("/api/channels/:id/webhook",   post(channel_webhook))
        
        // WebSocket 升级端点
        .route("/ws", get(ws_upgrade_handler))
        
        // 中间件（按顺序执行）
        .layer(TraceLayer::new_for_http())   // 请求日志
        .layer(CompressionLayer::new())       // 响应压缩
        .layer(CorsLayer::permissive())       // 跨域（CORS）
        .layer(middleware::from_fn_with_state(
            state.clone(), 
            auth_middleware,                  // 认证中间件
        ))
        .with_state(state);
    
    // 第三步：绑定端口，开始监听
    let addr = SocketAddr::from(([127, 0, 0, 1], config.gateway().port));
    let listener = TcpListener::bind(addr).await?;
    
    tracing::info!("Gateway listening on http://{}", addr);
    
    // 第四步：优雅停机（收到信号时等待现有请求完成）
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal.cancelled_owned())
        .await?;
    
    Ok(())
}
```

---

## 三、AppState：全局共享状态

所有 HTTP/WS 处理函数都可以访问 `AppState`：

```rust
// crates/openclaw-gateway/src/state.rs

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<ConfigManager>,    // 配置（Arc 允许多个处理函数共享）
    pub reply_pipeline: Arc<ReplyPipeline>,   // 消息处理流水线
    pub ws_clients: Arc<DashMap<String, WsClient>>,  // 活跃的 WS 连接
}
```

`Arc`（原子引用计数）是 Rust 中安全共享数据的方式。多个线程都持有 `Arc<AppState>` 的克隆，但实际上指向同一个数据，零拷贝。

---

## 四、认证系统

### 认证中间件

所有请求在到达路由处理器之前，先经过 `auth_middleware`：

```rust
// crates/openclaw-gateway/src/auth.rs

pub async fn auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 不需要认证的路径直接放行
    let path = request.uri().path();
    if path == "/health" || path == "/ready" {
        return Ok(next.run(request).await);
    }
    
    // 从请求头提取 Token
    let token = extract_token(&request)?;
    
    // 验证 Token
    if !verify_token(&token, &state.config) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    
    Ok(next.run(request).await)
}

fn extract_token(request: &Request) -> Result<String, StatusCode> {
    // 优先从 Authorization: Bearer <token> 头提取
    if let Some(auth) = request.headers().get("Authorization") {
        let value = auth.to_str().map_err(|_| StatusCode::UNAUTHORIZED)?;
        if let Some(token) = value.strip_prefix("Bearer ") {
            return Ok(token.to_string());
        }
    }
    
    // 其次从 query 参数 ?token=xxx 提取
    if let Some(query) = request.uri().query() {
        for (k, v) in url::form_urlencoded::parse(query.as_bytes()) {
            if k == "token" {
                return Ok(v.to_string());
            }
        }
    }
    
    Err(StatusCode::UNAUTHORIZED)
}
```

---

## 五、WebSocket 协议——双向实时通信

WebSocket 是 OpenClaw 最核心的通信方式，UI 客户端通过 WS 接收流式 AI 回复。

### 协议格式

所有 WS 消息都是 JSON 格式，有三种类型：

```
┌──────────────────────────────────────────────────────────────┐
│ 类型1：req（请求）——客户端发给服务器                           │
├──────────────────────────────────────────────────────────────┤
│ {                                                            │
│   "type": "req",                                             │
│   "id": "req_abc123",      ← 请求 ID（用于匹配响应）          │
│   "method": "agent",       ← 调用哪个方法                    │
│   "params": { ... }        ← 方法参数                        │
│ }                                                            │
└──────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────┐
│ 类型2：res（响应）——服务器发给客户端                           │
├──────────────────────────────────────────────────────────────┤
│ {                                                            │
│   "type": "res",                                             │
│   "id": "req_abc123",      ← 对应请求的 ID                   │
│   "ok": true,              ← 是否成功                        │
│   "payload": { ... },      ← 成功时的数据                    │
│   "error": { ... }         ← 失败时的错误信息                │
│ }                                                            │
└──────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────┐
│ 类型3：event（事件）——服务器主动推送给客户端                   │
├──────────────────────────────────────────────────────────────┤
│ {                                                            │
│   "type": "event",                                           │
│   "event": "agent.stream",  ← 事件名                        │
│   "payload": { ... },       ← 事件数据                       │
│   "seq": 42,                ← 序列号（防乱序）               │
│   "stateVersion": 7         ← 服务器状态版本                 │
│ }                                                            │
└──────────────────────────────────────────────────────────────┘
```

### Rust 中的类型定义

```rust
// crates/openclaw-gateway/src/ws/mod.rs

#[derive(Debug, Serialize, Deserialize)]
pub struct WsMessage {
    #[serde(rename = "type")]
    pub msg_type: WsMessageType,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,          // req/res 用
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,      // req 用
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,       // req 用
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,      // res/event 用
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<WsError>,      // res 失败时用
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,       // event 用
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<u64>,            // event 用
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WsMessageType {
    Req,    // 客户端请求
    Res,    // 服务器响应
    Event,  // 服务器主动推送
}
```

---

## 六、WebSocket 连接生命周期

```
客户端                                    Gateway 服务器
  │                                            │
  │─────── TCP 三次握手 ───────────────────────▶│
  │◀────── TCP 建立 ────────────────────────────│
  │                                            │
  │─────── HTTP GET /ws                ────────▶│  Upgrade: websocket
  │◀────── HTTP 101 Switching Protocols ────────│  协议升级成功
  │                                            │
  │═══════════════════ WebSocket 连接建立 ═══════════════════│
  │                                            │
  │ 【必须第一帧】                              │
  │─────── req:connect ────────────────────────▶│
  │  {                                         │
  │    type: "req",                            │
  │    id: "req_001",                          │
  │    method: "connect",                      │
  │    params: {                               │
  │      auth: "Bearer <token>",  ← 认证       │
  │      client: "my-ui",         ← 客户端标识 │
  │      caps: ["streaming"]      ← 能力声明   │
  │    }                                       │
  │  }                                         │
  │                                            │
  │◀────── res:connect ─────────────────────────│
  │  {                                         │
  │    type: "res",                            │
  │    id: "req_001",                          │
  │    ok: true,                               │
  │    payload: {                              │
  │      snapshot: { channels, agents, ... },  │ ← 当前系统状态快照
  │      policy: { maxAgentTurns: 25 }         │
  │    }                                       │
  │  }                                         │
  │                                            │
  │◀────── event:presence ──────────────────────│  服务器主动推送在线状态
  │◀────── event:tick ──────────────────────────│  心跳（每30秒）
  │                                            │
  │─────── req:agent ──────────────────────────▶│  发起 AI 对话
  │  {                                         │
  │    type: "req",                            │
  │    id: "req_002",                          │
  │    method: "agent",                        │
  │    params: {                               │
  │      session: "my_session",                │
  │      message: "帮我搜索 AI 新闻",           │
  │      model: "openai/gpt-4o"               │
  │    }                                       │
  │  }                                         │
  │                                            │
  │◀────── res:agent (accepted) ────────────────│  立即确认收到
  │  { type: "res", id: "req_002", ok: true }  │
  │                                            │
  │◀────── event:agent.stream ──────────────────│  流式输出（多次）
  │  { event: "agent.stream",                  │
  │    payload: { delta: "根据最新" } }         │
  │                                            │
  │◀────── event:agent.stream ──────────────────│
  │  { payload: { delta: "搜索结果..." } }      │
  │                                            │
  │◀────── event:agent.tool_start ──────────────│  工具调用开始
  │  { payload: { tool: "web_search", ... } }  │
  │                                            │
  │◀────── event:agent.tool_end ────────────────│  工具调用结束
  │                                            │
  │◀────── event:agent.done ────────────────────│  AI 回复完成
  │  { payload: { usage: { tokens: 1234 } } }  │
  │                                            │
  │─────── req:disconnect 或 直接关闭 ─────────▶│
```

---

## 七、主要 RPC 方法

| 方法名 | 调用方向 | 功能 |
|--------|---------|------|
| `connect` | 客户端→服务器 | 握手认证（必须第一帧）|
| `health` | 客户端→服务器 | 获取系统健康状态 |
| `status` | 客户端→服务器 | 获取简短状态摘要 |
| `agent` | 客户端→服务器 | 运行 AI Agent（流式响应）|
| `send` | 客户端→服务器 | 通过指定通道发送消息 |
| `system-presence` | 客户端→服务器 | 获取在线状态列表 |
| `node.*` | 客户端→服务器 | 节点管理（配对、调用）|

---

## 八、HTTP API：OpenAI 兼容接口

OpenClaw 实现了 OpenAI 兼容的 API，可以用任何 OpenAI 客户端直接接入：

```bash
# 发送消息（流式）
curl -X POST http://localhost:18789/v1/chat/completions \
  -H "Authorization: Bearer <your-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "openai/gpt-4o",
    "messages": [
      {"role": "user", "content": "你好！"}
    ],
    "stream": true
  }'

# 列出可用模型
curl http://localhost:18789/v1/models \
  -H "Authorization: Bearer <your-token>"
```

---

## 九、Webhook 处理——接收聊天平台回调

Telegram、Discord 等平台通过 Webhook 把消息推送给 OpenClaw：

```
Telegram 服务器
    │  POST /hooks/telegram_main
    │  Body: { update_id: ..., message: { text: "hello", ... } }
    ▼
Gateway /hooks/:name
    │
    ├── 根据 :name 找到对应的 Channel 插件
    │   name = "telegram_main" → TelegramChannel
    │
    ├── 调用 channel.handle_webhook(request)
    │
    └── Channel 插件解析 Telegram 格式
        → 转换为 InboundMessage
        → 交给 ReplyPipeline 处理
```

### Webhook 路由处理器

```rust
// crates/openclaw-gateway/src/routes/hooks.rs

pub async fn handle_webhook(
    Path(name): Path<String>,
    State(state): State<AppState>,
    request: Request<Body>,
) -> impl IntoResponse {
    // 找到对应的 Channel
    let channel = state.channel_registry.get(&name)
        .ok_or_else(|| {
            tracing::warn!("Unknown webhook: {}", name);
            StatusCode::NOT_FOUND
        })?;
    
    // 委托给 Channel 插件处理
    channel.handle_webhook(request).await
        .map_err(|e| {
            tracing::error!("Webhook error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}
```

---

## 十、中间件栈详解

axum 中间件按照**添加时的逆序**执行（洋葱模型）：

```
请求进入
    │
    ▼ auth_middleware     ← 最后添加的，最先执行
    │  └─ 验证 Token，未通过直接返回 401
    │
    ▼ CompressionLayer    ← 响应压缩（gzip/br/zstd）
    │
    ▼ CorsLayer           ← 跨域处理
    │
    ▼ TraceLayer          ← 请求追踪日志（记录请求/响应）
    │
    ▼ 路由处理器（handler）
    │
响应返回（经过同样的中间件，但方向相反）
```

---

## 十一、并发模型——为什么能支持 10 万+ WebSocket

Rust + tokio 的异步模型让每个 WebSocket 连接只占用很少的资源：

```
传统线程模型（Node.js 的 ws 库）：
  连接1  →  线程1  （1MB 栈内存）
  连接2  →  线程2  （1MB 栈内存）
  ...
  1万连接 → 1万线程 → ~10GB 内存

tokio 异步模型（OpenClaw Rust）：
  连接1  →  Task1  （几KB 内存）
  连接2  →  Task2  （几KB 内存）
  ...
  10万连接 → 10万 Task → ~1GB 内存（100 倍节省）
```

每个 WebSocket 连接对应一个 tokio `Task`（轻量级协程），等待消息时不占用线程，只在有数据时才被调度。

---

## 十二、快速测试 Gateway

启动服务器后，可以用这些命令验证：

```bash
# 健康检查（无需认证）
curl http://localhost:18789/health
# 返回: {"status":"ok","version":"2.0.0"}

# 有认证的请求
curl http://localhost:18789/v1/models \
  -H "Authorization: Bearer your_token_here"

# WebSocket 测试（需要 websocat 工具）
websocat ws://localhost:18789/ws?token=your_token_here

# 发送 connect 帧
{"type":"req","id":"1","method":"connect","params":{"auth":"Bearer token","client":"test"}}
```

---

*文档生成时间: 2026-02-23 | 基于 OpenClaw Rust Edition v2.0*
