use std::collections::HashMap;
use std::path::PathBuf;
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

/// The folder a user has opened via the native folder dialog (see
/// `commands::grant_folder_access`), if any. Its own `flows/` subdirectory, when
/// present, takes precedence over `.syl/flows/` for same-named flows — the project-local
/// override `plan.md` originally scoped ("a workspace folder can contain its own
/// flows/... project-specific flows override/extend the global .syl/flows/") but never
/// wired up until now.
#[derive(Default)]
pub struct WorkspaceFolderState {
    path: Mutex<Option<PathBuf>>,
}

impl WorkspaceFolderState {
    pub fn set(&self, path: PathBuf) {
        *crate::sync::lock(&self.path) = Some(path);
    }

    fn flows_dir(&self) -> Option<PathBuf> {
        crate::sync::lock(&self.path)
            .as_ref()
            .map(|root| root.join("flows"))
    }
}

pub struct FlowTurn {
    pub system_prompt: String,
    pub tool_allowlist: Vec<String>,
}

impl FlowState {
    pub fn ensure_and_take_turn(
        &self,
        conversation_id: &str,
        workspace_folder: &WorkspaceFolderState,
    ) -> Result<FlowTurn, String> {
        let mut runners = crate::sync::lock(&self.runners);
        if !runners.contains_key(conversation_id) {
            let runner = load_flow_runner(default_flow_name(), workspace_folder)?;
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

/// Resolves `<name>.json`, preferring the open workspace folder's own `flows/`
/// directory over the global `.syl/flows/` — a project-local flow silently shadows a
/// global one with the same name, the same way a project-local config typically wins
/// over a user-global one in other tools.
fn resolve_flow_path(name: &str, workspace_folder: &WorkspaceFolderState) -> PathBuf {
    if let Some(dir) = workspace_folder.flows_dir() {
        let candidate = dir.join(format!("{name}.json"));
        if candidate.is_file() {
            return candidate;
        }
    }
    workspace_paths::flows_dir().join(format!("{name}.json"))
}

fn load_flow_runner(
    name: &str,
    workspace_folder: &WorkspaceFolderState,
) -> Result<FlowRunner, String> {
    let path = resolve_flow_path(name, workspace_folder);
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let flow = executor::parse_flow(&bytes).map_err(|e| e.to_string())?;
    FlowRunner::new(flow).map_err(|e| e.to_string())
}

/// Every flow name visible from either source, deduplicated so a project-local flow
/// that shadows a global one of the same name is listed once, not twice.
#[tauri::command]
pub fn list_flows(
    workspace_folder: tauri::State<'_, WorkspaceFolderState>,
) -> Result<Vec<String>, String> {
    let mut names: Vec<String> = flow_names_in(&workspace_paths::flows_dir())?;
    if let Some(dir) = workspace_folder.flows_dir() {
        for name in flow_names_in(&dir).unwrap_or_default() {
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }
    Ok(names)
}

fn flow_names_in(dir: &std::path::Path) -> Result<Vec<String>, String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(Vec::new());
    };
    Ok(entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            entry
                .path()
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
        })
        .collect())
}

#[tauri::command]
pub fn load_flow(
    conversation_id: String,
    name: String,
    state: tauri::State<'_, FlowState>,
    workspace_folder: tauri::State<'_, WorkspaceFolderState>,
) -> Result<FlowStateInfo, String> {
    let runner = load_flow_runner(&name, &workspace_folder)?;
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
/// for the flow editor to load into the canvas. Same project-local-overrides-global
/// precedence as `load_flow`/`list_flows`.
#[tauri::command]
pub fn get_flow(
    name: String,
    workspace_folder: tauri::State<'_, WorkspaceFolderState>,
) -> Result<Flow, String> {
    safe_flow_name(&name)?;
    let path = resolve_flow_path(&name, &workspace_folder);
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    parse_flow(&bytes).map_err(|e| e.to_string())
}

/// Validates and writes a flow definition to `.syl/flows/<name>.json`.
/// Refuses to write anything that doesn't pass the same schema/semantic
/// validation used when loading a flow into a live conversation.
///
/// Always writes to the global `.syl/flows/` directory, not a project-local one, even
/// when a workspace folder is open — an explicit "save as project-local" action would
/// be a reasonable future addition, but silently choosing which directory to write to
/// based on whichever a same-named file was last *read* from would be surprising.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "syl-flows-test-{label}-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn resolve_flow_path_falls_back_to_global_when_no_workspace_folder_is_open() {
        let workspace_folder = WorkspaceFolderState::default();
        let path = resolve_flow_path("default", &workspace_folder);
        assert_eq!(path, workspace_paths::flows_dir().join("default.json"));
    }

    #[test]
    fn resolve_flow_path_falls_back_to_global_when_the_project_local_file_does_not_exist() {
        let project = temp_dir("no-local-file");
        let workspace_folder = WorkspaceFolderState::default();
        workspace_folder.set(project.clone());

        let path = resolve_flow_path("default", &workspace_folder);
        assert_eq!(path, workspace_paths::flows_dir().join("default.json"));

        std::fs::remove_dir_all(&project).unwrap();
    }

    #[test]
    fn resolve_flow_path_prefers_the_project_local_file_when_it_exists() {
        let project = temp_dir("has-local-file");
        std::fs::create_dir_all(project.join("flows")).unwrap();
        std::fs::write(project.join("flows").join("custom.json"), "{}").unwrap();

        let workspace_folder = WorkspaceFolderState::default();
        workspace_folder.set(project.clone());

        let path = resolve_flow_path("custom", &workspace_folder);
        assert_eq!(path, project.join("flows").join("custom.json"));

        std::fs::remove_dir_all(&project).unwrap();
    }

    #[test]
    fn flow_names_in_a_missing_directory_returns_empty_rather_than_erroring() {
        let missing = temp_dir("missing").join("does-not-exist");
        assert_eq!(flow_names_in(&missing).unwrap(), Vec::<String>::new());
    }
}
