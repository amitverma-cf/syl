use std::sync::Arc;

use core_types::app_config::app_config;
use core_types::workspace_paths;
use daemon::events::DaemonEvent;
use extension_host::ExtensionProcess;
use memory::{
    ConversationStore, ConversationSummary, EmbeddingStore, Message, SqliteConversationStore,
    ToolPermissionDecision, ToolPermissionStore,
};
use plugin_registry::ModelKind;
use tauri::ipc::{Channel, Response};
use tool::ToolExecutor;

use crate::daemon::DaemonState;
use crate::embeddings::OnnxModelState;
use crate::flows::{default_flow_name, FlowState, WorkspaceFolderState};
use crate::local_models::LocalModelState;
use crate::{AppState, ToolState};

/// Fully exits the app. The window's own close button just hides it (so the
/// app keeps running in the tray, matching the tray menu's existing
/// behavior) — this is the one real way to actually quit from inside the UI
/// itself, e.g. from the app menu.
#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

/// Grants the fs plugin read/write access to a directory the user just
/// picked via the native folder dialog. The capability file declares no
/// static path scope for fs commands (so by default nothing outside the
/// app's own workspace is reachable) — this is what actually opens up a
/// user-chosen folder, scoped to exactly what they picked rather than the
/// whole disk.
#[tauri::command]
pub fn grant_folder_access(
    path: String,
    app: tauri::AppHandle,
    workspace_folder: tauri::State<'_, WorkspaceFolderState>,
) -> Result<(), String> {
    use tauri_plugin_fs::FsExt;
    app.fs_scope()
        .allow_directory(&path, true)
        .map_err(|e| e.to_string())?;
    workspace_folder.set(std::path::PathBuf::from(&path));
    Ok(())
}

/// Raw wire framing for streamed generation events: a one-byte tag followed by a
/// UTF-8 payload (empty for `Done`). Sent as raw bytes through the IPC channel
/// (`tauri::ipc::Response`) instead of a per-piece JSON envelope — pieces are by
/// far the highest-frequency message on this channel (one per generated token),
/// so skipping JSON serialize/parse on the hot path is the one wire-format
/// optimization worth doing ahead of a profile.
const GENERATION_EVENT_TAG_PIECE: u8 = 0;
const GENERATION_EVENT_TAG_DONE: u8 = 1;
const GENERATION_EVENT_TAG_ERROR: u8 = 2;

fn send_generation_piece(on_event: &Channel<Response>, text: &str) {
    let mut bytes = Vec::with_capacity(1 + text.len());
    bytes.push(GENERATION_EVENT_TAG_PIECE);
    bytes.extend_from_slice(text.as_bytes());
    if let Err(err) = on_event.send(Response::new(bytes)) {
        tracing::error!(?err, "failed to send piece to channel");
    }
}

fn send_generation_done(on_event: &Channel<Response>) {
    let _ = on_event.send(Response::new(vec![GENERATION_EVENT_TAG_DONE]));
}

fn send_generation_error(on_event: &Channel<Response>, message: &str) {
    let mut bytes = Vec::with_capacity(1 + message.len());
    bytes.push(GENERATION_EVENT_TAG_ERROR);
    bytes.extend_from_slice(message.as_bytes());
    let _ = on_event.send(Response::new(bytes));
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
#[tracing::instrument(skip(
    on_event,
    state,
    tool_state,
    flow_state,
    daemon_state,
    local_model_state,
    workspace_folder,
    onnx_state
))]
pub async fn generate(
    prompt: String,
    conversation_id: String,
    model: Option<String>,
    local_model: Option<String>,
    on_event: Channel<Response>,
    state: tauri::State<'_, AppState>,
    tool_state: tauri::State<'_, ToolState>,
    flow_state: tauri::State<'_, FlowState>,
    daemon_state: tauri::State<'_, DaemonState>,
    local_model_state: tauri::State<'_, LocalModelState>,
    workspace_folder: tauri::State<'_, WorkspaceFolderState>,
    onnx_state: tauri::State<'_, OnnxModelState>,
) -> Result<(), String> {
    let store = state.conversation_store.clone();

    let flow_turn = flow_state.ensure_and_take_turn(&conversation_id, &workspace_folder)?;
    let tool_specs = tool_state
        .executor
        .tool_specs_filtered(&flow_turn.tool_allowlist);

    tracing::info!(
        %conversation_id,
        cloud_model = model.as_deref().unwrap_or(""),
        local_model = local_model.as_deref().unwrap_or(""),
        tool_count = tool_specs.len(),
        prompt_len = prompt.len(),
        "generate turn starting"
    );

    let mut effective_system_prompt = flow_turn.system_prompt.clone();
    effective_system_prompt.push_str(&compressed_history_block(&store, &conversation_id));
    effective_system_prompt
        .push_str(&retrieved_context_block(&store, &onnx_state, &conversation_id, &prompt).await);

    let conversation_id_for_store = conversation_id.clone();
    let store_for_embed = store.clone();
    let prompt_for_embed = prompt.clone();
    let cloud_model = model.filter(|m| !m.is_empty());
    let local_model_name = local_model.filter(|m| !m.is_empty());
    let piece_event = on_event.clone();
    let generation_result = match cloud_model {
        Some(model_id) => {
            run_generate_cloud(
                &store,
                &conversation_id_for_store,
                &prompt,
                &effective_system_prompt,
                &model_id,
                &tool_specs,
                &tool_state.executor,
                move |piece| send_generation_piece(&piece_event, piece),
            )
            .await
        }
        None => {
            let executor = tool_state.executor.clone();
            let system_prompt = effective_system_prompt.clone();
            match local_model_name {
                Some(name) => {
                    let process = local_model_state.get_loaded(&name).ok_or_else(|| {
                        format!("local model {name} is not loaded; load it first in Settings")
                    })?;
                    run_generate_with_process(
                        &process,
                        &store,
                        &conversation_id_for_store,
                        &prompt,
                        &system_prompt,
                        &tool_specs,
                        &executor,
                        move |piece| send_generation_piece(&piece_event, piece),
                    )
                    .await
                }
                None => {
                    run_generate(
                        &store,
                        &conversation_id_for_store,
                        &prompt,
                        &system_prompt,
                        &tool_specs,
                        &executor,
                        move |piece| send_generation_piece(&piece_event, piece),
                    )
                    .await
                }
            }
        }
    };

    match &generation_result {
        Ok(()) => {
            store_prompt_embedding(
                &store_for_embed,
                &onnx_state,
                &conversation_id,
                &prompt_for_embed,
            )
            .await;
            send_generation_done(&on_event);
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
            send_generation_error(&on_event, message);
        }
    }
    Ok(())
}

const FLOW_GEN_SYSTEM_PROMPT: &str = r#"You design finite-state conversation flows for an agentic assistant. \
Given a short description from the user, respond with ONLY a single JSON object (no prose, no markdown \
code fences) matching exactly this schema:

{
  "name": "kebab-case-flow-name",
  "initial_state": "name of one of the states below",
  "states": [
    {
      "name": "state name",
      "system_prompt": "the system prompt the assistant should use while in this state",
      "tool_allowlist": ["tool_name", "..."],
      "transitions": [ { "on": "trigger keyword or condition", "to_state": "another state name" } ]
    }
  ]
}

Rules: every to_state must reference a state that exists in "states". The first state should represent \
the start of the conversation. At least one state should have an empty "transitions" array to represent \
a natural end of the flow. Keep tool_allowlist empty unless the description clearly calls for a specific \
tool. Output nothing but the JSON object."#;

#[tauri::command]
pub async fn generate_flow_draft(
    prompt: String,
    model: Option<String>,
    local_model: Option<String>,
    local_model_state: tauri::State<'_, LocalModelState>,
) -> Result<String, String> {
    let cloud_model = model.filter(|m| !m.is_empty());
    let local_model_name = local_model.filter(|m| !m.is_empty());

    match cloud_model {
        Some(model_id) => {
            let client = provider::build_client(
                &workspace_paths::env_file(),
                &workspace_paths::custom_providers_file(),
            );
            provider::stream_chat(
                &client,
                &model_id,
                Some(FLOW_GEN_SYSTEM_PROMPT),
                &prompt,
                |_piece| {},
            )
            .await
            .map_err(|e| e.to_string())
        }
        None => {
            let name = local_model_name.ok_or_else(|| "no model specified".to_string())?;
            let process = local_model_state.get_loaded(&name).ok_or_else(|| {
                format!("local model {name} is not loaded; load it first in Settings")
            })?;
            let full_prompt = format!("{FLOW_GEN_SYSTEM_PROMPT}\n\nUser: {prompt}\nAssistant:");
            let max_tokens = app_config().local_engine.max_tokens;
            process
                .generate(&full_prompt, max_tokens, |_piece: &str| {})
                .await
                .map_err(|e| e.to_string())
        }
    }
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
        .create_conversation(&id, &title, default_flow_name())
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
pub fn list_tools(state: tauri::State<'_, ToolState>) -> Vec<tool::ToolSpec> {
    state.executor.tool_specs()
}

#[tauri::command]
pub fn respond_permission(
    request_id: u64,
    response: tool::PromptResponse,
    state: tauri::State<'_, ToolState>,
) {
    state.prompter.resolve(request_id, response);
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolPermissionEntry {
    pub tool_name: String,
    pub decision: ToolPermissionDecision,
}

/// Every remembered "Always allow"/"Always deny" decision for a conversation, so
/// Settings can show a real revoke UI instead of the only prior workaround (editing
/// the SQLite file by hand).
#[tauri::command]
pub fn list_tool_permissions(
    conversation_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ToolPermissionEntry>, String> {
    state
        .conversation_store
        .list_tool_permissions(&conversation_id)
        .map(|entries| {
            entries
                .into_iter()
                .map(|(tool_name, decision)| ToolPermissionEntry {
                    tool_name,
                    decision,
                })
                .collect()
        })
        .map_err(|e| e.to_string())
}

/// Forgets a remembered decision — the next call to that tool in this conversation
/// prompts again.
#[tauri::command]
pub fn clear_tool_permission(
    conversation_id: String,
    tool_name: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state
        .conversation_store
        .clear_tool_permission(&conversation_id, &tool_name)
        .map_err(|e| e.to_string())
}

/// Folds recent conversation history into the prompt, keeping the most recent
/// messages within `contextBudgetChars` (older messages are dropped first, per
/// `tool::compress_context`) — a real character budget instead of sending the
/// whole conversation unbounded on every turn.
fn compressed_history_block(store: &Arc<SqliteConversationStore>, conversation_id: &str) -> String {
    let messages = store.list_messages(conversation_id).unwrap_or_default();
    let budget = app_config().context_budget_chars;
    let compressed = tool::compress_context(&messages, budget);
    if compressed.is_empty() {
        return String::new();
    }
    let mut block = String::from("\n\nConversation history so far:\n");
    for message in &compressed {
        block.push_str(&format!("{}: {}\n", message.role, message.content));
    }
    block
}

/// Retrieves the most semantically relevant earlier messages in this conversation
/// via the loaded embedding model, if any — degrades gracefully (returns an empty
/// block) when no embedding model is loaded, matching the existing "no chat model
/// yet" graceful-degradation pattern elsewhere in this file.
async fn retrieved_context_block(
    store: &Arc<SqliteConversationStore>,
    onnx_state: &tauri::State<'_, OnnxModelState>,
    conversation_id: &str,
    prompt: &str,
) -> String {
    let Some(process) = onnx_state.any_loaded() else {
        tracing::debug!("no embedding model loaded; skipping RAG retrieval");
        return String::new();
    };

    let embedding = match process
        .call(
            "embedding.embed/v1",
            "embedding/embed",
            serde_json::json!({ "text": prompt }),
        )
        .await
        .ok()
        .and_then(|result| result.get("vector").cloned())
        .and_then(|value| serde_json::from_value::<Vec<f32>>(value).ok())
    {
        Some(embedding) => embedding,
        None => {
            tracing::debug!("failed to embed prompt for RAG retrieval; skipping");
            return String::new();
        }
    };

    let store = store.clone();
    let conversation_id_owned = conversation_id.to_string();
    let matches = match tauri::async_runtime::spawn_blocking(move || {
        store.search_similar(&conversation_id_owned, &embedding, 3)
    })
    .await
    {
        Ok(Ok(matches)) => matches,
        _ => {
            tracing::debug!("RAG similarity search failed; skipping");
            return String::new();
        }
    };

    if matches.is_empty() {
        return String::new();
    }
    let mut block =
        String::from("\n\nRelevant context retrieved from earlier in this conversation:\n");
    for m in &matches {
        block.push_str(&format!("- {}\n", m.content));
    }
    block
}

/// Embeds and stores the just-sent prompt so future turns can retrieve it via
/// `retrieved_context_block`. Best-effort: silently skips when no embedding model
/// is loaded, and logs (rather than fails the turn) on embedding/store errors.
async fn store_prompt_embedding(
    store: &Arc<SqliteConversationStore>,
    onnx_state: &tauri::State<'_, OnnxModelState>,
    conversation_id: &str,
    prompt: &str,
) {
    let Some(process) = onnx_state.any_loaded() else {
        return;
    };
    let embedding = match process
        .call(
            "embedding.embed/v1",
            "embedding/embed",
            serde_json::json!({ "text": prompt }),
        )
        .await
        .map_err(|e| e.to_string())
        .and_then(|result| {
            result
                .get("vector")
                .cloned()
                .ok_or_else(|| "embedding-worker response missing vector".to_string())
        })
        .and_then(|value| serde_json::from_value::<Vec<f32>>(value).map_err(|e| e.to_string()))
    {
        Ok(embedding) => embedding,
        Err(err) => {
            tracing::warn!(%err, "failed to embed prompt for storage");
            return;
        }
    };
    let store = store.clone();
    let conversation_id_owned = conversation_id.to_string();
    let prompt_owned = prompt.to_string();
    let result = tauri::async_runtime::spawn_blocking(move || {
        store
            .store_embedding(&conversation_id_owned, &prompt_owned, &embedding)
            .map_err(|e| e.to_string())
    })
    .await;
    match result {
        Err(err) => tracing::warn!(?err, "failed to join embedding-store task"),
        Ok(Err(err)) => tracing::warn!(%err, "failed to store prompt embedding"),
        Ok(Ok(())) => {}
    }
}

#[tracing::instrument(skip(store, tools, executor, on_piece))]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_generate_cloud(
    store: &Arc<SqliteConversationStore>,
    conversation_id: &str,
    prompt: &str,
    system_prompt: &str,
    model_id: &str,
    tools: &[tool::ToolSpec],
    executor: &ToolExecutor,
    mut on_piece: impl FnMut(&str) + Send,
) -> Result<(), String> {
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
        |piece| on_piece(piece),
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

/// Auto-resolves whichever chat model/engine the registry has, spawns it as
/// an isolated extension process just for this one turn, and kills it
/// afterward — the fallback path when the caller didn't already have a
/// specific local model loaded via `load_local_model`.
#[tracing::instrument(skip(store, tools, executor, on_piece))]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_generate(
    store: &Arc<SqliteConversationStore>,
    conversation_id: &str,
    prompt: &str,
    system_prompt: &str,
    tools: &[tool::ToolSpec],
    executor: &ToolExecutor,
    on_piece: impl FnMut(&str),
) -> Result<(), String> {
    let resolved = plugin_registry::resolve_model_for_kind(
        &workspace_paths::registry_dir(),
        &workspace_paths::models_dir(),
        &workspace_paths::engines_dir(),
        ModelKind::Chat,
    )
    .map_err(|e| e.to_string())?;

    let manifest = crate::local_models::build_chat_extension_manifest(
        &resolved.model_path,
        &resolved.engine_library_path,
        app_config().local_engine.context_size,
    )?;
    let process = ExtensionProcess::spawn(manifest)
        .await
        .map_err(|e| e.to_string())?;

    let result = run_generate_with_process(
        &process,
        store,
        conversation_id,
        prompt,
        system_prompt,
        tools,
        executor,
        on_piece,
    )
    .await;
    process.kill().await;
    result
}

#[tracing::instrument(skip(process, store, tools, executor, on_piece))]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_generate_with_process(
    process: &ExtensionProcess,
    store: &Arc<SqliteConversationStore>,
    conversation_id: &str,
    prompt: &str,
    system_prompt: &str,
    tools: &[tool::ToolSpec],
    executor: &ToolExecutor,
    mut on_piece: impl FnMut(&str),
) -> Result<(), String> {
    store
        .append_message(conversation_id, "user", prompt)
        .map_err(|e| e.to_string())?;

    let response = generate_with_tools_via_process(
        process,
        system_prompt,
        prompt,
        tools,
        executor,
        conversation_id,
        app_config().local_engine.max_tokens,
        &mut on_piece,
    )
    .await?;

    store
        .append_message(conversation_id, "assistant", &response)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// The tool-calling loop, now genuinely `async` end to end: talking to the
/// isolated `engine-worker` process over its stdio pipes doesn't block a
/// thread the way the old direct FFI call did, so this calls
/// `executor.call(...).await` directly instead of the previous
/// `Handle::current().block_on(...)` hack that only existed because
/// generation used to run inside a `spawn_blocking` closure. Reuses
/// `engine_host::tool_loop`'s backend-agnostic prompt/parsing helpers —
/// building the tool-catalog preamble and extracting a tool call from
/// generated text doesn't care whether the text came from an in-process
/// engine or an isolated extension process.
#[allow(clippy::too_many_arguments)]
async fn generate_with_tools_via_process(
    process: &ExtensionProcess,
    system_prompt: &str,
    user_prompt: &str,
    tools: &[tool::ToolSpec],
    executor: &ToolExecutor,
    conversation_id: &str,
    max_tokens: i32,
    mut on_piece: impl FnMut(&str),
) -> Result<String, String> {
    let mut running_prompt = format!(
        "{system_prompt}{}\n\nUser: {user_prompt}\nAssistant:",
        engine_host::tool_loop::tool_catalog_prompt(tools)
    );

    let max_tool_iterations = app_config().max_tool_iterations;
    for _ in 0..max_tool_iterations {
        let output = process
            .generate(&running_prompt, max_tokens, &mut on_piece)
            .await
            .map_err(|e| e.to_string())?;

        let Some((name, args)) = engine_host::tool_loop::extract_tool_call(&output) else {
            return Ok(output);
        };

        let result = executor.call(conversation_id, &name, args).await;
        let tool_output = match result {
            Ok(value) => value.to_string(),
            Err(err) => format!("error: {err}"),
        };

        running_prompt.push_str(&output);
        running_prompt.push_str(&format!("\nTool output: {tool_output}\nAssistant:"));
    }

    Err(format!(
        "tool-calling loop exceeded {max_tool_iterations} iterations without a final answer"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compressed_history_block_is_empty_for_a_conversation_with_no_messages() {
        let store = Arc::new(SqliteConversationStore::open_in_memory().unwrap());
        store.create_conversation("c1", "test", "default").unwrap();
        assert_eq!(compressed_history_block(&store, "c1"), "");
    }

    #[test]
    fn compressed_history_block_includes_every_message_within_budget() {
        let store = Arc::new(SqliteConversationStore::open_in_memory().unwrap());
        store.create_conversation("c1", "test", "default").unwrap();
        store.append_message("c1", "user", "hello").unwrap();
        store.append_message("c1", "assistant", "hi there").unwrap();

        let block = compressed_history_block(&store, "c1");
        assert!(block.contains("user: hello"));
        assert!(block.contains("assistant: hi there"));
    }

    #[test]
    fn compressed_history_block_drops_the_oldest_message_once_over_budget() {
        let store = Arc::new(SqliteConversationStore::open_in_memory().unwrap());
        store.create_conversation("c1", "test", "default").unwrap();
        store
            .append_message("c1", "user", &"a".repeat(8000))
            .unwrap();
        store
            .append_message("c1", "assistant", &"b".repeat(8000))
            .unwrap();

        let block = compressed_history_block(&store, "c1");
        assert!(!block.contains(&"a".repeat(8000)));
        assert!(block.contains(&"b".repeat(8000)));
    }
}
