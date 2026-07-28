use std::path::{Path, PathBuf};

use core_types::workspace_paths;
use plugin_registry::{DownloadSource, EngineEntry, ModelEntry};

pub fn ensure_workspace_seeded() {
    let syl_registry_dir = workspace_paths::registry_dir();
    if syl_registry_dir.join("engines.json").exists() {
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

    let _ = std::fs::remove_file(syl_registry_dir.join("local.engines.json"));
    let _ = std::fs::remove_file(syl_registry_dir.join("local.models.json"));

    seed_flows();
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
    let dest_path = seed_into(&entry.download_url, &dest_dir)?;
    Some(EngineEntry {
        download_url: file_url(&dest_path),
        ..entry
    })
}

fn seed_model(entry: ModelEntry) -> Option<ModelEntry> {
    let dest_dir = workspace_paths::models_dir();
    let dest_path = seed_into(&entry.download_url, &dest_dir)?;
    Some(ModelEntry {
        download_url: file_url(&dest_path),
        ..entry
    })
}

fn seed_into(download_url: &str, dest_dir: &Path) -> Option<PathBuf> {
    match plugin_registry::resolve_download_url(download_url) {
        Ok(DownloadSource::Local(source_path)) => {
            let source_dir = source_path.parent()?;
            copy_dir_files(source_dir, dest_dir).ok()?;
            Some(dest_dir.join(source_path.file_name()?))
        }
        Ok(DownloadSource::Remote(url)) => plugin_registry::download_to_cache(&url, dest_dir).ok(),
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
