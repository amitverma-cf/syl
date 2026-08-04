use std::path::Path;

use crate::{
    is_safe_relative_component, verify_registry_signature, EngineEntry, ModelEntry,
    PluginRegistryError,
};

/// A detached Ed25519 signature (hex) for each manifest file, checked against
/// a hex-encoded public key before anything else — provenance ("this really
/// came from the repo owner"), on top of the existing sha256/host-allowlist
/// checks below, which are integrity, not provenance. Optional today because
/// no real signing key has been provisioned/wired into a publish pipeline yet
/// (see `examples/sign_registry.rs`) — once `registryManifestPublicKey` is
/// set in `config/app.json`, a poll without valid signatures is rejected.
#[derive(Debug, Clone, Copy)]
pub struct RegistrySignatures<'a> {
    pub public_key_hex: &'a str,
    pub engines_signature_hex: &'a str,
    pub models_signature_hex: &'a str,
}

/// Parses, validates, and atomically persists a freshly-polled `engines.json`/
/// `models.json` pair into `registry_dir`.
///
/// `allowed_hosts` is the caller-supplied list of hosts a polled entry's
/// `download_url`/`extra_files` may point at — deliberately not a hardcoded constant
/// in this crate, so the app can make it a config value (`config/app.json`'s
/// `registryAllowedHosts`) rather than a code change when a legitimate new source is
/// added.
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
    allowed_hosts: &[String],
    signatures: Option<RegistrySignatures<'_>>,
) -> Result<(), PluginRegistryError> {
    if let Some(sigs) = signatures {
        verify_registry_signature(
            "engines.json",
            engines_json.as_bytes(),
            sigs.engines_signature_hex,
            sigs.public_key_hex,
        )?;
        verify_registry_signature(
            "models.json",
            models_json.as_bytes(),
            sigs.models_signature_hex,
            sigs.public_key_hex,
        )?;
    }

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
        validate_engine_entry(engine, allowed_hosts)?;
    }
    for model in &models {
        validate_model_entry(model, allowed_hosts)?;
    }

    write_atomically(&registry_dir.join("engines.json"), engines_json)?;
    write_atomically(&registry_dir.join("models.json"), models_json)?;

    Ok(())
}

fn validate_engine_entry(
    entry: &EngineEntry,
    allowed_hosts: &[String],
) -> Result<(), PluginRegistryError> {
    validate_remote_url(&entry.id, &entry.download_url, allowed_hosts)?;
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

fn validate_model_entry(
    entry: &ModelEntry,
    allowed_hosts: &[String],
) -> Result<(), PluginRegistryError> {
    validate_remote_url(&entry.name, &entry.download_url, allowed_hosts)?;
    for extra in &entry.extra_files {
        validate_remote_url(&entry.name, extra, allowed_hosts)?;
    }
    Ok(())
}

/// A polled registry entry's URL must be `https://` (never `file://` — a remote
/// response has no legitimate reason to name a path on the polling machine) and
/// resolve to an allowlisted host, so a compromised registry response can't redirect
/// a download at arbitrary attacker infrastructure.
fn validate_remote_url(
    entry_name: &str,
    raw_url: &str,
    allowed_hosts: &[String],
) -> Result<(), PluginRegistryError> {
    let parsed =
        url::Url::parse(raw_url).map_err(|err| PluginRegistryError::RejectedRemoteEntry {
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

    if !allowed_hosts.iter().any(|h| h == host) {
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
    use super::{apply_remote_registry, RegistrySignatures};
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

    fn allowed_hosts() -> Vec<String> {
        vec!["github.com".to_string(), "huggingface.co".to_string()]
    }

    #[test]
    fn accepts_and_persists_a_registry_with_only_allowlisted_hosts() {
        let dir = temp_registry_dir("ok");
        apply_remote_registry(
            &dir,
            good_engines_json(),
            good_models_json(),
            &allowed_hosts(),
            None,
        )
        .unwrap();

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

        let err = apply_remote_registry(
            &dir,
            malicious_engines,
            good_models_json(),
            &allowed_hosts(),
            None,
        )
        .unwrap_err();
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

        let err = apply_remote_registry(
            &dir,
            good_engines_json(),
            sneaky_models,
            &allowed_hosts(),
            None,
        )
        .unwrap_err();
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

        let err =
            apply_remote_registry(&dir, downgraded, good_models_json(), &allowed_hosts(), None)
                .unwrap_err();
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

        let err = apply_remote_registry(
            &dir,
            malicious_engines,
            good_models_json(),
            &allowed_hosts(),
            None,
        )
        .unwrap_err();
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

        let err = apply_remote_registry(
            &dir,
            good_engines_json(),
            sneaky_models,
            &allowed_hosts(),
            None,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            PluginRegistryError::RejectedRemoteEntry { .. }
        ));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_allowlist_is_genuinely_caller_configurable() {
        let dir = temp_registry_dir("custom-allowlist");
        let custom_host_engines = r#"[{"id":"llama-cpp","version":"1","platform":"windows-x64","download_url":"https://example.internal/llama.zip","sha256":null,"library_file":"llama.dll"}]"#;

        // Rejected against the default allowlist...
        let err = apply_remote_registry(
            &dir,
            custom_host_engines,
            good_models_json(),
            &allowed_hosts(),
            None,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            PluginRegistryError::RejectedRemoteEntry { .. }
        ));

        // ...but accepted once the caller adds that host to its own allowlist, proving
        // this isn't a hardcoded constant anymore.
        let custom_allowlist = vec!["example.internal".to_string(), "huggingface.co".to_string()];
        apply_remote_registry(
            &dir,
            custom_host_engines,
            good_models_json(),
            &custom_allowlist,
            None,
        )
        .unwrap();

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_second_good_poll_replaces_the_first() {
        let dir = temp_registry_dir("replace");
        apply_remote_registry(
            &dir,
            good_engines_json(),
            good_models_json(),
            &allowed_hosts(),
            None,
        )
        .unwrap();

        let updated_engines = r#"[{"id":"llama-cpp","version":"2","platform":"windows-x64","download_url":"https://github.com/ggml-org/llama.cpp/releases/download/v2/llama.zip","sha256":null,"library_file":"llama.dll"}]"#;
        apply_remote_registry(
            &dir,
            updated_engines,
            good_models_json(),
            &allowed_hosts(),
            None,
        )
        .unwrap();

        let engines = load_engine_entries(&dir).unwrap();
        assert_eq!(engines[0].version, "2");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_poll_with_valid_signatures_is_accepted() {
        use ed25519_dalek::{Signer, SigningKey};

        let signing_key = SigningKey::from_bytes(&[3u8; 32]);
        let public_key_hex = hex::encode(signing_key.verifying_key().to_bytes());
        let engines_signature_hex =
            hex::encode(signing_key.sign(good_engines_json().as_bytes()).to_bytes());
        let models_signature_hex =
            hex::encode(signing_key.sign(good_models_json().as_bytes()).to_bytes());

        let dir = temp_registry_dir("signed-ok");
        apply_remote_registry(
            &dir,
            good_engines_json(),
            good_models_json(),
            &allowed_hosts(),
            Some(RegistrySignatures {
                public_key_hex: &public_key_hex,
                engines_signature_hex: &engines_signature_hex,
                models_signature_hex: &models_signature_hex,
            }),
        )
        .unwrap();

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_poll_with_an_invalid_signature_is_rejected_and_writes_nothing() {
        use ed25519_dalek::SigningKey;

        let signing_key = SigningKey::from_bytes(&[3u8; 32]);
        let public_key_hex = hex::encode(signing_key.verifying_key().to_bytes());

        let dir = temp_registry_dir("signed-bad");
        let err = apply_remote_registry(
            &dir,
            good_engines_json(),
            good_models_json(),
            &allowed_hosts(),
            Some(RegistrySignatures {
                public_key_hex: &public_key_hex,
                engines_signature_hex: &"00".repeat(64),
                models_signature_hex: &"00".repeat(64),
            }),
        )
        .unwrap_err();
        assert!(matches!(err, PluginRegistryError::InvalidSignature { .. }));
        assert!(!dir.join("engines.json").exists());

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
