//! Internal event/pub-sub bus (tokio::sync::broadcast) shared across pillars.

use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub enum DaemonEvent {
    FlowStateChanged { flow: String, state: String },
    ToolCallCompleted { tool: String, ok: bool },
}

pub struct EventBus {
    sender: broadcast::Sender<DaemonEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DaemonEvent> {
        self.sender.subscribe()
    }

    pub fn publish(&self, event: DaemonEvent) {
        let _ = self.sender.send(event);
    }
}
