mod bootstrap;
mod commands;
mod daemon;
mod flows;
mod logging;
mod mcp;
mod models;
mod observability;
mod permission;
mod providers;

use std::sync::Arc;

use core_types::workspace_paths;
use daemon::DaemonState;
use memory::SqliteConversationStore;
use observability::ObservabilityState;
use permission::TauriPermissionPrompter;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
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
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(AppState {
            conversation_store: conversation_store.clone(),
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Hide instead of quitting so the daemon (cron jobs, event bus) keeps running
                // in the background — matches the tray-resident, not-a-true-OS-service design.
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .setup(move |app| {
            let prompter = Arc::new(TauriPermissionPrompter::new(app.handle().clone()));

            let workspace_root = workspace_paths::workspace_root().join("workspace");
            std::fs::create_dir_all(&workspace_root)?;

            let executor = ToolExecutor::new(prompter.clone(), permission_store);
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
            app.manage(mcp::McpState::default());
            mcp::reconnect_saved_servers(&app.handle().clone());

            let observability_state = Arc::new(ObservabilityState::default());
            observability::spawn_sampler(observability_state.clone());
            app.manage(observability_state);

            app.manage(flows::FlowState::default());

            let daemon_state = DaemonState::default();
            let event_bus = daemon_state.event_bus.clone();
            app.manage(daemon_state);
            tauri::async_runtime::spawn(daemon::spawn(event_bus));

            let show_item = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&show_item, &quit_item])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&tray_menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

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
            providers::list_providers,
            providers::set_provider_api_key,
            providers::list_cloud_models,
            providers::list_custom_providers,
            providers::add_custom_provider,
            mcp::list_mcp_servers,
            mcp::add_mcp_server,
            mcp::remove_mcp_server,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
