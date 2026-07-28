use std::path::{Path, PathBuf};

use crate::{
    load_engine_entries, load_model_entries, resolve_local_path, ModelKind, PluginRegistryError,
};

#[derive(Debug)]
pub struct ResolvedModel {
    pub model_name: String,
    pub model_path: PathBuf,
    pub engine_id: String,
    pub engine_library_path: PathBuf,
}

pub fn resolve_model_for_kind(
    registry_dir: &Path,
    models_cache_dir: &Path,
    engines_cache_dir: &Path,
    kind: ModelKind,
) -> Result<ResolvedModel, PluginRegistryError> {
    let models = load_model_entries(registry_dir)?;
    let model_entry = models
        .into_iter()
        .find(|m| m.kind == kind)
        .ok_or(PluginRegistryError::NoModelAvailable(kind))?;

    let engines = load_engine_entries(registry_dir)?;
    let engine_entry = engines
        .into_iter()
        .find(|e| e.id == model_entry.required_engine)
        .ok_or_else(|| PluginRegistryError::NoEngineAvailable {
            model: model_entry.name.clone(),
            engine: model_entry.required_engine.clone(),
        })?;

    let model_path = resolve_local_path(&model_entry.download_url, models_cache_dir)?;
    let engine_library_path = resolve_local_path(&engine_entry.download_url, engines_cache_dir)?;

    Ok(ResolvedModel {
        model_name: model_entry.name,
        model_path,
        engine_id: engine_entry.id,
        engine_library_path,
    })
}
