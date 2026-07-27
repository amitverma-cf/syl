# Testing

## Rust

**Unit tests** live in the same file as the code they test, in a `#[cfg(test)] mod tests`
block at the bottom — standard Rust convention, keeps the test next to the contract it
verifies. Every pillar crate's public trait implementations get unit tests for their
documented behavior (not 100%-line-coverage for its own sake — test the contract, not every
private helper).

**Integration tests** (cross-module behavior within a crate, or behavior that only makes
sense end-to-end) live in that crate's `tests/` directory, e.g. `crates/engine-host/tests/`.
Loading a real llama.cpp shared library through `engine-host` and running one inference call
belongs here — it's not a unit test of one function, it's a contract test of the whole
plugin-loading path.

**What must be tested before a branch is ready to merge** (checked in `git-workflow.md`
step 3):
- Every new public function/trait method in a pillar crate has at least one test covering its
  documented behavior and one covering its documented error case.
- Any parser (flow JSON, registry JSON) is tested against both valid and deliberately
  malformed input — malformed input must produce a typed error, never a panic.
- Any FFI boundary code (`engine-host`) is tested against the real native library it loads,
  not just mocked — a passing test that never touched the actual `.dll`/`.so` doesn't verify
  the ABI contract that matters.

**What is not required:** exhaustive property-based testing, 100% branch coverage, testing
framework/library glue code (e.g. don't write a test that just asserts `tokio::spawn` runs a
closure — that's testing tokio, not this project).

## TypeScript / React (`ui/`)

**Logic** (anything that isn't a component's render output — formatting helpers, IPC message
shaping, state-derivation functions) gets unit tests via Vitest, co-located as
`foo.test.ts` next to `foo.ts`.

**Components and UI behavior are not covered by automated component tests at this stage** —
per project norm, UI-affecting changes are verified by actually running the app and exercising
the feature (see `style-guide.md`'s "UI changes" section), not by maintaining a parallel
component-test suite for a single-maintainer project at this size. Revisit this if/when the
UI surface grows enough that manual verification stops being reliable.

## What "done" means

A change isn't complete when the code compiles — it's complete when its intended behavior can
be demonstrated: a passing test suite for logic-level changes, or an actual run of the app for
UI/end-to-end changes. The PR description (`git-workflow.md` step 5) states which one applied
and how it was verified.
