//! Manual smoke test: loads the local llama.cpp engine and a small embedding model, then
//! computes and prints embedding vectors for two sentences to confirm the pipeline works.
//!
//! Run with: `cargo run -p engine-host --example embed_test`

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
        .iter()
        .find(|m| m.name.to_lowercase().contains("minilm"))
        .expect("no embedding model entry in registry/local.models.json");
    let model_path = plugin_registry::resolve_local_path(&model_entry.huggingface_url)
        .expect("failed to resolve embedding model path");

    println!("engine: {}", library_path.display());
    println!("model:  {}", model_path.display());

    let mut engine = LlamaEngine::load(&library_path, &model_path, 512, true)
        .expect("failed to load embedding engine");

    let a = engine
        .embed("The cat sat on the mat.")
        .expect("embedding failed");
    let b = engine
        .embed("A feline rested on the rug.")
        .expect("embedding failed");
    let c = engine
        .embed("Quarterly revenue exceeded projections.")
        .expect("embedding failed");

    println!("dims: {}", a.len());
    println!("a[0..4] = {:?}", &a[..4.min(a.len())]);

    let sim_ab = cosine_similarity(&a, &b);
    let sim_ac = cosine_similarity(&a, &c);
    println!("cosine(similar sentences)   = {sim_ab:.4}");
    println!("cosine(unrelated sentences) = {sim_ac:.4}");
    assert!(
        sim_ab > sim_ac,
        "expected semantically similar sentences to score higher than unrelated ones"
    );
    println!("OK: similar sentences scored higher than unrelated ones");
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (norm_a * norm_b)
}
