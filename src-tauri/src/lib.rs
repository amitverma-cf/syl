mod bootstrap;
mod commands;
mod flows;
mod logging;
mod models;
mod observability;
mod permission;

use std::sync::Arc;

use core_types::workspace_paths;
use memory::SqliteConversationStore;
use observability::ObservabilityState;
use permission::TauriPermissionPrompter;
use tauri::Manager;
use tool::{ReadFileTool, RunCommandTool, ToolExecutor, WriteFileTool};

pub struct AppState {
    pub conversation_store: Arc<SqliteConversationStore>,
}

pub struct ToolState {
    pub executor: Arc<ToolExecutor>,
    pub prompter: Arc<TauriPermissionPrompter>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _log_guard = logging::init();

    bootstrap::ensure_workspace_seeded();

    let conversation_store = Arc::new(
        memory::open(&workspace_paths::conversation_db_path())
            .expect("failed to open conversation database"),
    );
    let permission_store = conversation_store.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            conversation_store: conversation_store.clone(),
        })
        .setup(move |app| {
            let prompter = Arc::new(TauriPermissionPrompter::new(app.handle().clone()));

            let workspace_root = workspace_paths::workspace_root().join("workspace");
            std::fs::create_dir_all(&workspace_root)?;

            let mut executor = ToolExecutor::new(prompter.clone(), permission_store);
            executor.register(Arc::new(ReadFileTool {
                workspace_root: workspace_root.clone(),
            }));
            executor.register(Arc::new(WriteFileTool {
                workspace_root: workspace_root.clone(),
            }));
            executor.register(Arc::new(RunCommandTool { workspace_root }));

            app.manage(ToolState {
                executor: Arc::new(executor),
                prompter,
            });

            let observability_state = Arc::new(ObservabilityState::default());
            observability::spawn_sampler(observability_state.clone());
            app.manage(observability_state);

            app.manage(flows::FlowState::default());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::generate,
            commands::list_messages,
            commands::call_tool,
            commands::respond_permission,
            models::list_available_models,
            models::download_model,
            observability::system_stats,
            flows::list_flows,
            flows::load_flow,
            flows::flow_status,
            flows::unload_flow,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
