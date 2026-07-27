//! Memory pillar: durable conversation/state store (SQLite + sqlite-vec, Decision #11)
//! plus context compression. Skeleton only — storage backend wired in a later pass.

use core_types::CoreResult;

pub trait ConversationStore: Send + Sync {
    fn append_message(&self, conversation_id: &str, role: &str, content: &str) -> CoreResult<()>;
}
