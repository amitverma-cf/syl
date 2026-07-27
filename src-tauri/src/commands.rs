//! Tauri commands exposed to the frontend.

use std::path::PathBuf;
use std::sync::Arc;

use engine_host::llama::LlamaEngine;
use memory::{ConversationStore, Message, SqliteConversationStore};
use tauri::ipc::Channel;

use crate::AppState;

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

/// Runs one generation call for `prompt` in `conversation_id` against the locally configured
/// llama.cpp engine and model, streaming each generated piece of text to `on_event`. The
/// prompt and the full response are persisted to the conversation store.
#[tauri::command]
#[tracing::instrument(skip(on_event, state))]
pub async fn generate(
    prompt: String,
    conversation_id: String,
    on_event: Channel<GenerationEvent>,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let store = state.conversation_store.clone();
    tauri::async_runtime::spawn_blocking(move || {
        match run_generate(&store, &conversation_id, &prompt, &on_event) {
            Ok(()) => {
                let _ = on_event.send(GenerationEvent::Done);
            }
            Err(message) => {
                tracing::error!(%message, "generate failed");
                let _ = on_event.send(GenerationEvent::Error { message });
            }
        }
    })
    .await
    .map_err(|e| e.to_string())
}

/// Returns every message stored for `conversation_id`, oldest first.
#[tauri::command]
pub fn list_messages(
    conversation_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Message>, String> {
    state
        .conversation_store
        .list_messages(&conversation_id)
        .map_err(|e| e.to_string())
}

#[tracing::instrument(skip(store, on_event))]
fn run_generate(
    store: &Arc<SqliteConversationStore>,
    conversation_id: &str,
    prompt: &str,
    on_event: &Channel<GenerationEvent>,
) -> Result<(), String> {
    store
        .append_message(conversation_id, "user", prompt)
        .map_err(|e| e.to_string())?;

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

    let response = engine
        .generate(prompt, 128, |piece| {
            if let Err(err) = on_event.send(GenerationEvent::Piece {
                text: piece.to_string(),
            }) {
                tracing::error!(?err, "failed to send piece to channel");
            }
        })
        .map_err(|e| e.to_string())?;

    store
        .append_message(conversation_id, "assistant", &response)
        .map_err(|e| e.to_string())?;
    Ok(())
}
