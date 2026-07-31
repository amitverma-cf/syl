use std::path::Path;

use crate::keys::set_api_key;

#[derive(Debug, thiserror::Error)]
pub enum CustomProviderError {
    #[error("network error fetching model list: {0}")]
    Network(#[from] Box<ureq::Error>),
    #[error("failed to parse model list response: {0}")]
    Parse(#[from] std::io::Error),
    #[error("response from {0}/models did not contain a usable model list")]
    NoModels(String),
    #[error("failed to read or write provider config: {0}")]
    Storage(#[from] serde_json::Error),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomProviderConfig {
    pub name: String,
    pub base_url: String,
    pub env_var: String,
    pub models: Vec<String>,
}

fn load_custom_providers(path: &Path) -> Vec<CustomProviderConfig> {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&contents).unwrap_or_default()
}

fn save_custom_providers(
    path: &Path,
    providers: &[CustomProviderConfig],
) -> Result<(), CustomProviderError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(providers)?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn list_custom_providers(path: &Path) -> Vec<CustomProviderConfig> {
    load_custom_providers(path)
}

fn env_var_name_for(provider_name: &str) -> String {
    let sanitized: String = provider_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    format!("CUSTOM_{sanitized}_API_KEY")
}

pub fn add_custom_provider(
    providers_path: &Path,
    env_path: &Path,
    name: &str,
    base_url: &str,
    api_key: Option<&str>,
) -> Result<CustomProviderConfig, CustomProviderError> {
    let base_url = base_url.trim_end_matches('/').to_string();
    let models = fetch_models(&base_url, api_key)?;
    let env_var = env_var_name_for(name);

    if let Some(key) = api_key.filter(|k| !k.is_empty()) {
        set_api_key(env_path, &env_var, key)?;
    }

    let config = CustomProviderConfig {
        name: name.to_string(),
        base_url,
        env_var,
        models,
    };

    let mut providers = load_custom_providers(providers_path);
    providers.retain(|p| p.name != config.name);
    providers.push(config.clone());
    save_custom_providers(providers_path, &providers)?;

    Ok(config)
}

fn fetch_models(base_url: &str, api_key: Option<&str>) -> Result<Vec<String>, CustomProviderError> {
    let url = format!("{base_url}/models");
    let mut request = ureq::get(&url);
    if let Some(key) = api_key.filter(|k| !k.is_empty()) {
        request = request.set("Authorization", &format!("Bearer {key}"));
    }
    let response = request.call().map_err(Box::new)?;
    let text = response.into_string()?;
    let body: serde_json::Value = serde_json::from_str(&text)?;

    let ids: Vec<String> = body
        .get("data")
        .and_then(|d| d.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("id").and_then(|id| id.as_str()))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    if ids.is_empty() {
        return Err(CustomProviderError::NoModels(base_url.to_string()));
    }
    Ok(ids)
}
