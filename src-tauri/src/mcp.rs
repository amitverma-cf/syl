use std::collections::HashMap;
use std::sync::Mutex;

use core_types::workspace_paths;
use tauri::Manager;
use tool::{
    McpConnectionHandle, McpServerConfig, McpToolBridge, McpToolDescriptor, McpTransportConfig,
    Tool,
};

use crate::ToolState;

struct ConnectedServer {
    tool_names: Vec<String>,
    handle: McpConnectionHandle,
}

#[derive(Default)]
pub struct McpState {
    connections: Mutex<HashMap<String, ConnectedServer>>,
}

#[tauri::command]
pub fn list_mcp_servers() -> Vec<McpServerConfig> {
    tool::load_mcp_servers(&workspace_paths::mcp_servers_file())
}

#[tauri::command]
pub async fn add_mcp_server(
    name: String,
    transport: McpTransportConfig,
    tool_state: tauri::State<'_, ToolState>,
    mcp_state: tauri::State<'_, McpState>,
) -> Result<Vec<McpToolDescriptor>, String> {
    let config = McpServerConfig {
        name: name.clone(),
        transport,
    };
    let (bridges, descriptors, handle) = McpToolBridge::connect(&config)
        .await
        .map_err(|e| e.to_string())?;

    let tool_names: Vec<String> = bridges.iter().map(|b| b.name().to_string()).collect();
    for bridge in bridges {
        tool_state.executor.register(std::sync::Arc::new(bridge));
    }
    crate::sync::lock(&mcp_state.connections)
        .insert(name.clone(), ConnectedServer { tool_names, handle });

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
    if let Some(connected) = crate::sync::lock(&mcp_state.connections).remove(&name) {
        for tool_name in &connected.tool_names {
            tool_state.executor.unregister(tool_name);
        }
        // Explicitly cancel the transport rather than only relying on the last
        // Arc<RunningService> clone being dropped once every bridge above is
        // unregistered — deterministic, immediate shutdown (and a real child-process
        // kill for the stdio transport) instead of "eventually, whenever".
        connected.handle.disconnect();
    }

    let path = workspace_paths::mcp_servers_file();
    let mut servers = tool::load_mcp_servers(&path);
    servers.retain(|s| s.name != name);
    tool::save_mcp_servers(&path, &servers).map_err(|e| e.to_string())
}

pub fn reconnect_saved_servers(app: &tauri::AppHandle) {
    let servers = tool::load_mcp_servers(&workspace_paths::mcp_servers_file());
    if servers.is_empty() {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        for config in servers {
            match McpToolBridge::connect(&config).await {
                Ok((bridges, _descriptors, handle)) => {
                    let tool_state = app.state::<ToolState>();
                    let mcp_state = app.state::<McpState>();
                    let tool_names: Vec<String> =
                        bridges.iter().map(|b| b.name().to_string()).collect();
                    for bridge in bridges {
                        tool_state.executor.register(std::sync::Arc::new(bridge));
                    }
                    crate::sync::lock(&mcp_state.connections)
                        .insert(config.name.clone(), ConnectedServer { tool_names, handle });
                    tracing::info!(server = %config.name, "reconnected saved MCP server");
                }
                Err(err) => {
                    tracing::warn!(server = %config.name, ?err, "failed to reconnect saved MCP server");
                }
            }
        }
    });
}
