mod dll;
pub mod gguf_meta;
pub mod llama;
mod mel;
pub mod onnx_asr;
pub mod onnx_embedding;
pub mod onnx_tts;
pub mod plugin;
pub mod stable_diffusion;
pub mod tool_loop;
pub mod watchdog;

#[allow(
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    dead_code,
    clippy::all
)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/llama_bindings.rs"));
}

#[allow(
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    dead_code,
    clippy::all
)]
mod ggml_bindings {
    include!(concat!(env!("OUT_DIR"), "/ggml_bindings.rs"));
}

#[allow(
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    dead_code,
    clippy::all
)]
mod sd_bindings {
    include!(concat!(env!("OUT_DIR"), "/sd_bindings.rs"));
}
