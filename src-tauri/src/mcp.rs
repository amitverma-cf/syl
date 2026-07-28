use std::collections::HashMap;
use std::sync::Mutex;

use core_types::workspace_paths;
use tauri::Manager;
use tool::{McpServerConfig, McpToolBridge, McpToolDescriptor, Tool};

use crate::ToolState;

/// Tracks which qualified tool names came from which connected MCP server, so a server can be
/// disconnected (and its tools unregistered from the shared `ToolExecutor`) without restarting
/// the app.
#[derive(Default)]
pub struct McpState {
    server_tool_names: Mutex<HashMap<String, Vec<String>>>,
}

#[tauri::command]
pub fn list_mcp_servers() -> Vec<McpServerConfig> {
    tool::load_mcp_servers(&workspace_paths::mcp_servers_file())
}

#[tauri::command]
pub async fn add_mcp_server(
    name: String,
    command: String,
    args: Vec<String>,
    tool_state: tauri::State<'_, ToolState>,
    mcp_state: tauri::State<'_, McpState>,
) -> Result<Vec<McpToolDescriptor>, String> {
    let config = McpServerConfig {
        name: name.clone(),
        command,
        args,
    };
    let (bridges, descriptors) = McpToolBridge::connect(&config)
        .await
        .map_err(|e| e.to_string())?;

    let tool_names: Vec<String> = bridges.iter().map(|b| b.name().to_string()).collect();
    for bridge in bridges {
        tool_state.executor.register(std::sync::Arc::new(bridge));
    }
    mcp_state
        .server_tool_names
        .lock()
        .unwrap()
        .insert(name.clone(), tool_names);

    let path = workspace_paths::mcp_servers_file();
    let mut servers = tool::load_mcp_servers(&path);
    servers.retain(|s| s.name != config.name);
    servers.push(config);
    tool::save_mcp_servers(&path, &servers).map_err(|e| e.to_string())?;

    Ok(descriptors)
}

#[tauri::command]
pub fn remove_mcp_server(
    name: String,
    tool_state: tauri::State<'_, ToolState>,
    mcp_state: tauri::State<'_, McpState>,
) -> Result<(), String> {
    if let Some(tool_names) = mcp_state.server_tool_names.lock().unwrap().remove(&name) {
        for tool_name in tool_names {
            tool_state.executor.unregister(&tool_name);
        }
    }

    let path = workspace_paths::mcp_servers_file();
    let mut servers = tool::load_mcp_servers(&path);
    servers.retain(|s| s.name != name);
    tool::save_mcp_servers(&path, &servers).map_err(|e| e.to_string())
}

/// Reconnects every MCP server saved from a previous run. Failures are logged, not fatal —
/// a server that's since become unreachable shouldn't block the rest of the app from starting.
pub fn reconnect_saved_servers(app: &tauri::AppHandle) {
    let servers = tool::load_mcp_servers(&workspace_paths::mcp_servers_file());
    if servers.is_empty() {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        for config in servers {
            match McpToolBridge::connect(&config).await {
                Ok((bridges, _descriptors)) => {
                    let tool_state = app.state::<ToolState>();
                    let mcp_state = app.state::<McpState>();
                    let tool_names: Vec<String> =
                        bridges.iter().map(|b| b.name().to_string()).collect();
                    for bridge in bridges {
                        tool_state.executor.register(std::sync::Arc::new(bridge));
                    }
                    mcp_state
                        .server_tool_names
                        .lock()
                        .unwrap()
                        .insert(config.name.clone(), tool_names);
                    tracing::info!(server = %config.name, "reconnected saved MCP server");
                }
                Err(err) => {
                    tracing::warn!(server = %config.name, ?err, "failed to reconnect saved MCP server");
                }
            }
        }
    });
}
