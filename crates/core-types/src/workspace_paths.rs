use std::path::PathBuf;

pub fn workspace_root() -> PathBuf {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let repo_root = repo_root.canonicalize().unwrap_or(repo_root);
    let repo_root_str = repo_root.to_string_lossy().to_string();
    let repo_root = match repo_root_str.strip_prefix(r"\\?\") {
        Some(stripped) => PathBuf::from(stripped),
        None => repo_root,
    };
    repo_root.join(".syl")
}

pub fn engines_dir() -> PathBuf {
    workspace_root().join("engines")
}

pub fn models_dir() -> PathBuf {
    workspace_root().join("models")
}

pub fn memory_dir() -> PathBuf {
    workspace_root().join("memory")
}

pub fn conversation_db_path() -> PathBuf {
    memory_dir().join("conversations.sqlite")
}

pub fn logs_dir() -> PathBuf {
    workspace_root().join("logs")
}

pub fn registry_dir() -> PathBuf {
    workspace_root().join("registry")
}

pub fn flows_dir() -> PathBuf {
    workspace_root().join("flows")
}

pub fn env_file() -> PathBuf {
    workspace_root().join(".env")
}

pub fn custom_providers_file() -> PathBuf {
    workspace_root().join("custom_providers.json")
}

pub fn mcp_servers_file() -> PathBuf {
    workspace_root().join("mcp_servers.json")
}
