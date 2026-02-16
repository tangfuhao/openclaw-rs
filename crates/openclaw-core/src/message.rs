use crate::types::{AgentId, ChannelId, ModelId, SessionKey};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Role in a conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// Media attachment in a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaAttachment {
    pub media_type: MediaType,
    pub url: Option<String>,
    pub data: Option<Vec<u8>>,
    pub mime_type: Option<String>,
    pub filename: Option<String>,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    Image,
    Audio,
    Video,
    Document,
    Sticker,
    Voice,
    Location,
    Contact,
}

/// Content block in a message (text, image, tool_use, tool_result, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    Image {
        source: ImageSource,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    Base64 {
        media_type: String,
        data: String,
    },
    Url {
        url: String,
    },
}

/// An inbound message from a channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    pub id: String,
    pub session_key: SessionKey,
    pub channel_id: ChannelId,
    pub sender_id: String,
    pub sender_name: Option<String>,
    pub text: Option<String>,
    pub media: Vec<MediaAttachment>,
    pub reply_to_message_id: Option<String>,
    pub thread_id: Option<String>,
    pub is_group: bool,
    pub group_id: Option<String>,
    pub group_name: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub raw: Option<serde_json::Value>,
}

impl InboundMessage {
    pub fn text_content(&self) -> &str {
        self.text.as_deref().unwrap_or("")
    }

    pub fn has_media(&self) -> bool {
        !self.media.is_empty()
    }
}

/// An outbound reply to send back through a channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundReply {
    pub id: String,
    pub session_key: SessionKey,
    pub channel_id: ChannelId,
    pub text: Option<String>,
    pub media: Vec<MediaAttachment>,
    pub reply_to_message_id: Option<String>,
    pub thread_id: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// A conversation turn stored in session history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub id: Uuid,
    pub role: Role,
    pub content: Vec<ContentBlock>,
    pub model: Option<ModelId>,
    pub agent_id: Option<AgentId>,
    pub timestamp: DateTime<Utc>,
    pub token_usage: Option<TokenUsage>,
}

/// Token usage statistics for a single LLM call.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub cache_read_tokens: Option<u32>,
    pub cache_write_tokens: Option<u32>,
}

/// Streaming delta from an LLM response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamDelta {
    TextDelta { text: String },
    ToolUseStart { id: String, name: String },
    ToolUseInputDelta { input_json: String },
    ToolUseEnd,
    MessageStart,
    MessageEnd { usage: Option<TokenUsage> },
}

/// Reply dispatch kind (mirrors the TypeScript dispatcher).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplyDispatchKind {
    Tool,
    Block,
    Final,
}
