use std::sync::Arc;

use core_types::workspace_paths;
use engine_host::llama::LlamaEngine;
use memory::{ConversationStore, Message, SqliteConversationStore};
use tauri::ipc::Channel;

use crate::AppState;

const CHAT_MODEL_NAME: &str = "Qwen3.5-0.8B-Q4_K_M";

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum GenerationEvent {
    Piece { text: String },
    Done,
    Error { message: String },
}

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

    let registry_dir = workspace_paths::registry_dir();

    let engines = plugin_registry::load_engine_entries(&registry_dir).map_err(|e| e.to_string())?;
    let engine_entry = engines
        .iter()
        .find(|e| e.id == "llama-cpp")
        .ok_or_else(|| "no llama-cpp entry in .syl/registry/engines.json".to_string())?;
    let library_path = plugin_registry::resolve_local_path(&engine_entry.download_url)
        .map_err(|e| e.to_string())?;

    let models = plugin_registry::load_model_entries(&registry_dir).map_err(|e| e.to_string())?;
    let model_entry = models
        .iter()
        .find(|m| m.name == CHAT_MODEL_NAME)
        .ok_or_else(|| "no chat model entry in .syl/registry/models.json".to_string())?;
    let model_path = plugin_registry::resolve_local_path(&model_entry.download_url)
        .map_err(|e| e.to_string())?;

    let mut engine =
        LlamaEngine::load(&library_path, &model_path, 2048, false).map_err(|e| e.to_string())?;

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
