pub mod api;
pub mod traits;
pub mod types;

// Re-export core channel types for plugin authors
pub use openclaw_core::channel::{ChannelInfo, ChannelPlugin, ChannelStatus, MessageSink};
pub use openclaw_core::message::{InboundMessage, MediaAttachment, MediaType, OutboundReply};
pub use openclaw_core::types::{ChannelId, SessionKey};
pub use traits::PluginService;
pub use types::*;
