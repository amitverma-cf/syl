//! Shared types and error kinds used across all five pillars.

pub mod workspace_paths;

use serde::{Deserialize, Serialize};

/// An error returned by any of the pillar crates.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    /// The called functionality has not been implemented yet.
    #[error("not implemented")]
    NotImplemented,
}

/// The result type returned by fallible operations across the pillar crates.
pub type CoreResult<T> = Result<T, CoreError>;

/// Identifies a loaded or loadable model instance, scoped to an engine.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelId(pub String);

/// Identifies which native engine backend a model runs on (llama.cpp, onnxruntime, sd, ...).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EngineId(pub String);

/// Identifies a single in-flight request to an engine, used by the batching scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RequestId(pub u64);
