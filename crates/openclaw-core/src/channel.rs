use crate::error::Result;
use crate::message::{InboundMessage, OutboundReply};
use crate::types::{ChannelId, SessionKey};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Metadata about a channel plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelInfo {
    pub id: ChannelId,
    pub name: String,
    pub description: String,
    pub version: String,
    pub supports_groups: bool,
    pub supports_threads: bool,
    pub supports_media: bool,
    pub supports_reactions: bool,
    pub supports_editing: bool,
    pub supports_voice: bool,
    pub max_message_length: Option<usize>,
}

/// Status of a channel connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelStatus {
    Connected,
    Connecting,
    Disconnected,
    Error(String),
}

/// Callback interface for channels to deliver inbound messages.
#[async_trait]
pub trait MessageSink: Send + Sync + Debug {
    async fn on_message(&self, message: InboundMessage) -> Result<()>;
    async fn on_status_change(&self, channel_id: &ChannelId, status: ChannelStatus);
}

/// The core trait that every channel plugin must implement.
#[async_trait]
pub trait ChannelPlugin: Send + Sync {
    /// Return static info about this channel.
    fn info(&self) -> &ChannelInfo;

    /// Initialize the channel with the given configuration.
    async fn initialize(&mut self, config: serde_json::Value) -> Result<()>;

    /// Start listening for inbound messages.
    async fn start(&mut self, sink: Box<dyn MessageSink>) -> Result<()>;

    /// Stop the channel.
    async fn stop(&mut self) -> Result<()>;

    /// Send a reply back through this channel.
    async fn send(&self, reply: OutboundReply) -> Result<()>;

    /// Current connection status.
    fn status(&self) -> ChannelStatus;

    /// Resolve a display name for the given peer ID.
    async fn resolve_name(&self, _session_key: &SessionKey) -> Option<String> {
        None
    }

    /// Handle an HTTP webhook request for this channel.
    async fn handle_webhook(
        &self,
        _path: &str,
        _headers: &[(String, String)],
        _body: bytes::Bytes,
    ) -> Result<(u16, String)> {
        Ok((404, "Not found".to_string()))
    }
}
