use std::path::Path;

use futures_util::StreamExt;
use genai::chat::{ChatMessage, ChatRequest, ChatStreamEvent};
use genai::resolver::{AuthData, AuthResolver};
use genai::Client;

use crate::keys::load_env_file;

#[derive(Debug, thiserror::Error)]
pub enum CloudChatError {
    #[error("no API key configured for this provider")]
    MissingApiKey,
    #[error("cloud provider request failed: {0}")]
    Request(#[from] genai::Error),
}

/// Builds a `genai` client whose API keys come from the app's own `.syl/.env` file rather
/// than the process environment — keys are entered once in Settings and persisted there.
pub fn build_client(env_file: &Path) -> Client {
    let entries = load_env_file(env_file);
    let auth_resolver = AuthResolver::from_resolver_fn(move |model_iden: genai::ModelIden| {
        let key = model_iden
            .adapter_kind
            .default_key_env_name()
            .and_then(|var| entries.get(var))
            .cloned();
        Ok(key.map(AuthData::from_single))
    });
    Client::builder().with_auth_resolver(auth_resolver).build()
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
