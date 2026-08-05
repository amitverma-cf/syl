use std::path::PathBuf;

/// The app's bundle identifier (matches `identifier` in `tauri.conf.json`) —
/// used to namespace this app's directory within the OS's shared per-user
/// app-data location.
const APP_IDENTIFIER: &str = "com.syl.app";

/// Root directory for all persisted app state (models, conversations, flows,
/// registry, logs, etc). Can be overridden with `SYL_WORKSPACE_DIR` to point
/// at an isolated directory — used by the E2E test suite so it never touches
/// a developer's real workspace.
///
/// Otherwise resolves to the OS-idiomatic per-user app-data directory (the
/// same location Tauri's own `app_data_dir()` would resolve to for this
/// app's identifier: `%APPDATA%\com.syl.app` on Windows, `~/Library/
/// Application Support/com.syl.app` on macOS, `$XDG_DATA_HOME/com.syl.app`
/// (or `~/.local/share/com.syl.app`) on Linux) — not the previous
/// `<repo>/.syl`, which only ever made sense for a dev checkout: for a real
/// installed build, `CARGO_MANIFEST_DIR` is a *build-time* constant baked
/// into the binary, pointing at wherever the app was built, not wherever
/// it's installed. See `legacy_repo_workspace_root` for the one-time
/// migration path off the old location.
pub fn workspace_root() -> PathBuf {
    if let Ok(dir) = std::env::var("SYL_WORKSPACE_DIR") {
        return PathBuf::from(dir);
    }

    match dirs::data_dir() {
        Some(dir) => dir.join(APP_IDENTIFIER),
        // No resolvable app-data directory (unusual, but not impossible on
        // some minimal/containerized environments) — fall back to the old
        // repo-relative location rather than panicking.
        None => legacy_repo_workspace_root(),
    }
}

/// The pre-migration workspace location (`<repo>/.syl`), kept around only so
/// a one-time startup migration (see `src-tauri/src/bootstrap.rs`) can find
/// and move any existing dev checkout's data into the new OS-idiomatic
/// location the first time it runs after this change.
pub fn legacy_repo_workspace_root() -> PathBuf {
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

pub fn scheduled_jobs_file() -> PathBuf {
    workspace_root().join("scheduled_jobs.json")
}

/// Genuinely user-adjustable app settings (autostart, resource limits,
/// telemetry opt-in/out) — distinct from `config/app.json`, which is
/// committed, build-time, repo-level config the app ships with and never
/// writes back to.
pub fn settings_file() -> PathBuf {
    workspace_root().join("settings.json")
}

/// Where installed extensions live, one subdirectory per extension id:
/// `.syl/extensions/<id>/manifest.json` plus whatever backend binary/assets
/// that extension ships alongside its manifest.
pub fn extensions_dir() -> PathBuf {
    workspace_root().join("extensions")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // SYL_WORKSPACE_DIR is process-global, so every test that touches it
    // must hold this lock for its whole body — `cargo test` runs tests in
    // this binary on multiple threads by default, and two tests setting the
    // env var to different values at the same time would race.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn workspace_root_honors_the_env_override() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        // SAFETY: serialized by ENV_LOCK above.
        unsafe {
            std::env::set_var("SYL_WORKSPACE_DIR", "/tmp/syl-test-override");
        }
        assert_eq!(workspace_root(), PathBuf::from("/tmp/syl-test-override"));
        unsafe {
            std::env::remove_var("SYL_WORKSPACE_DIR");
        }
    }

    #[test]
    fn legacy_repo_workspace_root_ends_with_dot_syl() {
        assert_eq!(legacy_repo_workspace_root().file_name().unwrap(), ".syl");
    }

    #[test]
    fn all_workspace_paths_are_nested_under_the_workspace_root() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        // SAFETY: serialized by ENV_LOCK above.
        unsafe {
            std::env::set_var("SYL_WORKSPACE_DIR", "/tmp/syl-test-nesting");
        }
        let root = workspace_root();
        assert!(engines_dir().starts_with(&root));
        assert!(models_dir().starts_with(&root));
        assert!(memory_dir().starts_with(&root));
        assert!(conversation_db_path().starts_with(&root));
        assert!(logs_dir().starts_with(&root));
        assert!(registry_dir().starts_with(&root));
        assert!(flows_dir().starts_with(&root));
        assert!(env_file().starts_with(&root));
        assert!(custom_providers_file().starts_with(&root));
        assert!(mcp_servers_file().starts_with(&root));
        assert!(scheduled_jobs_file().starts_with(&root));
        assert!(settings_file().starts_with(&root));
        assert!(extensions_dir().starts_with(&root));
        unsafe {
            std::env::remove_var("SYL_WORKSPACE_DIR");
        }
    }
}
