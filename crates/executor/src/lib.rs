use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct FlowState {
    pub name: String,
    pub system_prompt: String,
    pub tool_allowlist: Vec<String>,
    pub transitions: Vec<Transition>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Transition {
    pub on: String,
    pub to_state: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Flow {
    pub name: String,
    pub initial_state: String,
    pub states: Vec<FlowState>,
}

pub fn parse_flow(json_bytes: &[u8]) -> Result<Flow, serde_json::Error> {
    serde_json::from_slice(json_bytes)
}
