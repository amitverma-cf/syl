use core_types::workspace_paths;
use provider::{CloudModel, ProviderInfo};

#[tauri::command]
pub fn list_providers() -> Vec<ProviderInfo> {
    provider::list_providers(&workspace_paths::env_file())
}

#[tauri::command]
pub fn set_provider_api_key(env_var: String, key: String) -> Result<(), String> {
    provider::set_api_key(&workspace_paths::env_file(), &env_var, &key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_cloud_models() -> Vec<CloudModel> {
    provider::list_cloud_models()
}
