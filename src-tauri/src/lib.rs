mod audio;
mod bootstrap;
mod commands;
mod daemon;
mod embeddings;
mod flows;
mod images;
mod local_models;
mod logging;
mod mcp;
mod models;
mod observability;
mod permission;
mod providers;
mod scheduled_jobs;
mod settings;
mod speech;
mod sync;
mod updater;

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
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(AppState {
            conversation_store: conversation_store.clone(),
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .setup(move |app| {
            settings::apply_autostart(&app.handle().clone(), settings::load_settings().autostart);

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
            app.manage(flows::WorkspaceFolderState::default());
            app.manage(local_models::LocalModelState::default());
            app.manage(images::SdModelState::default());
            app.manage(embeddings::OnnxModelState::default());
            app.manage(speech::OnnxAsrState::default());
            app.manage(speech::OnnxTtsState::default());

            app.manage(DaemonState::default());
            tauri::async_runtime::spawn(daemon::spawn(app.handle().clone()));

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
            commands::generate_flow_draft,
            commands::grant_folder_access,
            commands::quit_app,
            commands::list_messages,
            commands::list_conversations,
            commands::create_conversation,
            commands::rename_conversation,
            commands::delete_conversation,
            commands::call_tool,
            commands::list_tools,
            commands::respond_permission,
            commands::list_tool_permissions,
            commands::clear_tool_permission,
            models::list_available_models,
            models::download_model,
            local_models::list_local_models,
            local_models::set_local_model_kind,
            local_models::load_local_model,
            local_models::unload_local_model,
            local_models::delete_local_model,
            local_models::count_local_tokens,
            local_models::local_context_size,
            images::load_image_model,
            images::unload_image_model,
            images::generate_image,
            embeddings::load_embedding_model,
            embeddings::unload_embedding_model,
            embeddings::embed_text,
            speech::load_asr_model,
            speech::unload_asr_model,
            speech::load_tts_model,
            speech::unload_tts_model,
            speech::transcribe_recording,
            speech::speak_text,
            observability::system_stats,
            flows::list_flows,
            flows::load_flow,
            flows::flow_status,
            flows::unload_flow,
            flows::get_flow,
            flows::save_flow,
            flows::delete_flow,
            flows::validate_flow_json,
            providers::list_providers,
            providers::set_provider_api_key,
            providers::delete_provider_api_key,
            providers::list_cloud_models,
            providers::list_custom_providers,
            providers::add_custom_provider,
            providers::update_custom_provider,
            providers::remove_custom_provider,
            mcp::list_mcp_servers,
            mcp::add_mcp_server,
            mcp::remove_mcp_server,
            scheduled_jobs::list_scheduled_jobs,
            scheduled_jobs::add_scheduled_job,
            scheduled_jobs::remove_scheduled_job,
            updater::check_for_update,
            updater::install_update,
            settings::get_settings,
            settings::update_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
