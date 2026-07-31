use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use base64::Engine;
use core_types::app_config::app_config;
use core_types::workspace_paths;
use engine_host::stable_diffusion::SdEngine;
use plugin_registry::ModelKind;

use crate::local_models::{discover_gguf_models, kind_for_path, registry_entries};

#[derive(Default)]
pub struct SdModelState {
    loaded: Mutex<HashMap<String, Arc<Mutex<SdEngine>>>>,
}

impl SdModelState {
    pub fn get_loaded(&self, name: &str) -> Option<Arc<Mutex<SdEngine>>> {
        crate::sync::lock(&self.loaded).get(name).cloned()
    }
}

#[tauri::command]
pub fn load_image_model(name: String, state: tauri::State<'_, SdModelState>) -> Result<(), String> {
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
        Some(ModelKind::Image) => {}
        Some(_) => return Err(format!("{name} is not an image model")),
        None => {
            return Err(format!(
                "{name} is not categorized yet; set its kind in Settings before loading"
            ));
        }
    }

    let sd_engine_config = &app_config().sd_engine;
    let engine_library_path = plugin_registry::resolve_engine_library_path(
        &workspace_paths::registry_dir(),
        &workspace_paths::engines_dir(),
        &sd_engine_config.id,
    )
    .map_err(|e| e.to_string())?;

    let n_threads = std::thread::available_parallelism()
        .map(|n| n.get() as i32)
        .unwrap_or(4);
    let engine =
        SdEngine::load(&engine_library_path, &model_path, n_threads).map_err(|e| e.to_string())?;

    crate::sync::lock(&state.loaded).insert(name, Arc::new(Mutex::new(engine)));
    Ok(())
}

#[tauri::command]
pub fn unload_image_model(
    name: String,
    state: tauri::State<'_, SdModelState>,
) -> Result<(), String> {
    crate::sync::lock(&state.loaded)
        .remove(&name)
        .map(|_| ())
        .ok_or_else(|| format!("{name} is not loaded"))
}

#[tauri::command]
pub async fn generate_image(
    model: String,
    prompt: String,
    negative_prompt: String,
    state: tauri::State<'_, SdModelState>,
) -> Result<String, String> {
    let engine = state
        .get_loaded(&model)
        .ok_or_else(|| format!("image model {model} is not loaded; load it first in Settings"))?;

    let sd_engine_config = &app_config().sd_engine;
    let (steps, width, height) = (
        sd_engine_config.steps,
        sd_engine_config.width,
        sd_engine_config.height,
    );

    let png = tauri::async_runtime::spawn_blocking(move || {
        let mut engine = crate::sync::lock(&engine);
        engine.txt2img(&prompt, &negative_prompt, width, height, steps, -1)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    let images_dir = workspace_paths::workspace_root().join("images");
    std::fs::create_dir_all(&images_dir).map_err(|e| e.to_string())?;
    let filename = format!("{}.png", uuid::Uuid::new_v4());
    std::fs::write(images_dir.join(&filename), &png).map_err(|e| e.to_string())?;

    let data_url = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&png)
    );
    Ok(data_url)
}
