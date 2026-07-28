use std::path::Path;

use engine_host::llama::LlamaEngine;
use memory::{EmbeddingStore, SqliteConversationStore};

#[test]
#[ignore]
fn real_embeddings_retrieve_the_semantically_closest_message() {
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

    let corpus = [
        "The cat sat on the mat.",
        "Quarterly revenue exceeded projections.",
        "The weather in Paris was sunny yesterday.",
    ];

    let store = SqliteConversationStore::open_in_memory().unwrap();
    for text in corpus {
        let embedding = engine.embed(text).unwrap();
        store.store_embedding("c1", text, &embedding).unwrap();
    }

    let query = engine.embed("A feline rested on a rug.").unwrap();
    let results = store.search_similar("c1", &query, 1).unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].content, "The cat sat on the mat.");
}
