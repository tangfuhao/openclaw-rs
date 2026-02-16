use super::{AgentTool, ToolDefinition};
use async_trait::async_trait;
use serde_json::json;
use tracing::debug;

pub struct WebSearchTool {
    client: reqwest::Client,
}

impl WebSearchTool {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl AgentTool for WebSearchTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "web_search".to_string(),
            description: "Search the web for information using a search query.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query"
                    },
                    "num_results": {
                        "type": "integer",
                        "description": "Number of results to return (default: 5)",
                        "default": 5
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn execute(&self, input: &serde_json::Value) -> anyhow::Result<String> {
        let query = input["query"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'query' parameter"))?;

        debug!(query = %query, "Performing web search");

        // TODO: Integrate with actual search API (e.g., SearXNG, Tavily, Brave Search)
        Ok(format!(
            "Web search results for '{query}' - search provider integration pending."
        ))
    }
}
