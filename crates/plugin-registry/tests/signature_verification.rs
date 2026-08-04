//! Proves the registry-signing path (`fetch_registry_signatures` +
//! `apply_remote_registry`'s `RegistrySignatures` check) is actually reachable and
//! correct end to end against a real local HTTP server — not just the isolated
//! `verify_registry_signature` unit tests in `signing.rs`. Mirrors
//! `checksum_verification.rs`'s real-local-HTTP-server convention.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::JoinHandle;

use ed25519_dalek::{Signer, SigningKey};
use plugin_registry::{
    apply_remote_registry, fetch_registry_signatures, fetch_remote_registry, PluginRegistryError,
    RegistrySignatures,
};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_dir(label: &str) -> std::path::PathBuf {
    let unique = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "syl-signature-test-{label}-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn test_keypair() -> (SigningKey, String) {
    let signing_key = SigningKey::from_bytes(&[3u8; 32]);
    let public_key_hex = hex::encode(signing_key.verifying_key().to_bytes());
    (signing_key, public_key_hex)
}

/// Spawns a background thread serving `routes` (path -> body) over as many
/// sequential connections as `routes` has entries, then shuts down.
fn serve_routes(routes: HashMap<&'static str, Vec<u8>>) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let expected_requests = routes.len();
    let handle = std::thread::spawn(move || {
        for _ in 0..expected_requests {
            let Ok((stream, _)) = listener.accept() else {
                return;
            };
            handle_one(stream, &routes);
        }
    });
    (format!("http://127.0.0.1:{port}"), handle)
}

fn handle_one(mut stream: TcpStream, routes: &HashMap<&'static str, Vec<u8>>) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut request_line = String::new();
    reader.read_line(&mut request_line).unwrap();
    // Drain remaining headers up to the blank line.
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
            break;
        }
    }
    let path = request_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .to_string();

    match routes.get(path.as_str()) {
        Some(body) => {
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(body);
        }
        None => {
            let _ = stream.write_all(
                b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
        }
    }
    let _ = stream.flush();
}

fn engines_body() -> &'static str {
    r#"[{"id":"llama-cpp","version":"1.0.0","platform":"windows-x64","download_url":"https://github.com/example/engine.zip","sha256":null,"library_file":"engine.dll"}]"#
}

fn models_body() -> &'static str {
    r#"[]"#
}

#[test]
fn a_correctly_signed_registry_pair_is_accepted() {
    let (signing_key, public_key_hex) = test_keypair();
    let engines_sig = hex::encode(signing_key.sign(engines_body().as_bytes()).to_bytes());
    let models_sig = hex::encode(signing_key.sign(models_body().as_bytes()).to_bytes());

    let routes = HashMap::from([
        ("/engines.json", engines_body().as_bytes().to_vec()),
        ("/models.json", models_body().as_bytes().to_vec()),
        ("/engines.json.sig", engines_sig.into_bytes()),
        ("/models.json.sig", models_sig.into_bytes()),
    ]);
    let (base_url, handle) = serve_routes(routes);

    let (engines_json, models_json) = fetch_remote_registry(&base_url).unwrap();
    let (engines_sig, models_sig) = fetch_registry_signatures(&base_url).unwrap();

    let registry_dir = temp_dir("signed-ok");
    apply_remote_registry(
        &registry_dir,
        &engines_json,
        &models_json,
        &["github.com".to_string()],
        Some(RegistrySignatures {
            public_key_hex: &public_key_hex,
            engines_signature_hex: &engines_sig,
            models_signature_hex: &models_sig,
        }),
    )
    .unwrap();

    assert!(registry_dir.join("engines.json").exists());
    handle.join().unwrap();
    std::fs::remove_dir_all(&registry_dir).unwrap();
}

#[test]
fn a_tampered_registry_body_fails_signature_verification() {
    let (signing_key, public_key_hex) = test_keypair();
    // Sign the real body, but serve different (tampered) bytes for engines.json.
    let engines_sig = hex::encode(signing_key.sign(engines_body().as_bytes()).to_bytes());
    let models_sig = hex::encode(signing_key.sign(models_body().as_bytes()).to_bytes());
    let tampered_engines = r#"[{"id":"malicious-engine","displayName":"x","download_url":"https://github.com/evil/engine.zip","sha256":null,"library_file":"engine.dll"}]"#;

    let routes = HashMap::from([
        ("/engines.json", tampered_engines.as_bytes().to_vec()),
        ("/models.json", models_body().as_bytes().to_vec()),
        ("/engines.json.sig", engines_sig.into_bytes()),
        ("/models.json.sig", models_sig.into_bytes()),
    ]);
    let (base_url, handle) = serve_routes(routes);

    let (engines_json, models_json) = fetch_remote_registry(&base_url).unwrap();
    let (engines_sig, models_sig) = fetch_registry_signatures(&base_url).unwrap();

    let registry_dir = temp_dir("signed-tampered");
    let err = apply_remote_registry(
        &registry_dir,
        &engines_json,
        &models_json,
        &["github.com".to_string()],
        Some(RegistrySignatures {
            public_key_hex: &public_key_hex,
            engines_signature_hex: &engines_sig,
            models_signature_hex: &models_sig,
        }),
    )
    .unwrap_err();

    assert!(matches!(err, PluginRegistryError::InvalidSignature { .. }));
    assert!(!registry_dir.join("engines.json").exists());

    handle.join().unwrap();
    std::fs::remove_dir_all(&registry_dir).unwrap();
}

#[test]
fn a_signature_from_the_wrong_key_is_rejected() {
    let (signing_key, _) = test_keypair();
    let (_, wrong_public_key_hex) = {
        let other = SigningKey::from_bytes(&[9u8; 32]);
        (other.clone(), hex::encode(other.verifying_key().to_bytes()))
    };
    let engines_sig = hex::encode(signing_key.sign(engines_body().as_bytes()).to_bytes());
    let models_sig = hex::encode(signing_key.sign(models_body().as_bytes()).to_bytes());

    let routes = HashMap::from([
        ("/engines.json", engines_body().as_bytes().to_vec()),
        ("/models.json", models_body().as_bytes().to_vec()),
        ("/engines.json.sig", engines_sig.into_bytes()),
        ("/models.json.sig", models_sig.into_bytes()),
    ]);
    let (base_url, handle) = serve_routes(routes);

    let (engines_json, models_json) = fetch_remote_registry(&base_url).unwrap();
    let (engines_sig, models_sig) = fetch_registry_signatures(&base_url).unwrap();

    let registry_dir = temp_dir("signed-wrong-key");
    let err = apply_remote_registry(
        &registry_dir,
        &engines_json,
        &models_json,
        &["github.com".to_string()],
        Some(RegistrySignatures {
            public_key_hex: &wrong_public_key_hex,
            engines_signature_hex: &engines_sig,
            models_signature_hex: &models_sig,
        }),
    )
    .unwrap_err();

    assert!(matches!(err, PluginRegistryError::InvalidSignature { .. }));
    handle.join().unwrap();
    std::fs::remove_dir_all(&registry_dir).unwrap();
}
