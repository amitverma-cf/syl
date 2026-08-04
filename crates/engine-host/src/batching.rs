use bumpalo::Bump;
use core_types::RequestId;

/// Backing arena for one batch's worth of per-request token-id arrays.
/// Deliberately *not* the engine's own KV-cache (that lives inside the native
/// engine library and is out of scope here) — this only replaces the
/// allocation strategy for the small per-request bookkeeping arrays
/// (`token_blocks`) that already cross the FFI boundary once per request per
/// batch. Instead of a fresh heap-allocated `Vec<u16>` per request, every
/// request's tokens in a batch are bump-allocated out of the same arena;
/// `reset()` reclaims all of them at once between batches — an O(1)
/// bump-pointer reset instead of N individual `Vec` drops.
#[derive(Default)]
pub struct BatchArena {
    bump: Bump,
}

impl BatchArena {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reclaims every allocation made from this arena so far. The borrow
    /// checker enforces the actual safety invariant here: this takes `&mut
    /// self`, so it cannot be called while any `BatchState` still borrows
    /// token slices from this arena — the same requirement that lets
    /// `push_request` avoid any `unsafe` code.
    pub fn reset(&mut self) {
        self.bump.reset();
    }

    pub fn allocated_bytes(&self) -> usize {
        self.bump.allocated_bytes()
    }
}

/// One batch's per-request bookkeeping: which requests are in it, their
/// generation status, and their token-id arrays (borrowed from a
/// `BatchArena` that outlives this `BatchState`).
#[derive(Default)]
pub struct BatchState<'arena> {
    pub request_ids: Vec<RequestId>,
    pub generation_status: Vec<u8>,
    token_blocks: Vec<&'arena [u16]>,
}

impl<'arena> BatchState<'arena> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one request's token-id array to the batch, copying `tokens` into
    /// `arena` rather than allocating a fresh `Vec` for it.
    pub fn push_request(
        &mut self,
        arena: &'arena BatchArena,
        id: RequestId,
        tokens: &[u16],
        status: u8,
    ) {
        let allocated = arena.bump.alloc_slice_copy(tokens);
        self.request_ids.push(id);
        self.generation_status.push(status);
        self.token_blocks.push(allocated);
    }

    pub fn token_blocks(&self) -> &[&'arena [u16]] {
        &self.token_blocks
    }

    pub fn len(&self) -> usize {
        self.request_ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.request_ids.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_batch_is_empty() {
        let batch = BatchState::new();
        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);
    }

    #[test]
    fn push_request_records_bookkeeping_and_the_arena_allocated_tokens() {
        let arena = BatchArena::new();
        let mut batch = BatchState::new();
        batch.push_request(&arena, RequestId(1), &[10, 20, 30], 0);
        batch.push_request(&arena, RequestId(2), &[40, 50], 1);

        assert_eq!(batch.len(), 2);
        assert_eq!(batch.request_ids, vec![RequestId(1), RequestId(2)]);
        assert_eq!(batch.generation_status, vec![0, 1]);
        assert_eq!(batch.token_blocks(), &[&[10u16, 20, 30][..], &[40, 50][..]]);
    }

    #[test]
    fn a_reset_arena_can_be_reused_for_the_next_batch() {
        let mut arena = BatchArena::new();
        {
            let mut batch = BatchState::new();
            batch.push_request(&arena, RequestId(1), &[1, 2, 3], 0);
            assert_eq!(batch.len(), 1);
        }
        // `batch` (and its borrows of `arena`) is out of scope here, so this
        // compiles — reset() taking `&mut self` while a `BatchState<'arena>`
        // still existed would be a borrow-checker error, not a runtime bug.
        arena.reset();

        let mut next_batch = BatchState::new();
        next_batch.push_request(&arena, RequestId(2), &[9, 9], 0);
        assert_eq!(next_batch.len(), 1);
        assert_eq!(next_batch.token_blocks(), &[&[9u16, 9][..]]);
    }

    #[test]
    fn resetting_reclaims_arena_memory_instead_of_growing_unbounded() {
        let mut arena = BatchArena::new();
        for _ in 0..100 {
            let mut batch = BatchState::new();
            for i in 0..8u16 {
                batch.push_request(&arena, RequestId(i as u64), &[i; 16], 0);
            }
            arena.reset();
        }
        // If every batch's allocations leaked instead of being reclaimed,
        // 100 batches * 8 requests * 32 bytes would add up to well over this.
        assert!(arena.allocated_bytes() < 4096);
    }
}
