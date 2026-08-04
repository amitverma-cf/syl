use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use tauri::Emitter;
use tokio::sync::oneshot;
use tool::{PermissionPrompter, PromptResponse};

/// How long a permission prompt waits for the user before auto-denying. Without this, a
/// request the user never sees (app minimized, notification missed, window closed
/// mid-turn) blocks the generation turn forever — there was previously no bound at all.
const PERMISSION_PROMPT_TIMEOUT: Duration = Duration::from_secs(300);

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

        match wait_for_response(receiver, PERMISSION_PROMPT_TIMEOUT).await {
            Ok(response) => response,
            Err(TimedOut) => {
                self.pending
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .remove(&request_id);
                tracing::warn!(
                    request_id,
                    tool_name,
                    timeout_secs = PERMISSION_PROMPT_TIMEOUT.as_secs(),
                    "permission prompt timed out with no answer; denying"
                );
                let _ = self.app.emit("tool-permission-timeout", request_id);
                PromptResponse::Deny
            }
        }
    }
}

struct TimedOut;

/// The timeout mechanics, factored out from `TauriPermissionPrompter::ask` so they're
/// unit-testable without a real `tauri::AppHandle` (which `ask` otherwise needs, to emit
/// events — not available to construct in a plain unit test).
async fn wait_for_response(
    receiver: oneshot::Receiver<PromptResponse>,
    timeout: Duration,
) -> Result<PromptResponse, TimedOut> {
    match tokio::time::timeout(timeout, receiver).await {
        Ok(result) => Ok(result.unwrap_or(PromptResponse::Deny)),
        Err(_elapsed) => Err(TimedOut),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn returns_the_real_response_when_it_arrives_before_the_timeout() {
        let (sender, receiver) = oneshot::channel();
        sender.send(PromptResponse::AllowOnce).unwrap();

        let result = wait_for_response(receiver, Duration::from_secs(300)).await;
        assert!(matches!(result, Ok(PromptResponse::AllowOnce)));
    }

    #[tokio::test(start_paused = true)]
    async fn times_out_and_reports_timed_out_when_nothing_ever_answers() {
        let (_sender, receiver) = oneshot::channel::<PromptResponse>();

        let wait = tokio::spawn(wait_for_response(receiver, Duration::from_secs(300)));
        tokio::time::advance(Duration::from_secs(301)).await;

        let result = wait.await.unwrap();
        assert!(matches!(result, Err(TimedOut)));
    }

    #[tokio::test(start_paused = true)]
    async fn does_not_time_out_a_moment_before_the_deadline() {
        let (sender, receiver) = oneshot::channel();
        let wait = tokio::spawn(wait_for_response(receiver, Duration::from_secs(300)));

        tokio::time::advance(Duration::from_secs(299)).await;
        sender.send(PromptResponse::Deny).unwrap();

        let result = wait.await.unwrap();
        assert!(matches!(result, Ok(PromptResponse::Deny)));
    }
}
