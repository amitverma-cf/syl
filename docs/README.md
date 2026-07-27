# Docs

Two kinds of documents live under `docs/`, and they are not interchangeable.

## `docs/active/` — local, temporary, never pushed

`docs/active/` is gitignored. It holds the current planning/task-tracking state for whatever
branch is in progress: `plan.md` (architecture decisions and their rationale) and `tasks.md`
(the phase/task checklist, see [git-workflow.md](git-workflow.md)). It's scratch space for
branch-by-branch work, not a record for other contributors — treat it like a notepad that
happens to live in the repo instead of `docs/active/`'s ignored path, so it survives branch
switches locally but never appears in a PR diff.

If something in `docs/active/` stops being true (a decision changes, a task is dropped),
overwrite it — it isn't versioned history, `git log` is.

## Everything else under `docs/` — permanent, pushed, reviewed

Everything outside `docs/active/` is real project documentation: committed, reviewed in PRs,
and expected to stay accurate. This is what a new contributor or user reads.

- [architecture.md](architecture.md) — the five pillars, supporting infrastructure, how they fit together.
- [style-guide.md](style-guide.md) — code style, formatting, what's banned/allowed, Rust and TypeScript.
- [testing.md](testing.md) — what gets tested, where, and how.
- [git-workflow.md](git-workflow.md) — branch-by-branch implementation process, from `docs/active/tasks.md` to a merged PR.

Rule of thumb: if it explains *why the code is the way it is* for someone reading this repo
in a year, it belongs in permanent docs. If it's *what I'm doing right now on this branch*,
it belongs in `docs/active/`.
