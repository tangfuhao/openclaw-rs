use axum::{
    body::Body,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response, Sse},
    routing::post,
    Json, Router,
};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::pin::Pin;
use tracing::{debug, error, info};

use crate::state::AppState;

/// OpenAI-compatible chat completion request.
#[derive(Debug, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub stream: Option<bool>,
    #[serde(default)]
    pub top_p: Option<f32>,
    #[serde(default)]
    pub frequency_penalty: Option<f32>,
    #[serde(default)]
    pub presence_penalty: Option<f32>,
    #[serde(default)]
    pub stop: Option<Vec<String>>,
    #[serde(default)]
    pub tools: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub tool_choice: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// OpenAI-compatible chat completion response.
#[derive(Debug, Serialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: &'static str,
    pub created: i64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Debug, Serialize)]
pub struct Choice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// SSE streaming chunk.
#[derive(Debug, Serialize)]
pub struct ChatCompletionChunk {
    pub id: String,
    pub object: &'static str,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChunkChoice>,
}

#[derive(Debug, Serialize)]
pub struct ChunkChoice {
    pub index: u32,
    pub delta: ChunkDelta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChunkDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<serde_json::Value>>,
}

/// Handle POST /v1/chat/completions
async fn chat_completions(
    State(state): State<AppState>,
    Json(request): Json<ChatCompletionRequest>,
) -> Result<Response, StatusCode> {
    info!(model = %request.model, stream = ?request.stream, "Chat completion request");

    let is_stream = request.stream.unwrap_or(false);
    let request_id = format!("chatcmpl-{}", uuid::Uuid::new_v4());
    let created = chrono::Utc::now().timestamp();

    if is_stream {
        // Return SSE stream
        let stream = async_stream::stream! {
            // Initial chunk with role
            let chunk = ChatCompletionChunk {
                id: request_id.clone(),
                object: "chat.completion.chunk",
                created,
                model: request.model.clone(),
                choices: vec![ChunkChoice {
                    index: 0,
                    delta: ChunkDelta {
                        role: Some("assistant".to_string()),
                        content: None,
                        tool_calls: None,
                    },
                    finish_reason: None,
                }],
            };
            yield Ok::<_, Infallible>(axum::response::sse::Event::default()
                .data(serde_json::to_string(&chunk).unwrap()));

            // TODO: Integrate with agent runner for actual LLM responses
            let placeholder = "OpenClaw gateway is running. Agent integration pending.";
            for word in placeholder.split_whitespace() {
                let chunk = ChatCompletionChunk {
                    id: request_id.clone(),
                    object: "chat.completion.chunk",
                    created,
                    model: request.model.clone(),
                    choices: vec![ChunkChoice {
                        index: 0,
                        delta: ChunkDelta {
                            role: None,
                            content: Some(format!("{word} ")),
                            tool_calls: None,
                        },
                        finish_reason: None,
                    }],
                };
                yield Ok(axum::response::sse::Event::default()
                    .data(serde_json::to_string(&chunk).unwrap()));
            }

            // Final chunk
            let chunk = ChatCompletionChunk {
                id: request_id.clone(),
                object: "chat.completion.chunk",
                created,
                model: request.model.clone(),
                choices: vec![ChunkChoice {
                    index: 0,
                    delta: ChunkDelta {
                        role: None,
                        content: None,
                        tool_calls: None,
                    },
                    finish_reason: Some("stop".to_string()),
                }],
            };
            yield Ok(axum::response::sse::Event::default()
                .data(serde_json::to_string(&chunk).unwrap()));

            yield Ok(axum::response::sse::Event::default().data("[DONE]"));
        };

        Ok(Sse::new(stream).into_response())
    } else {
        // Non-streaming response
        let response_text =
            "OpenClaw gateway is running. Agent integration pending.".to_string();

        let response = ChatCompletionResponse {
            id: request_id,
            object: "chat.completion",
            created,
            model: request.model,
            choices: vec![Choice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: Some(serde_json::Value::String(response_text)),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Usage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
        };

        Ok(Json(response).into_response())
    }
}

/// List available models.
#[derive(Serialize)]
struct ModelsResponse {
    object: &'static str,
    data: Vec<ModelEntry>,
}

#[derive(Serialize)]
struct ModelEntry {
    id: String,
    object: &'static str,
    created: i64,
    owned_by: String,
}

async fn list_models(State(state): State<AppState>) -> Json<ModelsResponse> {
    let config = state.config().get();
    let mut models = Vec::new();
    let now = chrono::Utc::now().timestamp();

    for (provider_name, provider_config) in &config.models.providers {
        for model_name in &provider_config.models {
            models.push(ModelEntry {
                id: format!("{provider_name}/{model_name}"),
                object: "model",
                created: now,
                owned_by: provider_name.clone(),
            });
        }
    }

    Json(ModelsResponse {
        object: "list",
        data: models,
    })
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/models", axum::routing::get(list_models))
}
