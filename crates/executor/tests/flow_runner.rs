use executor::{parse_flow, FlowRunner};

fn two_state_flow() -> &'static str {
    r#"{
        "name": "demo",
        "initial_state": "greeting",
        "states": [
            {
                "name": "greeting",
                "system_prompt": "Greet the user.",
                "tool_allowlist": ["read_file"],
                "transitions": [{ "on": "message", "to_state": "listing" }]
            },
            {
                "name": "listing",
                "system_prompt": "Summarize the file contents.",
                "tool_allowlist": ["read_file"],
                "on_enter_tool_call": { "tool": "read_file", "args": { "path": "notes.txt" } },
                "transitions": [{ "on": "message", "to_state": "greeting" }]
            }
        ]
    }"#
}

#[test]
fn starts_at_the_initial_state() {
    let flow = parse_flow(two_state_flow().as_bytes()).unwrap();
    let runner = FlowRunner::new(flow).unwrap();
    assert_eq!(runner.current_state().name, "greeting");
    assert!(runner.on_enter_tool_call().is_none());
}

#[test]
fn advance_follows_a_matching_transition() {
    let flow = parse_flow(two_state_flow().as_bytes()).unwrap();
    let mut runner = FlowRunner::new(flow).unwrap();
    assert!(runner.advance("message"));
    assert_eq!(runner.current_state().name, "listing");
    assert_eq!(runner.on_enter_tool_call().unwrap().tool, "read_file");
}

#[test]
fn advance_with_no_matching_transition_stays_put() {
    let flow = parse_flow(two_state_flow().as_bytes()).unwrap();
    let mut runner = FlowRunner::new(flow).unwrap();
    assert!(!runner.advance("unrelated_trigger"));
    assert_eq!(runner.current_state().name, "greeting");
}

#[test]
fn advance_cycles_back_through_the_flow() {
    let flow = parse_flow(two_state_flow().as_bytes()).unwrap();
    let mut runner = FlowRunner::new(flow).unwrap();
    runner.advance("message");
    runner.advance("message");
    assert_eq!(runner.current_state().name, "greeting");
}
