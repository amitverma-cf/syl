use std::sync::Arc;

use core_types::workspace_paths;
use daemon::events::DaemonEvent;
use engine_host::llama::LlamaEngine;
use memory::{ConversationStore, ConversationSummary, Message, SqliteConversationStore};
use plugin_registry::ModelKind;
use tauri::ipc::Channel;
use tool::ToolExecutor;

use crate::daemon::DaemonState;
use crate::flows::{FlowState, DEFAULT_FLOW_NAME};
use crate::{AppState, ToolState};

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum GenerationEvent {
    Piece { text: String },
    Done,
    Error { message: String },
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
#[tracing::instrument(skip(on_event, state, tool_state, flow_state, daemon_state))]
pub async fn generate(
    prompt: String,
    conversation_id: String,
    model: Option<String>,
    on_event: Channel<GenerationEvent>,
    state: tauri::State<'_, AppState>,
    tool_state: tauri::State<'_, ToolState>,
    flow_state: tauri::State<'_, FlowState>,
    daemon_state: tauri::State<'_, DaemonState>,
) -> Result<(), String> {
    let store = state.conversation_store.clone();

    let flow_turn = flow_state.ensure_and_take_turn(&conversation_id)?;
    let tool_specs = tool_state
        .executor
        .tool_specs_filtered(&flow_turn.tool_allowlist);

    let conversation_id_for_store = conversation_id.clone();
    let cloud_model = model.filter(|m| !m.is_empty());
    let generation_result = match cloud_model {
        Some(model_id) => {
            run_generate_cloud(
                &store,
                &conversation_id_for_store,
                &prompt,
                &flow_turn.system_prompt,
                &model_id,
                &tool_specs,
                &tool_state.executor,
                &on_event,
            )
            .await
        }
        None => {
            let executor = tool_state.executor.clone();
            let system_prompt = flow_turn.system_prompt.clone();
            tauri::async_runtime::spawn_blocking(move || {
                run_generate(
                    &store,
                    &conversation_id_for_store,
                    &prompt,
                    &system_prompt,
                    &tool_specs,
                    &executor,
                    &on_event,
                )
            })
            .await
            .map_err(|e| e.to_string())?
        }
    };

    match generation_result {
        Ok(()) => {
            if let Some(info) = flow_state.advance(&conversation_id, "message") {
                daemon_state
                    .event_bus
                    .publish(DaemonEvent::FlowStateChanged {
                        flow: info.flow_name,
                        state: info.state_name,
                    });
            }
        }
        Err(message) => {
            tracing::error!(%message, "generate failed");
        }
    }
    Ok(())
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
pub fn list_conversations(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ConversationSummary>, String> {
    state
        .conversation_store
        .list_conversations()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_conversation(
    id: String,
    title: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state
        .conversation_store
        .create_conversation(&id, &title, DEFAULT_FLOW_NAME)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_conversation(
    id: String,
    title: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state
        .conversation_store
        .rename_conversation(&id, &title)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_conversation(id: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    state
        .conversation_store
        .delete_conversation(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn call_tool(
    conversation_id: String,
    name: String,
    args: serde_json::Value,
    state: tauri::State<'_, ToolState>,
    daemon_state: tauri::State<'_, DaemonState>,
) -> Result<serde_json::Value, String> {
    let result = state.executor.call(&conversation_id, &name, args).await;
    daemon_state
        .event_bus
        .publish(DaemonEvent::ToolCallCompleted {
            tool: name,
            ok: result.is_ok(),
        });
    result.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn respond_permission(
    request_id: u64,
    response: tool::PromptResponse,
    state: tauri::State<'_, ToolState>,
) {
    state.prompter.resolve(request_id, response);
}

#[tracing::instrument(skip(store, tools, executor, on_event))]
#[allow(clippy::too_many_arguments)]
async fn run_generate_cloud(
    store: &Arc<SqliteConversationStore>,
    conversation_id: &str,
    prompt: &str,
    system_prompt: &str,
    model_id: &str,
    tools: &[tool::ToolSpec],
    executor: &ToolExecutor,
    on_event: &Channel<GenerationEvent>,
) -> Result<(), String> {
    let result: Result<(), String> = async {
        let store_for_user = store.clone();
        let conversation_id_owned = conversation_id.to_string();
        let prompt_owned = prompt.to_string();
        tauri::async_runtime::spawn_blocking(move || {
            store_for_user.append_message(&conversation_id_owned, "user", &prompt_owned)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

        let client = provider::build_client(
            &workspace_paths::env_file(),
            &workspace_paths::custom_providers_file(),
        );
        let response = provider::chat_with_tools(
            &client,
            model_id,
            Some(system_prompt),
            prompt,
            tools,
            executor,
            conversation_id,
            |piece| {
                if let Err(err) = on_event.send(GenerationEvent::Piece {
                    text: piece.to_string(),
                }) {
                    tracing::error!(?err, "failed to send piece to channel");
                }
            },
        )
        .await
        .map_err(|e| e.to_string())?;

        let store_for_assistant = store.clone();
        let conversation_id_owned = conversation_id.to_string();
        tauri::async_runtime::spawn_blocking(move || {
            store_for_assistant.append_message(&conversation_id_owned, "assistant", &response)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
        Ok(())
    }
    .await;

    match &result {
        Ok(()) => {
            let _ = on_event.send(GenerationEvent::Done);
        }
        Err(message) => {
            let _ = on_event.send(GenerationEvent::Error {
                message: message.clone(),
            });
        }
    }
    result
}

#[tracing::instrument(skip(store, tools, executor, on_event))]
#[allow(clippy::too_many_arguments)]
fn run_generate(
    store: &Arc<SqliteConversationStore>,
    conversation_id: &str,
    prompt: &str,
    system_prompt: &str,
    tools: &[tool::ToolSpec],
    executor: &ToolExecutor,
    on_event: &Channel<GenerationEvent>,
) -> Result<(), String> {
    let result = (|| -> Result<(), String> {
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

        let response = engine_host::tool_loop::generate_with_tools(
            &mut engine,
            system_prompt,
            prompt,
            tools,
            executor,
            conversation_id,
            128,
            |piece| {
                if let Err(err) = on_event.send(GenerationEvent::Piece {
                    text: piece.to_string(),
                }) {
                    tracing::error!(?err, "failed to send piece to channel");
                }
            },
        )?;

        store
            .append_message(conversation_id, "assistant", &response)
            .map_err(|e| e.to_string())?;
        Ok(())
    })();

    match &result {
        Ok(()) => {
            let _ = on_event.send(GenerationEvent::Done);
        }
        Err(message) => {
            let _ = on_event.send(GenerationEvent::Error {
                message: message.clone(),
            });
        }
    }
    result
}
