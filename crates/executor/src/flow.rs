use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum FlowError {
    #[error("flow file does not conform to the schema: {0}")]
    SchemaViolation(String),
    #[error("flow file is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("initial_state {0:?} does not match any state name")]
    UnknownInitialState(String),
    #[error("state {state:?} has a duplicate name")]
    DuplicateStateName { state: String },
    #[error("state {state:?} has a transition to unknown state {to_state:?}")]
    UnknownTransitionTarget { state: String, to_state: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallSpec {
    pub tool: String,
    pub args: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FlowState {
    pub name: String,
    pub system_prompt: String,
    pub tool_allowlist: Vec<String>,
    #[serde(default)]
    pub on_enter_tool_call: Option<ToolCallSpec>,
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

fn schema() -> Value {
    serde_json::json!({
        "type": "object",
        "required": ["name", "initial_state", "states"],
        "additionalProperties": false,
        "properties": {
            "name": { "type": "string", "minLength": 1 },
            "initial_state": { "type": "string", "minLength": 1 },
            "states": {
                "type": "array",
                "minItems": 1,
                "items": {
                    "type": "object",
                    "required": ["name", "system_prompt", "tool_allowlist", "transitions"],
                    "additionalProperties": false,
                    "properties": {
                        "name": { "type": "string", "minLength": 1 },
                        "system_prompt": { "type": "string" },
                        "tool_allowlist": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "on_enter_tool_call": {
                            "type": "object",
                            "required": ["tool", "args"],
                            "additionalProperties": false,
                            "properties": {
                                "tool": { "type": "string", "minLength": 1 },
                                "args": {}
                            }
                        },
                        "transitions": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "required": ["on", "to_state"],
                                "additionalProperties": false,
                                "properties": {
                                    "on": { "type": "string", "minLength": 1 },
                                    "to_state": { "type": "string", "minLength": 1 }
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}

fn validate_semantics(flow: &Flow) -> Result<(), FlowError> {
    let mut seen = std::collections::HashSet::new();
    for state in &flow.states {
        if !seen.insert(state.name.as_str()) {
            return Err(FlowError::DuplicateStateName {
                state: state.name.clone(),
            });
        }
    }
    if !seen.contains(flow.initial_state.as_str()) {
        return Err(FlowError::UnknownInitialState(flow.initial_state.clone()));
    }
    for state in &flow.states {
        for transition in &state.transitions {
            if !seen.contains(transition.to_state.as_str()) {
                return Err(FlowError::UnknownTransitionTarget {
                    state: state.name.clone(),
                    to_state: transition.to_state.clone(),
                });
            }
        }
    }
    Ok(())
}

pub fn parse_flow(json_bytes: &[u8]) -> Result<Flow, FlowError> {
    let value: Value = serde_json::from_slice(json_bytes)?;

    let validator = jsonschema::validator_for(&schema())
        .expect("flow schema is a static, valid JSON Schema document");
    let errors: Vec<String> = validator
        .iter_errors(&value)
        .map(|err| format!("{err} (at {})", err.instance_path()))
        .collect();
    if !errors.is_empty() {
        return Err(FlowError::SchemaViolation(errors.join("; ")));
    }

    let flow: Flow = serde_json::from_value(value)?;
    validate_semantics(&flow)?;
    Ok(flow)
}
