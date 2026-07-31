use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use core_types::app_config::app_config;
use core_types::workspace_paths;
use engine_host::llama::LlamaEngine;
use plugin_registry::{resolve_local_path, ModelEntry, ModelKind};

#[derive(Default)]
pub struct LocalModelState {
    loaded: Mutex<HashMap<String, Arc<Mutex<LlamaEngine>>>>,
}

impl LocalModelState {
    pub fn get_loaded(&self, name: &str) -> Option<Arc<Mutex<LlamaEngine>>> {
        crate::sync::lock(&self.loaded).get(name).cloned()
    }

    pub fn any_loaded(&self) -> Option<Arc<Mutex<LlamaEngine>>> {
        crate::sync::lock(&self.loaded).values().next().cloned()
    }
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalModelInfo {
    pub name: String,
    pub size_bytes: u64,
    pub loaded: bool,
    pub kind: Option<ModelKind>,
}

pub(crate) fn discover_gguf_models() -> Vec<(String, PathBuf, u64)> {
    let dir = workspace_paths::models_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut models: Vec<(String, PathBuf, u64)> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "gguf"))
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_stem()?.to_string_lossy().into_owned();
            let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
            Some((name, path, size_bytes))
        })
        .collect();
    models.sort_by(|a, b| a.0.cmp(&b.0));
    models
}

fn discover_folder_models(required_files: &[&str]) -> Vec<(String, PathBuf, u64)> {
    let dir = workspace_paths::models_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut models: Vec<(String, PathBuf, u64)> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| {
            let dir_path = entry.path();
            if !required_files.iter().all(|f| dir_path.join(f).exists()) {
                return None;
            }
            let name = dir_path.file_name()?.to_string_lossy().into_owned();
            let size_bytes = required_files
                .iter()
                .filter_map(|f| std::fs::metadata(dir_path.join(f)).ok())
                .map(|m| m.len())
                .sum();
            Some((name, dir_path, size_bytes))
        })
        .collect();
    models.sort_by(|a, b| a.0.cmp(&b.0));
    models
}

pub(crate) fn discover_onnx_embedding_models() -> Vec<(String, PathBuf, PathBuf, u64)> {
    discover_folder_models(&["model.onnx", "tokenizer.json"])
        .into_iter()
        .map(|(name, dir, size)| {
            (
                name.clone(),
                dir.join("model.onnx"),
                dir.join("tokenizer.json"),
                size,
            )
        })
        .collect()
}

pub(crate) fn discover_onnx_asr_models() -> Vec<(String, PathBuf, PathBuf, PathBuf, u64)> {
    discover_folder_models(&["encoder_model.onnx", "decoder_model.onnx", "tokenizer.json"])
        .into_iter()
        .map(|(name, dir, size)| {
            (
                name,
                dir.join("encoder_model.onnx"),
                dir.join("decoder_model.onnx"),
                dir.join("tokenizer.json"),
                size,
            )
        })
        .collect()
}

pub(crate) fn discover_onnx_tts_models() -> Vec<(String, PathBuf, PathBuf, u64)> {
    discover_folder_models(&["model.onnx", "vocab.json"])
        .into_iter()
        .map(|(name, dir, size)| (name, dir.join("model.onnx"), dir.join("vocab.json"), size))
        .collect()
}

fn read_gguf_quantization(path: &Path) -> String {
    let Ok(library_path) = plugin_registry::resolve_engine_library_path(
        &workspace_paths::registry_dir(),
        &workspace_paths::engines_dir(),
        &app_config().local_engine.id,
    ) else {
        return "unknown".to_string();
    };
    engine_host::gguf_meta::read_quantization(&library_path, path)
        .unwrap_or_else(|_| "unknown".to_string())
}

pub(crate) fn registry_entries() -> Vec<ModelEntry> {
    plugin_registry::load_model_entries(&workspace_paths::registry_dir()).unwrap_or_default()
}

pub(crate) fn kind_for_path(entries: &[ModelEntry], path: &Path) -> Option<ModelKind> {
    entries.iter().find_map(|entry| {
        let resolved =
            resolve_local_path(&entry.download_url, &workspace_paths::models_dir()).ok()?;
        (resolved == path).then_some(entry.kind)
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ModelFamily {
    Gguf,
    OnnxEmbedding,
    OnnxAsr,
    OnnxTts,
}

impl ModelFamily {
    fn only_allows(self) -> Option<ModelKind> {
        match self {
            ModelFamily::Gguf => None,
            ModelFamily::OnnxEmbedding => Some(ModelKind::Embedding),
            ModelFamily::OnnxAsr => Some(ModelKind::Asr),
            ModelFamily::OnnxTts => Some(ModelKind::Tts),
        }
    }
}

#[tauri::command]
pub fn list_local_models(
    state: tauri::State<'_, LocalModelState>,
    onnx_embedding_state: tauri::State<'_, crate::embeddings::OnnxModelState>,
    onnx_asr_state: tauri::State<'_, crate::speech::OnnxAsrState>,
    onnx_tts_state: tauri::State<'_, crate::speech::OnnxTtsState>,
) -> Vec<LocalModelInfo> {
    let loaded = crate::sync::lock(&state.loaded);
    let entries = registry_entries();
    let gguf_models = discover_gguf_models()
        .into_iter()
        .map(|(name, path, size_bytes)| LocalModelInfo {
            loaded: loaded.contains_key(&name),
            kind: kind_for_path(&entries, &path),
            name,
            size_bytes,
        });

    let embedding_loaded = onnx_embedding_state.loaded_names();
    let embedding_models = discover_onnx_embedding_models().into_iter().map(
        |(name, path, _tokenizer_path, size_bytes)| LocalModelInfo {
            loaded: embedding_loaded.contains(&name),
            kind: kind_for_path(&entries, &path),
            name,
            size_bytes,
        },
    );

    let asr_loaded = onnx_asr_state.loaded_names();
    let asr_models = discover_onnx_asr_models().into_iter().map(
        |(name, encoder_path, _decoder_path, _tokenizer_path, size_bytes)| LocalModelInfo {
            loaded: asr_loaded.contains(&name),
            kind: kind_for_path(&entries, &encoder_path),
            name,
            size_bytes,
        },
    );

    let tts_loaded = onnx_tts_state.loaded_names();
    let tts_models = discover_onnx_tts_models().into_iter().map(
        |(name, model_path, _vocab_path, size_bytes)| LocalModelInfo {
            loaded: tts_loaded.contains(&name),
            kind: kind_for_path(&entries, &model_path),
            name,
            size_bytes,
        },
    );

    gguf_models
        .chain(embedding_models)
        .chain(asr_models)
        .chain(tts_models)
        .collect()
}

#[tauri::command]
pub fn set_local_model_kind(name: String, kind: ModelKind) -> Result<(), String> {
    let gguf_hit = discover_gguf_models()
        .into_iter()
        .find(|(model_name, _, _)| *model_name == name)
        .map(|(_, path, size_bytes)| (path, size_bytes, ModelFamily::Gguf));
    let embedding_hit = discover_onnx_embedding_models()
        .into_iter()
        .find(|(model_name, _, _, _)| *model_name == name)
        .map(|(_, path, _tokenizer_path, size_bytes)| {
            (path, size_bytes, ModelFamily::OnnxEmbedding)
        });
    let asr_hit = discover_onnx_asr_models()
        .into_iter()
        .find(|(model_name, _, _, _, _)| *model_name == name)
        .map(|(_, encoder_path, _, _, size_bytes)| {
            (encoder_path, size_bytes, ModelFamily::OnnxAsr)
        });
    let tts_hit = discover_onnx_tts_models()
        .into_iter()
        .find(|(model_name, _, _, _)| *model_name == name)
        .map(|(_, model_path, _vocab_path, size_bytes)| {
            (model_path, size_bytes, ModelFamily::OnnxTts)
        });

    let (path, size_bytes, family) = gguf_hit
        .or(embedding_hit)
        .or(asr_hit)
        .or(tts_hit)
        .ok_or_else(|| {
            format!(
                "no model named {name} in {}",
                workspace_paths::models_dir().display()
            )
        })?;

    if let Some(only_allowed) = family.only_allows() {
        if kind != only_allowed {
            return Err(format!("{name} can only be marked as {only_allowed:?}"));
        }
    }

    let required_engine = match family {
        ModelFamily::OnnxEmbedding | ModelFamily::OnnxAsr | ModelFamily::OnnxTts => {
            app_config().onnx_engine.id.clone()
        }
        ModelFamily::Gguf => match kind {
            ModelKind::Chat | ModelKind::Embedding => app_config().local_engine.id.clone(),
            ModelKind::Image => app_config().sd_engine.id.clone(),
            ModelKind::Asr | ModelKind::Tts => {
                return Err(format!(
                    "{name} is a .gguf model and cannot be marked as {kind:?}"
                ));
            }
        },
    };

    let registry_dir = workspace_paths::registry_dir();
    let local_models_file = registry_dir.join("local.models.json");
    let mut entries =
        plugin_registry::load_local_model_entries(&registry_dir).map_err(|e| e.to_string())?;
    entries.retain(|entry| entry.name != name);
    let quantization = if family == ModelFamily::Gguf {
        read_gguf_quantization(&path)
    } else {
        "unknown".to_string()
    };
    entries.push(ModelEntry {
        quantization,
        name,
        kind,
        size_bytes,
        required_engine,
        download_url: format!("file://{}", path.display().to_string().replace('\\', "/")),
        sha256: None,
        extra_files: Vec::new(),
    });

    if let Some(parent) = local_models_file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())?;
    std::fs::write(&local_models_file, json).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_local_model(
    name: String,
    state: tauri::State<'_, LocalModelState>,
) -> Result<(), String> {
    if crate::sync::lock(&state.loaded).contains_key(&name) {
        return Ok(());
    }

    let entries = registry_entries();
    let (_, model_path, _) = discover_gguf_models()
        .into_iter()
        .find(|(model_name, _, _)| *model_name == name)
        .ok_or_else(|| {
            format!(
                "no .gguf file named {name} in {}",
                workspace_paths::models_dir().display()
            )
        })?;

    match kind_for_path(&entries, &model_path) {
        Some(ModelKind::Chat) => {}
        Some(ModelKind::Embedding | ModelKind::Image | ModelKind::Asr | ModelKind::Tts) => {
            return Err(format!("{name} is not a chat model"));
        }
        None => {
            return Err(format!(
                "{name} is not categorized yet; set its kind in Settings before loading"
            ));
        }
    }

    let local_engine_config = &app_config().local_engine;
    let engine_library_path = plugin_registry::resolve_engine_library_path(
        &workspace_paths::registry_dir(),
        &workspace_paths::engines_dir(),
        &local_engine_config.id,
    )
    .map_err(|e| e.to_string())?;

    let engine = LlamaEngine::load(
        &engine_library_path,
        &model_path,
        local_engine_config.context_size,
        false,
    )
    .map_err(|e| e.to_string())?;

    crate::sync::lock(&state.loaded).insert(name, Arc::new(Mutex::new(engine)));
    Ok(())
}

#[tauri::command]
pub fn unload_local_model(
    name: String,
    state: tauri::State<'_, LocalModelState>,
) -> Result<(), String> {
    crate::sync::lock(&state.loaded)
        .remove(&name)
        .map(|_| ())
        .ok_or_else(|| format!("{name} is not loaded"))
}

#[tauri::command]
pub fn delete_local_model(
    name: String,
    state: tauri::State<'_, LocalModelState>,
    onnx_embedding_state: tauri::State<'_, crate::embeddings::OnnxModelState>,
    onnx_asr_state: tauri::State<'_, crate::speech::OnnxAsrState>,
    onnx_tts_state: tauri::State<'_, crate::speech::OnnxTtsState>,
) -> Result<(), String> {
    crate::sync::lock(&state.loaded).remove(&name);
    onnx_embedding_state.remove_loaded(&name);
    onnx_asr_state.remove_loaded(&name);
    onnx_tts_state.remove_loaded(&name);

    let gguf_hit = discover_gguf_models()
        .into_iter()
        .find(|(model_name, _, _)| *model_name == name)
        .map(|(_, path, _)| path);
    let embedding_hit = discover_onnx_embedding_models()
        .into_iter()
        .find(|(model_name, _, _, _)| *model_name == name)
        .and_then(|(_, path, _, _)| path.parent().map(|p| p.to_path_buf()));
    let asr_hit = discover_onnx_asr_models()
        .into_iter()
        .find(|(model_name, _, _, _, _)| *model_name == name)
        .and_then(|(_, encoder_path, _, _, _)| encoder_path.parent().map(|p| p.to_path_buf()));
    let tts_hit = discover_onnx_tts_models()
        .into_iter()
        .find(|(model_name, _, _, _)| *model_name == name)
        .and_then(|(_, model_path, _, _)| model_path.parent().map(|p| p.to_path_buf()));

    let path = gguf_hit
        .or(embedding_hit)
        .or(asr_hit)
        .or(tts_hit)
        .ok_or_else(|| {
            format!(
                "no model named {name} in {}",
                workspace_paths::models_dir().display()
            )
        })?;

    if path.is_dir() {
        std::fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
    } else {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }

    let registry_dir = workspace_paths::registry_dir();
    let local_models_file = registry_dir.join("local.models.json");
    let entries =
        plugin_registry::load_local_model_entries(&registry_dir).map_err(|e| e.to_string())?;
    let remaining: Vec<_> = entries
        .into_iter()
        .filter(|entry| entry.name != name)
        .collect();
    let json = serde_json::to_string_pretty(&remaining).map_err(|e| e.to_string())?;
    std::fs::write(&local_models_file, json).map_err(|e| e.to_string())
}
