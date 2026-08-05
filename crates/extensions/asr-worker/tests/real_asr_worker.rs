//! End-to-end proof that the real `asr-worker` binary, spawned as a real
//! child process against a real local `.syl` onnxruntime engine and
//! whisper-tiny model, actually transcribes real (silent) PCM audio through
//! `extension_host::ExtensionProcess::call`. Run manually with
//! `cargo test -p asr-worker --test real_asr_worker -- --ignored`.

use std::path::Path;

use base64::Engine;
use extension_host::{ExtensionBackend, ExtensionManifest, ExtensionProcess};

fn resolve_real_paths() -> (String, String, String, String) {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.syl");
    let library_path = base.join("engines/onnxruntime/onnxruntime.dll");
    let encoder_path = base.join("models/whisper-tiny/encoder_model.onnx");
    let decoder_path = base.join("models/whisper-tiny/decoder_model.onnx");
    let tokenizer_path = base.join("models/whisper-tiny/tokenizer.json");
    assert!(
        library_path.exists() && encoder_path.exists() && decoder_path.exists(),
        "run the app once to seed .syl/engines and .syl/models with the onnxruntime engine and \
         an ASR model first"
    );
    (
        library_path.display().to_string(),
        encoder_path.display().to_string(),
        decoder_path.display().to_string(),
        tokenizer_path.display().to_string(),
    )
}

#[tokio::test]
#[ignore]
async fn transcribes_real_pcm_through_the_isolated_process() {
    let (library_path, encoder_path, decoder_path, tokenizer_path) = resolve_real_paths();
    let manifest = ExtensionManifest {
        id: "onnx-asr".to_string(),
        version: "1.0.0".to_string(),
        display_name: "ONNX Speech-to-Text Engine".to_string(),
        backend: Some(ExtensionBackend {
            command: env!("CARGO_BIN_EXE_asr-worker").to_string(),
            args: vec![
                "--library".to_string(),
                library_path,
                "--encoder".to_string(),
                encoder_path,
                "--decoder".to_string(),
                decoder_path,
                "--tokenizer".to_string(),
                tokenizer_path,
            ],
        }),
        provides: vec!["asr.transcribe/v1".to_string()],
        requires: vec![],
        contributes: None,
    };

    let process = ExtensionProcess::spawn(manifest).await.unwrap();

    // One second of silence at 16kHz — the point here is proving the real
    // IPC round trip (base64 PCM in, real onnx inference, text out) works,
    // not asserting on specific transcribed content.
    let pcm = vec![0.0f32; 16_000];
    let pcm_bytes: Vec<u8> = pcm.iter().flat_map(|s| s.to_le_bytes()).collect();
    let pcm_base64 = base64::engine::general_purpose::STANDARD.encode(pcm_bytes);

    let result = process
        .call(
            "asr.transcribe/v1",
            "asr/transcribe",
            serde_json::json!({ "pcmBase64": pcm_base64 }),
        )
        .await
        .unwrap();

    assert!(result.get("text").and_then(|v| v.as_str()).is_some());

    process.kill().await;
}
