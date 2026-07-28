use core_types::{CoreResult, EngineId, ModelId};

pub trait Provider: Send + Sync {
    fn engine_id(&self) -> &EngineId;
    fn model_id(&self) -> &ModelId;
}

#[derive(Default)]
pub struct ModelRegistry {}

impl ModelRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn list(&self) -> CoreResult<Vec<ModelId>> {
        Ok(Vec::new())
    }
}
