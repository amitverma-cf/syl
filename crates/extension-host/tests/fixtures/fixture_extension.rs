//! A real, minimal extension backend used only by extension-host's own
//! tests to exercise `ExtensionProcess` end to end without needing a real
//! GGUF model — mirrors this session's `mcp_http_fixture_server.rs`
//! precedent (a real, from-scratch protocol implementation spawned
//! in-test, not a mock).
//!
//! Speaks `inference.chat/v1`: `inference/generate` splits the prompt on
//! whitespace and streams each word back as an `inference/piece`
//! notification before responding with the joined text; `inference/countTokens`
//! returns the whitespace word count. A few prompts are magic sentinels for
//! deterministically exercising failure modes:
//! - `"CRASH"` exits the process immediately without responding.
//! - `"HANG"` silently drops the request — no response, no notifications —
//!   so a timeout test doesn't need a slow/flaky real delay to prove a hang
//!   is detected; the fixture just never answers.
//! - `"GARBAGE_MID_STREAM"` streams one real piece, then writes a
//!   deliberately non-JSON line to stdout before continuing normally —
//!   proving the host's malformed-line handling doesn't corrupt or hang the
//!   in-flight request.

use extension_host::RpcMessage;
use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let mut lines = BufReader::new(stdin).lines();
    let mut stdout = tokio::io::stdout();

    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(message) = RpcMessage::from_line(&line) else {
            continue;
        };
        if !message.is_request() {
            continue;
        }
        let id = message.id.unwrap();
        let method = message.method.clone().unwrap();

        match method.as_str() {
            "initialize" => {
                send(
                    &mut stdout,
                    RpcMessage::response_ok(id, json!({ "provides": ["inference.chat/v1"] })),
                )
                .await;
            }
            "inference/generate" => {
                let prompt = message
                    .params
                    .get("prompt")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                if prompt == "CRASH" {
                    std::process::exit(1);
                }
                if prompt == "HANG" {
                    continue;
                }
                if prompt == "GARBAGE_MID_STREAM" {
                    send(
                        &mut stdout,
                        RpcMessage::notification(
                            "inference/piece",
                            json!({ "requestId": id, "text": "before" }),
                        ),
                    )
                    .await;
                    let _ = stdout.write_all(b"not json at all\n").await;
                    let _ = stdout.flush().await;
                    send(
                        &mut stdout,
                        RpcMessage::response_ok(id, json!({ "text": "before after" })),
                    )
                    .await;
                    continue;
                }
                let words: Vec<&str> = prompt.split_whitespace().collect();
                for word in &words {
                    send(
                        &mut stdout,
                        RpcMessage::notification(
                            "inference/piece",
                            json!({ "requestId": id, "text": word }),
                        ),
                    )
                    .await;
                }
                send(
                    &mut stdout,
                    RpcMessage::response_ok(id, json!({ "text": words.join(" ") })),
                )
                .await;
            }
            "inference/countTokens" => {
                let text = message
                    .params
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let count = text.split_whitespace().count();
                send(
                    &mut stdout,
                    RpcMessage::response_ok(id, json!({ "count": count })),
                )
                .await;
            }
            other => {
                send(
                    &mut stdout,
                    RpcMessage::response_err(id, format!("unknown method {other:?}")),
                )
                .await;
            }
        }
    }
}

async fn send(stdout: &mut tokio::io::Stdout, message: RpcMessage) {
    let mut line = message.to_line().unwrap();
    line.push('\n');
    let _ = stdout.write_all(line.as_bytes()).await;
    let _ = stdout.flush().await;
}
