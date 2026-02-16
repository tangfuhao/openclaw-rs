pub mod browser;
pub mod cron;
pub mod image;
pub mod memory;
pub mod web_fetch;
pub mod web_search;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;

/// Metadata describing a tool available to agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Trait for implementing agent tools.
#[async_trait]
pub trait AgentTool: Send + Sync {
    fn definition(&self) -> ToolDefinition;
    async fn execute(&self, input: &serde_json::Value) -> anyhow::Result<String>;
}

/// Registry of available tools.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn AgentTool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Create a registry with default tools.
    pub fn with_defaults(http_client: reqwest::Client) -> Self {
        let mut registry = Self::new();
        registry.register(Arc::new(web_search::WebSearchTool::new(http_client.clone())));
        registry.register(Arc::new(web_fetch::WebFetchTool::new(http_client.clone())));
        registry.register(Arc::new(image::ImageGenerationTool::new(http_client.clone())));
        registry.register(Arc::new(cron::CronTool::new()));
        registry
    }

    pub fn register(&mut self, tool: Arc<dyn AgentTool>) {
        let name = tool.definition().name.clone();
        debug!(tool = %name, "Registering tool");
        self.tools.insert(name, tool);
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn AgentTool>> {
        self.tools.get(name)
    }

    pub fn list(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|t| t.definition()).collect()
    }

    /// Get tool definitions in OpenAI function-calling format.
    pub fn openai_tools_schema(&self) -> Vec<serde_json::Value> {
        self.tools
            .values()
            .map(|t| {
                let def = t.definition();
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": def.name,
                        "description": def.description,
                        "parameters": def.parameters,
                    }
                })
            })
            .collect()
    }

    pub async fn execute(&self, name: &str, input: &serde_json::Value) -> anyhow::Result<String> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Unknown tool: {name}"))?;

        debug!(tool = %name, "Executing tool");
        tool.execute(input).await
    }
}
