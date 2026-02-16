pub mod gemini;
pub mod local;
pub mod openai;
pub mod voyage;

use async_trait::async_trait;
use openclaw_core::EmbeddingProvider;

/// Trait for embedding providers.
#[async_trait]
pub trait EmbeddingService: Send + Sync {
    fn provider(&self) -> EmbeddingProvider;
    fn dimensions(&self) -> usize;
    async fn embed(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>>;
    async fn embed_single(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let results = self.embed(&[text.to_string()]).await?;
        results.into_iter().next().ok_or_else(|| anyhow::anyhow!("No embedding returned"))
    }
}

/// Create an embedding service from provider configuration.
pub fn create_embedding_service(
    provider: &EmbeddingProvider,
    api_key: Option<&str>,
    model: Option<&str>,
) -> anyhow::Result<Box<dyn EmbeddingService>> {
    match provider {
        EmbeddingProvider::OpenAi => {
            let key = api_key.ok_or_else(|| anyhow::anyhow!("OpenAI API key required"))?;
            Ok(Box::new(openai::OpenAiEmbeddings::new(key, model)))
        }
        EmbeddingProvider::Gemini => {
            let key = api_key.ok_or_else(|| anyhow::anyhow!("Gemini API key required"))?;
            Ok(Box::new(gemini::GeminiEmbeddings::new(key, model)))
        }
        EmbeddingProvider::Voyage => {
            let key = api_key.ok_or_else(|| anyhow::anyhow!("Voyage API key required"))?;
            Ok(Box::new(voyage::VoyageEmbeddings::new(key, model)))
        }
        EmbeddingProvider::Local => {
            Ok(Box::new(local::LocalEmbeddings::new()))
        }
        EmbeddingProvider::Auto => {
            // Try providers in order: OpenAI > Gemini > Voyage > Local
            if let Some(key) = api_key {
                Ok(Box::new(openai::OpenAiEmbeddings::new(key, model)))
            } else {
                Ok(Box::new(local::LocalEmbeddings::new()))
            }
        }
    }
}
