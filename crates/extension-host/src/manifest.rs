use std::path::Path;

/// An installed extension's declaration: what it is, how to spawn its
/// backend process, and what capabilities it provides/requires (see
/// `crate::capability` for the negotiation this drives).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionManifest {
    pub id: String,
    pub version: String,
    pub display_name: String,
    pub backend: ExtensionBackend,
    /// Capability IDs (e.g. `"inference.chat/v1"`) this extension implements
    /// — the host may call these methods on it.
    #[serde(default)]
    pub provides: Vec<String>,
    /// Capability IDs this extension needs the host to expose to it (e.g. a
    /// future extension needing `"workspace.fs/v1"`). Checked against what
    /// the host actually implements before the extension is ever spawned.
    #[serde(default)]
    pub requires: Vec<String>,
    /// UI surfaces this extension contributes — schema-only this pass, see
    /// `Contributions`.
    #[serde(default)]
    pub contributes: Option<Contributions>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionBackend {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

/// UI surfaces an extension can declare, rendered by the *host's* own
/// existing components rather than an extension-owned webview (the
/// extension-ecosystem plan's "contribution points, not webviews" decision).
/// Not implemented this pass — the reference chat extension contributes no
/// UI — but kept real and typed now so a later pass has an actual contract
/// to render against instead of inventing one from scratch alongside the
/// first extension that needs it.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contributions {
    #[serde(default)]
    pub settings_pane: Option<UiContribution>,
    #[serde(default)]
    pub sidebar_view: Option<UiContribution>,
    #[serde(default)]
    pub status_bar_item: Option<UiContribution>,
    #[serde(default)]
    pub commands: Vec<UiContribution>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiContribution {
    pub id: String,
    pub title: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("failed to read extension manifest {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse extension manifest {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: serde_json::Error,
    },
}

pub fn load_manifest(path: &Path) -> Result<ExtensionManifest, ManifestError> {
    let contents = std::fs::read_to_string(path).map_err(|source| ManifestError::Io {
        path: path.display().to_string(),
        source,
    })?;
    serde_json::from_str(&contents).map_err(|source| ManifestError::Parse {
        path: path.display().to_string(),
        source,
    })
}
