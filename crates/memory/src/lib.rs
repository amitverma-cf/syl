//! Durable storage for conversations and agent state.

mod sqlite_store;

pub use sqlite_store::SqliteConversationStore;

use std::path::Path;

/// An error returned by this crate.
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    /// The underlying SQLite database returned an error.
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    /// The parent directory for the database file could not be created.
    #[error("failed to create database directory {}: {source}", .path.display())]
    CreateDir {
        /// The directory that could not be created.
        path: std::path::PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

/// A single stored message in a conversation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Message {
    /// Who sent the message (e.g. `"user"`, `"assistant"`).
    pub role: String,
    /// The message text.
    pub content: String,
    /// When the message was stored, as a Unix timestamp in seconds.
    pub created_at: i64,
}

/// Persists conversation messages.
pub trait ConversationStore: Send + Sync {
    /// Appends a message to the given conversation, creating it if it doesn't already exist.
    ///
    /// # Errors
    /// Returns an error if the message could not be persisted.
    fn append_message(
        &self,
        conversation_id: &str,
        role: &str,
        content: &str,
    ) -> Result<(), MemoryError>;

    /// Returns every message stored for `conversation_id`, oldest first.
    ///
    /// # Errors
    /// Returns an error if the messages could not be read.
    fn list_messages(&self, conversation_id: &str) -> Result<Vec<Message>, MemoryError>;
}

/// Opens (creating if necessary) the SQLite conversation store at `db_path`, creating its
/// parent directory if needed.
///
/// # Errors
/// Returns an error if the parent directory can't be created or the database can't be opened.
pub fn open(db_path: &Path) -> Result<SqliteConversationStore, MemoryError> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| MemoryError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    SqliteConversationStore::open(db_path)
}
