use std::collections::BTreeMap;
use std::path::Path;

use syl_core::app_config::app_config;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub name: String,
    pub env_var: String,
    pub configured: bool,
}

pub fn load_env_file(path: &Path) -> BTreeMap<String, String> {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    contents
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            Some((key.trim().to_string(), unquote(value.trim())))
        })
        .collect()
}

fn unquote(value: &str) -> String {
    let bytes = value.as_bytes();
    if bytes.len() >= 2 {
        let (first, last) = (bytes[0], bytes[bytes.len() - 1]);
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return value[1..value.len() - 1].to_string();
        }
    }
    value.to_string()
}

pub fn write_env_file(path: &Path, entries: &BTreeMap<String, String>) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let contents: String = entries
        .iter()
        .map(|(key, value)| format!("{key}={value}\n"))
        .collect();
    std::fs::write(path, contents)
}

pub fn set_api_key(path: &Path, env_var: &str, key: &str) -> std::io::Result<()> {
    let mut entries = load_env_file(path);
    entries.insert(env_var.to_string(), key.to_string());
    write_env_file(path, &entries)
}

pub fn delete_api_key(path: &Path, env_var: &str) -> std::io::Result<()> {
    let mut entries = load_env_file(path);
    entries.remove(env_var);
    write_env_file(path, &entries)
}

pub fn list_providers(path: &Path) -> Vec<ProviderInfo> {
    let entries = load_env_file(path);
    app_config()
        .known_cloud_providers
        .iter()
        .map(|provider| ProviderInfo {
            name: provider.name.clone(),
            env_var: provider.env_var.clone(),
            configured: entries
                .get(&provider.env_var)
                .is_some_and(|v| !v.is_empty()),
        })
        .collect()
}
