//! Backend process for the `onnx-asr` extension: loads one `OnnxAsrEngine`
//! (unchanged, reused as-is from `crates/native-engines`) and speaks the
//! `asr.transcribe/v1` capability over `extension-host`'s stdio protocol.
//! Audio in is base64-encoded little-endian f32 PCM (16kHz mono) — the same
//! cost class as chat's existing per-token JSON notifications, no new
//! precedent needed for a large buffer in a JSON field.
//!
//! Usage: `asr-worker --library <path> --encoder <path> --decoder <path> --tokenizer <path>`

use std::io::{self, BufRead, Write};
use std::path::Path;

use base64::Engine;
use extension_host::RpcMessage;
use native_engines::onnx_asr::OnnxAsrEngine;
use serde_json::json;

fn main() {
    tracing_subscriber::fmt().with_writer(io::stderr).init();

    let args: Vec<String> = std::env::args().collect();
    let library_path =
        arg_value(&args, "--library").unwrap_or_else(|| fail("--library is required"));
    let encoder_path =
        arg_value(&args, "--encoder").unwrap_or_else(|| fail("--encoder is required"));
    let decoder_path =
        arg_value(&args, "--decoder").unwrap_or_else(|| fail("--decoder is required"));
    let tokenizer_path =
        arg_value(&args, "--tokenizer").unwrap_or_else(|| fail("--tokenizer is required"));

    let mut engine = match OnnxAsrEngine::load(
        Path::new(&library_path),
        Path::new(&encoder_path),
        Path::new(&decoder_path),
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
                    RpcMessage::response_ok(id, json!({ "provides": ["asr.transcribe/v1"] })),
                );
            }
            "asr/transcribe" => handle_transcribe(&mut engine, &mut stdout, id, &message.params),
            other => {
                send(
                    &mut stdout,
                    RpcMessage::response_err(id, format!("unknown method {other:?}")),
                );
            }
        }
    }
}

fn handle_transcribe(
    engine: &mut OnnxAsrEngine,
    stdout: &mut io::Stdout,
    id: u64,
    params: &serde_json::Value,
) {
    let pcm_base64 = params
        .get("pcmBase64")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let pcm = match base64::engine::general_purpose::STANDARD.decode(pcm_base64) {
        Ok(bytes) => bytes_to_f32(&bytes),
        Err(err) => {
            send(stdout, RpcMessage::response_err(id, err.to_string()));
            return;
        }
    };

    match engine.transcribe(&pcm) {
        Ok(text) => send(stdout, RpcMessage::response_ok(id, json!({ "text": text }))),
        Err(err) => send(stdout, RpcMessage::response_err(id, err.to_string())),
    }
}

fn bytes_to_f32(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
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
    eprintln!("asr-worker: {message}");
    std::process::exit(1);
}
