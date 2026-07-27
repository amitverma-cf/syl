# Style Guide

## Rust

**Formatting:** `rustfmt` with workspace defaults (no custom `rustfmt.toml` unless a specific
rule causes real friction — don't bikeshed formatting config). Run `cargo fmt` before every
commit; CI checks `cargo fmt --check`.

**Linting:** `cargo clippy --workspace --all-targets -- -D warnings`. Warnings are not
optional cleanup for later — a branch with clippy warnings doesn't get pushed (see
`git-workflow.md` step 4).

### Banned

- **`unwrap()` / `expect()` outside of tests and `main`/`run()` startup code.** Library code
  (anything in `crates/*`) returns `Result` and propagates with `?`. An `unwrap()` in a pillar
  crate is a crash waiting to happen in a GUI app where the user has no terminal to read a
  panic message from.
- **`unsafe` outside `engine-host`.** FFI into native engine plugins is the one place `unsafe`
  is inherent to the problem (Decision #2/#3 in the architecture doc). Anywhere else, `unsafe`
  is a sign the abstraction is wrong — fix the abstraction instead.
- **`println!`/`eprintln!` for anything other than a one-off local debug print you remove
  before committing.** Use `tracing` (already implied by the async/tokio stack) for anything
  that should survive in the codebase.
- **`#[allow(dead_code)]`, `#[allow(unused)]`, or similar left in committed code.** If
  something is genuinely unused, delete it. If it's a trait method required by a trait
  contract but not yet called, that's fine and doesn't need silencing — clippy won't flag it.
- **Backward-compatibility shims** — renaming instead of deleting unused code, `// removed`
  comments, deprecated-but-kept function variants. If it's unused, delete it. `git log` is the
  history, not the source tree.
- **Speculative abstraction** — a trait with one implementation "in case we need another
  engine later," a config option nothing reads yet, a generic parameter with one concrete use.
  Add the abstraction when the second concrete case shows up, not before.

### Allowed / expected

- **Doc comments (`///`) only on public items where the *why* isn't obvious from the
  signature** — a non-obvious invariant, a constraint inherited from the native ABI, a reason
  a parameter must be validated a certain way. Not a restatement of the function name. A
  well-named `fn preflight_check(...) -> bool` needs no comment; a comment explaining *why*
  the safety margin is 5% (per `docs/active/plan.md` Decision #5) does.
- **One responsibility per file, one pillar's concerns per crate.** If `engine-host/src/`
  grows a file mixing plugin loading and batching logic, split it — it already doesn't (see
  `plugin.rs`/`watchdog.rs`/`batching.rs`).
- **`Result<T, E>` with crate-specific error enums** (`thiserror`-derived), not `anyhow`
  inside library crates — callers need to match on error kinds (e.g. the UI needs to tell
  "model won't fit in RAM" apart from "plugin file not found"). `anyhow` is acceptable at the
  `src-tauri` binary boundary where errors just need to reach the user as a message.
- **`#[repr(C)]` and explicit layout for anything crossing the FFI boundary** in
  `engine-host` — no relying on Rust's default struct layout for data a C ABI reads.

## TypeScript / React (`ui/`)

**Formatting:** Prettier, default config. **Linting:** ESLint with the React + TypeScript
recommended rule sets; `any` is banned (`@typescript-eslint/no-explicit-any` as an error, not
a warning) — if a type is genuinely unknown, use `unknown` and narrow it.

### Banned

- Class components — function components + hooks only.
- Inline styles (`style={{...}}`) except for values that are genuinely computed at runtime
  (e.g. a measured pixel offset) — everything static goes through Tailwind classes.
- A component-library or animation dependency added "for later." Pull in exactly what a
  screen currently needs (per `docs/active/plan.md` Decision #1: React + Tailwind, nothing
  fancy by default).
- Prop drilling more than two levels — reach for React context or lift state instead of
  threading a prop through components that don't use it themselves.

### Allowed / expected

- Co-locate a component's types with the component unless a type is genuinely shared across
  many components, in which case it goes in `ui/src/types/`.
- Tailwind utility classes directly in JSX; a `className` string is not something to abstract
  behind a helper until it's repeated identically in three or more places.

## Comments, in general (both languages)

Default to no comments. When you do write one, it explains the *why*, not the *what* — a
hidden constraint, a workaround for a specific upstream bug, a non-obvious invariant. Never
reference the current task, branch, or PR number in a code comment (that belongs in the
commit message and PR description, per `git-workflow.md`) — comments rot as the codebase
evolves; commit history doesn't.

## UI changes

Per project norm: before calling a UI-affecting task done, run the app (`pnpm tauri dev` or
equivalent) and exercise the actual feature in the running window — golden path and at least
one edge case. Typecheck and `vite build` passing verifies the code compiles, not that the
feature works.
