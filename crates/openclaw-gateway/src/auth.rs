use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use openclaw_core::Scope;
use sha2::{Digest, Sha256};
use tracing::warn;

use crate::state::AppState;

/// Extract and validate the bearer token from an HTTP request.
pub fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

/// Verify a token against the configured auth token.
pub fn verify_token(token: &str, expected: &str) -> bool {
    // Constant-time comparison via hashing
    let token_hash = Sha256::digest(token.as_bytes());
    let expected_hash = Sha256::digest(expected.as_bytes());
    token_hash == expected_hash
}

/// Axum middleware for bearer token authentication.
pub async fn auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let config = state.config().get();

    // If no auth token configured, allow all requests
    let Some(expected_token) = &config.gateway.auth_token else {
        return Ok(next.run(request).await);
    };

    let Some(token) = extract_bearer_token(request.headers()) else {
        warn!("Missing authentication token");
        return Err(StatusCode::UNAUTHORIZED);
    };

    if !verify_token(&token, expected_token) {
        warn!("Invalid authentication token");
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(request).await)
}

/// Generate a random authentication challenge nonce.
pub fn generate_challenge_nonce() -> String {
    use rand::RngCore;
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 32];
    rng.fill_bytes(&mut bytes);
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_token() {
        assert!(verify_token("test-token", "test-token"));
        assert!(!verify_token("wrong-token", "test-token"));
    }
}
