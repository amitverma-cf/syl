//! Tauri commands exposed to the frontend.

use std::path::PathBuf;

use engine_host::llama::LlamaEngine;
use tauri::ipc::Channel;

/// One message sent to the frontend while a generation request is in progress.
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum GenerationEvent {
    /// A piece of generated text.
    Piece { text: String },
    /// Generation finished successfully.
    Done,
    /// Generation failed.
    Error { message: String },
}

/// Runs one generation call for `prompt` against the locally configured llama.cpp engine and
/// model, streaming each generated piece of text to `on_event`.
#[tauri::command]
#[tracing::instrument(skip(on_event))]
pub async fn generate(prompt: String, on_event: Channel<GenerationEvent>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || match run_generate(&prompt, &on_event) {
        Ok(()) => {
            let _ = on_event.send(GenerationEvent::Done);
        }
        Err(message) => {
            tracing::error!(%message, "generate failed");
            let _ = on_event.send(GenerationEvent::Error { message });
        }
    })
    .await
    .map_err(|e| e.to_string())
}

#[tracing::instrument(skip(on_event))]
fn run_generate(prompt: &str, on_event: &Channel<GenerationEvent>) -> Result<(), String> {
    let registry_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../registry");

    let engines = plugin_registry::load_engine_entries(&registry_dir).map_err(|e| e.to_string())?;
    let engine_entry = engines
        .iter()
        .find(|e| e.id == "llama-cpp")
        .ok_or_else(|| "no llama-cpp entry in registry/local.engines.json".to_string())?;
    let library_path = plugin_registry::resolve_local_path(&engine_entry.download_url)
        .map_err(|e| e.to_string())?;

    let models = plugin_registry::load_model_entries(&registry_dir).map_err(|e| e.to_string())?;
    let model_entry = models
        .first()
        .ok_or_else(|| "no entries in registry/local.models.json".to_string())?;
    let model_path = plugin_registry::resolve_local_path(&model_entry.huggingface_url)
        .map_err(|e| e.to_string())?;

    let mut engine =
        LlamaEngine::load(&library_path, &model_path, 2048).map_err(|e| e.to_string())?;

    engine
        .generate(prompt, 128, |piece| {
            if let Err(err) = on_event.send(GenerationEvent::Piece {
                text: piece.to_string(),
            }) {
                tracing::error!(?err, "failed to send piece to channel");
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}
