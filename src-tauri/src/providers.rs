use core_types::workspace_paths;
use provider::{CloudModel, CustomProviderConfig, ProviderInfo};

#[tauri::command]
pub fn list_providers() -> Vec<ProviderInfo> {
    provider::list_providers(&workspace_paths::env_file())
}

#[tauri::command]
pub fn set_provider_api_key(env_var: String, key: String) -> Result<(), String> {
    provider::set_api_key(&workspace_paths::env_file(), &env_var, &key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_provider_api_key(env_var: String) -> Result<(), String> {
    provider::delete_api_key(&workspace_paths::env_file(), &env_var).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_cloud_models() -> Vec<CloudModel> {
    provider::list_all_models(&workspace_paths::custom_providers_file())
}

#[tauri::command]
pub fn list_custom_providers() -> Vec<CustomProviderConfig> {
    provider::list_custom_providers(&workspace_paths::custom_providers_file())
}

#[tauri::command]
pub async fn add_custom_provider(
    name: String,
    base_url: String,
    api_key: Option<String>,
) -> Result<CustomProviderConfig, String> {
    tauri::async_runtime::spawn_blocking(move || {
        provider::add_custom_provider(
            &workspace_paths::custom_providers_file(),
            &workspace_paths::env_file(),
            &name,
            &base_url,
            api_key.as_deref(),
        )
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn update_custom_provider(
    name: String,
    base_url: String,
    api_key: Option<String>,
) -> Result<CustomProviderConfig, String> {
    tauri::async_runtime::spawn_blocking(move || {
        provider::update_custom_provider(
            &workspace_paths::custom_providers_file(),
            &workspace_paths::env_file(),
            &name,
            &base_url,
            api_key.as_deref(),
        )
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn remove_custom_provider(name: String) -> Result<(), String> {
    provider::remove_custom_provider(
        &workspace_paths::custom_providers_file(),
        &workspace_paths::env_file(),
        &name,
    )
    .map_err(|e| e.to_string())
}
