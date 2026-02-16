use openclaw_core::{AgentId, EmbeddingProvider, LlmProvider};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use validator::Validate;

/// Root configuration schema for OpenClaw.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct OpenClawConfig {
    /// Gateway server settings.
    #[serde(default)]
    pub gateway: GatewayConfig,

    /// Agent definitions.
    #[serde(default)]
    pub agents: HashMap<String, AgentConfig>,

    /// Model provider configuration.
    #[serde(default)]
    pub models: ModelsConfig,

    /// Memory / RAG settings.
    #[serde(default)]
    pub memory: MemoryConfig,

    /// Session defaults.
    #[serde(default)]
    pub sessions: SessionsConfig,

    /// Channel plugin configurations (keyed by channel ID).
    #[serde(default)]
    pub channels: HashMap<String, serde_json::Value>,

    /// Hook definitions.
    #[serde(default)]
    pub hooks: Vec<HookConfig>,

    /// Broadcast targets.
    #[serde(default)]
    pub broadcast: Vec<BroadcastConfig>,

    /// Approvals configuration.
    #[serde(default)]
    pub approvals: ApprovalsConfig,

    /// Logging settings.
    #[serde(default)]
    pub logging: LoggingConfig,
}

impl Default for OpenClawConfig {
    fn default() -> Self {
        Self {
            gateway: GatewayConfig::default(),
            agents: HashMap::new(),
            models: ModelsConfig::default(),
            memory: MemoryConfig::default(),
            sessions: SessionsConfig::default(),
            channels: HashMap::new(),
            hooks: Vec::new(),
            broadcast: Vec::new(),
            approvals: ApprovalsConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayConfig {
    #[serde(default = "default_gateway_port")]
    pub port: u16,

    #[serde(default)]
    pub host: Option<String>,

    #[serde(default)]
    pub auth_token: Option<String>,

    #[serde(default)]
    pub tls: Option<TlsConfig>,

    #[serde(default)]
    pub cors_origins: Vec<String>,

    #[serde(default)]
    pub allowed_origins: Vec<String>,

    /// Max WebSocket connections.
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    /// Control UI enabled.
    #[serde(default = "default_true")]
    pub control_ui: bool,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            port: default_gateway_port(),
            host: None,
            auth_token: None,
            tls: None,
            cors_origins: Vec::new(),
            allowed_origins: Vec::new(),
            max_connections: default_max_connections(),
            control_ui: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TlsConfig {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfig {
    #[serde(default)]
    pub display_name: Option<String>,

    /// Default model for this agent.
    pub model: Option<String>,

    /// System prompt template.
    #[serde(default)]
    pub system_prompt: Option<String>,

    /// Workspace directory path.
    #[serde(default)]
    pub workspace: Option<PathBuf>,

    /// Skills directory path.
    #[serde(default)]
    pub skills_dir: Option<PathBuf>,

    /// Available tools for this agent.
    #[serde(default)]
    pub tools: ToolsConfig,

    /// Maximum conversation turns before compaction.
    #[serde(default = "default_max_turns")]
    pub max_turns: usize,

    /// Temperature for LLM.
    #[serde(default)]
    pub temperature: Option<f32>,

    /// Max tokens for LLM response.
    #[serde(default)]
    pub max_tokens: Option<u32>,

    /// Subagent depth limit.
    #[serde(default = "default_max_subagent_depth")]
    pub max_subagent_depth: u32,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            display_name: None,
            model: None,
            system_prompt: None,
            workspace: None,
            skills_dir: None,
            tools: ToolsConfig::default(),
            max_turns: default_max_turns(),
            temperature: None,
            max_tokens: None,
            max_subagent_depth: default_max_subagent_depth(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsConfig {
    #[serde(default = "default_true")]
    pub web_search: bool,

    #[serde(default = "default_true")]
    pub web_fetch: bool,

    #[serde(default = "default_true")]
    pub memory: bool,

    #[serde(default)]
    pub image_generation: bool,

    #[serde(default)]
    pub browser: bool,

    #[serde(default)]
    pub cron: bool,

    #[serde(default)]
    pub code_execution: bool,

    #[serde(default)]
    pub custom: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelsConfig {
    /// Provider API keys.
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,

    /// Model aliases (e.g., "fast" -> "openai/gpt-4o-mini").
    #[serde(default)]
    pub aliases: HashMap<String, String>,

    /// Default model for all agents.
    #[serde(default)]
    pub default_model: Option<String>,
}

impl Default for ModelsConfig {
    fn default() -> Self {
        Self {
            providers: HashMap::new(),
            aliases: HashMap::new(),
            default_model: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default)]
    pub embedding_provider: Option<String>,

    #[serde(default)]
    pub embedding_model: Option<String>,

    #[serde(default = "default_chunk_size")]
    pub chunk_size: usize,

    #[serde(default = "default_chunk_overlap")]
    pub chunk_overlap: usize,

    /// Max results for memory search.
    #[serde(default = "default_search_limit")]
    pub search_limit: usize,

    #[serde(default = "default_vector_weight")]
    pub vector_weight: f32,

    #[serde(default = "default_text_weight")]
    pub text_weight: f32,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            embedding_provider: None,
            embedding_model: None,
            chunk_size: default_chunk_size(),
            chunk_overlap: default_chunk_overlap(),
            search_limit: default_search_limit(),
            vector_weight: default_vector_weight(),
            text_weight: default_text_weight(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionsConfig {
    /// Default compaction threshold (number of turns).
    #[serde(default = "default_compaction_threshold")]
    pub compaction_threshold: usize,

    /// Session expiry in seconds (0 = never).
    #[serde(default)]
    pub expiry_seconds: u64,

    /// Max concurrent sessions.
    #[serde(default = "default_max_sessions")]
    pub max_sessions: usize,
}

impl Default for SessionsConfig {
    fn default() -> Self {
        Self {
            compaction_threshold: default_compaction_threshold(),
            expiry_seconds: 0,
            max_sessions: default_max_sessions(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookConfig {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub events: Vec<String>,
    #[serde(default)]
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastConfig {
    pub name: String,
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalsConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub require_for: Vec<String>,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
}

impl Default for ApprovalsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            require_for: Vec::new(),
            timeout_seconds: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,

    #[serde(default)]
    pub json: bool,

    #[serde(default)]
    pub file: Option<PathBuf>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            json: false,
            file: None,
        }
    }
}

// Default value helpers
fn default_gateway_port() -> u16 {
    18789
}
fn default_max_connections() -> usize {
    1000
}
fn default_true() -> bool {
    true
}
fn default_max_turns() -> usize {
    100
}
fn default_max_subagent_depth() -> u32 {
    3
}
fn default_chunk_size() -> usize {
    512
}
fn default_chunk_overlap() -> usize {
    64
}
fn default_search_limit() -> usize {
    10
}
fn default_vector_weight() -> f32 {
    0.7
}
fn default_text_weight() -> f32 {
    0.3
}
fn default_compaction_threshold() -> usize {
    50
}
fn default_max_sessions() -> usize {
    10000
}
fn default_log_level() -> String {
    "info".to_string()
}
