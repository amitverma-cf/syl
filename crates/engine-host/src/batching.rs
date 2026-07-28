use core_types::RequestId;

#[derive(Default)]
pub struct BatchState {
    pub request_ids: Vec<RequestId>,
    pub token_blocks: Vec<u16>,
    pub generation_status: Vec<u8>,
}

impl BatchState {
    pub fn new() -> Self {
        Self::default()
    }
}
