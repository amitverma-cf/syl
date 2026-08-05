# syl — project conventions

For the full system map (what every crate/directory is and how they fit together), read
[`local/ai-docs/architecture.md`](local/ai-docs/architecture.md) first. This file is the
short list of standing rules that apply to every change in this repo.

## No inline comments

Never add `//` or `/// ` comments to code. If something needs explaining — a non-obvious
invariant, a workaround for a specific bug, the reason behind an `unsafe` block, why a
dependency was added or removed — write it into the relevant file under `local/ai-docs/` instead
(usually `architecture.md`; add a new file there if the topic doesn't fit an existing one) and
reference that file from the PR/commit description. Code should read clearly enough from naming
alone that it doesn't need a comment to explain *what* it does; docs explain *why*.

## Naming

- Crate, directory, and file names describe what the thing **is**, not how it happens to be
  implemented today. Prefer a name a new contributor could guess the purpose of without opening
  the file.
- Function and variable names spell out intent over abbreviation — match the existing test-name
  convention in this repo (e.g. `a_crashed_local_model_is_pruned_instead_of_reporting_loaded_forever`),
  which is deliberately long and literal rather than clever.
- Before picking a crate name, check it doesn't collide with an existing module name it will be
  imported alongside — e.g. `crates/flow-engine` is not named `flows` because `src-tauri/src/flows.rs`
  already owns that name as a Tauri-command module in the same binary; a plain `use flows::X` from
  inside `flows.rs` would be genuinely ambiguous. Also avoid shadowing an `std`/`core` prelude name
  (a crate is never named `core`).

## Extensions

Every local inference engine is a backend extension under `crates/extensions/<name>-worker/` —
`chat-worker`, `image-worker`, `embedding-worker`, `asr-worker`, `tts-worker` are the reference
set. A new one should follow the same shape: one small synchronous binary, one capability id
(`<domain>.<verb>/v1`), speaking `extension-host`'s stdio protocol, loading its native engine
from `crates/native-engines`. A UI-only extension (no backend process — see the Flow Editor)
uses `backend: None` in its manifest and only declares `contributes`.

## Verification

Before considering a change done: `cargo fmt --check`, `cargo clippy --workspace --all-targets`,
`cargo test --workspace` for any Rust change; `pnpm --dir ui build`/`lint`/`test` plus the `e2e/`
suite for any UI-observable change. See `local/ai-docs/testing.md` and `local/ai-docs/git-workflow.md`
for the full checklist.
