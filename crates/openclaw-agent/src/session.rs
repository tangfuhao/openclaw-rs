use chrono::{DateTime, Utc};
use openclaw_core::message::ConversationTurn;
use openclaw_core::types::SessionKey;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, info};

/// Metadata about a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEntry {
    pub key: SessionKey,
    pub agent_id: String,
    pub model: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub turn_count: usize,
    pub total_tokens: u64,
}

/// File-based session store with in-memory cache.
pub struct SessionStore {
    data_dir: PathBuf,
    sessions: RwLock<HashMap<SessionKey, SessionEntry>>,
    histories: RwLock<HashMap<SessionKey, Vec<ConversationTurn>>>,
}

impl SessionStore {
    pub fn new(data_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&data_dir).ok();
        Self {
            data_dir,
            sessions: RwLock::new(HashMap::new()),
            histories: RwLock::new(HashMap::new()),
        }
    }

    /// Get or create a session entry.
    pub fn get_or_create(&self, key: &SessionKey, agent_id: &str) -> SessionEntry {
        let mut sessions = self.sessions.write();
        sessions
            .entry(key.clone())
            .or_insert_with(|| SessionEntry {
                key: key.clone(),
                agent_id: agent_id.to_string(),
                model: None,
                created_at: Utc::now(),
                last_activity: Utc::now(),
                turn_count: 0,
                total_tokens: 0,
            })
            .clone()
    }

    /// List all sessions.
    pub fn list_sessions(&self) -> Vec<SessionEntry> {
        self.sessions.read().values().cloned().collect()
    }

    /// Get conversation history for a session.
    pub async fn get_history(
        &self,
        key: &SessionKey,
    ) -> anyhow::Result<Vec<ConversationTurn>> {
        // Try in-memory cache first
        {
            let histories = self.histories.read();
            if let Some(history) = histories.get(key) {
                return Ok(history.clone());
            }
        }

        // Try loading from disk
        let file_path = self.session_file_path(key);
        if file_path.exists() {
            let data = tokio::fs::read_to_string(&file_path).await?;
            let history: Vec<ConversationTurn> = serde_json::from_str(&data)?;
            let mut histories = self.histories.write();
            histories.insert(key.clone(), history.clone());
            return Ok(history);
        }

        Ok(Vec::new())
    }

    /// Append a conversation turn.
    pub async fn append_turn(
        &self,
        key: &SessionKey,
        turn: ConversationTurn,
    ) -> anyhow::Result<()> {
        {
            let mut histories = self.histories.write();
            let history = histories.entry(key.clone()).or_insert_with(Vec::new);
            history.push(turn);
        }

        // Update session metadata
        {
            let mut sessions = self.sessions.write();
            if let Some(session) = sessions.get_mut(key) {
                session.last_activity = Utc::now();
                session.turn_count += 1;
            }
        }

        // Persist to disk asynchronously
        self.persist(key).await?;

        Ok(())
    }

    /// Compact a session by summarizing old turns.
    pub async fn compact(&self, key: &SessionKey, keep_recent: usize) -> anyhow::Result<()> {
        let mut histories = self.histories.write();
        if let Some(history) = histories.get_mut(key) {
            if history.len() > keep_recent {
                let to_remove = history.len() - keep_recent;
                info!(session = %key, removed = to_remove, "Compacting session history");
                history.drain(..to_remove);
            }
        }
        drop(histories);
        self.persist(key).await
    }

    /// Delete a session.
    pub async fn delete(&self, key: &SessionKey) -> anyhow::Result<()> {
        self.sessions.write().remove(key);
        self.histories.write().remove(key);
        let file_path = self.session_file_path(key);
        if file_path.exists() {
            tokio::fs::remove_file(&file_path).await?;
        }
        Ok(())
    }

    /// Persist session history to disk.
    async fn persist(&self, key: &SessionKey) -> anyhow::Result<()> {
        let file_path = self.session_file_path(key);
        let history = {
            let histories = self.histories.read();
            histories.get(key).cloned().unwrap_or_default()
        };

        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let data = serde_json::to_string_pretty(&history)?;
        tokio::fs::write(&file_path, data).await?;
        debug!(session = %key, "Session persisted to disk");
        Ok(())
    }

    fn session_file_path(&self, key: &SessionKey) -> PathBuf {
        let safe_name = key.as_str().replace([':', '/', '\\'], "_");
        self.data_dir.join(format!("{safe_name}.json"))
    }
}
