use async_trait::async_trait;
use openclaw_core::channel::ChannelPlugin;

/// Extended plugin service trait with lifecycle hooks.
#[async_trait]
pub trait PluginService: Send + Sync {
    /// Return the channel plugin implementation.
    fn channel(&self) -> &dyn ChannelPlugin;

    /// Called when the plugin is loaded.
    async fn on_load(&mut self) -> anyhow::Result<()> { Ok(()) }

    /// Called when the plugin is unloaded.
    async fn on_unload(&mut self) -> anyhow::Result<()> { Ok(()) }

    /// Called when the configuration changes.
    async fn on_config_change(&mut self, _config: serde_json::Value) -> anyhow::Result<()> { Ok(()) }

    /// Return custom HTTP routes for this plugin.
    fn http_routes(&self) -> Vec<super::types::PluginRoute> { Vec::new() }
}
