//! Backend process for the `llama-cpp-chat` extension: loads one
//! `LlamaEngine` (unchanged, reused as-is from `crates/engine-host`) and
//! speaks the `inference.chat/v1` capability over `extension-host`'s
//! JSON-RPC-shaped stdio protocol. Deliberately synchronous/blocking (plain
//! `std::io`, no async runtime) — `LlamaEngine`'s own FFI calls are
//! synchronous anyway, and this process only ever handles one request at a
//! time, so there's nothing for an async runtime to buy here.
//!
//! Usage: `engine-worker --library <path> --model <path> [--n-ctx <u32>]`

use std::io::{self, BufRead, Write};
use std::path::Path;

use engine_host::llama::LlamaEngine;
use extension_host::RpcMessage;
use serde_json::json;

fn main() {
    tracing_subscriber::fmt().with_writer(io::stderr).init();

    let args: Vec<String> = std::env::args().collect();
    let library_path =
        arg_value(&args, "--library").unwrap_or_else(|| fail("--library is required"));
    let model_path = arg_value(&args, "--model").unwrap_or_else(|| fail("--model is required"));
    let n_ctx: u32 = arg_value(&args, "--n-ctx")
        .and_then(|s| s.parse().ok())
        .unwrap_or(2048);

    let mut engine = match LlamaEngine::load(
        Path::new(&library_path),
        Path::new(&model_path),
        n_ctx,
        false,
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
                    RpcMessage::response_ok(id, json!({ "provides": ["inference.chat/v1"] })),
                );
            }
            "inference/generate" => handle_generate(&mut engine, &mut stdout, id, &message.params),
            "inference/countTokens" => {
                handle_count_tokens(&engine, &mut stdout, id, &message.params)
            }
            other => {
                send(
                    &mut stdout,
                    RpcMessage::response_err(id, format!("unknown method {other:?}")),
                );
            }
        }
    }
}

fn handle_generate(
    engine: &mut LlamaEngine,
    stdout: &mut io::Stdout,
    id: u64,
    params: &serde_json::Value,
) {
    let prompt = params.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
    let max_tokens = params
        .get("maxTokens")
        .and_then(|v| v.as_i64())
        .unwrap_or(256) as i32;

    // `stdout` is locked per-write below (each `send` call takes its own
    // lock), so pieces streamed from inside the generate callback interleave
    // safely with nothing else writing concurrently in this single-threaded
    // process.
    let mut piece_stdout = io::stdout();
    let result = engine.generate(prompt, max_tokens, |piece| {
        send(
            &mut piece_stdout,
            RpcMessage::notification("inference/piece", json!({ "requestId": id, "text": piece })),
        );
    });

    match result {
        Ok(text) => send(stdout, RpcMessage::response_ok(id, json!({ "text": text }))),
        Err(err) => send(stdout, RpcMessage::response_err(id, err.to_string())),
    }
}

fn handle_count_tokens(
    engine: &LlamaEngine,
    stdout: &mut io::Stdout,
    id: u64,
    params: &serde_json::Value,
) {
    let text = params.get("text").and_then(|v| v.as_str()).unwrap_or("");
    match engine.count_tokens(text) {
        Ok(count) => send(
            stdout,
            RpcMessage::response_ok(id, json!({ "count": count })),
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
    eprintln!("engine-worker: {message}");
    std::process::exit(1);
}
