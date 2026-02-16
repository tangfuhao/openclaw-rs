use dashmap::DashMap;
use openclaw_core::message::InboundMessage;
use openclaw_core::types::SessionKey;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

/// Per-session message queue with debouncing and rate limiting.
pub struct MessageQueue {
    active: DashMap<SessionKey, QueueEntry>,
    debounce_ms: u64,
    max_queue_size: usize,
}

struct QueueEntry {
    messages: Vec<InboundMessage>,
    last_enqueue: Instant,
    processing: bool,
}

impl MessageQueue {
    pub fn new() -> Self {
        Self {
            active: DashMap::new(),
            debounce_ms: 500,
            max_queue_size: 10,
        }
    }

    /// Enqueue a message. Returns false if deduplicated or queue is full.
    pub async fn enqueue(&self, session_key: &SessionKey, message: &InboundMessage) -> bool {
        let mut entry = self
            .active
            .entry(session_key.clone())
            .or_insert_with(|| QueueEntry {
                messages: Vec::new(),
                last_enqueue: Instant::now(),
                processing: false,
            });

        // Check queue size limit
        if entry.messages.len() >= self.max_queue_size {
            warn!(session = %session_key, "Message queue full, dropping message");
            return false;
        }

        // Deduplicate by message ID
        if entry.messages.iter().any(|m| m.id == message.id) {
            debug!(session = %session_key, "Duplicate message, skipping");
            return false;
        }

        // Check if already processing
        if entry.processing {
            debug!(session = %session_key, "Session busy, queuing message");
        }

        entry.messages.push(message.clone());
        entry.last_enqueue = Instant::now();

        // Debounce: wait for more messages
        drop(entry);
        tokio::time::sleep(Duration::from_millis(self.debounce_ms)).await;

        true
    }

    /// Mark a session as no longer processing.
    pub async fn dequeue(&self, session_key: &SessionKey) {
        if let Some(mut entry) = self.active.get_mut(session_key) {
            entry.processing = false;
            entry.messages.clear();
        }
    }

    /// Get the number of pending messages for a session.
    pub fn pending_count(&self, session_key: &SessionKey) -> usize {
        self.active
            .get(session_key)
            .map(|e| e.messages.len())
            .unwrap_or(0)
    }

    /// Clean up stale queue entries.
    pub fn cleanup(&self, max_age: Duration) {
        self.active
            .retain(|_, entry| entry.last_enqueue.elapsed() < max_age || entry.processing);
    }
}
