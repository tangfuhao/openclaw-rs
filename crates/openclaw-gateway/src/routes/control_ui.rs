use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use tower_http::services::ServeDir;

use crate::state::AppState;

/// Serve the control UI SPA.
/// In production, this serves pre-built static files.
/// For now, we serve a placeholder.
async fn control_ui_index() -> Html<&'static str> {
    Html(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>OpenClaw Control Panel</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #0a0a0a; color: #e0e0e0;
            display: flex; justify-content: center; align-items: center;
            min-height: 100vh;
        }
        .container { text-align: center; padding: 2rem; }
        h1 { font-size: 2.5rem; margin-bottom: 1rem; color: #fff; }
        .badge {
            display: inline-block; padding: 0.25rem 0.75rem;
            background: #22c55e; color: #000; border-radius: 9999px;
            font-size: 0.875rem; font-weight: 600; margin-bottom: 1.5rem;
        }
        p { color: #888; font-size: 1.1rem; line-height: 1.6; }
        code { background: #1a1a1a; padding: 0.15rem 0.5rem; border-radius: 4px; color: #60a5fa; }
    </style>
</head>
<body>
    <div class="container">
        <h1>&#9889; OpenClaw</h1>
        <span class="badge">Rust Edition</span>
        <p>Gateway is running.<br>Control UI will be available here.</p>
        <p style="margin-top: 1rem;">API endpoint: <code>/v1/chat/completions</code></p>
    </div>
</body>
</html>"#,
    )
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/", get(control_ui_index))
}
