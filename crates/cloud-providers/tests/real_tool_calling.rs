use std::path::Path;
use std::sync::Arc;

use cloud_providers::{build_client, chat_with_tools};
use memory::SqliteConversationStore;
use tools::{AlwaysApprove, ReadFileTool, ToolExecutor};

/// Hits a real cloud provider and checks it actually chooses to call `read_file` on its own,
/// rather than the flow forcing the call — the whole point of Branch 2's model-driven loop.
/// Run manually with `cargo test -p provider --test real_tool_calling -- --ignored`.
#[tokio::test]
#[ignore]
async fn model_calls_read_file_tool_and_uses_its_real_content() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let env_file = repo_root.join(".syl/.env");
    let custom_providers_file = repo_root.join(".syl/custom_providers.json");

    let providers = cloud_providers::list_providers(&env_file);
    let configured = providers.iter().find(|p| p.configured).unwrap_or_else(|| {
        panic!(".syl/.env has no configured provider; add one API key to run this test")
    });
    let model_id = cloud_providers::list_cloud_models()
        .into_iter()
        .find(|m| m.provider == configured.name)
        .map(|m| m.id)
        .unwrap_or_else(|| panic!("no catalog model for provider {}", configured.name));

    let workspace_root = std::env::temp_dir().join(format!(
        "syl-provider-real-tool-calling-test-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&workspace_root).unwrap();
    std::fs::write(
        workspace_root.join("notes.txt"),
        "The project codename is Falcon. The launch date is October 3rd.",
    )
    .unwrap();

    let permission_store = Arc::new(SqliteConversationStore::open_in_memory().unwrap());
    let executor = ToolExecutor::new(Arc::new(AlwaysApprove), permission_store);
    executor.register(Arc::new(ReadFileTool {
        workspace_root: workspace_root.clone(),
    }));
    let tools = executor.tool_specs();

    let client = build_client(&env_file, &custom_providers_file);
    let mut pieces = Vec::new();
    let final_answer = chat_with_tools(
        &client,
        &model_id,
        Some("You are a helpful assistant with access to tools. Use a tool when it helps."),
        "Read notes.txt and tell me the project codename.",
        &tools,
        &executor,
        "real-tool-calling-test",
        |piece| pieces.push(piece.to_string()),
    )
    .await
    .unwrap();

    assert!(
        final_answer.to_lowercase().contains("falcon"),
        "expected the final answer to mention the real file content, got: {final_answer}"
    );

    std::fs::remove_dir_all(&workspace_root).ok();
}
