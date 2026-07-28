use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use memory::SqliteConversationStore;
use serde_json::{json, Value};
use tool::{Permission, PermissionPrompter, PromptResponse, Tool, ToolError, ToolExecutor};

struct EchoTool {
    permission: Permission,
}

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "Echoes its input back unchanged, for tests."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({ "type": "object", "properties": {} })
    }

    fn required_permission(&self) -> Permission {
        self.permission
    }

    async fn call(&self, args: Value) -> Result<Value, ToolError> {
        Ok(args)
    }
}

struct FixedPrompter(PromptResponse);

#[async_trait]
impl PermissionPrompter for FixedPrompter {
    async fn ask(&self, _tool_name: &str, _args: &Value) -> PromptResponse {
        self.0
    }
}

struct TrackingPrompter {
    called: Arc<AtomicBool>,
    response: PromptResponse,
}

#[async_trait]
impl PermissionPrompter for TrackingPrompter {
    async fn ask(&self, _tool_name: &str, _args: &Value) -> PromptResponse {
        self.called.store(true, Ordering::SeqCst);
        self.response
    }
}

fn permission_store() -> Arc<SqliteConversationStore> {
    Arc::new(SqliteConversationStore::open_in_memory().unwrap())
}

#[tokio::test]
async fn allow_permission_runs_without_prompting() {
    let called = Arc::new(AtomicBool::new(false));
    let executor = ToolExecutor::new(
        Arc::new(TrackingPrompter {
            called: called.clone(),
            response: PromptResponse::AllowOnce,
        }),
        permission_store(),
    );
    executor.register(Arc::new(EchoTool {
        permission: Permission::Allow,
    }));

    let result = executor.call("c1", "echo", json!({"x": 1})).await.unwrap();
    assert_eq!(result, json!({"x": 1}));
    assert!(!called.load(Ordering::SeqCst));
}

#[tokio::test]
async fn deny_permission_never_calls_the_tool_or_prompter() {
    let called = Arc::new(AtomicBool::new(false));
    let executor = ToolExecutor::new(
        Arc::new(TrackingPrompter {
            called: called.clone(),
            response: PromptResponse::AllowOnce,
        }),
        permission_store(),
    );
    executor.register(Arc::new(EchoTool {
        permission: Permission::Deny,
    }));

    let err = executor.call("c1", "echo", json!({})).await.unwrap_err();
    assert!(matches!(err, ToolError::PermissionDenied(name) if name == "echo"));
    assert!(!called.load(Ordering::SeqCst));
}

#[tokio::test]
async fn ask_permission_runs_the_tool_once_when_approved_once() {
    let executor = ToolExecutor::new(
        Arc::new(FixedPrompter(PromptResponse::AllowOnce)),
        permission_store(),
    );
    let executor = executor;
    executor.register(Arc::new(EchoTool {
        permission: Permission::Ask,
    }));

    let result = executor.call("c1", "echo", json!({"x": 2})).await.unwrap();
    assert_eq!(result, json!({"x": 2}));
}

#[tokio::test]
async fn ask_permission_denies_once_when_rejected() {
    let executor = ToolExecutor::new(
        Arc::new(FixedPrompter(PromptResponse::Deny)),
        permission_store(),
    );
    executor.register(Arc::new(EchoTool {
        permission: Permission::Ask,
    }));

    let err = executor.call("c1", "echo", json!({})).await.unwrap_err();
    assert!(matches!(err, ToolError::PermissionDenied(name) if name == "echo"));
}

#[tokio::test]
async fn allow_always_is_remembered_and_stops_prompting() {
    let called = Arc::new(AtomicBool::new(false));
    let executor = ToolExecutor::new(
        Arc::new(TrackingPrompter {
            called: called.clone(),
            response: PromptResponse::AllowAlways,
        }),
        permission_store(),
    );
    executor.register(Arc::new(EchoTool {
        permission: Permission::Ask,
    }));

    executor.call("c1", "echo", json!({})).await.unwrap();
    assert!(called.load(Ordering::SeqCst));

    called.store(false, Ordering::SeqCst);
    let result = executor.call("c1", "echo", json!({"y": 1})).await.unwrap();
    assert_eq!(result, json!({"y": 1}));
    assert!(!called.load(Ordering::SeqCst));
}

#[tokio::test]
async fn deny_always_is_remembered_and_stops_prompting() {
    let called = Arc::new(AtomicBool::new(false));
    let executor = ToolExecutor::new(
        Arc::new(TrackingPrompter {
            called: called.clone(),
            response: PromptResponse::DenyAlways,
        }),
        permission_store(),
    );
    executor.register(Arc::new(EchoTool {
        permission: Permission::Ask,
    }));

    executor.call("c1", "echo", json!({})).await.unwrap_err();
    assert!(called.load(Ordering::SeqCst));

    called.store(false, Ordering::SeqCst);
    let err = executor.call("c1", "echo", json!({})).await.unwrap_err();
    assert!(matches!(err, ToolError::PermissionDenied(_)));
    assert!(!called.load(Ordering::SeqCst));
}

#[tokio::test]
async fn remembered_permission_is_scoped_per_conversation() {
    let called = Arc::new(AtomicBool::new(false));
    let executor = ToolExecutor::new(
        Arc::new(TrackingPrompter {
            called: called.clone(),
            response: PromptResponse::AllowAlways,
        }),
        permission_store(),
    );
    executor.register(Arc::new(EchoTool {
        permission: Permission::Ask,
    }));

    executor.call("c1", "echo", json!({})).await.unwrap();
    assert!(called.load(Ordering::SeqCst));

    // c2 has never been granted anything, so even though c1's "always allow" was persisted,
    // calling from c2 must prompt again rather than inheriting c1's grant.
    called.store(false, Ordering::SeqCst);
    executor.call("c2", "echo", json!({"z": 1})).await.unwrap();
    assert!(called.load(Ordering::SeqCst));
}

#[tokio::test]
async fn unknown_tool_returns_a_typed_error() {
    let executor = ToolExecutor::new(
        Arc::new(FixedPrompter(PromptResponse::AllowOnce)),
        permission_store(),
    );
    let err = executor
        .call("c1", "does-not-exist", json!({}))
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::UnknownTool(name) if name == "does-not-exist"));
}
