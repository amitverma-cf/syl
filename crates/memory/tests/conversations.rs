use memory::{ConversationStore, SqliteConversationStore};

#[test]
fn create_then_list_returns_the_conversation() {
    let store = SqliteConversationStore::open_in_memory().unwrap();
    store
        .create_conversation("c1", "First chat", "default")
        .unwrap();

    let conversations = store.list_conversations().unwrap();
    assert_eq!(conversations.len(), 1);
    assert_eq!(conversations[0].id, "c1");
    assert_eq!(conversations[0].title, "First chat");
    assert_eq!(conversations[0].flow_name, "default");
}

#[test]
fn list_conversations_is_empty_when_none_created() {
    let store = SqliteConversationStore::open_in_memory().unwrap();
    assert!(store.list_conversations().unwrap().is_empty());
}

#[test]
fn list_conversations_orders_most_recently_updated_first() {
    let store = SqliteConversationStore::open_in_memory().unwrap();
    store.create_conversation("c1", "Older", "default").unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    store.create_conversation("c2", "Newer", "default").unwrap();

    let conversations = store.list_conversations().unwrap();
    assert_eq!(conversations[0].id, "c2");
    assert_eq!(conversations[1].id, "c1");
}

#[test]
fn rename_conversation_updates_the_title() {
    let store = SqliteConversationStore::open_in_memory().unwrap();
    store
        .create_conversation("c1", "Old title", "default")
        .unwrap();
    store.rename_conversation("c1", "New title").unwrap();

    let conversations = store.list_conversations().unwrap();
    assert_eq!(conversations[0].title, "New title");
}

#[test]
fn set_conversation_flow_updates_the_flow_name() {
    let store = SqliteConversationStore::open_in_memory().unwrap();
    store.create_conversation("c1", "Chat", "default").unwrap();
    store
        .set_conversation_flow("c1", "coding_assistant")
        .unwrap();

    let conversations = store.list_conversations().unwrap();
    assert_eq!(conversations[0].flow_name, "coding_assistant");
}

#[test]
fn delete_conversation_removes_it_and_its_messages() {
    let store = SqliteConversationStore::open_in_memory().unwrap();
    store.create_conversation("c1", "Chat", "default").unwrap();
    store.append_message("c1", "user", "hello").unwrap();

    store.delete_conversation("c1").unwrap();

    assert!(store.list_conversations().unwrap().is_empty());
    assert!(store.list_messages("c1").unwrap().is_empty());
}

#[test]
fn delete_conversation_does_not_affect_other_conversations() {
    let store = SqliteConversationStore::open_in_memory().unwrap();
    store
        .create_conversation("c1", "Chat 1", "default")
        .unwrap();
    store
        .create_conversation("c2", "Chat 2", "default")
        .unwrap();

    store.delete_conversation("c1").unwrap();

    let remaining = store.list_conversations().unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, "c2");
}
