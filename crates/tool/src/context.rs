use memory::Message;

pub fn compress_context(messages: &[Message], max_total_chars: usize) -> Vec<Message> {
    let mut kept = Vec::new();
    let mut total = 0usize;

    for message in messages.iter().rev() {
        let len = message.content.len();
        if total + len > max_total_chars && !kept.is_empty() {
            break;
        }
        kept.push(message.clone());
        total += len;
    }

    kept.reverse();
    kept
}
