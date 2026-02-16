use crate::ws::connection::WsClient;
use dashmap::DashMap;
use openclaw_config::ConfigManager;
use openclaw_core::{ConnectionId, SessionKey};
use std::sync::Arc;
use tokio::sync::broadcast;

/// Shared application state passed through axum's state extractor.
#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

pub struct AppStateInner {
    /// Configuration manager with hot-reload support.
    pub config: ConfigManager,

    /// Active WebSocket clients, keyed by connection ID.
    pub ws_clients: DashMap<ConnectionId, WsClient>,

    /// Event broadcast channel for gateway-wide events.
    pub event_tx: broadcast::Sender<GatewayEvent>,

    /// Server start time.
    pub started_at: chrono::DateTime<chrono::Utc>,

    /// Shutdown signal.
    pub shutdown_tx: tokio::sync::watch::Sender<bool>,
    pub shutdown_rx: tokio::sync::watch::Receiver<bool>,
}

impl AppState {
    pub fn new(config: ConfigManager) -> Self {
        let (event_tx, _) = broadcast::channel(1024);
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

        Self {
            inner: Arc::new(AppStateInner {
                config,
                ws_clients: DashMap::new(),
                event_tx,
                started_at: chrono::Utc::now(),
                shutdown_tx,
                shutdown_rx,
            }),
        }
    }

    pub fn config(&self) -> &ConfigManager {
        &self.inner.config
    }

    pub fn ws_clients(&self) -> &DashMap<ConnectionId, WsClient> {
        &self.inner.ws_clients
    }

    pub fn event_tx(&self) -> &broadcast::Sender<GatewayEvent> {
        &self.inner.event_tx
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<GatewayEvent> {
        self.inner.event_tx.subscribe()
    }

    pub fn started_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.inner.started_at
    }

    pub fn connection_count(&self) -> usize {
        self.inner.ws_clients.len()
    }

    pub fn trigger_shutdown(&self) {
        let _ = self.inner.shutdown_tx.send(true);
    }

    pub fn shutdown_rx(&self) -> tokio::sync::watch::Receiver<bool> {
        self.inner.shutdown_rx.clone()
    }
}

/// Gateway-wide events broadcast to all interested listeners.
#[derive(Debug, Clone)]
pub enum GatewayEvent {
    ClientConnected(ConnectionId),
    ClientDisconnected(ConnectionId),
    ConfigReloaded,
    MessageReceived {
        session_key: SessionKey,
    },
    ReplySent {
        session_key: SessionKey,
    },
    Shutdown,
}
