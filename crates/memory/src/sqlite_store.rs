//! SQLite-backed [`ConversationStore`] implementation.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;

use crate::{ConversationStore, MemoryError, Message};

/// A [`ConversationStore`] backed by a single SQLite database file.
pub struct SqliteConversationStore {
    conn: Mutex<Connection>,
}

impl SqliteConversationStore {
    /// Opens the database at `db_path`, creating the schema if it doesn't already exist.
    ///
    /// # Errors
    /// Returns an error if the database can't be opened or the schema can't be created.
    pub fn open(db_path: &Path) -> Result<Self, MemoryError> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS messages (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                conversation_id TEXT NOT NULL,
                role            TEXT NOT NULL,
                content         TEXT NOT NULL,
                created_at      INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_messages_conversation
                ON messages (conversation_id, id);",
        )?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn in_memory_store() -> SqliteConversationStore {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE messages (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                conversation_id TEXT NOT NULL,
                role            TEXT NOT NULL,
                content         TEXT NOT NULL,
                created_at      INTEGER NOT NULL
            );",
        )
        .unwrap();
        SqliteConversationStore {
            conn: Mutex::new(conn),
        }
    }

    #[test]
    fn append_then_list_returns_messages_in_order() {
        let store = in_memory_store();
        store.append_message("c1", "user", "hello").unwrap();
        store.append_message("c1", "assistant", "hi there").unwrap();

        let messages = store.list_messages("c1").unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content, "hello");
        assert_eq!(messages[1].role, "assistant");
        assert_eq!(messages[1].content, "hi there");
    }

    #[test]
    fn list_messages_on_unknown_conversation_returns_empty() {
        let store = in_memory_store();
        let messages = store.list_messages("does-not-exist").unwrap();
        assert!(messages.is_empty());
    }

    #[test]
    fn conversations_do_not_leak_into_each_other() {
        let store = in_memory_store();
        store.append_message("c1", "user", "in c1").unwrap();
        store.append_message("c2", "user", "in c2").unwrap();

        let c1_messages = store.list_messages("c1").unwrap();
        assert_eq!(c1_messages.len(), 1);
        assert_eq!(c1_messages[0].content, "in c1");
    }

    #[test]
    fn open_creates_schema_on_a_fresh_database() {
        let dir = std::env::temp_dir().join(format!("syl-memory-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("test.sqlite");

        let store = SqliteConversationStore::open(&db_path).unwrap();
        store.append_message("c1", "user", "hello").unwrap();
        let messages = store.list_messages("c1").unwrap();
        assert_eq!(messages.len(), 1);

        drop(store);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
