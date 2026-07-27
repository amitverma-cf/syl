# Git Workflow

One task from `docs/active/tasks.md` = one branch = one PR = one merge = branch deleted. No
long-lived feature branches, no stacking unrelated changes into one branch.

## The loop

1. **Pick the next unchecked task** in `docs/active/tasks.md`, top to bottom within the
   current phase. Don't jump ahead to a later phase's task unless it's explicitly unblocked.
2. **Sync main and branch:**
   ```bash
   git checkout main
   git pull
   git checkout -b <type>/<pillar-or-area>-<short-desc>
   ```
   Branch naming: `feat/engine-host-llama-plugin-load`, `fix/executor-flow-schema-validation`,
   `chore/workspace-clippy-config`. `type` is one of `feat`, `fix`, `chore`, `docs`, `test`,
   `refactor` — matches the commit-message convention below.
3. **Implement the task.** Keep the branch scoped to exactly what the task describes — if you
   notice something else worth doing, add it as a new task in `docs/active/tasks.md` rather
   than doing it inline.
4. **Before pushing, the branch must pass all of:**
   - `cargo fmt --check` (workspace)
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo build --workspace`
   - `cargo test --workspace`
   - `pnpm --dir ui build` (typecheck + Vite build)
   - `pnpm --dir ui lint` (once ESLint is wired in Phase 0 follow-up)
   - For UI-affecting changes: manually exercised in the running app per `style-guide.md`'s
     "UI changes" rule — not just typechecked.
5. **Commit using Conventional Commits**, one logical commit per coherent change (squash
   fixup commits before pushing if you iterated):
   ```
   feat(engine-host): load llama.cpp plugin via libloading

   Verifies the plugin ABI symbol table on load and surfaces a typed
   error instead of panicking if a required symbol is missing.
   ```
6. **Push and open a PR** against `main`. PR description states the task's testable end goal
   from `docs/active/tasks.md` and how it was verified (paste the command output or describe
   the manual UI check).
7. **Auto-merge once the checklist in step 4 passes** (`gh pr merge --squash`, so `main`
   history is one commit per task) — this is a solo-maintainer repo, so PRs don't wait for
   separate human review by default. Revisit this if the project gains other contributors.
8. **Clean up locally:**
   ```bash
   git checkout main
   git pull
   git branch -d <branch-name>
   ```
9. **Check off the task** in `docs/active/tasks.md`, update it if scope shifted, move to the
   next one. Go to step 2.

## Why this shape

`agent.cpp` didn't stall because of a bad idea — it stalled mid-rewrite with a large amount of
uncommitted, unlanded work sitting in the working tree (see `docs/active/plan.md`'s "lessons
carried forward"). A branch that can't be merged within roughly one task's worth of work is a
branch that's easy to abandon. Small, always-mergeable branches mean `main` is always in a
working state, and there's never a multi-week uncommitted rewrite to lose.

## What never happens

- No direct commits to `main`.
- No branch lives past its PR being merged — delete it locally every time, not just
  occasionally.
- No `--no-verify`, no `--force` push to `main`, no skipping the check list in step 4 "just
  this once."
- No branch that bundles two unrelated tasks because they touched nearby code — split them.
