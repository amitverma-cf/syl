use provider::CustomProviderConfig;

fn temp_json_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "syl-provider-custom-test-{name}-{}.json",
        std::process::id()
    ))
}

fn temp_env_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "syl-provider-custom-test-{name}-{}.env",
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
fn remove_custom_provider_deletes_the_entry_and_its_stored_key() {
    let providers_path = temp_json_path("remove");
    let env_path = temp_env_path("remove");
    let configs = vec![
        CustomProviderConfig {
            name: "acme".to_string(),
            base_url: "https://acme.example/v1".to_string(),
            env_var: "CUSTOM_ACME_API_KEY".to_string(),
            models: vec!["acme-large".to_string()],
        },
        CustomProviderConfig {
            name: "other".to_string(),
            base_url: "https://other.example/v1".to_string(),
            env_var: "CUSTOM_OTHER_API_KEY".to_string(),
            models: vec!["other-model".to_string()],
        },
    ];
    std::fs::write(&providers_path, serde_json::to_string(&configs).unwrap()).unwrap();
    provider::set_api_key(&env_path, "CUSTOM_ACME_API_KEY", "sk-acme").unwrap();
    provider::set_api_key(&env_path, "CUSTOM_OTHER_API_KEY", "sk-other").unwrap();

    provider::remove_custom_provider(&providers_path, &env_path, "acme").unwrap();

    let remaining = provider::list_custom_providers(&providers_path);
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].name, "other");

    let key_providers = provider::list_providers(&env_path);
    let _ = key_providers; // list_providers only reports known cloud providers, not custom ones
    let entries = provider::load_env_file(&env_path);
    assert!(!entries.contains_key("CUSTOM_ACME_API_KEY"));
    assert_eq!(entries.get("CUSTOM_OTHER_API_KEY").unwrap(), "sk-other");

    std::fs::remove_file(&providers_path).ok();
    std::fs::remove_file(&env_path).ok();
}

#[test]
fn remove_custom_provider_is_a_no_op_when_the_name_does_not_exist() {
    let providers_path = temp_json_path("remove-missing");
    let env_path = temp_env_path("remove-missing");
    let configs = vec![CustomProviderConfig {
        name: "acme".to_string(),
        base_url: "https://acme.example/v1".to_string(),
        env_var: "CUSTOM_ACME_API_KEY".to_string(),
        models: vec!["acme-large".to_string()],
    }];
    std::fs::write(&providers_path, serde_json::to_string(&configs).unwrap()).unwrap();

    provider::remove_custom_provider(&providers_path, &env_path, "does-not-exist").unwrap();

    assert_eq!(provider::list_custom_providers(&providers_path).len(), 1);

    std::fs::remove_file(&providers_path).ok();
}

/// Serves a fixed `/models` response for exactly one request, so tests can point
/// `update_custom_provider`'s real discovery call at something other than the
/// network.
fn serve_models_once(body: &'static str) -> (String, std::thread::JoinHandle<()>) {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = std::thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut request_line = String::new();
        reader.read_line(&mut request_line).unwrap();
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap_or(0) == 0 || line.trim().is_empty() {
                break;
            }
        }
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(header.as_bytes()).unwrap();
        stream.write_all(body.as_bytes()).unwrap();
        stream.flush().unwrap();
    });
    (format!("http://127.0.0.1:{port}"), handle)
}

#[test]
fn update_custom_provider_replaces_the_existing_entry_via_a_real_discovery_call() {
    let providers_path = temp_json_path("update");
    let env_path = temp_env_path("update");
    let original = vec![CustomProviderConfig {
        name: "acme".to_string(),
        base_url: "http://stale.invalid/v1".to_string(),
        env_var: "CUSTOM_ACME_API_KEY".to_string(),
        models: vec!["stale-model".to_string()],
    }];
    std::fs::write(&providers_path, serde_json::to_string(&original).unwrap()).unwrap();

    let (base_url, handle) = serve_models_once(r#"{"data":[{"id":"fresh-model"}]}"#);
    let updated =
        provider::update_custom_provider(&providers_path, &env_path, "acme", &base_url, None)
            .unwrap();
    handle.join().unwrap();

    assert_eq!(updated.base_url, base_url);
    assert_eq!(updated.models, vec!["fresh-model".to_string()]);

    let stored = provider::list_custom_providers(&providers_path);
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].base_url, base_url);

    std::fs::remove_file(&providers_path).ok();
    std::fs::remove_file(&env_path).ok();
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
