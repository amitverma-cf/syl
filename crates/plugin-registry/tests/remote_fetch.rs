use plugin_registry::fetch_remote_registry;

#[test]
#[ignore]
fn fetches_the_live_registry_from_github() {
    let (engines_json, models_json) =
        fetch_remote_registry("https://raw.githubusercontent.com/amitverma-cf/syl/main/registry")
            .unwrap();

    serde_json::from_str::<serde_json::Value>(&engines_json).unwrap();
    serde_json::from_str::<serde_json::Value>(&models_json).unwrap();
}
