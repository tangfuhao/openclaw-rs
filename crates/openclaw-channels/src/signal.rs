use async_trait::async_trait;
use openclaw_core::channel::*;
use openclaw_core::message::*;
use openclaw_core::types::*;
use tracing::info;

pub struct SignalChannel {
    info: ChannelInfo,
    status: ChannelStatus,
}

impl SignalChannel {
    pub fn new() -> Self {
        Self {
            info: ChannelInfo {
                id: ChannelId::new("signal"),
                name: "Signal".to_string(),
                description: "Signal Messenger integration via signal-cli".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                supports_groups: true,
                supports_threads: false,
                supports_media: true,
                supports_reactions: true,
                supports_editing: false,
                supports_voice: false,
                max_message_length: None,
            },
            status: ChannelStatus::Disconnected,
        }
    }
}

#[async_trait]
impl ChannelPlugin for SignalChannel {
    fn info(&self) -> &ChannelInfo { &self.info }
    async fn initialize(&mut self, _config: serde_json::Value) -> openclaw_core::Result<()> { Ok(()) }
    async fn start(&mut self, _sink: Box<dyn MessageSink>) -> openclaw_core::Result<()> {
        self.status = ChannelStatus::Connected;
        info!("Signal channel started");
        Ok(())
    }
    async fn stop(&mut self) -> openclaw_core::Result<()> {
        self.status = ChannelStatus::Disconnected;
        Ok(())
    }
    async fn send(&self, _reply: OutboundReply) -> openclaw_core::Result<()> {
        // TODO: signal-cli REST API integration
        Ok(())
    }
    fn status(&self) -> ChannelStatus { self.status.clone() }
}
