use openclaw_core::types::{AgentId, SessionKey};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

/// A registered subagent instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentEntry {
    pub id: String,
    pub parent_agent_id: AgentId,
    pub session_key: SessionKey,
    pub depth: u32,
    pub task: String,
    pub status: SubagentStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubagentStatus {
    Running,
    Completed,
    Failed,
    Aborted,
}

/// Registry for tracking active subagents.
pub struct SubagentRegistry {
    entries: RwLock<HashMap<String, SubagentEntry>>,
    max_depth: u32,
}

impl SubagentRegistry {
    pub fn new(max_depth: u32) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            max_depth,
        }
    }

    /// Register a new subagent, checking depth limit.
    pub fn register(
        &self,
        parent_agent_id: &AgentId,
        session_key: &SessionKey,
        depth: u32,
        task: &str,
    ) -> Result<String, String> {
        if depth >= self.max_depth {
            return Err(format!(
                "Subagent depth limit ({}) exceeded",
                self.max_depth
            ));
        }

        let id = Uuid::new_v4().to_string();
        let entry = SubagentEntry {
            id: id.clone(),
            parent_agent_id: parent_agent_id.clone(),
            session_key: session_key.clone(),
            depth,
            task: task.to_string(),
            status: SubagentStatus::Running,
            created_at: chrono::Utc::now(),
        };

        self.entries.write().insert(id.clone(), entry);
        info!(subagent_id = %id, depth, "Subagent registered");
        Ok(id)
    }

    /// Mark a subagent as completed.
    pub fn complete(&self, id: &str) {
        if let Some(entry) = self.entries.write().get_mut(id) {
            entry.status = SubagentStatus::Completed;
        }
    }

    /// Mark a subagent as failed.
    pub fn fail(&self, id: &str) {
        if let Some(entry) = self.entries.write().get_mut(id) {
            entry.status = SubagentStatus::Failed;
        }
    }

    /// Abort a subagent.
    pub fn abort(&self, id: &str) {
        if let Some(entry) = self.entries.write().get_mut(id) {
            entry.status = SubagentStatus::Aborted;
        }
    }

    /// List all active subagents.
    pub fn list_active(&self) -> Vec<SubagentEntry> {
        self.entries
            .read()
            .values()
            .filter(|e| e.status == SubagentStatus::Running)
            .cloned()
            .collect()
    }

    /// Clean up completed/failed/aborted entries older than the given duration.
    pub fn cleanup(&self, max_age: std::time::Duration) {
        let cutoff = chrono::Utc::now() - chrono::Duration::from_std(max_age).unwrap_or_default();
        self.entries.write().retain(|_, e| {
            e.status == SubagentStatus::Running || e.created_at > cutoff
        });
    }
}
