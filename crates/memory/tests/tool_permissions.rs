use memory::{SqliteConversationStore, ToolPermissionDecision, ToolPermissionStore};

#[test]
fn set_then_get_returns_the_remembered_decision() {
    let store = SqliteConversationStore::open_in_memory().unwrap();
    store
        .set_tool_permission("c1", "run_command", ToolPermissionDecision::Allow)
        .unwrap();

    let decision = store.get_tool_permission("c1", "run_command").unwrap();
    assert_eq!(decision, Some(ToolPermissionDecision::Allow));
}

#[test]
fn get_with_no_remembered_decision_returns_none() {
    let store = SqliteConversationStore::open_in_memory().unwrap();
    assert_eq!(
        store.get_tool_permission("c1", "run_command").unwrap(),
        None
    );
}

#[test]
fn clear_tool_permission_forgets_a_remembered_decision() {
    let store = SqliteConversationStore::open_in_memory().unwrap();
    store
        .set_tool_permission("c1", "run_command", ToolPermissionDecision::Allow)
        .unwrap();

    store.clear_tool_permission("c1", "run_command").unwrap();

    assert_eq!(
        store.get_tool_permission("c1", "run_command").unwrap(),
        None
    );
}

#[test]
fn clear_tool_permission_on_nothing_remembered_is_a_no_op() {
    let store = SqliteConversationStore::open_in_memory().unwrap();
    store.clear_tool_permission("c1", "run_command").unwrap();
    assert_eq!(
        store.get_tool_permission("c1", "run_command").unwrap(),
        None
    );
}

#[test]
fn clear_tool_permission_does_not_affect_other_tools_or_conversations() {
    let store = SqliteConversationStore::open_in_memory().unwrap();
    store
        .set_tool_permission("c1", "run_command", ToolPermissionDecision::Allow)
        .unwrap();
    store
        .set_tool_permission("c1", "write_file", ToolPermissionDecision::Deny)
        .unwrap();
    store
        .set_tool_permission("c2", "run_command", ToolPermissionDecision::Allow)
        .unwrap();

    store.clear_tool_permission("c1", "run_command").unwrap();

    assert_eq!(
        store.get_tool_permission("c1", "run_command").unwrap(),
        None
    );
    assert_eq!(
        store.get_tool_permission("c1", "write_file").unwrap(),
        Some(ToolPermissionDecision::Deny)
    );
    assert_eq!(
        store.get_tool_permission("c2", "run_command").unwrap(),
        Some(ToolPermissionDecision::Allow)
    );
}

#[test]
fn list_tool_permissions_returns_every_remembered_decision_for_a_conversation() {
    let store = SqliteConversationStore::open_in_memory().unwrap();
    store
        .set_tool_permission("c1", "run_command", ToolPermissionDecision::Allow)
        .unwrap();
    store
        .set_tool_permission("c1", "write_file", ToolPermissionDecision::Deny)
        .unwrap();
    store
        .set_tool_permission("c2", "run_command", ToolPermissionDecision::Allow)
        .unwrap();

    let entries = store.list_tool_permissions("c1").unwrap();
    assert_eq!(
        entries,
        vec![
            ("run_command".to_string(), ToolPermissionDecision::Allow),
            ("write_file".to_string(), ToolPermissionDecision::Deny),
        ]
    );
}

#[test]
fn list_tool_permissions_on_a_conversation_with_none_is_empty() {
    let store = SqliteConversationStore::open_in_memory().unwrap();
    assert!(store.list_tool_permissions("c1").unwrap().is_empty());
}

#[test]
fn set_tool_permission_updates_an_existing_decision() {
    let store = SqliteConversationStore::open_in_memory().unwrap();
    store
        .set_tool_permission("c1", "run_command", ToolPermissionDecision::Deny)
        .unwrap();
    store
        .set_tool_permission("c1", "run_command", ToolPermissionDecision::Allow)
        .unwrap();

    assert_eq!(
        store.get_tool_permission("c1", "run_command").unwrap(),
        Some(ToolPermissionDecision::Allow)
    );
    assert_eq!(store.list_tool_permissions("c1").unwrap().len(), 1);
}
