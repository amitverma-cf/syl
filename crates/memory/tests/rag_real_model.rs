use std::path::Path;

use extension_registry::ModelKind;
use memory::{EmbeddingStore, SqliteConversationStore};
use native_engines::llama::LlamaEngine;

#[test]
#[ignore]
fn real_embeddings_retrieve_the_semantically_closest_message() {
    let registry_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.syl/registry");
    let cache_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.syl");

    let resolved = extension_registry::resolve_model_for_kind(
        &registry_dir,
        &cache_dir.join("models"),
        &cache_dir.join("engines"),
        ModelKind::Embedding,
    )
    .expect(".syl workspace has no embedding model registered; run the app once to seed it");

    let mut engine = LlamaEngine::load(
        &resolved.engine_library_path,
        &resolved.model_path,
        512,
        true,
    )
    .unwrap();

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
