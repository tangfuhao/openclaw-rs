use rusqlite::{params, Connection};
use std::path::Path;
use tracing::{debug, info, warn};

/// Initialize the SQLite database with required tables.
pub fn init_db(path: &Path) -> anyhow::Result<Connection> {
    let conn = Connection::open(path)?;

    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

    // Chunks table
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS chunks (
            id TEXT PRIMARY KEY,
            source TEXT NOT NULL,
            model TEXT NOT NULL,
            content TEXT NOT NULL,
            metadata TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_chunks_source ON chunks(source);
        CREATE INDEX IF NOT EXISTS idx_chunks_model ON chunks(model);",
    )?;

    // FTS5 virtual table for full-text search
    conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(
            content,
            content_rowid='rowid',
            tokenize='porter unicode61'
        );",
    )?;

    // Try to load sqlite-vec extension for vector search
    let vec_available = try_load_sqlite_vec(&conn);
    if vec_available {
        conn.execute_batch(
            "CREATE VIRTUAL TABLE IF NOT EXISTS chunks_vec USING vec0(
                id TEXT PRIMARY KEY,
                embedding FLOAT[1536]
            );",
        )?;
        info!("sqlite-vec extension loaded, vector search enabled");
    } else {
        warn!("sqlite-vec extension not available, falling back to in-memory cosine similarity");
    }

    info!("Memory database initialized at {}", path.display());
    Ok(conn)
}

fn try_load_sqlite_vec(_conn: &Connection) -> bool {
    // sqlite-vec extension loading requires the `load_extension` feature in rusqlite.
    // For now, we default to the in-memory cosine similarity fallback.
    // To enable: compile rusqlite with `load_extension` feature and provide the vec0 shared library.
    false
}

/// Insert a chunk into the database.
pub fn insert_chunk(
    conn: &Connection,
    id: &str,
    source: &str,
    model: &str,
    content: &str,
    metadata: Option<&str>,
) -> anyhow::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO chunks (id, source, model, content, metadata) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, source, model, content, metadata],
    )?;

    // Update FTS index
    conn.execute(
        "INSERT OR REPLACE INTO chunks_fts (rowid, content) VALUES ((SELECT rowid FROM chunks WHERE id = ?1), ?2)",
        params![id, content],
    )?;

    Ok(())
}

/// Insert a vector embedding for a chunk.
pub fn insert_vector(conn: &Connection, id: &str, embedding: &[f32]) -> anyhow::Result<()> {
    let blob = vector_to_blob(embedding);
    conn.execute(
        "INSERT OR REPLACE INTO chunks_vec (id, embedding) VALUES (?1, ?2)",
        params![id, blob],
    )?;
    Ok(())
}

/// Full-text search using FTS5.
pub fn fts_search(conn: &Connection, query: &str, limit: usize) -> anyhow::Result<Vec<(String, f64)>> {
    let mut stmt = conn.prepare(
        "SELECT c.id, rank FROM chunks_fts f
         JOIN chunks c ON c.rowid = f.rowid
         WHERE chunks_fts MATCH ?1
         ORDER BY rank
         LIMIT ?2",
    )?;

    let rows = stmt.query_map(params![query, limit as i64], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
    })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

/// Vector similarity search using sqlite-vec.
pub fn vector_search(
    conn: &Connection,
    query_vec: &[f32],
    model: &str,
    limit: usize,
) -> anyhow::Result<Vec<(String, f64)>> {
    let blob = vector_to_blob(query_vec);
    let mut stmt = conn.prepare(
        "SELECT v.id, vec_distance_cosine(v.embedding, ?1) AS dist
         FROM chunks_vec v
         JOIN chunks c ON c.id = v.id
         WHERE c.model = ?2
         ORDER BY dist ASC
         LIMIT ?3",
    )?;

    let rows = stmt.query_map(params![blob, model, limit as i64], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
    })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

/// Delete all chunks for a given source.
pub fn delete_source(conn: &Connection, source: &str) -> anyhow::Result<usize> {
    let count = conn.execute("DELETE FROM chunks WHERE source = ?1", params![source])?;
    Ok(count)
}

fn vector_to_blob(vec: &[f32]) -> Vec<u8> {
    vec.iter().flat_map(|f| f.to_le_bytes()).collect()
}
