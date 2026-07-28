use std::sync::Mutex;

use core_types::workspace_paths;
use executor::FlowRunner;

#[derive(Default)]
pub struct FlowState {
    pub runner: Mutex<Option<FlowRunner>>,
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
    name: String,
    state: tauri::State<'_, FlowState>,
) -> Result<FlowStateInfo, String> {
    let path = workspace_paths::flows_dir().join(format!("{name}.json"));
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let flow = executor::parse_flow(&bytes).map_err(|e| e.to_string())?;
    let runner = FlowRunner::new(flow).map_err(|e| e.to_string())?;
    let info = describe(&runner);
    *state.runner.lock().unwrap() = Some(runner);
    Ok(info)
}

#[tauri::command]
pub fn flow_status(state: tauri::State<'_, FlowState>) -> Option<FlowStateInfo> {
    state.runner.lock().unwrap().as_ref().map(describe)
}

#[tauri::command]
pub fn unload_flow(state: tauri::State<'_, FlowState>) {
    *state.runner.lock().unwrap() = None;
}
