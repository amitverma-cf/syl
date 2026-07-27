//! Continuous-batching state for in-flight requests to a single engine.

use core_types::RequestId;

/// The set of requests currently in flight for one engine, stored as parallel arrays
/// (request id, token block, generation status) indexed by position.
#[derive(Default)]
pub struct BatchState {
    /// Id of each in-flight request, in slot order.
    pub request_ids: Vec<RequestId>,
    /// Most recently generated token block for each in-flight request, in slot order.
    pub token_blocks: Vec<u16>,
    /// Generation status of each in-flight request, in slot order.
    pub generation_status: Vec<u8>,
}

impl BatchState {
    /// Creates an empty batch with no in-flight requests.
    pub fn new() -> Self {
        Self::default()
    }
}
