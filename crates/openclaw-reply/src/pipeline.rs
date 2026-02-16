use crate::directive::parse_directives;
use crate::dispatcher::ReplyDispatcher;
use crate::queue::MessageQueue;
use openclaw_agent::{AgentRunRequest, AgentRunner};
use openclaw_config::ConfigManager;
use openclaw_core::message::{InboundMessage, OutboundReply, StreamDelta};
use openclaw_core::types::{AgentId, ModelId, SessionKey};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// The main auto-reply processing pipeline.
pub struct ReplyPipeline {
    config: ConfigManager,
    agent_runner: Arc<AgentRunner>,
    message_queue: Arc<MessageQueue>,
}

impl ReplyPipeline {
    pub fn new(config: ConfigManager, agent_runner: Arc<AgentRunner>) -> Self {
        Self {
            config,
            agent_runner,
            message_queue: Arc::new(MessageQueue::new()),
        }
    }

    /// Process an inbound message through the full pipeline.
    pub async fn process(&self, message: InboundMessage) -> anyhow::Result<Option<OutboundReply>> {
        let session_key = message.session_key.clone();
        info!(session = %session_key, "Processing inbound message");

        // Step 1: Queue management (debounce, dedup)
        if !self.message_queue.enqueue(&session_key, &message).await {
            debug!(session = %session_key, "Message deduplicated or rate-limited");
            return Ok(None);
        }

        // Step 2: Resolve agent
        let config = self.config.get();
        let agent_id = self.resolve_agent(&message);
        let agent_config = config
            .agents
            .get(agent_id.as_str())
            .cloned()
            .unwrap_or_default();

        // Step 3: Parse inline directives
        let text = message.text_content().to_string();
        let (clean_text, directives) = parse_directives(&text);

        // Step 4: Resolve model
        let model = self.resolve_model(&directives, &agent_config, &config);

        // Step 5: Build agent run request
        let run_request = AgentRunRequest {
            session_key: session_key.clone(),
            agent_id: agent_id.clone(),
            model,
            message: clean_text,
            media: message.media.clone(),
            config: agent_config,
        };

        // Step 6: Run the agent with streaming
        let (delta_tx, mut delta_rx) = mpsc::unbounded_channel::<StreamDelta>();

        let runner = self.agent_runner.clone();
        let run_handle = tokio::spawn(async move { runner.run(run_request, delta_tx).await });

        // Step 7: Collect streaming deltas into final response
        let mut response_text = String::new();
        while let Some(delta) = delta_rx.recv().await {
            match &delta {
                StreamDelta::TextDelta { text } => {
                    response_text.push_str(text);
                }
                StreamDelta::MessageEnd { .. } => break,
                _ => {}
            }
        }

        // Wait for agent to finish
        match run_handle.await {
            Ok(Ok(result)) => {
                self.message_queue.dequeue(&session_key).await;

                Ok(Some(OutboundReply {
                    id: uuid::Uuid::new_v4().to_string(),
                    session_key,
                    channel_id: message.channel_id,
                    text: Some(result.response_text),
                    media: Vec::new(),
                    reply_to_message_id: Some(message.id),
                    thread_id: message.thread_id,
                    metadata: None,
                }))
            }
            Ok(Err(e)) => {
                error!(session = %session_key, error = %e, "Agent run failed");
                self.message_queue.dequeue(&session_key).await;
                Err(e)
            }
            Err(e) => {
                error!(session = %session_key, "Agent task panicked: {e}");
                self.message_queue.dequeue(&session_key).await;
                anyhow::bail!("Agent task panicked: {e}")
            }
        }
    }

    fn resolve_agent(&self, message: &InboundMessage) -> AgentId {
        // Default agent resolution: use "default" agent
        // TODO: Support per-channel agent mapping
        AgentId::default_agent()
    }

    fn resolve_model(
        &self,
        directives: &[crate::directive::Directive],
        agent_config: &openclaw_config::schema::AgentConfig,
        config: &openclaw_config::OpenClawConfig,
    ) -> ModelId {
        // Check directives for model override
        for d in directives {
            if let crate::directive::Directive::Model(model_str) = d {
                if let Some(model) = ModelId::parse(model_str) {
                    return model;
                }
                // Try alias
                if let Some(resolved) = config.models.aliases.get(model_str.as_str()) {
                    if let Some(model) = ModelId::parse(resolved) {
                        return model;
                    }
                }
            }
        }

        // Use agent config model
        if let Some(model_str) = &agent_config.model {
            if let Some(model) = ModelId::parse(model_str) {
                return model;
            }
        }

        // Use global default
        if let Some(model_str) = &config.models.default_model {
            if let Some(model) = ModelId::parse(model_str) {
                return model;
            }
        }

        // Fallback
        ModelId::new("openai", "gpt-4o")
    }
}

