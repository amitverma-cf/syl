use plugin_registry::{verify_sha256, PluginRegistryError};

fn scratch_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("syl-checksum-test-{name}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn accepts_a_matching_sha256() {
    let dir = scratch_dir("match");
    let path = dir.join("file.bin");
    std::fs::write(&path, b"hello world").unwrap();

    let expected = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
    verify_sha256(&path, expected).unwrap();
}

#[test]
fn rejects_a_mismatched_sha256() {
    let dir = scratch_dir("mismatch");
    let path = dir.join("file.bin");
    std::fs::write(&path, b"hello world").unwrap();

    let err = verify_sha256(
        &path,
        "0000000000000000000000000000000000000000000000000000000000000000",
    )
    .unwrap_err();
    assert!(matches!(err, PluginRegistryError::ChecksumMismatch { .. }));
}
