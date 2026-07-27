//! Tool pillar: async-first tool execution with permission tiers (Allow/Ask/Deny),
//! sandboxing, native tools + MCP-client escape hatch. Decision #12: keep agent.cpp's
//! permission model, rebuild the execution path async-first (fixes its blocking-tools flaw).

use async_trait::async_trait;
use core_types::CoreResult;

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
    async fn call(&self, args: serde_json::Value) -> CoreResult<serde_json::Value>;
}
