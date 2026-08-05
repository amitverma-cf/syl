use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use base64::Engine;
use daemon::events::{DaemonEvent, EventBus};
use extension_host::{ExtensionManifest, ExtensionProcess};
use extension_registry::ModelKind;
use serde_json::json;
use syl_core::app_config::app_config;
use syl_core::workspace_paths;

use crate::local_models::{
    discover_onnx_asr_models, discover_onnx_tts_models, kind_for_path, registry_entries,
};

const ASR_CAPABILITY: &str = "asr.transcribe/v1";
const TTS_CAPABILITY: &str = "tts.synthesize/v1";

fn f32_to_bytes(samples: &[f32]) -> Vec<u8> {
    samples.iter().flat_map(|s| s.to_le_bytes()).collect()
}

fn bytes_to_f32(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

macro_rules! extension_process_state {
    ($state:ident) => {
        #[derive(Default)]
        pub struct $state {
            loaded: Mutex<HashMap<String, Arc<ExtensionProcess>>>,
            event_bus: Mutex<Option<Arc<EventBus>>>,
        }

        impl $state {
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
        }
    };
}

extension_process_state!(OnnxAsrState);
extension_process_state!(OnnxTtsState);

fn prune_dead(loaded: &mut HashMap<String, Arc<ExtensionProcess>>) -> Vec<String> {
    let dead: Vec<String> = loaded
        .iter()
        .filter(|(_, process)| !process.is_alive())
        .map(|(name, _)| name.clone())
        .collect();
    loaded.retain(|_, process| process.is_alive());
    dead
}

fn build_asr_extension_manifest(
    encoder_path: &std::path::Path,
    decoder_path: &std::path::Path,
    tokenizer_path: &std::path::Path,
    engine_library_path: &std::path::Path,
) -> Result<ExtensionManifest, String> {
    let manifest = extension_host::find_extension("onnx-asr").ok_or_else(|| {
        "the onnx-asr extension is not installed under .syl/extensions/".to_string()
    })?;
    extension_host::with_backend_args(
        manifest,
        vec![
            "--library".to_string(),
            engine_library_path.display().to_string(),
            "--encoder".to_string(),
            encoder_path.display().to_string(),
            "--decoder".to_string(),
            decoder_path.display().to_string(),
            "--tokenizer".to_string(),
            tokenizer_path.display().to_string(),
        ],
    )
}

fn build_tts_extension_manifest(
    model_path: &std::path::Path,
    vocab_path: &std::path::Path,
    engine_library_path: &std::path::Path,
) -> Result<ExtensionManifest, String> {
    let manifest = extension_host::find_extension("onnx-tts").ok_or_else(|| {
        "the onnx-tts extension is not installed under .syl/extensions/".to_string()
    })?;
    extension_host::with_backend_args(
        manifest,
        vec![
            "--library".to_string(),
            engine_library_path.display().to_string(),
            "--model".to_string(),
            model_path.display().to_string(),
            "--vocab".to_string(),
            vocab_path.display().to_string(),
        ],
    )
}

#[tauri::command]
pub async fn load_asr_model(
    name: String,
    state: tauri::State<'_, OnnxAsrState>,
) -> Result<(), String> {
    if state.get_loaded(&name).is_some() {
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
    let engine_library_path = extension_registry::resolve_engine_library_path(
        &workspace_paths::registry_dir(),
        &workspace_paths::engines_dir(),
        &onnx_engine_config.id,
    )
    .map_err(|e| e.to_string())?;

    let manifest = build_asr_extension_manifest(
        &encoder_path,
        &decoder_path,
        &tokenizer_path,
        &engine_library_path,
    )?;

    let process = ExtensionProcess::spawn(manifest)
        .await
        .map_err(|e| e.to_string())?;

    crate::sync::lock(&state.loaded).insert(name, Arc::new(process));
    Ok(())
}

#[tauri::command]
pub async fn unload_asr_model(
    name: String,
    state: tauri::State<'_, OnnxAsrState>,
) -> Result<(), String> {
    let process = crate::sync::lock(&state.loaded)
        .remove(&name)
        .ok_or_else(|| format!("{name} is not loaded"))?;
    process.kill().await;
    Ok(())
}

#[tauri::command]
pub async fn load_tts_model(
    name: String,
    state: tauri::State<'_, OnnxTtsState>,
) -> Result<(), String> {
    if state.get_loaded(&name).is_some() {
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
    let engine_library_path = extension_registry::resolve_engine_library_path(
        &workspace_paths::registry_dir(),
        &workspace_paths::engines_dir(),
        &onnx_engine_config.id,
    )
    .map_err(|e| e.to_string())?;

    let manifest = build_tts_extension_manifest(&model_path, &vocab_path, &engine_library_path)?;

    let process = ExtensionProcess::spawn(manifest)
        .await
        .map_err(|e| e.to_string())?;

    crate::sync::lock(&state.loaded).insert(name, Arc::new(process));
    Ok(())
}

#[tauri::command]
pub async fn unload_tts_model(
    name: String,
    state: tauri::State<'_, OnnxTtsState>,
) -> Result<(), String> {
    let process = crate::sync::lock(&state.loaded)
        .remove(&name)
        .ok_or_else(|| format!("{name} is not loaded"))?;
    process.kill().await;
    Ok(())
}

#[tauri::command]
pub async fn transcribe_recording(
    model: String,
    seconds: f32,
    state: tauri::State<'_, OnnxAsrState>,
) -> Result<String, String> {
    let process = state
        .get_loaded(&model)
        .ok_or_else(|| format!("asr model {model} is not loaded; load it first in Settings"))?;

    let pcm =
        tauri::async_runtime::spawn_blocking(move || crate::audio::record_16khz_mono(seconds))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;

    let pcm_base64 = base64::engine::general_purpose::STANDARD.encode(f32_to_bytes(&pcm));
    let result = process
        .call(
            ASR_CAPABILITY,
            "asr/transcribe",
            json!({ "pcmBase64": pcm_base64 }),
        )
        .await
        .map_err(|e| e.to_string())?;

    result
        .get("text")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| "asr-worker response missing text".to_string())
}

#[tauri::command]
pub async fn speak_text(
    model: String,
    text: String,
    state: tauri::State<'_, OnnxTtsState>,
) -> Result<(), String> {
    let process = state
        .get_loaded(&model)
        .ok_or_else(|| format!("tts model {model} is not loaded; load it first in Settings"))?;

    let result = process
        .call(TTS_CAPABILITY, "tts/synthesize", json!({ "text": text }))
        .await
        .map_err(|e| e.to_string())?;

    let pcm_base64 = result
        .get("pcmBase64")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "tts-worker response missing pcmBase64".to_string())?;
    let pcm_bytes = base64::engine::general_purpose::STANDARD
        .decode(pcm_base64)
        .map_err(|e| e.to_string())?;
    let pcm = bytes_to_f32(&pcm_bytes);

    tauri::async_runtime::spawn_blocking(move || crate::audio::play_16khz_mono(&pcm))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}
