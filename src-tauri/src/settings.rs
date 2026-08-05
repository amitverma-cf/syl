use syl_core::workspace_paths;
use tauri_plugin_autostart::ManagerExt;

/// Every user-adjustable app setting, persisted to `.syl/settings.json`.
/// Security-sensitive trust anchors (registry poll URL, allowed hosts,
/// signing public key) are deliberately NOT here — see `syl_core::registry_trust`
/// — since letting a user (or malware) edit them would defeat their purpose.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default)]
    pub autostart: bool,
    #[serde(default)]
    pub telemetry_enabled: bool,
    /// Caps how many local (GGUF) models can be loaded into memory at once.
    /// `load_local_model` enforces this — loading past the cap fails with a
    /// clear error rather than silently exhausting system memory.
    #[serde(default = "default_max_concurrent_local_models")]
    pub max_concurrent_local_models: u32,
    /// Name of the flow loaded when a new conversation doesn't specify one.
    #[serde(default = "default_flow_name")]
    pub default_flow_name: String,
    /// How many tool-call round trips a single turn is allowed before the
    /// model must answer directly.
    #[serde(default = "default_max_tool_iterations")]
    pub max_tool_iterations: u32,
    /// Character budget `tools::compress_context` trims retrieved/prior
    /// context down to before it's folded into a prompt.
    #[serde(default = "default_context_budget_chars")]
    pub context_budget_chars: usize,
    /// Local (llama.cpp) chat engine tuning.
    #[serde(default = "default_local_engine_context_size")]
    pub local_engine_context_size: u32,
    #[serde(default = "default_local_engine_max_tokens")]
    pub local_engine_max_tokens: i32,
    /// Local Stable Diffusion image-generation tuning.
    #[serde(default = "default_image_engine_steps")]
    pub image_engine_steps: i32,
    #[serde(default = "default_image_engine_width")]
    pub image_engine_width: i32,
    #[serde(default = "default_image_engine_height")]
    pub image_engine_height: i32,
}

fn default_max_concurrent_local_models() -> u32 {
    3
}
fn default_flow_name() -> String {
    "default".to_string()
}
fn default_max_tool_iterations() -> u32 {
    8
}
fn default_context_budget_chars() -> usize {
    12000
}
fn default_local_engine_context_size() -> u32 {
    2048
}
fn default_local_engine_max_tokens() -> i32 {
    128
}
fn default_image_engine_steps() -> i32 {
    20
}
fn default_image_engine_width() -> i32 {
    512
}
fn default_image_engine_height() -> i32 {
    512
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            autostart: false,
            telemetry_enabled: false,
            max_concurrent_local_models: default_max_concurrent_local_models(),
            default_flow_name: default_flow_name(),
            max_tool_iterations: default_max_tool_iterations(),
            context_budget_chars: default_context_budget_chars(),
            local_engine_context_size: default_local_engine_context_size(),
            local_engine_max_tokens: default_local_engine_max_tokens(),
            image_engine_steps: default_image_engine_steps(),
            image_engine_width: default_image_engine_width(),
            image_engine_height: default_image_engine_height(),
        }
    }
}

pub fn load_settings() -> AppSettings {
    let path = workspace_paths::settings_file();
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return AppSettings::default();
    };
    serde_json::from_str(&contents).unwrap_or_default()
}

fn save_settings(settings: &AppSettings) -> Result<(), String> {
    let path = workspace_paths::settings_file();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

/// Applies the autostart setting to the OS's real autostart registration —
/// called once at startup (with whatever was last persisted) and again every
/// time the setting changes, so the OS-level state never drifts from what
/// Settings shows the user.
pub fn apply_autostart(app: &tauri::AppHandle, enabled: bool) {
    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    if let Err(err) = result {
        tracing::warn!(?err, enabled, "failed to apply autostart setting");
    }
}

#[tauri::command]
pub fn get_settings() -> AppSettings {
    load_settings()
}

#[tauri::command]
pub fn update_settings(settings: AppSettings, app: tauri::AppHandle) -> Result<(), String> {
    apply_autostart(&app, settings.autostart);
    save_settings(&settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_are_conservative() {
        let settings = AppSettings::default();
        assert!(!settings.autostart);
        assert!(!settings.telemetry_enabled);
        assert!(settings.max_concurrent_local_models > 0);
    }

    #[test]
    fn settings_round_trip_through_json() {
        let settings = AppSettings {
            autostart: true,
            telemetry_enabled: true,
            max_concurrent_local_models: 5,
            ..AppSettings::default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        let parsed: AppSettings = serde_json::from_str(&json).unwrap();
        assert!(parsed.autostart);
        assert!(parsed.telemetry_enabled);
        assert_eq!(parsed.max_concurrent_local_models, 5);
    }

    #[test]
    fn missing_fields_in_stored_json_fall_back_to_defaults() {
        let parsed: AppSettings = serde_json::from_str("{}").unwrap();
        assert!(!parsed.autostart);
        assert_eq!(parsed.max_concurrent_local_models, 3);
        assert_eq!(parsed.default_flow_name, "default");
        assert_eq!(parsed.max_tool_iterations, 8);
        assert_eq!(parsed.context_budget_chars, 12000);
        assert_eq!(parsed.local_engine_context_size, 2048);
        assert_eq!(parsed.local_engine_max_tokens, 128);
        assert_eq!(parsed.image_engine_steps, 20);
        assert_eq!(parsed.image_engine_width, 512);
        assert_eq!(parsed.image_engine_height, 512);
    }
}
