use crate::prompt::build_system_prompt;
use crate::session::SessionStore;
use crate::tools::ToolRegistry;
use openclaw_config::schema::AgentConfig;
use openclaw_core::message::{
    ContentBlock, ConversationTurn, Role, StreamDelta, TokenUsage,
};
use openclaw_core::types::{AgentId, ModelId, SessionKey};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info};
use uuid::Uuid;

/// Request to run an agent.
#[derive(Debug, Clone)]
pub struct AgentRunRequest {
    pub session_key: SessionKey,
    pub agent_id: AgentId,
    pub model: ModelId,
    pub message: String,
    pub media: Vec<openclaw_core::message::MediaAttachment>,
    pub config: AgentConfig,
}

/// Result of an agent run.
#[derive(Debug, Clone, Serialize)]
pub struct AgentRunResult {
    pub session_key: SessionKey,
    pub agent_id: AgentId,
    pub response_text: String,
    pub tool_calls: Vec<ToolCallResult>,
    pub usage: TokenUsage,
    pub model: ModelId,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolCallResult {
    pub tool_name: String,
    pub input: serde_json::Value,
    pub output: String,
    pub is_error: bool,
    pub duration_ms: u64,
}

/// The main agent execution engine.
pub struct AgentRunner {
    session_store: Arc<SessionStore>,
    tool_registry: Arc<ToolRegistry>,
    http_client: reqwest::Client,
}

impl AgentRunner {
    pub fn new(session_store: Arc<SessionStore>, tool_registry: Arc<ToolRegistry>) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .expect("HTTP client creation should not fail");

        Self {
            session_store,
            tool_registry,
            http_client,
        }
    }

    /// Run an agent with the given request, streaming deltas to the channel.
    pub async fn run(
        &self,
        request: AgentRunRequest,
        delta_tx: mpsc::UnboundedSender<StreamDelta>,
    ) -> anyhow::Result<AgentRunResult> {
        let start = std::time::Instant::now();
        info!(
            session = %request.session_key,
            agent = %request.agent_id,
            model = %request.model,
            "Starting agent run"
        );

        // Build system prompt
        let system_prompt = build_system_prompt(&request.config, &request.agent_id);

        // Get conversation history
        let history = self
            .session_store
            .get_history(&request.session_key)
            .await?;

        // Build messages array for LLM
        let mut messages = Vec::new();

        // System message
        messages.push(LlmMessage {
            role: "system".to_string(),
            content: system_prompt,
        });

        // History
        for turn in &history {
            let content = turn
                .content
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::Text { text } => Some(text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");

            messages.push(LlmMessage {
                role: match turn.role {
                    Role::User => "user".to_string(),
                    Role::Assistant => "assistant".to_string(),
                    Role::System => "system".to_string(),
                    Role::Tool => "tool".to_string(),
                },
                content,
            });
        }

        // Current user message
        messages.push(LlmMessage {
            role: "user".to_string(),
            content: request.message.clone(),
        });

        // Call LLM (agentic loop with tool execution)
        let mut total_usage = TokenUsage::default();
        let mut tool_results = Vec::new();
        let mut final_response = String::new();
        let max_iterations = 10;

        for iteration in 0..max_iterations {
            debug!(iteration, "Agent loop iteration");

            let llm_result = self
                .call_llm(&request.model, &messages, &request.config, &delta_tx)
                .await?;

            total_usage.prompt_tokens += llm_result.usage.prompt_tokens;
            total_usage.completion_tokens += llm_result.usage.completion_tokens;
            total_usage.total_tokens += llm_result.usage.total_tokens;

            if llm_result.tool_calls.is_empty() {
                // No tool calls, this is the final response
                final_response = llm_result.text;
                break;
            }

            // Execute tool calls
            messages.push(LlmMessage {
                role: "assistant".to_string(),
                content: llm_result.text.clone(),
            });

            for tool_call in &llm_result.tool_calls {
                let tool_start = std::time::Instant::now();
                let tool_output = self
                    .tool_registry
                    .execute(&tool_call.name, &tool_call.input)
                    .await;

                let (output, is_error) = match tool_output {
                    Ok(output) => (output, false),
                    Err(e) => (format!("Error: {e}"), true),
                };

                tool_results.push(ToolCallResult {
                    tool_name: tool_call.name.clone(),
                    input: tool_call.input.clone(),
                    output: output.clone(),
                    is_error,
                    duration_ms: tool_start.elapsed().as_millis() as u64,
                });

                messages.push(LlmMessage {
                    role: "tool".to_string(),
                    content: output,
                });

                let _ = delta_tx.send(StreamDelta::ToolUseEnd);
            }

            if iteration == max_iterations - 1 {
                final_response = llm_result.text;
            }
        }

        // Save to session history
        self.session_store
            .append_turn(
                &request.session_key,
                ConversationTurn {
                    id: Uuid::new_v4(),
                    role: Role::User,
                    content: vec![ContentBlock::Text {
                        text: request.message.clone(),
                    }],
                    model: None,
                    agent_id: Some(request.agent_id.clone()),
                    timestamp: Utc::now(),
                    token_usage: None,
                },
            )
            .await?;

        self.session_store
            .append_turn(
                &request.session_key,
                ConversationTurn {
                    id: Uuid::new_v4(),
                    role: Role::Assistant,
                    content: vec![ContentBlock::Text {
                        text: final_response.clone(),
                    }],
                    model: Some(request.model.clone()),
                    agent_id: Some(request.agent_id.clone()),
                    timestamp: Utc::now(),
                    token_usage: Some(total_usage.clone()),
                },
            )
            .await?;

        let duration_ms = start.elapsed().as_millis() as u64;
        info!(
            session = %request.session_key,
            duration_ms,
            tokens = total_usage.total_tokens,
            "Agent run completed"
        );

        let _ = delta_tx.send(StreamDelta::MessageEnd {
            usage: Some(total_usage.clone()),
        });

        Ok(AgentRunResult {
            session_key: request.session_key,
            agent_id: request.agent_id,
            response_text: final_response,
            tool_calls: tool_results,
            usage: total_usage,
            model: request.model,
            duration_ms,
        })
    }

    /// Call the LLM API (supports OpenAI-compatible endpoints).
    async fn call_llm(
        &self,
        model: &ModelId,
        messages: &[LlmMessage],
        config: &AgentConfig,
        _delta_tx: &mpsc::UnboundedSender<StreamDelta>,
    ) -> anyhow::Result<LlmResponse> {
        // TODO: Route to correct provider based on model.provider
        // For now, use a generic OpenAI-compatible call
        let base_url = "https://api.openai.com/v1";

        let body = serde_json::json!({
            "model": model.model,
            "messages": messages,
            "temperature": config.temperature.unwrap_or(0.7),
            "max_tokens": config.max_tokens.unwrap_or(4096),
            "stream": false,
        });

        let response = self
            .http_client
            .post(format!("{base_url}/chat/completions"))
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!("LLM API error ({status}): {error_body}");
        }

        let resp: serde_json::Value = response.json().await?;

        let text = resp["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let usage = TokenUsage {
            prompt_tokens: resp["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: resp["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: resp["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32,
            ..Default::default()
        };

        // Parse tool calls if present
        let tool_calls = resp["choices"][0]["message"]["tool_calls"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|tc| {
                        Some(LlmToolCall {
                            name: tc["function"]["name"].as_str()?.to_string(),
                            input: serde_json::from_str(
                                tc["function"]["arguments"].as_str().unwrap_or("{}"),
                            )
                            .unwrap_or_default(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(LlmResponse {
            text,
            tool_calls,
            usage,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct LlmMessage {
    role: String,
    content: String,
}

struct LlmResponse {
    text: String,
    tool_calls: Vec<LlmToolCall>,
    usage: TokenUsage,
}

struct LlmToolCall {
    name: String,
    input: serde_json::Value,
}
