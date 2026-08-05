use std::io::Write;

use extension_registry::resolve_engine_library_path;

#[test]
#[ignore]
fn downloads_and_extracts_a_real_zip_engine() {
    let scratch = std::env::temp_dir().join("syl-zip-engine-test");
    let registry_dir = scratch.join("registry");
    let engines_dir = scratch.join("engines");
    std::fs::create_dir_all(&registry_dir).unwrap();
    let _ = std::fs::remove_dir_all(&engines_dir);

    let engines_json = r#"[
      {
        "id": "onnxruntime-zip-test",
        "version": "1.20.1",
        "platform": "windows-x64",
        "download_url": "https://github.com/microsoft/onnxruntime/releases/download/v1.20.1/onnxruntime-win-x64-1.20.1.zip",
        "sha256": null,
        "library_file": "onnxruntime-win-x64-1.20.1/lib/onnxruntime.dll"
      }
    ]"#;
    let mut file = std::fs::File::create(registry_dir.join("engines.json")).unwrap();
    file.write_all(engines_json.as_bytes()).unwrap();
    drop(file);
    std::fs::write(registry_dir.join("models.json"), "[]").unwrap();

    let resolved =
        resolve_engine_library_path(&registry_dir, &engines_dir, "onnxruntime-zip-test").unwrap();

    assert!(
        resolved.exists(),
        "expected {} to exist after extraction",
        resolved.display()
    );
    assert!(resolved.file_name().unwrap() == "onnxruntime.dll");
}
