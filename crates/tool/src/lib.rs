mod context;
mod mcp;
mod native;

pub use context::compress_context;
pub use mcp::{
    load_mcp_servers, save_mcp_servers, McpServerConfig, McpToolBridge, McpToolDescriptor,
    McpTransportConfig,
};
pub use native::{ReadFileTool, RunCommandTool, WriteFileTool};

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use memory::{ToolPermissionDecision, ToolPermissionStore};

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("no tool registered with name {0}")]
    UnknownTool(String),
    #[error("permission denied for tool {0}")]
    PermissionDenied(String),
    #[error("{0}")]
    InvalidArgs(String),
    #[error("{0}")]
    Execution(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("permission store error: {0}")]
    PermissionStore(#[from] memory::MemoryError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    Allow,
    Ask,
    Deny,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn required_permission(&self) -> Permission;
    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PromptResponse {
    AllowOnce,
    AllowAlways,
    Deny,
    DenyAlways,
}

#[async_trait]
pub trait PermissionPrompter: Send + Sync {
    async fn ask(&self, tool_name: &str, args: &serde_json::Value) -> PromptResponse;
}

pub struct AlwaysApprove;

#[async_trait]
impl PermissionPrompter for AlwaysApprove {
    async fn ask(&self, _tool_name: &str, _args: &serde_json::Value) -> PromptResponse {
        PromptResponse::AllowOnce
    }
}

pub struct ToolExecutor {
    tools: RwLock<HashMap<String, Arc<dyn Tool>>>,
    prompter: Arc<dyn PermissionPrompter>,
    permissions: Arc<dyn ToolPermissionStore>,
}

impl ToolExecutor {
    pub fn new(
        prompter: Arc<dyn PermissionPrompter>,
        permissions: Arc<dyn ToolPermissionStore>,
    ) -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
            prompter,
            permissions,
        }
    }

    /// Takes `&self` (not `&mut self`) so tools can be registered after startup — e.g. an MCP
    /// server's tools, discovered only once the user connects it from the running app.
    pub fn register(&self, tool: Arc<dyn Tool>) {
        self.tools
            .write()
            .unwrap()
            .insert(tool.name().to_string(), tool);
    }

    pub fn unregister(&self, name: &str) {
        self.tools.write().unwrap().remove(name);
    }

    pub fn tool_names(&self) -> Vec<String> {
        self.tools.read().unwrap().keys().cloned().collect()
    }

    pub async fn call(
        &self,
        conversation_id: &str,
        name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, ToolError> {
        let tool = self
            .tools
            .read()
            .unwrap()
            .get(name)
            .cloned()
            .ok_or_else(|| ToolError::UnknownTool(name.to_string()))?;

        match tool.required_permission() {
            Permission::Deny => Err(ToolError::PermissionDenied(name.to_string())),
            Permission::Allow => tool.call(args).await,
            Permission::Ask => {
                let remembered = self
                    .permissions
                    .get_tool_permission(conversation_id, name)?;
                match remembered {
                    Some(ToolPermissionDecision::Allow) => tool.call(args).await,
                    Some(ToolPermissionDecision::Deny) => {
                        Err(ToolError::PermissionDenied(name.to_string()))
                    }
                    None => match self.prompter.ask(name, &args).await {
                        PromptResponse::AllowOnce => tool.call(args).await,
                        PromptResponse::AllowAlways => {
                            self.permissions.set_tool_permission(
                                conversation_id,
                                name,
                                ToolPermissionDecision::Allow,
                            )?;
                            tool.call(args).await
                        }
                        PromptResponse::Deny => Err(ToolError::PermissionDenied(name.to_string())),
                        PromptResponse::DenyAlways => {
                            self.permissions.set_tool_permission(
                                conversation_id,
                                name,
                                ToolPermissionDecision::Deny,
                            )?;
                            Err(ToolError::PermissionDenied(name.to_string()))
                        }
                    },
                }
            }
        }
    }
}
