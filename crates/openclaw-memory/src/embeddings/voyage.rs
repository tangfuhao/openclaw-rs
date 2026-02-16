use super::EmbeddingService;
use async_trait::async_trait;
use openclaw_core::EmbeddingProvider;
use tracing::debug;

pub struct VoyageEmbeddings {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl VoyageEmbeddings {
    pub fn new(api_key: &str, model: Option<&str>) -> Self {
        Self {
            api_key: api_key.to_string(),
            model: model.unwrap_or("voyage-3-lite").to_string(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl EmbeddingService for VoyageEmbeddings {
    fn provider(&self) -> EmbeddingProvider { EmbeddingProvider::Voyage }
    fn dimensions(&self) -> usize { 1024 }

    async fn embed(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>> {
        debug!(model = %self.model, count = texts.len(), "Voyage embedding request");

        let body = serde_json::json!({
            "model": self.model,
            "input": texts,
        });

        let resp = self.client
            .post("https://api.voyageai.com/v1/embeddings")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send().await?;

        let data: serde_json::Value = resp.json().await?;
        let embeddings = data["data"].as_array()
            .ok_or_else(|| anyhow::anyhow!("Invalid Voyage embeddings response"))?
            .iter()
            .map(|item| {
                item["embedding"].as_array()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                    .collect()
            })
            .collect();

        Ok(embeddings)
    }
}
