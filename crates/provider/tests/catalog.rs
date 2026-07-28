use provider::list_cloud_models;

#[test]
fn catalog_covers_every_known_provider() {
    let models = list_cloud_models();
    let providers: std::collections::HashSet<_> =
        models.iter().map(|m| m.provider.as_str()).collect();
    for expected in [
        "OpenAI",
        "Anthropic",
        "Gemini",
        "Groq",
        "xAI",
        "DeepSeek",
        "Cohere",
    ] {
        assert!(
            providers.contains(expected),
            "expected catalog to include a model for {expected}"
        );
    }
}

#[test]
fn catalog_entries_have_non_empty_ids_and_labels() {
    for model in list_cloud_models() {
        assert!(!model.id.is_empty());
        assert!(!model.label.is_empty());
    }
}
