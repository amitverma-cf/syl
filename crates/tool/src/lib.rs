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
