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
    #[error(
        "embedding dimension does not match this conversation's existing embeddings; \
         mixing embedding models within one workspace is not supported"
    )]
    EmbeddingDimensionMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationSummary {
    pub id: String,
    pub title: String,
    pub flow_name: String,
    pub created_at: i64,
    pub updated_at: i64,
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

    fn create_conversation(
        &self,
        id: &str,
        title: &str,
        flow_name: &str,
    ) -> Result<(), MemoryError>;

    fn list_conversations(&self) -> Result<Vec<ConversationSummary>, MemoryError>;

    fn rename_conversation(&self, id: &str, title: &str) -> Result<(), MemoryError>;

    fn set_conversation_flow(&self, id: &str, flow_name: &str) -> Result<(), MemoryError>;

    fn delete_conversation(&self, id: &str) -> Result<(), MemoryError>;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ToolPermissionDecision {
    Allow,
    Deny,
}

pub trait ToolPermissionStore: Send + Sync {
    fn get_tool_permission(
        &self,
        conversation_id: &str,
        tool_name: &str,
    ) -> Result<Option<ToolPermissionDecision>, MemoryError>;

    fn set_tool_permission(
        &self,
        conversation_id: &str,
        tool_name: &str,
        decision: ToolPermissionDecision,
    ) -> Result<(), MemoryError>;

    /// Forgets a remembered "Always" decision, so the next call to that tool in this
    /// conversation prompts again. A no-op if nothing was remembered.
    fn clear_tool_permission(
        &self,
        conversation_id: &str,
        tool_name: &str,
    ) -> Result<(), MemoryError>;

    /// Every remembered decision for a conversation, for a revoke UI to list.
    fn list_tool_permissions(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<(String, ToolPermissionDecision)>, MemoryError>;
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
