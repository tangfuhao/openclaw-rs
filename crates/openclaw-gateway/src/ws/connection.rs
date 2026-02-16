use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use openclaw_core::{ConnectionId, Scope};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::auth;
use crate::state::AppState;
use crate::ws::methods::dispatch_method;
use crate::ws::protocol::WsMessage;

/// Represents a connected WebSocket client.
#[derive(Debug, Clone)]
pub struct WsClient {
    pub id: ConnectionId,
    pub scopes: Vec<Scope>,
    pub authenticated: bool,
    pub tx: mpsc::UnboundedSender<String>,
}

impl WsClient {
    pub fn send(&self, msg: &WsMessage) -> bool {
        match serde_json::to_string(msg) {
            Ok(json) => self.tx.send(json).is_ok(),
            Err(e) => {
                error!("Failed to serialize WS message: {e}");
                false
            }
        }
    }

    pub fn has_scope(&self, scope: &Scope) -> bool {
        self.scopes.iter().any(|s| s.implies(scope))
    }
}

/// Handle a new WebSocket connection.
pub async fn handle_ws_connection(socket: WebSocket, state: AppState) {
    let conn_id = ConnectionId::new();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    let client = WsClient {
        id: conn_id.clone(),
        scopes: Vec::new(),
        authenticated: false,
        tx: tx.clone(),
    };

    state.ws_clients().insert(conn_id.clone(), client.clone());
    info!(conn_id = %conn_id, "WebSocket client connected");

    let (mut ws_sink, mut ws_stream) = socket.split();

    // Send challenge
    let nonce = auth::generate_challenge_nonce();
    let challenge = WsMessage::connect_challenge(&nonce);
    if let Ok(json) = serde_json::to_string(&challenge) {
        let _ = ws_sink.send(Message::Text(json.into())).await;
    }

    // Outbound task: forward messages from mpsc to WebSocket
    let outbound = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sink.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Inbound: process messages from WebSocket
    let state_clone = state.clone();
    let conn_id_clone = conn_id.clone();
    while let Some(Ok(msg)) = ws_stream.next().await {
        match msg {
            Message::Text(text) => {
                let text_str: &str = &text;
                match serde_json::from_str::<WsMessage>(text_str) {
                    Ok(ws_msg) => {
                        handle_inbound_message(&state_clone, &conn_id_clone, ws_msg).await;
                    }
                    Err(e) => {
                        debug!("Invalid WS message: {e}");
                        let err = WsMessage::error_response(
                            "",
                            super::protocol::error_codes::INVALID_REQUEST,
                            "Invalid message format",
                        );
                        if let Ok(json) = serde_json::to_string(&err) {
                            let _ = tx.send(json);
                        }
                    }
                }
            }
            Message::Ping(_data) => {
                // Pong is handled automatically by axum's WS layer
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    // Cleanup
    outbound.abort();
    state.ws_clients().remove(&conn_id);
    info!(conn_id = %conn_id, "WebSocket client disconnected");
}

async fn handle_inbound_message(state: &AppState, conn_id: &ConnectionId, msg: WsMessage) {
    let Some(method) = &msg.method else {
        return;
    };
    let request_id = msg.id.as_deref().unwrap_or("");

    debug!(conn_id = %conn_id, method = %method, "WS method call");

    let response = dispatch_method(state, conn_id, method, msg.data.clone()).await;

    if let Some(client) = state.ws_clients().get(conn_id) {
        let reply = match response {
            Ok(data) => WsMessage::response(request_id, data),
            Err(e) => WsMessage::error_response(
                request_id,
                super::protocol::error_codes::INTERNAL_ERROR,
                &e,
            ),
        };
        client.send(&reply);
    }
}
