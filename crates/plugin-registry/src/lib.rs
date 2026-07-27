//! Loads the engine plugin and model catalog listings, and resolves each listing's
//! download URL to a location the rest of the app can read from.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// An error returned by this crate.
#[derive(Debug, thiserror::Error)]
pub enum PluginRegistryError {
    /// A registry file could not be read from disk.
    #[error("failed to read registry file {path}: {source}")]
    Io {
        /// The file that could not be read.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// A registry file's contents were not valid JSON, or did not match the expected shape.
    #[error("failed to parse registry file {path}: {source}")]
    Parse {
        /// The file that failed to parse.
        path: PathBuf,
        /// The underlying parse error.
        #[source]
        source: serde_json::Error,
    },
    /// A download URL did not use a supported scheme (`file://`, `http://`, `https://`).
    #[error("unsupported or malformed download URL: {0}")]
    InvalidUrl(String),
    /// A `file://` URL pointed at a path that does not exist.
    #[error("local file does not exist: {}", .0.display())]
    LocalFileMissing(PathBuf),
    /// A `http://`/`https://` URL was resolved, but downloading it is not implemented yet.
    #[error("remote downloads are not implemented yet: {0}")]
    RemoteNotImplemented(String),
}

/// One entry in the engine plugin listing: a downloadable build of an inference engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineEntry {
    /// The engine's unique id.
    pub id: String,
    /// The version of this build.
    pub version: String,
    /// The target platform this build runs on (e.g. `windows-x64-cuda`).
    pub platform: String,
    /// Where to obtain this build: a `file://` path for local development, or an
    /// `http://`/`https://` URL once builds are hosted.
    pub download_url: String,
    /// SHA-256 hash of the downloaded file, for integrity verification. Not required for
    /// `file://` sources, since those are already local and trusted.
    pub sha256: Option<String>,
}

/// One entry in the model catalog: a model available to download and run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    /// The model's display name.
    pub name: String,
    /// The download size, in bytes.
    pub size_bytes: u64,
    /// The quantization variant of this listing (e.g. `Q4_K_M`).
    pub quantization: String,
    /// The id of the engine required to run this model.
    pub required_engine: String,
    /// Where to obtain this model: a `file://` path for local development, or a Hugging Face
    /// `https://` URL once models are catalogued for download.
    pub huggingface_url: String,
    /// SHA-256 hash of the downloaded file, for integrity verification. Not required for
    /// `file://` sources, since those are already local and trusted.
    pub sha256: Option<String>,
}

/// Where a resolved download URL points to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadSource {
    /// The file already exists at this path on the local filesystem.
    Local(PathBuf),
    /// The file must be downloaded from this URL.
    Remote(String),
}

/// Loads `engines.json` from `registry_dir`, merged with `local.engines.json` if present.
///
/// # Errors
/// Returns an error if `engines.json` is missing or fails to parse, or if a present
/// `local.engines.json` fails to parse.
pub fn load_engine_entries(registry_dir: &Path) -> Result<Vec<EngineEntry>, PluginRegistryError> {
    let mut entries = load_json_array(&registry_dir.join("engines.json"))?;
    let local_path = registry_dir.join("local.engines.json");
    if local_path.exists() {
        entries.extend(load_json_array::<EngineEntry>(&local_path)?);
    }
    Ok(entries)
}

/// Loads `models.json` from `registry_dir`, merged with `local.models.json` if present.
///
/// # Errors
/// Returns an error if `models.json` is missing or fails to parse, or if a present
/// `local.models.json` fails to parse.
pub fn load_model_entries(registry_dir: &Path) -> Result<Vec<ModelEntry>, PluginRegistryError> {
    let mut entries = load_json_array(&registry_dir.join("models.json"))?;
    let local_path = registry_dir.join("local.models.json");
    if local_path.exists() {
        entries.extend(load_json_array::<ModelEntry>(&local_path)?);
    }
    Ok(entries)
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

/// Resolves a download URL to either a local path (`file://`) or a remote URL
/// (`http://`/`https://`).
///
/// # Errors
/// Returns [`PluginRegistryError::InvalidUrl`] if `url` uses an unsupported scheme, or
/// [`PluginRegistryError::LocalFileMissing`] if a `file://` URL points at a path that does
/// not exist.
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

/// Resolves `url` to a local path ready to use immediately.
///
/// # Errors
/// Returns an error if `url` does not resolve to a local file, or if it resolves to a remote
/// URL (remote downloading is not implemented yet).
pub fn resolve_local_path(url: &str) -> Result<PathBuf, PluginRegistryError> {
    match resolve_download_url(url)? {
        DownloadSource::Local(path) => Ok(path),
        DownloadSource::Remote(url) => Err(PluginRegistryError::RemoteNotImplemented(url)),
    }
}

/// Converts the path component of a `file://` URL to a native filesystem path string.
fn file_url_path_to_native(raw_path: &str) -> String {
    let trimmed = raw_path.strip_prefix('/').unwrap_or(raw_path);
    // A Windows drive-letter path (e.g. "C:/Users/...") must not keep its leading slash;
    // any other absolute path must.
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
