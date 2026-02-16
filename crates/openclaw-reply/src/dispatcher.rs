use openclaw_core::message::{OutboundReply, ReplyDispatchKind};
use openclaw_core::types::SessionKey;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{debug, error};

/// Payload sent through the dispatcher.
#[derive(Debug, Clone)]
pub struct ReplyPayload {
    pub kind: ReplyDispatchKind,
    pub session_key: SessionKey,
    pub text: Option<String>,
    pub is_final: bool,
}

/// Reply dispatcher that handles tool results, streaming blocks, and final replies.
pub struct ReplyDispatcher {
    tx: mpsc::UnboundedSender<ReplyPayload>,
    queued_counts: parking_lot::Mutex<HashMap<ReplyDispatchKind, usize>>,
}

impl ReplyDispatcher {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<ReplyPayload>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (
            Self {
                tx,
                queued_counts: parking_lot::Mutex::new(HashMap::new()),
            },
            rx,
        )
    }

    /// Send a tool result payload.
    pub fn send_tool_result(&self, session_key: SessionKey, text: String) -> bool {
        self.send(ReplyPayload {
            kind: ReplyDispatchKind::Tool,
            session_key,
            text: Some(text),
            is_final: false,
        })
    }

    /// Send a streaming block payload.
    pub fn send_block(&self, session_key: SessionKey, text: String) -> bool {
        self.send(ReplyPayload {
            kind: ReplyDispatchKind::Block,
            session_key,
            text: Some(text),
            is_final: false,
        })
    }

    /// Send the final reply payload.
    pub fn send_final(&self, session_key: SessionKey, text: String) -> bool {
        self.send(ReplyPayload {
            kind: ReplyDispatchKind::Final,
            session_key,
            text: Some(text),
            is_final: true,
        })
    }

    fn send(&self, payload: ReplyPayload) -> bool {
        let kind = payload.kind;
        let result = self.tx.send(payload).is_ok();
        if result {
            *self
                .queued_counts
                .lock()
                .entry(kind)
                .or_insert(0) += 1;
        }
        result
    }

    pub fn get_queued_counts(&self) -> HashMap<ReplyDispatchKind, usize> {
        self.queued_counts.lock().clone()
    }
}
