use std::path::{Path, PathBuf};

use core_types::workspace_paths;
use plugin_registry::{DownloadSource, EngineEntry, ModelEntry};

pub fn ensure_workspace_seeded() {
    migrate_legacy_workspace();
    seed_flows();
    seed_extensions();

    let syl_registry_dir = workspace_paths::registry_dir();
    let engines_seeded = syl_registry_dir.join("engines.json").exists();
    let models_seeded = syl_registry_dir.join("models.json").exists();
    if engines_seeded && models_seeded {
        return;
    }

    tracing::info!(dir = %workspace_paths::workspace_root().display(), "seeding local .syl workspace");

    let repo_registry_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../registry");
    let base_engines = plugin_registry::load_engine_entries(&repo_registry_dir).unwrap_or_default();
    let base_models = plugin_registry::load_model_entries(&repo_registry_dir).unwrap_or_default();
    let local_engines =
        plugin_registry::load_local_engine_entries(&syl_registry_dir).unwrap_or_default();
    let local_models =
        plugin_registry::load_local_model_entries(&syl_registry_dir).unwrap_or_default();

    let seeded_engines: Vec<EngineEntry> = base_engines
        .into_iter()
        .chain(local_engines)
        .filter_map(seed_engine)
        .collect();
    let seeded_models: Vec<ModelEntry> = base_models
        .into_iter()
        .chain(local_models)
        .filter_map(seed_model)
        .collect();

    if let Err(err) = std::fs::create_dir_all(&syl_registry_dir) {
        tracing::error!(?err, "failed to create .syl/registry");
        return;
    }
    write_json(&syl_registry_dir.join("engines.json"), &seeded_engines);
    write_json(&syl_registry_dir.join("models.json"), &seeded_models);
}

/// One-time migration off the pre-OS-app-data-dir workspace location
/// (`<repo>/.syl`) into the real per-user app-data directory
/// `workspace_root()` now resolves to. A no-op whenever `SYL_WORKSPACE_DIR`
/// is set (E2E test isolation has no "legacy" concept to migrate from), the
/// new location already has data (either a previous migration already ran,
/// or this is a genuinely fresh install with nothing to migrate), or no
/// legacy directory exists (a fresh install, or an installed build whose
/// `CARGO_MANIFEST_DIR`-derived "repo root" never existed on this machine).
fn migrate_legacy_workspace() {
    if std::env::var("SYL_WORKSPACE_DIR").is_ok() {
        return;
    }
    let new_root = workspace_paths::workspace_root();
    let legacy_root = workspace_paths::legacy_repo_workspace_root();
    if new_root == legacy_root || new_root.exists() || !legacy_root.exists() {
        return;
    }

    tracing::info!(
        from = %legacy_root.display(),
        to = %new_root.display(),
        "migrating workspace to the OS-idiomatic app-data directory"
    );

    if let Some(parent) = new_root.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            tracing::error!(
                ?err,
                "failed to create parent of the new workspace directory"
            );
            return;
        }
    }

    if let Err(err) = migrate_dir(&legacy_root, &new_root) {
        tracing::error!(?err, "failed to migrate legacy workspace");
    }
}

/// Moves everything from `from` into `to`. Tries a plain rename first — atomic
/// and instant when both paths are on the same volume (the common case, both
/// under the same user profile drive) — and only falls back to a recursive
/// copy-then-remove when that fails (e.g. the two locations happen to be on
/// different volumes).
fn migrate_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    copy_dir_recursive(from, to)?;
    std::fs::remove_dir_all(from)
}

fn copy_dir_recursive(source_dir: &Path, dest_dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dest_dir)?;
    for entry in std::fs::read_dir(source_dir)? {
        let entry = entry?;
        let dest = dest_dir.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &dest)?;
        } else {
            std::fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}

/// Writes every backend-worker extension's manifest every launch (not a
/// once-only seed like the registry/flows below) — each `backend.command`
/// path depends on where *this* run's own executable lives (dev build
/// output dir vs. a packaged sidecar location), which can change between
/// builds, so it needs to self-heal on every startup rather than go stale
/// after the first run.
fn seed_extensions() {
    seed_worker_extension(
        "llama-cpp-chat",
        "llama.cpp Chat Engine",
        "engine-worker",
        vec!["inference.chat/v1".to_string()],
    );
    seed_worker_extension(
        "stable-diffusion-image",
        "Stable Diffusion Image Generator",
        "sd-worker",
        vec!["image.generate/v1".to_string()],
    );
    seed_worker_extension(
        "onnx-embedding",
        "ONNX Embedding Engine",
        "embedding-worker",
        vec!["embedding.embed/v1".to_string()],
    );
    seed_worker_extension(
        "onnx-asr",
        "ONNX Speech-to-Text Engine",
        "asr-worker",
        vec!["asr.transcribe/v1".to_string()],
    );
    seed_worker_extension(
        "onnx-tts",
        "ONNX Text-to-Speech Engine",
        "tts-worker",
        vec!["tts.synthesize/v1".to_string()],
    );
    seed_flow_editor_extension();
}

fn seed_worker_extension(id: &str, display_name: &str, binary_name: &str, provides: Vec<String>) {
    let Ok(worker_path) = crate::local_models::resolve_worker_binary_path(binary_name) else {
        tracing::warn!(%id, "could not resolve the {binary_name} binary path; this extension will not load");
        return;
    };

    let manifest = extension_host::ExtensionManifest {
        id: id.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        display_name: display_name.to_string(),
        backend: Some(extension_host::ExtensionBackend {
            command: worker_path.display().to_string(),
            args: Vec::new(),
        }),
        provides,
        requires: Vec::new(),
        contributes: None,
    };
    write_extension_manifest(id, &manifest);
}

/// The Flow Editor is a UI-only extension: pure in-memory host-side logic
/// with no FFI/crash risk to isolate, so it declares no `backend` — it only
/// contributes a sidebar entry the host's generic contribution renderer
/// picks up, instead of the entry point being hardcoded in the frontend.
fn seed_flow_editor_extension() {
    let manifest = extension_host::ExtensionManifest {
        id: "flow-editor".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        display_name: "Flows".to_string(),
        backend: None,
        provides: Vec::new(),
        requires: Vec::new(),
        contributes: Some(extension_host::Contributions {
            settings_pane: None,
            sidebar_view: Some(extension_host::UiContribution {
                id: "flow-editor".to_string(),
                title: "Flow Editor".to_string(),
            }),
            status_bar_item: None,
            commands: vec![extension_host::UiContribution {
                id: "open-flow-editor".to_string(),
                title: "Open Flow Editor".to_string(),
            }],
        }),
    };
    write_extension_manifest("flow-editor", &manifest);
}

fn write_extension_manifest(id: &str, manifest: &extension_host::ExtensionManifest) {
    let dir = workspace_paths::extensions_dir().join(id);
    if let Err(err) = std::fs::create_dir_all(&dir) {
        tracing::error!(?err, %id, "failed to create .syl/extensions/{id}");
        return;
    }
    let Ok(json) = serde_json::to_string_pretty(manifest) else {
        return;
    };
    if let Err(err) = std::fs::write(dir.join("manifest.json"), json) {
        tracing::error!(?err, %id, "failed to write extension manifest");
    }
}

fn seed_flows() {
    let repo_flows_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../flows");
    let dest_dir = workspace_paths::flows_dir();
    if let Err(err) = copy_dir_files(&repo_flows_dir, &dest_dir) {
        tracing::warn!(?err, "failed to seed .syl/flows");
    }
}

fn seed_engine(entry: EngineEntry) -> Option<EngineEntry> {
    let dest_dir = workspace_paths::engines_dir().join(&entry.id);
    let dest_path = seed_into(&entry.download_url, &dest_dir, entry.sha256.as_deref())?;
    Some(EngineEntry {
        download_url: file_url(&dest_path),
        ..entry
    })
}

fn seed_model(entry: ModelEntry) -> Option<ModelEntry> {
    let dest_dir = workspace_paths::models_dir();
    let dest_path = seed_into(&entry.download_url, &dest_dir, entry.sha256.as_deref())?;
    Some(ModelEntry {
        download_url: file_url(&dest_path),
        ..entry
    })
}

fn seed_into(
    download_url: &str,
    dest_dir: &Path,
    expected_sha256: Option<&str>,
) -> Option<PathBuf> {
    match plugin_registry::resolve_download_url(download_url) {
        Ok(DownloadSource::Local(source_path)) => {
            let source_dir = source_path.parent()?;
            copy_dir_files(source_dir, dest_dir).ok()?;
            Some(dest_dir.join(source_path.file_name()?))
        }
        Ok(DownloadSource::Remote(url)) => {
            plugin_registry::download_to_cache(&url, dest_dir, expected_sha256).ok()
        }
        Err(err) => {
            tracing::warn!(?err, url = %download_url, "skipping seed, source not resolvable");
            None
        }
    }
}

fn copy_dir_files(source_dir: &Path, dest_dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dest_dir)?;
    for entry in std::fs::read_dir(source_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let dest = dest_dir.join(entry.file_name());
            if !dest.exists() {
                std::fs::copy(entry.path(), dest)?;
            }
        }
    }
    Ok(())
}

fn file_url(path: &Path) -> String {
    format!("file://{}", path.display().to_string().replace('\\', "/"))
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(json) => {
            if let Err(err) = std::fs::write(path, json) {
                tracing::error!(?err, path = %path.display(), "failed to write registry file");
            }
        }
        Err(err) => {
            tracing::error!(?err, path = %path.display(), "failed to serialize registry file")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "syl-bootstrap-test-{label}-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn migrate_dir_moves_nested_files_and_removes_the_source() {
        let from = temp_dir("migrate-from");
        std::fs::write(from.join("top.txt"), "top level").unwrap();
        std::fs::create_dir_all(from.join("nested")).unwrap();
        std::fs::write(from.join("nested").join("deep.txt"), "nested file").unwrap();

        let to = temp_dir("migrate-to");
        std::fs::remove_dir_all(&to).unwrap(); // migrate_dir creates the destination itself

        migrate_dir(&from, &to).unwrap();

        assert!(!from.exists());
        assert_eq!(
            std::fs::read_to_string(to.join("top.txt")).unwrap(),
            "top level"
        );
        assert_eq!(
            std::fs::read_to_string(to.join("nested").join("deep.txt")).unwrap(),
            "nested file"
        );

        std::fs::remove_dir_all(&to).ok();
    }

    #[test]
    fn copy_dir_recursive_copies_without_removing_the_source() {
        let from = temp_dir("copy-from");
        std::fs::write(from.join("file.txt"), "contents").unwrap();

        let to = temp_dir("copy-to");
        std::fs::remove_dir_all(&to).unwrap();

        copy_dir_recursive(&from, &to).unwrap();

        assert!(from.join("file.txt").exists());
        assert_eq!(
            std::fs::read_to_string(to.join("file.txt")).unwrap(),
            "contents"
        );

        std::fs::remove_dir_all(&from).ok();
        std::fs::remove_dir_all(&to).ok();
    }
}
