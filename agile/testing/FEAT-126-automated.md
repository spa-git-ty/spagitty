<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-126 — Automated test record

**Item:** [FEAT-126](../items/FEAT-126-merger-says-a-branch-is-behind-its-remote-and-pulls-it.md)

| Suite | Cases |
| --- | --- |
| `crates/spagitty-core/src/ops.rs` | Against a bare remote on disk that both branches track. A branch checked out nowhere is moved up to its upstream, and the branch checked out neither moves nor stops being the one checked out. The branch checked out is pulled where it is and its files follow. A branch with a commit its upstream lacks is refused and left where it was. A branch that tracks nothing is refused in words. |
| `src/lib/merger/plan.test.ts` | Which rows count as behind: local branches with an upstream that has commits they lack, and not level ones, ahead-only ones, ones that track nothing, ones never fetched, or remote branches. The line, for one commit and for several. No refusal for a branch that is only behind; the sentence for one that is also ahead, for one commit and for several. |
| `src/routes/merge/page.test.ts` | Mounted. Nothing new on a card whose branch is level or tracks nothing. The line and *Pull now* on the card of the branch that is behind and not on the other; pressing it asks for that branch by name, reads the refs again, and lets the lanes go. The same for the branch that is not checked out. A branch with commits of its own has the button disabled with the reason, and a press asks for nothing. While a fetch runs the button waits and says for what, and the store refuses too. When git refuses, the lanes are let go and the refs are read again. |

## Not covered

- The `pull_branch` command in `src-tauri` is three lines over the core function and has no test of its own. `cargo clippy -p spagitty --all-targets` compiles it.
- A branch checked out in a second worktree. The same code path as the branch checked out here, with another directory.
- A hosted remote, and credentials.

## Results — 2026-10-12

- `cargo test -p spagitty-core --lib ops::`: 16 passed, on Windows.
- `cargo clippy -p spagitty-core -p spagitty --all-targets -- -D warnings`: clean. `cargo fmt --all -- --check`: clean.
- `bunx vitest run`: 189 files, 4099 tests, all passed.
- `bun run check`: zero errors, zero warnings. `bun run build`: succeeded.
- The whole Rust suite was not run here; `cargo test -p spagitty` does not start natively on this machine.
