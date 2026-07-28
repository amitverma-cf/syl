use std::sync::atomic::{AtomicU64, Ordering};

use plugin_registry::{resolve_model_for_kind, ModelKind, PluginRegistryError};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_dir(label: &str) -> std::path::PathBuf {
    let unique = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "syl-resolve-{label}-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn file_url(path: &std::path::Path) -> String {
    format!("file://{}", path.to_string_lossy().replace('\\', "/"))
}

#[test]
fn resolves_the_model_and_engine_matching_the_requested_kind() {
    let registry_dir = temp_dir("registry");
    let cache_dir = temp_dir("cache");

    let engine_file = registry_dir.join("test-engine.bin");
    std::fs::write(&engine_file, b"engine").unwrap();
    let model_file = registry_dir.join("test-model.gguf");
    std::fs::write(&model_file, b"model").unwrap();

    std::fs::write(registry_dir.join("engines.json"), "[]").unwrap();
    std::fs::write(
        registry_dir.join("local.engines.json"),
        format!(
            r#"[{{"id":"test-engine","version":"1.0","platform":"test","download_url":"{}","sha256":null}}]"#,
            file_url(&engine_file)
        ),
    )
    .unwrap();
    std::fs::write(registry_dir.join("models.json"), "[]").unwrap();
    std::fs::write(
        registry_dir.join("local.models.json"),
        format!(
            r#"[{{"name":"test-model","kind":"chat","size_bytes":1,"quantization":"q1","required_engine":"test-engine","download_url":"{}","sha256":null}}]"#,
            file_url(&model_file)
        ),
    )
    .unwrap();

    let resolved =
        resolve_model_for_kind(&registry_dir, &cache_dir, &cache_dir, ModelKind::Chat).unwrap();
    assert_eq!(resolved.model_name, "test-model");
    assert_eq!(resolved.engine_id, "test-engine");
    assert_eq!(resolved.model_path, model_file);
    assert_eq!(resolved.engine_library_path, engine_file);

    std::fs::remove_dir_all(&registry_dir).unwrap();
    std::fs::remove_dir_all(&cache_dir).unwrap();
}

#[test]
fn errors_clearly_when_no_model_of_the_requested_kind_exists() {
    let registry_dir = temp_dir("registry-empty");
    let cache_dir = temp_dir("cache-empty");
    std::fs::write(registry_dir.join("engines.json"), "[]").unwrap();
    std::fs::write(registry_dir.join("models.json"), "[]").unwrap();

    let err = resolve_model_for_kind(&registry_dir, &cache_dir, &cache_dir, ModelKind::Embedding)
        .unwrap_err();
    assert!(matches!(
        err,
        PluginRegistryError::NoModelAvailable(ModelKind::Embedding)
    ));

    std::fs::remove_dir_all(&registry_dir).unwrap();
    std::fs::remove_dir_all(&cache_dir).unwrap();
}

#[test]
fn errors_clearly_when_the_required_engine_is_missing() {
    let registry_dir = temp_dir("registry-no-engine");
    let cache_dir = temp_dir("cache-no-engine");
    std::fs::write(registry_dir.join("engines.json"), "[]").unwrap();
    std::fs::write(
        registry_dir.join("models.json"),
        r#"[{"name":"test-model","kind":"chat","size_bytes":1,"quantization":"q1","required_engine":"missing-engine","download_url":"file:///x.gguf","sha256":null}]"#,
    )
    .unwrap();

    let err =
        resolve_model_for_kind(&registry_dir, &cache_dir, &cache_dir, ModelKind::Chat).unwrap_err();
    assert!(matches!(err, PluginRegistryError::NoEngineAvailable { .. }));

    std::fs::remove_dir_all(&registry_dir).unwrap();
    std::fs::remove_dir_all(&cache_dir).unwrap();
}
