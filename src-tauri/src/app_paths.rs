//! Resolves the app's per-user data directory and its subdirectories.

use std::path::PathBuf;

/// Returns the app's per-user data directory (OS-appropriate location), creating no
/// directories itself.
pub fn data_dir() -> PathBuf {
    directories::ProjectDirs::from("com", "syl", "syl")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("data"))
}

/// Returns the directory persisted log files are written to.
pub fn logs_dir() -> PathBuf {
    data_dir().join("logs")
}

/// Returns the path to the conversation database file.
pub fn conversation_db_path() -> PathBuf {
    data_dir().join("db").join("conversations.sqlite")
}
