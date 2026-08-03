use std::path::Path;

use engine_host::llama::LlamaEngine;
use executor::{parse_flow, FlowRunner};
use tool::{ReadFileTool, Tool};

/// End-to-end check that the demo flow actually drives a real model through both of its
/// states, including running its `on_enter_tool_call` and folding the tool's real output into
/// the next prompt. Requires a local `.syl` workspace with the `llama-cpp` engine and a
/// `Qwen3.5-2B-Q4_K_M.gguf` chat model already present — run manually with
/// `cargo test -p executor --test real_flow_run -- --ignored`.
#[tokio::test]
#[ignore]
async fn demo_flow_runs_two_states_with_a_real_model() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let syl_dir = repo_root.join(".syl");

    let engine_entries = plugin_registry::load_engine_entries(&syl_dir.join("registry"))
        .expect(".syl/registry/engines.json is missing; run the app once to seed the workspace");
    let engine_entry = engine_entries
        .into_iter()
        .find(|entry| entry.id == "llama-cpp")
        .expect("no llama-cpp engine registered in .syl/registry/engines.json");
    let engine_library_path = plugin_registry::resolve_local_path(
        &engine_entry.download_url,
        &syl_dir.join("engines"),
        engine_entry.sha256.as_deref(),
    )
    .expect("failed to resolve the llama-cpp engine library path");

    let model_path = syl_dir.join("models").join("Qwen3.5-2B-Q4_K_M.gguf");
    assert!(
        model_path.exists(),
        "expected {} to exist for this test",
        model_path.display()
    );

    let workspace_root = std::env::temp_dir().join(format!(
        "syl-executor-real-flow-test-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&workspace_root).unwrap();
    std::fs::write(
        workspace_root.join("notes.txt"),
        "The project codename is Falcon. The launch date is October 3rd.",
    )
    .unwrap();

    let flow_json = std::fs::read(repo_root.join("flows/demo.json")).unwrap();
    let flow = parse_flow(&flow_json).unwrap();
    let mut runner = FlowRunner::new(flow).unwrap();
    assert_eq!(runner.current_state().name, "greeting");
    assert!(runner.on_enter_tool_call().is_none());

    let mut engine = LlamaEngine::load(&engine_library_path, &model_path, 2048, false).unwrap();

    let first_prompt = format!(
        "{}\n\nUser: Hi, can you check my notes file?",
        runner.current_state().system_prompt
    );
    let first_response = engine.generate(&first_prompt, 64, |_piece| {}).unwrap();
    assert!(
        !first_response.trim().is_empty(),
        "expected non-empty generation for the greeting state"
    );

    assert!(runner.advance("message"));
    assert_eq!(runner.current_state().name, "file_summary");

    let tool_call = runner
        .on_enter_tool_call()
        .expect("file_summary state should declare an on_enter_tool_call")
        .clone();
    assert_eq!(tool_call.tool, "read_file");

    let read_file = ReadFileTool {
        workspace_root: workspace_root.clone(),
    };
    let tool_result = read_file.call(tool_call.args).await.unwrap();
    let file_contents = tool_result["content"].as_str().unwrap();
    assert!(file_contents.contains("Falcon"));

    let second_prompt = format!(
        "{}\n\nTool output: {}\n\nUser: What did you find?",
        runner.current_state().system_prompt,
        file_contents
    );
    let second_response = engine.generate(&second_prompt, 96, |_piece| {}).unwrap();
    assert!(
        !second_response.trim().is_empty(),
        "expected non-empty generation for the file_summary state"
    );

    assert!(runner.advance("message"));
    assert_eq!(runner.current_state().name, "greeting");

    std::fs::remove_dir_all(&workspace_root).ok();
}
