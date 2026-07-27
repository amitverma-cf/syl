# Git Workflow

One unit of work = one branch = one PR = one merge = branch deleted. No long-lived feature
branches, no stacking unrelated changes into one branch.

## The loop

1. **Sync main and branch:**
   ```bash
   git checkout main
   git pull
   git checkout -b <type>/<pillar-or-area>-<short-desc>
   ```
   Branch naming: `feat/engine-host-llama-plugin-load`, `fix/executor-flow-schema-validation`,
   `chore/workspace-clippy-config`. `type` is one of `feat`, `fix`, `chore`, `docs`, `test`,
   `refactor` — matches the commit-message convention below.
2. **Implement one coherent piece of work per branch.** If something else worth doing comes up
   mid-branch, note it and handle it as its own branch later rather than folding it in.
3. **Before pushing, the branch must pass all of:**
   - `cargo fmt --check` (workspace)
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo build --workspace`
   - `cargo test --workspace`
   - `pnpm --dir ui build` (typecheck + Vite build)
   - `pnpm --dir ui lint` (once ESLint is wired in)
   - For UI-affecting changes: manually exercised in the running app per `style-guide.md`'s
     "UI changes" rule — not just typechecked.
4. **Commit using Conventional Commits**, one logical commit per coherent change (squash
   fixup commits before pushing if you iterated):
   ```
   feat(engine-host): load llama.cpp plugin via libloading

   Verifies the plugin ABI symbol table on load and surfaces a typed
   error instead of panicking if a required symbol is missing.
   ```
5. **Push and open a PR** against `main`. PR description states what the change does and how
   it was verified (paste the command output or describe the manual UI check).
6. **PRs are reviewed and merged manually** by the repo owner — no automated merging. Once
   merged on GitHub:
   ```bash
   git checkout main
   git pull
   git branch -d <branch-name>
   ```
7. Go to step 1 for the next piece of work.

## Why this shape

Small, always-mergeable branches keep `main` in a working state and avoid the failure mode of
a large, unlanded rewrite sitting uncommitted for weeks — that kind of stall is exactly what
kills a solo project's momentum.

## What never happens

- No direct commits to `main`.
- No branch lives past its PR being merged — delete it locally every time, not just
  occasionally.
- No `--no-verify`, no `--force` push to `main`, no skipping the checklist in step 3 "just
  this once."
- No branch that bundles two unrelated pieces of work because they touched nearby code —
  split them.
