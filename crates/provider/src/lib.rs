//! Provider pillar: abstracts "a thing that can run inference" — local engines
//! (via engine-host FFI) and cloud APIs (HTTP) — behind a single trait, backed
//! by a multi-model registry rather than a single global instance.

use core_types::{CoreResult, EngineId, ModelId};

/// A model available for inference, whether local (engine-backed) or remote (cloud API).
pub trait Provider: Send + Sync {
    fn engine_id(&self) -> &EngineId;
    fn model_id(&self) -> &ModelId;
}

/// Tracks all loaded/available models across all engines. Explicitly multi-model:
/// agent.cpp's single-hardcoded-instance design is the mistake this replaces.
#[derive(Default)]
pub struct ModelRegistry {
    // populated once Provider implementations exist
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn list(&self) -> CoreResult<Vec<ModelId>> {
        Ok(Vec::new())
    }
}
