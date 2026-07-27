//! One-time setup that seeds the local `.syl` workspace (engines, models, registry) the first
//! time the app runs, from whatever the repo-relative dev registry currently points at.

use std::path::{Path, PathBuf};

use core_types::workspace_paths;
use plugin_registry::{EngineEntry, ModelEntry};

/// Populates `.syl/engines/`, `.syl/models/`, and `.syl/registry/` from the repo-relative
/// `registry/` folder (including any local dev overrides) if `.syl/registry/engines.json`
/// doesn't already exist. A no-op on subsequent runs.
pub fn ensure_workspace_seeded() {
    let registry_dir = workspace_paths::registry_dir();
    if registry_dir.join("engines.json").exists() {
        return;
    }

    tracing::info!(dir = %workspace_paths::workspace_root().display(), "seeding local .syl workspace");

    let source_registry_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../registry");
    let source_engines =
        plugin_registry::load_engine_entries(&source_registry_dir).unwrap_or_default();
    let source_models =
        plugin_registry::load_model_entries(&source_registry_dir).unwrap_or_default();

    let seeded_engines: Vec<EngineEntry> =
        source_engines.into_iter().filter_map(seed_engine).collect();
    let seeded_models: Vec<ModelEntry> = source_models.into_iter().filter_map(seed_model).collect();

    if let Err(err) = std::fs::create_dir_all(&registry_dir) {
        tracing::error!(?err, "failed to create .syl/registry");
        return;
    }
    write_json(&registry_dir.join("engines.json"), &seeded_engines);
    write_json(&registry_dir.join("models.json"), &seeded_models);
}

fn seed_engine(entry: EngineEntry) -> Option<EngineEntry> {
    let source_path = match plugin_registry::resolve_local_path(&entry.download_url) {
        Ok(path) => path,
        Err(err) => {
            tracing::warn!(?err, engine = %entry.id, "skipping engine seed, source not resolvable");
            return None;
        }
    };
    let source_dir = source_path.parent()?;
    let dest_dir = workspace_paths::engines_dir().join(&entry.id);

    if let Err(err) = copy_dir_files(source_dir, &dest_dir) {
        tracing::error!(?err, engine = %entry.id, "failed to copy engine files");
        return None;
    }

    let dest_path = dest_dir.join(source_path.file_name()?);
    Some(EngineEntry {
        download_url: file_url(&dest_path),
        ..entry
    })
}

fn seed_model(entry: ModelEntry) -> Option<ModelEntry> {
    let source_path = match plugin_registry::resolve_local_path(&entry.huggingface_url) {
        Ok(path) => path,
        Err(err) => {
            tracing::warn!(?err, model = %entry.name, "skipping model seed, source not resolvable");
            return None;
        }
    };
    let dest_dir = workspace_paths::models_dir();
    if let Err(err) = std::fs::create_dir_all(&dest_dir) {
        tracing::error!(?err, "failed to create .syl/models");
        return None;
    }
    let dest_path = dest_dir.join(source_path.file_name()?);
    if !dest_path.exists() {
        if let Err(err) = std::fs::copy(&source_path, &dest_path) {
            tracing::error!(?err, model = %entry.name, "failed to copy model file");
            return None;
        }
    }

    Some(ModelEntry {
        huggingface_url: file_url(&dest_path),
        ..entry
    })
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
