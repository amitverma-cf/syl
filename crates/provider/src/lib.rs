mod catalog;
mod cloud;
mod keys;

pub use catalog::{list_cloud_models, CloudModel};
pub use cloud::{build_client, stream_chat, CloudChatError};
pub use keys::{list_providers, set_api_key, ProviderInfo};

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
