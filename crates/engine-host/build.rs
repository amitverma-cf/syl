use std::env;
use std::path::PathBuf;

fn main() {
    let vendor_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/llama-cpp");
    let wrapper = vendor_dir.join("wrapper.h");

    println!("cargo:rerun-if-changed={}", wrapper.display());

    let bindings = bindgen::Builder::default()
        .header(wrapper.to_string_lossy())
        .clang_arg(format!("-I{}", vendor_dir.display()))
        .allowlist_function("llama_.*")
        .allowlist_type("llama_.*")
        .allowlist_var("LLAMA_.*")
        .derive_default(true)
        .layout_tests(false)
        .dynamic_library_name("LlamaCpp")
        .dynamic_link_require_all(false)
        .generate()
        .expect("failed to generate llama.cpp bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("llama_bindings.rs");
    bindings
        .write_to_file(&out_path)
        .expect("failed to write llama.cpp bindings");

    // ggml's backend registry (which discovers/loads ggml-cpu-*.dll etc.) lives in a
    // separate shared library from llama.dll, so it needs its own dynamic-loading binding.
    let ggml_bindings = bindgen::Builder::default()
        .header(wrapper.to_string_lossy())
        .clang_arg(format!("-I{}", vendor_dir.display()))
        .allowlist_function("ggml_backend_load.*")
        .derive_default(true)
        .layout_tests(false)
        .dynamic_library_name("GgmlBackend")
        .dynamic_link_require_all(false)
        .generate()
        .expect("failed to generate ggml backend bindings");

    let ggml_out_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("ggml_bindings.rs");
    ggml_bindings
        .write_to_file(&ggml_out_path)
        .expect("failed to write ggml backend bindings");
}
