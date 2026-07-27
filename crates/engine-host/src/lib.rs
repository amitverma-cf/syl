//! Dynamically loads native engine plugins (llama.cpp, onnxruntime, stable-diffusion, ...)
//! via `libloading`, one dedicated worker thread per engine, plus the resource watchdog
//! and continuous-batching scheduler. See plan Decisions #2, #3, #5, #6, #8.

pub mod batching;
pub mod plugin;
pub mod watchdog;
