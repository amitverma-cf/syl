use std::path::Path;

use futures_util::StreamExt;
use genai::adapter::AdapterKind;
use genai::chat::{ChatMessage, ChatRequest, ChatStreamEvent};
use genai::resolver::{AuthData, AuthResolver, Endpoint, ServiceTargetResolver};
use genai::{Client, ModelIden, ServiceTarget};

use crate::custom::list_custom_providers;
use crate::keys::load_env_file;

#[derive(Debug, thiserror::Error)]
pub enum CloudChatError {
    #[error("no API key configured for this provider")]
    MissingApiKey,
    #[error("cloud provider request failed: {0}")]
    Request(#[from] genai::Error),
}

/// The model-ID prefix used for custom OpenAI-compatible providers: `custom::<provider>::<model>`.
const CUSTOM_PREFIX: &str = "custom::";

/// Builds a `genai` client whose API keys come from the app's own `.syl/.env` file rather
/// than the process environment — keys are entered once in Settings and persisted there.
/// Also routes any `custom::<provider>::<model>` model ID to that provider's saved base URL
/// (an OpenAI-compatible endpoint added via `add_custom_provider`) instead of a built-in adapter.
pub fn build_client(env_file: &Path, custom_providers_file: &Path) -> Client {
    let entries = load_env_file(env_file);
    let auth_entries = entries.clone();
    let auth_resolver = AuthResolver::from_resolver_fn(move |model_iden: ModelIden| {
        let key = model_iden
            .adapter_kind
            .default_key_env_name()
            .and_then(|var| auth_entries.get(var))
            .cloned();
        Ok(key.map(AuthData::from_single))
    });

    let custom_providers = list_custom_providers(custom_providers_file);
    let target_resolver = ServiceTargetResolver::from_resolver_fn(
        move |target: ServiceTarget| -> Result<ServiceTarget, genai::resolver::Error> {
            let model_name = target.model.model_name.to_string();
            let Some(rest) = model_name.strip_prefix(CUSTOM_PREFIX) else {
                return Ok(target);
            };
            let Some((provider_name, real_model)) = rest.split_once("::") else {
                return Ok(target);
            };
            let Some(provider) = custom_providers.iter().find(|p| p.name == provider_name) else {
                return Ok(target);
            };

            let endpoint = Endpoint::from_owned(format!("{}/", provider.base_url));
            let auth = entries
                .get(&provider.env_var)
                .cloned()
                .map(AuthData::from_single)
                .unwrap_or(AuthData::from_single(String::new()));
            let model = ModelIden::new(AdapterKind::OpenAI, real_model.to_string());
            Ok(ServiceTarget {
                endpoint,
                auth,
                model,
            })
        },
    );

    Client::builder()
        .with_auth_resolver(auth_resolver)
        .with_service_target_resolver(target_resolver)
        .build()
}

/// Streams a single-turn chat completion from a cloud model, invoking `on_piece` for each
/// text chunk as it arrives, and returns the full accumulated response text.
pub async fn stream_chat(
    client: &Client,
    model_id: &str,
    system_prompt: Option<&str>,
    user_prompt: &str,
    mut on_piece: impl FnMut(&str),
) -> Result<String, CloudChatError> {
    let mut messages = Vec::new();
    if let Some(system_prompt) = system_prompt {
        messages.push(ChatMessage::system(system_prompt));
    }
    messages.push(ChatMessage::user(user_prompt));

    let mut stream = client
        .exec_chat_stream(model_id, ChatRequest::new(messages), None)
        .await?
        .stream;

    let mut full_text = String::new();
    while let Some(event) = stream.next().await {
        if let ChatStreamEvent::Chunk(chunk) = event? {
            on_piece(&chunk.content);
            full_text.push_str(&chunk.content);
        }
    }
    Ok(full_text)
}
