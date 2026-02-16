use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for an agent.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn default_agent() -> Self {
        Self("default".to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a channel (e.g., "telegram", "discord", "slack").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelId(pub String);

impl ChannelId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ChannelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Compound session key: channel:accountId:peerId
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionKey(pub String);

impl SessionKey {
    pub fn new(channel: &str, account_id: &str, peer_id: &str) -> Self {
        Self(format!("{channel}:{account_id}:{peer_id}"))
    }

    pub fn from_raw(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Parse the session key into (channel, account_id, peer_id).
    pub fn parts(&self) -> Option<(&str, &str, &str)> {
        let mut parts = self.0.splitn(3, ':');
        let channel = parts.next()?;
        let account_id = parts.next()?;
        let peer_id = parts.next()?;
        Some((channel, account_id, peer_id))
    }
}

impl fmt::Display for SessionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a gateway connection.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(pub Uuid);

impl ConnectionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ConnectionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Model identifier: provider/model-name
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelId {
    pub provider: String,
    pub model: String,
}

impl ModelId {
    pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
        }
    }

    /// Parse "provider/model" format.
    pub fn parse(s: &str) -> Option<Self> {
        let (provider, model) = s.split_once('/')?;
        Some(Self {
            provider: provider.to_string(),
            model: model.to_string(),
        })
    }

    pub fn to_string_repr(&self) -> String {
        format!("{}/{}", self.provider, self.model)
    }
}

impl fmt::Display for ModelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.provider, self.model)
    }
}

/// RBAC scope for gateway authentication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Admin,
    Read,
    Write,
    Approvals,
    Pairing,
    Node,
}

impl Scope {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Admin => "operator.admin",
            Self::Read => "operator.read",
            Self::Write => "operator.write",
            Self::Approvals => "operator.approvals",
            Self::Pairing => "operator.pairing",
            Self::Node => "node",
        }
    }

    /// Admin implies all other scopes.
    pub fn implies(&self, other: &Scope) -> bool {
        matches!(self, Self::Admin) || self == other
    }
}

/// LLM provider enumeration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    OpenAi,
    Anthropic,
    Google,
    OpenRouter,
    AwsBedrock,
    Custom(String),
}

impl fmt::Display for LlmProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OpenAi => write!(f, "openai"),
            Self::Anthropic => write!(f, "anthropic"),
            Self::Google => write!(f, "google"),
            Self::OpenRouter => write!(f, "openrouter"),
            Self::AwsBedrock => write!(f, "aws-bedrock"),
            Self::Custom(name) => write!(f, "{name}"),
        }
    }
}

/// Embedding provider enumeration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddingProvider {
    OpenAi,
    Gemini,
    Voyage,
    Local,
    Auto,
}

impl fmt::Display for EmbeddingProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OpenAi => write!(f, "openai"),
            Self::Gemini => write!(f, "gemini"),
            Self::Voyage => write!(f, "voyage"),
            Self::Local => write!(f, "local"),
            Self::Auto => write!(f, "auto"),
        }
    }
}
