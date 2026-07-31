use std::path::Path;
use std::sync::Once;

use ort::session::Session;
use ort::value::Tensor;
use tokenizers::Tokenizer;

use crate::mel::{log_mel_spectrogram, N_FRAMES, N_MELS};

static INIT_RUNTIME: Once = Once::new();

const START_OF_TRANSCRIPT: i64 = 50258;
const LANGUAGE_EN: i64 = 50259;
const TASK_TRANSCRIBE: i64 = 50359;
const NO_TIMESTAMPS: i64 = 50363;
const END_OF_TEXT: i64 = 50257;
const MAX_NEW_TOKENS: usize = 128;

#[derive(Debug, thiserror::Error)]
pub enum OnnxAsrError {
    #[error("failed to load onnxruntime library: {0}")]
    RuntimeInit(String),
    #[error("failed to load tokenizer from {}: {1}", .0.display())]
    TokenizerLoad(std::path::PathBuf, String),
    #[error("failed to load onnx model from {}: {1}", .0.display())]
    ModelLoad(std::path::PathBuf, String),
    #[error("onnx inference failed: {0}")]
    Inference(String),
}

pub struct OnnxAsrEngine {
    encoder: Session,
    decoder: Session,
    tokenizer: Tokenizer,
}

impl OnnxAsrEngine {
    pub fn load(
        runtime_library_path: &Path,
        encoder_path: &Path,
        decoder_path: &Path,
        tokenizer_path: &Path,
    ) -> Result<Self, OnnxAsrError> {
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
            return Err(OnnxAsrError::RuntimeInit(e));
        }

        let tokenizer = Tokenizer::from_file(tokenizer_path).map_err(|e| {
            OnnxAsrError::TokenizerLoad(tokenizer_path.to_path_buf(), e.to_string())
        })?;

        let encoder = Session::builder()
            .map_err(|e| OnnxAsrError::ModelLoad(encoder_path.to_path_buf(), e.to_string()))?
            .commit_from_file(encoder_path)
            .map_err(|e| OnnxAsrError::ModelLoad(encoder_path.to_path_buf(), e.to_string()))?;

        let decoder = Session::builder()
            .map_err(|e| OnnxAsrError::ModelLoad(decoder_path.to_path_buf(), e.to_string()))?
            .commit_from_file(decoder_path)
            .map_err(|e| OnnxAsrError::ModelLoad(decoder_path.to_path_buf(), e.to_string()))?;

        Ok(Self {
            encoder,
            decoder,
            tokenizer,
        })
    }

    pub fn transcribe(&mut self, pcm_16khz_mono: &[f32]) -> Result<String, OnnxAsrError> {
        let mel = log_mel_spectrogram(pcm_16khz_mono);
        let input_features = Tensor::from_array(([1usize, N_MELS, N_FRAMES], mel))
            .map_err(|e| OnnxAsrError::Inference(e.to_string()))?;

        let encoder_outputs = self
            .encoder
            .run(ort::inputs!["input_features" => input_features])
            .map_err(|e| OnnxAsrError::Inference(e.to_string()))?;
        let (hidden_shape, hidden_data) = encoder_outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| OnnxAsrError::Inference(e.to_string()))?;
        let hidden_shape: Vec<usize> = hidden_shape.iter().map(|&d| d as usize).collect();
        let hidden_data = hidden_data.to_vec();

        let mut tokens = vec![
            START_OF_TRANSCRIPT,
            LANGUAGE_EN,
            TASK_TRANSCRIBE,
            NO_TIMESTAMPS,
        ];

        for _ in 0..MAX_NEW_TOKENS {
            let encoder_hidden_states =
                Tensor::from_array((hidden_shape.clone(), hidden_data.clone()))
                    .map_err(|e| OnnxAsrError::Inference(e.to_string()))?;
            let input_ids = Tensor::from_array(([1usize, tokens.len()], tokens.clone()))
                .map_err(|e| OnnxAsrError::Inference(e.to_string()))?;

            let outputs = self
                .decoder
                .run(ort::inputs![
                    "input_ids" => input_ids,
                    "encoder_hidden_states" => encoder_hidden_states,
                ])
                .map_err(|e| OnnxAsrError::Inference(e.to_string()))?;

            let (logits_shape, logits_data) = outputs[0]
                .try_extract_tensor::<f32>()
                .map_err(|e| OnnxAsrError::Inference(e.to_string()))?;
            let vocab_size = logits_shape[2] as usize;
            let seq_len = logits_shape[1] as usize;
            let last_logits = &logits_data[(seq_len - 1) * vocab_size..seq_len * vocab_size];

            let next_token = last_logits
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(idx, _)| idx as i64)
                .unwrap_or(END_OF_TEXT);

            if next_token == END_OF_TEXT {
                break;
            }
            tokens.push(next_token);
        }

        let generated: Vec<u32> = tokens[4..].iter().map(|&t| t as u32).collect();
        self.tokenizer
            .decode(&generated, true)
            .map_err(|e| OnnxAsrError::Inference(e.to_string()))
    }
}
