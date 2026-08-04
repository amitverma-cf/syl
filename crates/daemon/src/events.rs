use tokio::sync::broadcast;

/// The in-process broadcast channel below hands `DaemonEvent` values around as
/// owned Rust values — no serialization involved, so `rkyv` buys nothing on
/// that path today. The zero-copy `Archive` representation exists so that if
/// this bus is ever backed by real byte-level transport (e.g. crossing a
/// process boundary, per architecture.md's process-per-engine-isolation
/// direction), encoding/decoding is already in place and tested.
#[derive(Debug, Clone, PartialEq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub enum DaemonEvent {
    FlowStateChanged { flow: String, state: String },
    ToolCallCompleted { tool: String, ok: bool },
    RegistryPolled { ok: bool },
    ScheduledJobFired { job: String, ok: bool },
    LocalModelCrashed { name: String },
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
