use std::path::Path;

use native_engines::onnx_embedding::OnnxEmbeddingEngine;

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (norm_a * norm_b)
}

#[test]
#[ignore]
fn embeds_real_text_with_sensible_similarity() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let runtime_library_path = repo_root.join(".syl/engines/onnxruntime/onnxruntime.dll");
    let model_dir = repo_root.join(".syl/models/all-MiniLM-L6-v2");
    let model_path = model_dir.join("model.onnx");
    let tokenizer_path = model_dir.join("tokenizer.json");

    assert!(
        runtime_library_path.exists(),
        "expected {} to exist",
        runtime_library_path.display()
    );
    assert!(
        model_path.exists(),
        "expected {} to exist",
        model_path.display()
    );
    assert!(
        tokenizer_path.exists(),
        "expected {} to exist",
        tokenizer_path.display()
    );

    let mut engine =
        OnnxEmbeddingEngine::load(&runtime_library_path, &model_path, &tokenizer_path).unwrap();

    let cat = engine.embed("a cat sits on the mat").unwrap();
    let kitten = engine.embed("a kitten rests on the rug").unwrap();
    let rocket = engine.embed("a rocket launches into orbit").unwrap();

    assert_eq!(cat.len(), 384);
    assert_eq!(kitten.len(), 384);

    let related = cosine_similarity(&cat, &kitten);
    let unrelated = cosine_similarity(&cat, &rocket);

    println!("related={related} unrelated={unrelated}");
    assert!(
        related > unrelated,
        "expected semantically related sentences to be more similar: related={related}, unrelated={unrelated}"
    );
    assert!(
        related > 0.5,
        "expected related similarity > 0.5, got {related}"
    );
}
