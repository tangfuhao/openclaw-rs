use openclaw_core::types::SessionKey;

/// API interface provided to plugins by the gateway.
pub trait PluginApi: Send + Sync {
    /// Send a message to a session.
    fn send_message(&self, session_key: &SessionKey, text: &str) -> anyhow::Result<()>;

    /// Get the current configuration for this plugin.
    fn get_config(&self) -> serde_json::Value;

    /// Log a message.
    fn log(&self, level: &str, message: &str);

    /// Store a key-value pair in plugin storage.
    fn storage_set(&self, key: &str, value: &str) -> anyhow::Result<()>;

    /// Get a value from plugin storage.
    fn storage_get(&self, key: &str) -> anyhow::Result<Option<String>>;
}
