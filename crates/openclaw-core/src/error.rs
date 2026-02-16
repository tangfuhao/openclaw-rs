use thiserror::Error;

/// Unified error type for the OpenClaw system.
#[derive(Error, Debug)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Channel error [{channel}]: {message}")]
    Channel { channel: String, message: String },

    #[error("Agent error: {0}")]
    Agent(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Memory/RAG error: {0}")]
    Memory(String),

    #[error("Plugin error [{plugin}]: {message}")]
    Plugin { plugin: String, message: String },

    #[error("Gateway error: {0}")]
    Gateway(String),

    #[error("LLM provider error [{provider}]: {message}")]
    LlmProvider { provider: String, message: String },

    #[error("Tool execution error [{tool}]: {message}")]
    ToolExecution { tool: String, message: String },

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Rate limited: {0}")]
    RateLimited(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, Error>;
