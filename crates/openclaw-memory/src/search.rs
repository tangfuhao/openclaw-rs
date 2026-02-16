use serde::{Deserialize, Serialize};

/// A search query against the memory index.
#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub text: String,
    pub limit: usize,
    pub vector_weight: f32,
    pub text_weight: f32,
    pub source_filter: Option<String>,
}

/// A single search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub chunk_id: String,
    pub content: String,
    pub source: String,
    pub score: f64,
    pub metadata: Option<serde_json::Value>,
}

/// Merge vector and FTS results using weighted reciprocal rank fusion.
pub fn hybrid_merge(
    vector_results: &[(String, f64)],
    fts_results: &[(String, f64)],
    vector_weight: f32,
    text_weight: f32,
    limit: usize,
) -> Vec<(String, f64)> {
    use std::collections::HashMap;

    let k = 60.0; // RRF constant
    let mut scores: HashMap<String, f64> = HashMap::new();

    for (rank, (id, _dist)) in vector_results.iter().enumerate() {
        let rrf = vector_weight as f64 / (k + rank as f64 + 1.0);
        *scores.entry(id.clone()).or_default() += rrf;
    }

    for (rank, (id, _score)) in fts_results.iter().enumerate() {
        let rrf = text_weight as f64 / (k + rank as f64 + 1.0);
        *scores.entry(id.clone()).or_default() += rrf;
    }

    let mut ranked: Vec<(String, f64)> = scores.into_iter().collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    ranked.truncate(limit);
    ranked
}

/// Compute cosine similarity between two vectors (in-memory fallback).
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom > 0.0 { dot / denom } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);

        let c = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &c).abs() < 1e-6);
    }

    #[test]
    fn test_hybrid_merge() {
        let vec_results = vec![("a".into(), 0.1), ("b".into(), 0.2), ("c".into(), 0.3)];
        let fts_results = vec![("b".into(), -5.0), ("d".into(), -3.0), ("a".into(), -1.0)];

        let merged = hybrid_merge(&vec_results, &fts_results, 0.7, 0.3, 5);
        assert!(!merged.is_empty());
        // "a" and "b" should be top results since they appear in both
    }
}
