use std::sync::atomic::{AtomicU64, Ordering};

use plugin_registry::{load_engine_entries, load_model_entries, PluginRegistryError};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_registry_dir() -> std::path::PathBuf {
    let unique = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "syl-plugin-registry-test-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn load_engine_entries_merges_local_override() {
    let dir = temp_registry_dir();
    std::fs::write(dir.join("engines.json"), "[]").unwrap();
    std::fs::write(
        dir.join("local.engines.json"),
        r#"[{"id":"test-engine","version":"1.0","platform":"test","download_url":"file:///does/not/matter.bin","sha256":null}]"#,
    )
    .unwrap();

    let entries = load_engine_entries(&dir).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, "test-engine");

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn load_engine_entries_errors_on_malformed_json() {
    let dir = temp_registry_dir();
    std::fs::write(dir.join("engines.json"), "not valid json").unwrap();

    let err = load_engine_entries(&dir).unwrap_err();
    assert!(matches!(err, PluginRegistryError::Parse { .. }));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn load_model_entries_merges_local_override() {
    let dir = temp_registry_dir();
    std::fs::write(dir.join("models.json"), "[]").unwrap();
    std::fs::write(
        dir.join("local.models.json"),
        r#"[{"name":"test-model","kind":"chat","size_bytes":1,"quantization":"q1","required_engine":"test-engine","download_url":"file:///does/not/matter.gguf","sha256":null}]"#,
    )
    .unwrap();

    let entries = load_model_entries(&dir).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "test-model");

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn load_entries_without_any_registry_files_returns_error() {
    let dir = temp_registry_dir();
    let err = load_engine_entries(&dir).unwrap_err();
    assert!(matches!(err, PluginRegistryError::Io { .. }));
    std::fs::remove_dir_all(&dir).unwrap();
}
