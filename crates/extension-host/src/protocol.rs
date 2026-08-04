//! The host↔extension wire protocol: newline-delimited JSON-RPC-shaped
//! messages over the extension backend's stdin/stdout. Deliberately *not*
//! built on `rmcp` — `rmcp`'s `Service`/`ServerHandler` traits are fixed to
//! MCP's own method vocabulary (`initialize`, `tools/list`, `tools/call`,
//! …), not a generic arbitrary-method RPC framework, so it can't carry a
//! different capability namespace like `inference.chat/v1`. This is a small,
//! hand-rolled JSON-RPC-2.0-shaped layer instead — request (has `id` and
//! `method`), response (has `id` and `result`/`error`, no `method`), and
//! notification (has `method`, no `id`) — reusing the *process-spawning*
//! pattern already proven in `crates/tool/src/mcp.rs` (`tokio::process::Command`),
//! just not its session machinery.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcMessage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub params: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcErrorPayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcErrorPayload {
    pub message: String,
}

impl RpcMessage {
    pub fn request(id: u64, method: &str, params: Value) -> Self {
        Self {
            id: Some(id),
            method: Some(method.to_string()),
            params,
            result: None,
            error: None,
        }
    }

    pub fn response_ok(id: u64, result: Value) -> Self {
        Self {
            id: Some(id),
            method: None,
            params: Value::Null,
            result: Some(result),
            error: None,
        }
    }

    pub fn response_err(id: u64, message: impl Into<String>) -> Self {
        Self {
            id: Some(id),
            method: None,
            params: Value::Null,
            result: None,
            error: Some(RpcErrorPayload {
                message: message.into(),
            }),
        }
    }

    pub fn notification(method: &str, params: Value) -> Self {
        Self {
            id: None,
            method: Some(method.to_string()),
            params,
            result: None,
            error: None,
        }
    }

    pub fn is_request(&self) -> bool {
        self.id.is_some() && self.method.is_some()
    }

    pub fn is_notification(&self) -> bool {
        self.id.is_none() && self.method.is_some()
    }

    pub fn is_response(&self) -> bool {
        self.id.is_some() && self.method.is_none()
    }

    /// Serializes to one line (no trailing newline — the caller writes the
    /// separator, matching `BufRead::lines()` stripping it on the read side).
    pub fn to_line(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_line(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_round_trips_and_is_classified_correctly() {
        let msg = RpcMessage::request(1, "inference/generate", serde_json::json!({"prompt": "hi"}));
        let line = msg.to_line().unwrap();
        let parsed = RpcMessage::from_line(&line).unwrap();
        assert!(parsed.is_request());
        assert!(!parsed.is_notification());
        assert!(!parsed.is_response());
        assert_eq!(parsed.method.as_deref(), Some("inference/generate"));
    }

    #[test]
    fn a_response_round_trips_and_is_classified_correctly() {
        let msg = RpcMessage::response_ok(1, serde_json::json!({"text": "hello"}));
        let parsed = RpcMessage::from_line(&msg.to_line().unwrap()).unwrap();
        assert!(parsed.is_response());
        assert!(!parsed.is_request());
        assert_eq!(parsed.result.unwrap()["text"], "hello");
    }

    #[test]
    fn a_notification_round_trips_and_is_classified_correctly() {
        let msg = RpcMessage::notification(
            "inference/piece",
            serde_json::json!({"requestId": 1, "text": "he"}),
        );
        let parsed = RpcMessage::from_line(&msg.to_line().unwrap()).unwrap();
        assert!(parsed.is_notification());
        assert!(!parsed.is_request());
        assert!(!parsed.is_response());
    }

    #[test]
    fn an_error_response_carries_the_message() {
        let msg = RpcMessage::response_err(2, "boom");
        let parsed = RpcMessage::from_line(&msg.to_line().unwrap()).unwrap();
        assert_eq!(parsed.error.unwrap().message, "boom");
    }
}
