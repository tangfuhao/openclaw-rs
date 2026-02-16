use async_trait::async_trait;
use openclaw_core::channel::*;
use openclaw_core::message::*;
use openclaw_core::types::*;
use serde::Deserialize;
use tracing::info;

#[derive(Debug, Deserialize)]
struct DiscordConfig {
    bot_token: String,
    application_id: String,
    #[serde(default)]
    allowed_servers: Vec<String>,
}

pub struct DiscordChannel {
    info: ChannelInfo,
    config: Option<DiscordConfig>,
    client: reqwest::Client,
    status: ChannelStatus,
}

impl DiscordChannel {
    pub fn new() -> Self {
        Self {
            info: ChannelInfo {
                id: ChannelId::new("discord"),
                name: "Discord".to_string(),
                description: "Discord Bot integration".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                supports_groups: true,
                supports_threads: true,
                supports_media: true,
                supports_reactions: true,
                supports_editing: true,
                supports_voice: false,
                max_message_length: Some(2000),
            },
            config: None,
            client: reqwest::Client::new(),
            status: ChannelStatus::Disconnected,
        }
    }
}

#[async_trait]
impl ChannelPlugin for DiscordChannel {
    fn info(&self) -> &ChannelInfo { &self.info }

    async fn initialize(&mut self, config: serde_json::Value) -> openclaw_core::Result<()> {
        self.config = Some(serde_json::from_value(config).map_err(|e| {
            openclaw_core::Error::Config(format!("Invalid Discord config: {e}"))
        })?);
        Ok(())
    }

    async fn start(&mut self, _sink: Box<dyn MessageSink>) -> openclaw_core::Result<()> {
        self.status = ChannelStatus::Connected;
        info!("Discord channel started");
        // TODO: Connect to Discord Gateway via WebSocket
        Ok(())
    }

    async fn stop(&mut self) -> openclaw_core::Result<()> {
        self.status = ChannelStatus::Disconnected;
        info!("Discord channel stopped");
        Ok(())
    }

    async fn send(&self, reply: OutboundReply) -> openclaw_core::Result<()> {
        let Some(config) = &self.config else {
            return Err(openclaw_core::Error::Channel {
                channel: "discord".into(), message: "Not configured".into(),
            });
        };

        let channel_id = reply.session_key.parts()
            .map(|(_, _, peer)| peer).unwrap_or("");

        if let Some(text) = &reply.text {
            let body = serde_json::json!({ "content": text });
            self.client
                .post(format!("https://discord.com/api/v10/channels/{channel_id}/messages"))
                .header("Authorization", format!("Bot {}", config.bot_token))
                .json(&body)
                .send().await
                .map_err(|e| openclaw_core::Error::Channel {
                    channel: "discord".into(), message: e.to_string(),
                })?;
        }

        Ok(())
    }

    fn status(&self) -> ChannelStatus { self.status.clone() }
}
