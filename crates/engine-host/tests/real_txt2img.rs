use std::path::Path;

use engine_host::stable_diffusion::SdEngine;

#[test]
#[ignore]
fn generates_a_real_png() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let engine_dir = repo_root.join(".syl/engines/stable-diffusion");
    let library_path = engine_dir.join("stable-diffusion.dll");
    let model_path = repo_root.join(".syl/models/stable-diffusion-v1-5-Q4_0.gguf");

    assert!(
        library_path.exists(),
        "expected {} to exist",
        library_path.display()
    );
    assert!(
        model_path.exists(),
        "expected {} to exist",
        model_path.display()
    );

    let mut engine = SdEngine::load(&library_path, &model_path, 4).unwrap();
    let png = engine
        .txt2img(
            "a red apple on a white table",
            "blurry, low quality",
            256,
            256,
            8,
            42,
        )
        .unwrap();

    assert!(
        png.starts_with(&[0x89, b'P', b'N', b'G']),
        "output is not a valid PNG"
    );
    println!("generated {} bytes of PNG", png.len());

    std::fs::write(std::env::temp_dir().join("syl_sd_smoke.png"), &png).unwrap();
}
