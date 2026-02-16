pub mod discord;
pub mod irc;
pub mod matrix;
pub mod signal;
pub mod slack;
pub mod telegram;
pub mod whatsapp;

use openclaw_core::channel::ChannelPlugin;

/// Registry of available channel implementations.
pub struct ChannelRegistry {
    channels: Vec<Box<dyn ChannelPlugin>>,
}

impl ChannelRegistry {
    pub fn new() -> Self {
        Self { channels: Vec::new() }
    }

    pub fn register(&mut self, channel: Box<dyn ChannelPlugin>) {
        tracing::info!(channel = %channel.info().id, "Registered channel");
        self.channels.push(channel);
    }

    pub fn get(&self, id: &str) -> Option<&dyn ChannelPlugin> {
        self.channels.iter().find(|c| c.info().id.as_str() == id).map(|c| c.as_ref())
    }

    pub fn list(&self) -> Vec<&dyn ChannelPlugin> {
        self.channels.iter().map(|c| c.as_ref()).collect()
    }
}
