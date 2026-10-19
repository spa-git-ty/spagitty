<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-146 — Automated test record

**Item:** [FEAT-146](../items/FEAT-146-a-conflict-resolver-chosen-in-rules.md)

| Layer | Suite | Cases |
| --- | --- | --- |
| Opening and finishing | `crates/spagitty-farm/src/workspace/resolve.rs` | A conflict opens in the task's worktree and not the person's; a resolution with a marker left is refused and a clean one is committed with the destination as a parent; giving up puts the worktree back; a copy fetches the destination before merging. |
| A copy's door | `crates/spagitty-farm/src/workspace/copy.rs` | A merge Spagitty started is taken only once allowed, and its paths are measured from the destination. |
| End to end | `crates/spagitty-farm/tests/lane.rs` | A command resolves a conflict, the task is back in review with main untouched, and merges on the next try; a command that leaves markers stops the task and leaves no merge behind; the task's agent is sent back, finishes the merge, and the task merges. |
| Existing | same | *A conflict blocks the task and touches no file* still passes with the default. |
| The screen | `src/lib/farm` vitest | The Rules editor and wizard suites pass with the new section. |

## Not covered

- A real agent resolving a real conflict.
- The lane tests use `/bin/sh` agents and run in WSL, not natively on Windows.
