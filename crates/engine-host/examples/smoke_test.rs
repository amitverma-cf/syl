//! Manual smoke test: loads the local llama.cpp engine and model resolved via the plugin
//! registry's local overrides, and runs one generation call, printing tokens as they arrive.
//!
//! Run with: `cargo run -p engine-host --example smoke_test`

use std::io::Write;
use std::path::Path;

use engine_host::llama::LlamaEngine;

fn main() {
    let registry_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../registry");

    let engines =
        plugin_registry::load_engine_entries(&registry_dir).expect("failed to load engine entries");
    let llama_engine_entry = engines
        .iter()
        .find(|e| e.id == "llama-cpp")
        .expect("no llama-cpp entry in registry/local.engines.json");
    let library_path = plugin_registry::resolve_local_path(&llama_engine_entry.download_url)
        .expect("failed to resolve llama-cpp engine path");

    let models =
        plugin_registry::load_model_entries(&registry_dir).expect("failed to load model entries");
    let model_entry = models
        .first()
        .expect("no entries in registry/local.models.json");
    let model_path = plugin_registry::resolve_local_path(&model_entry.huggingface_url)
        .expect("failed to resolve model path");

    println!("engine: {}", library_path.display());
    println!("model:  {}", model_path.display());

    let mut engine =
        LlamaEngine::load(&library_path, &model_path, 2048).expect("failed to load engine");

    println!("--- generating ---");
    let full = engine
        .generate("The capital of France is", 64, |piece| {
            print!("{piece}");
            std::io::stdout().flush().ok();
        })
        .expect("generation failed");

    println!("\n--- full output ---\n{full}");
}
