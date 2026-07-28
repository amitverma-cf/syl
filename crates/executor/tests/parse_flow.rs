use executor::{parse_flow, FlowError};

fn valid_two_state_flow() -> &'static str {
    r#"{
        "name": "demo",
        "initial_state": "greeting",
        "states": [
            {
                "name": "greeting",
                "system_prompt": "Greet the user and offer to list files.",
                "tool_allowlist": ["read_file"],
                "transitions": [{ "on": "message", "to_state": "listing" }]
            },
            {
                "name": "listing",
                "system_prompt": "Summarize the file contents for the user.",
                "tool_allowlist": ["read_file"],
                "on_enter_tool_call": { "tool": "read_file", "args": { "path": "notes.txt" } },
                "transitions": []
            }
        ]
    }"#
}

#[test]
fn parses_a_valid_two_state_flow() {
    let flow = parse_flow(valid_two_state_flow().as_bytes()).unwrap();
    assert_eq!(flow.name, "demo");
    assert_eq!(flow.states.len(), 2);
    assert_eq!(
        flow.states[1].on_enter_tool_call.as_ref().unwrap().tool,
        "read_file"
    );
}

#[test]
fn rejects_malformed_json() {
    let err = parse_flow(b"{not json").unwrap_err();
    assert!(matches!(err, FlowError::Json(_)));
}

#[test]
fn rejects_schema_violation_missing_required_field() {
    let json = r#"{ "name": "demo", "states": [] }"#;
    let err = parse_flow(json.as_bytes()).unwrap_err();
    assert!(matches!(err, FlowError::SchemaViolation(_)));
}

#[test]
fn rejects_unknown_initial_state() {
    let json = r#"{
        "name": "demo",
        "initial_state": "nowhere",
        "states": [
            { "name": "a", "system_prompt": "", "tool_allowlist": [], "transitions": [] }
        ]
    }"#;
    let err = parse_flow(json.as_bytes()).unwrap_err();
    assert!(matches!(err, FlowError::UnknownInitialState(state) if state == "nowhere"));
}

#[test]
fn rejects_duplicate_state_names() {
    let json = r#"{
        "name": "demo",
        "initial_state": "a",
        "states": [
            { "name": "a", "system_prompt": "", "tool_allowlist": [], "transitions": [] },
            { "name": "a", "system_prompt": "", "tool_allowlist": [], "transitions": [] }
        ]
    }"#;
    let err = parse_flow(json.as_bytes()).unwrap_err();
    assert!(matches!(err, FlowError::DuplicateStateName { state } if state == "a"));
}

#[test]
fn rejects_transition_to_unknown_state() {
    let json = r#"{
        "name": "demo",
        "initial_state": "a",
        "states": [
            {
                "name": "a",
                "system_prompt": "",
                "tool_allowlist": [],
                "transitions": [{ "on": "message", "to_state": "nowhere" }]
            }
        ]
    }"#;
    let err = parse_flow(json.as_bytes()).unwrap_err();
    assert!(
        matches!(err, FlowError::UnknownTransitionTarget { to_state, .. } if to_state == "nowhere")
    );
}
