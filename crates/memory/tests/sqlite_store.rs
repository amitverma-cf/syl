use memory::{ConversationStore, SqliteConversationStore};

#[test]
fn append_then_list_returns_messages_in_order() {
    let store = SqliteConversationStore::open_in_memory().unwrap();
    store.append_message("c1", "user", "hello").unwrap();
    store.append_message("c1", "assistant", "hi there").unwrap();

    let messages = store.list_messages("c1").unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].role, "user");
    assert_eq!(messages[0].content, "hello");
    assert_eq!(messages[1].role, "assistant");
    assert_eq!(messages[1].content, "hi there");
}

#[test]
fn list_messages_on_unknown_conversation_returns_empty() {
    let store = SqliteConversationStore::open_in_memory().unwrap();
    let messages = store.list_messages("does-not-exist").unwrap();
    assert!(messages.is_empty());
}

#[test]
fn conversations_do_not_leak_into_each_other() {
    let store = SqliteConversationStore::open_in_memory().unwrap();
    store.append_message("c1", "user", "in c1").unwrap();
    store.append_message("c2", "user", "in c2").unwrap();

    let c1_messages = store.list_messages("c1").unwrap();
    assert_eq!(c1_messages.len(), 1);
    assert_eq!(c1_messages[0].content, "in c1");
}

#[test]
fn open_creates_schema_on_a_fresh_database() {
    let dir = std::env::temp_dir().join(format!("syl-memory-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let db_path = dir.join("test.sqlite");

    let store = memory::open(&db_path).unwrap();
    store.append_message("c1", "user", "hello").unwrap();
    let messages = store.list_messages("c1").unwrap();
    assert_eq!(messages.len(), 1);

    drop(store);
    std::fs::remove_dir_all(&dir).unwrap();
}
