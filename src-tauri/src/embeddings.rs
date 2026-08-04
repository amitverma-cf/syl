use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use core_types::app_config::app_config;
use core_types::workspace_paths;
use daemon::events::{DaemonEvent, EventBus};
use extension_host::{ExtensionManifest, ExtensionProcess};
use plugin_registry::ModelKind;
use serde_json::json;

use crate::local_models::{discover_onnx_embedding_models, kind_for_path, registry_entries};

const CAPABILITY: &str = "embedding.embed/v1";

#[derive(Default)]
pub struct OnnxModelState {
    loaded: Mutex<HashMap<String, Arc<ExtensionProcess>>>,
    event_bus: Mutex<Option<Arc<EventBus>>>,
}

impl OnnxModelState {
    pub fn set_event_bus(&self, event_bus: Arc<EventBus>) {
        *crate::sync::lock(&self.event_bus) = Some(event_bus);
    }

    fn publish_crashed(&self, name: &str) {
        if let Some(bus) = crate::sync::lock(&self.event_bus).as_ref() {
            bus.publish(DaemonEvent::LocalModelCrashed {
                name: name.to_string(),
            });
        }
    }

    pub fn get_loaded(&self, name: &str) -> Option<Arc<ExtensionProcess>> {
        let mut loaded = crate::sync::lock(&self.loaded);
        match loaded.get(name) {
            Some(process) if !process.is_alive() => {
                loaded.remove(name);
                drop(loaded);
                self.publish_crashed(name);
                None
            }
            Some(process) => Some(process.clone()),
            None => None,
        }
    }

    pub fn loaded_names(&self) -> HashSet<String> {
        let mut loaded = crate::sync::lock(&self.loaded);
        let crashed = prune_dead(&mut loaded);
        let result = loaded.keys().cloned().collect();
        drop(loaded);
        for name in crashed {
            self.publish_crashed(&name);
        }
        result
    }

    pub fn remove_loaded(&self, name: &str) {
        crate::sync::lock(&self.loaded).remove(name);
    }

    pub fn any_loaded(&self) -> Option<Arc<ExtensionProcess>> {
        let mut loaded = crate::sync::lock(&self.loaded);
        let crashed = prune_dead(&mut loaded);
        let result = loaded.values().next().cloned();
        drop(loaded);
        for name in crashed {
            self.publish_crashed(&name);
        }
        result
    }
}

fn prune_dead(loaded: &mut HashMap<String, Arc<ExtensionProcess>>) -> Vec<String> {
    let dead: Vec<String> = loaded
        .iter()
        .filter(|(_, process)| !process.is_alive())
        .map(|(name, _)| name.clone())
        .collect();
    loaded.retain(|_, process| process.is_alive());
    dead
}

fn build_embedding_extension_manifest(
    model_path: &std::path::Path,
    engine_library_path: &std::path::Path,
    tokenizer_path: &std::path::Path,
) -> Result<ExtensionManifest, String> {
    let manifest = extension_host::find_extension("onnx-embedding").ok_or_else(|| {
        "the onnx-embedding extension is not installed under .syl/extensions/".to_string()
    })?;
    extension_host::with_backend_args(
        manifest,
        vec![
            "--library".to_string(),
            engine_library_path.display().to_string(),
            "--model".to_string(),
            model_path.display().to_string(),
            "--tokenizer".to_string(),
            tokenizer_path.display().to_string(),
        ],
    )
}

#[tauri::command]
pub async fn load_embedding_model(
    name: String,
    state: tauri::State<'_, OnnxModelState>,
) -> Result<(), String> {
    if state.get_loaded(&name).is_some() {
        return Ok(());
    }

    let entries = registry_entries();
    let (_, model_path, tokenizer_path, _) = discover_onnx_embedding_models()
        .into_iter()
        .find(|(model_name, _, _, _)| *model_name == name)
        .ok_or_else(|| {
            format!(
                "no onnx model named {name} in {}",
                workspace_paths::models_dir().display()
            )
        })?;

    match kind_for_path(&entries, &model_path) {
        Some(ModelKind::Embedding) => {}
        Some(_) => return Err(format!("{name} is not an embedding model")),
        None => {
            return Err(format!(
                "{name} is not categorized yet; set its kind in Settings before loading"
            ));
        }
    }

    let onnx_engine_config = &app_config().onnx_engine;
    let engine_library_path = plugin_registry::resolve_engine_library_path(
        &workspace_paths::registry_dir(),
        &workspace_paths::engines_dir(),
        &onnx_engine_config.id,
    )
    .map_err(|e| e.to_string())?;

    let manifest =
        build_embedding_extension_manifest(&model_path, &engine_library_path, &tokenizer_path)?;

    let process = ExtensionProcess::spawn(manifest)
        .await
        .map_err(|e| e.to_string())?;

    crate::sync::lock(&state.loaded).insert(name, Arc::new(process));
    Ok(())
}

#[tauri::command]
pub async fn unload_embedding_model(
    name: String,
    state: tauri::State<'_, OnnxModelState>,
) -> Result<(), String> {
    let process = crate::sync::lock(&state.loaded)
        .remove(&name)
        .ok_or_else(|| format!("{name} is not loaded"))?;
    process.kill().await;
    Ok(())
}

#[tauri::command]
pub async fn embed_text(
    model: String,
    text: String,
    state: tauri::State<'_, OnnxModelState>,
) -> Result<Vec<f32>, String> {
    let process = state.get_loaded(&model).ok_or_else(|| {
        format!("embedding model {model} is not loaded; load it first in Settings")
    })?;

    let result = process
        .call(CAPABILITY, "embedding/embed", json!({ "text": text }))
        .await
        .map_err(|e| e.to_string())?;

    serde_json::from_value(
        result
            .get("vector")
            .cloned()
            .ok_or_else(|| "embedding-worker response missing vector".to_string())?,
    )
    .map_err(|e| e.to_string())
}
