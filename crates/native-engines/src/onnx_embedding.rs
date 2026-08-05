use std::path::Path;
use std::sync::Once;

use ort::session::Session;
use ort::value::Tensor;
use tokenizers::Tokenizer;

static INIT_RUNTIME: Once = Once::new();

#[derive(Debug, thiserror::Error)]
pub enum OnnxEmbeddingError {
    #[error("failed to load onnxruntime library: {0}")]
    RuntimeInit(String),
    #[error("failed to load tokenizer from {}: {1}", .0.display())]
    TokenizerLoad(std::path::PathBuf, String),
    #[error("failed to load onnx model from {}: {1}", .0.display())]
    ModelLoad(std::path::PathBuf, String),
    #[error("tokenization failed: {0}")]
    Tokenize(String),
    #[error("onnx inference failed: {0}")]
    Inference(String),
    #[error("model produced no embedding output")]
    NoOutput,
}

pub struct OnnxEmbeddingEngine {
    session: Session,
    tokenizer: Tokenizer,
}

impl OnnxEmbeddingEngine {
    pub fn load(
        runtime_library_path: &Path,
        model_path: &Path,
        tokenizer_path: &Path,
    ) -> Result<Self, OnnxEmbeddingError> {
        let mut init_err = None;
        INIT_RUNTIME.call_once(|| {
            match ort::init_from(runtime_library_path.to_string_lossy().to_string()) {
                Ok(builder) => {
                    if !builder.commit() {
                        init_err = Some("onnxruntime environment commit failed".to_string());
                    }
                }
                Err(e) => init_err = Some(e.to_string()),
            }
        });
        if let Some(e) = init_err {
            return Err(OnnxEmbeddingError::RuntimeInit(e));
        }

        let tokenizer = Tokenizer::from_file(tokenizer_path).map_err(|e| {
            OnnxEmbeddingError::TokenizerLoad(tokenizer_path.to_path_buf(), e.to_string())
        })?;

        let session = Session::builder()
            .map_err(|e| OnnxEmbeddingError::ModelLoad(model_path.to_path_buf(), e.to_string()))?
            .commit_from_file(model_path)
            .map_err(|e| OnnxEmbeddingError::ModelLoad(model_path.to_path_buf(), e.to_string()))?;

        Ok(Self { session, tokenizer })
    }

    pub fn embed(&mut self, text: &str) -> Result<Vec<f32>, OnnxEmbeddingError> {
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| OnnxEmbeddingError::Tokenize(e.to_string()))?;

        let ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        let mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|&m| m as i64)
            .collect();
        let type_ids: Vec<i64> = encoding.get_type_ids().iter().map(|&t| t as i64).collect();
        let seq_len = ids.len();

        let input_ids = Tensor::from_array(([1usize, seq_len], ids))
            .map_err(|e| OnnxEmbeddingError::Inference(e.to_string()))?;
        let attention_mask = Tensor::from_array(([1usize, seq_len], mask.clone()))
            .map_err(|e| OnnxEmbeddingError::Inference(e.to_string()))?;
        let token_type_ids = Tensor::from_array(([1usize, seq_len], type_ids))
            .map_err(|e| OnnxEmbeddingError::Inference(e.to_string()))?;

        let outputs = self
            .session
            .run(ort::inputs![
                "input_ids" => input_ids,
                "attention_mask" => attention_mask,
                "token_type_ids" => token_type_ids,
            ])
            .map_err(|e| OnnxEmbeddingError::Inference(e.to_string()))?;

        let (shape, data) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| OnnxEmbeddingError::Inference(e.to_string()))?;

        if shape.len() < 3 {
            return Err(OnnxEmbeddingError::NoOutput);
        }
        let hidden_size = shape[2] as usize;

        Ok(mean_pool(data, seq_len, hidden_size, &mask))
    }
}

fn mean_pool(
    token_embeddings: &[f32],
    seq_len: usize,
    hidden_size: usize,
    mask: &[i64],
) -> Vec<f32> {
    let mut pooled = vec![0f32; hidden_size];
    let mut valid_tokens = 0f32;

    for (t, &m) in mask.iter().enumerate().take(seq_len) {
        if m == 0 {
            continue;
        }
        valid_tokens += 1.0;
        let offset = t * hidden_size;
        for h in 0..hidden_size {
            pooled[h] += token_embeddings[offset + h];
        }
    }

    if valid_tokens > 0.0 {
        for v in pooled.iter_mut() {
            *v /= valid_tokens;
        }
    }

    pooled
}
