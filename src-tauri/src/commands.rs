use std::sync::Arc;

use core_types::workspace_paths;
use engine_host::llama::LlamaEngine;
use memory::{ConversationStore, Message, SqliteConversationStore};
use plugin_registry::ModelKind;
use tauri::ipc::Channel;

use crate::{AppState, ToolState};

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

#[tauri::command]
pub async fn call_tool(
    conversation_id: String,
    name: String,
    args: serde_json::Value,
    state: tauri::State<'_, ToolState>,
) -> Result<serde_json::Value, String> {
    state
        .executor
        .call(&conversation_id, &name, args)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn respond_permission(
    request_id: u64,
    response: tool::PromptResponse,
    state: tauri::State<'_, ToolState>,
) {
    state.prompter.resolve(request_id, response);
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

    let resolved = plugin_registry::resolve_model_for_kind(
        &workspace_paths::registry_dir(),
        &workspace_paths::models_dir(),
        &workspace_paths::engines_dir(),
        ModelKind::Chat,
    )
    .map_err(|e| e.to_string())?;

    let mut engine = LlamaEngine::load(
        &resolved.engine_library_path,
        &resolved.model_path,
        2048,
        false,
    )
    .map_err(|e| e.to_string())?;

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
