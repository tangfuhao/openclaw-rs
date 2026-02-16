use async_trait::async_trait;
use openclaw_core::channel::*;
use openclaw_core::message::*;
use openclaw_core::types::*;
use tracing::info;

pub struct IrcChannel {
    info: ChannelInfo,
    status: ChannelStatus,
}

impl IrcChannel {
    pub fn new() -> Self {
        Self {
            info: ChannelInfo {
                id: ChannelId::new("irc"),
                name: "IRC".to_string(),
                description: "IRC protocol integration".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                supports_groups: true,
                supports_threads: false,
                supports_media: false,
                supports_reactions: false,
                supports_editing: false,
                supports_voice: false,
                max_message_length: Some(512),
            },
            status: ChannelStatus::Disconnected,
        }
    }
}

#[async_trait]
impl ChannelPlugin for IrcChannel {
    fn info(&self) -> &ChannelInfo { &self.info }
    async fn initialize(&mut self, _config: serde_json::Value) -> openclaw_core::Result<()> { Ok(()) }
    async fn start(&mut self, _sink: Box<dyn MessageSink>) -> openclaw_core::Result<()> {
        self.status = ChannelStatus::Connected;
        info!("IRC channel started");
        Ok(())
    }
    async fn stop(&mut self) -> openclaw_core::Result<()> {
        self.status = ChannelStatus::Disconnected;
        Ok(())
    }
    async fn send(&self, _reply: OutboundReply) -> openclaw_core::Result<()> { Ok(()) }
    fn status(&self) -> ChannelStatus { self.status.clone() }
}
