# Architecture

syl is a local-first, GUI-based agentic app: a Rust core handling inference, tool execution,
and agent orchestration, wrapped in a Tauri (React + Tailwind) shell, with native inference
engines (llama.cpp, ONNX Runtime, Stable Diffusion, and future backends) loaded dynamically at
runtime rather than compiled in.

## The five pillars

Each pillar is an independent crate under `crates/`, with a narrow public trait/type surface
other pillars depend on.

1. **Provider** (`crates/provider`) — abstracts "a thing that can run inference": local
   engines (via `engine-host`'s FFI) and cloud APIs (HTTP). Backed by a multi-model registry —
   many models, across many engines, can be loaded and addressed simultaneously, rather than
   one global model instance. A single hardcoded model made it structurally hard to support
   multiple concurrent models or roles, so the registry design is multi-model from the start.
2. **Memory** (`crates/memory`) — the durable conversation store, currently a `ConversationStore`
   trait backed by SQLite (via `rusqlite`, bundled). SQLite is a single file: zero ops, trivial
   backup. Vector search (`sqlite-vec`) and context compression are planned for when embeddings
   and long conversations actually exist — not wired in yet, since nothing currently produces
   embeddings to search over.
3. **Tool** (`crates/tool`) — async-first tool execution, gated by permission tiers
   (Allow/Ask/Deny), with sandboxed filesystem/terminal tools as first-class citizens and an
   MCP-client escape hatch for anything not natively supported. Async is foundational, not
   bolted on: a GUI app needs tool calls to run without freezing the UI thread, and scheduled
   background jobs (see Daemon) need to run concurrently with user-initiated tool calls.
4. **Executor** (`crates/executor`) — runs multi-step agent workflows as a finite-state
   machine loaded from a JSON flow file (states, transitions, per-state system prompt and tool
   allowlist), not hardcoded in Rust. JSON was chosen over YAML/TOML specifically because
   flows are expected to be authored or edited by LLMs as often as by hand: LLMs produce
   syntactically valid JSON far more reliably than YAML (whitespace-sensitive, easy to corrupt
   via indentation drift) or TOML (awkward for nested state graphs), and JSON Schema gives a
   cheap, precise validation step before a flow is loaded. Flow files are parsed with
   `simd-json` for speed.
5. **Daemon** (`crates/daemon`) — background/OS-integrated process management (tray-resident,
   start-on-login), scheduled jobs (cron/time-based), parallel task orchestration, and the
   internal event/pub-sub bus that Executor and Tool publish to. These four responsibilities
   share one pillar rather than four because they're all instances of the same underlying
   property none of the other pillars have: running independent of, and alongside, the
   user's active request/response cycle. Built on `tokio` (async runtime, scheduled jobs via
   `tokio-cron-scheduler`, parallel execution via `tokio::spawn`/`JoinSet`, the event bus via
   `tokio::sync::broadcast`) rather than a hand-rolled cooperative scheduler, because real
   concurrency — not round-robin cooperative scheduling — is required once scheduled jobs,
   concurrent tool calls, and concurrent engine requests are all expected to coexist.
   Runs as a tray-resident, start-on-login background process (via Tauri's autostart plugin
   and system tray API) rather than a true OS service (Windows Service / `launchd` daemon /
   `systemd` system unit): a true service needs admin/root privileges to install and its own
   install/update plumbing separate from the app, which is unwarranted weight for something a
   single user starts and stops themselves — tray-resident-with-autostart satisfies "keeps
   running in the background" without that cost, and matches how comparable local-model apps
   already behave.

## Supporting infrastructure

- **Engine Host** (`crates/engine-host`) — dynamically loads native engine plugins via
  `libloading` rather than statically linking them, so a new engine build or hardware-specific
  variant can ship without recompiling the app. Each engine gets one dedicated OS worker
  thread inside the single app process, rather than a separate child process per engine: this
  is the lower-overhead option (shared memory, zero-copy tensor handoff where possible) at the
  accepted cost that a crash in one engine's native code can, in principle, take down the
  whole app — a tradeoff worth revisiting toward process-per-engine isolation only if that
  proves too fragile in practice for a single-user local app. Also owns the resource watchdog
  (a pre-flight footprint estimate before a model load is attempted, plus a background OS-level
  memory poll that triggers a soft-unload under a safety margin — the hybrid is used because a
  pre-flight check alone can't see memory pressure from other running engines/apps, and a
  reactive poll alone can't prevent an oversized load from being attempted in the first place)
  and a Struct-of-Arrays continuous-batching scheduler per engine type (parallel primitive
  arrays for request ids/token blocks/status, chosen over one array of per-request structs
  because SoA keeps the hot batching-iteration loop cache-line-friendly, which matters here
  specifically because the app targets many parallel in-flight requests across multiple engine
  types at once).

  The llama.cpp bindings (`crates/engine-host/src/llama.rs`) are generated at build time by
  `bindgen` in its dynamic-library mode, from headers vendored under
  `crates/engine-host/vendor/llama-cpp/` — this produces a `libloading`-backed struct with the
  correct C struct layouts (rather than hand-transcribing them, which risks a subtly wrong
  field size or ordering causing memory corruption) while still loading the actual engine
  `.dll`/`.so` at runtime, not linking against it at compile time. llama.cpp's compute backends
  (CPU, Vulkan, CUDA, ...) are themselves dynamically loaded plugins, discovered via
  `ggml_backend_load_all_from_path` at engine-load time — a separate small dynamic-loading
  binding, since that function lives in a different shared library (`ggml.dll`/`libggml.so`)
  than the main engine library.
- **Plugin Registry** (`crates/plugin-registry`) — fetches `registry/engines.json` (available
  engine plugin builds) and `registry/models.json` (curated Hugging Face model catalog) from
  this repo, kept as two separate files because they change at different cadences (engines
  rarely, models often) and serve different consumers in the app (the engine loader vs. the
  model-browsing UI). This is how new engines, hardware-specific builds, or recommended models
  reach users without an app release — a manifest commit instead of a new binary. The registry
  never hosts model weights itself, only metadata and a Hugging Face download URL + hash, to
  keep the registry small and fast to update. Each entry's download URL may be a `file://`
  path (resolved directly, for local development) or an `http://`/`https://` URL (fetched and
  cached once real hosting exists); `registry/engines.json` and `registry/models.json` hold
  only the entries meant to ship to every user, while a machine-local, gitignored
  `registry/local.engines.json`/`registry/local.models.json`, merged in when present, is where
  a developer's own `file://` entries live — so no local filesystem paths ever reach the
  committed registry.
- **`src-tauri/`** — the Tauri app shell. Streams inference output to the frontend via Tauri's
  `Channel` API (currently JSON-serialized per event; switching the hot token-streaming path to
  raw bytes to avoid that per-event serialization cost is a later optimization, not yet done).
  Ordinary JSON IPC is used for low-frequency control-plane calls (opening a conversation,
  listing models), where the extra serialization cost is immaterial and debuggability matters
  more. Structured logging (`tracing`) writes human-readable output to the console and a
  daily-rotated file under the OS-appropriate app data directory, so behavior — including
  per-request timing and tokens/sec — can be inspected after the app window has closed, not
  just while it's running.
- **`ui/`** — React + TypeScript + Tailwind CSS frontend. Chosen over a native immediate-mode
  UI (e.g. Dear ImGui) because the actual inference bottleneck (matrix multiplication) is
  identical either way — it happens via FFI in `engine-host` regardless of UI framework — so a
  leaner native UI process buys negligible real benefit here, while costing markdown/CSS/
  accessibility support that a chat-and-tool-call interface needs. No state-management
  framework, animation library, or component kit is added until a specific screen needs it.

## Data layout

- Per-user app data directory (`app_data_dir()` — `%APPDATA%\syl`, `~/Library/Application
  Support/syl`, `~/.local/share/syl`): `db/` (SQLite store), `plugins/` (downloaded engine
  binaries), `models/` (downloaded weights), `flows/` (global flow files), `registry-cache/`,
  `config.json`.
- **User-opened workspace folders** are a separate concept from app data — like an IDE, the
  Tool pillar's filesystem/terminal access is scoped to a folder the user explicitly opens, not
  to the app data directory. Models/plugins are shared, expensive-to-fetch resources that
  shouldn't be duplicated per workspace or lost if a workspace folder is deleted or moved,
  which is why they live in the app data directory rather than inside whatever folder happens
  to be open.

## Distribution

One universal installer per OS (Tauri bundler: MSI/NSIS, DMG, deb/AppImage/RPM), containing
only the app shell — no engine binaries, no models. Hardware-specific engine selection
(CPU/GPU vendor detection) happens at first run through the Plugin Registry rather than
through separate per-hardware installer builds: this reuses the exact mechanism already needed
for zero-touch engine updates, instead of duplicating that hardware-detection logic a second
time at install time. App-shell updates ship via Tauri's updater plugin against GitHub
Releases, independently of engine/model updates.

## Serialization boundaries

There is no single fastest serialization format for every boundary in this app — each one is
chosen for what crosses it:

- Engine (native FFI) ↔ Rust: raw `#[repr(C)]` structs passed directly, no serialization at
  all — this crosses a C ABI, not a serialization boundary.
- Flow files on disk: JSON, parsed with `simd-json` (a maintained Rust port of the `simdjson`
  algorithm) for fast parsing of a format expected to be LLM-generated.
- Internal Rust↔Rust hot paths (e.g. the Daemon event bus): `rkyv`, whose wire bytes are the
  struct layout itself — no parsing step — at the accepted cost of a data format pinned per
  struct version, acceptable because nothing outside the process ever needs to read it.
- Rust core → UI hot path (token streaming, tool-call progress): Tauri `Channel` raw bytes,
  bypassing JSON entirely rather than just parsing it faster.
- Everything else (registry files, tool-call args, low-frequency control-plane calls):
  `serde_json` — simple, debuggable, and fast enough at low call volume.

## Lineage

syl succeeds an earlier C++ project that stalled mid-rewrite. Its clean ideas carried forward:
dynamically-loaded, hardware-matched engine backends; Provider/Memory/Tool/Executor as
pillars; FSM-based flows; permission-tiered tools. Its structural mistakes are the reasons
this project is shaped the way it is: a single hardcoded model instance made multi-model
support a late, invasive change, so Provider is multi-model from day one; synchronous/blocking
tools made a responsive GUI impossible, so Tool and Daemon are async-first from day one;
hardcoded (rather than file-based) flows meant no way to author or share workflows without
recompiling, so Executor is file-driven from day one; and a strict zero-allocation/
no-exceptions/no-vtable discipline, while performance-motivated, actively fought a later pivot
toward an interactive, concurrent, GUI-driven product — Rust's ownership model gets most of
the same safety and performance benefit without that rigidity, which is part of why this
project is written in Rust rather than C++.
