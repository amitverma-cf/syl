//! Resolves the app's local workspace directory and its subdirectories.
//!
//! Currently `~/.syl/`, used directly as a temporary stand-in for the OS-appropriate per-user
//! app data directory the shipped app will eventually use.

use std::path::PathBuf;

/// Returns the app's local workspace root (`~/.syl/`).
pub fn workspace_root() -> PathBuf {
    directories::UserDirs::new()
        .map(|dirs| dirs.home_dir().join(".syl"))
        .unwrap_or_else(|| PathBuf::from(".syl"))
}

/// Returns the directory downloaded/installed engine plugins are stored in.
pub fn engines_dir() -> PathBuf {
    workspace_root().join("engines")
}

/// Returns the directory downloaded model weights are stored in.
pub fn models_dir() -> PathBuf {
    workspace_root().join("models")
}

/// Returns the directory the conversation database and other durable state live in.
pub fn memory_dir() -> PathBuf {
    workspace_root().join("memory")
}

/// Returns the path to the conversation database file.
pub fn conversation_db_path() -> PathBuf {
    memory_dir().join("conversations.sqlite")
}

/// Returns the directory persisted log files are written to.
pub fn logs_dir() -> PathBuf {
    workspace_root().join("logs")
}

/// Returns the directory engine/model registry files (`engines.json`, `models.json`, and any
/// local overrides) are read from.
pub fn registry_dir() -> PathBuf {
    workspace_root().join("registry")
}
