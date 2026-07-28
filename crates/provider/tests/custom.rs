use provider::CustomProviderConfig;

fn temp_json_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "syl-provider-custom-test-{name}-{}.json",
        std::process::id()
    ))
}

#[test]
fn list_custom_providers_is_empty_when_file_missing() {
    let path = temp_json_path("missing");
    assert!(provider::list_custom_providers(&path).is_empty());
}

#[test]
fn list_custom_providers_reads_a_hand_written_config_file() {
    let path = temp_json_path("roundtrip");
    let configs = vec![CustomProviderConfig {
        name: "my-local-server".to_string(),
        base_url: "http://localhost:8080/v1".to_string(),
        env_var: "CUSTOM_MY_LOCAL_SERVER_API_KEY".to_string(),
        models: vec!["some-model".to_string()],
    }];
    std::fs::write(&path, serde_json::to_string(&configs).unwrap()).unwrap();

    let loaded = provider::list_custom_providers(&path);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].name, "my-local-server");
    assert_eq!(loaded[0].base_url, "http://localhost:8080/v1");
    assert_eq!(loaded[0].models, vec!["some-model".to_string()]);

    std::fs::remove_file(&path).ok();
}

#[test]
fn list_all_models_merges_catalog_and_custom_providers() {
    let path = temp_json_path("merge");
    let configs = vec![CustomProviderConfig {
        name: "acme".to_string(),
        base_url: "https://acme.example/v1".to_string(),
        env_var: "CUSTOM_ACME_API_KEY".to_string(),
        models: vec!["acme-large".to_string(), "acme-small".to_string()],
    }];
    std::fs::write(&path, serde_json::to_string(&configs).unwrap()).unwrap();

    let models = provider::list_all_models(&path);
    assert!(models.iter().any(|m| m.provider == "OpenAI"));
    assert!(models
        .iter()
        .any(|m| m.id == "custom::acme::acme-large" && m.provider == "acme"));
    assert!(models
        .iter()
        .any(|m| m.id == "custom::acme::acme-small" && m.provider == "acme"));

    std::fs::remove_file(&path).ok();
}
