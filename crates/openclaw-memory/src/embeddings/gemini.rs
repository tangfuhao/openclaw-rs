use super::EmbeddingService;
use async_trait::async_trait;
use openclaw_core::EmbeddingProvider;
use tracing::debug;

pub struct GeminiEmbeddings {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl GeminiEmbeddings {
    pub fn new(api_key: &str, model: Option<&str>) -> Self {
        Self {
            api_key: api_key.to_string(),
            model: model.unwrap_or("text-embedding-004").to_string(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl EmbeddingService for GeminiEmbeddings {
    fn provider(&self) -> EmbeddingProvider { EmbeddingProvider::Gemini }
    fn dimensions(&self) -> usize { 768 }

    async fn embed(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>> {
        debug!(model = %self.model, count = texts.len(), "Gemini embedding request");

        let mut all_embeddings = Vec::new();
        for text in texts {
            let body = serde_json::json!({
                "model": format!("models/{}", self.model),
                "content": { "parts": [{ "text": text }] },
            });

            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:embedContent?key={}",
                self.model, self.api_key
            );

            let resp = self.client.post(&url).json(&body).send().await?;
            let data: serde_json::Value = resp.json().await?;

            let embedding: Vec<f32> = data["embedding"]["values"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|v| v.as_f64().map(|f| f as f32))
                .collect();

            all_embeddings.push(embedding);
        }

        Ok(all_embeddings)
    }
}
