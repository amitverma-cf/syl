//! Fetches and verifies the GitHub-hosted registry (Decision #4): `registry/engines.json`
//! (engine plugin listings) and `registry/models.json` (Hugging Face model catalog).
//! Downloads engine binaries and exposes the model catalog for the UI's browse screen.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct EngineEntry {
    pub id: String,
    pub version: String,
    pub platform: String,
    pub download_url: String,
    pub sha256: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelEntry {
    pub name: String,
    pub size_bytes: u64,
    pub quantization: String,
    pub required_engine: String,
    pub huggingface_url: String,
    pub sha256: String,
}

pub fn parse_engines(json: &str) -> serde_json::Result<Vec<EngineEntry>> {
    serde_json::from_str(json)
}

pub fn parse_models(json: &str) -> serde_json::Result<Vec<ModelEntry>> {
    serde_json::from_str(json)
}
