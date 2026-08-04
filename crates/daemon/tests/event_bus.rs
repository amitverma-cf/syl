use daemon::events::{DaemonEvent, EventBus};

#[tokio::test]
async fn subscriber_receives_a_published_event() {
    let bus = EventBus::new(8);
    let mut receiver = bus.subscribe();

    bus.publish(DaemonEvent::FlowStateChanged {
        flow: "demo".to_string(),
        state: "greeting".to_string(),
    });

    let event = receiver.recv().await.unwrap();
    assert!(matches!(
        event,
        DaemonEvent::FlowStateChanged { flow, state }
            if flow == "demo" && state == "greeting"
    ));
}

#[tokio::test]
async fn multiple_subscribers_each_receive_the_event() {
    let bus = EventBus::new(8);
    let mut first = bus.subscribe();
    let mut second = bus.subscribe();

    bus.publish(DaemonEvent::ToolCallCompleted {
        tool: "read_file".to_string(),
        ok: true,
    });

    assert!(matches!(
        first.recv().await.unwrap(),
        DaemonEvent::ToolCallCompleted { ok: true, .. }
    ));
    assert!(matches!(
        second.recv().await.unwrap(),
        DaemonEvent::ToolCallCompleted { ok: true, .. }
    ));
}

#[tokio::test]
async fn publish_with_no_subscribers_does_not_panic() {
    let bus = EventBus::new(8);
    bus.publish(DaemonEvent::FlowStateChanged {
        flow: "demo".to_string(),
        state: "greeting".to_string(),
    });
}
