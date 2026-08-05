use std::path::{Path, PathBuf};

use crate::{
    download_and_extract_zip, download_to_dir, load_engine_entries, load_model_entries,
    resolve_download_url, resolve_local_path, DownloadSource, ModelEntry, ModelKind,
    PluginRegistryError,
};

#[derive(Debug)]
pub struct ResolvedModel {
    pub model_name: String,
    pub model_path: PathBuf,
    pub engine_id: String,
    pub engine_library_path: PathBuf,
}

pub fn resolve_engine_library_path(
    registry_dir: &Path,
    engines_cache_dir: &Path,
    engine_id: &str,
) -> Result<PathBuf, PluginRegistryError> {
    let engines = load_engine_entries(registry_dir)?;
    let engine_entry = engines
        .into_iter()
        .find(|e| e.id == engine_id)
        .ok_or_else(|| PluginRegistryError::EngineNotFound(engine_id.to_string()))?;

    if !engine_entry.download_url.ends_with(".zip") {
        return resolve_local_path(
            &engine_entry.download_url,
            engines_cache_dir,
            engine_entry.sha256.as_deref(),
        );
    }

    let library_file = engine_entry.library_file.as_deref().ok_or_else(|| {
        PluginRegistryError::InvalidUrl(format!(
            "engine {engine_id} downloads as a zip but has no library_file set"
        ))
    })?;

    match resolve_download_url(&engine_entry.download_url)? {
        DownloadSource::Local(path) => Ok(path),
        DownloadSource::Remote(url) => {
            let extract_dir = engines_cache_dir.join(&engine_entry.id);
            download_and_extract_zip(&url, &extract_dir, engine_entry.sha256.as_deref())?;
            join_contained(&extract_dir, library_file)
        }
    }
}

/// Joins `relative` onto `base`, rejecting anything that would escape `base` — an
/// absolute path (which `Path::join` would otherwise let silently replace the whole
/// path) or a `..` component that walks back out of the extraction directory. This
/// guards `library_file` (and any other archive-relative path taken from a registry
/// entry) against pointing at an arbitrary file outside the directory it was just
/// extracted into.
fn join_contained(base: &Path, relative: &str) -> Result<PathBuf, PluginRegistryError> {
    if !crate::is_safe_relative_component(relative) {
        return Err(PluginRegistryError::InvalidUrl(format!(
            "{relative} must be a '..'-free path relative to the engine's extracted directory"
        )));
    }

    let joined = base.join(relative);

    // Canonicalize both sides and re-check containment as defense in depth against a
    // symlink planted inside the extracted archive that a purely lexical check above
    // wouldn't catch. The extraction directory must exist by this point (it was just
    // populated by the zip extraction, or already existed from a prior run).
    if let (Ok(canonical_base), Ok(canonical_joined)) = (base.canonicalize(), joined.canonicalize())
    {
        if !canonical_joined.starts_with(&canonical_base) {
            return Err(PluginRegistryError::InvalidUrl(format!(
                "{relative} resolves outside the engine's extracted directory"
            )));
        }
    }

    Ok(joined)
}

pub fn resolve_model_entry_files(
    entry: &ModelEntry,
    models_cache_dir: &Path,
) -> Result<PathBuf, PluginRegistryError> {
    if entry.extra_files.is_empty() {
        return resolve_local_path(
            &entry.download_url,
            models_cache_dir,
            entry.sha256.as_deref(),
        );
    }

    let model_dir = models_cache_dir.join(&entry.name);
    let primary_path = match resolve_download_url(&entry.download_url)? {
        DownloadSource::Local(path) => path,
        DownloadSource::Remote(url) => download_to_dir(&url, &model_dir)?,
    };
    for extra_url in &entry.extra_files {
        match resolve_download_url(extra_url)? {
            DownloadSource::Local(_) => {}
            DownloadSource::Remote(url) => {
                download_to_dir(&url, &model_dir)?;
            }
        }
    }
    Ok(primary_path)
}

pub fn resolve_model_for_kind(
    registry_dir: &Path,
    models_cache_dir: &Path,
    engines_cache_dir: &Path,
    kind: ModelKind,
) -> Result<ResolvedModel, PluginRegistryError> {
    let mut models = load_model_entries(registry_dir)?;
    models.sort_by(|a, b| a.name.cmp(&b.name));
    let model_entry = models
        .into_iter()
        .find(|m| m.kind == kind)
        .ok_or(PluginRegistryError::NoModelAvailable(kind))?;

    let engines = load_engine_entries(registry_dir)?;
    let engine_entry = engines
        .into_iter()
        .find(|e| e.id == model_entry.required_engine)
        .ok_or_else(|| PluginRegistryError::NoEngineAvailable {
            model: model_entry.name.clone(),
            engine: model_entry.required_engine.clone(),
        })?;

    let model_path = resolve_local_path(
        &model_entry.download_url,
        models_cache_dir,
        model_entry.sha256.as_deref(),
    )?;
    let engine_library_path = resolve_local_path(
        &engine_entry.download_url,
        engines_cache_dir,
        engine_entry.sha256.as_deref(),
    )?;

    Ok(ResolvedModel {
        model_name: model_entry.name,
        model_path,
        engine_id: engine_entry.id,
        engine_library_path,
    })
}

#[cfg(test)]
mod tests {
    use super::join_contained;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let unique = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "syl-join-contained-{label}-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn accepts_a_plain_relative_path_inside_the_base() {
        let base = temp_dir("ok");
        std::fs::write(base.join("lib.dll"), b"x").unwrap();
        let resolved = join_contained(&base, "lib.dll").unwrap();
        assert_eq!(resolved, base.join("lib.dll"));
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn accepts_a_nested_relative_path_inside_the_base() {
        let base = temp_dir("nested");
        std::fs::create_dir_all(base.join("lib")).unwrap();
        std::fs::write(base.join("lib").join("engine.dll"), b"x").unwrap();
        let resolved = join_contained(&base, "lib/engine.dll").unwrap();
        assert_eq!(resolved, base.join("lib").join("engine.dll"));
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn rejects_a_parent_directory_traversal() {
        let base = temp_dir("traversal");
        let err = join_contained(&base, "../../../../Windows/Temp/evil.dll").unwrap_err();
        assert!(matches!(err, crate::PluginRegistryError::InvalidUrl(_)));
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn rejects_an_absolute_path() {
        let base = temp_dir("absolute");
        #[cfg(windows)]
        let absolute = r"C:\Windows\System32\evil.dll";
        #[cfg(not(windows))]
        let absolute = "/etc/evil.so";
        let err = join_contained(&base, absolute).unwrap_err();
        assert!(matches!(err, crate::PluginRegistryError::InvalidUrl(_)));
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn rejects_a_traversal_hidden_in_the_middle_of_the_path() {
        let base = temp_dir("mid-traversal");
        let err = join_contained(&base, "lib/../../evil.dll").unwrap_err();
        assert!(matches!(err, crate::PluginRegistryError::InvalidUrl(_)));
        std::fs::remove_dir_all(&base).unwrap();
    }
}
