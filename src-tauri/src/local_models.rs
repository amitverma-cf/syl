use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use core_types::app_config::app_config;
use core_types::workspace_paths;
use daemon::events::{DaemonEvent, EventBus};
use extension_host::{ExtensionManifest, ExtensionProcess};
use plugin_registry::{resolve_local_path, ModelEntry, ModelKind};

#[derive(Default)]
pub struct LocalModelState {
    loaded: Mutex<HashMap<String, Arc<ExtensionProcess>>>,
    // Set once during app setup (`lib.rs`), after both this state and
    // `DaemonState` are managed — lets a crash discovered here (in a plain
    // `tauri::State` method with no `AppHandle` of its own) still reach the
    // frontend as a real toast instead of only a `tracing::warn!` line.
    event_bus: Mutex<Option<Arc<EventBus>>>,
}

impl LocalModelState {
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

    /// Returns the loaded process for `name` — evicting it first if it has
    /// crashed, so a dead process is never handed back as if it were still
    /// usable. A crashed model now correctly, immediately shows as "not
    /// loaded" instead of staying stuck reporting `loaded: true` forever.
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

    /// Names of every currently loaded (alive) model, pruning any crashed
    /// entries as a side effect — used by `list_local_models` so its
    /// reported `loaded` flags are never stale.
    fn loaded_names(&self) -> std::collections::HashSet<String> {
        let mut loaded = crate::sync::lock(&self.loaded);
        let crashed = prune_dead(&mut loaded);
        let result = loaded.keys().cloned().collect();
        drop(loaded);
        for name in crashed {
            self.publish_crashed(&name);
        }
        result
    }
}

/// Removes every dead entry from `loaded` and returns the names that were
/// removed, so callers holding this state's lock can publish a crash event
/// for each one after releasing it.
fn prune_dead(loaded: &mut HashMap<String, Arc<ExtensionProcess>>) -> Vec<String> {
    let dead: Vec<String> = loaded
        .iter()
        .filter(|(_, process)| !process.is_alive())
        .map(|(name, _)| name.clone())
        .collect();
    loaded.retain(|_, process| process.is_alive());
    dead
}

/// Resolves an extension worker binary's path next to the running app's own
/// executable — works in dev (`cargo build` puts every worker binary in the
/// same `target/debug`/`target/release` directory) and, once packaged, for a
/// Tauri sidecar (`bundle.externalBin`), which is likewise placed alongside
/// the main executable. See the extension-ecosystem plan's packaging note.
pub(crate) fn resolve_worker_binary_path(binary_name: &str) -> Result<PathBuf, String> {
    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = current_exe
        .parent()
        .ok_or_else(|| "current executable has no parent directory".to_string())?;
    Ok(dir.join(format!("{binary_name}{}", std::env::consts::EXE_SUFFIX)))
}

/// The `llama-cpp-chat` extension's own manifest declares `provides`/
/// `requires` and its backend command (the `engine-worker` binary), but not
/// which `.gguf` model to load — that's chosen per-load by the user, not a
/// property of the extension itself. This builds a real per-load manifest
/// from the installed extension's declared backend, appending the concrete
/// `--library`/`--model`/`--n-ctx` args for this specific load.
pub(crate) fn build_chat_extension_manifest(
    model_path: &Path,
    engine_library_path: &Path,
    n_ctx: u32,
) -> Result<ExtensionManifest, String> {
    let manifest = extension_host::find_extension("llama-cpp-chat").ok_or_else(|| {
        "the llama-cpp-chat extension is not installed under .syl/extensions/".to_string()
    })?;
    extension_host::with_backend_args(
        manifest,
        vec![
            "--library".to_string(),
            engine_library_path.display().to_string(),
            "--model".to_string(),
            model_path.display().to_string(),
            "--n-ctx".to_string(),
            n_ctx.to_string(),
        ],
    )
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
        let resolved = resolve_local_path(
            &entry.download_url,
            &workspace_paths::models_dir(),
            entry.sha256.as_deref(),
        )
        .ok()?;
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
    let loaded = state.loaded_names();
    let entries = registry_entries();
    let gguf_models = discover_gguf_models()
        .into_iter()
        .map(|(name, path, size_bytes)| LocalModelInfo {
            loaded: loaded.contains(&name),
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
pub async fn load_local_model(
    name: String,
    state: tauri::State<'_, LocalModelState>,
) -> Result<(), String> {
    if state.get_loaded(&name).is_some() {
        return Ok(());
    }

    let max_concurrent = crate::settings::load_settings().max_concurrent_local_models;
    let currently_loaded = state.loaded_names().len() as u32;
    if currently_loaded >= max_concurrent {
        return Err(format!(
            "already at the configured limit of {max_concurrent} concurrently loaded local \
             model(s); unload one first or raise the limit in Settings"
        ));
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

    let manifest = build_chat_extension_manifest(
        &model_path,
        &engine_library_path,
        local_engine_config.context_size,
    )?;

    // The model now loads in its own isolated `engine-worker` process — if
    // llama.cpp segfaults or panics across its FFI boundary, only that
    // process dies; the host observes a clean crash error instead of going
    // down with it (see `extension_host::ExtensionProcess`).
    let process = ExtensionProcess::spawn(manifest)
        .await
        .map_err(|e| e.to_string())?;

    crate::sync::lock(&state.loaded).insert(name, Arc::new(process));
    Ok(())
}

/// Real token count for `text` against a specific loaded local model's own
/// tokenizer — not an approximation. Errors if that model isn't loaded.
#[tauri::command]
pub async fn count_local_tokens(
    name: String,
    text: String,
    state: tauri::State<'_, LocalModelState>,
) -> Result<usize, String> {
    let process = state
        .get_loaded(&name)
        .ok_or_else(|| format!("{name} is not loaded"))?;
    process.count_tokens(&text).await.map_err(|e| e.to_string())
}

/// The context window (in tokens) the local chat engine is configured with.
#[tauri::command]
pub fn local_context_size() -> i32 {
    app_config().local_engine.context_size as i32
}

#[tauri::command]
pub async fn unload_local_model(
    name: String,
    state: tauri::State<'_, LocalModelState>,
) -> Result<(), String> {
    let process = crate::sync::lock(&state.loaded)
        .remove(&name)
        .ok_or_else(|| format!("{name} is not loaded"))?;
    process.kill().await;
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_model_file(label: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "syl-local-models-test-{label}-{}-{unique}.gguf",
            std::process::id()
        ));
        std::fs::write(&path, b"fake gguf contents").unwrap();
        path
    }

    fn file_url(path: &Path) -> String {
        format!("file://{}", path.to_string_lossy().replace('\\', "/"))
    }

    fn entry(name: &str, kind: ModelKind, download_url: String) -> ModelEntry {
        ModelEntry {
            name: name.to_string(),
            kind,
            size_bytes: 0,
            quantization: "Q4_K_M".to_string(),
            required_engine: "llama-cpp".to_string(),
            download_url,
            sha256: None,
            extra_files: Vec::new(),
        }
    }

    #[test]
    fn kind_for_path_finds_the_entry_whose_resolved_download_matches() {
        let chat_file = temp_model_file("chat");
        let embed_file = temp_model_file("embed");
        let entries = vec![
            entry("chat-model", ModelKind::Chat, file_url(&chat_file)),
            entry("embed-model", ModelKind::Embedding, file_url(&embed_file)),
        ];

        assert_eq!(kind_for_path(&entries, &chat_file), Some(ModelKind::Chat));
        assert_eq!(
            kind_for_path(&entries, &embed_file),
            Some(ModelKind::Embedding)
        );

        std::fs::remove_file(&chat_file).ok();
        std::fs::remove_file(&embed_file).ok();
    }

    #[test]
    fn kind_for_path_returns_none_for_a_path_not_in_the_registry() {
        let chat_file = temp_model_file("unregistered-chat");
        let entries = vec![entry("chat-model", ModelKind::Chat, file_url(&chat_file))];

        let unrelated = std::env::temp_dir().join("some-other-file.gguf");
        assert_eq!(kind_for_path(&entries, &unrelated), None);

        std::fs::remove_file(&chat_file).ok();
    }

    #[test]
    fn kind_for_path_returns_none_for_an_empty_registry() {
        let path = std::env::temp_dir().join("anything.gguf");
        assert_eq!(kind_for_path(&[], &path), None);
    }

    /// A real, non-mocked proof of the A1 fix: a genuinely spawned
    /// `engine-worker` process that's then killed must not linger in
    /// `LocalModelState` as if it were still usable. Needs a real local
    /// `.syl` workspace with a chat model (same requirement as
    /// `engine-worker`'s own `real_engine_worker.rs` tests) and the
    /// `engine-worker` binary already built (`cargo build -p engine-worker`)
    /// — run manually with
    /// `cargo test -p syl --lib -- --ignored a_crashed_local_model`.
    #[tokio::test]
    #[ignore]
    async fn a_crashed_local_model_is_pruned_instead_of_reporting_loaded_forever() {
        let registry_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.syl/registry");
        let cache_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.syl");
        let resolved = plugin_registry::resolve_model_for_kind(
            &registry_dir,
            &cache_dir.join("models"),
            &cache_dir.join("engines"),
            ModelKind::Chat,
        )
        .expect(".syl workspace has no chat model registered; run the app once to seed it");

        let engine_worker_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/debug/engine-worker.exe");
        assert!(
            engine_worker_path.exists(),
            "run `cargo build -p engine-worker` first"
        );

        let manifest = extension_host::ExtensionManifest {
            id: "llama-cpp-chat".to_string(),
            version: "1.0.0".to_string(),
            display_name: "llama.cpp Chat Engine".to_string(),
            backend: Some(extension_host::ExtensionBackend {
                command: engine_worker_path.display().to_string(),
                args: vec![
                    "--library".to_string(),
                    resolved.engine_library_path.display().to_string(),
                    "--model".to_string(),
                    resolved.model_path.display().to_string(),
                    "--n-ctx".to_string(),
                    "2048".to_string(),
                ],
            }),
            provides: vec!["inference.chat/v1".to_string()],
            requires: Vec::new(),
            contributes: None,
        };

        let process = ExtensionProcess::spawn(manifest).await.unwrap();
        let state = LocalModelState::default();
        crate::sync::lock(&state.loaded).insert("test-model".to_string(), Arc::new(process));

        assert!(state.get_loaded("test-model").is_some());
        assert!(state.loaded_names().contains("test-model"));

        state.get_loaded("test-model").unwrap().kill().await;
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        assert!(state.get_loaded("test-model").is_none());
        assert!(!state.loaded_names().contains("test-model"));
    }
}
