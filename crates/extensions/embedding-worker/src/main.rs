//! Backend process for the `onnx-embedding` extension: loads one
//! `OnnxEmbeddingEngine` (unchanged, reused as-is from `crates/native-engines`)
//! and speaks the `embedding.embed/v1` capability over `extension-host`'s
//! stdio protocol.
//!
//! Usage: `embedding-worker --library <path> --model <path> --tokenizer <path>`

use std::io::{self, BufRead, Write};
use std::path::Path;

use extension_host::RpcMessage;
use native_engines::onnx_embedding::OnnxEmbeddingEngine;
use serde_json::json;

fn main() {
    tracing_subscriber::fmt().with_writer(io::stderr).init();

    let args: Vec<String> = std::env::args().collect();
    let library_path =
        arg_value(&args, "--library").unwrap_or_else(|| fail("--library is required"));
    let model_path = arg_value(&args, "--model").unwrap_or_else(|| fail("--model is required"));
    let tokenizer_path =
        arg_value(&args, "--tokenizer").unwrap_or_else(|| fail("--tokenizer is required"));

    let mut engine = match OnnxEmbeddingEngine::load(
        Path::new(&library_path),
        Path::new(&model_path),
        Path::new(&tokenizer_path),
    ) {
        Ok(engine) => engine,
        Err(err) => fail(&format!("failed to load engine: {err}")),
    };

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(message) = RpcMessage::from_line(&line) else {
            tracing::warn!(%line, "received a malformed protocol line");
            continue;
        };
        if !message.is_request() {
            continue;
        }
        let id = message.id.expect("is_request guarantees id is Some");
        let method = message
            .method
            .clone()
            .expect("is_request guarantees method is Some");

        match method.as_str() {
            "initialize" => {
                send(
                    &mut stdout,
                    RpcMessage::response_ok(id, json!({ "provides": ["embedding.embed/v1"] })),
                );
            }
            "embedding/embed" => handle_embed(&mut engine, &mut stdout, id, &message.params),
            other => {
                send(
                    &mut stdout,
                    RpcMessage::response_err(id, format!("unknown method {other:?}")),
                );
            }
        }
    }
}

fn handle_embed(
    engine: &mut OnnxEmbeddingEngine,
    stdout: &mut io::Stdout,
    id: u64,
    params: &serde_json::Value,
) {
    let text = params.get("text").and_then(|v| v.as_str()).unwrap_or("");
    match engine.embed(text) {
        Ok(vector) => send(
            stdout,
            RpcMessage::response_ok(id, json!({ "vector": vector })),
        ),
        Err(err) => send(stdout, RpcMessage::response_err(id, err.to_string())),
    }
}

fn send(stdout: &mut io::Stdout, message: RpcMessage) {
    let Ok(mut line) = message.to_line() else {
        return;
    };
    line.push('\n');
    let _ = stdout.write_all(line.as_bytes());
    let _ = stdout.flush();
}

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn fail(message: &str) -> ! {
    eprintln!("embedding-worker: {message}");
    std::process::exit(1);
}
