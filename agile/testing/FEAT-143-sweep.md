<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-143 — Sweep

**Item:** [FEAT-143](../items/FEAT-143-a-slice-per-task.md)

None of these rows has been run in the application. The author is doing the visual checks.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT143-01 | New farm | Step 3, *Where each task works* | Two choices; *A slice per task* is tagged `new`. The choice is in Your farm's Rules line after the farm starts, and in the Rules sheet. | P1 | Not run. |
| SWEEP-FEAT143-02 | A slice farm, a task with *May touch* `docs/**` | Run it; open its worktree | Only the top-level files and `docs/` are checked out. | P1 | Not run. |
| SWEEP-FEAT143-03 | The same task | Let it reach Checks | The worktree is whole while the checks run; `cargo test --workspace` builds. | P1 | Not run. |
| SWEEP-FEAT143-04 | The same task sent back | Its next run | The slice is narrowed again; the agent's earlier commits are still there. | P2 | Not run. |
| SWEEP-FEAT143-05 | A task with *Reads* | Edit it, then run it | *Reads* is kept; those paths are in the slice too. | P2 | Not run. |
| SWEEP-FEAT143-06 | A task with no paths | Run it on a slice farm | It works in the whole tree. | P2 | Not run. |
