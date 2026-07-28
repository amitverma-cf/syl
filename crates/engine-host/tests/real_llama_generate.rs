use std::path::Path;

use engine_host::llama::LlamaEngine;

#[test]
#[ignore]
fn generates_text_with_real_engine() {
    let registry_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.syl/registry");

    let engines = plugin_registry::load_engine_entries(&registry_dir).unwrap();
    let engine_entry = engines.iter().find(|e| e.id == "llama-cpp").unwrap();
    let library_path = plugin_registry::resolve_local_path(&engine_entry.download_url).unwrap();

    let models = plugin_registry::load_model_entries(&registry_dir).unwrap();
    let model_entry = models
        .iter()
        .find(|m| m.name == "Qwen3.5-0.8B-Q4_K_M")
        .unwrap();
    let model_path = plugin_registry::resolve_local_path(&model_entry.download_url).unwrap();

    let mut engine = LlamaEngine::load(&library_path, &model_path, 2048, false).unwrap();

    let output = engine
        .generate("The capital of France is", 32, |_piece| {})
        .unwrap();

    assert!(!output.trim().is_empty());
}
