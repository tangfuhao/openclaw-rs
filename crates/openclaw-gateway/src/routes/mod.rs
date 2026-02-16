pub mod control_ui;
pub mod health;
pub mod hooks;
pub mod openai_compat;
pub mod plugin_api;

use axum::Router;
use crate::state::AppState;

/// Build the main HTTP router with all routes (returns Router<AppState>).
pub fn build_router(_state: AppState) -> Router<AppState> {
    Router::new()
        .merge(health::routes())
        .merge(openai_compat::routes())
        .merge(hooks::routes())
        .merge(plugin_api::routes())
        .merge(control_ui::routes())
}
