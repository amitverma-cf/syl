#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudModel {
    pub id: String,
    pub provider: String,
    pub label: String,
}

/// A starter catalog of stable model IDs per provider, for the UI's model picker. Users can
/// also type any other model ID `genai` can resolve (it dispatches by name prefix — see
/// `genai::adapter::AdapterKind`), so this list is a convenience, not an allowlist.
pub fn list_cloud_models() -> Vec<CloudModel> {
    vec![
        CloudModel {
            id: "gpt-4.1".to_string(),
            provider: "OpenAI".to_string(),
            label: "GPT-4.1".to_string(),
        },
        CloudModel {
            id: "gpt-4.1-mini".to_string(),
            provider: "OpenAI".to_string(),
            label: "GPT-4.1 mini".to_string(),
        },
        CloudModel {
            id: "claude-opus-5".to_string(),
            provider: "Anthropic".to_string(),
            label: "Claude Opus 5".to_string(),
        },
        CloudModel {
            id: "claude-sonnet-5".to_string(),
            provider: "Anthropic".to_string(),
            label: "Claude Sonnet 5".to_string(),
        },
        CloudModel {
            id: "claude-haiku-4-5".to_string(),
            provider: "Anthropic".to_string(),
            label: "Claude Haiku 4.5".to_string(),
        },
        CloudModel {
            id: "gemini-2.0-flash".to_string(),
            provider: "Gemini".to_string(),
            label: "Gemini 2.0 Flash".to_string(),
        },
        CloudModel {
            id: "groq::llama-3.3-70b-versatile".to_string(),
            provider: "Groq".to_string(),
            label: "Llama 3.3 70B (Groq)".to_string(),
        },
        CloudModel {
            id: "grok-3-mini".to_string(),
            provider: "xAI".to_string(),
            label: "Grok 3 mini".to_string(),
        },
        CloudModel {
            id: "deepseek-chat".to_string(),
            provider: "DeepSeek".to_string(),
            label: "DeepSeek Chat".to_string(),
        },
        CloudModel {
            id: "command-r-plus".to_string(),
            provider: "Cohere".to_string(),
            label: "Command R+".to_string(),
        },
    ]
}
