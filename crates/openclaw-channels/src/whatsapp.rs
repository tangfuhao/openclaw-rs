use async_trait::async_trait;
use openclaw_core::channel::*;
use openclaw_core::message::*;
use openclaw_core::types::*;
use serde::Deserialize;
use tracing::info;

#[derive(Debug, Deserialize)]
struct WhatsAppConfig {
    phone_number_id: String,
    access_token: String,
    verify_token: String,
    #[serde(default)]
    business_account_id: Option<String>,
}

pub struct WhatsAppChannel {
    info: ChannelInfo,
    config: Option<WhatsAppConfig>,
    client: reqwest::Client,
    status: ChannelStatus,
}

impl WhatsAppChannel {
    pub fn new() -> Self {
        Self {
            info: ChannelInfo {
                id: ChannelId::new("whatsapp"),
                name: "WhatsApp".to_string(),
                description: "WhatsApp Business Cloud API integration".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                supports_groups: true,
                supports_threads: false,
                supports_media: true,
                supports_reactions: true,
                supports_editing: false,
                supports_voice: true,
                max_message_length: Some(4096),
            },
            config: None,
            client: reqwest::Client::new(),
            status: ChannelStatus::Disconnected,
        }
    }
}

#[async_trait]
impl ChannelPlugin for WhatsAppChannel {
    fn info(&self) -> &ChannelInfo { &self.info }

    async fn initialize(&mut self, config: serde_json::Value) -> openclaw_core::Result<()> {
        self.config = Some(serde_json::from_value(config).map_err(|e| {
            openclaw_core::Error::Config(format!("Invalid WhatsApp config: {e}"))
        })?);
        Ok(())
    }

    async fn start(&mut self, _sink: Box<dyn MessageSink>) -> openclaw_core::Result<()> {
        self.status = ChannelStatus::Connected;
        info!("WhatsApp channel started");
        Ok(())
    }

    async fn stop(&mut self) -> openclaw_core::Result<()> {
        self.status = ChannelStatus::Disconnected;
        Ok(())
    }

    async fn send(&self, reply: OutboundReply) -> openclaw_core::Result<()> {
        let Some(config) = &self.config else {
            return Err(openclaw_core::Error::Channel {
                channel: "whatsapp".into(), message: "Not configured".into(),
            });
        };

        let to = reply.session_key.parts()
            .map(|(_, _, peer)| peer).unwrap_or("");

        if let Some(text) = &reply.text {
            let body = serde_json::json!({
                "messaging_product": "whatsapp",
                "to": to,
                "type": "text",
                "text": { "body": text },
            });

            let url = format!(
                "https://graph.facebook.com/v18.0/{}/messages",
                config.phone_number_id
            );

            self.client
                .post(&url)
                .bearer_auth(&config.access_token)
                .json(&body)
                .send().await
                .map_err(|e| openclaw_core::Error::Channel {
                    channel: "whatsapp".into(), message: e.to_string(),
                })?;
        }

        Ok(())
    }

    fn status(&self) -> ChannelStatus { self.status.clone() }
}
