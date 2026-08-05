use std::path::Path;

use native_engines::gguf_meta::read_quantization;

#[test]
#[ignore]
fn reads_real_quantization_from_gguf_metadata() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let library_path = repo_root.join(".syl/engines/llama-cpp/llama.dll");
    let gguf_path = repo_root.join(".syl/models/LFM2.5-350M-Q4_K_M.gguf");

    assert!(
        library_path.exists(),
        "expected {} to exist",
        library_path.display()
    );
    assert!(
        gguf_path.exists(),
        "expected {} to exist",
        gguf_path.display()
    );

    let quantization = read_quantization(&library_path, &gguf_path).unwrap();
    println!("quantization: {quantization}");

    assert_eq!(quantization, "Q4_K_M");
}
