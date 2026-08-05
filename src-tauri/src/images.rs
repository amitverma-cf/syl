use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use base64::Engine;
use daemon::events::{DaemonEvent, EventBus};
use extension_host::{ExtensionManifest, ExtensionProcess};
use extension_registry::ModelKind;
use serde_json::json;
use syl_core::engine_ids::IMAGE_ENGINE_ID;
use syl_core::workspace_paths;

use crate::local_models::{discover_gguf_models, kind_for_path, registry_entries};

const CAPABILITY: &str = "image.generate/v1";

#[derive(Default)]
pub struct SdModelState {
    loaded: Mutex<HashMap<String, Arc<ExtensionProcess>>>,
    event_bus: Mutex<Option<Arc<EventBus>>>,
}

impl SdModelState {
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
}

fn build_sd_extension_manifest(
    model_path: &std::path::Path,
    engine_library_path: &std::path::Path,
    n_threads: i32,
) -> Result<ExtensionManifest, String> {
    let manifest = extension_host::find_extension("stable-diffusion-image").ok_or_else(|| {
        "the stable-diffusion-image extension is not installed under .syl/extensions/".to_string()
    })?;
    extension_host::with_backend_args(
        manifest,
        vec![
            "--library".to_string(),
            engine_library_path.display().to_string(),
            "--model".to_string(),
            model_path.display().to_string(),
            "--n-threads".to_string(),
            n_threads.to_string(),
        ],
    )
}

#[tauri::command]
pub async fn load_image_model(
    name: String,
    state: tauri::State<'_, SdModelState>,
) -> Result<(), String> {
    if state.get_loaded(&name).is_some() {
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

    let engine_library_path = extension_registry::resolve_engine_library_path(
        &workspace_paths::registry_dir(),
        &workspace_paths::engines_dir(),
        IMAGE_ENGINE_ID,
    )
    .map_err(|e| e.to_string())?;

    let n_threads = std::thread::available_parallelism()
        .map(|n| n.get() as i32)
        .unwrap_or(4);
    let manifest = build_sd_extension_manifest(&model_path, &engine_library_path, n_threads)?;

    // The model now loads in its own isolated `image-worker` process — a
    // crash inside stable-diffusion.cpp's FFI boundary takes down only that
    // process, not the host (see `extension_host::ExtensionProcess`).
    let process = ExtensionProcess::spawn(manifest)
        .await
        .map_err(|e| e.to_string())?;

    crate::sync::lock(&state.loaded).insert(name, Arc::new(process));
    Ok(())
}

#[tauri::command]
pub async fn unload_image_model(
    name: String,
    state: tauri::State<'_, SdModelState>,
) -> Result<(), String> {
    let process = crate::sync::lock(&state.loaded)
        .remove(&name)
        .ok_or_else(|| format!("{name} is not loaded"))?;
    process.kill().await;
    Ok(())
}

#[tauri::command]
pub async fn generate_image(
    model: String,
    prompt: String,
    negative_prompt: String,
    state: tauri::State<'_, SdModelState>,
) -> Result<String, String> {
    let process = state
        .get_loaded(&model)
        .ok_or_else(|| format!("image model {model} is not loaded; load it first in Settings"))?;

    let settings = crate::settings::load_settings();
    let (steps, width, height) = (
        settings.image_engine_steps,
        settings.image_engine_width,
        settings.image_engine_height,
    );

    let result = process
        .call(
            CAPABILITY,
            "image/generate",
            json!({
                "prompt": prompt,
                "negativePrompt": negative_prompt,
                "width": width,
                "height": height,
                "steps": steps,
                "seed": -1,
            }),
        )
        .await
        .map_err(|e| e.to_string())?;

    let png_base64 = result
        .get("pngBase64")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "image-worker response missing pngBase64".to_string())?;
    let png = base64::engine::general_purpose::STANDARD
        .decode(png_base64)
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
