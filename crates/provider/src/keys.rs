use std::collections::BTreeMap;
use std::path::Path;

/// The cloud providers this app has a built-in catalog entry for, keyed by the environment
/// variable name `genai` resolves for that provider by default.
pub const KNOWN_PROVIDERS: &[(&str, &str)] = &[
    ("OpenAI", "OPENAI_API_KEY"),
    ("Anthropic", "ANTHROPIC_API_KEY"),
    ("Gemini", "GEMINI_API_KEY"),
    ("Groq", "GROQ_API_KEY"),
    ("xAI", "XAI_API_KEY"),
    ("DeepSeek", "DEEPSEEK_API_KEY"),
    ("Cohere", "COHERE_API_KEY"),
];

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub name: String,
    pub env_var: String,
    pub configured: bool,
}

/// Parses a minimal `.env` file: one `KEY=VALUE` pair per line, blank lines and `#` comments
/// ignored. Not a general-purpose dotenv implementation — API keys don't need quoting,
/// multi-line values, or variable expansion, so there's nothing else to support.
pub fn load_env_file(path: &Path) -> BTreeMap<String, String> {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    contents
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            Some((key.trim().to_string(), value.trim().to_string()))
        })
        .collect()
}

pub fn write_env_file(path: &Path, entries: &BTreeMap<String, String>) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let contents: String = entries
        .iter()
        .map(|(key, value)| format!("{key}={value}\n"))
        .collect();
    std::fs::write(path, contents)
}

pub fn set_api_key(path: &Path, env_var: &str, key: &str) -> std::io::Result<()> {
    let mut entries = load_env_file(path);
    entries.insert(env_var.to_string(), key.to_string());
    write_env_file(path, &entries)
}

pub fn list_providers(path: &Path) -> Vec<ProviderInfo> {
    let entries = load_env_file(path);
    KNOWN_PROVIDERS
        .iter()
        .map(|(name, env_var)| ProviderInfo {
            name: name.to_string(),
            env_var: env_var.to_string(),
            configured: entries.get(*env_var).is_some_and(|v| !v.is_empty()),
        })
        .collect()
}
