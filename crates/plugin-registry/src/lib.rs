mod fetch;
mod resolve;

pub use fetch::{download_to_cache, fetch_remote_registry, is_cached};
pub use resolve::{resolve_model_for_kind, ResolvedModel};

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum PluginRegistryError {
    #[error("failed to read registry file {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse registry file {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("unsupported or malformed download URL: {0}")]
    InvalidUrl(String),
    #[error("local file does not exist: {}", .0.display())]
    LocalFileMissing(PathBuf),
    #[error("no {0:?} model is available; download one from the model catalog")]
    NoModelAvailable(ModelKind),
    #[error("model {model} requires engine {engine}, which is not available")]
    NoEngineAvailable { model: String, engine: String },
    #[error("network error fetching {url}: {source}")]
    Network {
        url: String,
        #[source]
        source: std::io::Error,
    },
    #[error("http error {status} fetching {url}")]
    Http { url: String, status: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelKind {
    Chat,
    Embedding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineEntry {
    pub id: String,
    pub version: String,
    pub platform: String,
    pub download_url: String,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub name: String,
    pub kind: ModelKind,
    pub size_bytes: u64,
    pub quantization: String,
    pub required_engine: String,
    pub download_url: String,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadSource {
    Local(PathBuf),
    Remote(String),
}

pub fn load_engine_entries(registry_dir: &Path) -> Result<Vec<EngineEntry>, PluginRegistryError> {
    let mut entries = load_json_array(&registry_dir.join("engines.json"))?;
    entries.extend(load_local_engine_entries(registry_dir)?);
    Ok(entries)
}

pub fn load_model_entries(registry_dir: &Path) -> Result<Vec<ModelEntry>, PluginRegistryError> {
    let mut entries = load_json_array(&registry_dir.join("models.json"))?;
    entries.extend(load_local_model_entries(registry_dir)?);
    Ok(entries)
}

pub fn load_local_engine_entries(dir: &Path) -> Result<Vec<EngineEntry>, PluginRegistryError> {
    load_optional_json_array(&dir.join("local.engines.json"))
}

pub fn load_local_model_entries(dir: &Path) -> Result<Vec<ModelEntry>, PluginRegistryError> {
    load_optional_json_array(&dir.join("local.models.json"))
}

fn load_optional_json_array<T: for<'de> Deserialize<'de>>(
    path: &Path,
) -> Result<Vec<T>, PluginRegistryError> {
    if path.exists() {
        load_json_array(path)
    } else {
        Ok(Vec::new())
    }
}

fn load_json_array<T: for<'de> Deserialize<'de>>(
    path: &Path,
) -> Result<Vec<T>, PluginRegistryError> {
    let contents = std::fs::read_to_string(path).map_err(|source| PluginRegistryError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&contents).map_err(|source| PluginRegistryError::Parse {
        path: path.to_path_buf(),
        source,
    })
}

pub fn resolve_download_url(url: &str) -> Result<DownloadSource, PluginRegistryError> {
    if let Some(raw_path) = url.strip_prefix("file://") {
        let path = PathBuf::from(file_url_path_to_native(raw_path));
        if !path.exists() {
            return Err(PluginRegistryError::LocalFileMissing(path));
        }
        Ok(DownloadSource::Local(path))
    } else if url.starts_with("http://") || url.starts_with("https://") {
        Ok(DownloadSource::Remote(url.to_string()))
    } else {
        Err(PluginRegistryError::InvalidUrl(url.to_string()))
    }
}

pub fn resolve_local_path(url: &str, cache_dir: &Path) -> Result<PathBuf, PluginRegistryError> {
    match resolve_download_url(url)? {
        DownloadSource::Local(path) => Ok(path),
        DownloadSource::Remote(url) => download_to_cache(&url, cache_dir),
    }
}

fn file_url_path_to_native(raw_path: &str) -> String {
    let trimmed = raw_path.strip_prefix('/').unwrap_or(raw_path);
    if trimmed.as_bytes().get(1) == Some(&b':') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}
