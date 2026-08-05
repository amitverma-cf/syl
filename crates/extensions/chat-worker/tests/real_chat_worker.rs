//! End-to-end proof that the real `chat-worker` binary, spawned as a real
//! child process against a real local GGUF model, actually works through
//! `extension_host::ExtensionProcess` — and, the actual point of the whole
//! extension-ecosystem pass, that killing it mid-generation is detected
//! cleanly instead of taking the host process down. Needs a local `.syl`
//! workspace with the `llama-cpp` engine and a chat model already present
//! (same requirement as `native-engines`'s `real_llama_generate.rs`) — run
//! manually with `cargo test -p chat-worker --test real_chat_worker -- --ignored`.

use std::path::Path;

use extension_host::{
    ExtensionBackend, ExtensionManifest, ExtensionProcess, ExtensionProcessError,
};
use extension_registry::ModelKind;

fn resolve_real_chat_model() -> (String, String) {
    let registry_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.syl/registry");
    let cache_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.syl");

    let resolved = extension_registry::resolve_model_for_kind(
        &registry_dir,
        &cache_dir.join("models"),
        &cache_dir.join("engines"),
        ModelKind::Chat,
    )
    .expect(".syl workspace has no chat model registered; run the app once to seed it");

    (
        resolved.engine_library_path.display().to_string(),
        resolved.model_path.display().to_string(),
    )
}

async fn spawn_real_chat_worker() -> std::sync::Arc<ExtensionProcess> {
    let (library_path, model_path) = resolve_real_chat_model();
    let manifest = ExtensionManifest {
        id: "llama-cpp-chat".to_string(),
        version: "1.0.0".to_string(),
        display_name: "llama.cpp Chat Engine".to_string(),
        backend: Some(ExtensionBackend {
            command: env!("CARGO_BIN_EXE_chat-worker").to_string(),
            args: vec![
                "--library".to_string(),
                library_path,
                "--model".to_string(),
                model_path,
                "--n-ctx".to_string(),
                "2048".to_string(),
            ],
        }),
        provides: vec!["inference.chat/v1".to_string()],
        requires: vec![],
        contributes: None,
    };
    std::sync::Arc::new(ExtensionProcess::spawn(manifest).await.unwrap())
}

#[tokio::test]
#[ignore]
async fn generates_real_text_and_counts_real_tokens_through_the_isolated_process() {
    let process = spawn_real_chat_worker().await;

    let mut piece_count = 0usize;
    let output = process
        .generate("The capital of France is", 32, |_piece| piece_count += 1)
        .await
        .unwrap();
    assert!(!output.trim().is_empty());
    assert!(piece_count > 0);

    let count = process
        .count_tokens("The capital of France is")
        .await
        .unwrap();
    assert!(count > 0);

    process.kill().await;
}

#[tokio::test]
#[ignore]
async fn killing_the_real_child_process_mid_generation_is_detected_cleanly() {
    let process = spawn_real_chat_worker().await;

    // Kill the real llama.cpp-loaded process out from under an in-flight
    // generate call — this is the scenario a native segfault/panic inside
    // llama.cpp itself would produce.
    let kill_handle = tokio::spawn({
        let process = process.clone();
        async move {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            process.kill().await;
        }
    });

    let result = process
        .generate(
            "Write a very long detailed essay about the history of computing",
            512,
            |_piece| {},
        )
        .await;
    assert!(matches!(result, Err(ExtensionProcessError::Crashed)));

    kill_handle.await.unwrap();
    assert!(!process.is_alive());
}
