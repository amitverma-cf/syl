use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudProviderEntry {
    pub name: String,
    pub env_var: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalEngineConfig {
    pub id: String,
    pub context_size: u32,
    pub max_tokens: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdEngineConfig {
    pub id: String,
    pub steps: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnnxEngineConfig {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub known_cloud_providers: Vec<CloudProviderEntry>,
    pub default_flow_name: String,
    pub registry_poll_url: String,
    pub registry_allowed_hosts: Vec<String>,
    /// Hex-encoded Ed25519 public key the registry manifest's signatures must
    /// verify against. `None` (the default until a real keypair is
    /// provisioned and wired into a publish pipeline — see
    /// `plugin_registry`'s `examples/sign_registry.rs`) means signature
    /// verification is skipped, matching today's actual registryPollUrl,
    /// which publishes no signatures.
    #[serde(default)]
    pub registry_manifest_public_key: Option<String>,
    pub max_tool_iterations: u32,
    pub context_budget_chars: usize,
    pub local_engine: LocalEngineConfig,
    pub sd_engine: SdEngineConfig,
    pub onnx_engine: OnnxEngineConfig,
}

static APP_CONFIG: OnceLock<AppConfig> = OnceLock::new();

pub fn app_config() -> &'static AppConfig {
    APP_CONFIG.get_or_init(|| {
        let path =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/app.json");
        let contents = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        serde_json::from_str(&contents)
            .unwrap_or_else(|err| panic!("failed to parse {}: {err}", path.display()))
    })
}
