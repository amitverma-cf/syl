use std::collections::HashMap;
use std::sync::Mutex;

use core_types::workspace_paths;
use executor::FlowRunner;

/// The flow every new conversation starts on, unless the user picks a different one — its
/// definition ships in the repo-root `flows/` directory like every other flow (see
/// `flows/default.json`).
pub const DEFAULT_FLOW_NAME: &str = "default";

/// One `FlowRunner` per conversation — conversations run independent flow instances, not a
/// single global flow, so switching between chats doesn't reset or share state.
#[derive(Default)]
pub struct FlowState {
    runners: Mutex<HashMap<String, FlowRunner>>,
}

/// What a chat turn needs from the active flow: the system prompt and the state's tool
/// allowlist (empty means every registered tool is allowed — the model itself decides
/// whether and which tool to call, via the tool-calling loop in `commands::generate`).
pub struct FlowTurn {
    pub system_prompt: String,
    pub tool_allowlist: Vec<String>,
}

impl FlowState {
    /// Loads the `default` flow for `conversation_id` first if it has no runner yet, then
    /// returns the current state's turn info.
    pub fn ensure_and_take_turn(&self, conversation_id: &str) -> Result<FlowTurn, String> {
        let mut runners = self.runners.lock().unwrap();
        if !runners.contains_key(conversation_id) {
            let runner = load_flow_runner(DEFAULT_FLOW_NAME)?;
            runners.insert(conversation_id.to_string(), runner);
        }
        let runner = runners.get(conversation_id).expect("just inserted above");
        Ok(FlowTurn {
            system_prompt: runner.current_state().system_prompt.clone(),
            tool_allowlist: runner.current_state().tool_allowlist.clone(),
        })
    }

    pub fn advance(&self, conversation_id: &str, trigger: &str) -> Option<FlowStateInfo> {
        let mut runners = self.runners.lock().unwrap();
        let runner = runners.get_mut(conversation_id)?;
        runner.advance(trigger);
        Some(describe(runner))
    }
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowStateInfo {
    pub flow_name: String,
    pub state_name: String,
    pub system_prompt: String,
}

fn describe(runner: &FlowRunner) -> FlowStateInfo {
    FlowStateInfo {
        flow_name: runner.flow_name().to_string(),
        state_name: runner.current_state().name.clone(),
        system_prompt: runner.current_state().system_prompt.clone(),
    }
}

fn load_flow_runner(name: &str) -> Result<FlowRunner, String> {
    let path = workspace_paths::flows_dir().join(format!("{name}.json"));
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let flow = executor::parse_flow(&bytes).map_err(|e| e.to_string())?;
    FlowRunner::new(flow).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_flows() -> Result<Vec<String>, String> {
    let dir = workspace_paths::flows_dir();
    let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
    let names = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            entry
                .path()
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
        })
        .collect();
    Ok(names)
}

#[tauri::command]
pub fn load_flow(
    conversation_id: String,
    name: String,
    state: tauri::State<'_, FlowState>,
) -> Result<FlowStateInfo, String> {
    let runner = load_flow_runner(&name)?;
    let info = describe(&runner);
    state
        .runners
        .lock()
        .unwrap()
        .insert(conversation_id, runner);
    Ok(info)
}

#[tauri::command]
pub fn flow_status(
    conversation_id: String,
    state: tauri::State<'_, FlowState>,
) -> Option<FlowStateInfo> {
    state
        .runners
        .lock()
        .unwrap()
        .get(&conversation_id)
        .map(describe)
}

#[tauri::command]
pub fn unload_flow(conversation_id: String, state: tauri::State<'_, FlowState>) {
    state.runners.lock().unwrap().remove(&conversation_id);
}
