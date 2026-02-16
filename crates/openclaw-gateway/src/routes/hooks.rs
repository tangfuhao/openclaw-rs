use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::post,
    Json, Router,
};
use serde::Serialize;
use tracing::{info, warn};

use crate::state::AppState;

#[derive(Serialize)]
struct HookResponse {
    status: &'static str,
    message: String,
}

/// Handle POST /hooks/:hook_name
async fn handle_hook(
    State(state): State<AppState>,
    Path(hook_name): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<HookResponse>, StatusCode> {
    info!(hook = %hook_name, body_size = body.len(), "Incoming webhook");

    let config = state.config().get();

    // Find the matching hook configuration
    let hook_config = config
        .hooks
        .iter()
        .find(|h| h.name == hook_name);

    let Some(hook_config) = hook_config else {
        warn!(hook = %hook_name, "Unknown hook");
        return Err(StatusCode::NOT_FOUND);
    };

    // Verify hook token if configured
    if let Some(expected_token) = &hook_config.token {
        let provided_token = headers
            .get("x-hook-token")
            .or_else(|| headers.get("authorization"))
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        let provided_token = provided_token
            .strip_prefix("Bearer ")
            .unwrap_or(provided_token);

        if provided_token != expected_token {
            warn!(hook = %hook_name, "Invalid hook token");
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    // TODO: Dispatch hook event to agent system
    info!(hook = %hook_name, "Hook processed successfully");

    Ok(Json(HookResponse {
        status: "ok",
        message: format!("Hook '{hook_name}' processed"),
    }))
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/hooks/{hook_name}", post(handle_hook))
}
