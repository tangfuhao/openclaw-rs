use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use tracing::{debug, info};

use crate::state::AppState;

#[derive(Serialize)]
struct ChannelListResponse {
    channels: Vec<ChannelStatus>,
}

#[derive(Serialize)]
struct ChannelStatus {
    id: String,
    name: String,
    status: String,
}

/// GET /api/channels - list configured channels
async fn list_channels(State(state): State<AppState>) -> Json<ChannelListResponse> {
    let config = state.config().get();

    let channels: Vec<ChannelStatus> = config
        .channels
        .keys()
        .map(|id| ChannelStatus {
            id: id.clone(),
            name: id.clone(),
            status: "configured".to_string(),
        })
        .collect();

    Json(ChannelListResponse { channels })
}

/// POST /api/channels/:channel_id/webhook - channel-specific webhook
async fn channel_webhook(
    State(state): State<AppState>,
    Path(channel_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<impl IntoResponse, StatusCode> {
    info!(channel = %channel_id, "Channel webhook received");

    // TODO: Route to the appropriate channel plugin
    Ok(Json(serde_json::json!({
        "status": "ok",
        "channel": channel_id,
    })))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/channels", get(list_channels))
        .route("/api/channels/{channel_id}/webhook", post(channel_webhook))
}
