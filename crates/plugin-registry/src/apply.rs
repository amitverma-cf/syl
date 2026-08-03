use std::path::Path;

use crate::{is_safe_relative_component, EngineEntry, ModelEntry, PluginRegistryError};

/// Hosts a polled registry entry's `download_url`/`extra_files` is allowed to point
/// at. This is deliberately narrow and specific to what the app's own committed
/// `registry/engines.json`/`models.json` actually reference today (GitHub Releases
/// for engine archives, Hugging Face for model weights) — not a general-purpose
/// allowlist, so it only needs to grow if a legitimate new source is added.
const ALLOWED_REMOTE_HOSTS: &[&str] = &["github.com", "huggingface.co"];

/// Parses, validates, and atomically persists a freshly-polled `engines.json`/
/// `models.json` pair into `registry_dir`.
///
/// Every entry across both files is validated before anything is written — one bad
/// entry rejects the whole poll rather than silently dropping just that entry, since
/// a partially-trusted registry is a confusing state to reason about. On success,
/// each file is written to a `.tmp` sibling and `rename`d into place, which is atomic
/// on the same filesystem — a concurrent read of `engines.json`/`models.json` (e.g.
/// `resolve_engine_library_path` running because a generation request came in while
/// this poll is landing) always sees either the complete old file or the complete new
/// one, never a partially-written one.
pub fn apply_remote_registry(
    registry_dir: &Path,
    engines_json: &str,
    models_json: &str,
) -> Result<(), PluginRegistryError> {
    let engines: Vec<EngineEntry> =
        serde_json::from_str(engines_json).map_err(|source| PluginRegistryError::Parse {
            path: registry_dir.join("engines.json"),
            source,
        })?;
    let models: Vec<ModelEntry> =
        serde_json::from_str(models_json).map_err(|source| PluginRegistryError::Parse {
            path: registry_dir.join("models.json"),
            source,
        })?;

    for engine in &engines {
        validate_engine_entry(engine)?;
    }
    for model in &models {
        validate_model_entry(model)?;
    }

    write_atomically(&registry_dir.join("engines.json"), engines_json)?;
    write_atomically(&registry_dir.join("models.json"), models_json)?;

    Ok(())
}

fn validate_engine_entry(entry: &EngineEntry) -> Result<(), PluginRegistryError> {
    validate_remote_url(&entry.id, &entry.download_url)?;
    if let Some(library_file) = &entry.library_file {
        if !is_safe_relative_component(library_file) {
            return Err(PluginRegistryError::RejectedRemoteEntry {
                entry: entry.id.clone(),
                reason: format!("library_file {library_file} is not a safe relative path"),
            });
        }
    }
    Ok(())
}

fn validate_model_entry(entry: &ModelEntry) -> Result<(), PluginRegistryError> {
    validate_remote_url(&entry.name, &entry.download_url)?;
    for extra in &entry.extra_files {
        validate_remote_url(&entry.name, extra)?;
    }
    Ok(())
}

/// A polled registry entry's URL must be `https://` (never `file://` — a remote
/// response has no legitimate reason to name a path on the polling machine) and
/// resolve to an allowlisted host, so a compromised registry response can't redirect
/// a download at arbitrary attacker infrastructure.
fn validate_remote_url(entry_name: &str, raw_url: &str) -> Result<(), PluginRegistryError> {
    let parsed = url::Url::parse(raw_url).map_err(|err| PluginRegistryError::RejectedRemoteEntry {
        entry: entry_name.to_string(),
        reason: format!("{raw_url} is not a valid URL: {err}"),
    })?;

    if parsed.scheme() != "https" {
        return Err(PluginRegistryError::RejectedRemoteEntry {
            entry: entry_name.to_string(),
            reason: format!("{raw_url} must use https"),
        });
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| PluginRegistryError::RejectedRemoteEntry {
            entry: entry_name.to_string(),
            reason: format!("{raw_url} has no host"),
        })?;

    if !ALLOWED_REMOTE_HOSTS.contains(&host) {
        return Err(PluginRegistryError::RejectedRemoteEntry {
            entry: entry_name.to_string(),
            reason: format!("{host} is not an allowed download host"),
        });
    }

    Ok(())
}

fn write_atomically(dest: &Path, contents: &str) -> Result<(), PluginRegistryError> {
    let tmp = dest.with_extension("json.tmp");
    std::fs::write(&tmp, contents).map_err(|source| PluginRegistryError::Io {
        path: tmp.clone(),
        source,
    })?;
    std::fs::rename(&tmp, dest).map_err(|source| PluginRegistryError::Io {
        path: dest.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::apply_remote_registry;
    use crate::{load_engine_entries, load_model_entries, PluginRegistryError};
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_registry_dir(label: &str) -> std::path::PathBuf {
        let unique = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "syl-apply-registry-{label}-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn good_engines_json() -> &'static str {
        r#"[{"id":"llama-cpp","version":"1","platform":"windows-x64","download_url":"https://github.com/ggml-org/llama.cpp/releases/download/v1/llama.zip","sha256":null,"library_file":"llama.dll"}]"#
    }

    fn good_models_json() -> &'static str {
        r#"[{"name":"test-model","kind":"chat","size_bytes":1,"quantization":"q1","required_engine":"llama-cpp","download_url":"https://huggingface.co/org/repo/resolve/main/model.gguf","sha256":null,"extra_files":[]}]"#
    }

    #[test]
    fn accepts_and_persists_a_registry_with_only_allowlisted_hosts() {
        let dir = temp_registry_dir("ok");
        apply_remote_registry(&dir, good_engines_json(), good_models_json()).unwrap();

        let engines = load_engine_entries(&dir).unwrap();
        assert_eq!(engines.len(), 1);
        assert_eq!(engines[0].id, "llama-cpp");
        let models = load_model_entries(&dir).unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].name, "test-model");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_the_whole_poll_when_one_engine_entry_uses_a_non_allowlisted_host() {
        let dir = temp_registry_dir("bad-host");
        // Seed a last-known-good file first, matching how a real poll would only ever
        // run after the workspace already has a seeded registry.
        std::fs::write(dir.join("engines.json"), good_engines_json()).unwrap();
        std::fs::write(dir.join("models.json"), good_models_json()).unwrap();

        let malicious_engines = r#"[{"id":"llama-cpp","version":"1","platform":"windows-x64","download_url":"https://attacker.example/llama.zip","sha256":null,"library_file":"llama.dll"}]"#;

        let err = apply_remote_registry(&dir, malicious_engines, good_models_json()).unwrap_err();
        assert!(matches!(
            err,
            PluginRegistryError::RejectedRemoteEntry { .. }
        ));

        // The last-known-good file must be untouched — a bad poll must not corrupt or
        // partially overwrite what was already trusted.
        let engines = load_engine_entries(&dir).unwrap();
        assert_eq!(
            engines[0].download_url,
            "https://github.com/ggml-org/llama.cpp/releases/download/v1/llama.zip"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_a_file_url_even_though_resolve_download_url_would_normally_accept_it() {
        let dir = temp_registry_dir("file-url");
        let sneaky_models = r#"[{"name":"test-model","kind":"chat","size_bytes":1,"quantization":"q1","required_engine":"llama-cpp","download_url":"file:///C:/Windows/System32/calc.exe","sha256":null,"extra_files":[]}]"#;

        let err = apply_remote_registry(&dir, good_engines_json(), sneaky_models).unwrap_err();
        assert!(matches!(
            err,
            PluginRegistryError::RejectedRemoteEntry { .. }
        ));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_an_http_downgrade() {
        let dir = temp_registry_dir("http-downgrade");
        let downgraded = r#"[{"id":"llama-cpp","version":"1","platform":"windows-x64","download_url":"http://github.com/ggml-org/llama.cpp/releases/download/v1/llama.zip","sha256":null,"library_file":"llama.dll"}]"#;

        let err = apply_remote_registry(&dir, downgraded, good_models_json()).unwrap_err();
        assert!(matches!(
            err,
            PluginRegistryError::RejectedRemoteEntry { .. }
        ));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_a_traversal_in_library_file() {
        let dir = temp_registry_dir("library-file-traversal");
        let malicious_engines = r#"[{"id":"llama-cpp","version":"1","platform":"windows-x64","download_url":"https://github.com/ggml-org/llama.cpp/releases/download/v1/llama.zip","sha256":null,"library_file":"../../../evil.dll"}]"#;

        let err = apply_remote_registry(&dir, malicious_engines, good_models_json()).unwrap_err();
        assert!(matches!(
            err,
            PluginRegistryError::RejectedRemoteEntry { .. }
        ));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_an_extra_file_url_pointing_at_a_non_allowlisted_host() {
        let dir = temp_registry_dir("extra-files-bad-host");
        let sneaky_models = r#"[{"name":"test-model","kind":"chat","size_bytes":1,"quantization":"q1","required_engine":"llama-cpp","download_url":"https://huggingface.co/org/repo/resolve/main/model.gguf","sha256":null,"extra_files":["https://attacker.example/payload"]}]"#;

        let err = apply_remote_registry(&dir, good_engines_json(), sneaky_models).unwrap_err();
        assert!(matches!(
            err,
            PluginRegistryError::RejectedRemoteEntry { .. }
        ));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_second_good_poll_replaces_the_first() {
        let dir = temp_registry_dir("replace");
        apply_remote_registry(&dir, good_engines_json(), good_models_json()).unwrap();

        let updated_engines = r#"[{"id":"llama-cpp","version":"2","platform":"windows-x64","download_url":"https://github.com/ggml-org/llama.cpp/releases/download/v2/llama.zip","sha256":null,"library_file":"llama.dll"}]"#;
        apply_remote_registry(&dir, updated_engines, good_models_json()).unwrap();

        let engines = load_engine_entries(&dir).unwrap();
        assert_eq!(engines[0].version, "2");

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
