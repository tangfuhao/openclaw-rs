use crate::chunker::chunk_text;
use crate::embeddings::{self, EmbeddingService};
use crate::search::{SearchQuery, SearchResult, hybrid_merge};
use crate::sqlite;
use openclaw_config::schema::MemoryConfig;
use parking_lot::Mutex;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, error, info};

/// The main memory index manager combining vector and FTS search.
pub struct MemoryIndexManager {
    db: Mutex<Connection>,
    embedding_service: Box<dyn EmbeddingService>,
    config: MemoryConfig,
    db_path: PathBuf,
}

impl MemoryIndexManager {
    /// Create a new memory index manager.
    pub fn new(
        data_dir: &Path,
        embedding_service: Box<dyn EmbeddingService>,
        config: MemoryConfig,
    ) -> anyhow::Result<Self> {
        let db_path = data_dir.join("memory.db");
        let conn = sqlite::init_db(&db_path)?;

        Ok(Self {
            db: Mutex::new(conn),
            embedding_service,
            config,
            db_path,
        })
    }

    /// Index a document (split into chunks, embed, and store).
    pub async fn index_document(
        &self,
        source: &str,
        content: &str,
    ) -> anyhow::Result<usize> {
        let chunks = chunk_text(content, self.config.chunk_size, self.config.chunk_overlap);
        if chunks.is_empty() {
            return Ok(0);
        }

        info!(source = %source, chunks = chunks.len(), "Indexing document");

        // Generate embeddings
        let embeddings = self.embedding_service.embed(&chunks).await?;
        let model = self.embedding_service.provider().to_string();

        // Store in database
        let db = self.db.lock();
        for (i, (chunk, embedding)) in chunks.iter().zip(embeddings.iter()).enumerate() {
            let chunk_id = format!("{source}:{i}");
            sqlite::insert_chunk(&db, &chunk_id, source, &model, chunk, None)?;
            sqlite::insert_vector(&db, &chunk_id, embedding)?;
        }

        Ok(chunks.len())
    }

    /// Search the memory index.
    pub async fn search(&self, query: SearchQuery) -> anyhow::Result<Vec<SearchResult>> {
        let db = self.db.lock();

        // Vector search
        let query_embedding = self.embedding_service.embed_single(&query.text).await?;
        let model = self.embedding_service.provider().to_string();
        let vector_results = sqlite::vector_search(&db, &query_embedding, &model, query.limit * 2)
            .unwrap_or_default();

        // FTS search
        let fts_results = sqlite::fts_search(&db, &query.text, query.limit * 2)
            .unwrap_or_default();

        // Hybrid merge
        let merged = hybrid_merge(
            &vector_results,
            &fts_results,
            query.vector_weight,
            query.text_weight,
            query.limit,
        );

        // Fetch full content for results
        let mut results = Vec::new();
        for (chunk_id, score) in &merged {
            let mut stmt = db.prepare(
                "SELECT content, source, metadata FROM chunks WHERE id = ?1"
            )?;

            if let Ok(row) = stmt.query_row(rusqlite::params![chunk_id], |row| {
                Ok(SearchResult {
                    chunk_id: chunk_id.clone(),
                    content: row.get(0)?,
                    source: row.get(1)?,
                    score: *score,
                    metadata: row.get::<_, Option<String>>(2)?
                        .and_then(|s| serde_json::from_str(&s).ok()),
                })
            }) {
                results.push(row);
            }
        }

        debug!(query = %query.text, results = results.len(), "Memory search completed");
        Ok(results)
    }

    /// Remove all indexed content for a source.
    pub fn remove_source(&self, source: &str) -> anyhow::Result<usize> {
        let db = self.db.lock();
        sqlite::delete_source(&db, source)
    }
}
