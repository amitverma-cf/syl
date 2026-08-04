use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use core_types::app_config::app_config;
use core_types::workspace_paths;
use engine_host::onnx_embedding::OnnxEmbeddingEngine;
use plugin_registry::ModelKind;

use crate::local_models::{discover_onnx_embedding_models, kind_for_path, registry_entries};

#[derive(Default)]
pub struct OnnxModelState {
    loaded: Mutex<HashMap<String, Arc<Mutex<OnnxEmbeddingEngine>>>>,
}

impl OnnxModelState {
    pub fn get_loaded(&self, name: &str) -> Option<Arc<Mutex<OnnxEmbeddingEngine>>> {
        crate::sync::lock(&self.loaded).get(name).cloned()
    }

    pub fn loaded_names(&self) -> HashSet<String> {
        crate::sync::lock(&self.loaded).keys().cloned().collect()
    }

    pub fn remove_loaded(&self, name: &str) {
        crate::sync::lock(&self.loaded).remove(name);
    }

    pub fn any_loaded(&self) -> Option<Arc<Mutex<OnnxEmbeddingEngine>>> {
        crate::sync::lock(&self.loaded).values().next().cloned()
    }
}

#[tauri::command]
pub fn load_embedding_model(
    name: String,
    state: tauri::State<'_, OnnxModelState>,
) -> Result<(), String> {
    if crate::sync::lock(&state.loaded).contains_key(&name) {
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

    let engine = OnnxEmbeddingEngine::load(&engine_library_path, &model_path, &tokenizer_path)
        .map_err(|e| e.to_string())?;

    crate::sync::lock(&state.loaded).insert(name, Arc::new(Mutex::new(engine)));
    Ok(())
}

#[tauri::command]
pub fn unload_embedding_model(
    name: String,
    state: tauri::State<'_, OnnxModelState>,
) -> Result<(), String> {
    crate::sync::lock(&state.loaded)
        .remove(&name)
        .map(|_| ())
        .ok_or_else(|| format!("{name} is not loaded"))
}

#[tauri::command]
pub async fn embed_text(
    model: String,
    text: String,
    state: tauri::State<'_, OnnxModelState>,
) -> Result<Vec<f32>, String> {
    let engine = state.get_loaded(&model).ok_or_else(|| {
        format!("embedding model {model} is not loaded; load it first in Settings")
    })?;

    tauri::async_runtime::spawn_blocking(move || {
        let mut engine = crate::sync::lock(&engine);
        engine.embed(&text)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}
