//! Loads and represents agent workflows as a finite-state machine defined in JSON.

use serde::{Deserialize, Serialize};

/// One state in a [`Flow`]: the system prompt and allowed tools while active in this state,
/// and the transitions out of it.
#[derive(Debug, Serialize, Deserialize)]
pub struct FlowState {
    /// This state's unique name within its flow.
    pub name: String,
    /// The system prompt used while this state is active.
    pub system_prompt: String,
    /// Names of the tools the agent may call while this state is active.
    pub tool_allowlist: Vec<String>,
    /// Transitions out of this state.
    pub transitions: Vec<Transition>,
}

/// A single edge in a flow's state graph.
#[derive(Debug, Serialize, Deserialize)]
pub struct Transition {
    /// The trigger that causes this transition (e.g. a tool result or a user action).
    pub on: String,
    /// The name of the state to move to when `on` occurs.
    pub to_state: String,
}

/// A complete agent workflow: a named set of states and the transitions between them.
#[derive(Debug, Serialize, Deserialize)]
pub struct Flow {
    /// This flow's unique name.
    pub name: String,
    /// The name of the state execution starts in.
    pub initial_state: String,
    /// Every state in this flow.
    pub states: Vec<FlowState>,
}

/// Parses a flow definition from JSON bytes.
///
/// # Errors
/// Returns an error if `json_bytes` is not valid JSON or does not match the `Flow` shape.
pub fn parse_flow(mut json_bytes: Vec<u8>) -> Result<Flow, simd_json::Error> {
    simd_json::from_slice(&mut json_bytes)
}
