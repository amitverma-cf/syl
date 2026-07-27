//! Async tool execution, gated by a per-tool permission level.

use async_trait::async_trait;
use core_types::CoreResult;

/// The permission level required before a tool call is allowed to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    /// Runs without prompting the user.
    Allow,
    /// Prompts the user for approval before each call.
    Ask,
    /// Never runs.
    Deny,
}

/// A callable tool the agent can invoke.
#[async_trait]
pub trait Tool: Send + Sync {
    /// The tool's unique name, as referenced in flow/tool-allowlist definitions.
    fn name(&self) -> &str;
    /// The permission level required to call this tool.
    fn required_permission(&self) -> Permission;
    /// Runs the tool with the given arguments and returns its result.
    ///
    /// # Errors
    /// Returns an error if the tool call fails.
    async fn call(&self, args: serde_json::Value) -> CoreResult<serde_json::Value>;
}
