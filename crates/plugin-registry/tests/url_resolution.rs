use plugin_registry::{
    resolve_download_url, resolve_local_path, DownloadSource, PluginRegistryError,
};

#[test]
fn resolve_download_url_accepts_remote_https() {
    let resolved = resolve_download_url("https://example.invalid/file.bin").unwrap();
    assert_eq!(
        resolved,
        DownloadSource::Remote("https://example.invalid/file.bin".to_string())
    );
}

#[test]
fn resolve_download_url_rejects_unsupported_scheme() {
    let err = resolve_download_url("ftp://example.invalid/file.bin").unwrap_err();
    assert!(matches!(err, PluginRegistryError::InvalidUrl(_)));
}

#[test]
fn resolve_download_url_errors_on_missing_local_file() {
    let err = resolve_download_url("file:///C:/definitely/does/not/exist.bin").unwrap_err();
    assert!(matches!(err, PluginRegistryError::LocalFileMissing(_)));
}

#[test]
fn resolve_download_url_finds_existing_local_file() {
    let tmp = std::env::temp_dir().join("syl-plugin-registry-url-test.txt");
    std::fs::write(&tmp, b"test").unwrap();
    let url = format!("file://{}", tmp.to_string_lossy().replace('\\', "/"));
    let resolved = resolve_download_url(&url).unwrap();
    assert!(matches!(resolved, DownloadSource::Local(_)));
    std::fs::remove_file(&tmp).unwrap();
}

#[test]
fn resolve_local_path_returns_local_file_directly() {
    let tmp = std::env::temp_dir().join("syl-plugin-registry-local-path-test.txt");
    std::fs::write(&tmp, b"test").unwrap();
    let url = format!("file://{}", tmp.to_string_lossy().replace('\\', "/"));
    let cache_dir = std::env::temp_dir().join("syl-plugin-registry-cache-unused");

    let resolved = resolve_local_path(&url, &cache_dir, None).unwrap();
    assert_eq!(resolved, tmp);
    std::fs::remove_file(&tmp).unwrap();
}
