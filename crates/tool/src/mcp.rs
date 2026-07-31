use rmcp::model::CallToolRequestParams;
use rmcp::service::RunningService;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::transport::{ConfigureCommandExt, StreamableHttpClientTransport, TokioChildProcess};
use rmcp::{RoleClient, ServiceExt};

use crate::{Permission, Tool, ToolError};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", tag = "transport")]
pub enum McpTransportConfig {
    #[serde(rename = "stdio")]
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
    },
    #[serde(rename = "http")]
    Http {
        url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bearer_token: Option<String>,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    #[serde(flatten)]
    pub transport: McpTransportConfig,
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

pub struct McpToolBridge {
    qualified_name: String,
    tool_name: String,
    description: String,
    input_schema: serde_json::Value,
    client: std::sync::Arc<RunningService<RoleClient, ()>>,
}

impl McpToolBridge {
    pub async fn connect(
        config: &McpServerConfig,
    ) -> Result<(Vec<Self>, Vec<McpToolDescriptor>), ToolError> {
        let client: RunningService<RoleClient, ()> = match &config.transport {
            McpTransportConfig::Stdio { command, args } => {
                let transport = TokioChildProcess::new(
                    tokio::process::Command::new(command).configure(|cmd| {
                        cmd.args(args);
                    }),
                )
                .map_err(|e| ToolError::Execution(format!("failed to spawn MCP server: {e}")))?;
                ().serve(transport).await.map_err(|e| {
                    ToolError::Execution(format!("failed to connect to MCP server: {e}"))
                })?
            }
            McpTransportConfig::Http { url, bearer_token } => {
                let mut http_config = StreamableHttpClientTransportConfig::default();
                http_config.uri = url.clone().into();
                http_config.auth_header = bearer_token.clone();
                let transport = StreamableHttpClientTransport::from_config(http_config);
                ().serve(transport).await.map_err(|e| {
                    ToolError::Execution(format!("failed to connect to MCP server: {e}"))
                })?
            }
        };
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
                description: t
                    .description
                    .as_ref()
                    .map(|d| d.to_string())
                    .unwrap_or_default(),
                input_schema: serde_json::Value::Object((*t.input_schema).clone()),
                client: client.clone(),
            })
            .collect();

        Ok((bridges, descriptors))
    }
}

#[async_trait::async_trait]
impl Tool for McpToolBridge {
    fn name(&self) -> &str {
        &self.qualified_name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn input_schema(&self) -> serde_json::Value {
        self.input_schema.clone()
    }

    fn required_permission(&self) -> Permission {
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
