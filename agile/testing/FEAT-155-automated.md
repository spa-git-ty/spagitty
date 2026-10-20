<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-155 — Automated test record

**Item:** [FEAT-155](../items/FEAT-155-untrack.md)

- `crates/spagitty-core/src/untrack.rs`: on `main`, a file and a folder are untracked, stay on disk, get their lines, and `git status` is clean; on `release`, only `release` moves — `main`, a staged file and the `.gitignore` on disk are unchanged and no worktree is left; a staged change refuses; an uncommitted `.gitignore` refuses only when lines would be added; nothing chosen, `../elsewhere`, an empty message and an untracked path refuse; lines are anchored and escaped, and one already there is not added twice.
- `src/lib/untrack/untrack.test.ts`: the suggested message; the screen's `.gitignore` lines match the Rust ones; the branch's own tree is read; a folder takes in what was chosen inside it; a new branch lets the choice go; a typed message is kept; the commit is asked for and the push started to the remote it names, and not when push is off or there is no remote; a refusal is said and the choice kept. The screen starts on the checked-out branch, lists its tree, shows the paths, lines and message before anything is written, ticks and locks a file inside a chosen folder, and says what it committed.
- `src/lib/nav.test.ts`: `/untrack` after `/reflog`.

## Results — 2026-10-20

- `cargo test -p spagitty-core --lib untrack`: 6 passed.
- `bunx vitest run src/lib/untrack src/lib/nav.test.ts src/lib/ui`: 248 passed.
- `bun run check`: 0 errors, 0 warnings.
