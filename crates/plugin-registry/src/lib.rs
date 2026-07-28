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
    #[error("remote downloads are not implemented yet: {0}")]
    RemoteNotImplemented(String),
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
    let path = dir.join("local.engines.json");
    if path.exists() {
        load_json_array(&path)
    } else {
        Ok(Vec::new())
    }
}

pub fn load_local_model_entries(dir: &Path) -> Result<Vec<ModelEntry>, PluginRegistryError> {
    let path = dir.join("local.models.json");
    if path.exists() {
        load_json_array(&path)
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

pub fn resolve_local_path(url: &str) -> Result<PathBuf, PluginRegistryError> {
    match resolve_download_url(url)? {
        DownloadSource::Local(path) => Ok(path),
        DownloadSource::Remote(url) => Err(PluginRegistryError::RemoteNotImplemented(url)),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_windows_file_url_to_native_path() {
        assert_eq!(
            file_url_path_to_native("/C:/Users/amit/model.gguf"),
            "C:/Users/amit/model.gguf"
        );
    }

    #[test]
    fn resolves_unix_file_url_to_native_path() {
        assert_eq!(
            file_url_path_to_native("/home/amit/model.gguf"),
            "/home/amit/model.gguf"
        );
    }

    #[test]
    fn resolve_download_url_accepts_remote_https() {
        let resolved = resolve_download_url("https://example.com/model.gguf").unwrap();
        assert_eq!(
            resolved,
            DownloadSource::Remote("https://example.com/model.gguf".to_string())
        );
    }

    #[test]
    fn resolve_download_url_rejects_unsupported_scheme() {
        let err = resolve_download_url("ftp://example.com/model.gguf").unwrap_err();
        assert!(matches!(err, PluginRegistryError::InvalidUrl(_)));
    }

    #[test]
    fn resolve_download_url_errors_on_missing_local_file() {
        let err = resolve_download_url("file:///C:/definitely/does/not/exist.gguf").unwrap_err();
        assert!(matches!(err, PluginRegistryError::LocalFileMissing(_)));
    }

    #[test]
    fn resolve_download_url_finds_existing_local_file() {
        let tmp = std::env::temp_dir().join("syl-plugin-registry-test-file.txt");
        std::fs::write(&tmp, b"test").unwrap();
        let url = format!("file://{}", tmp.to_string_lossy().replace('\\', "/"));
        let resolved = resolve_download_url(&url).unwrap();
        assert!(matches!(resolved, DownloadSource::Local(_)));
        std::fs::remove_file(&tmp).unwrap();
    }

    #[test]
    fn resolve_local_path_errors_on_remote_url() {
        let err = resolve_local_path("https://example.com/model.gguf").unwrap_err();
        assert!(matches!(err, PluginRegistryError::RemoteNotImplemented(_)));
    }

    #[test]
    fn load_engine_entries_merges_local_override() {
        let dir =
            std::env::temp_dir().join(format!("syl-plugin-registry-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("engines.json"), "[]").unwrap();
        std::fs::write(
            dir.join("local.engines.json"),
            r#"[{"id":"llama-cpp","version":"local-dev","platform":"windows-x64","download_url":"file:///C:/does/not/matter.dll","sha256":null}]"#,
        )
        .unwrap();

        let entries = load_engine_entries(&dir).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "llama-cpp");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn load_engine_entries_errors_on_malformed_json() {
        let dir = std::env::temp_dir().join(format!(
            "syl-plugin-registry-test-malformed-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("engines.json"), "not valid json").unwrap();

        let err = load_engine_entries(&dir).unwrap_err();
        assert!(matches!(err, PluginRegistryError::Parse { .. }));

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
