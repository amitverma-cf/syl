//! Durable storage for conversations and agent state.

use core_types::CoreResult;

/// Persists conversation messages.
pub trait ConversationStore: Send + Sync {
    /// Appends a message to the given conversation.
    ///
    /// # Errors
    /// Returns an error if the message could not be persisted.
    fn append_message(&self, conversation_id: &str, role: &str, content: &str) -> CoreResult<()>;
}
