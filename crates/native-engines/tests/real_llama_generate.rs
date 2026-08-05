use std::path::Path;

use extension_registry::ModelKind;
use native_engines::llama::LlamaEngine;

#[test]
#[ignore]
fn generates_text_with_real_engine() {
    let registry_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.syl/registry");
    let cache_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.syl");

    let resolved = extension_registry::resolve_model_for_kind(
        &registry_dir,
        &cache_dir.join("models"),
        &cache_dir.join("engines"),
        ModelKind::Chat,
    )
    .expect(".syl workspace has no chat model registered; run the app once to seed it");

    let mut engine = LlamaEngine::load(
        &resolved.engine_library_path,
        &resolved.model_path,
        2048,
        false,
    )
    .unwrap();

    let output = engine
        .generate("The capital of France is", 32, |_piece| {})
        .unwrap();

    assert!(!output.trim().is_empty());
}
