# Architecture

syl is a local-first, GUI-based agentic app: a Rust core handling inference, tool execution,
and agent orchestration, wrapped in a Tauri (React + Tailwind) shell, with native inference
engines (llama.cpp, ONNX Runtime, Stable Diffusion, and future backends) loaded dynamically at
runtime rather than compiled in.

The full decision-by-decision rationale (alternatives considered, tradeoffs, why each choice
won) lives in `docs/active/plan.md` while a decision is being worked through, and gets
summarized here once it's settled and load-bearing. This document describes what the system
*is*; `docs/active/plan.md` and the git history describe how it got that way.

## The five pillars

Each pillar is an independent crate under `crates/`, with a narrow public trait/type surface
other pillars depend on.

1. **Provider** (`crates/provider`) — abstracts "a thing that can run inference": local
   engines (via `engine-host`'s FFI) and cloud APIs (HTTP). Backed by a multi-model registry —
   many models, across many engines, can be loaded and addressed simultaneously.
2. **Memory** (`crates/memory`) — the durable conversation/state store (SQLite + `sqlite-vec`)
   and context compression.
3. **Tool** (`crates/tool`) — async-first tool execution, gated by permission tiers
   (Allow/Ask/Deny), with sandboxed filesystem/terminal tools as first-class citizens and an
   MCP-client escape hatch for anything not natively supported.
4. **Executor** (`crates/executor`) — runs multi-step agent workflows as a finite-state
   machine loaded from a JSON flow file (states, transitions, per-state system prompt and tool
   allowlist), not hardcoded in Rust.
5. **Daemon** (`crates/daemon`) — background/OS-integrated process management (tray-resident,
   start-on-login), scheduled jobs (cron/time-based), parallel task orchestration, and the
   internal event/pub-sub bus that Executor and Tool publish to.

## Supporting infrastructure

- **Engine Host** (`crates/engine-host`) — dynamically loads native engine plugins via
  `libloading`, one dedicated worker thread per engine (not a separate process — see
  `docs/active/plan.md` Decision #3 for the isolation-tradeoff rationale), a hybrid resource
  watchdog (pre-flight footprint estimate + background OS memory polling), and a
  Struct-of-Arrays continuous-batching scheduler per engine type.
- **Plugin Registry** (`crates/plugin-registry`) — fetches `registry/engines.json` (available
  engine plugin builds) and `registry/models.json` (curated Hugging Face model catalog) from
  this repo, enabling new engines and recommended models to ship without an app release.
- **`src-tauri/`** — the Tauri app shell. Streams batched inference output to the frontend via
  Tauri's `Channel` raw-byte API (not JSON) on the hot path; uses ordinary JSON IPC for
  low-frequency control-plane calls.
- **`ui/`** — React + TypeScript + Tailwind CSS frontend.

## Data layout

- Per-user app data directory (`app_data_dir()` — `%APPDATA%\syl`, `~/Library/Application
  Support/syl`, `~/.local/share/syl`): `db/` (SQLite store), `plugins/` (downloaded engine
  binaries), `models/` (downloaded weights), `flows/` (global flow files), `registry-cache/`,
  `config.json`.
- **User-opened workspace folders** are a separate concept from app data — like an IDE, the
  Tool pillar's filesystem/terminal access is scoped to a folder the user explicitly opens, not
  to the app data directory.

## Distribution

One universal installer per OS (Tauri bundler: MSI/NSIS, DMG, deb/AppImage/RPM), containing
only the app shell. Hardware-specific engine selection (CPU/GPU vendor detection) happens at
first run through the Plugin Registry, not through separate per-hardware installer builds.
App-shell updates ship via Tauri's updater plugin against GitHub Releases, independently of
engine/model updates.

## Serialization boundaries

- Engine (native FFI) ↔ Rust: raw `#[repr(C)]` structs, no serialization.
- Flow files on disk: JSON, parsed with `simd-json`.
- Internal Rust↔Rust hot paths (e.g. the Daemon event bus): `rkyv`, zero-copy.
- Rust core → UI hot path (token streaming, tool-call progress): Tauri `Channel` raw bytes.
- Everything else (registry files, tool-call args, low-frequency control-plane calls):
  `serde_json`.

## Lineage

syl succeeds an earlier C++ project, `agent.cpp`, which stalled mid-rewrite. Its clean ideas
(dynamically-loaded, hardware-matched engine backends; Provider/Memory/Tool/Executor as
pillars; FSM-based flows; permission-tiered tools) carry forward. Its structural mistakes
(a single hardcoded model instance, synchronous/blocking tools, hardcoded rather than
file-based flows, and an allocation/exception/vtable purity that fought its own later GUI
pivot) are the reasons this project is multi-model, async-first, file-driven, and written in
Rust from the start rather than retrofitted later.
