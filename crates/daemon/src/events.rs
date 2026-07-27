//! A shared publish/subscribe bus for events raised across pillars.

use tokio::sync::broadcast;

/// An event published on the [`EventBus`].
#[derive(Debug, Clone)]
pub enum DaemonEvent {
    /// A flow moved to a new state.
    FlowStateChanged {
        /// Name of the flow that changed state.
        flow: String,
        /// Name of the state the flow moved to.
        state: String,
    },
    /// A tool call finished running.
    ToolCallCompleted {
        /// Name of the tool that was called.
        tool: String,
        /// Whether the call succeeded.
        ok: bool,
    },
}

/// A multi-producer, multi-consumer channel for [`DaemonEvent`]s.
pub struct EventBus {
    sender: broadcast::Sender<DaemonEvent>,
}

impl EventBus {
    /// Creates a new event bus that buffers up to `capacity` unread events per subscriber.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Returns a new receiver that will observe every event published from this point on.
    pub fn subscribe(&self) -> broadcast::Receiver<DaemonEvent> {
        self.sender.subscribe()
    }

    /// Publishes `event` to all current subscribers.
    pub fn publish(&self, event: DaemonEvent) {
        let _ = self.sender.send(event);
    }
}
