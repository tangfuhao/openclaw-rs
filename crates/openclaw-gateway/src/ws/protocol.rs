use serde::{Deserialize, Serialize};

/// A JSON-RPC-like message for the WebSocket protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessage {
    /// Method name (e.g., "chat.send", "health.check").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,

    /// Request ID for request-response correlation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Payload data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,

    /// Error message (for responses).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<WsError>,

    /// Event name (for server-pushed events).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsError {
    pub code: i32,
    pub message: String,
}

impl WsMessage {
    /// Create a request message.
    pub fn request(method: &str, id: &str, data: serde_json::Value) -> Self {
        Self {
            method: Some(method.to_string()),
            id: Some(id.to_string()),
            data: Some(data),
            error: None,
            event: None,
        }
    }

    /// Create a success response.
    pub fn response(id: &str, data: serde_json::Value) -> Self {
        Self {
            method: None,
            id: Some(id.to_string()),
            data: Some(data),
            error: None,
            event: None,
        }
    }

    /// Create an error response.
    pub fn error_response(id: &str, code: i32, message: &str) -> Self {
        Self {
            method: None,
            id: Some(id.to_string()),
            data: None,
            error: Some(WsError {
                code,
                message: message.to_string(),
            }),
            event: None,
        }
    }

    /// Create a server-pushed event.
    pub fn event(event_name: &str, data: serde_json::Value) -> Self {
        Self {
            method: None,
            id: None,
            data: Some(data),
            error: None,
            event: Some(event_name.to_string()),
        }
    }

    /// Challenge event sent on new connections.
    pub fn connect_challenge(nonce: &str) -> Self {
        Self::event(
            "connect.challenge",
            serde_json::json!({ "nonce": nonce }),
        )
    }

    /// Connected event sent after successful authentication.
    pub fn connected() -> Self {
        Self::event(
            "connect.success",
            serde_json::json!({ "status": "connected" }),
        )
    }
}

/// Error codes for the WebSocket protocol.
pub mod error_codes {
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    pub const AUTH_REQUIRED: i32 = -32000;
    pub const AUTH_FAILED: i32 = -32001;
    pub const RATE_LIMITED: i32 = -32002;
    pub const PERMISSION_DENIED: i32 = -32003;
}
