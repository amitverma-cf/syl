//! Loads native inference engine plugins at runtime and manages the resources they use.

pub mod batching;
pub mod llama;
pub mod plugin;
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
