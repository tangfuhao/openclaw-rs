use super::EmbeddingService;
use async_trait::async_trait;
use openclaw_core::EmbeddingProvider;
use tracing::debug;

/// Local embedding fallback using simple TF-IDF-like hashing.
/// In production, this would integrate with llama.cpp or ONNX Runtime for GGUF models.
pub struct LocalEmbeddings {
    dimensions: usize,
}

impl LocalEmbeddings {
    pub fn new() -> Self {
        Self { dimensions: 384 }
    }
}

#[async_trait]
impl EmbeddingService for LocalEmbeddings {
    fn provider(&self) -> EmbeddingProvider { EmbeddingProvider::Local }
    fn dimensions(&self) -> usize { self.dimensions }

    async fn embed(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>> {
        debug!(count = texts.len(), "Local embedding (hash-based fallback)");

        // Simple hash-based embedding for development/fallback.
        // Production should use llama.cpp bindings with a GGUF model.
        Ok(texts.iter().map(|text| hash_embed(text, self.dimensions)).collect())
    }
}

/// Create a deterministic pseudo-embedding from text using hashing.
/// This is NOT a real semantic embedding — it's a fallback for when no API is available.
fn hash_embed(text: &str, dims: usize) -> Vec<f32> {
    let mut vec = vec![0.0f32; dims];
    let words: Vec<&str> = text.split_whitespace().collect();

    for (i, word) in words.iter().enumerate() {
        let hash = simple_hash(word);
        let idx = (hash as usize) % dims;
        vec[idx] += 1.0;

        // Bigram features
        if i + 1 < words.len() {
            let bigram = format!("{} {}", word, words[i + 1]);
            let bh = simple_hash(&bigram);
            let bidx = (bh as usize) % dims;
            vec[bidx] += 0.5;
        }
    }

    // L2 normalize
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in &mut vec {
            *v /= norm;
        }
    }

    vec
}

fn simple_hash(s: &str) -> u64 {
    let mut hash: u64 = 5381;
    for b in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(b as u64);
    }
    hash
}
