//! End-to-end proof that the real `sd-worker` binary, spawned as a real
//! child process against a real local `.syl` stable-diffusion engine and
//! model, actually generates a real image through
//! `extension_host::ExtensionProcess::call`. Run manually with
//! `cargo test -p sd-worker --test real_sd_worker -- --ignored`.

use std::path::Path;

use extension_host::{ExtensionBackend, ExtensionManifest, ExtensionProcess};

fn resolve_real_paths() -> (String, String) {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.syl");
    let library_path = base.join("engines/stable-diffusion/stable-diffusion.dll");
    let model_path = base.join("models/stable-diffusion-v1-5-Q4_0.gguf");
    assert!(
        library_path.exists() && model_path.exists(),
        "run the app once to seed .syl/engines and .syl/models with a stable-diffusion engine \
         and model first"
    );
    (
        library_path.display().to_string(),
        model_path.display().to_string(),
    )
}

#[tokio::test]
#[ignore]
async fn generates_a_real_image_through_the_isolated_process() {
    let (library_path, model_path) = resolve_real_paths();
    let manifest = ExtensionManifest {
        id: "stable-diffusion-image".to_string(),
        version: "1.0.0".to_string(),
        display_name: "Stable Diffusion Image Generator".to_string(),
        backend: Some(ExtensionBackend {
            command: env!("CARGO_BIN_EXE_sd-worker").to_string(),
            args: vec![
                "--library".to_string(),
                library_path,
                "--model".to_string(),
                model_path,
                "--n-threads".to_string(),
                "4".to_string(),
            ],
        }),
        provides: vec!["image.generate/v1".to_string()],
        requires: vec![],
        contributes: None,
    };

    let process = ExtensionProcess::spawn(manifest).await.unwrap();
    let result = process
        .call(
            "image.generate/v1",
            "image/generate",
            serde_json::json!({
                "prompt": "a red circle on a white background",
                "negativePrompt": "",
                "width": 64,
                "height": 64,
                "steps": 4,
                "seed": 42,
            }),
        )
        .await
        .unwrap();

    let png_base64 = result.get("pngBase64").and_then(|v| v.as_str()).unwrap();
    assert!(!png_base64.is_empty());

    process.kill().await;
}
