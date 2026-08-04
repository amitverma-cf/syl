use std::path::Path;

use core_types::workspace_paths;

use crate::manifest::{load_manifest, ExtensionManifest};

/// Discovers installed extensions under `.syl/extensions/<id>/manifest.json`.
/// Local sideload only this pass — no download/registry step, matching how
/// `local.models.json` already layers local overrides on top of the remote
/// engine/model registry today. A real hosted extensions catalog (with
/// download/verify, mirroring `plugin_registry::apply_remote_registry`) is
/// future work.
pub fn discover_installed_extensions() -> Vec<ExtensionManifest> {
    discover_in(&workspace_paths::extensions_dir())
}

fn discover_in(dir: &Path) -> Vec<ExtensionManifest> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| {
            let manifest_path = entry.path().join("manifest.json");
            match load_manifest(&manifest_path) {
                Ok(manifest) => Some(manifest),
                Err(err) if manifest_path.exists() => {
                    tracing::warn!(?err, path = %manifest_path.display(), "skipping invalid extension manifest");
                    None
                }
                Err(_) => None,
            }
        })
        .collect()
}

/// Finds one installed extension by id, resolving its `manifest.backend`
/// paths (relative to its own directory) to absolute paths so the caller
/// can spawn it regardless of the current working directory.
pub fn find_extension(id: &str) -> Option<ExtensionManifest> {
    discover_installed_extensions()
        .into_iter()
        .find(|m| m.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::ExtensionBackend;

    fn temp_dir(label: &str) -> std::path::PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "syl-extension-manager-test-{label}-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_manifest(dir: &Path, id: &str) {
        let manifest = ExtensionManifest {
            id: id.to_string(),
            version: "1.0.0".to_string(),
            display_name: id.to_string(),
            backend: ExtensionBackend {
                command: "echo".to_string(),
                args: vec![],
            },
            provides: vec!["inference.chat/v1".to_string()],
            requires: vec![],
            contributes: None,
        };
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join("manifest.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn discovers_every_valid_manifest_in_the_directory() {
        let root = temp_dir("discover");
        write_manifest(&root.join("llama-cpp-chat"), "llama-cpp-chat");
        write_manifest(&root.join("other-ext"), "other-ext");

        let found = discover_in(&root);
        assert_eq!(found.len(), 2);
        assert!(found.iter().any(|m| m.id == "llama-cpp-chat"));
        assert!(found.iter().any(|m| m.id == "other-ext"));

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn skips_a_directory_with_an_invalid_manifest_rather_than_erroring() {
        let root = temp_dir("invalid");
        write_manifest(&root.join("good"), "good");
        std::fs::create_dir_all(root.join("bad")).unwrap();
        std::fs::write(root.join("bad").join("manifest.json"), "{not json").unwrap();

        let found = discover_in(&root);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "good");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_missing_extensions_directory_returns_empty_rather_than_erroring() {
        let missing = temp_dir("missing").join("does-not-exist");
        assert!(discover_in(&missing).is_empty());
    }
}
