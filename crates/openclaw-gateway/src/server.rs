use crate::auth::auth_middleware;
use crate::routes;
use crate::state::AppState;
use crate::ws::connection::handle_ws_connection;
use axum::{
    extract::{State, WebSocketUpgrade},
    middleware,
    response::IntoResponse,
    routing::get,
    Router,
};
use openclaw_config::ConfigManager;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{error, info};

/// Start the gateway HTTP/WebSocket server.
pub async fn start_gateway_server(config: ConfigManager) -> anyhow::Result<()> {
    let cfg = config.get();
    let port = cfg.gateway.port;
    let host = cfg.gateway.host.as_deref().unwrap_or("0.0.0.0");

    let state = AppState::new(config.clone());

    // Build the router
    let app = Router::new()
        // WebSocket upgrade endpoint
        .route("/ws", get(ws_upgrade_handler))
        // API routes
        .merge(routes::build_router(state.clone()))
        // Global layers
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state.clone());

    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    let listener = TcpListener::bind(addr).await?;

    info!("OpenClaw gateway listening on http://{addr}");
    info!("WebSocket endpoint: ws://{addr}/ws");
    info!("OpenAI-compatible API: http://{addr}/v1/chat/completions");

    // Start config file watcher
    if let Err(e) = openclaw_config::watcher::watch_config(config).await {
        error!("Failed to start config watcher: {e}");
    }

    // Graceful shutdown
    let mut shutdown_rx = state.shutdown_rx();
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.changed().await;
            info!("Gateway shutting down...");
        })
        .await?;

    Ok(())
}

/// Handle WebSocket upgrade requests.
async fn ws_upgrade_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws_connection(socket, state))
}
