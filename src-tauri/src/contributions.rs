/// One extension's UI contribution, flattened for the frontend — same
/// `Vec<SerializableStruct>` shape every other `list_*` command in this
/// codebase uses (`list_local_models`, `list_flows`, etc).
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionContribution {
    pub extension_id: String,
    pub kind: ContributionKind,
    pub id: String,
    pub title: String,
}

#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ContributionKind {
    SettingsPane,
    SidebarView,
    StatusBarItem,
    Command,
}

/// Flattens every installed extension's `contributes` into a single list the
/// frontend renders generically — the mechanism a UI-only extension (like
/// the Flow Editor) or a future backend extension's own settings pane both
/// go through, instead of each contribution type needing bespoke wiring.
#[tauri::command]
pub fn list_contributions() -> Vec<ExtensionContribution> {
    extension_host::discover_installed_extensions()
        .into_iter()
        .flat_map(|manifest| {
            let extension_id = manifest.id.clone();
            let contributes = manifest.contributes;
            let mut out = Vec::new();
            if let Some(contributes) = contributes {
                if let Some(pane) = contributes.settings_pane {
                    out.push(ExtensionContribution {
                        extension_id: extension_id.clone(),
                        kind: ContributionKind::SettingsPane,
                        id: pane.id,
                        title: pane.title,
                    });
                }
                if let Some(view) = contributes.sidebar_view {
                    out.push(ExtensionContribution {
                        extension_id: extension_id.clone(),
                        kind: ContributionKind::SidebarView,
                        id: view.id,
                        title: view.title,
                    });
                }
                if let Some(item) = contributes.status_bar_item {
                    out.push(ExtensionContribution {
                        extension_id: extension_id.clone(),
                        kind: ContributionKind::StatusBarItem,
                        id: item.id,
                        title: item.title,
                    });
                }
                for command in contributes.commands {
                    out.push(ExtensionContribution {
                        extension_id: extension_id.clone(),
                        kind: ContributionKind::Command,
                        id: command.id,
                        title: command.title,
                    });
                }
            }
            out
        })
        .collect()
}
