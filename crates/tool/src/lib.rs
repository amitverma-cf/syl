mod context;
mod native;

pub use context::compress_context;
pub use native::{ReadFileTool, RunCommandTool, WriteFileTool};

use std::collections::HashMap;
use std::sync::Arc;

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
    tools: HashMap<String, Arc<dyn Tool>>,
    prompter: Arc<dyn PermissionPrompter>,
    permissions: Arc<dyn ToolPermissionStore>,
}

impl ToolExecutor {
    pub fn new(
        prompter: Arc<dyn PermissionPrompter>,
        permissions: Arc<dyn ToolPermissionStore>,
    ) -> Self {
        Self {
            tools: HashMap::new(),
            prompter,
            permissions,
        }
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub async fn call(
        &self,
        conversation_id: &str,
        name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, ToolError> {
        let tool = self
            .tools
            .get(name)
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
