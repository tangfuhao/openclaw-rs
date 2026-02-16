use serde::{Deserialize, Serialize};

/// A custom HTTP route registered by a plugin.
#[derive(Debug, Clone)]
pub struct PluginRoute {
    pub method: HttpMethod,
    pub path: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

/// Plugin metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub homepage: Option<String>,
}

/// Plugin health check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHealth {
    pub healthy: bool,
    pub message: Option<String>,
    pub details: Option<serde_json::Value>,
}
