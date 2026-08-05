//! End-to-end proof that the real `tts-worker` binary, spawned as a real
//! child process against a real local `.syl` onnxruntime engine and
//! mms-tts-eng model, actually synthesizes real audio through
//! `extension_host::ExtensionProcess::call`. Run manually with
//! `cargo test -p tts-worker --test real_tts_worker -- --ignored`.

use std::path::Path;

use base64::Engine;
use extension_host::{ExtensionBackend, ExtensionManifest, ExtensionProcess};

fn resolve_real_paths() -> (String, String, String) {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.syl");
    let library_path = base.join("engines/onnxruntime/onnxruntime.dll");
    let model_path = base.join("models/mms-tts-eng/model.onnx");
    let vocab_path = base.join("models/mms-tts-eng/vocab.json");
    assert!(
        library_path.exists() && model_path.exists() && vocab_path.exists(),
        "run the app once to seed .syl/engines and .syl/models with the onnxruntime engine and \
         a TTS model first"
    );
    (
        library_path.display().to_string(),
        model_path.display().to_string(),
        vocab_path.display().to_string(),
    )
}

#[tokio::test]
#[ignore]
async fn synthesizes_real_audio_through_the_isolated_process() {
    let (library_path, model_path, vocab_path) = resolve_real_paths();
    let manifest = ExtensionManifest {
        id: "onnx-tts".to_string(),
        version: "1.0.0".to_string(),
        display_name: "ONNX Text-to-Speech Engine".to_string(),
        backend: Some(ExtensionBackend {
            command: env!("CARGO_BIN_EXE_tts-worker").to_string(),
            args: vec![
                "--library".to_string(),
                library_path,
                "--model".to_string(),
                model_path,
                "--vocab".to_string(),
                vocab_path,
            ],
        }),
        provides: vec!["tts.synthesize/v1".to_string()],
        requires: vec![],
        contributes: None,
    };

    let process = ExtensionProcess::spawn(manifest).await.unwrap();
    let result = process
        .call(
            "tts.synthesize/v1",
            "tts/synthesize",
            serde_json::json!({ "text": "hello world" }),
        )
        .await
        .unwrap();

    let pcm_base64 = result.get("pcmBase64").and_then(|v| v.as_str()).unwrap();
    let pcm_bytes = base64::engine::general_purpose::STANDARD
        .decode(pcm_base64)
        .unwrap();
    assert!(!pcm_bytes.is_empty());

    process.kill().await;
}
