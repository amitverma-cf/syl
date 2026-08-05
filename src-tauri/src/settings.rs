use syl_core::workspace_paths;
use tauri_plugin_autostart::ManagerExt;

/// Genuinely user-adjustable app settings, persisted to `.syl/settings.json`
/// (distinct from `config/app.json`, which is committed, build-time, repo-
/// level config the app ships with and never writes back to).
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
}

fn default_max_concurrent_local_models() -> u32 {
    3
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            autostart: false,
            telemetry_enabled: false,
            max_concurrent_local_models: default_max_concurrent_local_models(),
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
    }
}
