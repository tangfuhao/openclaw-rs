use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
    uptime_seconds: i64,
    connections: usize,
}

async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    let uptime = chrono::Utc::now() - state.started_at();
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: uptime.num_seconds(),
        connections: state.connection_count(),
    })
}

async fn ready() -> &'static str {
    "ok"
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
        .route("/ready", get(ready))
}
