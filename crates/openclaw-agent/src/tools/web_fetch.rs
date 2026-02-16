use super::{AgentTool, ToolDefinition};
use async_trait::async_trait;
use serde_json::json;
use tracing::debug;

pub struct WebFetchTool {
    client: reqwest::Client,
}

impl WebFetchTool {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl AgentTool for WebFetchTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "web_fetch".to_string(),
            description: "Fetch the content of a web page and return it as readable text."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to fetch"
                    }
                },
                "required": ["url"]
            }),
        }
    }

    async fn execute(&self, input: &serde_json::Value) -> anyhow::Result<String> {
        let url = input["url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'url' parameter"))?;

        debug!(url = %url, "Fetching web page");

        let response = self
            .client
            .get(url)
            .header("User-Agent", "OpenClaw/1.0")
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("HTTP {status} for {url}");
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text/html")
            .to_string();

        let body = response.text().await?;

        // Basic HTML tag stripping for readability
        let text = if content_type.contains("html") {
            strip_html_tags(&body)
        } else {
            body
        };

        // Truncate very long content
        let max_len = 50_000;
        if text.len() > max_len {
            Ok(format!("{}...\n[Truncated at {max_len} chars]", &text[..max_len]))
        } else {
            Ok(text)
        }
    }
}

/// Simple HTML tag stripper.
fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;

    for ch in html.chars() {
        match ch {
            '<' => {
                in_tag = true;
            }
            '>' => {
                in_tag = false;
            }
            _ if !in_tag => {
                result.push(ch);
            }
            _ => {}
        }
    }

    // Collapse whitespace
    let mut collapsed = String::with_capacity(result.len());
    let mut prev_ws = false;
    for ch in result.chars() {
        if ch.is_whitespace() {
            if !prev_ws {
                collapsed.push(' ');
            }
            prev_ws = true;
        } else {
            collapsed.push(ch);
            prev_ws = false;
        }
    }

    collapsed.trim().to_string()
}
