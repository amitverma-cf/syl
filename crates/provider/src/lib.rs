//! Traits and types for running inference against local engines or cloud APIs.

use core_types::{CoreResult, EngineId, ModelId};

/// A single model available for inference, whether backed by a local engine or a remote API.
pub trait Provider: Send + Sync {
    /// Returns the id of the engine this model runs on.
    fn engine_id(&self) -> &EngineId;
    /// Returns the id of this model.
    fn model_id(&self) -> &ModelId;
}

/// Tracks every model currently loaded or available across all engines.
#[derive(Default)]
pub struct ModelRegistry {
    // populated once Provider implementations exist
}

impl ModelRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the ids of all models currently registered.
    pub fn list(&self) -> CoreResult<Vec<ModelId>> {
        Ok(Vec::new())
    }
}
