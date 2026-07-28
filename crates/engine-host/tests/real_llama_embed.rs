use std::path::Path;

use engine_host::llama::LlamaEngine;

#[test]
#[ignore]
fn embeddings_are_semantically_meaningful() {
    let registry_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.syl/registry");

    let engines = plugin_registry::load_engine_entries(&registry_dir).unwrap();
    let engine_entry = engines.iter().find(|e| e.id == "llama-cpp").unwrap();
    let library_path = plugin_registry::resolve_local_path(&engine_entry.download_url).unwrap();

    let models = plugin_registry::load_model_entries(&registry_dir).unwrap();
    let model_entry = models
        .iter()
        .find(|m| m.name.to_lowercase().contains("minilm"))
        .unwrap();
    let model_path = plugin_registry::resolve_local_path(&model_entry.download_url).unwrap();

    let mut engine = LlamaEngine::load(&library_path, &model_path, 512, true).unwrap();

    let a = engine.embed("The cat sat on the mat.").unwrap();
    let b = engine.embed("A feline rested on the rug.").unwrap();
    let c = engine
        .embed("Quarterly revenue exceeded projections.")
        .unwrap();

    let sim_related = cosine_similarity(&a, &b);
    let sim_unrelated = cosine_similarity(&a, &c);

    assert!(
        sim_related > sim_unrelated,
        "expected related sentences ({sim_related}) to score higher than unrelated ones ({sim_unrelated})"
    );
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (norm_a * norm_b)
}
