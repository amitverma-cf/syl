use extension_host::{
    ExtensionBackend, ExtensionManifest, ExtensionProcess, ExtensionProcessError,
};

fn fixture_manifest() -> ExtensionManifest {
    ExtensionManifest {
        id: "fixture".to_string(),
        version: "1.0.0".to_string(),
        display_name: "Fixture Extension".to_string(),
        backend: ExtensionBackend {
            command: env!("CARGO_BIN_EXE_fixture_extension").to_string(),
            args: vec![],
        },
        provides: vec!["inference.chat/v1".to_string()],
        requires: vec![],
        contributes: None,
    }
}

#[tokio::test]
async fn spawning_a_real_extension_completes_the_initialize_handshake() {
    let process = ExtensionProcess::spawn(fixture_manifest()).await.unwrap();
    assert!(process.is_alive());
    assert!(process.provides("inference.chat/v1"));
    process.kill().await;
}

#[tokio::test]
async fn an_extension_with_an_unsupported_requirement_fails_to_spawn() {
    let mut manifest = fixture_manifest();
    manifest.requires = vec!["workspace.fs/v1".to_string()];

    let result = ExtensionProcess::spawn(manifest).await;
    assert!(matches!(
        result,
        Err(ExtensionProcessError::UnsupportedRequirement(_, _))
    ));
}

#[tokio::test]
async fn generate_streams_pieces_and_returns_the_full_text() {
    let process = ExtensionProcess::spawn(fixture_manifest()).await.unwrap();

    let mut pieces = Vec::new();
    let text = process
        .generate("hello there world", 32, |piece| {
            pieces.push(piece.to_string())
        })
        .await
        .unwrap();

    assert_eq!(pieces, vec!["hello", "there", "world"]);
    assert_eq!(text, "hello there world");

    process.kill().await;
}

#[tokio::test]
async fn count_tokens_returns_the_real_word_count() {
    let process = ExtensionProcess::spawn(fixture_manifest()).await.unwrap();
    let count = process.count_tokens("one two three four").await.unwrap();
    assert_eq!(count, 4);
    process.kill().await;
}

#[tokio::test]
async fn calling_a_capability_the_manifest_does_not_provide_is_rejected() {
    let mut manifest = fixture_manifest();
    manifest.provides = vec![];
    let process = ExtensionProcess::spawn(manifest).await.unwrap();

    let err = process.count_tokens("hi").await.unwrap_err();
    assert!(matches!(
        err,
        ExtensionProcessError::CapabilityNotProvided(_)
    ));

    process.kill().await;
}

/// The actual point of the whole extension-ecosystem pass: if the backend
/// process dies mid-request, the host must surface a clean typed error
/// instead of hanging forever or taking the host process down with it.
#[tokio::test]
async fn a_crashing_extension_process_is_detected_cleanly() {
    let process = ExtensionProcess::spawn(fixture_manifest()).await.unwrap();

    let err = process
        .generate("CRASH", 32, |_piece| {})
        .await
        .unwrap_err();
    assert!(matches!(err, ExtensionProcessError::Crashed));

    // Give the reader task a moment to observe the closed stdout pipe.
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert!(!process.is_alive());

    // A subsequent call against the now-dead process must also fail
    // cleanly, not hang or panic.
    let err2 = process.count_tokens("hi").await.unwrap_err();
    assert!(matches!(err2, ExtensionProcessError::Crashed));
}
