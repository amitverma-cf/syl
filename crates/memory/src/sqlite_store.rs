use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;

use crate::{
    ConversationStore, ConversationSummary, EmbeddingMatch, EmbeddingStore, MemoryError, Message,
    ToolPermissionDecision, ToolPermissionStore,
};

pub struct SqliteConversationStore {
    conn: Mutex<Connection>,
}

const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS messages (
        id              INTEGER PRIMARY KEY AUTOINCREMENT,
        conversation_id TEXT NOT NULL,
        role            TEXT NOT NULL,
        content         TEXT NOT NULL,
        created_at      INTEGER NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_messages_conversation
        ON messages (conversation_id, id);
    CREATE TABLE IF NOT EXISTS embeddings (
        id              INTEGER PRIMARY KEY AUTOINCREMENT,
        conversation_id TEXT NOT NULL,
        content         TEXT NOT NULL,
        dims            INTEGER NOT NULL,
        vector          BLOB NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_embeddings_conversation
        ON embeddings (conversation_id);
    CREATE TABLE IF NOT EXISTS tool_permissions (
        conversation_id TEXT NOT NULL,
        tool_name       TEXT NOT NULL,
        decision        TEXT NOT NULL,
        PRIMARY KEY (conversation_id, tool_name)
    );
    CREATE TABLE IF NOT EXISTS conversations (
        id         TEXT PRIMARY KEY,
        title      TEXT NOT NULL,
        flow_name  TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    );
";

impl SqliteConversationStore {
    pub fn open(db_path: &Path) -> Result<Self, MemoryError> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn open_in_memory() -> Result<Self, MemoryError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

impl ConversationStore for SqliteConversationStore {
    fn append_message(
        &self,
        conversation_id: &str,
        role: &str,
        content: &str,
    ) -> Result<(), MemoryError> {
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let conn = self
            .conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        conn.execute(
            "INSERT INTO messages (conversation_id, role, content, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            (conversation_id, role, content, created_at),
        )?;
        Ok(())
    }

    fn list_messages(&self, conversation_id: &str) -> Result<Vec<Message>, MemoryError> {
        let conn = self
            .conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut stmt = conn.prepare(
            "SELECT role, content, created_at FROM messages
             WHERE conversation_id = ?1
             ORDER BY id ASC",
        )?;
        let rows = stmt.query_map((conversation_id,), |row| {
            Ok(Message {
                role: row.get(0)?,
                content: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(row?);
        }
        Ok(messages)
    }

    fn create_conversation(
        &self,
        id: &str,
        title: &str,
        flow_name: &str,
    ) -> Result<(), MemoryError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let conn = self
            .conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        conn.execute(
            "INSERT INTO conversations (id, title, flow_name, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
            (id, title, flow_name, now),
        )?;
        Ok(())
    }

    fn list_conversations(&self) -> Result<Vec<ConversationSummary>, MemoryError> {
        let conn = self
            .conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut stmt = conn.prepare(
            "SELECT id, title, flow_name, created_at, updated_at FROM conversations
             ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map((), |row| {
            Ok(ConversationSummary {
                id: row.get(0)?,
                title: row.get(1)?,
                flow_name: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;

        let mut conversations = Vec::new();
        for row in rows {
            conversations.push(row?);
        }
        Ok(conversations)
    }

    fn rename_conversation(&self, id: &str, title: &str) -> Result<(), MemoryError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let conn = self
            .conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        conn.execute(
            "UPDATE conversations SET title = ?2, updated_at = ?3 WHERE id = ?1",
            (id, title, now),
        )?;
        Ok(())
    }

    fn set_conversation_flow(&self, id: &str, flow_name: &str) -> Result<(), MemoryError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let conn = self
            .conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        conn.execute(
            "UPDATE conversations SET flow_name = ?2, updated_at = ?3 WHERE id = ?1",
            (id, flow_name, now),
        )?;
        Ok(())
    }

    fn delete_conversation(&self, id: &str) -> Result<(), MemoryError> {
        let conn = self
            .conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        conn.execute("DELETE FROM conversations WHERE id = ?1", (id,))?;
        conn.execute("DELETE FROM messages WHERE conversation_id = ?1", (id,))?;
        conn.execute("DELETE FROM embeddings WHERE conversation_id = ?1", (id,))?;
        conn.execute(
            "DELETE FROM tool_permissions WHERE conversation_id = ?1",
            (id,),
        )?;
        Ok(())
    }
}

impl EmbeddingStore for SqliteConversationStore {
    fn store_embedding(
        &self,
        conversation_id: &str,
        content: &str,
        embedding: &[f32],
    ) -> Result<(), MemoryError> {
        let conn = self
            .conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        conn.execute(
            "INSERT INTO embeddings (conversation_id, content, dims, vector)
             VALUES (?1, ?2, ?3, ?4)",
            (
                conversation_id,
                content,
                embedding.len() as i64,
                encode_vector(embedding),
            ),
        )?;
        Ok(())
    }

    fn search_similar(
        &self,
        conversation_id: &str,
        query: &[f32],
        top_k: usize,
    ) -> Result<Vec<EmbeddingMatch>, MemoryError> {
        let conn = self
            .conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut stmt =
            conn.prepare("SELECT content, vector FROM embeddings WHERE conversation_id = ?1")?;
        let rows = stmt.query_map((conversation_id,), |row| {
            let content: String = row.get(0)?;
            let vector: Vec<u8> = row.get(1)?;
            Ok((content, vector))
        })?;

        let mut scored = Vec::new();
        for row in rows {
            let (content, raw) = row?;
            let vector = decode_vector(&raw)?;
            let score = cosine_similarity(query, &vector);
            scored.push(EmbeddingMatch { content, score });
        }

        scored.sort_by(|a, b| b.score.total_cmp(&a.score));
        scored.truncate(top_k);
        Ok(scored)
    }
}

impl ToolPermissionStore for SqliteConversationStore {
    fn get_tool_permission(
        &self,
        conversation_id: &str,
        tool_name: &str,
    ) -> Result<Option<ToolPermissionDecision>, MemoryError> {
        let conn = self
            .conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let decision: Option<String> = conn
            .query_row(
                "SELECT decision FROM tool_permissions
                 WHERE conversation_id = ?1 AND tool_name = ?2",
                (conversation_id, tool_name),
                |row| row.get(0),
            )
            .ok();

        Ok(decision.map(|d| match d.as_str() {
            "allow" => ToolPermissionDecision::Allow,
            _ => ToolPermissionDecision::Deny,
        }))
    }

    fn set_tool_permission(
        &self,
        conversation_id: &str,
        tool_name: &str,
        decision: ToolPermissionDecision,
    ) -> Result<(), MemoryError> {
        let decision_str = match decision {
            ToolPermissionDecision::Allow => "allow",
            ToolPermissionDecision::Deny => "deny",
        };
        let conn = self
            .conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        conn.execute(
            "INSERT INTO tool_permissions (conversation_id, tool_name, decision)
             VALUES (?1, ?2, ?3)
             ON CONFLICT (conversation_id, tool_name) DO UPDATE SET decision = excluded.decision",
            (conversation_id, tool_name, decision_str),
        )?;
        Ok(())
    }
}

fn encode_vector(vector: &[f32]) -> Vec<u8> {
    vector.iter().flat_map(|f| f.to_le_bytes()).collect()
}

fn decode_vector(raw: &[u8]) -> Result<Vec<f32>, MemoryError> {
    if !raw.len().is_multiple_of(4) {
        return Err(MemoryError::CorruptEmbedding(raw.len()));
    }
    Ok(raw
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect())
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}
