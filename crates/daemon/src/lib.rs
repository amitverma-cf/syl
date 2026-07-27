//! Daemon pillar (Decision #10): background/OS-integrated process management,
//! scheduled jobs, parallel task execution, and the internal event/pub-sub bus
//! that Executor and Tool publish to and consume from. Backed by tokio.

pub mod events;
pub mod scheduler;
