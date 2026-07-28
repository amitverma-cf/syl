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
    #[error("stored embedding has a corrupt length ({0} bytes, not a multiple of 4)")]
    CorruptEmbedding(usize),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EmbeddingMatch {
    pub content: String,
    pub score: f32,
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

pub trait EmbeddingStore: Send + Sync {
    fn store_embedding(
        &self,
        conversation_id: &str,
        content: &str,
        embedding: &[f32],
    ) -> Result<(), MemoryError>;

    fn search_similar(
        &self,
        conversation_id: &str,
        query: &[f32],
        top_k: usize,
    ) -> Result<Vec<EmbeddingMatch>, MemoryError>;
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
