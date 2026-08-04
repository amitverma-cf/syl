mod catalog;
mod cloud;
mod custom;
mod keys;

pub use catalog::{list_cloud_models, CloudModel};
pub use cloud::{build_client, chat_with_tools, stream_chat, CloudChatError};
pub use custom::{
    add_custom_provider, list_custom_providers, remove_custom_provider, update_custom_provider,
    CustomProviderConfig, CustomProviderError,
};
pub use keys::{delete_api_key, list_providers, load_env_file, set_api_key, ProviderInfo};

use std::path::Path;

pub fn list_all_models(custom_providers_file: &Path) -> Vec<CloudModel> {
    let mut models = catalog::list_cloud_models();
    for provider in custom::list_custom_providers(custom_providers_file) {
        for model_id in &provider.models {
            models.push(CloudModel {
                id: format!("custom::{}::{}", provider.name, model_id),
                provider: provider.name.clone(),
                label: model_id.clone(),
            });
        }
    }
    models
}

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
