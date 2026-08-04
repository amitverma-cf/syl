use std::path::Path;
use std::sync::{Mutex, Once};

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

static REGISTER_SQLITE_VEC: Once = Once::new();

/// Registers the `sqlite-vec` extension process-wide via `sqlite3_auto_extension`
/// so every connection opened afterward (including this one) has `vec0` virtual
/// tables and functions like `vec_distance_cosine` available — `Once` because
/// SQLite already dedups repeated `sqlite3_auto_extension` calls with the same
/// function pointer, but there's no reason to pay even that check on every
/// `open`/`open_in_memory` call.
fn register_sqlite_vec_extension() {
    REGISTER_SQLITE_VEC.call_once(|| unsafe {
        rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute::<
            *const (),
            unsafe extern "C" fn(
                *mut rusqlite::ffi::sqlite3,
                *mut *const std::ffi::c_char,
                *const rusqlite::ffi::sqlite3_api_routines,
            ) -> std::ffi::c_int,
        >(
            sqlite_vec::sqlite3_vec_init as *const ()
        )));
    });
}

impl SqliteConversationStore {
    pub fn open(db_path: &Path) -> Result<Self, MemoryError> {
        register_sqlite_vec_extension();
        let conn = Connection::open(db_path)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn open_in_memory() -> Result<Self, MemoryError> {
        register_sqlite_vec_extension();
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
        // vec_embeddings is created lazily on first store_embedding call, so it
        // may not exist yet — nothing to delete in that case.
        let vec_table_exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = 'vec_embeddings')",
            [],
            |row| row.get(0),
        )?;
        if vec_table_exists {
            conn.execute(
                "DELETE FROM vec_embeddings WHERE conversation_id = ?1",
                (id,),
            )?;
        }
        conn.execute(
            "DELETE FROM tool_permissions WHERE conversation_id = ?1",
            (id,),
        )?;
        Ok(())
    }
}

/// A `vec0` virtual table's vector column dimension is fixed at
/// `CREATE VIRTUAL TABLE` time, so the table is created lazily on the first
/// `store_embedding` call, sized to whatever dimension that first embedding
/// has. A later embedding of a different dimension is rejected with a clear
/// error rather than silently corrupting or truncating the vector — mixing
/// embedding models within one workspace was never really meaningful for
/// retrieval anyway (their vector spaces aren't comparable).
fn ensure_vec_table(conn: &Connection, dims: usize) -> Result<(), MemoryError> {
    conn.execute_batch(&format!(
        "CREATE VIRTUAL TABLE IF NOT EXISTS vec_embeddings USING vec0(
            conversation_id TEXT PARTITION KEY,
            +content TEXT,
            embedding FLOAT[{dims}]
        )"
    ))?;
    Ok(())
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
        ensure_vec_table(&conn, embedding.len())?;
        conn.execute(
            "INSERT INTO vec_embeddings (conversation_id, content, embedding)
             VALUES (?1, ?2, ?3)",
            (conversation_id, content, encode_vector(embedding)),
        )
        .map_err(|source| {
            // sqlite-vec reports a dimension mismatch as a generic SQLITE_ERROR
            // (not a distinct error code), so the only reliable signal is its
            // own error message text.
            let is_dimension_mismatch = matches!(&source, rusqlite::Error::SqliteFailure(_, Some(msg)) if msg.contains("Dimension mismatch"));
            if is_dimension_mismatch {
                MemoryError::EmbeddingDimensionMismatch
            } else {
                MemoryError::Database(source)
            }
        })?;
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

        // No `vec_embeddings` table yet means nothing has ever been stored in
        // this store — a real "no matches" case, not an error.
        let table_exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = 'vec_embeddings')",
            [],
            |row| row.get(0),
        )?;
        if !table_exists {
            return Ok(Vec::new());
        }

        // A "full scan" KNN query (ranking by vec_distance_cosine directly,
        // rather than the indexed `embedding MATCH ?` form) — appropriate at
        // per-conversation scale, and lets a plain `WHERE conversation_id = ?`
        // filter combine naturally with ORDER BY/LIMIT.
        let mut stmt = conn.prepare(
            "SELECT content, vec_distance_cosine(embedding, ?1) AS distance
             FROM vec_embeddings
             WHERE conversation_id = ?2
             ORDER BY distance ASC
             LIMIT ?3",
        )?;
        let rows = stmt.query_map(
            (encode_vector(query), conversation_id, top_k as i64),
            |row| {
                let content: String = row.get(0)?;
                let distance: f32 = row.get(1)?;
                Ok(EmbeddingMatch {
                    content,
                    score: 1.0 - distance,
                })
            },
        )?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(MemoryError::Database)
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

    fn clear_tool_permission(
        &self,
        conversation_id: &str,
        tool_name: &str,
    ) -> Result<(), MemoryError> {
        let conn = self
            .conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        conn.execute(
            "DELETE FROM tool_permissions WHERE conversation_id = ?1 AND tool_name = ?2",
            (conversation_id, tool_name),
        )?;
        Ok(())
    }

    fn list_tool_permissions(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<(String, ToolPermissionDecision)>, MemoryError> {
        let conn = self
            .conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut stmt = conn.prepare(
            "SELECT tool_name, decision FROM tool_permissions WHERE conversation_id = ?1
             ORDER BY tool_name ASC",
        )?;
        let rows = stmt.query_map((conversation_id,), |row| {
            let tool_name: String = row.get(0)?;
            let decision: String = row.get(1)?;
            Ok((
                tool_name,
                match decision.as_str() {
                    "allow" => ToolPermissionDecision::Allow,
                    _ => ToolPermissionDecision::Deny,
                },
            ))
        })?;
        let mut permissions = Vec::new();
        for row in rows {
            permissions.push(row?);
        }
        Ok(permissions)
    }
}

/// `vec0` accepts a `float[N]` column value as its raw little-endian byte
/// layout directly, so this is also the wire format `store_embedding`/
/// `search_similar` pass across the FFI boundary — no separate decode step
/// needed on the read side since `sqlite-vec`'s own functions operate on
/// this layout internally.
fn encode_vector(vector: &[f32]) -> Vec<u8> {
    vector.iter().flat_map(|f| f.to_le_bytes()).collect()
}
