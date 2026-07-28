mod sqlite_store;

pub use sqlite_store::SqliteConversationStore;

use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("failed to create database directory {}: {source}", .path.display())]
    CreateDir {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

pub trait ConversationStore: Send + Sync {
    fn append_message(
        &self,
        conversation_id: &str,
        role: &str,
        content: &str,
    ) -> Result<(), MemoryError>;

    fn list_messages(&self, conversation_id: &str) -> Result<Vec<Message>, MemoryError>;
}

pub fn open(db_path: &Path) -> Result<SqliteConversationStore, MemoryError> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| MemoryError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    SqliteConversationStore::open(db_path)
}
