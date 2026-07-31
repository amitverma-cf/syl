use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use core_types::app_config::app_config;
use core_types::workspace_paths;
use engine_host::onnx_asr::OnnxAsrEngine;
use engine_host::onnx_tts::OnnxTtsEngine;
use plugin_registry::ModelKind;

use crate::local_models::{
    discover_onnx_asr_models, discover_onnx_tts_models, kind_for_path, registry_entries,
};

#[derive(Default)]
pub struct OnnxAsrState {
    loaded: Mutex<HashMap<String, Arc<Mutex<OnnxAsrEngine>>>>,
}

impl OnnxAsrState {
    pub fn get_loaded(&self, name: &str) -> Option<Arc<Mutex<OnnxAsrEngine>>> {
        crate::sync::lock(&self.loaded).get(name).cloned()
    }

    pub fn loaded_names(&self) -> HashSet<String> {
        crate::sync::lock(&self.loaded).keys().cloned().collect()
    }

    pub fn remove_loaded(&self, name: &str) {
        crate::sync::lock(&self.loaded).remove(name);
    }
}

#[derive(Default)]
pub struct OnnxTtsState {
    loaded: Mutex<HashMap<String, Arc<Mutex<OnnxTtsEngine>>>>,
}

impl OnnxTtsState {
    pub fn get_loaded(&self, name: &str) -> Option<Arc<Mutex<OnnxTtsEngine>>> {
        crate::sync::lock(&self.loaded).get(name).cloned()
    }

    pub fn loaded_names(&self) -> HashSet<String> {
        crate::sync::lock(&self.loaded).keys().cloned().collect()
    }

    pub fn remove_loaded(&self, name: &str) {
        crate::sync::lock(&self.loaded).remove(name);
    }
}

#[tauri::command]
pub fn load_asr_model(name: String, state: tauri::State<'_, OnnxAsrState>) -> Result<(), String> {
    if crate::sync::lock(&state.loaded).contains_key(&name) {
        return Ok(());
    }

    let entries = registry_entries();
    let (_, encoder_path, decoder_path, tokenizer_path, _) = discover_onnx_asr_models()
        .into_iter()
        .find(|(model_name, _, _, _, _)| *model_name == name)
        .ok_or_else(|| {
            format!(
                "no onnx asr model named {name} in {}",
                workspace_paths::models_dir().display()
            )
        })?;

    match kind_for_path(&entries, &encoder_path) {
        Some(ModelKind::Asr) => {}
        Some(_) => return Err(format!("{name} is not an ASR model")),
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

    let engine = OnnxAsrEngine::load(
        &engine_library_path,
        &encoder_path,
        &decoder_path,
        &tokenizer_path,
    )
    .map_err(|e| e.to_string())?;

    crate::sync::lock(&state.loaded).insert(name, Arc::new(Mutex::new(engine)));
    Ok(())
}

#[tauri::command]
pub fn unload_asr_model(name: String, state: tauri::State<'_, OnnxAsrState>) -> Result<(), String> {
    crate::sync::lock(&state.loaded)
        .remove(&name)
        .map(|_| ())
        .ok_or_else(|| format!("{name} is not loaded"))
}

#[tauri::command]
pub fn load_tts_model(name: String, state: tauri::State<'_, OnnxTtsState>) -> Result<(), String> {
    if crate::sync::lock(&state.loaded).contains_key(&name) {
        return Ok(());
    }

    let entries = registry_entries();
    let (_, model_path, vocab_path, _) = discover_onnx_tts_models()
        .into_iter()
        .find(|(model_name, _, _, _)| *model_name == name)
        .ok_or_else(|| {
            format!(
                "no onnx tts model named {name} in {}",
                workspace_paths::models_dir().display()
            )
        })?;

    match kind_for_path(&entries, &model_path) {
        Some(ModelKind::Tts) => {}
        Some(_) => return Err(format!("{name} is not a TTS model")),
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

    let engine = OnnxTtsEngine::load(&engine_library_path, &model_path, &vocab_path)
        .map_err(|e| e.to_string())?;

    crate::sync::lock(&state.loaded).insert(name, Arc::new(Mutex::new(engine)));
    Ok(())
}

#[tauri::command]
pub fn unload_tts_model(name: String, state: tauri::State<'_, OnnxTtsState>) -> Result<(), String> {
    crate::sync::lock(&state.loaded)
        .remove(&name)
        .map(|_| ())
        .ok_or_else(|| format!("{name} is not loaded"))
}

#[tauri::command]
pub async fn transcribe_recording(
    model: String,
    seconds: f32,
    state: tauri::State<'_, OnnxAsrState>,
) -> Result<String, String> {
    let engine = state
        .get_loaded(&model)
        .ok_or_else(|| format!("asr model {model} is not loaded; load it first in Settings"))?;

    let pcm =
        tauri::async_runtime::spawn_blocking(move || crate::audio::record_16khz_mono(seconds))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;

    tauri::async_runtime::spawn_blocking(move || {
        let mut engine = crate::sync::lock(&engine);
        engine.transcribe(&pcm)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn speak_text(
    model: String,
    text: String,
    state: tauri::State<'_, OnnxTtsState>,
) -> Result<(), String> {
    let engine = state
        .get_loaded(&model)
        .ok_or_else(|| format!("tts model {model} is not loaded; load it first in Settings"))?;

    let pcm = tauri::async_runtime::spawn_blocking(move || {
        let mut engine = crate::sync::lock(&engine);
        engine.synthesize(&text)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    tauri::async_runtime::spawn_blocking(move || crate::audio::play_16khz_mono(&pcm))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}
