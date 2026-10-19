<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-146 — Sweep

**Item:** [FEAT-146](../items/FEAT-146-a-conflict-resolver-chosen-in-rules.md)

None of these rows has been run in the application. The author is doing the visual checks.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT146-01 | New farm | Step 3, *More rules* | *When a merge conflicts* reads *You, in Merger*; choosing *A command* shows a command field. | P1 | Not run. |
| SWEEP-FEAT146-02 | A farm on *The task's agent*, a task that conflicts with main | Let it merge | The task's note reads *Resolving conflicts with main in …*; the agent runs again; the task returns to review; main and your files are untouched until it merges. | P1 | Not run. |
| SWEEP-FEAT146-03 | A farm on *A command* (`mergiraf solve`) | The same | The command runs; on success the task is reviewed again; on failure it stops with the command's output and no merge is left in its worktree. | P2 | Not run. |
