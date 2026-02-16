use openclaw_core::ConnectionId;
use serde_json::{json, Value};
use tracing::{debug, warn};

use crate::state::AppState;

/// Dispatch a WebSocket RPC method call to the appropriate handler.
pub async fn dispatch_method(
    state: &AppState,
    conn_id: &ConnectionId,
    method: &str,
    data: Option<Value>,
) -> Result<Value, String> {
    match method {
        "health.check" => handle_health(state).await,
        "health.ping" => Ok(json!({ "pong": true })),

        "config.get" => handle_config_get(state).await,

        "chat.send" => handle_chat_send(state, conn_id, data).await,
        "chat.abort" => handle_chat_abort(state, conn_id, data).await,
        "chat.history" => handle_chat_history(state, data).await,

        "sessions.list" => handle_sessions_list(state).await,
        "sessions.patch" => handle_sessions_patch(state, data).await,

        "models.list" => handle_models_list(state).await,

        "channels.list" => handle_channels_list(state).await,
        "channels.status" => handle_channels_status(state, data).await,

        "logs.tail" => handle_logs_tail(state, data).await,

        _ => {
            warn!(method = %method, "Unknown WS method");
            Err(format!("Unknown method: {method}"))
        }
    }
}

async fn handle_health(state: &AppState) -> Result<Value, String> {
    let uptime = chrono::Utc::now() - state.started_at();
    Ok(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_seconds": uptime.num_seconds(),
        "connections": state.connection_count(),
    }))
}

async fn handle_config_get(state: &AppState) -> Result<Value, String> {
    let config = state.config().get();
    serde_json::to_value(&*config).map_err(|e| e.to_string())
}

async fn handle_chat_send(
    state: &AppState,
    conn_id: &ConnectionId,
    data: Option<Value>,
) -> Result<Value, String> {
    let data = data.ok_or("Missing data for chat.send")?;
    let session_key = data["sessionKey"]
        .as_str()
        .ok_or("Missing sessionKey")?;
    let message = data["message"]
        .as_str()
        .ok_or("Missing message")?;

    debug!(session = %session_key, "Chat message received via WS");

    // TODO: Route to agent runner
    Ok(json!({
        "status": "queued",
        "sessionKey": session_key,
    }))
}

async fn handle_chat_abort(
    state: &AppState,
    conn_id: &ConnectionId,
    data: Option<Value>,
) -> Result<Value, String> {
    let data = data.ok_or("Missing data for chat.abort")?;
    let session_key = data["sessionKey"]
        .as_str()
        .ok_or("Missing sessionKey")?;

    // TODO: Abort running agent
    Ok(json!({ "status": "aborted", "sessionKey": session_key }))
}

async fn handle_chat_history(state: &AppState, data: Option<Value>) -> Result<Value, String> {
    let data = data.ok_or("Missing data for chat.history")?;
    let _session_key = data["sessionKey"]
        .as_str()
        .ok_or("Missing sessionKey")?;

    // TODO: Retrieve from session store
    Ok(json!({ "messages": [] }))
}

async fn handle_sessions_list(state: &AppState) -> Result<Value, String> {
    // TODO: Retrieve from session store
    Ok(json!({ "sessions": [] }))
}

async fn handle_sessions_patch(state: &AppState, data: Option<Value>) -> Result<Value, String> {
    let _data = data.ok_or("Missing data for sessions.patch")?;
    // TODO: Patch session
    Ok(json!({ "status": "ok" }))
}

async fn handle_models_list(state: &AppState) -> Result<Value, String> {
    let config = state.config().get();
    let mut models = Vec::new();
    for (provider, pc) in &config.models.providers {
        for model in &pc.models {
            models.push(json!({
                "id": format!("{provider}/{model}"),
                "provider": provider,
                "model": model,
            }));
        }
    }
    Ok(json!({ "models": models }))
}

async fn handle_channels_list(state: &AppState) -> Result<Value, String> {
    let config = state.config().get();
    let channels: Vec<Value> = config
        .channels
        .keys()
        .map(|id| json!({ "id": id, "status": "configured" }))
        .collect();
    Ok(json!({ "channels": channels }))
}

async fn handle_channels_status(state: &AppState, data: Option<Value>) -> Result<Value, String> {
    let data = data.ok_or("Missing data")?;
    let channel_id = data["channelId"].as_str().ok_or("Missing channelId")?;

    Ok(json!({
        "channelId": channel_id,
        "status": "configured",
    }))
}

async fn handle_logs_tail(state: &AppState, data: Option<Value>) -> Result<Value, String> {
    // TODO: Implement log tailing
    Ok(json!({ "logs": [] }))
}
