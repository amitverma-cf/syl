use std::path::Path;

use core_types::app_config::app_config;
use futures_util::StreamExt;
use genai::adapter::AdapterKind;
use genai::chat::{
    ChatMessage, ChatRequest, ChatStreamEvent, Tool as GenaiTool, ToolResponse as GenaiToolResponse,
};
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
    #[error("tool-calling loop exceeded {0} iterations without a final answer")]
    ToolIterationLimitExceeded(u32),
}

const CUSTOM_PREFIX: &str = "custom::";

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

#[allow(clippy::too_many_arguments)]
pub async fn chat_with_tools(
    client: &Client,
    model_id: &str,
    system_prompt: Option<&str>,
    user_prompt: &str,
    tools: &[tool::ToolSpec],
    executor: &tool::ToolExecutor,
    conversation_id: &str,
    mut on_piece: impl FnMut(&str),
) -> Result<String, CloudChatError> {
    let mut messages = Vec::new();
    if let Some(system_prompt) = system_prompt {
        messages.push(ChatMessage::system(system_prompt));
    }
    messages.push(ChatMessage::user(user_prompt));

    let genai_tools: Vec<GenaiTool> = tools
        .iter()
        .map(|spec| {
            GenaiTool::new(spec.name.clone())
                .with_description(spec.description.clone())
                .with_schema(spec.input_schema.clone())
        })
        .collect();

    let max_tool_iterations = app_config().max_tool_iterations;
    for _ in 0..max_tool_iterations {
        let mut request = ChatRequest::new(messages.clone());
        if !genai_tools.is_empty() {
            request = request.with_tools(genai_tools.clone());
        }
        let response = client.exec_chat(model_id, request, None).await?;
        let tool_calls: Vec<_> = response.tool_calls().into_iter().cloned().collect();
        if tool_calls.is_empty() {
            let text = response.into_first_text().unwrap_or_default();
            on_piece(&text);
            return Ok(text);
        }

        messages.push(ChatMessage::from(tool_calls.clone()));
        for call in &tool_calls {
            let result = executor
                .call(conversation_id, &call.fn_name, call.fn_arguments.clone())
                .await;
            let content = match result {
                Ok(value) => value.to_string(),
                Err(err) => format!("error: {err}"),
            };
            messages.push(ChatMessage::from(GenaiToolResponse::from_tool_call(
                call, content,
            )));
        }
    }

    Err(CloudChatError::ToolIterationLimitExceeded(
        max_tool_iterations,
    ))
}
