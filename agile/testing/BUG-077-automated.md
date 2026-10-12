<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-077 — Automated test record

**Item:** [BUG-077](../items/BUG-077-a-farm-task-merged-in-merger-skips-the-farms-checks.md)

| Suite | Cases |
| --- | --- |
| `crates/spagitty-farm/tests/supplemental.rs` | With a required supplemental review and no evidence, a task in review whose branch is already in `main` is refused by `check_mergeable` and by `mark_merged`, and keeps its status and its worktree; after the review passes both accept it, the task is done, and telling the farm again changes nothing. A task that is not in review is refused by both, and keeps its status and its worktree. |
| `crates/spagitty-farm/tests/pipeline.rs` | The farm's own merge and FEAT-117's merge in Merger still pass with the shared check: a task is refused as merged until its work is in the destination, then finished; a merge of a task in the wrong status is refused before git is touched (run in WSL: it uses `/bin/sh`). |
| `src/routes/farm/page.test.ts` | *Merge it in Merger* asks the Farm first and then opens Merger on the task; when the Farm refuses, the notice carries its reason and Merger is not opened; a task with no branch says so. |

## Results — 2026-10-12

- `cargo test -p spagitty-farm --test supplemental` on Windows: 14 passed.
- `cargo test -p spagitty-farm` in WSL: 432 unit, 54 pipeline and 14 supplemental passed; the 4 live agent tests are ignored.
- `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `bunx vitest run`: 185 files, 3851 tests, all passed. `bun run check`: zero errors, zero warnings.
