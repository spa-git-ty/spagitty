<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-143 — Automated test record

**Item:** [FEAT-143](../items/FEAT-143-a-slice-per-task.md)

| Layer | Suite | Cases |
| --- | --- | --- |
| The patterns | `crates/spagitty-farm/src/workspace/slice.rs` | A slice is the top-level files and the declared paths, anchored, without repeats; a task with no paths works in the whole tree. |
| The provider | `crates/spagitty-farm/src/workspace/slice.rs` | A slice on a fixture repository holds its paths and the top-level files and not the rest, while the person's checkout stays whole; widening gives the checks the whole tree and narrowing puts the slice back without losing the agent's edit; widening a worktree that was never narrowed changes nothing. |
| The prompt | `crates/spagitty-farm/src/context.rs` | A slice farm's prompt says what is checked out and that the checks run on the whole tree; a worktree farm's says nothing about it. |
| End to end | `crates/spagitty-farm/tests/pipeline.rs` | On a slice farm the agent does not see `src/`, its work lands in `docs/`, and the checks, which need `src/`, pass. |
| The setup | `crates/spagitty-farm/tests/wizard.rs` | The workspace choice is remembered with the crew and rules; a farm written before reads as worktrees. |
| Existing | `crates/spagitty-farm/tests/pipeline.rs` | Every worktree farm test passes unchanged. |

## Not covered

- Not looked at in the running application; the author is checking it.
- A real agent in a slice has not been run.
- The pipeline test uses a `/bin/sh` agent: it runs in WSL (Arch, git 2.5x), not natively on Windows. The provider's own tests run natively.

## Results — 2026-10-18

- `cargo test -p spagitty-farm --lib -- workspace::slice context::` (Windows): 24 passed, 0 failed.
- `cargo test -p spagitty-farm` (WSL, Arch): 443 unit, 55 pipeline, 14 supplemental passed; the wizard suite's same-millisecond failure there was fixed and `--test wizard` passes 7 of 7 on Windows.
- `bun run check`: 0 errors, 0 warnings.
