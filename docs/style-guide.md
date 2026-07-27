# Style Guide

## Rust

**Formatting:** `rustfmt` with workspace defaults (no custom `rustfmt.toml` unless a specific
rule causes real friction — don't bikeshed formatting config). Run `cargo fmt` before every
commit; CI checks `cargo fmt --check`.

**Linting:** `cargo clippy --workspace --all-targets -- -D warnings`. Warnings are not
optional cleanup for later — a branch with clippy warnings doesn't get pushed (see
`git-workflow.md`).

### Banned

- **`unwrap()` / `expect()` outside of tests and `main`/`run()` startup code.** Library code
  (anything in `crates/*`) returns `Result` and propagates with `?`. An `unwrap()` in a pillar
  crate is a crash waiting to happen in a GUI app where the user has no terminal to read a
  panic message from.
- **`unsafe` outside `engine-host`.** FFI into native engine plugins is the one place `unsafe`
  is inherent to the problem. Anywhere else, `unsafe` is a sign the abstraction is wrong — fix
  the abstraction instead.
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

- **Doc comments (`///`) on every public item, describing what it does, what it takes, and
  what it returns** — not why it exists or why it was built this way. Follow standard Rust
  doc-comment convention: a one-line summary, and `# Errors` / `# Panics` / `# Safety`
  sections where applicable (mandatory on any `unsafe fn`). Write these the way a reader
  outside the project — someone who has never seen the design discussion — needs them, so
  they can use the function correctly from its signature and doc comment alone. Rationale for
  *why* something is designed the way it is belongs in `docs/architecture.md`, not in the
  code.
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
  screen currently needs.
- Prop drilling more than two levels — reach for React context or lift state instead of
  threading a prop through components that don't use it themselves.

### Allowed / expected

- Co-locate a component's types with the component unless a type is genuinely shared across
  many components, in which case it goes in `ui/src/types/`.
- Tailwind utility classes directly in JSX; a `className` string is not something to abstract
  behind a helper until it's repeated identically in three or more places.
- JSDoc on exported functions/hooks describing parameters and return value, same standard as
  Rust doc comments below.

## Comments, in general (both languages)

Code comments and doc comments describe **what** a function, type, or module does — its
inputs, its outputs, and how to use it — the way a reader of any large public repository
expects to be able to understand an API from its signature and doc comment without reading
the implementation.

Code comments never contain:
- **Why** something was designed this way, what alternatives were considered, or what
  tradeoff was made — that belongs in `docs/architecture.md`.
- Anything about the current task, branch, PR, or a plan for future work — that belongs in
  the commit message and PR description (`git-workflow.md`), never in the source tree.
- References to prior projects, prior mistakes, or how a piece of code came to be.

If a comment would only make sense to someone who read the design discussion, it doesn't
belong in the code — either the code needs to be clearer on its own, or the explanation
belongs in `docs/`.

## UI changes

Per project norm: before calling a UI-affecting change done, run the app (`pnpm tauri dev` or
equivalent) and exercise the actual feature in the running window — golden path and at least
one edge case. Typecheck and `vite build` passing verifies the code compiles, not that the
feature works.
