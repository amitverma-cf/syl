use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;

use crate::{ConversationStore, EmbeddingMatch, EmbeddingStore, MemoryError, Message};

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
