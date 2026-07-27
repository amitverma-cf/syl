//! Shared types and error kinds used across all five pillars.

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("not implemented")]
    NotImplemented,
}

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
