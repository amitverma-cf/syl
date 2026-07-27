# syl

A local-first, GUI-based agentic app: a Rust core handling inference, tool execution, and
agent orchestration, wrapped in a Tauri (React + Tailwind) shell, with native inference
engines (llama.cpp, ONNX Runtime, Stable Diffusion, and future backends) loaded dynamically at
runtime.

## Status

Early development. The workspace skeleton (five pillar crates, Engine Host, Plugin Registry,
Tauri + React shell) is in place; pillar implementations are being built out branch by branch.

## Docs

- [docs/architecture.md](docs/architecture.md) — the five pillars, supporting infrastructure,
  data layout, distribution, and why they're built this way.
- [docs/style-guide.md](docs/style-guide.md) — code style, formatting, and what's
  banned/allowed, for Rust and TypeScript.
- [docs/testing.md](docs/testing.md) — what gets tested, where, and how.
- [docs/git-workflow.md](docs/git-workflow.md) — branching and PR process.

## Development

```bash
cargo build --workspace   # Rust workspace
pnpm --dir ui build       # frontend
pnpm tauri dev             # run the app (from src-tauri/, or via `pnpm --dir ui tauri dev`)
```

See [docs/git-workflow.md](docs/git-workflow.md) before opening a PR.

## License

MIT — see [LICENSE](LICENSE).
