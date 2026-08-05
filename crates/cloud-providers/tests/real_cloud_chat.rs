use std::path::Path;

use cloud_providers::{build_client, stream_chat};

/// Hits a real cloud provider (whichever has a key in `.syl/.env`). Run manually with
/// `cargo test -p provider --test real_cloud_chat -- --ignored`.
#[tokio::test]
#[ignore]
async fn streams_a_real_response_from_a_configured_provider() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let env_file = repo_root.join(".syl/.env");
    let custom_providers_file = repo_root.join(".syl/custom_providers.json");

    let providers = cloud_providers::list_providers(&env_file);
    let configured = providers.iter().find(|p| p.configured).unwrap_or_else(|| {
        panic!(".syl/.env has no configured provider; add one API key to run this test")
    });

    let model_id = cloud_providers::list_cloud_models()
        .into_iter()
        .find(|m| m.provider == configured.name)
        .map(|m| m.id)
        .unwrap_or_else(|| panic!("no catalog model for provider {}", configured.name));

    let client = build_client(&env_file, &custom_providers_file);
    let mut pieces = Vec::new();
    let full_text = stream_chat(
        &client,
        &model_id,
        Some("Answer in one short sentence."),
        "What is the capital of France?",
        |piece| pieces.push(piece.to_string()),
    )
    .await
    .unwrap();

    assert!(!pieces.is_empty(), "expected at least one streamed chunk");
    assert!(!full_text.trim().is_empty());
    assert!(full_text.to_lowercase().contains("paris"));
}
