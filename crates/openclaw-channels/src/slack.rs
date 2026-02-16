use async_trait::async_trait;
use openclaw_core::channel::*;
use openclaw_core::message::*;
use openclaw_core::types::*;
use serde::Deserialize;
use tracing::info;

#[derive(Debug, Deserialize)]
struct SlackConfig {
    bot_token: String,
    app_token: Option<String>,
    signing_secret: String,
}

pub struct SlackChannel {
    info: ChannelInfo,
    config: Option<SlackConfig>,
    client: reqwest::Client,
    status: ChannelStatus,
}

impl SlackChannel {
    pub fn new() -> Self {
        Self {
            info: ChannelInfo {
                id: ChannelId::new("slack"),
                name: "Slack".to_string(),
                description: "Slack Bot integration via Events API".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                supports_groups: true,
                supports_threads: true,
                supports_media: true,
                supports_reactions: true,
                supports_editing: true,
                supports_voice: false,
                max_message_length: Some(40000),
            },
            config: None,
            client: reqwest::Client::new(),
            status: ChannelStatus::Disconnected,
        }
    }
}

#[async_trait]
impl ChannelPlugin for SlackChannel {
    fn info(&self) -> &ChannelInfo { &self.info }

    async fn initialize(&mut self, config: serde_json::Value) -> openclaw_core::Result<()> {
        self.config = Some(serde_json::from_value(config).map_err(|e| {
            openclaw_core::Error::Config(format!("Invalid Slack config: {e}"))
        })?);
        Ok(())
    }

    async fn start(&mut self, _sink: Box<dyn MessageSink>) -> openclaw_core::Result<()> {
        self.status = ChannelStatus::Connected;
        info!("Slack channel started");
        Ok(())
    }

    async fn stop(&mut self) -> openclaw_core::Result<()> {
        self.status = ChannelStatus::Disconnected;
        Ok(())
    }

    async fn send(&self, reply: OutboundReply) -> openclaw_core::Result<()> {
        let Some(config) = &self.config else {
            return Err(openclaw_core::Error::Channel {
                channel: "slack".into(), message: "Not configured".into(),
            });
        };

        let channel = reply.session_key.parts()
            .map(|(_, _, peer)| peer).unwrap_or("");

        if let Some(text) = &reply.text {
            let mut body = serde_json::json!({
                "channel": channel,
                "text": text,
            });
            if let Some(thread_ts) = &reply.thread_id {
                body["thread_ts"] = serde_json::Value::String(thread_ts.clone());
            }

            self.client
                .post("https://slack.com/api/chat.postMessage")
                .bearer_auth(&config.bot_token)
                .json(&body)
                .send().await
                .map_err(|e| openclaw_core::Error::Channel {
                    channel: "slack".into(), message: e.to_string(),
                })?;
        }

        Ok(())
    }

    fn status(&self) -> ChannelStatus { self.status.clone() }

    async fn handle_webhook(
        &self, _path: &str, _headers: &[(String, String)], body: bytes::Bytes,
    ) -> openclaw_core::Result<(u16, String)> {
        let payload: serde_json::Value = serde_json::from_slice(&body)
            .map_err(|e| openclaw_core::Error::Channel {
                channel: "slack".into(), message: format!("Invalid body: {e}"),
            })?;

        // Handle URL verification challenge
        if payload["type"].as_str() == Some("url_verification") {
            let challenge = payload["challenge"].as_str().unwrap_or("");
            return Ok((200, challenge.to_string()));
        }

        Ok((200, "ok".to_string()))
    }
}
