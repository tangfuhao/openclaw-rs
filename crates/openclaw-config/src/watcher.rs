use crate::loader::ConfigManager;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// Start watching the config file for changes and auto-reload.
pub async fn watch_config(config_manager: ConfigManager) -> anyhow::Result<()> {
    let config_path = config_manager.config_path().to_path_buf();

    let (tx, mut rx) = mpsc::channel::<()>(1);

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| match res {
            Ok(event) => {
                if event.kind.is_modify() || event.kind.is_create() {
                    let _ = tx.try_send(());
                }
            }
            Err(e) => {
                warn!("File watcher error: {e}");
            }
        },
        notify::Config::default(),
    )?;

    // Watch the parent directory of the config file
    if let Some(parent) = config_path.parent() {
        watcher.watch(parent, RecursiveMode::NonRecursive)?;
    }

    info!("Watching config file for changes: {}", config_path.display());

    // Debounce: only reload after 500ms of no further changes
    tokio::spawn(async move {
        let _watcher = watcher; // Keep watcher alive
        let mut debounce_handle: Option<tokio::task::JoinHandle<()>> = None;

        while rx.recv().await.is_some() {
            // Cancel previous debounce timer
            if let Some(handle) = debounce_handle.take() {
                handle.abort();
            }

            let cm = config_manager.clone();
            debounce_handle = Some(tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                match cm.reload() {
                    Ok(()) => info!("Config auto-reloaded successfully"),
                    Err(e) => error!("Failed to auto-reload config: {e}"),
                }
            }));
        }
    });

    Ok(())
}
