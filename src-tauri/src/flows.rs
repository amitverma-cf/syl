use std::collections::HashMap;
use std::sync::Mutex;

use core_types::app_config::app_config;
use core_types::workspace_paths;
use executor::{parse_flow, Flow, FlowRunner};

pub fn default_flow_name() -> &'static str {
    &app_config().default_flow_name
}

#[derive(Default)]
pub struct FlowState {
    runners: Mutex<HashMap<String, FlowRunner>>,
}

pub struct FlowTurn {
    pub system_prompt: String,
    pub tool_allowlist: Vec<String>,
}

impl FlowState {
    pub fn ensure_and_take_turn(&self, conversation_id: &str) -> Result<FlowTurn, String> {
        let mut runners = crate::sync::lock(&self.runners);
        if !runners.contains_key(conversation_id) {
            let runner = load_flow_runner(default_flow_name())?;
            runners.insert(conversation_id.to_string(), runner);
        }
        let runner = runners.get(conversation_id).expect("just inserted above");
        Ok(FlowTurn {
            system_prompt: runner.current_state().system_prompt.clone(),
            tool_allowlist: runner.current_state().tool_allowlist.clone(),
        })
    }

    pub fn advance(&self, conversation_id: &str, trigger: &str) -> Option<FlowStateInfo> {
        let mut runners = crate::sync::lock(&self.runners);
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
    crate::sync::lock(&state.runners).insert(conversation_id, runner);
    Ok(info)
}

#[tauri::command]
pub fn flow_status(
    conversation_id: String,
    state: tauri::State<'_, FlowState>,
) -> Option<FlowStateInfo> {
    crate::sync::lock(&state.runners)
        .get(&conversation_id)
        .map(describe)
}

#[tauri::command]
pub fn unload_flow(conversation_id: String, state: tauri::State<'_, FlowState>) {
    crate::sync::lock(&state.runners).remove(&conversation_id);
}

fn safe_flow_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains("..")
        || name.contains(std::path::MAIN_SEPARATOR)
    {
        return Err(format!("invalid flow name: {name:?}"));
    }
    Ok(())
}

/// Parses and validates a flow definition without touching disk. Used by the
/// flow editor to check hand-edited or AI-generated JSON before it can be
/// inserted into the canvas or saved.
#[tauri::command]
pub fn validate_flow_json(json: String) -> Result<Flow, String> {
    parse_flow(json.as_bytes()).map_err(|e| e.to_string())
}

/// Reads a flow file straight off disk (not tied to any conversation),
/// for the flow editor to load into the canvas.
#[tauri::command]
pub fn get_flow(name: String) -> Result<Flow, String> {
    safe_flow_name(&name)?;
    let path = workspace_paths::flows_dir().join(format!("{name}.json"));
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    parse_flow(&bytes).map_err(|e| e.to_string())
}

/// Validates and writes a flow definition to `.syl/flows/<name>.json`.
/// Refuses to write anything that doesn't pass the same schema/semantic
/// validation used when loading a flow into a live conversation.
#[tauri::command]
pub fn save_flow(name: String, json: String) -> Result<(), String> {
    safe_flow_name(&name)?;
    let flow = parse_flow(json.as_bytes()).map_err(|e| e.to_string())?;
    if flow.name != name {
        return Err(format!(
            "flow name in file ({:?}) does not match save target ({name:?})",
            flow.name
        ));
    }
    let dir = workspace_paths::flows_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{name}.json"));
    let pretty = serde_json::to_string_pretty(&flow).map_err(|e| e.to_string())?;
    std::fs::write(&path, pretty).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_flow(name: String) -> Result<(), String> {
    safe_flow_name(&name)?;
    let path = workspace_paths::flows_dir().join(format!("{name}.json"));
    std::fs::remove_file(&path).map_err(|e| e.to_string())
}
