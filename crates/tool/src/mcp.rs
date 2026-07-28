use rmcp::model::CallToolRequestParams;
use rmcp::service::RunningService;
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
use rmcp::{RoleClient, ServiceExt};

use crate::{Permission, Tool, ToolError};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

pub fn load_mcp_servers(path: &std::path::Path) -> Vec<McpServerConfig> {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&contents).unwrap_or_default()
}

pub fn save_mcp_servers(
    path: &std::path::Path,
    servers: &[McpServerConfig],
) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(servers).map_err(std::io::Error::from)?;
    std::fs::write(path, json)
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolDescriptor {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

/// Bridges one tool exposed by a connected MCP server into this app's own `Tool` trait, so
/// MCP tools flow through the same permission gate and `ToolExecutor` as native tools —
/// callers don't need to know a tool came from MCP rather than being built in.
pub struct McpToolBridge {
    qualified_name: String,
    tool_name: String,
    client: std::sync::Arc<RunningService<RoleClient, ()>>,
}

impl McpToolBridge {
    /// Connects to an MCP server over stdio (spawning `command args...` as a child process)
    /// and returns one bridge per tool the server advertises, plus their schemas for display.
    pub async fn connect(
        config: &McpServerConfig,
    ) -> Result<(Vec<Self>, Vec<McpToolDescriptor>), ToolError> {
        let transport = TokioChildProcess::new(
            tokio::process::Command::new(&config.command).configure(|cmd| {
                cmd.args(&config.args);
            }),
        )
        .map_err(|e| ToolError::Execution(format!("failed to spawn MCP server: {e}")))?;

        let client = ()
            .serve(transport)
            .await
            .map_err(|e| ToolError::Execution(format!("failed to connect to MCP server: {e}")))?;
        let client = std::sync::Arc::new(client);

        let tools = client
            .list_all_tools()
            .await
            .map_err(|e| ToolError::Execution(format!("failed to list MCP tools: {e}")))?;

        let descriptors = tools
            .iter()
            .map(|t| McpToolDescriptor {
                name: t.name.to_string(),
                description: t.description.as_ref().map(|d| d.to_string()),
                input_schema: serde_json::Value::Object((*t.input_schema).clone()),
            })
            .collect();

        let bridges = tools
            .into_iter()
            .map(|t| Self {
                qualified_name: format!("mcp::{}::{}", config.name, t.name),
                tool_name: t.name.to_string(),
                client: client.clone(),
            })
            .collect();

        Ok((bridges, descriptors))
    }
}

#[async_trait::async_trait]
impl Tool for McpToolBridge {
    fn name(&self) -> &str {
        // Namespaced by server name (`mcp::<server>::<tool>`) so two MCP servers can't
        // collide on a tool name inside `ToolExecutor`'s single flat tool map.
        &self.qualified_name
    }

    fn required_permission(&self) -> Permission {
        // MCP servers are arbitrary third-party code with real side effects (filesystem,
        // network, other services) — always ask, same posture as run_command.
        Permission::Ask
    }

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let arguments = match args {
            serde_json::Value::Object(map) => Some(map),
            serde_json::Value::Null => None,
            other => {
                return Err(ToolError::InvalidArgs(format!(
                    "MCP tool arguments must be a JSON object, got {other}"
                )))
            }
        };

        let mut request = CallToolRequestParams::new(self.tool_name.clone());
        if let Some(arguments) = arguments {
            request = request.with_arguments(arguments);
        }
        let result = self
            .client
            .call_tool(request)
            .await
            .map_err(|e| ToolError::Execution(format!("MCP tool call failed: {e}")))?;

        serde_json::to_value(result)
            .map_err(|e| ToolError::Execution(format!("failed to serialize MCP result: {e}")))
    }
}
