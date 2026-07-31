use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use async_trait::async_trait;
use tauri::Emitter;
use tokio::sync::oneshot;
use tool::{PermissionPrompter, PromptResponse};

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PermissionRequestPayload {
    request_id: u64,
    tool: String,
    args: serde_json::Value,
}

pub struct TauriPermissionPrompter {
    app: tauri::AppHandle,
    pending: Mutex<HashMap<u64, oneshot::Sender<PromptResponse>>>,
    next_id: AtomicU64,
}

impl TauriPermissionPrompter {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self {
            app,
            pending: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(0),
        }
    }

    pub fn resolve(&self, request_id: u64, response: PromptResponse) {
        let sender = self
            .pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&request_id);
        if let Some(sender) = sender {
            let _ = sender.send(response);
        }
    }
}

#[async_trait]
impl PermissionPrompter for TauriPermissionPrompter {
    async fn ask(&self, tool_name: &str, args: &serde_json::Value) -> PromptResponse {
        let request_id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (sender, receiver) = oneshot::channel();
        self.pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(request_id, sender);

        let payload = PermissionRequestPayload {
            request_id,
            tool: tool_name.to_string(),
            args: args.clone(),
        };
        if self.app.emit("tool-permission-request", payload).is_err() {
            return PromptResponse::Deny;
        }

        tracing::info!(
            request_id,
            tool_name,
            "waiting for user to answer permission prompt"
        );
        receiver.await.unwrap_or(PromptResponse::Deny)
    }
}
