//! Backend process for the `stable-diffusion-image` extension: loads one
//! `SdEngine` (unchanged, reused as-is from `crates/engine-host`) and speaks
//! the `image.generate/v1` capability over `extension-host`'s stdio
//! protocol. One-shot, non-streaming — mirrors `engine-worker`'s
//! synchronous, single-request-at-a-time design.
//!
//! Usage: `sd-worker --library <path> --model <path> [--n-threads <i32>]`

use std::io::{self, BufRead, Write};
use std::path::Path;

use base64::Engine;
use engine_host::stable_diffusion::SdEngine;
use extension_host::RpcMessage;
use serde_json::json;

fn main() {
    tracing_subscriber::fmt().with_writer(io::stderr).init();

    let args: Vec<String> = std::env::args().collect();
    let library_path =
        arg_value(&args, "--library").unwrap_or_else(|| fail("--library is required"));
    let model_path = arg_value(&args, "--model").unwrap_or_else(|| fail("--model is required"));
    let n_threads: i32 = arg_value(&args, "--n-threads")
        .and_then(|s| s.parse().ok())
        .unwrap_or(4);

    let mut engine =
        match SdEngine::load(Path::new(&library_path), Path::new(&model_path), n_threads) {
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
                    RpcMessage::response_ok(id, json!({ "provides": ["image.generate/v1"] })),
                );
            }
            "image/generate" => handle_generate(&mut engine, &mut stdout, id, &message.params),
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
    engine: &mut SdEngine,
    stdout: &mut io::Stdout,
    id: u64,
    params: &serde_json::Value,
) {
    let prompt = params.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
    let negative_prompt = params
        .get("negativePrompt")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let width = params.get("width").and_then(|v| v.as_i64()).unwrap_or(512) as i32;
    let height = params.get("height").and_then(|v| v.as_i64()).unwrap_or(512) as i32;
    let steps = params.get("steps").and_then(|v| v.as_i64()).unwrap_or(20) as i32;
    let seed = params.get("seed").and_then(|v| v.as_i64()).unwrap_or(-1);

    match engine.txt2img(prompt, negative_prompt, width, height, steps, seed) {
        Ok(png_bytes) => {
            let png_base64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
            send(
                stdout,
                RpcMessage::response_ok(id, json!({ "pngBase64": png_base64 })),
            );
        }
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
    eprintln!("sd-worker: {message}");
    std::process::exit(1);
}
