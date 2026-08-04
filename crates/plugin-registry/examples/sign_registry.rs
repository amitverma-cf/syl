//! Standalone signing tool for the registry manifest (`engines.json`/
//! `models.json`) — run at publish time, never shipped as part of the app,
//! which only ever verifies (see `plugin_registry::verify_registry_signature`).
//!
//! Usage:
//!   `generate-key`
//!       Prints a new random Ed25519 keypair (hex secret + public key). Keep
//!       the secret key offline; publish the public key into
//!       `config/app.json`'s `registryManifestPublicKey`.
//!   `sign <secret-key-hex> <file>`
//!       Prints the hex-encoded detached signature of `file`'s bytes.

use std::path::PathBuf;

use ed25519_dalek::{Signer, SigningKey};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("generate-key") => generate_key(),
        Some("sign") => {
            let secret_key_hex = args.get(2).unwrap_or_else(|| usage_and_exit());
            let file = args.get(3).unwrap_or_else(|| usage_and_exit());
            sign(secret_key_hex, PathBuf::from(file));
        }
        _ => {
            usage_and_exit();
        }
    }
}

fn usage_and_exit() -> ! {
    eprintln!("usage:");
    eprintln!("  sign_registry generate-key");
    eprintln!("  sign_registry sign <secret-key-hex> <file>");
    std::process::exit(1);
}

fn generate_key() {
    let mut secret_bytes = [0u8; 32];
    getrandom::fill(&mut secret_bytes).expect("failed to read system randomness");
    let signing_key = SigningKey::from_bytes(&secret_bytes);
    println!("secret key (keep offline): {}", hex::encode(secret_bytes));
    println!(
        "public key (set as registryManifestPublicKey): {}",
        hex::encode(signing_key.verifying_key().to_bytes())
    );
}

fn sign(secret_key_hex: &str, file: PathBuf) {
    let secret_bytes: [u8; 32] = hex::decode(secret_key_hex)
        .expect("secret key must be valid hex")
        .try_into()
        .expect("secret key must be exactly 32 bytes");
    let signing_key = SigningKey::from_bytes(&secret_bytes);

    let data = std::fs::read(&file).unwrap_or_else(|e| {
        eprintln!("failed to read {}: {e}", file.display());
        std::process::exit(1);
    });
    let signature = signing_key.sign(&data);
    println!("{}", hex::encode(signature.to_bytes()));
}
