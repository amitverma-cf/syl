use core_types::workspace_paths;
use plugin_registry::{DownloadSource, ModelKind};

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogModel {
    pub name: String,
    pub kind: ModelKind,
    pub size_bytes: u64,
    pub quantization: String,
    pub required_engine: String,
    pub already_downloaded: bool,
    pub fits_in_available_memory: bool,
}

#[tauri::command]
pub fn list_available_models() -> Result<Vec<CatalogModel>, String> {
    let registry_dir = workspace_paths::registry_dir();
    let models = plugin_registry::load_model_entries(&registry_dir).map_err(|e| e.to_string())?;
    let available_memory = crate::observability::available_memory_bytes();
    let models_dir = workspace_paths::models_dir();

    Ok(models
        .into_iter()
        .map(|m| {
            let already_downloaded = match plugin_registry::resolve_download_url(&m.download_url) {
                Ok(DownloadSource::Local(_)) => true,
                Ok(DownloadSource::Remote(url)) => plugin_registry::is_cached(&url, &models_dir),
                Err(_) => false,
            };
            CatalogModel {
                fits_in_available_memory: m.size_bytes <= available_memory,
                already_downloaded,
                name: m.name,
                kind: m.kind,
                size_bytes: m.size_bytes,
                quantization: m.quantization,
                required_engine: m.required_engine,
            }
        })
        .collect())
}

#[tauri::command]
pub async fn download_model(name: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let registry_dir = workspace_paths::registry_dir();
        let models =
            plugin_registry::load_model_entries(&registry_dir).map_err(|e| e.to_string())?;
        let entry = models
            .into_iter()
            .find(|m| m.name == name)
            .ok_or_else(|| format!("no model named {name} in the registry"))?;
        plugin_registry::resolve_local_path(&entry.download_url, &workspace_paths::models_dir())
            .map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
