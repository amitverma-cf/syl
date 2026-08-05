use cloud_providers::{CloudModel, CustomProviderConfig, ProviderInfo};
use syl_core::workspace_paths;

#[tauri::command]
pub fn list_providers() -> Vec<ProviderInfo> {
    cloud_providers::list_providers(&workspace_paths::env_file())
}

#[tauri::command]
pub fn set_provider_api_key(env_var: String, key: String) -> Result<(), String> {
    cloud_providers::set_api_key(&workspace_paths::env_file(), &env_var, &key)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_provider_api_key(env_var: String) -> Result<(), String> {
    cloud_providers::delete_api_key(&workspace_paths::env_file(), &env_var)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_cloud_models() -> Vec<CloudModel> {
    cloud_providers::list_all_models(&workspace_paths::custom_providers_file())
}

#[tauri::command]
pub fn list_custom_providers() -> Vec<CustomProviderConfig> {
    cloud_providers::list_custom_providers(&workspace_paths::custom_providers_file())
}

#[tauri::command]
pub async fn add_custom_provider(
    name: String,
    base_url: String,
    api_key: Option<String>,
) -> Result<CustomProviderConfig, String> {
    tauri::async_runtime::spawn_blocking(move || {
        cloud_providers::add_custom_provider(
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
        cloud_providers::update_custom_provider(
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
    cloud_providers::remove_custom_provider(
        &workspace_paths::custom_providers_file(),
        &workspace_paths::env_file(),
        &name,
    )
    .map_err(|e| e.to_string())
}
