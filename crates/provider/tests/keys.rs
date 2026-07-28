use provider::{list_providers, set_api_key};

fn temp_env_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "syl-provider-keys-test-{name}-{}-{}.env",
        std::process::id(),
        name.len()
    ))
}

#[test]
fn list_providers_reports_all_known_providers_unconfigured_when_file_missing() {
    let path = temp_env_path("missing");
    let providers = list_providers(&path);
    assert_eq!(providers.len(), 7);
    assert!(providers.iter().all(|p| !p.configured));
    assert!(providers
        .iter()
        .any(|p| p.name == "OpenAI" && p.env_var == "OPENAI_API_KEY"));
    assert!(providers
        .iter()
        .any(|p| p.name == "Anthropic" && p.env_var == "ANTHROPIC_API_KEY"));
}

#[test]
fn set_api_key_persists_and_is_reflected_in_list_providers() {
    let path = temp_env_path("set-one");
    set_api_key(&path, "ANTHROPIC_API_KEY", "sk-test-123").unwrap();

    let providers = list_providers(&path);
    let anthropic = providers.iter().find(|p| p.name == "Anthropic").unwrap();
    assert!(anthropic.configured);

    let openai = providers.iter().find(|p| p.name == "OpenAI").unwrap();
    assert!(!openai.configured);

    std::fs::remove_file(&path).ok();
}

#[test]
fn set_api_key_twice_updates_the_same_key_without_duplicating_it() {
    let path = temp_env_path("update");
    set_api_key(&path, "OPENAI_API_KEY", "sk-old").unwrap();
    set_api_key(&path, "OPENAI_API_KEY", "sk-new").unwrap();

    let contents = std::fs::read_to_string(&path).unwrap();
    assert_eq!(contents.matches("OPENAI_API_KEY").count(), 1);
    assert!(contents.contains("sk-new"));
    assert!(!contents.contains("sk-old"));

    std::fs::remove_file(&path).ok();
}

#[test]
fn set_api_key_preserves_other_providers_keys() {
    let path = temp_env_path("preserve");
    set_api_key(&path, "OPENAI_API_KEY", "sk-openai").unwrap();
    set_api_key(&path, "GEMINI_API_KEY", "sk-gemini").unwrap();

    let providers = list_providers(&path);
    assert!(
        providers
            .iter()
            .find(|p| p.name == "OpenAI")
            .unwrap()
            .configured
    );
    assert!(
        providers
            .iter()
            .find(|p| p.name == "Gemini")
            .unwrap()
            .configured
    );
    assert!(
        !providers
            .iter()
            .find(|p| p.name == "Groq")
            .unwrap()
            .configured
    );

    std::fs::remove_file(&path).ok();
}
