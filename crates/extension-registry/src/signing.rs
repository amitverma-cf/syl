//! Ed25519 signing/verification for the remote registry manifest
//! (`engines.json`/`models.json`). Signing is provenance ("this really came
//! from the repo owner"); the existing sha256 checksum on each entry's
//! `download_url` is integrity ("this wasn't corrupted/tampered after
//! signing") — neither replaces the other.
//!
//! Signing itself happens out-of-band, at publish time, with a small
//! standalone binary (`examples/sign_registry.rs` in this crate) — never
//! shipped as part of the running app, which only ever verifies.

use ed25519_dalek::{Signature, Verifier, VerifyingKey};

use crate::PluginRegistryError;

/// Verifies `data` against a hex-encoded Ed25519 signature and a hex-encoded
/// public key. `file` is only used to name the file in a rejected error.
pub fn verify_registry_signature(
    file: &str,
    data: &[u8],
    signature_hex: &str,
    public_key_hex: &str,
) -> Result<(), PluginRegistryError> {
    let public_key_bytes: [u8; 32] = hex::decode(public_key_hex)
        .ok()
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| PluginRegistryError::InvalidSignature {
            file: file.to_string(),
            reason: "public key is not 32 bytes of valid hex".to_string(),
        })?;
    let verifying_key = VerifyingKey::from_bytes(&public_key_bytes).map_err(|e| {
        PluginRegistryError::InvalidSignature {
            file: file.to_string(),
            reason: format!("invalid public key: {e}"),
        }
    })?;

    let signature_bytes: [u8; 64] = hex::decode(signature_hex)
        .ok()
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| PluginRegistryError::InvalidSignature {
            file: file.to_string(),
            reason: "signature is not 64 bytes of valid hex".to_string(),
        })?;
    let signature = Signature::from_bytes(&signature_bytes);

    verifying_key
        .verify(data, &signature)
        .map_err(|e| PluginRegistryError::InvalidSignature {
            file: file.to_string(),
            reason: e.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn test_keypair() -> (SigningKey, String) {
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let public_key_hex = hex::encode(signing_key.verifying_key().to_bytes());
        (signing_key, public_key_hex)
    }

    #[test]
    fn a_valid_signature_from_the_matching_key_verifies() {
        let (signing_key, public_key_hex) = test_keypair();
        let data = b"[{\"id\":\"llama-cpp\"}]";
        let signature = hex::encode(signing_key.sign(data).to_bytes());

        verify_registry_signature("engines.json", data, &signature, &public_key_hex).unwrap();
    }

    #[test]
    fn a_signature_over_different_data_is_rejected() {
        let (signing_key, public_key_hex) = test_keypair();
        let signature = hex::encode(signing_key.sign(b"original bytes").to_bytes());

        let err = verify_registry_signature(
            "engines.json",
            b"tampered bytes",
            &signature,
            &public_key_hex,
        )
        .unwrap_err();
        assert!(matches!(err, PluginRegistryError::InvalidSignature { .. }));
    }

    #[test]
    fn a_signature_from_a_different_key_is_rejected() {
        let (signing_key, _) = test_keypair();
        let data = b"[{\"id\":\"llama-cpp\"}]";
        let signature = hex::encode(signing_key.sign(data).to_bytes());

        let (_, wrong_public_key_hex) = {
            let other = SigningKey::from_bytes(&[9u8; 32]);
            let hex_key = hex::encode(other.verifying_key().to_bytes());
            (other, hex_key)
        };

        let err =
            verify_registry_signature("engines.json", data, &signature, &wrong_public_key_hex)
                .unwrap_err();
        assert!(matches!(err, PluginRegistryError::InvalidSignature { .. }));
    }

    #[test]
    fn a_malformed_public_key_is_rejected_with_a_clear_reason() {
        let data = b"data";
        let err = verify_registry_signature("engines.json", data, "aa", "not-hex").unwrap_err();
        assert!(matches!(err, PluginRegistryError::InvalidSignature { .. }));
    }

    #[test]
    fn a_malformed_signature_is_rejected_with_a_clear_reason() {
        let (_, public_key_hex) = test_keypair();
        let data = b"data";
        let err = verify_registry_signature("engines.json", data, "not-hex", &public_key_hex)
            .unwrap_err();
        assert!(matches!(err, PluginRegistryError::InvalidSignature { .. }));
    }
}
