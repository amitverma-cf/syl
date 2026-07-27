mod bootstrap;
mod commands;
mod logging;

use std::sync::Arc;

use core_types::workspace_paths;
use memory::SqliteConversationStore;

/// Shared app state available to every Tauri command.
pub struct AppState {
    pub conversation_store: Arc<SqliteConversationStore>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _log_guard = logging::init();

    bootstrap::ensure_workspace_seeded();

    let conversation_store = memory::open(&workspace_paths::conversation_db_path())
        .expect("failed to open conversation database");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            conversation_store: Arc::new(conversation_store),
        })
        .invoke_handler(tauri::generate_handler![
            commands::generate,
            commands::list_messages
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
