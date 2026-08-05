use memory::Message;
use tools::compress_context;

fn message(content: &str) -> Message {
    Message {
        role: "user".to_string(),
        content: content.to_string(),
        created_at: 0,
    }
}

#[test]
fn keeps_all_messages_when_under_budget() {
    let messages = vec![message("hi"), message("there"), message("friend")];
    let result = compress_context(&messages, 1000);
    assert_eq!(result.len(), 3);
}

#[test]
fn drops_oldest_messages_first_when_over_budget() {
    let messages = vec![
        message("aaaaaaaaaa"),
        message("bbbbbbbbbb"),
        message("cccccccccc"),
    ];
    let result = compress_context(&messages, 15);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].content, "cccccccccc");
}

#[test]
fn preserves_chronological_order() {
    let messages = vec![message("one"), message("two"), message("three")];
    let result = compress_context(&messages, 1000);
    assert_eq!(result[0].content, "one");
    assert_eq!(result[1].content, "two");
    assert_eq!(result[2].content, "three");
}

#[test]
fn always_keeps_at_least_the_most_recent_message() {
    let messages = vec![message("a very long message that alone exceeds the budget")];
    let result = compress_context(&messages, 5);
    assert_eq!(result.len(), 1);
}

#[test]
fn empty_input_returns_empty_output() {
    let result = compress_context(&[], 1000);
    assert!(result.is_empty());
}
