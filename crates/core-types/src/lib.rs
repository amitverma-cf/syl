pub mod workspace_paths;

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("not implemented")]
    NotImplemented,
}

pub type CoreResult<T> = Result<T, CoreError>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EngineId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RequestId(pub u64);
