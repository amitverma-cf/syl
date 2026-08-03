use std::ffi::c_char;
use std::path::Path;

use crate::bindings::LlamaCpp;
use crate::dll::{load_ggml_backends, path_to_cstring, prioritize_dll_directory};

#[derive(Debug, thiserror::Error)]
pub enum LlamaError {
    #[error("failed to load llama.cpp library: {0}")]
    LibraryLoad(#[source] libloading::Error),
    #[error("failed to load model from {}", .0.display())]
    ModelLoad(std::path::PathBuf),
    #[error("failed to create llama.cpp context")]
    ContextCreate,
    #[error("failed to tokenize prompt")]
    Tokenize,
    #[error("llama_decode failed with code {0}")]
    Decode(i32),
    #[error("no embedding was produced; was the engine loaded with embeddings enabled?")]
    EmbeddingUnavailable,
}

pub struct LlamaEngine {
    lib: LlamaCpp,
    model: *mut crate::bindings::llama_model,
    ctx: *mut crate::bindings::llama_context,
}

unsafe impl Send for LlamaEngine {}

impl LlamaEngine {
    #[tracing::instrument(skip(library_path, model_path), fields(
        library = %library_path.display(),
        model = %model_path.display(),
    ))]
    pub fn load(
        library_path: &Path,
        model_path: &Path,
        n_ctx: u32,
        embeddings: bool,
    ) -> Result<Self, LlamaError> {
        let load_start = std::time::Instant::now();

        if let Some(dir) = library_path.parent() {
            prioritize_dll_directory(dir);
        }

        // SAFETY: `library_path` is expected to point at a llama.cpp build implementing the
        // public llama.h C API this binding was generated from.
        let lib = unsafe { LlamaCpp::new(library_path) }.map_err(LlamaError::LibraryLoad)?;

        // SAFETY: must be called once before any other llama.cpp function.
        unsafe { lib.llama_backend_init() };

        let backend_dir_buf = library_path.parent().unwrap_or_else(|| Path::new("."));
        load_ggml_backends(backend_dir_buf).map_err(LlamaError::LibraryLoad)?;

        let model_path_c = path_to_cstring(model_path);
        let model_params = unsafe { lib.llama_model_default_params() };
        let model = unsafe { lib.llama_model_load_from_file(model_path_c.as_ptr(), model_params) };
        if model.is_null() {
            return Err(LlamaError::ModelLoad(model_path.to_path_buf()));
        }

        let mut ctx_params = unsafe { lib.llama_context_default_params() };
        ctx_params.n_ctx = n_ctx;
        if embeddings {
            ctx_params.embeddings = true;
            ctx_params.pooling_type = crate::bindings::llama_pooling_type_LLAMA_POOLING_TYPE_MEAN;
        }

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

    #[tracing::instrument(skip(self, text), fields(text_len = text.len()))]
    pub fn embed(&mut self, text: &str) -> Result<Vec<f32>, LlamaError> {
        let embed_start = std::time::Instant::now();
        let vocab = unsafe { self.lib.llama_model_get_vocab(self.model) };
        let mut tokens = self.tokenize(vocab, text, true)?;

        let batch = unsafe {
            self.lib
                .llama_batch_get_one(tokens.as_mut_ptr(), tokens.len() as i32)
        };
        let rc = unsafe { self.lib.llama_decode(self.ctx, batch) };
        if rc != 0 {
            tracing::error!(code = rc, "decode step failed");
            return Err(LlamaError::Decode(rc));
        }

        let n_embd = unsafe { self.lib.llama_model_n_embd(self.model) };
        let ptr = unsafe { self.lib.llama_get_embeddings_seq(self.ctx, 0) };
        if ptr.is_null() || n_embd <= 0 {
            return Err(LlamaError::EmbeddingUnavailable);
        }

        // SAFETY: llama.cpp guarantees `ptr` points to `n_embd` valid `f32`s when non-null.
        let embedding = unsafe { std::slice::from_raw_parts(ptr, n_embd as usize) }.to_vec();

        tracing::info!(
            dims = embedding.len(),
            elapsed_ms = embed_start.elapsed().as_millis(),
            "embedding computed"
        );
        Ok(embedding)
    }

    /// Real token count for `text` using this model's own vocabulary — the
    /// same tokenizer `generate`/`embed` use to build their prompts, not an
    /// approximation.
    pub fn count_tokens(&self, text: &str) -> Result<usize, LlamaError> {
        let vocab = unsafe { self.lib.llama_model_get_vocab(self.model) };
        Ok(self.tokenize(vocab, text, true)?.len())
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
