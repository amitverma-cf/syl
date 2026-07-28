use std::path::Path;
use std::sync::Arc;

use engine_host::llama::LlamaEngine;
use engine_host::tool_loop::generate_with_tools;
use memory::SqliteConversationStore;
use tool::{AlwaysApprove, ReadFileTool, ToolExecutor};

/// Hits a real local model and checks it actually chooses to call `read_file` via the
/// prompt-engineered JSON convention, rather than the flow forcing the call. Requires a local
/// `.syl` workspace with the `llama-cpp` engine and a `Qwen3.5-2B-Q4_K_M.gguf` chat model
/// already present — run manually with
/// `cargo test -p engine-host --test real_tool_calling -- --ignored`.
#[test]
#[ignore]
fn model_calls_read_file_tool_and_uses_its_real_content() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let syl_dir = repo_root.join(".syl");

    let engine_entries = plugin_registry::load_engine_entries(&syl_dir.join("registry"))
        .expect(".syl/registry/engines.json is missing; run the app once to seed the workspace");
    let engine_entry = engine_entries
        .into_iter()
        .find(|entry| entry.id == "llama-cpp")
        .expect("no llama-cpp engine registered in .syl/registry/engines.json");
    let engine_library_path =
        plugin_registry::resolve_local_path(&engine_entry.download_url, &syl_dir.join("engines"))
            .expect("failed to resolve the llama-cpp engine library path");

    let model_path = syl_dir.join("models").join("Qwen3.5-2B-Q4_K_M.gguf");
    assert!(
        model_path.exists(),
        "expected {} to exist for this test",
        model_path.display()
    );

    let workspace_root = std::env::temp_dir().join(format!(
        "syl-engine-host-real-tool-calling-test-{}",
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

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _guard = runtime.enter();

    let mut engine = LlamaEngine::load(&engine_library_path, &model_path, 2048, false).unwrap();
    let final_answer = generate_with_tools(
        &mut engine,
        "You are a helpful assistant with access to tools. Use a tool when it helps.",
        "Read notes.txt and tell me the project codename.",
        &tools,
        &executor,
        "real-tool-calling-test",
        128,
        |_piece| {},
    )
    .unwrap();

    assert!(
        final_answer.to_lowercase().contains("falcon"),
        "expected the final answer to mention the real file content, got: {final_answer}"
    );

    std::fs::remove_dir_all(&workspace_root).ok();
}
