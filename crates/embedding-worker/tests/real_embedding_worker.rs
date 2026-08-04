//! End-to-end proof that the real `embedding-worker` binary, spawned as a
//! real child process against a real local `.syl` onnxruntime engine and
//! model, actually embeds real text through
//! `extension_host::ExtensionProcess::call`. Run manually with
//! `cargo test -p embedding-worker --test real_embedding_worker -- --ignored`.

use std::path::Path;

use extension_host::{ExtensionBackend, ExtensionManifest, ExtensionProcess};

fn resolve_real_paths() -> (String, String, String) {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.syl");
    let library_path = base.join("engines/onnxruntime/onnxruntime.dll");
    let model_path = base.join("models/all-MiniLM-L6-v2/model.onnx");
    let tokenizer_path = base.join("models/all-MiniLM-L6-v2/tokenizer.json");
    assert!(
        library_path.exists() && model_path.exists() && tokenizer_path.exists(),
        "run the app once to seed .syl/engines and .syl/models with the onnxruntime engine and \
         an embedding model first"
    );
    (
        library_path.display().to_string(),
        model_path.display().to_string(),
        tokenizer_path.display().to_string(),
    )
}

#[tokio::test]
#[ignore]
async fn embeds_real_text_through_the_isolated_process() {
    let (library_path, model_path, tokenizer_path) = resolve_real_paths();
    let manifest = ExtensionManifest {
        id: "onnx-embedding".to_string(),
        version: "1.0.0".to_string(),
        display_name: "ONNX Embedding Engine".to_string(),
        backend: Some(ExtensionBackend {
            command: env!("CARGO_BIN_EXE_embedding-worker").to_string(),
            args: vec![
                "--library".to_string(),
                library_path,
                "--model".to_string(),
                model_path,
                "--tokenizer".to_string(),
                tokenizer_path,
            ],
        }),
        provides: vec!["embedding.embed/v1".to_string()],
        requires: vec![],
        contributes: None,
    };

    let process = ExtensionProcess::spawn(manifest).await.unwrap();
    let result = process
        .call(
            "embedding.embed/v1",
            "embedding/embed",
            serde_json::json!({ "text": "the quick brown fox" }),
        )
        .await
        .unwrap();

    let vector: Vec<f32> = serde_json::from_value(result.get("vector").cloned().unwrap()).unwrap();
    assert!(!vector.is_empty());

    process.kill().await;
}
