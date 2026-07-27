//! A minimal, single-sequence text generation wrapper around a dynamically loaded llama.cpp
//! shared library.

use std::ffi::{c_char, CString};
use std::path::Path;

use crate::bindings::LlamaCpp;
use crate::ggml_bindings::GgmlBackend;

/// An error returned while loading or running the llama.cpp engine.
#[derive(Debug, thiserror::Error)]
pub enum LlamaError {
    /// The shared library at the given path could not be loaded.
    #[error("failed to load llama.cpp library: {0}")]
    LibraryLoad(#[source] libloading::Error),
    /// The model file could not be loaded.
    #[error("failed to load model from {}", .0.display())]
    ModelLoad(std::path::PathBuf),
    /// The inference context could not be created.
    #[error("failed to create llama.cpp context")]
    ContextCreate,
    /// The prompt could not be tokenized.
    #[error("failed to tokenize prompt")]
    Tokenize,
    /// A decode step failed.
    #[error("llama_decode failed with code {0}")]
    Decode(i32),
}

/// A loaded llama.cpp model and inference context, ready to generate text for one sequence
/// at a time.
pub struct LlamaEngine {
    lib: LlamaCpp,
    model: *mut crate::bindings::llama_model,
    ctx: *mut crate::bindings::llama_context,
}

// The underlying llama.cpp context is only ever accessed through `&mut self`, so it is safe
// to move a `LlamaEngine` across threads as long as it is not used concurrently.
unsafe impl Send for LlamaEngine {}

impl LlamaEngine {
    /// Loads the llama.cpp shared library at `library_path` and the model at `model_path`,
    /// creating an inference context sized to `n_ctx` tokens.
    ///
    /// # Errors
    /// Returns an error if the library or model fails to load, or the context fails to create.
    #[tracing::instrument(skip(library_path, model_path), fields(
        library = %library_path.display(),
        model = %model_path.display(),
    ))]
    pub fn load(library_path: &Path, model_path: &Path, n_ctx: u32) -> Result<Self, LlamaError> {
        let load_start = std::time::Instant::now();

        // SAFETY: `library_path` is expected to point at a llama.cpp build implementing the
        // public llama.h C API this binding was generated from.
        let lib = unsafe { LlamaCpp::new(library_path) }.map_err(LlamaError::LibraryLoad)?;

        // SAFETY: must be called once before any other llama.cpp function.
        unsafe { lib.llama_backend_init() };

        // llama.cpp's compute backends (CPU, CUDA, ...) are themselves plugins, loaded from
        // the same directory as the main library. The loader for them lives in ggml-base,
        // a separate shared library from llama.dll itself.
        let backend_dir_buf = library_path.parent().unwrap_or_else(|| Path::new("."));
        let backend_dir = path_to_cstring(backend_dir_buf);
        let ggml_base_path = backend_dir_buf.join(ggml_base_library_name());
        let ggml = unsafe { GgmlBackend::new(&ggml_base_path) }.map_err(LlamaError::LibraryLoad)?;
        unsafe { ggml.ggml_backend_load_all_from_path(backend_dir.as_ptr()) };

        let model_path_c = path_to_cstring(model_path);
        let model_params = unsafe { lib.llama_model_default_params() };
        let model = unsafe { lib.llama_model_load_from_file(model_path_c.as_ptr(), model_params) };
        if model.is_null() {
            return Err(LlamaError::ModelLoad(model_path.to_path_buf()));
        }

        let mut ctx_params = unsafe { lib.llama_context_default_params() };
        ctx_params.n_ctx = n_ctx;

        let ctx = unsafe { lib.llama_init_from_model(model, ctx_params) };
        if ctx.is_null() {
            unsafe { lib.llama_model_free(model) };
            return Err(LlamaError::ContextCreate);
        }

        tracing::info!(
            elapsed_ms = load_start.elapsed().as_millis(),
            "engine loaded"
        );
        Ok(Self { lib, model, ctx })
    }

    /// Runs greedy generation for `prompt`, calling `on_token` with each generated piece of
    /// text as it's produced, up to `max_tokens` new tokens or the model's end-of-generation
    /// token, whichever comes first. Returns the full generated text.
    ///
    /// # Errors
    /// Returns an error if the prompt cannot be tokenized or a decode step fails.
    #[tracing::instrument(skip(self, prompt, on_token), fields(prompt_len = prompt.len()))]
    pub fn generate(
        &mut self,
        prompt: &str,
        max_tokens: i32,
        mut on_token: impl FnMut(&str),
    ) -> Result<String, LlamaError> {
        let generate_start = std::time::Instant::now();
        let vocab = unsafe { self.lib.llama_model_get_vocab(self.model) };

        let prompt_tokens = self.tokenize(vocab, prompt, true)?;

        let sampler = unsafe {
            let chain = self
                .lib
                .llama_sampler_chain_init(self.lib.llama_sampler_chain_default_params());
            self.lib
                .llama_sampler_chain_add(chain, self.lib.llama_sampler_init_greedy());
            chain
        };

        let mut output = String::new();
        let mut tokens = prompt_tokens;
        let mut n_generated = 0u32;

        for _ in 0..max_tokens {
            let batch = unsafe {
                self.lib
                    .llama_batch_get_one(tokens.as_mut_ptr(), tokens.len() as i32)
            };
            let rc = unsafe { self.lib.llama_decode(self.ctx, batch) };
            if rc != 0 {
                unsafe { self.lib.llama_sampler_free(sampler) };
                tracing::error!(code = rc, "decode step failed");
                return Err(LlamaError::Decode(rc));
            }

            let next = unsafe { self.lib.llama_sampler_sample(sampler, self.ctx, -1) };
            unsafe { self.lib.llama_sampler_accept(sampler, next) };

            if unsafe { self.lib.llama_vocab_is_eog(vocab, next) } {
                break;
            }

            let piece = self.token_to_piece(vocab, next);
            tracing::trace!(%piece, "generated piece");
            output.push_str(&piece);
            on_token(&piece);
            n_generated += 1;

            tokens = vec![next];
        }

        unsafe { self.lib.llama_sampler_free(sampler) };

        let elapsed = generate_start.elapsed();
        tracing::info!(
            tokens = n_generated,
            elapsed_ms = elapsed.as_millis(),
            tokens_per_sec = n_generated as f64 / elapsed.as_secs_f64(),
            "generation finished"
        );
        Ok(output)
    }

    fn tokenize(
        &self,
        vocab: *const crate::bindings::llama_vocab,
        text: &str,
        add_special: bool,
    ) -> Result<Vec<crate::bindings::llama_token>, LlamaError> {
        let text_bytes = text.as_bytes();
        let mut buf = vec![0i32; text_bytes.len() + 8];
        let n = unsafe {
            self.lib.llama_tokenize(
                vocab,
                text_bytes.as_ptr().cast::<c_char>(),
                text_bytes.len() as i32,
                buf.as_mut_ptr(),
                buf.len() as i32,
                add_special,
                true,
            )
        };
        if n < 0 {
            return Err(LlamaError::Tokenize);
        }
        buf.truncate(n as usize);
        Ok(buf)
    }

    fn token_to_piece(
        &self,
        vocab: *const crate::bindings::llama_vocab,
        token: crate::bindings::llama_token,
    ) -> String {
        let mut buf = [0u8; 128];
        let n = unsafe {
            self.lib.llama_token_to_piece(
                vocab,
                token,
                buf.as_mut_ptr().cast::<c_char>(),
                buf.len() as i32,
                0,
                true,
            )
        };
        if n <= 0 {
            return String::new();
        }
        String::from_utf8_lossy(&buf[..n as usize]).into_owned()
    }
}

impl Drop for LlamaEngine {
    fn drop(&mut self) {
        unsafe {
            self.lib.llama_free(self.ctx);
            self.lib.llama_model_free(self.model);
            self.lib.llama_backend_free();
        }
    }
}

fn path_to_cstring(path: &Path) -> CString {
    CString::new(path.to_string_lossy().as_bytes()).unwrap_or_default()
}

fn ggml_base_library_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "ggml.dll"
    } else if cfg!(target_os = "macos") {
        "libggml.dylib"
    } else {
        "libggml.so"
    }
}
