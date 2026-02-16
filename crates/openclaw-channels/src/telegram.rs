use async_trait::async_trait;
use openclaw_core::channel::*;
use openclaw_core::message::*;
use openclaw_core::types::*;
use serde::Deserialize;
use tracing::{error, info};

#[derive(Debug, Deserialize)]
struct TelegramConfig {
    bot_token: String,
    #[serde(default)]
    allowed_users: Vec<String>,
    #[serde(default)]
    webhook_url: Option<String>,
}

pub struct TelegramChannel {
    info: ChannelInfo,
    config: Option<TelegramConfig>,
    client: reqwest::Client,
    status: ChannelStatus,
    sink: Option<Box<dyn MessageSink>>,
}

impl TelegramChannel {
    pub fn new() -> Self {
        Self {
            info: ChannelInfo {
                id: ChannelId::new("telegram"),
                name: "Telegram".to_string(),
                description: "Telegram Bot API integration".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                supports_groups: true,
                supports_threads: true,
                supports_media: true,
                supports_reactions: true,
                supports_editing: true,
                supports_voice: true,
                max_message_length: Some(4096),
            },
            config: None,
            client: reqwest::Client::new(),
            status: ChannelStatus::Disconnected,
            sink: None,
        }
    }

    fn api_url(&self, method: &str) -> String {
        let token = self.config.as_ref().map(|c| c.bot_token.as_str()).unwrap_or("");
        format!("https://api.telegram.org/bot{token}/{method}")
    }
}

#[async_trait]
impl ChannelPlugin for TelegramChannel {
    fn info(&self) -> &ChannelInfo { &self.info }

    async fn initialize(&mut self, config: serde_json::Value) -> openclaw_core::Result<()> {
        self.config = Some(serde_json::from_value(config).map_err(|e| {
            openclaw_core::Error::Config(format!("Invalid Telegram config: {e}"))
        })?);
        Ok(())
    }

    async fn start(&mut self, sink: Box<dyn MessageSink>) -> openclaw_core::Result<()> {
        self.sink = Some(sink);
        self.status = ChannelStatus::Connected;
        info!("Telegram channel started");
        // TODO: Start long-polling or set up webhook
        Ok(())
    }

    async fn stop(&mut self) -> openclaw_core::Result<()> {
        self.status = ChannelStatus::Disconnected;
        self.sink = None;
        info!("Telegram channel stopped");
        Ok(())
    }

    async fn send(&self, reply: OutboundReply) -> openclaw_core::Result<()> {
        let Some(_config) = &self.config else {
            return Err(openclaw_core::Error::Channel {
                channel: "telegram".into(),
                message: "Not configured".into(),
            });
        };

        let chat_id = reply.session_key.parts()
            .map(|(_, _, peer)| peer)
            .unwrap_or("");

        if let Some(text) = &reply.text {
            let body = serde_json::json!({
                "chat_id": chat_id,
                "text": text,
                "parse_mode": "Markdown",
            });

            let resp = self.client
                .post(&self.api_url("sendMessage"))
                .json(&body)
                .send().await
                .map_err(|e| openclaw_core::Error::Channel {
                    channel: "telegram".into(),
                    message: e.to_string(),
                })?;

            if !resp.status().is_success() {
                let err = resp.text().await.unwrap_or_default();
                error!(error = %err, "Telegram sendMessage failed");
            }
        }

        Ok(())
    }

    fn status(&self) -> ChannelStatus { self.status.clone() }

    async fn handle_webhook(
        &self, _path: &str, _headers: &[(String, String)], body: bytes::Bytes,
    ) -> openclaw_core::Result<(u16, String)> {
        let update: serde_json::Value = serde_json::from_slice(&body)
            .map_err(|e| openclaw_core::Error::Channel {
                channel: "telegram".into(),
                message: format!("Invalid webhook body: {e}"),
            })?;

        if let Some(message) = update.get("message") {
            let chat_id = message["chat"]["id"].to_string();
            let from_id = message["from"]["id"].to_string();
            let from_name = message["from"]["first_name"].as_str().unwrap_or("Unknown");
            let text = message["text"].as_str().unwrap_or("");
            let is_group = message["chat"]["type"].as_str() != Some("private");

            let inbound = InboundMessage {
                id: message["message_id"].to_string(),
                session_key: SessionKey::new("telegram", &from_id, &chat_id),
                channel_id: ChannelId::new("telegram"),
                sender_id: from_id,
                sender_name: Some(from_name.to_string()),
                text: Some(text.to_string()),
                media: Vec::new(),
                reply_to_message_id: message["reply_to_message"]["message_id"]
                    .as_i64().map(|id| id.to_string()),
                thread_id: None,
                is_group,
                group_id: if is_group { Some(chat_id.clone()) } else { None },
                group_name: message["chat"]["title"].as_str().map(String::from),
                timestamp: chrono::Utc::now(),
                raw: Some(update.clone()),
            };

            if let Some(sink) = &self.sink {
                let _ = sink.on_message(inbound).await;
            }
        }

        Ok((200, "ok".to_string()))
    }
}
