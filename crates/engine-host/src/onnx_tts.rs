use std::collections::HashMap;
use std::path::Path;
use std::sync::Once;

use ort::session::Session;
use ort::value::Tensor;

static INIT_RUNTIME: Once = Once::new();

const BLANK_CHAR: char = '_';

#[derive(Debug, thiserror::Error)]
pub enum OnnxTtsError {
    #[error("failed to load onnxruntime library: {0}")]
    RuntimeInit(String),
    #[error("failed to load vocab from {}: {1}", .0.display())]
    VocabLoad(std::path::PathBuf, String),
    #[error("failed to load onnx model from {}: {1}", .0.display())]
    ModelLoad(std::path::PathBuf, String),
    #[error("onnx inference failed: {0}")]
    Inference(String),
    #[error("text produced no synthesizable characters")]
    EmptyInput,
}

pub struct OnnxTtsEngine {
    session: Session,
    vocab: HashMap<char, i64>,
    blank_id: i64,
}

impl OnnxTtsEngine {
    pub fn load(
        runtime_library_path: &Path,
        model_path: &Path,
        vocab_path: &Path,
    ) -> Result<Self, OnnxTtsError> {
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
            return Err(OnnxTtsError::RuntimeInit(e));
        }

        let vocab_contents = std::fs::read_to_string(vocab_path)
            .map_err(|e| OnnxTtsError::VocabLoad(vocab_path.to_path_buf(), e.to_string()))?;
        let raw_vocab: HashMap<String, i64> = serde_json::from_str(&vocab_contents)
            .map_err(|e| OnnxTtsError::VocabLoad(vocab_path.to_path_buf(), e.to_string()))?;
        let vocab: HashMap<char, i64> = raw_vocab
            .into_iter()
            .filter_map(|(k, v)| k.chars().next().map(|c| (c, v)))
            .collect();
        let blank_id = *vocab.get(&BLANK_CHAR).ok_or_else(|| {
            OnnxTtsError::VocabLoad(
                vocab_path.to_path_buf(),
                "vocab is missing the blank token '_'".to_string(),
            )
        })?;

        let session = Session::builder()
            .map_err(|e| OnnxTtsError::ModelLoad(model_path.to_path_buf(), e.to_string()))?
            .commit_from_file(model_path)
            .map_err(|e| OnnxTtsError::ModelLoad(model_path.to_path_buf(), e.to_string()))?;

        Ok(Self {
            session,
            vocab,
            blank_id,
        })
    }

    fn tokenize(&self, text: &str) -> Vec<i64> {
        let ids: Vec<i64> = text
            .to_lowercase()
            .chars()
            .filter_map(|c| self.vocab.get(&c).copied())
            .collect();

        let mut interspersed = vec![self.blank_id; ids.len() * 2 + 1];
        for (i, id) in ids.into_iter().enumerate() {
            interspersed[i * 2 + 1] = id;
        }
        interspersed
    }

    pub fn synthesize(&mut self, text: &str) -> Result<Vec<f32>, OnnxTtsError> {
        let ids = self.tokenize(text);
        if ids.len() <= 1 {
            return Err(OnnxTtsError::EmptyInput);
        }
        let seq_len = ids.len();
        let attention_mask = vec![1i64; seq_len];

        let input_ids = Tensor::from_array(([1usize, seq_len], ids))
            .map_err(|e| OnnxTtsError::Inference(e.to_string()))?;
        let attention_mask = Tensor::from_array(([1usize, seq_len], attention_mask))
            .map_err(|e| OnnxTtsError::Inference(e.to_string()))?;

        let outputs = self
            .session
            .run(ort::inputs![
                "input_ids" => input_ids,
                "attention_mask" => attention_mask,
            ])
            .map_err(|e| OnnxTtsError::Inference(e.to_string()))?;

        let (_, waveform) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| OnnxTtsError::Inference(e.to_string()))?;

        Ok(waveform.to_vec())
    }
}
