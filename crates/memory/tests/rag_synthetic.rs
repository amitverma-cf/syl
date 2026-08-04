use memory::{EmbeddingStore, MemoryError, SqliteConversationStore};

#[test]
fn search_similar_ranks_the_closest_vector_first() {
    let store = SqliteConversationStore::open_in_memory().unwrap();

    store
        .store_embedding("c1", "the cat sat on the mat", &[1.0, 0.0, 0.0])
        .unwrap();
    store
        .store_embedding("c1", "a feline rested on the rug", &[0.9, 0.1, 0.0])
        .unwrap();
    store
        .store_embedding(
            "c1",
            "quarterly revenue exceeded projections",
            &[0.0, 0.0, 1.0],
        )
        .unwrap();

    let results = store.search_similar("c1", &[1.0, 0.0, 0.0], 2).unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].content, "the cat sat on the mat");
    assert_eq!(results[1].content, "a feline rested on the rug");
    assert!(results[0].score > results[1].score);
    assert!(results[1].score > 0.5);
}

#[test]
fn search_similar_respects_top_k() {
    let store = SqliteConversationStore::open_in_memory().unwrap();
    for i in 0..5 {
        store
            .store_embedding("c1", &format!("message {i}"), &[i as f32, 1.0, 0.0])
            .unwrap();
    }

    let results = store.search_similar("c1", &[0.0, 1.0, 0.0], 3).unwrap();
    assert_eq!(results.len(), 3);
}

#[test]
fn search_similar_does_not_leak_across_conversations() {
    let store = SqliteConversationStore::open_in_memory().unwrap();
    store
        .store_embedding("c1", "only in c1", &[1.0, 0.0])
        .unwrap();
    store
        .store_embedding("c2", "only in c2", &[1.0, 0.0])
        .unwrap();

    let results = store.search_similar("c1", &[1.0, 0.0], 10).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].content, "only in c1");
}

#[test]
fn search_similar_on_empty_conversation_returns_empty() {
    let store = SqliteConversationStore::open_in_memory().unwrap();
    let results = store
        .search_similar("does-not-exist", &[1.0, 0.0], 5)
        .unwrap();
    assert!(results.is_empty());
}

#[test]
fn storing_an_embedding_of_a_different_dimension_is_rejected() {
    let store = SqliteConversationStore::open_in_memory().unwrap();
    store
        .store_embedding("c1", "first embedding, 3 dims", &[1.0, 0.0, 0.0])
        .unwrap();

    let err = store
        .store_embedding("c1", "second embedding, 2 dims", &[1.0, 0.0])
        .unwrap_err();
    assert!(matches!(err, MemoryError::EmbeddingDimensionMismatch));
}
