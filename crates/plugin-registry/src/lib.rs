//! Fetches and parses the engine plugin and model catalog listings.

use serde::{Deserialize, Serialize};

/// One entry in `registry/engines.json`: a downloadable build of an inference engine plugin.
#[derive(Debug, Serialize, Deserialize)]
pub struct EngineEntry {
    /// The engine's unique id.
    pub id: String,
    /// The version of this build.
    pub version: String,
    /// The target platform this build runs on (e.g. `windows-x64-cuda`).
    pub platform: String,
    /// URL to download this build from.
    pub download_url: String,
    /// SHA-256 hash of the downloaded file, for integrity verification.
    pub sha256: String,
}

/// One entry in `registry/models.json`: a model available to download and run.
#[derive(Debug, Serialize, Deserialize)]
pub struct ModelEntry {
    /// The model's display name.
    pub name: String,
    /// The download size, in bytes.
    pub size_bytes: u64,
    /// The quantization variant of this listing (e.g. `Q4_K_M`).
    pub quantization: String,
    /// The id of the engine required to run this model.
    pub required_engine: String,
    /// URL to download this model's weights from Hugging Face.
    pub huggingface_url: String,
    /// SHA-256 hash of the downloaded file, for integrity verification.
    pub sha256: String,
}

/// Parses a list of engine plugin entries from `registry/engines.json` contents.
///
/// # Errors
/// Returns an error if `json` is not valid JSON or does not match the expected shape.
pub fn parse_engines(json: &str) -> serde_json::Result<Vec<EngineEntry>> {
    serde_json::from_str(json)
}

/// Parses a list of model catalog entries from `registry/models.json` contents.
///
/// # Errors
/// Returns an error if `json` is not valid JSON or does not match the expected shape.
pub fn parse_models(json: &str) -> serde_json::Result<Vec<ModelEntry>> {
    serde_json::from_str(json)
}
